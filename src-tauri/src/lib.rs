use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Manager};

const IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    ".idea",
    ".next",
    ".nuxt",
    ".venv",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
    "venv",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRoot {
    #[serde(default)]
    pub kind: Option<String>,
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddRootResult {
    pub roots: Vec<ScanRoot>,
    pub added: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSummary {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositorySummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub scan_root: String,
    pub current_branch: Option<String>,
    pub is_detached: bool,
    pub upstream: Option<String>,
    pub remotes: Vec<RemoteSummary>,
    pub local_branches: Vec<String>,
    pub remote_branches: Vec<String>,
    pub ahead: u32,
    pub behind: u32,
    pub is_dirty: bool,
    pub has_conflicts: bool,
    pub operation_in_progress: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanIssue {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSnapshot {
    pub roots: Vec<ScanRoot>,
    pub repositories: Vec<RepositorySummary>,
    pub issues: Vec<ScanIssue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchCommit {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
    pub author: String,
    pub committed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFile {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub conflicted: bool,
}

fn data_root() -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        Ok(base.join("RepoDeck-dev"))
    } else {
        std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .map(Path::to_path_buf)
            .ok_or("无法定位安装目录".into())
    }
}
fn local_file(folder: &str, name: &str) -> Result<PathBuf, String> {
    let directory = data_root()?.join(folder);
    fs::create_dir_all(&directory).map_err(|e| {
        format!(
            "数据目录不可写 {}，请将应用安装到可写目录：{e}",
            directory.display()
        )
    })?;
    Ok(directory.join(name))
}
fn atomic_write(file: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = file.with_extension("tmp");
    fs::write(&temporary, contents).map_err(|e| e.to_string())?;
    fs::rename(temporary, file).map_err(|e| e.to_string())
}
fn roots_file(app: &AppHandle) -> Result<PathBuf, String> {
    let file = local_file("data", "scan-roots.json")?;
    if !file.exists() {
        let current = app.path().app_config_dir().map_err(|e| e.to_string())?;
        for legacy in [
            current.join("scan-roots.json"),
            current
                .parent()
                .unwrap_or(&current)
                .join("com.gitscope.app")
                .join("scan-roots.json"),
        ] {
            if legacy.exists() {
                let bytes = fs::read(&legacy).map_err(|e| e.to_string())?;
                serde_json::from_slice::<Vec<ScanRoot>>(&bytes)
                    .map_err(|e| format!("旧目录数据损坏，已保留原文件：{e}"))?;
                atomic_write(&file, &bytes)?;
                break;
            }
        }
    }
    Ok(file)
}
#[tauri::command]
fn read_preferences() -> Result<Option<serde_json::Value>, String> {
    let file = local_file("setting", "preferences.json")?;
    if !file.exists() {
        return Ok(None);
    }
    let bytes = fs::read(file).map_err(|e| e.to_string())?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("偏好文件损坏，已保留原文件：{e}"))?;
    if !value.is_object() {
        return Err("偏好文件格式无效，已保留原文件".into());
    }
    Ok(Some(value))
}
#[tauri::command]
fn save_preferences(preferences: serde_json::Value) -> Result<(), String> {
    atomic_write(
        &local_file("setting", "preferences.json")?,
        &serde_json::to_vec_pretty(&preferences).map_err(|e| e.to_string())?,
    )
}
#[tauri::command]
fn get_data_location() -> Result<String, String> {
    Ok(data_root()?.to_string_lossy().into_owned())
}

fn read_roots(app: &AppHandle) -> Result<Vec<ScanRoot>, String> {
    let file = roots_file(app)?;
    if !file.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read(&file).map_err(|error| format!("无法读取扫描目录设置：{error}"))?;
    serde_json::from_slice(&contents).map_err(|error| format!("扫描目录设置格式无效：{error}"))
}

fn write_roots(app: &AppHandle, roots: &[ScanRoot]) -> Result<(), String> {
    let file = roots_file(app)?;
    let contents = serde_json::to_vec_pretty(roots)
        .map_err(|error| format!("无法整理扫描目录设置：{error}"))?;
    atomic_write(&file, &contents)
}

#[tauri::command]
fn get_scan_roots(app: AppHandle) -> Result<Vec<ScanRoot>, String> {
    read_roots(&app)
}

#[tauri::command]
fn add_scan_root(
    path: String,
    kind: Option<String>,
    app: AppHandle,
) -> Result<AddRootResult, String> {
    let selected_path = PathBuf::from(path);
    let canonical_path = selected_path
        .canonicalize()
        .map_err(|error| format!("无法访问所选目录：{error}"))?;
    if !canonical_path.is_dir() {
        return Err("请选择一个目录。".into());
    }

    let path = canonical_path.to_string_lossy().into_owned();
    let mut roots = read_roots(&app)?;
    let added = !roots.iter().any(|root| paths_equal(&root.path, &path));
    if added {
        let kind = kind.unwrap_or_else(|| {
            if has_git_metadata(&canonical_path) {
                "repository".into()
            } else {
                "workspace".into()
            }
        });
        if kind != "workspace" && kind != "repository" {
            return Err("无效的目录类型".into());
        }
        if kind == "repository" && !has_git_metadata(&canonical_path) {
            return Err("请选择 Git 仓库根目录".into());
        }
        roots.push(ScanRoot {
            path,
            kind: Some(kind),
        });
        roots.sort_by(|left, right| left.path.to_lowercase().cmp(&right.path.to_lowercase()));
        write_roots(&app, &roots)?;
    }

    Ok(AddRootResult { roots, added })
}

#[tauri::command]
fn remove_scan_root(path: String, app: AppHandle) -> Result<Vec<ScanRoot>, String> {
    let comparison_path = PathBuf::from(&path)
        .canonicalize()
        .map(|canonical| canonical.to_string_lossy().into_owned())
        .unwrap_or(path);
    let mut roots = read_roots(&app)?;
    roots.retain(|root| !paths_equal(&root.path, &comparison_path));
    write_roots(&app, &roots)?;
    Ok(roots)
}

#[tauri::command]
fn scan_repositories(app: AppHandle) -> Result<ScanSnapshot, String> {
    let roots = read_roots(&app)?;
    let mut candidates: BTreeMap<String, (PathBuf, String)> = BTreeMap::new();
    let mut issues = Vec::new();

    for root in &roots {
        let root_path = PathBuf::from(&root.path);
        if !root_path.is_dir() {
            issues.push(ScanIssue {
                path: root.path.clone(),
                message: "目录不存在或无法访问。".into(),
            });
            continue;
        }
        if root.kind.as_deref() == Some("repository") {
            if has_git_metadata(&root_path) {
                candidates.insert(path_key(&root.path), (root_path, root.path.clone()));
            } else {
                issues.push(ScanIssue {
                    path: root.path.clone(),
                    message: "该目录已不是 Git 仓库".into(),
                });
            }
        } else {
            scan_directory(&root_path, &root.path, 0, &mut candidates, &mut issues);
        }
    }

    let mut repositories = Vec::new();
    for (_, (path, scan_root)) in candidates {
        match inspect_repository(&path, scan_root) {
            Ok(repository) => repositories.push(repository),
            Err(message) => issues.push(ScanIssue {
                path: path.to_string_lossy().into_owned(),
                message,
            }),
        }
    }

    repositories.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then(left.path.to_lowercase().cmp(&right.path.to_lowercase()))
    });

    Ok(ScanSnapshot {
        roots,
        repositories,
        issues,
    })
}

fn scan_directory(
    path: &Path,
    scan_root: &str,
    depth: usize,
    candidates: &mut BTreeMap<String, (PathBuf, String)>,
    issues: &mut Vec<ScanIssue>,
) {
    if has_git_metadata(path) {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let display_path = canonical_path.to_string_lossy().into_owned();
        candidates
            .entry(path_key(&display_path))
            .or_insert((canonical_path, scan_root.to_string()));
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            issues.push(ScanIssue {
                path: path.to_string_lossy().into_owned(),
                message: format!("无法读取目录：{error}"),
            });
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                issues.push(ScanIssue {
                    path: path.to_string_lossy().into_owned(),
                    message: format!("无法读取目录项：{error}"),
                });
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                issues.push(ScanIssue {
                    path: entry.path().to_string_lossy().into_owned(),
                    message: format!("无法检查目录项：{error}"),
                });
                continue;
            }
        };

        // Symlinks are intentionally not followed, avoiding cycles and out-of-root scans.
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        let child_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if IGNORED_DIRECTORIES.contains(&name.as_str()) {
            continue;
        }

        scan_directory(&child_path, scan_root, depth + 1, candidates, issues);
    }
}

