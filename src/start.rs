use crate::process::pid_exists;
use crate::session_store::{Session, SessionStore};
use crate::skim;
use crate::ui;
use anyhow::{Context, Result, bail};
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
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

pub fn start(tex_file: PathBuf, verbose: bool) -> Result<()> {
    let store = SessionStore::livetex_cache();
    remove_stale_sessions_or_fail_if_running(&store, &tex_file, verbose)?;

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
        match skim::open_pdf(&session.pdf_file) {
            Ok(preview) => {
                store.record_skim_document_path(&session.id, &preview.document_path)?;

                match preview.document_open_confirmed {
                    Some(false) => {
                        ui::warning(
                            "Opened PDF in Skim, but could not confirm the document window.",
                        );
                    }
                    None => {
                        ui::warning(
                            "Opened PDF in Skim, but AppleScript monitoring could not be confirmed.",
                        );
                    }
                    Some(true) => {}
                }

                if let Err(error) =
                    spawn_monitor(&session.id, preview.document_open_confirmed == Some(true))
                {
                    ui::warning(format!(
                        "Started compiler, but could not monitor Skim: {error:#}"
                    ));
                }

                match preview.app_pid {
                    Some(pid) => {
                        store.record_skim_pid(&session.id, pid)?;
                        Some(pid)
                    }
                    None => {
                        ui::warning("Started compiler, but could not confirm the Skim process.");
                        None
                    }
                }
            }
            Err(error) => {
                ui::warning(format!(
                    "Started latexmk, but could not open Skim: {error:#}"
                ));
                None
            }
        }
    } else {
        ui::warning(
            "Started compiler, but the PDF was not created within 30 seconds. Run `livetex logs` for details.",
        );
        None
    };

    ui::started_session(&session, latexmk_pid, skim_pid, verbose);

    Ok(())
}

fn spawn_monitor(session_id: &str, seen_open: bool) -> Result<()> {
    let live_tex = std::env::current_exe().with_context(|| "Could not determine LiveTex binary")?;
    let mut command = Command::new(live_tex);
    command.arg("monitor").arg(session_id);

    if seen_open {
        command.arg("--seen-open");
    }

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| "Could not start LiveTex monitor")?;

    Ok(())
}

fn remove_stale_sessions_or_fail_if_running(
    store: &SessionStore,
    tex_file: &Path,
    verbose: bool,
) -> Result<()> {
    for session in store.sessions_for_tex_file(tex_file)? {
        match session.latexmk_pid {
            Some(pid) if pid_exists(pid) => {
                let tex_file = session.tex_file.unwrap_or_else(|| tex_file.to_path_buf());

                if verbose {
                    bail!("Active session already exists for {tex_file:?} with latexmk PID {pid}");
                }

                bail!(
                    "Live preview already running for {}",
                    ui::path_label(&tex_file)
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
