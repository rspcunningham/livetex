use crate::process::{pid_exists, terminate_process, terminate_process_group};
use crate::session_store::{SessionStore, SessionSummary};
use anyhow::Result;
use std::path::Path;

pub fn active_sessions(store: &SessionStore) -> Result<Vec<SessionSummary>> {
    Ok(store
        .list_sessions()?
        .into_iter()
        .filter(is_active_session)
        .collect())
}

pub fn active_sessions_for_tex_file(
    store: &SessionStore,
    tex_file: &Path,
) -> Result<Vec<SessionSummary>> {
    Ok(store
        .sessions_for_tex_file(tex_file)?
        .into_iter()
        .filter(is_active_session)
        .collect())
}

pub fn is_active_session(session: &SessionSummary) -> bool {
    session.latexmk_pid.is_some_and(pid_exists)
}

pub fn stop_session(store: &SessionStore, session: &SessionSummary) -> Result<()> {
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

    Ok(())
}

pub fn format_session_summary(session: &SessionSummary) -> String {
    let tex_file = session
        .tex_file
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    let pdf_file = session
        .pdf_file
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    let latexmk_pid = session
        .latexmk_pid
        .map(|pid| pid.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    let skim_pid = session
        .skim_pid
        .map(|pid| pid.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    format!(
        "{}\n  tex: {}\n  pdf: {}\n  latexmk pid: {}\n  skim pid: {}\n  dir: {:?}\n  log: {:?}",
        session.id,
        tex_file,
        pdf_file,
        latexmk_pid,
        skim_pid,
        session.session_dir,
        session.log_path
    )
}

pub fn format_session_picker_label(session: &SessionSummary) -> String {
    let tex_file = session
        .tex_file
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    let latexmk_pid = session
        .latexmk_pid
        .map(|pid| pid.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    format!(
        "{tex_file}  [latexmk pid: {latexmk_pid}, session: {}]",
        session.id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn formats_missing_session_fields_as_unknown() {
        let session = SessionSummary {
            id: "session-id".to_string(),
            tex_file: None,
            pdf_file: None,
            latexmk_pid: None,
            skim_pid: None,
            session_dir: PathBuf::from("/tmp/livetex/session-id"),
            log_path: PathBuf::from("/tmp/livetex/session-id/latexmk.log"),
        };

        assert_eq!(
            format_session_summary(&session),
            "session-id\n  tex: <unknown>\n  pdf: <unknown>\n  latexmk pid: <unknown>\n  skim pid: <unknown>\n  dir: \"/tmp/livetex/session-id\"\n  log: \"/tmp/livetex/session-id/latexmk.log\""
        );
    }

    #[test]
    fn formats_picker_label_on_one_line() {
        let session = SessionSummary {
            id: "session-id".to_string(),
            tex_file: Some(PathBuf::from("/tmp/project/main.tex")),
            pdf_file: None,
            latexmk_pid: Some(1234),
            skim_pid: None,
            session_dir: PathBuf::from("/tmp/livetex/session-id"),
            log_path: PathBuf::from("/tmp/livetex/session-id/latexmk.log"),
        };

        assert_eq!(
            format_session_picker_label(&session),
            "/tmp/project/main.tex  [latexmk pid: 1234, session: session-id]"
        );
    }
}
