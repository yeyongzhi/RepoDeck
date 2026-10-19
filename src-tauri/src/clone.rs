use serde::Serialize;
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{mpsc, Arc, Mutex, OnceLock},
    time::Duration,
};
use tauri::{ipc::Channel, AppHandle};

fn validate(url: &str, name: &str) -> Result<(), String> {
    let supported = url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("ssh://")
        || (url.contains('@') && url.contains(':') && !url.contains("://"));
    if !supported || url.chars().any(char::is_whitespace) || url.starts_with('-') {
        return Err("请输入 HTTPS 或 SSH Git 仓库地址".into());
    }
    if name.is_empty()
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || name == "."
        || name == ".."
    {
        return Err("请输入有效的项目目录名，不能包含路径分隔符或特殊字符".into());
    }
    let stem = name.split('.').next().unwrap_or("").to_uppercase();
    if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.ends_with(['1', '2', '3', '4', '5', '6', '7', '8', '9']))
    {
        return Err("项目目录名是系统保留名称，请更换".into());
    }
    Ok(())
}

#[derive(Default)]
struct Control {
    stop: Option<bool>,
    finished: bool,
}
type Controls = Mutex<HashMap<String, Arc<Mutex<Control>>>>;
fn controls() -> &'static Controls {
    static CONTROLS: OnceLock<Controls> = OnceLock::new();
    CONTROLS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[tauri::command]
pub fn stop_clone(id: String, delete_files: bool) -> Result<(), String> {
    let control = controls()
        .lock()
        .map_err(|e| e.to_string())?
        .get(&id)
        .cloned()
        .ok_or("任务已结束，请查看结果")?;
    let mut state = control.lock().map_err(|e| e.to_string())?;
    if state.finished {
        return Err("任务已完成，无法停止".into());
    }
    if state.stop.is_none() {
        state.stop = Some(delete_files);
    }
    Ok(())
}

// Start Git suspended, assign it to a job, then resume it. All descendants
// inherit the job, so stopping cannot leave remote helpers or checkout running.
#[cfg(windows)]
mod process_job {
    use std::{ffi::c_void, os::windows::io::AsRawHandle, process::Child};
    type Handle = *mut c_void;
    #[repr(C)]
    struct ThreadEntry {
        size: u32,
        usage: u32,
        id: u32,
        owner: u32,
        priority: i32,
        delta: i32,
        flags: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> Handle;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn TerminateJobObject(job: Handle, code: u32) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
        fn CreateToolhelp32Snapshot(flags: u32, process: u32) -> Handle;
        fn Thread32First(snapshot: Handle, entry: *mut ThreadEntry) -> i32;
        fn Thread32Next(snapshot: Handle, entry: *mut ThreadEntry) -> i32;
        fn OpenThread(access: u32, inherit: i32, id: u32) -> Handle;
        fn ResumeThread(thread: Handle) -> u32;
    }
    pub struct Job(Handle);
    impl Job {
        pub fn attach(child: &Child) -> Result<Self, String> {
            // SAFETY: all handles are checked, struct layout follows THREADENTRY32,
            // and the child is suspended until assignment completes.
            unsafe {
                let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if handle.is_null() {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                let job = Self(handle);
                if AssignProcessToJobObject(handle, child.as_raw_handle()) == 0 {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                let snapshot = CreateToolhelp32Snapshot(4, 0);
                if snapshot == -1isize as Handle {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                let mut entry = ThreadEntry {
                    size: std::mem::size_of::<ThreadEntry>() as u32,
                    usage: 0,
                    id: 0,
                    owner: 0,
                    priority: 0,
                    delta: 0,
                    flags: 0,
                };
                let mut found = Thread32First(snapshot, &mut entry);
                let mut resumed = false;
                while found != 0 {
                    if entry.owner == child.id() {
                        let thread = OpenThread(2, 0, entry.id);
                        if !thread.is_null() {
                            resumed = ResumeThread(thread) != u32::MAX;
                            CloseHandle(thread);
                        }
                        break;
                    }
                    found = Thread32Next(snapshot, &mut entry);
                }
                CloseHandle(snapshot);
                if !resumed {
                    return Err("无法启动 Git 工作线程".into());
                }
                Ok(job)
            }
        }
        pub fn terminate(&self) -> Result<(), String> {
            // SAFETY: this object owns a live job handle.
            if unsafe { TerminateJobObject(self.0, 1) } == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(())
        }
    }
    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                TerminateJobObject(self.0, 1);
                CloseHandle(self.0);
            }
        }
    }
}

