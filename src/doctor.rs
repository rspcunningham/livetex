use crate::finder_service;
use crate::session_store::SessionStore;
use crate::skim;
use crate::skim_defaults;
use crate::ui;
use anyhow::{Context, Result, bail};
use console::style;
use std::fs;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckLevel {
    Required,
    Recommended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckState {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug)]
struct Check {
    level: CheckLevel,
    state: CheckState,
    label: &'static str,
    detail: String,
}

impl Check {
    fn pass(level: CheckLevel, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            level,
            state: CheckState::Pass,
            label,
            detail: detail.into(),
        }
    }

    fn warn(level: CheckLevel, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            level,
            state: CheckState::Warn,
            label,
            detail: detail.into(),
        }
    }

    fn fail(level: CheckLevel, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            level,
            state: CheckState::Fail,
            label,
            detail: detail.into(),
        }
    }
}

pub fn doctor(verbose: bool) -> Result<()> {
    let checks = run_checks();
    let has_required_failure = checks
        .iter()
        .any(|check| check.level == CheckLevel::Required && check.state == CheckState::Fail);

    ui::info("Checking LiveTex prerequisites");

    for check in &checks {
        print_check(check, verbose);
    }

    if has_required_failure {
        bail!("LiveTex is missing required prerequisites");
    }

    if checks.iter().any(|check| check.state == CheckState::Warn) {
        ui::warning("LiveTex can run, but recommended setup is incomplete.");
    } else {
        ui::success("LiveTex setup looks ready");
    }

    Ok(())
}

fn run_checks() -> Vec<Check> {
    vec![
        check_macos(),
        check_command("latexmk", CheckLevel::Required),
        check_command("open", CheckLevel::Required),
        check_command("pgrep", CheckLevel::Required),
        check_command("defaults", CheckLevel::Recommended),
        check_command("osascript", CheckLevel::Recommended),
        check_skim_app(),
        check_skim_automation(),
        check_skim_default(skim_defaults::AUTO_CHECK_KEY),
        check_skim_default(skim_defaults::AUTO_RELOAD_KEY),
        check_finder_service(),
        check_cache_dir(),
    ]
}

fn check_macos() -> Check {
    if std::env::consts::OS == "macos" {
        Check::pass(CheckLevel::Required, "macOS", "running on macOS")
    } else {
        Check::fail(
            CheckLevel::Required,
            "macOS",
            format!("running on {}", std::env::consts::OS),
        )
    }
}

fn check_command(command: &'static str, level: CheckLevel) -> Check {
    match which(command) {
        Ok(Some(path)) => Check::pass(level, command, path),
        Ok(None) => Check::fail(level, command, "not found on PATH"),
        Err(error) => Check::fail(level, command, format!("could not inspect PATH: {error}")),
    }
}

fn check_skim_app() -> Check {
    let output = Command::new("open")
        .arg("-Ra")
        .arg("Skim")
        .stdin(Stdio::null())
        .output();

    match output {
        Ok(output) if output.status.success() => {
            Check::pass(CheckLevel::Required, "Skim", "application is installed")
        }
        Ok(output) => Check::fail(
            CheckLevel::Required,
            "Skim",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ),
        Err(error) => Check::fail(
            CheckLevel::Required,
            "Skim",
            format!("could not run open: {error}"),
        ),
    }
}

fn check_skim_automation() -> Check {
    if let Err(error) = skim::launch() {
        return Check::warn(
            CheckLevel::Recommended,
            "Skim AppleScript",
            format!("could not launch Skim: {error:#}"),
        );
    }

    match skim::can_read_documents() {
        Ok(Some(true)) => Check::pass(CheckLevel::Recommended, "Skim AppleScript", "ready"),
        Ok(Some(false)) => Check::warn(
            CheckLevel::Recommended,
            "Skim AppleScript",
            "could not read Skim documents",
        ),
        Ok(None) => Check::warn(
            CheckLevel::Recommended,
            "Skim AppleScript",
            "not checked because Skim did not launch",
        ),
        Err(error) => Check::warn(
            CheckLevel::Recommended,
            "Skim AppleScript",
            format!("{error:#}"),
        ),
    }
}

fn check_skim_default(key: &'static str) -> Check {
    match skim_defaults::read(key) {
        Ok(Some(value)) => {
            if skim_defaults::is_enabled(&value) {
                Check::pass(CheckLevel::Recommended, key, "enabled")
            } else {
                Check::warn(CheckLevel::Recommended, key, "disabled")
            }
        }
        Ok(None) => Check::warn(CheckLevel::Recommended, key, "not set"),
        Err(error) => Check::warn(
            CheckLevel::Recommended,
            key,
            format!("could not run defaults: {error}"),
        ),
    }
}

fn check_finder_service() -> Check {
    match finder_service::status() {
        Ok(status) if status.current => Check::pass(
            CheckLevel::Recommended,
            "Finder Quick Action",
            status.path.display().to_string(),
        ),
        Ok(status) if status.installed => Check::warn(
            CheckLevel::Recommended,
            "Finder Quick Action",
            format!(
                "installed at {}, but it does not point at {}. Run `livetex setup`.",
                status.path.display(),
                status.expected_binary.display()
            ),
        ),
        Ok(status) => Check::warn(
            CheckLevel::Recommended,
            "Finder Quick Action",
            format!(
                "not installed. Run `livetex setup`; expected {}",
                status.path.display()
            ),
        ),
        Err(error) => Check::warn(
            CheckLevel::Recommended,
            "Finder Quick Action",
            format!("{error:#}"),
        ),
    }
}

fn check_cache_dir() -> Check {
    let cache_dir = SessionStore::livetex_cache().root_dir();

    match fs::create_dir_all(&cache_dir)
        .with_context(|| format!("could not create cache directory: {cache_dir:?}"))
    {
        Ok(()) => Check::pass(
            CheckLevel::Required,
            "cache dir",
            cache_dir.display().to_string(),
        ),
        Err(error) => Check::fail(CheckLevel::Required, "cache dir", format!("{error:#}")),
    }
}

fn which(command: &str) -> Result<Option<String>> {
    let output = Command::new("which")
        .arg(command)
        .stdin(Stdio::null())
        .output()
        .with_context(|| "could not run which")?;

    if !output.status.success() {
        return Ok(None);
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!path.is_empty()).then_some(path))
}

fn print_check(check: &Check, verbose: bool) {
    let icon = match check.state {
        CheckState::Pass => style("✔").green(),
        CheckState::Warn => style("!").yellow(),
        CheckState::Fail => style("✘").red(),
    };
    let label = match check.state {
        CheckState::Pass => style(check.label).green().bold(),
        CheckState::Warn => style(check.label).yellow().bold(),
        CheckState::Fail => style(check.label).red().bold(),
    };

    println!("{icon} {label}");

    if verbose || check.state != CheckState::Pass {
        ui::detail("detail", &check.detail);
    }
}
