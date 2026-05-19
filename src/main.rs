use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod export_pdf;
mod process;
mod session_lifecycle;
mod session_picker;
mod session_store;
mod start;
mod stop;

use session_lifecycle::{active_sessions, format_session_summary, is_active_session};
use session_store::{SessionStore, SessionSummary};

#[derive(Debug, Parser)]
#[command(name = "livetex")]
#[command(about = "Live LaTeX compiler with elegant PDF serving")]

struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Start {
        tex_file: PathBuf,
    },
    Stop {
        tex_file: Option<PathBuf>,
    },
    Export {
        tex_file: PathBuf,
    },
    List,
    Logs {
        tex_file: PathBuf,

        #[arg(short = 'n', long = "lines", default_value_t = 100)]
        lines: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = SessionStore::livetex_cache();

    match cli.command {
        Command::Start { tex_file } => start::start(tex_file)?,
        Command::Stop { tex_file } => stop::stop(tex_file)?,
        Command::Export { tex_file } => export_pdf::export(tex_file)?,
        Command::List => {
            let sessions = active_sessions(&store)?;

            if sessions.is_empty() {
                println!("No active sessions.");
            }

            for session in sessions {
                println!("{}", format_session_summary(&session));
            }
        }
        Command::Logs { tex_file, lines } => {
            let sessions = store.sessions_for_tex_file(&tex_file)?;
            let session = select_session_for_logs(sessions, &tex_file)?;
            let log_path = session.log_path;
            let contents = std::fs::read_to_string(&log_path)
                .with_context(|| format!("could not read log file: {:?}", log_path))?;
            let log_lines: Vec<&str> = contents.lines().rev().take(lines).collect();

            for line in log_lines.iter().rev() {
                println!("{line}");
            }
        }
    }

    Ok(())
}

fn select_session_for_logs(
    mut sessions: Vec<SessionSummary>,
    tex_file: &PathBuf,
) -> Result<SessionSummary> {
    if sessions.is_empty() {
        bail!("No session found for {:?}", tex_file);
    }

    let active_sessions: Vec<_> = sessions
        .iter()
        .filter(|session| is_active_session(session))
        .cloned()
        .collect();

    match active_sessions.len() {
        1 => Ok(active_sessions.into_iter().next().unwrap()),
        0 if sessions.len() == 1 => Ok(sessions.remove(0)),
        _ => bail!(
            "Multiple sessions found for {:?}; stop duplicate sessions before reading logs",
            tex_file
        ),
    }
}