fn has_git_metadata(path: &Path) -> bool {
    path.join(".git").exists()
}

fn inspect_repository(path: &Path, scan_root: String) -> Result<RepositorySummary, String> {
    let status = run_git(
        path,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=normal",
        ],
    )?;

    let mut current_branch = None;
    let mut is_detached = false;
    let mut upstream = None;
    let mut ahead = 0;
    let mut behind = 0;
    let mut is_dirty = false;

    for line in status.lines() {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            if value == "(detached)" {
                is_detached = true;
            } else if value != "(initial)" {
                current_branch = Some(value.to_string());
            }
        } else if let Some(value) = line.strip_prefix("# branch.upstream ") {
            upstream = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("# branch.ab ") {
            let mut counts = value.split_whitespace();
            ahead = counts
                .next()
                .and_then(|count| count.strip_prefix('+'))
                .and_then(|count| count.parse().ok())
                .unwrap_or(0);
            behind = counts
                .next()
                .and_then(|count| count.strip_prefix('-'))
                .and_then(|count| count.parse().ok())
                .unwrap_or(0);
        } else if !line.is_empty() && !line.starts_with('#') {
            is_dirty = true;
        }
    }
    let has_conflicts = !run_git(path, &["ls-files", "-u"])?.is_empty();
    let operation_in_progress = git_operation_in_progress(path)?;

    let local_branches = git_lines(path, &["branch", "--format=%(refname:short)"])?;
    let remote_branches: Vec<String> =
        git_lines(path, &["branch", "--remotes", "--format=%(refname:short)"])?
            .into_iter()
            .filter(|branch| !branch.ends_with("/HEAD"))
            .collect();
    if upstream
        .as_ref()
        .is_some_and(|branch| !remote_branches.iter().any(|remote| remote == branch))
    {
        upstream = None;
    }

    let remotes = git_lines(path, &["remote"])?
        .into_iter()
        .map(|name| {
            let url = run_git(path, &["remote", "get-url", &name])
                .map(|url| redact_remote_url(&url))
                .unwrap_or_default();
            RemoteSummary { name, url }
        })
        .collect();

    let display_path = path.to_string_lossy().into_owned();
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| display_path.clone());

    Ok(RepositorySummary {
        id: path_key(&display_path),
        name,
        path: display_path,
        scan_root,
        current_branch,
        is_detached,
        upstream,
        remotes,
        local_branches,
        remote_branches,
        ahead,
        behind,
        is_dirty,
        has_conflicts,
        operation_in_progress,
    })
}

