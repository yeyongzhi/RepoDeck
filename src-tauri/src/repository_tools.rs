use super::*;
use serde_json::{json, Value};

fn commit_ref(repo: &Path, value: &str) -> Result<String, String> {
    if value.starts_with('-') || value.contains('\0') {
        return Err("无效引用".into());
    }
    run_git(
        repo,
        &["rev-parse", "--verify", &format!("{value}^{{commit}}")],
    )
}

fn branch_name(repo: &Path, value: &str) -> Result<(), String> {
    if value.starts_with('-') || value == "HEAD" || value.trim() != value {
        return Err("分支名称无效".into());
    }
    run_git(repo, &["check-ref-format", &format!("refs/heads/{value}")]).map(|_| ())
}

// Read-only operations resolve references to object IDs before invoking diff/log.
#[tauri::command]
pub fn repository_read(
    path: String,
    kind: String,
    first: Option<String>,
    second: Option<String>,
) -> Result<Value, String> {
    let repo = canonical_repository(&path)?;
    let first = first.unwrap_or_default();
    match kind.as_str() {
        "commit" => {
            let hash = commit_ref(&repo, &first)?;
            let message = run_git(
                &repo,
                &[
                    "show",
                    "-s",
                    "--format=%B%n%n作者：%an <%ae>%n提交时间：%cI",
                    &hash,
                ],
            )?;
            let files = run_git(
                &repo,
                &[
                    "diff-tree",
                    "--root",
                    "--no-commit-id",
                    "--name-only",
                    "-r",
                    "-m",
                    &hash,
                ],
            )?;
            let diff = run_git(
                &repo,
                &[
                    "show",
                    "--format=",
                    "--first-parent",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    &hash,
                    "--",
                ],
            )?;
            Ok(json!({"message": message, "files": files, "diff": diff}))
        }
        "worktree" => {
            validate_changed_paths(&repo, std::slice::from_ref(&first))?;
            let changes = get_worktree_changes(path.clone())?;
            let change = changes
                .iter()
                .find(|item| item.path == first)
                .ok_or("文件状态已变化，请刷新")?;
            if change.untracked {
                let file = repo
                    .join(&first)
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                if !file.starts_with(&repo) {
                    return Err("文件不在仓库中".into());
                }
                let metadata = fs::metadata(&file).map_err(|e| e.to_string())?;
                let text = if metadata.len() > 2_000_000 {
                    "文件过大，请在编辑器中查看".into()
                } else {
                    match fs::read(&file) {
                        Ok(bytes) if !bytes.contains(&0) => {
                            String::from_utf8_lossy(&bytes).into_owned()
                        }
                        _ => "二进制文件无法显示文本差异".into(),
                    }
                };
                return Ok(json!({"message": "未跟踪文件", "files": first, "diff": text}));
            }
            let staged = run_git(
                &repo,
                &[
                    "diff",
                    "--cached",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    "--",
                    &first,
                ],
            )?;
            let unstaged = run_git(
                &repo,
                &[
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    "--",
                    &first,
                ],
            )?;
            Ok(
                json!({"message": first, "files": "", "diff": format!("已暂存\n{staged}\n\n未暂存\n{unstaged}")}),
            )
        }
        "compare" => {
            let left = commit_ref(&repo, &first)?;
            let right = commit_ref(&repo, &second.unwrap_or_default())?;
            let range = format!("{left}...{right}");
            let message = run_git(
                &repo,
                &[
                    "log",
                    "--left-right",
                    "--format=%m %h %s",
                    "-100",
                    &range,
                    "--",
                ],
            )?;
            let files = run_git(&repo, &["diff", "--stat", &left, &right, "--"])?;
            let diff = run_git(
                &repo,
                &[
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    &left,
                    &right,
                    "--",
                ],
            )?;
            Ok(
                json!({"message": format!("独有提交（< 左侧，> 右侧，最多 100 条）\n{message}"), "files": files, "diff": diff}),
            )
        }
        "tags" => Ok(json!(git_lines(&repo, &["tag", "--sort=-creatordate"])?)),
        "stashes" => {
            let output = run_git(&repo, &["stash", "list", "--format=%H%x1f%gd%x1f%gs"])?;
            Ok(json!(output.lines().filter_map(|line| {
                let mut parts = line.splitn(3, '\u{1f}');
                Some(json!({"hash":parts.next()?, "reference":parts.next()?, "subject":parts.next()?}))
            }).collect::<Vec<_>>()))
        }
        "stash" => {
            let hash = commit_ref(&repo, &first)?;
            let diff = run_git(
                &repo,
                &[
                    "stash",
                    "show",
                    "--include-untracked",
                    "-p",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    &hash,
                ],
            )?;
            Ok(json!({"message": "暂存改动", "files": "", "diff": diff}))
        }
        _ => Err("不支持的查询".into()),
    }
}

