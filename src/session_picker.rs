use crate::session_lifecycle::{active_sessions, format_session_picker_label};
use crate::session_store::{SessionStore, SessionSummary};
use anyhow::{Context, Result, bail};
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::IsTerminal;

pub fn select_active_session(store: &SessionStore) -> Result<Option<SessionSummary>> {
    let sessions = active_sessions(store)?;

    if sessions.is_empty() {
        bail!("No active sessions.");
    }

    ensure_interactive_terminal()?;

    let labels: Vec<String> = sessions.iter().map(format_session_picker_label).collect();
    let selected_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Stop which LiveTeX session?")
        .items(&labels)
        .default(0)
        .interact_opt()
        .with_context(|| "Could not read session selection")?;

    Ok(selected_index.map(|index| sessions[index].clone()))
}

fn ensure_interactive_terminal() -> Result<()> {
    if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
        return Ok(());
    }

    bail!(
        "Cannot select a session without an interactive terminal; pass a .tex file to `livetex stop <path_to_latex_file>`"
    );
}
