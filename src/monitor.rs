use crate::process::pid_exists;
use crate::session_lifecycle::stop_compile_session;
use crate::session_store::{SessionStore, SessionSummary};
use crate::skim;
use anyhow::Result;
use std::thread;
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub fn monitor(session_id: String, seen_open: bool, verbose: bool) -> Result<()> {
    let store = SessionStore::livetex_cache();
    let mut seen_document_open = seen_open;

    loop {
        let Some(session) = store.session(&session_id)? else {
            return Ok(());
        };

        if !session.latexmk_pid.is_some_and(pid_exists) {
            store.stop_session(&session.id)?;
            return Ok(());
        }

        if let Some(document_path) = preview_document_path(&session) {
            match skim::document_is_open(&document_path) {
                Ok(true) => {
                    seen_document_open = true;
                }
                Ok(false) if seen_document_open => {
                    stop_compile_session(&store, &session, verbose)?;
                    return Ok(());
                }
                Ok(false) => {}
                Err(_) => {}
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn preview_document_path(session: &SessionSummary) -> Option<std::path::PathBuf> {
    session
        .skim_document_path
        .clone()
        .or_else(|| session.pdf_file.clone())
}