fn git_lines(path: &Path, args: &[&str]) -> Result<Vec<String>, String> {
    Ok(run_git(path, args)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

// Every Git operation goes through this builder, including staging and commits.
#[tauri::command]
fn open_repository_target(target: String, local: bool) -> Result<(), String> {
    let target = if local {
        let path = PathBuf::from(&target)
            .canonicalize()
            .map_err(|e| format!("无法访问目录：{e}"))?;
        if !path.is_dir() || !has_git_metadata(&path) {
            return Err("该目录不是 Git 仓库".into());
        }
        let value = path.to_string_lossy().into_owned();
        if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else {
            value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
        }
    } else {
        let url = tauri::Url::parse(&target).map_err(|_| "无效的远程地址")?;
        if !matches!(url.scheme(), "https" | "http")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("仅支持 HTTP 或 HTTPS 仓库地址".into());
        }
        url.to_string()
    };
    #[cfg(windows)]
    let mut command = Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");
    command
        .arg(target)
        .spawn()
        .map_err(|e| format!("打开失败：{e}"))?;
    Ok(())
}

fn git_command(path: &Path) -> Command {
    let mut command = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.arg("-C").arg(path);
    command
}

fn run_git(path: &Path, args: &[&str]) -> Result<String, String> {
    let output = git_command(path).args(args).output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "找不到 Git。请先安装 Git，并确保 git 命令已加入 PATH。".to_string()
        } else {
            format!("无法启动 Git：{error}")
        }
    })?;

    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if details.is_empty() {
            "Git 无法读取此仓库。".into()
        } else {
            details
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

fn redact_remote_url(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://") {
        let authority_end = rest.find('/').unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        let suffix = &rest[authority_end..];
        let safe_authority = authority
            .rsplit_once('@')
            .map(|(_, host)| host)
            .unwrap_or(authority);
        return format!("{scheme}://{safe_authority}{suffix}");
    }

    if let Some((user, host_and_path)) = url.split_once('@') {
        if !user.contains('/')
            && !user.contains('\\')
            && !user.contains(':')
            && host_and_path.contains(':')
        {
            return host_and_path.to_string();
        }
    }

    url.to_string()
}

fn canonical_repository(path: &str) -> Result<PathBuf, String> {
    let canonical = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("无法访问仓库目录：{error}"))?;
    if !has_git_metadata(&canonical) {
        return Err("所选目录不是 Git 仓库。".into());
    }
    Ok(canonical)
}

