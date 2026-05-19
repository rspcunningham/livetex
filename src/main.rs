use anyhow::{Context, Result, bail};
use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod export_pdf;
mod process;
mod session_lifecycle;
mod session_picker;
mod session_store;
mod start;
mod stop;
mod ui;

use session_lifecycle::{active_sessions, is_active_session};
use session_store::{SessionStore, SessionSummary};

const HELP_TEMPLATE: &str = "\
{before-help}{about-with-newline}
? Usage › {usage}

{all-args}{after-help}";

#[derive(Debug, Parser)]
#[command(name = "livetex")]
#[command(about = "Live LaTeX compiler with elegant PDF serving")]
#[command(styles = cli_styles())]
#[command(help_template = HELP_TEMPLATE)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    Start { tex_file: PathBuf },
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    Stop { tex_file: Option<PathBuf> },
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    Export { tex_file: PathBuf },
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    List,
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    Logs {
        tex_file: PathBuf,

        #[arg(short = 'n', long = "lines", default_value_t = 100)]
        lines: usize,
    },
}

fn main() {
    if let Err(error) = run() {
        ui::error(&error);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let store = SessionStore::livetex_cache();

    match cli.command {
        Command::Start { tex_file } => start::start(tex_file)?,
        Command::Stop { tex_file } => stop::stop(tex_file)?,
        Command::Export { tex_file } => export_pdf::export(tex_file)?,
        Command::List => {
            let sessions = active_sessions(&store)?;
            ui::session_list(&sessions);
        }
        Command::Logs { tex_file, lines } => {
            let sessions = store.sessions_for_tex_file(&tex_file)?;
            let session = select_session_for_logs(sessions, &tex_file)?;
            let log_path = session.log_path.clone();
            let contents = std::fs::read_to_string(&log_path)
                .with_context(|| format!("could not read log file: {:?}", log_path))?;
            let log_lines: Vec<&str> = contents.lines().rev().take(lines).collect();

            ui::logs_header(&session, lines);

            for line in log_lines.iter().rev() {
                println!("{line}");
            }
        }
    }

    Ok(())
}

fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default().bold())
        .usage(AnsiColor::Yellow.on_default().bold())
        .literal(AnsiColor::Green.on_default().bold())
        .placeholder(AnsiColor::Cyan.on_default())
        .error(AnsiColor::Red.on_default().bold())
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Yellow.on_default())
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
