use anyhow::{Context, Result, bail};
use std::process::{Command, Stdio};

pub const BUNDLE_ID: &str = "net.sourceforge.skim-app.skim";
pub const AUTO_CHECK_KEY: &str = "SKAutoCheckFileUpdate";
pub const AUTO_RELOAD_KEY: &str = "SKAutoReloadFileUpdate";
pub const RELOAD_KEYS: [&str; 2] = [AUTO_CHECK_KEY, AUTO_RELOAD_KEY];

pub fn read(key: &str) -> Result<Option<String>> {
    let output = Command::new("defaults")
        .arg("read")
        .arg(BUNDLE_ID)
        .arg(key)
        .stdin(Stdio::null())
        .output()
        .with_context(|| "could not run defaults")?;

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}

pub fn write_enabled(key: &str) -> Result<()> {
    let status = Command::new("defaults")
        .arg("write")
        .arg(BUNDLE_ID)
        .arg(key)
        .arg("-bool")
        .arg("true")
        .stdin(Stdio::null())
        .status()
        .with_context(|| "could not run defaults")?;

    if status.success() {
        Ok(())
    } else {
        bail!("defaults failed with status: {status}")
    }
}

pub fn is_enabled(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_enabled_values() {
        assert!(is_enabled("1\n"));
        assert!(is_enabled("true"));
        assert!(is_enabled("YES"));
    }

    #[test]
    fn parses_disabled_values() {
        assert!(!is_enabled("0\n"));
        assert!(!is_enabled("false"));
        assert!(!is_enabled(""));
    }
}
