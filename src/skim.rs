use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

pub fn open_pdf(pdf_file: &Path) -> Result<Option<u32>> {
    let pids_before = pids()?;
    let status = Command::new("open")
        .arg("-a")
        .arg("Skim")
        .arg(pdf_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| "Could not run macOS open command")?;

    ensure_success(status, "Could not open PDF with Skim")?;

    thread::sleep(Duration::from_millis(500));

    let pids_after = pids()?;
    let pids_before: HashSet<u32> = pids_before.into_iter().collect();

    Ok(pids_after
        .iter()
        .copied()
        .find(|pid| !pids_before.contains(pid))
        .or_else(|| pids_after.into_iter().max()))
}

fn pids() -> Result<Vec<u32>> {
    let output = Command::new("pgrep")
        .arg("-x")
        .arg("Skim")
        .output()
        .with_context(|| "Could not run pgrep")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(stdout
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect())
}

fn ensure_success(status: ExitStatus, message: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("{message}: {status}")
    }
}
