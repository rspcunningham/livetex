use crate::session_lifecycle::stop_session;
use crate::session_picker::select_active_session;
use crate::session_store::SessionStore;
use anyhow::{Result, bail};
use std::path::PathBuf;

pub fn stop(tex_file: Option<PathBuf>, verbose: bool) -> Result<()> {
    let store = SessionStore::livetex_cache();
    let sessions = match tex_file {
        Some(tex_file) => {
            let sessions = store.sessions_for_tex_file(&tex_file)?;

            if sessions.is_empty() {
                bail!("No active session found for {tex_file:?}");
            }

            sessions
        }
        None => match select_active_session(&store, verbose)? {
            Some(session) => vec![session],
            None => return Ok(()),
        },
    };

    for session in sessions {
        stop_session(&store, &session, verbose)?;
    }

    Ok(())
}