#[tauri::command]
fn get_repository_tree(
    path: String,
    relative_path: Option<String>,
) -> Result<Vec<TreeEntry>, String> {
    let repository = canonical_repository(&path)?;
    let directory = match relative_path.as_deref().filter(|value| !value.is_empty()) {
        Some(relative) => {
            let relative_path = Path::new(relative);
            if relative_path.is_absolute()
                || relative_path.components().any(|part| {
                    matches!(
                        part,
                        std::path::Component::ParentDir | std::path::Component::Prefix(_)
                    )
                })
            {
                return Err("目录路径无效。".into());
            }
            let candidate = repository
                .join(relative_path)
                .canonicalize()
                .map_err(|error| format!("无法访问目录：{error}"))?;
            if !candidate.starts_with(&repository) || !candidate.is_dir() {
                return Err("目录不在此仓库内。".into());
            }
            candidate
        }
        None => repository.clone(),
    };

    let mut entries = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|error| format!("无法读取目录：{error}"))?
    {
        let entry = entry.map_err(|error| format!("无法读取目录项：{error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("无法检查目录项：{error}"))?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" || IGNORED_DIRECTORIES.contains(&name.to_lowercase().as_str()) {
            continue;
        }
        let full_path = entry.path();
        let relative = full_path
            .strip_prefix(&repository)
            .map_err(|_| "目录不在此仓库内。".to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        entries.push(TreeEntry {
            name,
            path: relative,
            is_directory: file_type.is_dir(),
        });
    }
    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then(left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
fn get_branch_commits(
    path: String,
    branch: String,
    limit: Option<u32>,
) -> Result<Vec<BranchCommit>, String> {
    let repository = canonical_repository(&path)?;
    let local_branches = git_lines(&repository, &["branch", "--format=%(refname:short)"])?;
    let remote_branches = git_lines(
        &repository,
        &["branch", "--remotes", "--format=%(refname:short)"],
    )?;
    let reference = if branch == "HEAD" {
        if run_git(&repository, &["rev-parse", "--verify", "HEAD"]).is_err() {
            return Ok(Vec::new());
        }
        "HEAD".to_string()
    } else if local_branches.iter().any(|candidate| candidate == &branch) {
        format!("refs/heads/{branch}")
    } else if remote_branches
        .iter()
        .any(|candidate| candidate == &branch && !candidate.ends_with("/HEAD"))
    {
        format!("refs/remotes/{branch}")
    } else {
        return Err("所选分支不存在或已被删除。".into());
    };
    let limit = limit.unwrap_or(10).clamp(1, 100).to_string();
    let format = "%H%x1f%h%x1f%s%x1f%an%x1f%aI";
    let output = run_git(
        &repository,
        &[
            "log",
            "-n",
            &limit,
            &format!("--format={format}"),
            &reference,
        ],
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\u{1f}');
            Some(BranchCommit {
                hash: fields.next()?.to_string(),
                short_hash: fields.next()?.to_string(),
                subject: fields.next()?.to_string(),
                author: fields.next()?.to_string(),
                committed_at: fields.next()?.to_string(),
            })
        })
        .collect())
}

#[tauri::command]
fn get_worktree_changes(path: String) -> Result<Vec<ChangedFile>, String> {
    let repository = canonical_repository(&path)?;
    let output = run_git(
        &repository,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    Ok(output
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let code = &line[..2];
            let file_path = line[3..].to_string();
            Some(ChangedFile {
                path: file_path,
                status: code.to_string(),
                staged: code
                    .chars()
                    .next()
                    .is_some_and(|value| value != ' ' && value != '?'),
                unstaged: code
                    .chars()
                    .nth(1)
                    .is_some_and(|value| value != ' ' && value != '?'),
                untracked: code == "??",
                conflicted: code.contains('U') || code == "AA" || code == "DD",
            })
        })
        .collect())
}

fn validate_changed_paths(repository: &Path, requested: &[String]) -> Result<Vec<String>, String> {
    if requested.is_empty() {
        return Err("请至少选择一个变更文件。".into());
    }
    let changes = get_worktree_changes(repository.to_string_lossy().into_owned())?;
    let available: std::collections::HashSet<String> =
        changes.into_iter().map(|change| change.path).collect();
    let mut selected = Vec::new();
    for path in requested {
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir | std::path::Component::Prefix(_)
                )
            })
            || !available.contains(path)
        {
            return Err(format!("变更文件已变化或路径无效：{path}"));
        }
        if !selected.contains(path) {
            selected.push(path.clone());
        }
    }
    Ok(selected)
}

