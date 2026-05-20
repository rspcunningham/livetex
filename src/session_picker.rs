use crate::session_lifecycle::{active_sessions, format_session_picker_label};
use crate::session_store::{SessionStore, SessionSummary};
use anyhow::{Context, Result, bail};
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::IsTerminal;

pub fn select_active_session(
    store: &SessionStore,
    verbose: bool,
) -> Result<Option<SessionSummary>> {
    select_active_session_with_prompt(
        store,
        verbose,
        "Stop which LiveTeX session?",
        "pass a .tex file to `livetex stop <path_to_latex_file>`",
    )
}

pub fn select_active_session_with_prompt(
    store: &SessionStore,
    verbose: bool,
    prompt: &str,
    non_interactive_hint: &str,
) -> Result<Option<SessionSummary>> {
    let sessions = active_sessions(store)?;

    if sessions.is_empty() {
        bail!("No active sessions.");
    }

    ensure_interactive_terminal(non_interactive_hint)?;

    let labels: Vec<String> = sessions
        .iter()
        .map(|session| format_session_picker_label(session, verbose))
        .collect();
    let selected_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&labels)
        .default(0)
        .interact_opt()
        .with_context(|| "Could not read session selection")?;

    Ok(selected_index.map(|index| sessions[index].clone()))
}

fn ensure_interactive_terminal(non_interactive_hint: &str) -> Result<()> {
    if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
        return Ok(());
    }

    bail!("Cannot select a session without an interactive terminal; {non_interactive_hint}");
}