#[tauri::command]
pub fn repository_action(
    path: String,
    kind: String,
    first: Option<String>,
    second: Option<String>,
) -> Result<String, String> {
    let repo = canonical_repository(&path)?;
    let snapshot = inspect_repository(&repo, path.clone())?;
    if snapshot.operation_in_progress || snapshot.has_conflicts {
        return Err("请先解决冲突并完成当前 Git 操作".into());
    }
    let first = first.unwrap_or_default();
    let second = second.unwrap_or_default();
    match kind.as_str() {
        "rename" => {
            if !snapshot.local_branches.contains(&first) {
                return Err("本地分支不存在".into());
            }
            branch_name(&repo, &second)?;
            run_git(&repo, &["branch", "-m", "--", &first, &second])
        }
        "upstream" | "unset-upstream" => {
            if !snapshot.local_branches.contains(&first) {
                return Err("本地分支不存在".into());
            }
            if kind == "upstream" {
                if !snapshot.remote_branches.contains(&second) {
                    return Err("远程分支不存在，请先 Fetch".into());
                }
                run_git(
                    &repo,
                    &[
                        "branch",
                        &format!("--set-upstream-to=refs/remotes/{second}"),
                        "--",
                        &first,
                    ],
                )
            } else {
                run_git(&repo, &["branch", "--unset-upstream", "--", &first])
            }
        }
        "publish" => {
            if !snapshot.local_branches.contains(&first)
                || !snapshot.remotes.iter().any(|item| item.name == second)
            {
                return Err("请选择本地分支和远程仓库".into());
            }
            run_git(
                &repo,
                &[
                    "push",
                    "--set-upstream",
                    "--",
                    &second,
                    &format!("refs/heads/{first}:refs/heads/{first}"),
                ],
            )
        }
        "tag-branch" => {
            branch_name(&repo, &second)?;
            let hash = commit_ref(&repo, &format!("refs/tags/{first}"))?;
            run_git(&repo, &["branch", "--no-track", "--", &second, &hash])
        }
        "stash-save" => {
            if !snapshot.is_dirty {
                return Err("工作区没有需要保存的改动".into());
            }
            run_git(
                &repo,
                &[
                    "stash",
                    "push",
                    "--include-untracked",
                    "--message",
                    if first.trim().is_empty() {
                        "RepoDeck 暂存"
                    } else {
                        &first
                    },
                ],
            )
        }
        "stash-apply" | "stash-drop" => {
            let entries = run_git(&repo, &["stash", "list", "--format=%H%x1f%gd"])?;
            let reference = entries
                .lines()
                .find_map(|line| {
                    line.split_once('\u{1f}')
                        .filter(|(hash, _)| *hash == first)
                        .map(|(_, reference)| reference.to_string())
                })
                .ok_or("暂存记录已变化，请刷新")?;
            if kind == "stash-apply" {
                if snapshot.is_dirty {
                    return Err("请先保存或提交当前改动，再恢复暂存".into());
                }
                run_git(&repo, &["stash", "apply", "--index", &reference])
            } else {
                run_git(&repo, &["stash", "drop", &reference])
            }
        }
        _ => Err("不支持的操作".into()),
    }
}

