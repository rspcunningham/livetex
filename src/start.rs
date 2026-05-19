use crate::session_store::SessionStore;
use anyhow::Result;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn spawn_worker(absolute_path: &Path, log_path: &Path) -> Result<u32> {
    let current_exe = std::env::current_exe()?;

    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let stderr = stdout.try_clone()?;

    let child = Command::new(current_exe)
        .arg("worker")
        .arg(absolute_path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()?;

    Ok(child.id())
}

pub fn start(tex_file: PathBuf) -> Result<()> {
    let store = SessionStore::livetex_cache();
    let session = store.start_session(tex_file)?;
    let worker_pid = spawn_worker(&session.tex_file, &session.log_path)?;
    store.record_worker_pid(&session.id, worker_pid)?;

    println!(
        "Starting compile with absolute path: {:?}",
        session.tex_file
    );
    println!("Session ID: {}", session.id);
    println!("Session directory: {:?}", session.session_dir);
    println!("Log file: {:?}", session.log_path);
    println!("PID file: {:?}", session.worker_pid_path);
    println!("Worker PID: {}", worker_pid);

    Ok(())
}
