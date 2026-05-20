use crate::session_store::{Session, SessionSummary};
use anyhow::Error;
use console::{Style, style};
use std::fmt::Display;
use std::path::Path;

pub fn success(message: impl Display) {
    println!("{} {}", style("✔").green(), style(message).bold());
}

pub fn warning(message: impl Display) {
    eprintln!("{} {}", style("!").yellow(), style(message).yellow());
}

pub fn info(message: impl Display) {
    println!("{} {}", style("?").yellow(), style(message).bold());
}

pub fn error(error: &Error) {
    eprintln!("{} {}", style("✘").red(), style("Error").red().bold());

    for (index, cause) in error.chain().enumerate() {
        if index == 0 {
            eprintln!("  {} {}", dim("·"), cause);
        } else {
            eprintln!("  {} {}", dim("caused by:"), cause);
        }
    }
}

pub fn detail(label: &str, value: impl Display) {
    println!("  {} {:<14} {}", dim("·"), dim(label), style(value).green());
}

pub fn detail_path(label: &str, path: &Path) {
    detail(label, path.display());
}

pub fn no_active_sessions() {
    info("No active sessions.");
}

pub fn started_session(session: &Session, latexmk_pid: u32, skim_pid: Option<u32>, verbose: bool) {
    success(format!(
        "Started live preview for {}",
        path_label(&session.tex_file)
    ));

    if !verbose {
        if skim_pid.is_some() {
            success("Opened PDF in Skim");
        }

        return;
    }

    detail("session", &session.id);
    detail_path("tex", &session.tex_file);
    detail_path("pdf", &session.pdf_file);
    detail_path("work dir", &session.session_dir);
    detail_path("log", &session.log_path);
    detail("latexmk pid", latexmk_pid);

    if let Some(skim_pid) = skim_pid {
        detail("skim pid", skim_pid);
    }
}

pub fn stopped_latexmk(pid: u32) {
    success(format!("Stopped latexmk process {pid}"));
}

pub fn stopped_skim(pid: u32) {
    success(format!("Stopped Skim process {pid}"));
}

pub fn stopped_session(session: &SessionSummary, verbose: bool) {
    let label = session
        .tex_file
        .as_deref()
        .map(path_label)
        .unwrap_or_else(|| "<unknown>".to_string());

    success(format!("Stopped live preview for {label}"));

    if verbose {
        detail("session", &session.id);
    }
}

pub fn exported_pdf(path: &Path, verbose: bool) {
    success(format!("Exported PDF to {}", path_label(path)));

    if verbose {
        detail_path("pdf", path);
    }
}

pub fn session_list(sessions: &[SessionSummary], verbose: bool) {
    if sessions.is_empty() {
        no_active_sessions();
        return;
    }

    info(format!(
        "{} active {}",
        sessions.len(),
        if sessions.len() == 1 {
            "preview"
        } else {
            "previews"
        }
    ));

    for session in sessions {
        if verbose {
            println!();
            session_summary(session);
        } else {
            session_summary_concise(session);
        }
    }
}

pub fn session_summary_concise(session: &SessionSummary) {
    let label = session
        .tex_file
        .as_deref()
        .map(path_label)
        .unwrap_or_else(|| "<unknown>".to_string());

    println!("{} {}", style("›").yellow(), style(label).bold());
}

pub fn session_summary(session: &SessionSummary) {
    println!("{} {}", style("›").yellow(), style(&session.id).bold());

    match &session.tex_file {
        Some(path) => detail_path("tex", path),
        None => detail("tex", "<unknown>"),
    }

    match &session.pdf_file {
        Some(path) => detail_path("pdf", path),
        None => detail("pdf", "<unknown>"),
    }

    detail("latexmk pid", display_pid(session.latexmk_pid));
    detail("skim pid", display_pid(session.skim_pid));
    detail_path("work dir", &session.session_dir);
    detail_path("log", &session.log_path);
}

pub fn logs_header(session: &SessionSummary, turns: usize, verbose: bool) {
    let label = session
        .tex_file
        .as_deref()
        .map(path_label)
        .unwrap_or_else(|| "<unknown>".to_string());

    let turn_label = if turns == 1 {
        "last compile".to_string()
    } else {
        format!("last {turns} compiles")
    };

    info(format!("Logs for {label}, {turn_label}"));

    if !verbose {
        println!();
        return;
    }

    if let Some(tex_file) = &session.tex_file {
        detail_path("tex", tex_file);
    }

    detail_path("log", &session.log_path);
    detail("turns", turns);
    println!();
}

fn display_pid(pid: Option<u32>) -> String {
    pid.map(|pid| pid.to_string())
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn dim(value: impl Display) -> console::StyledObject<String> {
    Style::new().black().bright().apply_to(value.to_string())
}

pub fn path_label(path: &Path) -> String {
    path.file_name()
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
