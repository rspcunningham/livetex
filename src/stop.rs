use crate::process::{pid_exists, terminate_process, terminate_process_group};
use crate::session_store::SessionStore;
use anyhow::{Result, bail};
use std::path::PathBuf;

pub fn stop(tex_file: PathBuf) -> Result<()> {
    let store = SessionStore::livetex_cache();
    let sessions = store.sessions_for_tex_file(&tex_file)?;

    if sessions.is_empty() {
        bail!("No active session found for {:?}", tex_file);
    }

    for session in sessions {
        if let Some(latexmk_pid) = session.latexmk_pid {
            if pid_exists(latexmk_pid) {
                terminate_process_group(latexmk_pid)?;

                if pid_exists(latexmk_pid) {
                    terminate_process(latexmk_pid)?;
                }

                println!("stopped latexmk process {latexmk_pid}");
            }
        }

        if let Some(skim_pid) = session.skim_pid {
            if pid_exists(skim_pid) {
                terminate_process(skim_pid)?;
                println!("stopped Skim process {skim_pid}");
            }
        }

        store.stop_session(&session.id)?;

        println!("stopped session {}", session.id);
    }

    Ok(())
}
