use anyhow::Result;
use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod doctor;
mod export_pdf;
mod logs;
mod manage;
mod monitor;
mod process;
mod session_lifecycle;
mod session_picker;
mod session_store;
mod setup;
mod skim;
mod skim_defaults;
mod start;
mod stop;
mod ui;

use session_lifecycle::active_sessions;
use session_store::SessionStore;

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
    #[arg(short, long, global = true)]
    verbose: bool,

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
    Export { tex_file: Option<PathBuf> },
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    List,
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    Manage,
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    Doctor,
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    Setup,
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    Logs {
        tex_file: PathBuf,

        #[arg(short = 't', long = "turns", default_value_t = 1)]
        turns: usize,
    },
    #[command(hide = true)]
    #[command(styles = cli_styles())]
    #[command(help_template = HELP_TEMPLATE)]
    Monitor {
        session_id: String,

        #[arg(long)]
        seen_open: bool,
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
    let verbose = cli.verbose;

    match cli.command {
        Command::Start { tex_file } => start::start(tex_file, verbose)?,
        Command::Stop { tex_file } => stop::stop(tex_file, verbose)?,
        Command::Export { tex_file } => export_pdf::export(tex_file, verbose)?,
        Command::List => {
            let sessions = active_sessions(&store)?;
            ui::session_list(&sessions, verbose);
        }
        Command::Manage => manage::manage(verbose)?,
        Command::Doctor => doctor::doctor(verbose)?,
        Command::Setup => setup::setup(verbose)?,
        Command::Logs { tex_file, turns } => logs::show(tex_file, turns, verbose)?,
        Command::Monitor {
            session_id,
            seen_open,
        } => monitor::monitor(session_id, seen_open, verbose)?,
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