fn kill_tree(
    child: &mut std::process::Child,
    #[cfg(windows)] job: &process_job::Job,
) -> Result<(), String> {
    #[cfg(windows)]
    job.terminate()?;
    #[cfg(not(windows))]
    child.kill().map_err(|e| e.to_string())?;
    child.wait().map_err(|e| e.to_string())?;
    Ok(())
}

// Only remove the new directory reserved by this task, never a pre-existing path.
fn cleanup_target(parent: &Path, target: &Path) -> Result<(), String> {
    if !target.exists() {
        return Ok(());
    }
    let metadata = std::fs::symlink_metadata(target).map_err(|e| e.to_string())?;
    let resolved = target.canonicalize().map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() || resolved.parent() != Some(parent) || resolved != target
    {
        return Err("目标路径发生变化，已保留文件，请手动检查".into());
    }
    #[cfg(windows)]
    fn writable(path: &Path) -> Result<(), String> {
        let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            return Ok(());
        }
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
                writable(&entry.map_err(|e| e.to_string())?.path())?;
            }
        } else if metadata.permissions().readonly() {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            std::fs::set_permissions(path, permissions).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[cfg(windows)]
    writable(&resolved)?;
    std::fs::remove_dir_all(&resolved)
        .map_err(|e| format!("已停止，但删除失败，文件保留在 {}：{e}", target.display()))
}

