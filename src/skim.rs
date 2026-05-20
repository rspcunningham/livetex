use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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

#[allow(dead_code)]
pub fn open_document_paths() -> Result<Vec<PathBuf>> {
    let output = run_osascript(
        r#"
tell application "Skim"
    set documentPaths to {}
    repeat with skimDocument in documents
        try
            set end of documentPaths to POSIX path of (get path of skimDocument)
        end try
    end repeat
end tell

set AppleScript's text item delimiters to linefeed
return documentPaths as text
"#,
    )?;

    Ok(parse_document_paths(&output))
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

#[allow(dead_code)]
fn run_osascript(script: &str) -> Result<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::null())
        .output()
        .with_context(|| "Could not run osascript")?;

    if !output.status.success() {
        bail!(
            "osascript failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[allow(dead_code)]
fn parse_document_paths(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn ensure_success(status: ExitStatus, message: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("{message}: {status}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_document_paths_from_osascript_output() {
        assert_eq!(
            parse_document_paths("/tmp/main.pdf\n\n/Users/robin/report.pdf\n"),
            vec![
                PathBuf::from("/tmp/main.pdf"),
                PathBuf::from("/Users/robin/report.pdf")
            ]
        );
    }
}
