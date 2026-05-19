use crate::process::pid_exists;
use crate::session_store::{Session, SessionStore};
use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

fn spawn_latexmk(session: &Session) -> Result<u32> {
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&session.log_path)?;

    let stderr = stdout.try_clone()?;

    let workdir = session
        .tex_file
        .parent()
        .with_context(|| format!("Could not determine workdir for {:?}", session.tex_file))?;
    let tex_file_name = session
        .tex_file
        .file_name()
        .with_context(|| format!("Could not determine file name for {:?}", session.tex_file))?;

    let mut command = Command::new("latexmk");
    command
        .arg("-pdf")
        .arg("-pvc")
        .arg("-interaction=nonstopmode")
        .arg("-synctex=1")
        .arg("-view=none")
        .arg(format!("-outdir={}", session.session_dir.display()))
        .arg(tex_file_name)
        .current_dir(workdir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    #[cfg(unix)]
    {
        command.process_group(0);
    }

    let child = command
        .spawn()
        .with_context(|| "Could not start latexmk. Is latexmk installed and on PATH?")?;

    Ok(child.id())
}

pub fn start(tex_file: PathBuf) -> Result<()> {
    let store = SessionStore::livetex_cache();
    remove_stale_sessions_or_fail_if_running(&store, &tex_file)?;

    let session = store.start_session(tex_file)?;
    let latexmk_pid = match spawn_latexmk(&session) {
        Ok(pid) => pid,
        Err(error) => {
            let _ = store.stop_session(&session.id);
            return Err(error);
        }
    };

    store.record_latexmk_pid(&session.id, latexmk_pid)?;

    let skim_pid = if wait_for_pdf(&session.pdf_file, Duration::from_secs(30)) {
        match open_with_skim(&session.pdf_file) {
            Ok(Some(pid)) => {
                store.record_skim_pid(&session.id, pid)?;
                Some(pid)
            }
            Ok(None) => {
                eprintln!("Started latexmk, but could not determine Skim PID.");
                None
            }
            Err(error) => {
                eprintln!("Started latexmk, but could not open Skim: {error:#}");
                None
            }
        }
    } else {
        eprintln!(
            "Started latexmk, but {:?} was not created within 30 seconds. See logs for details.",
            session.pdf_file
        );
        None
    };

    println!("Started LiveTeX session");
    println!("Session ID: {}", session.id);
    println!("TeX file: {:?}", session.tex_file);
    println!("PDF file: {:?}", session.pdf_file);
    println!("Session directory: {:?}", session.session_dir);
    println!("Log file: {:?}", session.log_path);
    println!("latexmk PID: {}", latexmk_pid);

    if let Some(skim_pid) = skim_pid {
        println!("Skim PID: {}", skim_pid);
    }

    Ok(())
}

fn remove_stale_sessions_or_fail_if_running(store: &SessionStore, tex_file: &Path) -> Result<()> {
    for session in store.sessions_for_tex_file(tex_file)? {
        match session.latexmk_pid {
            Some(pid) if pid_exists(pid) => {
                bail!(
                    "Active session already exists for {:?} with latexmk PID {}",
                    session.tex_file.unwrap_or_else(|| tex_file.to_path_buf()),
                    pid
                );
            }
            _ => store.stop_session(&session.id)?,
        }
    }

    Ok(())
}

fn wait_for_pdf(pdf_file: &Path, timeout: Duration) -> bool {
    let interval = Duration::from_millis(250);
    let mut elapsed = Duration::ZERO;

    while elapsed < timeout {
        if pdf_file.exists() {
            return true;
        }

        thread::sleep(interval);
        elapsed += interval;
    }

    pdf_file.exists()
}

fn open_with_skim(pdf_file: &Path) -> Result<Option<u32>> {
    let pids_before = skim_pids()?;
    let status = Command::new("open")
        .arg("-a")
        .arg("Skim")
        .arg(pdf_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| "Could not run macOS open command")?;

    ensure_success(status, "Could not open PDF with Skim")?;

    thread::sleep(Duration::from_millis(500));

    let pids_after = skim_pids()?;
    let pids_before: HashSet<u32> = pids_before.into_iter().collect();

    Ok(pids_after
        .iter()
        .copied()
        .find(|pid| !pids_before.contains(pid))
        .or_else(|| pids_after.into_iter().max()))
}

fn skim_pids() -> Result<Vec<u32>> {
    let output = Command::new("pgrep")
        .arg("-x")
        .arg("Skim")
        .output()
        .with_context(|| "Could not run pgrep")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(stdout
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect())
}

fn ensure_success(status: ExitStatus, message: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("{message}: {status}")
    }
}
