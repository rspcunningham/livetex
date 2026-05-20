use crate::finder_service;
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

    ui::info("Installing Finder Quick Action");
    let workflow_dir = finder_service::install()?;
    ui::success("Installed LiveTex Start Preview");

    if verbose {
        ui::detail_path("workflow", &workflow_dir);
    }

    Ok(())
}