#[tauri::command]
fn stage_files(path: String, files: Vec<String>) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    if has_unmerged_paths(&repository)? || git_operation_in_progress(&repository)? {
        return Err("Git 正在进行合并或变基，请先在 IDE 或终端完成该操作。".into());
    }
    let files = validate_changed_paths(&repository, &files)?;
    let mut command = git_command(&repository);
    command.arg("add").arg("--").args(files);
    let output = command
        .output()
        .map_err(|error| format!("无法启动 Git：{error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

#[tauri::command]
fn commit_files(path: String, files: Vec<String>, message: String) -> Result<(), String> {
    let message = message.trim();
    if message.is_empty() {
        return Err("请填写提交说明。".into());
    }
    let repository = canonical_repository(&path)?;
    if has_unmerged_paths(&repository)? || git_operation_in_progress(&repository)? {
        return Err("Git 正在进行合并或变基，请先在 IDE 或终端完成该操作。".into());
    }
    let files = validate_changed_paths(&repository, &files)?;
    let mut add = git_command(&repository);
    add.arg("add").arg("--").args(&files);
    let add_output = add
        .output()
        .map_err(|error| format!("无法启动 Git：{error}"))?;
    if !add_output.status.success() {
        return Err(String::from_utf8_lossy(&add_output.stderr)
            .trim()
            .to_string());
    }
    let mut command = git_command(&repository);
    command
        .arg("commit")
        .arg("--only")
        .arg("-m")
        .arg(message)
        .arg("--")
        .args(files);
    let output = command
        .output()
        .map_err(|error| format!("无法启动 Git：{error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

#[tauri::command]
fn fetch_repository(path: String) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    run_git(&repository, &["fetch", "--all"])?;
    Ok(())
}

#[tauri::command]
fn pull_fast_forward(path: String) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    let snapshot = inspect_repository(&repository, repository.to_string_lossy().into_owned())?;
    if snapshot.operation_in_progress {
        return Err("Git 正在进行合并或变基，请先在 IDE 或终端完成该操作。".into());
    }
    if snapshot.is_dirty {
        return Err("工作区有未提交改动，请先审阅并处理后再拉取。".into());
    }
    if snapshot.upstream.is_none() {
        return Err("当前分支没有配置远端跟踪分支。".into());
    }
    if snapshot.ahead != 0 || snapshot.behind == 0 {
        return Err("当前分支不是可以直接快进的状态，请刷新并检查分支关系。".into());
    }
    run_git(&repository, &["merge", "--ff-only", "@{upstream}"])?;
    Ok(())
}

#[tauri::command]
fn push_current_branch(path: String) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    let snapshot = inspect_repository(&repository, repository.to_string_lossy().into_owned())?;
    if snapshot.operation_in_progress {
        return Err("Git 正在进行合并或变基，请先在 IDE 或终端完成该操作。".into());
    }
    if snapshot.is_detached {
        return Err("当前处于 detached HEAD，无法推送当前分支。".into());
    }
    if snapshot.upstream.is_none() {
        return Err("当前分支没有配置远端跟踪分支。请先在 Git 工具中设置 upstream。".into());
    }
    run_git(&repository, &["push"])?;
    Ok(())
}

#[tauri::command]
fn integrate_upstream(path: String, strategy: String) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    let snapshot = inspect_repository(&repository, repository.to_string_lossy().into_owned())?;
    if snapshot.operation_in_progress {
        return Err("Git 正在进行合并或变基，请先在 IDE 或终端完成该操作。".into());
    }
    if snapshot.is_dirty {
        return Err("工作区有未提交改动，请先处理后再整合远端提交。".into());
    }
    if snapshot.upstream.is_none() {
        return Err("当前分支没有配置远端跟踪分支。".into());
    }
    if snapshot.ahead == 0 || snapshot.behind == 0 {
        return Err("当前分支尚未与远端分叉，请刷新后重新检查。".into());
    }
    match strategy.as_str() {
        "merge" => {
            run_git(&repository, &["merge", "--no-edit", "@{upstream}"])?;
        }
        "rebase" => {
            run_git(&repository, &["rebase", "@{upstream}"])?;
        }
        _ => return Err("不支持的分支整合方式。".into()),
    }
    Ok(())
}

#[tauri::command]
fn checkout_branch(
    path: String,
    branch: String,
    remote_branch: Option<String>,
) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    let snapshot = inspect_repository(&repository, repository.to_string_lossy().into_owned())?;
    if snapshot.operation_in_progress {
        return Err("Git 正在进行合并或变基，请先在 IDE 或终端完成该操作。".into());
    }
    if snapshot.is_dirty {
        return Err("工作区有未提交改动，请先处理后再切换分支。".into());
    }
    if let Some(remote_branch) = remote_branch {
        if !snapshot.remote_branches.contains(&remote_branch) {
            return Err("所选远端分支不存在或已被删除，请先 Fetch。".into());
        }
        if snapshot.local_branches.contains(&branch) {
            return Err("该本地分支已存在，请选择已有本地分支，或填写其他名称。".into());
        }
        // Validate a literal name before passing it to Git's branch creation option.
        if branch.is_empty()
            || branch.starts_with('-')
            || branch == "HEAD"
            || branch.trim() != branch
            || run_git(
                &repository,
                &["check-ref-format", &format!("refs/heads/{branch}")],
            )
            .is_err()
        {
            return Err("本地分支名无效，请使用有效的 Git 分支名，例如 feature/design。".into());
        }
        let remote_ref = format!("refs/remotes/{remote_branch}");
        run_git(
            &repository,
            &["switch", "--track", "--create", &branch, "--", &remote_ref],
        )?;
        return Ok(());
    }
    if !snapshot.local_branches.contains(&branch) {
        return Err("本地分支不存在。".into());
    }
    run_git(&repository, &["switch", "--", &branch])?;
    Ok(())
}

