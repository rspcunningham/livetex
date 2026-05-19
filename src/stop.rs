use crate::session_store::SessionStore;
use anyhow::{Result, bail};
use nix::errno::Errno;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::path::PathBuf;

pub fn stop(tex_file: PathBuf) -> Result<()> {
    let store = SessionStore::livetex_cache();
    let sessions = store.sessions_for_tex_file(&tex_file)?;

    if sessions.is_empty() {
        bail!("No active session found for {:?}", tex_file);
    }

    for session in sessions {
        let worker_pid = session
            .worker_pid
            .ok_or_else(|| anyhow::anyhow!("Session {} has no worker PID", session.id))?;

        kill_worker(worker_pid)?;
        store.stop_session(&session.id)?;

        println!("stopped session {}", session.id);
        println!("stopped worker process {worker_pid}");
    }

    Ok(())
}

fn kill_worker(worker_pid: u32) -> Result<()> {
    if worker_pid > i32::MAX as u32 {
        bail!("Invalid worker PID: {}", worker_pid);
    }

    let pid = Pid::from_raw(worker_pid as i32);

    match kill(pid, Signal::SIGTERM) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(error) => bail!("Could not stop worker process {}: {}", worker_pid, error),
    }
}
