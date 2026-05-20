use crate::skim_defaults;
use crate::ui;
use anyhow::Result;

pub fn setup(verbose: bool) -> Result<()> {
    ui::info("Configuring Skim defaults");

    for key in skim_defaults::RELOAD_KEYS {
        skim_defaults::write_enabled(key)?;
        ui::success(format!("Enabled {key}"));
    }

    if verbose {
        ui::detail("bundle", skim_defaults::BUNDLE_ID);
    }

    Ok(())
}
