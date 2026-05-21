use anyhow::Result;
use clap::builder::styling::{AnsiColor, Styles};
use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

mod doctor;
mod export_pdf;
mod finder_service;
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
#[command(arg_required_else_help = true)]
#[command(override_usage = "livetex [OPTIONS] <TEX_FILE>\n       livetex [OPTIONS] <COMMAND>")]
struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Command>,

    #[arg(
        value_name = "TEX_FILE",
        help = "Start a live preview for this .tex file"
    )]
    tex_file: Option<PathBuf>,
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

    match (cli.command, cli.tex_file) {
        (Some(Command::Start { tex_file }), None) | (None, Some(tex_file)) => {
            start::start(tex_file, verbose)?
        }
        (Some(Command::Stop { tex_file }), None) => stop::stop(tex_file, verbose)?,
        (Some(Command::Export { tex_file }), None) => export_pdf::export(tex_file, verbose)?,
        (Some(Command::List), None) => {
            let sessions = active_sessions(&store)?;
            ui::session_list(&sessions, verbose);
        }
        (Some(Command::Manage), None) => manage::manage(verbose)?,
        (Some(Command::Doctor), None) => doctor::doctor(verbose)?,
        (Some(Command::Setup), None) => setup::setup(verbose)?,
        (Some(Command::Logs { tex_file, turns }), None) => logs::show(tex_file, turns, verbose)?,
        (
            Some(Command::Monitor {
                session_id,
                seen_open,
            }),
            None,
        ) => monitor::monitor(session_id, seen_open, verbose)?,
        (None, None) => {
            Cli::command().print_help()?;
            println!();
        }
        (Some(command), Some(tex_file)) => {
            anyhow::bail!(
                "Cannot combine `{}` with top-level TEX_FILE `{}`",
                command.name(),
                ui::path_label(&tex_file)
            );
        }
    }

    Ok(())
}

impl Command {
    fn name(&self) -> &'static str {
        match self {
            Self::Start { .. } => "start",
            Self::Stop { .. } => "stop",
            Self::Export { .. } => "export",
            Self::List => "list",
            Self::Manage => "manage",
            Self::Doctor => "doctor",
            Self::Setup => "setup",
            Self::Logs { .. } => "logs",
            Self::Monitor { .. } => "monitor",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_top_level_tex_file_as_start_shortcut() {
        let cli = Cli::try_parse_from(["livetex", "sample.tex"]).unwrap();

        assert!(cli.command.is_none());
        assert_eq!(cli.tex_file, Some(PathBuf::from("sample.tex")));
    }

    #[test]
    fn still_parses_explicit_start_subcommand() {
        let cli = Cli::try_parse_from(["livetex", "start", "sample.tex"]).unwrap();

        assert!(matches!(cli.command, Some(Command::Start { .. })));
        assert!(cli.tex_file.is_none());
    }

    #[test]
    fn still_parses_other_subcommands() {
        let cli = Cli::try_parse_from(["livetex", "logs", "sample.tex", "--turns", "2"]).unwrap();

        match cli.command {
            Some(Command::Logs { tex_file, turns }) => {
                assert_eq!(tex_file, PathBuf::from("sample.tex"));
                assert_eq!(turns, 2);
            }
            command => panic!("expected logs command, got {command:?}"),
        }

        assert!(cli.tex_file.is_none());
    }
}
