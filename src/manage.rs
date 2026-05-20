use crate::export_pdf;
use crate::logs;
use crate::session_lifecycle::{active_sessions, stop_session};
use crate::session_picker::select_active_session_with_prompt;
use crate::session_store::SessionStore;
use crate::ui;
use anyhow::{Context, Result};
use dialoguer::{Select, theme::ColorfulTheme};

#[derive(Debug, Clone, Copy)]
enum SessionAction {
    Export,
    Logs,
    Stop,
}

pub fn manage(verbose: bool) -> Result<()> {
    let store = SessionStore::livetex_cache();
    let sessions = active_sessions(&store)?;
    ui::session_list(&sessions, verbose);

    if sessions.is_empty() {
        return Ok(());
    }

    let Some(session) = select_active_session_with_prompt(
        &store,
        verbose,
        "Manage which LiveTeX session?",
        "use `livetex list`, `livetex export <path_to_latex_file>`, or `livetex stop <path_to_latex_file>`",
    )?
    else {
        return Ok(());
    };

    match select_action()? {
        Some(SessionAction::Export) => export_pdf::export_session(&session, verbose)?,
        Some(SessionAction::Logs) => logs::show_session(&session, 1, verbose)?,
        Some(SessionAction::Stop) => stop_session(&store, &session, verbose)?,
        None => {}
    }

    Ok(())
}

fn select_action() -> Result<Option<SessionAction>> {
    let actions = [
        ("Export PDF", SessionAction::Export),
        ("Show logs", SessionAction::Logs),
        ("Stop preview", SessionAction::Stop),
    ];
    let labels: Vec<_> = actions.iter().map(|(label, _)| *label).collect();
    let selected_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What do you want to do?")
        .items(&labels)
        .default(0)
        .interact_opt()
        .with_context(|| "Could not read action selection")?;

    Ok(selected_index.map(|index| actions[index].1))
}