#[tauri::command]
fn create_branch(
    path: String,
    branch: String,
    start_ref: String,
    switch_after: bool,
) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    let snapshot = inspect_repository(&repository, repository.to_string_lossy().into_owned())?;
    if snapshot.operation_in_progress || snapshot.has_conflicts {
        return Err("请先完成当前 Git 操作并解决冲突。".into());
    }
    if switch_after && snapshot.is_dirty {
        return Err("工作区有未提交改动；请先处理，或取消“创建后切换”。".into());
    }
    if snapshot.local_branches.contains(&branch) {
        return Err("该本地分支已存在，请使用其他名称。".into());
    }
    if branch.is_empty()
        || branch.starts_with('-')
        || branch == "HEAD"
        || branch.trim() != branch
        || run_git(
            &repository,
            &["check-ref-format", &format!("refs/heads/{branch}")],
        )
        .is_err()
    {
        return Err("本地分支名无效，例如可使用 feature/login。".into());
    }
    validate_branch_ref(&snapshot, &start_ref)?;
    run_git(
        &repository,
        &["rev-parse", "--verify", &format!("{start_ref}^{{commit}}")],
    )
    .map_err(|_| "起点没有有效提交，请先创建首次提交或 Fetch。".to_string())?;
    let track = if start_ref.starts_with("refs/remotes/") {
        "--track"
    } else {
        "--no-track"
    };
    if switch_after {
        run_git(
            &repository,
            &["switch", track, "--create", &branch, "--", &start_ref],
        )?;
    } else {
        run_git(&repository, &["branch", track, "--", &branch, &start_ref])?;
    }
    Ok(())
}

fn validate_branch_ref(snapshot: &RepositorySummary, reference: &str) -> Result<(), String> {
    if reference == "HEAD"
        || reference
            .strip_prefix("refs/heads/")
            .is_some_and(|name| snapshot.local_branches.iter().any(|branch| branch == name))
        || reference
            .strip_prefix("refs/remotes/")
            .is_some_and(|name| snapshot.remote_branches.iter().any(|branch| branch == name))
    {
        Ok(())
    } else {
        Err("所选分支不存在，请刷新后重新选择。".into())
    }
}

#[tauri::command]
fn merge_branch(path: String, source_ref: String, target_branch: String) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    let snapshot = inspect_repository(&repository, repository.to_string_lossy().into_owned())?;
    if snapshot.is_dirty || snapshot.has_conflicts || snapshot.operation_in_progress {
        return Err("请先处理工作区改动、冲突和未完成的 Git 操作，再合并。".into());
    }
    if snapshot.current_branch.as_deref() != Some(target_branch.as_str()) {
        return Err("当前分支已变化，请刷新并重新确认合并目标。".into());
    }
    validate_branch_ref(&snapshot, &source_ref)?;
    if source_ref == "HEAD" || source_ref == format!("refs/heads/{target_branch}") {
        return Err("不能将当前分支合并到自身。".into());
    }
    run_git(&repository, &["merge", "--no-edit", "--", &source_ref]).map_err(|reason| {
        format!("合并未完成：{reason}。如存在冲突，请在 IDE 或终端解决后刷新。")
    })?;
    Ok(())
}

