use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod session_store;
mod start;
mod stop;
mod worker;

use session_store::SessionStore;

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
        tex_file: PathBuf,
    },
    List,
    Logs {
        session_id: String,

        #[arg(short = 'n', long = "lines", default_value_t = 100)]
        lines: usize,
    },
    Worker {
        absolute_tex_file_path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = SessionStore::livetex_cache();

    match cli.command {
        Command::Start { tex_file } => start::start(tex_file)?,
        Command::Stop { tex_file } => stop::stop(tex_file)?,
        Command::List => {
            let sessions = store.list_sessions()?;

            if sessions.is_empty() {
                println!("no sessions in {:?}", store.root_dir());
            }

            for session in sessions {
                let tex_file = session
                    .tex_file
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<unknown>".to_string());

                println!(
                    "{}\n  tex: {}\n  pid: {}\n  dir: {:?}\n  log: {:?}",
                    session.id,
                    tex_file,
                    session
                        .worker_pid
                        .map(|pid| pid.to_string())
                        .unwrap_or_else(|| "<unknown>".to_string()),
                    session.session_dir,
                    session.log_path
                );
            }
        }
        Command::Logs { session_id, lines } => {
            let log_path = store.log_path(&session_id)?;
            let contents = std::fs::read_to_string(&log_path)
                .with_context(|| format!("could not read log file: {:?}", log_path))?;
            let log_lines: Vec<&str> = contents.lines().rev().take(lines).collect();

            for line in log_lines.iter().rev() {
                println!("{line}");
            }
        }
        Command::Worker {
            absolute_tex_file_path,
        } => worker::run_worker(absolute_tex_file_path),
    }

    Ok(())
}
