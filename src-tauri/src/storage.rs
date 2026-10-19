use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path, sync::Mutex};

static WRITE: Mutex<()> = Mutex::new(());
const KEYS: &[&str] = &[
    "roots",
    "registry",
    "groups",
    "directories",
    "pins",
    "visits",
    "last",
    "activities",
    "clones",
];
fn error(e: impl std::fmt::Display) -> String {
    format!("本地数据库：{e}")
}
fn connection_at(path: &Path) -> Result<Connection, String> {
    let db = Connection::open(path).map_err(error)?;
    db.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(error)?;
    let version: i64 = db
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(error)?;
    if version > 1 {
        return Err("数据库来自更新版本的应用，已保留原数据，请使用匹配版本打开".into());
    }
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS business_data (key TEXT PRIMARY KEY, value TEXT NOT NULL CHECK(json_valid(value))); PRAGMA user_version=1;").map_err(error)?;
    Ok(db)
}
fn connection() -> Result<Connection, String> {
    connection_at(&super::local_file("data", "repodeck.sqlite")?)
}
fn get(db: &Connection, key: &str) -> Result<Option<Value>, String> {
    let raw: Option<String> = db
        .query_row("SELECT value FROM business_data WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()
        .map_err(error)?;
    raw.map(|s| serde_json::from_str(&s).map_err(error))
        .transpose()
}
fn put(db: &Connection, key: &str, value: &Value) -> Result<(), String> {
    db.execute("INSERT INTO business_data(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value.to_string()]).map_err(error)?;
    Ok(())
}
pub fn read(key: &str) -> Result<Option<Value>, String> {
    get(&connection()?, key)
}
pub fn write(key: &str, value: &Value) -> Result<(), String> {
    let _lock = WRITE.lock().map_err(error)?;
    put(&connection()?, key, value)
}
pub fn migrate(key: &str, legacy: &Path) -> Result<Option<Value>, String> {
    let _lock = WRITE.lock().map_err(error)?;
    let db = connection()?;
    if let Some(value) = get(&db, key)? {
        return Ok(Some(value));
    }
    if !legacy.exists() {
        return Ok(None);
    }
    let value: Value = serde_json::from_slice(&fs::read(legacy).map_err(error)?)
        .map_err(|e| format!("旧数据损坏，未覆盖原文件：{e}"))?;
    validate(key, &value)?;
    put(&db, key, &value)?;
    Ok(Some(value))
}
fn strings(v: &Value) -> bool {
    v.as_array().is_some_and(|a| {
        a.len() <= 20000
            && a.iter()
                .all(|x| x.as_str().is_some_and(|s| s.len() <= 32768))
    })
}
fn validate(key: &str, v: &Value) -> Result<(), String> {
    let valid = match key {
        "registry" => v.is_object() && strings(&v["paths"]) && strings(&v["excluded"]),
        "roots" => v.as_array().is_some_and(|a| {
            a.iter().all(|r| {
                r["path"].is_string()
                    && (r["kind"].is_null()
                        || ["repository", "workspace"].contains(&r["kind"].as_str().unwrap_or("")))
            })
        }),
        "groups" => {
            serde_json::from_value::<Vec<super::repository_tools::RepositoryGroup>>(v.clone())
                .map(|g| super::repository_tools::validate_groups(&g).is_ok())
                .unwrap_or(false)
        }
        "pins" | "directories" => strings(v),
        "visits" => v
            .as_object()
            .is_some_and(|o| o.values().all(Value::is_number)),
        "last" => v.is_string() || v.is_null(),
        "activities" | "clones" => v.as_array().is_some_and(|a| {
            a.len() <= 1000
                && a.iter()
                    .all(|r| r["id"].is_string() && r["path"].is_string() && if key == "activities" {
                        r["label"].is_string() && r["detail"].is_string() && r["at"].is_string() && ["running","success","error"].contains(&r["status"].as_str().unwrap_or(""))
                    } else {
                        r["name"].is_string() && r["error"].is_string() && r["output"].is_string() && r["register"].is_boolean() && r["percent"].as_f64().is_some_and(|p| (0.0..=100.0).contains(&p)) && ["running","stopping","stopped","success","error"].contains(&r["state"].as_str().unwrap_or(""))
                    })
        }),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(format!("数据格式无效：{key}"))
    }
}
#[tauri::command]
pub fn read_business_data(key: String) -> Result<Option<Value>, String> {
    if !KEYS.contains(&key.as_str()) {
        return Err("未知数据分类".into());
    }
    read(&key)
}
#[tauri::command]
pub fn save_business_data(key: String, value: Value) -> Result<(), String> {
    validate(&key, &value)?;
    write(&key, &value)
}
#[tauri::command]
pub fn migrate_browser_data(
    values: BTreeMap<String, Value>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Native JSON was the source of truth; migrate it before stale WebView caches.
    super::read_roots(&app)?;
    super::read_repository_registry()?;
    super::repository_tools::read_repository_groups()?;
    super::clone::read_clone_directories()?;
    let _lock = WRITE.lock().map_err(error)?;
    let mut db = connection()?;
    let tx = db.transaction().map_err(error)?;
    for (key, value) in values {
        if ![
            "activities",
            "clones",
            "pins",
            "visits",
            "last",
            "groups",
            "directories",
        ]
        .contains(&key.as_str())
        {
            continue;
        }
        validate(&key, &value)?;
        if get(&tx, &key)?.is_none() {
            put(&tx, &key, &value)?;
        }
    }
    tx.commit().map_err(error)
}
fn snapshot(db: &Connection) -> Result<BTreeMap<String, Value>, String> {
    let mut values = BTreeMap::new();
    let mut statement = db
        .prepare("SELECT key,value FROM business_data")
        .map_err(error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(error)?;
    for row in rows {
        let (key, raw) = row.map_err(error)?;
        if KEYS.contains(&key.as_str()) {
            values.insert(key, serde_json::from_str(&raw).map_err(error)?);
        }
    }
    Ok(values)
}
fn bundle(path: &str) -> Result<BTreeMap<String, Value>, String> {
    if fs::metadata(path).map_err(error)?.len() > 20_000_000 {
        return Err("导入文件超过 20 MB".into());
    }
    let value: Value = serde_json::from_slice(&fs::read(path).map_err(error)?).map_err(error)?;
    if value["format"] != "repodeck-data" || value["version"] != 1 {
        return Err("不支持的数据包格式或版本".into());
    }
    let values: BTreeMap<String, Value> =
        serde_json::from_value(value["data"].clone()).map_err(error)?;
    for (key, value) in &values {
        validate(key, value)?;
    }
    Ok(values)
}
fn merge_array(current: Value, incoming: Value, identity: &str) -> Value {
    let mut result = current.as_array().cloned().unwrap_or_default();
    for item in incoming.as_array().into_iter().flatten() {
        let token = if identity.is_empty() {
            item.clone()
        } else {
            item[identity].clone()
        };
        if !result.iter().any(|r| {
            if identity.is_empty() {
                match (r.as_str(), item.as_str()) {
                    (Some(a), Some(b)) => super::paths_equal(a, b),
                    _ => r == item,
                }
            } else if identity == "path" {
                super::paths_equal(
                    r[identity].as_str().unwrap_or(""),
                    token.as_str().unwrap_or(""),
                )
            } else {
                r[identity] == token
            }
        }) {
            result.push(item.clone());
        }
    }
    Value::Array(result)
}
fn merged(key: &str, current: Value, incoming: Value) -> Value {
    match key {
        "registry" => {
            let paths = merge_array(current["paths"].clone(), incoming["paths"].clone(), "");
            let mut excluded = merge_array(
                current["excluded"].clone(),
                incoming["excluded"].clone(),
                "",
            );
            if let Some(a) = excluded.as_array_mut() {
                a.retain(|p| !paths.as_array().unwrap().contains(p));
            }
            json!({"paths":paths,"excluded":excluded})
        }
        "visits" => {
            let mut map = current.as_object().cloned().unwrap_or_default();
            for (key, value) in incoming.as_object().into_iter().flatten() {
                if map.get(key).and_then(Value::as_f64).unwrap_or(0.) < value.as_f64().unwrap_or(0.)
                {
                    map.insert(key.clone(), value.clone());
                }
            }
            Value::Object(map)
        }
        "last" => {
            if current.is_null() {
                incoming
            } else {
                current
            }
        }
        "roots" => merge_array(current, incoming, "path"),
        "groups" => {
            let mut list = current.as_array().cloned().unwrap_or_default();
            for group in incoming.as_array().into_iter().flatten() {
                if let Some(existing) = list.iter_mut().find(|g| {
                    g["id"] == group["id"]
                        || g["name"].as_str().unwrap_or("").to_lowercase()
                            == group["name"].as_str().unwrap_or("").to_lowercase()
                }) {
                    existing["members"] =
                        merge_array(existing["members"].clone(), group["members"].clone(), "");
                } else {
                    list.push(group.clone());
                }
            }
            Value::Array(list)
        }
        "directories" => {
            let mut value = merge_array(current,incoming,"");
            if let Some(a) = value.as_array_mut() { a.truncate(8); }
            value
        }
        "activities" | "clones" => {
            let mut value = merge_array(current, incoming, "id");
            if let Some(a) = value.as_array_mut() {
                a.truncate(1000);
            }
            value
        }
        _ => merge_array(current, incoming, ""),
    }
}
#[tauri::command]
pub fn export_app_data(path: String) -> Result<(), String> {
    let db = connection()?;
    let data = snapshot(&db)?;
    super::atomic_write(
        Path::new(&path),
        &serde_json::to_vec_pretty(&json!({"format":"repodeck-data", "version":1, "data":data}))
            .map_err(error)?,
    )
}
#[tauri::command]
pub fn preview_app_import(path: String) -> Result<Value, String> {
    let values = bundle(&path)?;
    let paths = values
        .get("registry")
        .and_then(|v| v["paths"].as_array())
        .cloned()
        .unwrap_or_default();
    let missing: Vec<Value> = paths
        .iter()
        .filter(|p| !Path::new(p.as_str().unwrap_or("")).is_dir())
        .cloned()
        .collect();
    Ok(
        json!({"repositories":paths.len(),"groups": values.get("groups").and_then(Value::as_array).map_or(0,Vec::len),"missing":missing}),
    )
}
#[tauri::command]
pub fn import_app_data(path: String) -> Result<(), String> {
    let values = bundle(&path)?;
    let _lock = WRITE.lock().map_err(error)?;
    let mut db = connection()?;
    let previous = snapshot(&db)?;
    let backup = super::local_file(
        "backup",
        &format!(
            "before-import-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(error)?
                .as_millis()
        ),
    )?;
    super::atomic_write(
        &backup,
        &serde_json::to_vec_pretty(&json!({"format":"repodeck-data","version":1,"data":previous}))
            .map_err(error)?,
    )?;
    apply_import(&mut db, values)
}
fn apply_import(db: &mut Connection, values: BTreeMap<String, Value>) -> Result<(), String> {
    let tx = db.transaction().map_err(error)?;
    for (key, value) in values {
        let next = merged(&key, get(&tx, &key)?.unwrap_or(Value::Null), value);
        validate(&key, &next)?;
        put(&tx, &key, &next)?;
    }
    tx.commit().map_err(error)
}

#[tauri::command]
pub fn relocate_repository(old_path: String, new_path: String) -> Result<String, String> {
    let target = super::canonical_repository(&new_path)?
        .to_string_lossy()
        .into_owned();
    let _lock = WRITE.lock().map_err(error)?;
    let mut db = connection()?;
    relocate_in(&mut db, &old_path, &target)?;
    Ok(target)
}
fn relocate_in(db: &mut Connection, old_path: &str, target: &str) -> Result<(), String> {
    let old_id = super::path_key(old_path);
    let new_id = super::path_key(target);
    let tx = db.transaction().map_err(error)?;
    let mut registry = get(&tx, "registry")?.unwrap_or_else(|| json!({"paths":[],"excluded":[]}));
    let mut paths = registry["paths"].as_array().cloned().unwrap_or_default();
    paths.retain(|p| {
        super::path_key(p.as_str().unwrap_or("")) != old_id
            && super::path_key(p.as_str().unwrap_or("")) != new_id
    });
    paths.push(json!(target));
    registry["paths"] = json!(paths);
    if let Some(a) = registry["excluded"].as_array_mut() {
        a.retain(|p| super::path_key(p.as_str().unwrap_or("")) != new_id);
        a.push(json!(old_path));
    }
    put(&tx, "registry", &registry)?;
    let mut roots = get(&tx, "roots")?.unwrap_or_else(|| json!([]));
    if let Some(a) = roots.as_array_mut() {
        for root in a {
            if root["kind"] == "repository"
                && super::path_key(root["path"].as_str().unwrap_or("")) == old_id
            {
                root["path"] = json!(target);
            }
        }
    }
    put(&tx, "roots", &roots)?;
    let mut groups = get(&tx, "groups")?.unwrap_or_else(|| json!([]));
    for group in groups.as_array_mut().ok_or("分组数据无效")? {
        if let Some(members) = group["members"].as_array_mut() {
            for member in members.iter_mut() {
                if member == &json!(old_id) {
                    *member = json!(new_id);
                }
            }
            let mut unique = Vec::new();
            members.retain(|m| {
                if unique.contains(m) {
                    false
                } else {
                    unique.push(m.clone());
                    true
                }
            });
        }
    }
    put(&tx, "groups", &groups)?;
    let pins = get(&tx, "pins")?.unwrap_or_else(|| json!([]));
    let mapped: Vec<Value> = pins
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            if p == &json!(old_id) {
                json!(new_id)
            } else {
                p.clone()
            }
        })
        .collect();
    put(&tx, "pins", &merge_array(json!([]), json!(mapped), ""))?;
    let mut visits = get(&tx, "visits")?.unwrap_or_else(|| json!({}));
    if let Some(o) = visits.as_object_mut() {
        if let Some(time) = o.remove(&old_id) {
            let previous = o.get(&new_id).and_then(Value::as_f64).unwrap_or(0.);
            if time.as_f64().unwrap_or(0.) > previous {
                o.insert(new_id.clone(), time);
            }
        }
    }
    put(&tx, "visits", &visits)?;
    let encoded: String = new_id
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    put(&tx, "last", &json!(format!("/repositories/{encoded}")))?;
    tx.commit().map_err(error)?;
    Ok(())
}

#[tauri::command]
pub fn export_preferences(path: String) -> Result<(), String> {
    let value = super::read_preferences()?.unwrap_or_else(|| json!({}));
    super::atomic_write(
        Path::new(&path),
        &serde_json::to_vec_pretty(
            &json!({"format":"repodeck-preferences","version":1,"preferences":value}),
        )
        .map_err(error)?,
    )
}
#[tauri::command]
pub fn import_preferences(path: String, apply: bool) -> Result<Value, String> {
    if fs::metadata(&path).map_err(error)?.len() > 1_000_000 {
        return Err("偏好文件过大".into());
    }
    let value: Value = serde_json::from_slice(&fs::read(path).map_err(error)?).map_err(error)?;
    if value["format"] != "repodeck-preferences"
        || value["version"] != 1
        || !value["preferences"].is_object()
    {
        return Err("无效的偏好文件".into());
    }
    if apply {
        if let Some(old) = super::read_preferences()? {
            super::atomic_write(
                &super::local_file("backup", "preferences-before-import.json")?,
                &serde_json::to_vec_pretty(&old).map_err(error)?,
            )?;
        }
        super::save_preferences(value["preferences"].clone())?;
    }
    Ok(value["preferences"].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sqlite_transaction_rolls_back_invalid_import_and_keeps_data() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE business_data(key TEXT PRIMARY KEY,value TEXT NOT NULL CHECK(json_valid(value)))").unwrap();
        put(&db, "pins", &json!(["old"])).unwrap();
        {
            let tx = db.transaction().unwrap();
            put(&tx, "pins", &json!(["new"])).unwrap();
        }
        assert_eq!(get(&db, "pins").unwrap().unwrap(), json!(["old"]));
        assert!(validate("registry", &json!({"paths": [12]})).is_err());
        assert_eq!(
            merged("pins", json!(["a"]), json!(["a", "b"])),
            json!(["a", "b"])
        );
    }
    #[test]
    fn export_bundle_round_trip_merges_groups_pins_and_missing_paths() {
        let root = std::env::temp_dir().join(format!(
            "repodeck-storage-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut db = connection_at(&root.join("test.sqlite")).unwrap();
        put(
            &db,
            "registry",
            &json!({"paths":[root.join("offline-repo").to_string_lossy()],"excluded":[]}),
        )
        .unwrap();
        put(&db, "pins", &json!(["repo-a"])).unwrap();
        put(
            &db,
            "groups",
            &json!([{"id":"work","name":"Work","color":"primary","members":["repo-a"]}]),
        )
        .unwrap();
        let file = root.join("export.json");
        fs::write(
            &file,
            serde_json::to_vec(
                &json!({"format":"repodeck-data","version":1,"data":snapshot(&db).unwrap()}),
            )
            .unwrap(),
        )
        .unwrap();
        let values = bundle(file.to_str().unwrap()).unwrap();
        put(&db, "pins", &json!(["repo-b"])).unwrap();
        put(
            &db,
            "groups",
            &json!([{"id":"work","name":"Work","color":"primary","members":["repo-b"]}]),
        )
        .unwrap();
        apply_import(&mut db, values).unwrap();
        assert_eq!(
            get(&db, "pins").unwrap().unwrap(),
            json!(["repo-b", "repo-a"])
        );
        assert_eq!(
            get(&db, "groups").unwrap().unwrap()[0]["members"],
            json!(["repo-b", "repo-a"])
        );
        let before = snapshot(&db).unwrap();
        let mut invalid = BTreeMap::new();
        invalid.insert("pins".into(), json!(["new"]));
        invalid.insert("unknown".into(), json!({}));
        assert!(apply_import(&mut db, invalid).is_err());
        assert_eq!(snapshot(&db).unwrap(), before);
        assert_eq!(
            db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        drop(db);
        assert!(root.starts_with(std::env::temp_dir()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relocating_updates_associations_without_moving_any_project_files() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE business_data(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        let old = "E:\\Old\\repo";
        let next = "E:\\New\\repo";
        let old_id = super::super::path_key(old);
        let new_id = super::super::path_key(next);
        put(&db, "registry", &json!({"paths":[old],"excluded":[]})).unwrap();
        put(&db, "roots", &json!([{"path":old,"kind":"repository"}])).unwrap();
        put(
            &db,
            "groups",
            &json!([{"id":"g","name":"Work","color":"primary","members":[old_id]}]),
        )
        .unwrap();
        put(&db, "pins", &json!([old_id])).unwrap();
        put(&db, "visits", &json!({old_id:123})).unwrap();
        relocate_in(&mut db, old, next).unwrap();
        assert_eq!(
            get(&db, "registry").unwrap().unwrap()["paths"],
            json!([next])
        );
        assert_eq!(
            get(&db, "groups").unwrap().unwrap()[0]["members"],
            json!([new_id])
        );
        assert_eq!(get(&db, "pins").unwrap().unwrap(), json!([new_id]));
        assert_eq!(get(&db, "visits").unwrap().unwrap()[new_id], 123);
    }
}
