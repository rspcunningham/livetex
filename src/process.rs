use anyhow::{Result, bail};
use nix::errno::Errno;
use nix::sys::signal::{Signal, kill, killpg};
use nix::unistd::Pid;

pub fn pid_exists(pid: u32) -> bool {
    if pid > i32::MAX as u32 {
        return false;
    }

    match kill(Pid::from_raw(pid as i32), None) {
        Ok(()) | Err(Errno::EPERM) => true,
        Err(Errno::ESRCH) => false,
        Err(_) => false,
    }
}

pub fn terminate_process(pid: u32) -> Result<()> {
    if pid > i32::MAX as u32 {
        bail!("Invalid PID: {pid}");
    }

    match kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(error) => bail!("Could not stop process {pid}: {error}"),
    }
}

pub fn terminate_process_group(process_group_id: u32) -> Result<()> {
    if process_group_id > i32::MAX as u32 {
        bail!("Invalid process group ID: {process_group_id}");
    }

    match killpg(Pid::from_raw(process_group_id as i32), Signal::SIGTERM) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(error) => bail!("Could not stop process group {process_group_id}: {error}"),
    }
}