#[tauri::command]
fn delete_local_branch(path: String, branch: String) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    let snapshot = inspect_repository(&repository, repository.to_string_lossy().into_owned())?;
    if snapshot.operation_in_progress || snapshot.has_conflicts {
        return Err("请先完成当前 Git 操作并解决冲突。".into());
    }
    if snapshot.current_branch.as_deref() == Some(branch.as_str()) {
        return Err("不能删除当前分支，请先切换到其他分支。".into());
    }
    if !snapshot.local_branches.contains(&branch) {
        return Err("本地分支已不存在，请刷新。".into());
    }
    let reference = format!("refs/heads/{branch}");
    if run_git(
        &repository,
        &["merge-base", "--is-ancestor", &reference, "HEAD"],
    )
    .is_err()
    {
        return Err("此分支有尚未合并到当前 HEAD 的提交，未执行删除；请先确认并合并。".into());
    }
    run_git(&repository, &["branch", "--delete", "--", &branch])?;
    Ok(())
}

#[tauri::command]
fn set_upstream(path: String, remote_branch: String) -> Result<(), String> {
    let repository = canonical_repository(&path)?;
    let snapshot = inspect_repository(&repository, repository.to_string_lossy().into_owned())?;
    if snapshot.operation_in_progress {
        return Err("Git 正在进行合并或变基，请先在 IDE 或终端完成该操作。".into());
    }
    let current = snapshot
        .current_branch
        .ok_or_else(|| "当前处于 detached HEAD，无法设置 upstream。".to_string())?;
    if !snapshot
        .remote_branches
        .iter()
        .any(|branch| branch == &remote_branch)
    {
        return Err("所选远端分支不存在或已被删除，请先 Fetch。".into());
    }
    let upstream_arg = format!("--set-upstream-to=refs/remotes/{remote_branch}");
    let mut command = git_command(&repository);
    command
        .arg("branch")
        .arg(upstream_arg)
        .arg("--")
        .arg(current);
    let output = command
        .output()
        .map_err(|error| format!("无法启动 Git：{error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

fn has_unmerged_paths(repository: &Path) -> Result<bool, String> {
    Ok(!run_git(repository, &["ls-files", "-u"])?.is_empty())
}

fn git_operation_in_progress(repository: &Path) -> Result<bool, String> {
    for marker in [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "BISECT_LOG",
        "rebase-apply",
        "rebase-merge",
        "sequencer",
    ] {
        let git_path = run_git(repository, &["rev-parse", "--git-path", marker])?;
        let git_path = PathBuf::from(git_path);
        let resolved = if git_path.is_absolute() {
            git_path
        } else {
            repository.join(git_path)
        };
        if resolved.exists() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn path_key(path: &str) -> String {
    #[cfg(windows)]
    {
        path.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        path.to_string()
    }
}

fn paths_equal(left: &str, right: &str) -> bool {
    path_key(left) == path_key(right)
}

#[derive(Clone, Serialize, Deserialize)]
struct WindowState {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}
fn restore_window(window: &tauri::WebviewWindow) -> Option<WindowState> {
    let file = local_file("setting", "window.json").ok()?;
    let mut state: WindowState = serde_json::from_slice(&fs::read(file).ok()?).ok()?;
    let monitors = window.available_monitors().ok()?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let area = monitor.work_area();
            state.x >= area.position.x
                && state.y >= area.position.y
                && (state.x as i64) < area.position.x as i64 + area.size.width as i64
                && (state.y as i64) < area.position.y as i64 + area.size.height as i64
        })
        .or_else(|| monitors.first())?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    state.width = state.width.max((720.0 * scale) as u32).min(area.size.width);
    state.height = state
        .height
        .max((480.0 * scale) as u32)
        .min(area.size.height);
    state.x = state.x.clamp(
        area.position.x,
        area.position.x + (area.size.width - state.width) as i32,
    );
    state.y = state.y.clamp(
        area.position.y,
        area.position.y + (area.size.height - state.height) as i32,
    );
    window
        .set_size(tauri::PhysicalSize::new(state.width, state.height))
        .ok()?;
    window
        .set_position(tauri::PhysicalPosition::new(state.x, state.y))
        .ok()?;
    if state.maximized {
        window.maximize().ok()?;
    }
    Some(state)
}
fn remember_window(window: &tauri::WebviewWindow, restored: Option<WindowState>) {
    let state = restored.or_else(|| {
        let position = window.outer_position().ok()?;
        let size = window.inner_size().ok()?;
        Some(WindowState {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
            maximized: false,
        })
    });
    let state = std::sync::Arc::new(std::sync::Mutex::new(state));
    let handle = window.clone();
    window.on_window_event(move |event| {
        if matches!(
            event,
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
        ) {
            if !handle.is_maximized().unwrap_or(true) && !handle.is_minimized().unwrap_or(true) {
                if let (Ok(position), Ok(size), Ok(mut stored)) =
                    (handle.outer_position(), handle.inner_size(), state.lock())
                {
                    if let Some(value) = stored.as_mut() {
                        value.x = position.x;
                        value.y = position.y;
                        value.width = size.width;
                        value.height = size.height;
                    }
                }
            }
            return;
        }
        if !matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
            return;
        }
        if let Ok(mut stored) = state.lock() {
            if let Some(state) = stored.as_mut() {
                state.maximized = handle.is_maximized().unwrap_or(false);
                if !state.maximized && !handle.is_minimized().unwrap_or(false) {
                    if let (Ok(position), Ok(size)) = (handle.outer_position(), handle.inner_size())
                    {
                        state.x = position.x;
                        state.y = position.y;
                        state.width = size.width;
                        state.height = size.height;
                    }
                }
                if let (Ok(file), Ok(bytes)) = (
                    local_file("setting", "window.json"),
                    serde_json::to_vec(state),
                ) {
                    if let Err(error) = atomic_write(&file, &bytes) {
                        eprintln!("无法保存窗口状态：{error}");
                    }
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let area = monitor.work_area();
                    let scale = monitor.scale_factor();
                    let size = initial_window_size(area.size, scale);
                    let minimum = tauri::PhysicalSize::new(
                        (720.0 * scale).round().min(size.width as f64) as u32,
                        (480.0 * scale).round().min(size.height as f64) as u32,
                    );
                    window.set_min_size(Some(minimum))?;
                    window.set_size(size)?;
                    window.set_position(tauri::PhysicalPosition::new(
                        area.position.x + ((area.size.width - size.width) / 2) as i32,
                        area.position.y + ((area.size.height - size.height) / 2) as i32,
                    ))?;
                }
                let restored = restore_window(&window);
                remember_window(&window, restored);
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_preferences,
            save_preferences,
            get_data_location,
            open_repository_target,
            get_scan_roots,
            add_scan_root,
            remove_scan_root,
            scan_repositories,
            get_repository_tree,
            get_branch_commits,
            get_worktree_changes,
            stage_files,
            commit_files,
            fetch_repository,
            pull_fast_forward,
            push_current_branch,
            checkout_branch,
            create_branch,
            merge_branch,
            delete_local_branch,
            integrate_upstream,
            set_upstream
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}

fn initial_window_size(area: tauri::PhysicalSize<u32>, scale: f64) -> tauri::PhysicalSize<u32> {
    tauri::PhysicalSize::new(
        (1080.0 * scale)
            .round()
            .min((area.width as f64 * 0.92).floor())
            .max(1.0) as u32,
        (700.0 * scale)
            .round()
            .min((area.height as f64 * 0.92).floor())
            .max(1.0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::initial_window_size;
    use tauri::PhysicalSize;

    #[test]
    fn initial_window_fits_small_screen_with_display_scaling() {
        let size = initial_window_size(PhysicalSize::new(1366, 728), 1.25);
        assert_eq!(size, PhysicalSize::new(1256, 669));
    }

    #[test]
    fn initial_window_uses_compact_default_on_large_screen() {
        let size = initial_window_size(PhysicalSize::new(2560, 1400), 1.5);
        assert_eq!(size, PhysicalSize::new(1620, 1050));
    }
    #[test]
    fn scan_finds_deep_and_nested_repositories_but_skips_dependencies() {
        use super::*;
        let root = std::env::temp_dir().join(format!("repodeck-scan-{}", std::process::id()));
        let deep = root.join("a/b/c/d/e/f/project");
        fs::create_dir_all(deep.join(".git")).unwrap();
        fs::create_dir_all(deep.join("nested/.git")).unwrap();
        fs::create_dir_all(root.join("node_modules/dependency/.git")).unwrap();
        let mut candidates = BTreeMap::new();
        let mut issues = Vec::new();
        scan_directory(&root, "workspace", 0, &mut candidates, &mut issues);
        assert_eq!(candidates.len(), 2);
        assert!(issues.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn legacy_roots_keep_their_paths_and_allow_missing_kind() {
        let roots: Vec<super::ScanRoot> = serde_json::from_str(r#"[{"path":"E:/Code"}]"#).unwrap();
        assert_eq!(roots[0].path, "E:/Code");
        assert!(roots[0].kind.is_none());
    }
}
