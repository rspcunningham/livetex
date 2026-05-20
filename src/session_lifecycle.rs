use crate::process::{pid_exists, terminate_process, terminate_process_group};
use crate::session_store::{SessionStore, SessionSummary};
use crate::ui;
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

pub fn stop_session(store: &SessionStore, session: &SessionSummary, verbose: bool) -> Result<()> {
    terminate_compile_process(session, verbose)?;

    if let Some(skim_pid) = session.skim_pid {
        if pid_exists(skim_pid) {
            terminate_process(skim_pid)?;

            if verbose {
                ui::stopped_skim(skim_pid);
            }
        }
    }

    store.stop_session(&session.id)?;

    ui::stopped_session(session, verbose);

    Ok(())
}

pub fn stop_compile_session(
    store: &SessionStore,
    session: &SessionSummary,
    verbose: bool,
) -> Result<()> {
    terminate_compile_process(session, verbose)?;
    store.stop_session(&session.id)?;

    if verbose {
        ui::stopped_session(session, verbose);
    }

    Ok(())
}

fn terminate_compile_process(session: &SessionSummary, verbose: bool) -> Result<()> {
    if let Some(latexmk_pid) = session.latexmk_pid {
        if pid_exists(latexmk_pid) {
            terminate_process_group(latexmk_pid)?;

            if pid_exists(latexmk_pid) {
                terminate_process(latexmk_pid)?;
            }

            if verbose {
                ui::stopped_latexmk(latexmk_pid);
            }
        }
    }

    Ok(())
}

pub fn format_session_picker_label(session: &SessionSummary, verbose: bool) -> String {
    let tex_file = session
        .tex_file
        .as_ref()
        .map(|path| {
            if verbose {
                path.display().to_string()
            } else {
                ui::path_label(path)
            }
        })
        .unwrap_or_else(|| "<unknown>".to_string());

    if !verbose {
        return tex_file;
    }

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
    fn formats_picker_label_on_one_line() {
        let session = SessionSummary {
            id: "session-id".to_string(),
            tex_file: Some(PathBuf::from("/tmp/project/main.tex")),
            pdf_file: None,
            latexmk_pid: Some(1234),
            skim_pid: None,
            skim_document_path: None,
            session_dir: PathBuf::from("/tmp/livetex/session-id"),
            log_path: PathBuf::from("/tmp/livetex/session-id/latexmk.log"),
        };

        assert_eq!(
            format_session_picker_label(&session, true),
            "/tmp/project/main.tex  [latexmk pid: 1234, session: session-id]"
        );
    }

    #[test]
    fn formats_picker_label_concisely_by_default() {
        let session = SessionSummary {
            id: "session-id".to_string(),
            tex_file: Some(PathBuf::from("/tmp/project/main.tex")),
            pdf_file: None,
            latexmk_pid: Some(1234),
            skim_pid: None,
            skim_document_path: None,
            session_dir: PathBuf::from("/tmp/livetex/session-id"),
            log_path: PathBuf::from("/tmp/livetex/session-id/latexmk.log"),
        };

        assert_eq!(format_session_picker_label(&session, false), "main.tex");
    }
}
