use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const OPEN_CONFIRM_TIMEOUT: Duration = Duration::from_secs(8);
const OPEN_CONFIRM_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone)]
pub struct PreviewHandle {
    pub app_pid: Option<u32>,
    pub document_path: PathBuf,
    pub document_open_confirmed: Option<bool>,
}

pub fn open_pdf(pdf_file: &Path) -> Result<PreviewHandle> {
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

    let app_pid = wait_for_app_pid(&pids_before, OPEN_CONFIRM_TIMEOUT)?;

    Ok(PreviewHandle {
        app_pid,
        document_path: pdf_file.to_path_buf(),
        document_open_confirmed: wait_for_document_open(pdf_file, OPEN_CONFIRM_TIMEOUT),
    })
}

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

pub fn document_is_open(pdf_file: &Path) -> Result<bool> {
    if !is_running()? {
        return Ok(false);
    }

    let pdf_file = pdf_file
        .canonicalize()
        .unwrap_or_else(|_| pdf_file.to_path_buf());

    Ok(open_document_paths()?
        .into_iter()
        .any(|document_path| document_path.canonicalize().unwrap_or(document_path) == pdf_file))
}

pub fn close_document(pdf_file: &Path) -> Result<bool> {
    if !is_running()? {
        return Ok(false);
    }

    let target_path = pdf_file.to_string_lossy();
    let script = format!(
        r#"
set targetPath to "{}"
tell application "Skim"
    repeat with skimDocument in documents
        try
            if POSIX path of (get path of skimDocument) is targetPath then
                close skimDocument
                return "closed"
            end if
        end try
    end repeat
end tell
return "missing"
"#,
        escape_applescript_string(&target_path)
    );
    let output = run_osascript(&script)?;

    Ok(output.trim() == "closed")
}

pub fn can_read_documents() -> Result<Option<bool>> {
    if !is_running()? {
        return Ok(None);
    }

    open_document_paths().map(|_| Some(true))
}

pub fn launch() -> Result<()> {
    let status = Command::new("open")
        .arg("-gj")
        .arg("-a")
        .arg("Skim")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| "Could not run macOS open command")?;

    ensure_success(status, "Could not launch Skim")
}

pub fn is_running() -> Result<bool> {
    Ok(!pids()?.is_empty())
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

fn wait_for_app_pid(pids_before: &[u32], timeout: Duration) -> Result<Option<u32>> {
    let pids_before: HashSet<u32> = pids_before.iter().copied().collect();
    let deadline = Instant::now() + timeout;

    loop {
        let pids_after = pids()?;

        if let Some(pid) = preview_pid(&pids_before, &pids_after) {
            return Ok(Some(pid));
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }

        thread::sleep(remaining.min(OPEN_CONFIRM_INTERVAL));
    }
}

fn preview_pid(pids_before: &HashSet<u32>, pids_after: &[u32]) -> Option<u32> {
    pids_after
        .iter()
        .copied()
        .find(|pid| !pids_before.contains(pid))
        .or_else(|| pids_after.iter().copied().max())
}

fn wait_for_document_open(pdf_file: &Path, timeout: Duration) -> Option<bool> {
    let deadline = Instant::now() + timeout;
    let mut apple_script_available = false;

    loop {
        match document_is_open(pdf_file) {
            Ok(true) => return Some(true),
            Ok(false) => apple_script_available = true,
            Err(_) => {}
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return apple_script_available.then_some(false);
        }

        thread::sleep(remaining.min(OPEN_CONFIRM_INTERVAL));
    }
}

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

fn parse_document_paths(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn escape_applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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

    #[test]
    fn escapes_applescript_strings() {
        assert_eq!(
            escape_applescript_string(r#"/tmp/a "quoted" \ file.pdf"#),
            r#"/tmp/a \"quoted\" \\ file.pdf"#
        );
    }

    #[test]
    fn preview_pid_prefers_new_skim_process() {
        let pids_before = HashSet::from([10, 20]);
        let pids_after = [10, 20, 30];

        assert_eq!(preview_pid(&pids_before, &pids_after), Some(30));
    }

    #[test]
    fn preview_pid_falls_back_to_existing_skim_process() {
        let pids_before = HashSet::from([10, 20]);
        let pids_after = [10, 20];

        assert_eq!(preview_pid(&pids_before, &pids_after), Some(20));
    }
}
