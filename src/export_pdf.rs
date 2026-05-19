use crate::session_lifecycle::active_sessions_for_tex_file;
use crate::session_store::{SessionStore, SessionSummary};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn export(tex_file: PathBuf) -> Result<()> {
    validate_tex_file(&tex_file)?;

    let store = SessionStore::livetex_cache();
    let canonical_tex_file = tex_file
        .canonicalize()
        .with_context(|| format!("Could not resolve path: {:?}", tex_file))?;

    let active_sessions = active_sessions_for_tex_file(&store, &canonical_tex_file)?;

    match active_sessions.len() {
        0 => compile_directly(&canonical_tex_file)?,
        1 => copy_session_pdf(&active_sessions[0], &canonical_tex_file)?,
        _ => bail!(
            "Multiple active sessions found for {:?}; stop duplicate sessions before exporting",
            canonical_tex_file
        ),
    }

    Ok(())
}

fn copy_session_pdf(session: &SessionSummary, tex_file: &Path) -> Result<()> {
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

    println!("Exported {:?}", destination_pdf);

    Ok(())
}

fn compile_directly(tex_file: &Path) -> Result<()> {
    let workdir = tex_file
        .parent()
        .with_context(|| format!("Could not determine workdir for {:?}", tex_file))?;
    let tex_file_name = tex_file
        .file_name()
        .with_context(|| format!("Could not determine file name for {:?}", tex_file))?;

    let status = Command::new("latexmk")
        .arg("-pdf")
        .arg("-interaction=nonstopmode")
        .arg("-synctex=1")
        .arg(tex_file_name)
        .current_dir(workdir)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| "Could not start latexmk. Is latexmk installed and on PATH?")?;

    if !status.success() {
        bail!("latexmk failed with status: {status}");
    }

    println!("Exported {:?}", pdf_next_to_tex_file(tex_file)?);

    Ok(())
}

fn validate_tex_file(tex_file: &Path) -> Result<()> {
    if !tex_file.exists() {
        bail!("File not found: {:?}", tex_file);
    }

    if tex_file
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("tex")
    {
        bail!("File is not a .tex file: {:?}", tex_file);
    }

    Ok(())
}

fn pdf_next_to_tex_file(tex_file: &Path) -> Result<PathBuf> {
    let file_name = tex_file
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Could not determine file name for {:?}", tex_file))?;

    Ok(tex_file.with_file_name(file_name).with_extension("pdf"))
}

fn copy_atomically(source: &Path, destination: &Path) -> Result<()> {
    let destination_dir = destination
        .parent()
        .with_context(|| format!("Could not determine destination dir for {:?}", destination))?;
    let destination_file_name = destination
        .file_name()
        .with_context(|| {
            format!(
                "Could not determine destination file name for {:?}",
                destination
            )
        })?
        .to_string_lossy();
    let temp_destination = destination_dir.join(format!(
        ".{}.livetex-export-tmp-{}",
        destination_file_name,
        std::process::id()
    ));

    fs::copy(source, &temp_destination)
        .with_context(|| format!("Could not copy {:?} to {:?}", source, temp_destination))?;
    fs::rename(&temp_destination, destination)
        .with_context(|| format!("Could not move {:?} to {:?}", temp_destination, destination))?;

    Ok(())
}
