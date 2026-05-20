use crate::session_lifecycle::active_sessions_for_tex_file;
use crate::session_picker::select_active_session_with_prompt;
use crate::session_store::{SessionStore, SessionSummary};
use crate::ui;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn export(tex_file: Option<PathBuf>, verbose: bool) -> Result<()> {
    let store = SessionStore::livetex_cache();

    let Some(tex_file) = tex_file else {
        return export_selected_session(&store, verbose);
    };

    validate_tex_file(&tex_file)?;

    let canonical_tex_file = tex_file
        .canonicalize()
        .with_context(|| format!("Could not resolve path: {tex_file:?}"))?;

    let active_sessions = active_sessions_for_tex_file(&store, &canonical_tex_file)?;

    match active_sessions.len() {
        0 => compile_directly(&canonical_tex_file, verbose)?,
        1 => copy_session_pdf(&active_sessions[0], &canonical_tex_file, verbose)?,
        _ => bail!(
            "Multiple active sessions found for {canonical_tex_file:?}; stop duplicate sessions before exporting"
        ),
    }

    Ok(())
}

fn export_selected_session(store: &SessionStore, verbose: bool) -> Result<()> {
    let Some(session) = select_active_session_with_prompt(
        store,
        verbose,
        "Export which LiveTeX session?",
        "pass a .tex file to `livetex export <path_to_latex_file>`",
    )?
    else {
        return Ok(());
    };

    let tex_file = session
        .tex_file
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Session {} has no .tex path", session.id))?;

    copy_session_pdf(&session, tex_file, verbose)
}

fn copy_session_pdf(session: &SessionSummary, tex_file: &Path, verbose: bool) -> Result<()> {
    let source_pdf = session
        .pdf_file
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Session {} has no PDF path", session.id))?;

    if !source_pdf.exists() {
        bail!(
            "Session PDF does not exist yet: {:?}. Check the compile log at {:?}",
            source_pdf,
            session.log_path
        );
    }

    let destination_pdf = pdf_next_to_tex_file(tex_file)?;
    copy_atomically(source_pdf, &destination_pdf)?;

    ui::exported_pdf(&destination_pdf, verbose);

    Ok(())
}

fn compile_directly(tex_file: &Path, verbose: bool) -> Result<()> {
    let workdir = tex_file
        .parent()
        .with_context(|| format!("Could not determine workdir for {tex_file:?}"))?;
    let tex_file_name = tex_file
        .file_name()
        .with_context(|| format!("Could not determine file name for {tex_file:?}"))?;

    let mut command = Command::new("latexmk");
    command
        .arg("-pdf")
        .arg("-interaction=nonstopmode")
        .arg("-synctex=1")
        .arg(tex_file_name)
        .current_dir(workdir)
        .stdin(Stdio::null());

    if verbose {
        let status = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| "Could not start latexmk. Is latexmk installed and on PATH?")?;

        if !status.success() {
            bail!("latexmk failed with status: {status}");
        }
    } else {
        let output = command
            .output()
            .with_context(|| "Could not start latexmk. Is latexmk installed and on PATH?")?;

        if !output.status.success() {
            eprint!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            bail!("latexmk failed with status: {}", output.status);
        }
    }

    ui::exported_pdf(&pdf_next_to_tex_file(tex_file)?, verbose);

    Ok(())
}

fn validate_tex_file(tex_file: &Path) -> Result<()> {
    if !tex_file.exists() {
        bail!("File not found: {tex_file:?}");
    }

    if tex_file
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("tex")
    {
        bail!("File is not a .tex file: {tex_file:?}");
    }

    Ok(())
}

fn pdf_next_to_tex_file(tex_file: &Path) -> Result<PathBuf> {
    let file_name = tex_file
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Could not determine file name for {tex_file:?}"))?;

    Ok(tex_file.with_file_name(file_name).with_extension("pdf"))
}

fn copy_atomically(source: &Path, destination: &Path) -> Result<()> {
    let destination_dir = destination
        .parent()
        .with_context(|| format!("Could not determine destination dir for {destination:?}"))?;
    let destination_file_name = destination
        .file_name()
        .with_context(|| format!("Could not determine destination file name for {destination:?}"))?
        .to_string_lossy();
    let temp_destination = destination_dir.join(format!(
        ".{}.livetex-export-tmp-{}",
        destination_file_name,
        std::process::id()
    ));

    fs::copy(source, &temp_destination)
        .with_context(|| format!("Could not copy {source:?} to {temp_destination:?}"))?;
    fs::rename(&temp_destination, destination)
        .with_context(|| format!("Could not move {temp_destination:?} to {destination:?}"))?;

    Ok(())
}