fn clone_into(
    url: &str,
    parent: &std::path::Path,
    target: &std::path::Path,
    progress: impl Fn(String),
    control: &Arc<Mutex<Control>>,
) -> Result<(), String> {
    // Git for Windows cannot create a worktree using a verbatim (\\?\) target.
    // -C already selects the canonical parent, so pass only the project name.
    let relative_target = target
        .strip_prefix(parent)
        .map_err(|_| "目标目录必须位于存放目录内")?;
    if relative_target.components().count() != 1
        || !matches!(
            relative_target.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err("目标目录必须是存放目录的直接子目录".into());
    }
    let mut command = super::git_command(parent);
    command
        .args(["clone", "--progress", "--", url])
        .arg(relative_target)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        .env(
            "GIT_SSH_COMMAND",
            "ssh -o BatchMode=yes -o StrictHostKeyChecking=yes",
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000 | 0x00000004);
    }
    let mut child = command.spawn().map_err(|e| format!("无法启动 Git：{e}"))?;
    #[cfg(windows)]
    let job = match process_job::Job::attach(&child) {
        Ok(job) => job,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("无法创建克隆进程组：{e}"));
        }
    };
    let mut stderr = child.stderr.take().ok_or("无法读取克隆进度")?;
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut buffer = [0u8; 2048];
        loop {
            match stderr.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(Ok(buffer[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                    break;
                }
            }
        }
    });
    let mut tail = String::new();
    loop {
        if control.lock().map_err(|e| e.to_string())?.stop.is_some() {
            kill_tree(
                &mut child,
                #[cfg(windows)]
                &job,
            )?;
            let _ = reader.join();
            return Ok(());
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(bytes)) => {
                let chunk = String::from_utf8_lossy(&bytes).replace(url, "[远程仓库]");
                progress(chunk.clone());
                tail.push_str(&chunk);
                if tail.len() > 12000 {
                    tail = tail
                        .chars()
                        .rev()
                        .take(6000)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                }
            }
            Ok(Err(e)) => {
                kill_tree(
                    &mut child,
                    #[cfg(windows)]
                    &job,
                )?;
                let _ = reader.join();
                return Err(e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = reader.join();
    let status = child.wait().map_err(|e| e.to_string())?;
    if control.lock().map_err(|e| e.to_string())?.stop.is_some() {
        return Ok(());
    }
    if !status.success() {
        return Err(format!("克隆失败：{}", tail.trim()));
    }
    Ok(())
}

#[tauri::command]
pub fn read_clone_directories() -> Result<Vec<String>, String> {
    serde_json::from_value(super::storage::migrate("directories", &super::local_file("data", "clone-directories.json")?)?.unwrap_or_else(|| serde_json::json!([]))).map_err(|e|e.to_string())
}

#[tauri::command]
pub fn remember_clone_directory(path: String) -> Result<Vec<String>, String> {
    let path = PathBuf::from(path)
        .canonicalize()
        .map_err(|e| format!("无法访问目录：{e}"))?;
    if !path.is_dir() {
        return Err("请选择目录".into());
    }
    let path = path.to_string_lossy().into_owned();
    let mut directories = read_clone_directories()?;
    directories.retain(|item| !item.eq_ignore_ascii_case(&path));
    directories.insert(0, path);
    directories.truncate(8);
    super::storage::write("directories", &serde_json::to_value(&directories).map_err(|e| e.to_string())?)?;
    Ok(directories)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneResult {
    path: String,
    stopped: bool,
    deleted: bool,
}

#[tauri::command]
pub async fn clone_repository(
    id: String,
    url: String,
    parent: String,
    name: String,
    register: bool,
    progress: Channel<String>,
    app: AppHandle,
) -> Result<CloneResult, String> {
    let control = Arc::new(Mutex::new(Control::default()));
    {
        let mut active = controls().lock().map_err(|e| e.to_string())?;
        if !active.is_empty() {
            return Err("已有克隆任务正在执行".into());
        }
        active.insert(id.clone(), control.clone());
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        let url = url.trim();
        let name = name.trim();
        validate(url, name)?;
        let parent = PathBuf::from(parent)
            .canonicalize()
            .map_err(|e| format!("无法访问存放目录：{e}"))?;
        if !parent.is_dir() {
            return Err("请选择有效的存放目录".into());
        }
        let target = parent.join(name);
        // Atomic reservation ensures cancellation can only delete this task's files.
        std::fs::create_dir(&target)
            .map_err(|e| format!("无法创建项目目录（目录可能已存在）：{e}"))?;
        let cloned = clone_into(
            url,
            &parent,
            &target,
            |chunk| {
                let _ = progress.send(chunk);
            },
            &control,
        );
        let stop = {
            let mut state = control.lock().map_err(|e| e.to_string())?;
            state.finished = true;
            state.stop
        };
        let path = target.to_string_lossy().into_owned();
        if let Some(delete) = stop {
            // A process termination failure must never be followed by deletion.
            cloned?;
            if delete {
                cleanup_target(&parent, &target)?;
            }
            return Ok(CloneResult {
                path,
                stopped: true,
                deleted: delete,
            });
        }
        cloned?;
        if register {
            super::add_scan_root(path.clone(), Some("repository".into()), app).map_err(|e| {
                format!(
                    "代码已克隆到 {}，自动登记失败：{e}。可从本地添加。",
                    target.display()
                )
            })?;
        }
        Ok(CloneResult {
            path,
            stopped: false,
            deleted: false,
        })
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    controls().lock().map_err(|e| e.to_string())?.remove(&id);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_remote_and_single_directory_name() {
        for url in [
            "https://github.com/org/repo.git",
            "git@github.com:org/repo.git",
            "ssh://git@example.com/repo",
        ] {
            assert!(validate(url, "my-project").is_ok());
        }
        for name in [
            "../escape",
            "C:\\repo",
            "..",
            "CON",
            "lpt1.txt",
            "trailing.",
        ] {
            assert!(validate("https://example.com/repo", name).is_err());
        }
        assert!(validate("--upload-pack=malicious", "repo").is_err());
        assert!(validate("file:///tmp/repo", "repo").is_err());
    }
    #[test]
    fn clones_repository_reports_progress_and_preserves_existing_target() {
        let root = std::env::temp_dir().join(format!(
            "repodeck-clone-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        super::super::run_git(&source, &["init"]).unwrap();
        std::fs::write(source.join("README.md"), "clone integration test").unwrap();
        super::super::run_git(&source, &["add", "."]).unwrap();
        super::super::run_git(
            &source,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "initial",
            ],
        )
        .unwrap();
        // Match production: canonicalize() produces a \\?\ path on Windows.
        let parent = root.canonicalize().unwrap();
        let target = parent.join("克隆项目 with spaces");
        std::fs::create_dir(&target).unwrap();
        let output = std::sync::Mutex::new(String::new());
        clone_into(
            source.to_str().unwrap(),
            &parent,
            &target,
            |chunk| output.lock().unwrap().push_str(&chunk),
            &Arc::new(Mutex::new(Control::default())),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("README.md")).unwrap(),
            "clone integration test"
        );
        assert!(!output.lock().unwrap().is_empty());
        assert!(clone_into(
            source.to_str().unwrap(),
            &parent,
            &target,
            |_| {},
            &Arc::new(Mutex::new(Control::default()))
        )
        .is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("README.md")).unwrap(),
            "clone integration test"
        );
        assert!(root.starts_with(std::env::temp_dir()));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stop_terminates_git_and_supports_keep_or_delete() {
        use std::io::Read;
        use std::net::TcpListener;
        for delete in [false, true] {
            let root = std::env::temp_dir().join(format!(
                "repodeck-stop-test-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&root).unwrap();
            let parent = root.canonicalize().unwrap();
            let target = parent.join("partial");
            std::fs::create_dir(&target).unwrap();
            let control = Arc::new(Mutex::new(Control::default()));
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/repo.git", listener.local_addr().unwrap());
            let request_control = control.clone();
            let partial_target = target.clone();
            let server = std::thread::spawn(move || {
                listener.set_nonblocking(true).unwrap();
                let deadline = std::time::Instant::now() + Duration::from_secs(8);
                let mut stream = loop {
                    if let Ok((stream, _)) = listener.accept() {
                        break stream;
                    }
                    assert!(std::time::Instant::now() < deadline, "Git did not connect");
                    std::thread::sleep(Duration::from_millis(20));
                };
                std::fs::write(partial_target.join("partial.bin"), "downloaded bytes").unwrap();
                request_control.lock().unwrap().stop = Some(delete);
                stream
                    .set_read_timeout(Some(Duration::from_secs(4)))
                    .unwrap();
                let mut bytes = [0; 1024];
                while let Ok(n) = stream.read(&mut bytes) {
                    if n == 0 {
                        break;
                    }
                }
            });
            let started = std::time::Instant::now();
            clone_into(&url, &parent, &target, |_| {}, &control).unwrap();
            assert!(
                started.elapsed() < Duration::from_secs(4),
                "stop should interrupt a stalled transfer"
            );
            server.join().unwrap();
            assert!(target.join("partial.bin").exists());
            if delete {
                let mut permissions = std::fs::metadata(target.join("partial.bin"))
                    .unwrap()
                    .permissions();
                permissions.set_readonly(true);
                std::fs::set_permissions(target.join("partial.bin"), permissions).unwrap();
                cleanup_target(&parent, &target).unwrap();
                assert!(!target.exists());
            } else {
                assert_eq!(
                    std::fs::read_to_string(target.join("partial.bin")).unwrap(),
                    "downloaded bytes"
                );
            }
            assert!(cleanup_target(&parent, &parent).is_err());
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}