#[tauri::command]
pub fn open_repository_app(path: String, kind: String) -> Result<(), String> {
    let repo = canonical_repository(&path)?;
    let preferences = super::read_preferences()?.unwrap_or_else(|| json!({}));
    let custom = preferences[if kind == "editor" {"editorPath"} else {"terminalPath"}].as_str().unwrap_or("");
    if !custom.is_empty() {
        let executable = PathBuf::from(custom);
        if !executable.is_absolute() || !executable.is_file() { return Err("配置的工具路径不存在，请在设置中重新选择".into()); }
        #[cfg(windows)]
        if !executable.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) { return Err("请选择 .exe 可执行程序".into()); }
        let mut command = Command::new(executable);
        if kind == "editor" { command.arg(&repo); } else if kind == "terminal" { command.current_dir(&repo); } else { return Err("不支持的应用入口".into()); }
        return command.spawn().map(|_|()).map_err(|e| format!("打开配置的工具失败：{e}"));
    }
    let mut command = match kind.as_str() {
        "editor" => {
            #[cfg(windows)]
            let executable = std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|base| base.join("Programs/Microsoft VS Code/Code.exe"))
                .filter(|file| file.is_file())
                .unwrap_or_else(|| PathBuf::from("code"));
            #[cfg(not(windows))]
            let executable = PathBuf::from("code");
            let mut command = Command::new(executable);
            command.arg(&repo);
            command
        }
        "terminal" => {
            #[cfg(windows)]
            let mut command = Command::new("powershell.exe");
            #[cfg(not(windows))]
            let mut command = Command::new("x-terminal-emulator");
            command.current_dir(&repo);
            command
        }
        _ => return Err("不支持的应用入口".into()),
    };
    command
        .spawn()
        .map_err(|e| format!("无法打开应用，请确认已安装：{e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("repodeck-tools-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            run_git(&path, &["init", "-b", "main"]).unwrap();
            run_git(&path, &["config", "core.autocrlf", "false"]).unwrap();
            run_git(&path, &["config", "user.name", "Test"]).unwrap();
            run_git(&path, &["config", "user.email", "test@example.invalid"]).unwrap();
            fs::write(path.join("example.txt"), "original\n").unwrap();
            run_git(&path, &["add", "."]).unwrap();
            run_git(&path, &["commit", "-m", "initial"]).unwrap();
            Self(path)
        }
        fn path(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }
        fn read(&self, kind: &str, first: &str, second: &str) -> Value {
            repository_read(
                self.path(),
                kind.into(),
                Some(first.into()),
                Some(second.into()),
            )
            .unwrap()
        }
        fn action(&self, kind: &str, first: &str, second: &str) {
            repository_action(
                self.path(),
                kind.into(),
                Some(first.into()),
                Some(second.into()),
            )
            .unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            // Only remove the exact uniquely-created fixture directory.
            assert!(self.0.starts_with(std::env::temp_dir()));
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn diff_and_stash_round_trip() {
        let repo = Fixture::new();
        let commit = repo.read("commit", "HEAD", "");
        assert!(commit["diff"].as_str().unwrap().contains("+original"));
        fs::write(repo.0.join("example.txt"), "staged\n").unwrap();
        run_git(&repo.0, &["add", "example.txt"]).unwrap();
        fs::write(repo.0.join("example.txt"), "unstaged\n").unwrap();
        fs::write(repo.0.join("new.txt"), "new content\n").unwrap();
        let diff = repo.read("worktree", "example.txt", "");
        assert!(diff["diff"].as_str().unwrap().contains("+staged"));
        assert!(diff["diff"].as_str().unwrap().contains("+unstaged"));
        assert!(repo.read("worktree", "new.txt", "")["diff"]
            .as_str()
            .unwrap()
            .contains("new content"));
        assert!(repository_read(
            repo.path(),
            "worktree".into(),
            Some("../outside".into()),
            None
        )
        .is_err());
        repo.action("stash-save", "test stash", "");
        assert!(run_git(&repo.0, &["status", "--porcelain"])
            .unwrap()
            .is_empty());
        let entries = repo.read("stashes", "", "");
        let hash = entries[0]["hash"].as_str().unwrap();
        assert!(repo.read("stash", hash, "")["diff"]
            .as_str()
            .unwrap()
            .contains("new content"));
        repo.action("stash-apply", hash, "");
        assert_eq!(
            fs::read_to_string(repo.0.join("example.txt")).unwrap(),
            "unstaged\n"
        );
        assert!(repo.0.join("new.txt").exists());
        repo.action("stash-drop", hash, "");
        assert_eq!(repo.read("stashes", "", ""), json!([]));
    }
    #[test]
    fn branch_tags_compare_and_publish() {
        let repo = Fixture::new();
        run_git(&repo.0, &["tag", "v1.0.0"]).unwrap();
        assert_eq!(repo.read("tags", "", ""), json!(["v1.0.0"]));
        repo.action("tag-branch", "v1.0.0", "feature");
        repo.action("rename", "feature", "renamed");
        assert!(repository_action(
            repo.path(),
            "rename".into(),
            Some("renamed".into()),
            Some("--bad".into())
        )
        .is_err());
        fs::write(repo.0.join("example.txt"), "changed\n").unwrap();
        run_git(&repo.0, &["commit", "-am", "changed"]).unwrap();
        let compare = repo.read("compare", "refs/heads/renamed", "refs/heads/main");
        assert!(compare["diff"].as_str().unwrap().contains("+changed"));
        assert!(compare["message"].as_str().unwrap().contains("changed"));
        let bare = repo.0.join("remote.git");
        fs::create_dir(&bare).unwrap();
        run_git(&bare, &["init", "--bare"]).unwrap();
        run_git(
            &repo.0,
            &["remote", "add", "origin", bare.to_str().unwrap()],
        )
        .unwrap();
        repo.action("publish", "main", "origin");
        repo.action("upstream", "renamed", "origin/main");
        assert_eq!(
            run_git(
                &repo.0,
                &["rev-parse", "--abbrev-ref", "renamed@{upstream}"]
            )
            .unwrap(),
            "origin/main"
        );
        repo.action("unset-upstream", "renamed", "");
        assert!(run_git(
            &repo.0,
            &["rev-parse", "--abbrev-ref", "renamed@{upstream}"]
        )
        .is_err());
    }
}

#[tauri::command]
pub fn get_runtime_info(app: AppHandle) -> Value {
    json!({"version": app.package_info().version.to_string(), "gitVersion": run_git(&std::env::temp_dir(), &["--version"]).unwrap_or_else(|error| error)})
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RepositoryGroup {
    id: String,
    name: String,
    color: String,
    members: Vec<String>,
}
pub(crate) fn validate_groups(groups: &[RepositoryGroup]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    if groups.len() > 200 {
        return Err("最多支持 200 个分组".into());
    }
    for group in groups {
        if group.id.is_empty()
            || !ids.insert(&group.id)
            || group.name.trim().is_empty()
            || group.name.chars().count() > 50
            || !names.insert(group.name.trim().to_lowercase())
        {
            return Err("分组名称或标识无效、重复，名称最多 50 个字符".into());
        }
        if !(group.color.len() == 7 && group.color.starts_with('#') && group.color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())) && !["primary", "success", "warning", "info", "muted"].contains(&group.color.as_str()) {
            return Err("无效的分组颜色".into());
        }
        if group.members.len() > 10000 {
            return Err("分组成员数量过多".into());
        }
    }
    Ok(())
}
#[tauri::command]
pub fn read_repository_groups() -> Result<Vec<RepositoryGroup>, String> {
    let groups: Vec<RepositoryGroup> = serde_json::from_value(storage::migrate("groups", &local_file("data", "repository-groups.json")?)?.unwrap_or_else(|| json!([]))).map_err(|e|e.to_string())?;
    validate_groups(&groups)?;
    Ok(groups)
}
#[tauri::command]
pub fn save_repository_groups(groups: Vec<RepositoryGroup>) -> Result<(), String> {
    validate_groups(&groups)?;
    storage::write("groups", &serde_json::to_value(groups).map_err(|e|e.to_string())?)
}

#[cfg(test)]
mod group_tests {
    use super::*;
    #[test]
    fn groups_round_trip_and_validate_multiple_membership() {
        let raw = r#"[{"id":"a","name":"我的产品","color":"primary","members":["repo1","repo2"]},{"id":"b","name":"正在维护","color":"success","members":["repo1"]}]"#;
        let groups: Vec<RepositoryGroup> = serde_json::from_str(raw).unwrap();
        assert!(validate_groups(&groups).is_ok());
        let saved = serde_json::to_vec(&groups).unwrap();
        let restored: Vec<RepositoryGroup> = serde_json::from_slice(&saved).unwrap();
        assert_eq!(restored[0].members, vec!["repo1", "repo2"]);
        assert_eq!(restored[1].members, vec!["repo1"]);
        assert_eq!(restored[0].name, "我的产品");
    }
    #[test]
    fn groups_reject_duplicates_and_invalid_names() {
        let mut groups = vec![RepositoryGroup {
            id: "one".into(),
            name: "产品".into(),
            color: "primary".into(),
            members: vec![],
        }];
        groups.push(groups[0].clone());
        assert!(validate_groups(&groups).is_err());
        groups.pop();
        groups[0].name = " ".into();
        assert!(validate_groups(&groups).is_err());
        groups[0].name = "产品".into();
        groups[0].color = "invalid".into();
        assert!(validate_groups(&groups).is_err());
    }
}
