use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

const TEX_FILE_NAME: &str = "tex_file";
const LOG_FILE_NAME: &str = "latexmk.log";
const LATEXMK_PID_FILE_NAME: &str = "latexmk.pid";
const SKIM_PID_FILE_NAME: &str = "skim.pid";
const SKIM_DOCUMENT_PATH_FILE_NAME: &str = "skim_document_path";

#[derive(Debug, Clone)]
pub struct SessionStore {
    root_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub tex_file: PathBuf,
    pub pdf_file: PathBuf,
    pub session_dir: PathBuf,
    pub log_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub tex_file: Option<PathBuf>,
    pub pdf_file: Option<PathBuf>,
    pub latexmk_pid: Option<u32>,
    pub skim_pid: Option<u32>,
    pub skim_document_path: Option<PathBuf>,
    pub session_dir: PathBuf,
    pub log_path: PathBuf,
}

impl SessionStore {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn livetex_cache() -> Self {
        let root_dir = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("livetex")
            .join("sessions");

        Self::new(root_dir)
    }

    pub fn root_dir(&self) -> PathBuf {
        self.root_dir.clone()
    }

    pub fn start_session(&self, tex_file: PathBuf) -> Result<Session> {
        if !tex_file.exists() {
            bail!("File not found: {tex_file:?}");
        }

        if tex_file.extension().and_then(|e| e.to_str()) != Some("tex") {
            bail!("File is not a .tex file: {tex_file:?}");
        }

        let tex_file = tex_file
            .canonicalize()
            .with_context(|| format!("Could not resolve path: {tex_file:?}"))?;

        let id = Uuid::new_v4().to_string();
        let session_dir = self.root_dir.join(&id);
        let pdf_file = pdf_path_for_tex_file(&session_dir, &tex_file)?;
        let log_path = session_dir.join(LOG_FILE_NAME);

        fs::create_dir_all(&session_dir)
            .with_context(|| format!("Could not create session directory: {session_dir:?}"))?;
        fs::write(
            session_dir.join(TEX_FILE_NAME),
            tex_file.to_string_lossy().as_ref(),
        )
        .with_context(|| format!("Could not write session metadata: {session_dir:?}"))?;

        Ok(Session {
            id,
            tex_file,
            pdf_file,
            session_dir,
            log_path,
        })
    }

    pub fn record_latexmk_pid(&self, session_id: &str, latexmk_pid: u32) -> Result<()> {
        let latexmk_pid_path = self.latexmk_pid_path(session_id)?;

        fs::write(&latexmk_pid_path, latexmk_pid.to_string())
            .with_context(|| format!("Could not write latexmk PID: {latexmk_pid_path:?}"))?;

        Ok(())
    }

    pub fn record_skim_pid(&self, session_id: &str, skim_pid: u32) -> Result<()> {
        let skim_pid_path = self.skim_pid_path(session_id)?;

        fs::write(&skim_pid_path, skim_pid.to_string())
            .with_context(|| format!("Could not write Skim PID: {skim_pid_path:?}"))?;

        Ok(())
    }

    pub fn record_skim_document_path(
        &self,
        session_id: &str,
        skim_document_path: &Path,
    ) -> Result<()> {
        let skim_document_path_path = self.skim_document_path_path(session_id)?;

        fs::write(
            &skim_document_path_path,
            skim_document_path.to_string_lossy().as_ref(),
        )
        .with_context(|| {
            format!("Could not write Skim document path: {skim_document_path_path:?}")
        })?;

        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let mut sessions = Vec::new();

        if !self.root_dir.exists() {
            return Ok(sessions);
        }

        for entry in fs::read_dir(&self.root_dir)
            .with_context(|| format!("Could not read sessions directory: {:?}", self.root_dir))?
        {
            let entry = entry?;
            let session_dir = entry.path();

            if !session_dir.is_dir() {
                continue;
            }

            let id = entry.file_name().to_string_lossy().to_string();
            let tex_file = fs::read_to_string(session_dir.join(TEX_FILE_NAME))
                .ok()
                .map(|path| PathBuf::from(path.trim()));
            let pdf_file = tex_file
                .as_ref()
                .and_then(|tex_file| pdf_path_for_tex_file(&session_dir, tex_file).ok());
            let latexmk_pid = fs::read_to_string(session_dir.join(LATEXMK_PID_FILE_NAME))
                .ok()
                .and_then(|pid| pid.trim().parse().ok());
            let skim_pid = fs::read_to_string(session_dir.join(SKIM_PID_FILE_NAME))
                .ok()
                .and_then(|pid| pid.trim().parse().ok());
            let skim_document_path =
                fs::read_to_string(session_dir.join(SKIM_DOCUMENT_PATH_FILE_NAME))
                    .ok()
                    .map(|path| PathBuf::from(path.trim()));

            sessions.push(SessionSummary {
                id,
                tex_file,
                pdf_file,
                latexmk_pid,
                skim_pid,
                skim_document_path,
                log_path: session_dir.join(LOG_FILE_NAME),
                session_dir,
            });
        }

        sessions.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(sessions)
    }

    pub fn sessions_for_tex_file(&self, tex_file: &Path) -> Result<Vec<SessionSummary>> {
        let tex_file = tex_file
            .canonicalize()
            .with_context(|| format!("Could not resolve path: {tex_file:?}"))?;

        let sessions = self
            .list_sessions()?
            .into_iter()
            .filter(|session| session.tex_file.as_deref() == Some(tex_file.as_path()))
            .collect();

        Ok(sessions)
    }

    pub fn stop_session(&self, session_id: &str) -> Result<()> {
        let session_dir = self.session_dir(session_id)?;

        if session_dir.exists() {
            fs::remove_dir_all(&session_dir)
                .with_context(|| format!("Could not remove session directory: {session_dir:?}"))?;
        }

        Ok(())
    }

    pub fn session_dir(&self, session_id: &str) -> Result<PathBuf> {
        validate_session_id(session_id)?;
        Ok(self.root_dir.join(session_id))
    }

    pub fn latexmk_pid_path(&self, session_id: &str) -> Result<PathBuf> {
        Ok(self.session_dir(session_id)?.join(LATEXMK_PID_FILE_NAME))
    }

    pub fn skim_pid_path(&self, session_id: &str) -> Result<PathBuf> {
        Ok(self.session_dir(session_id)?.join(SKIM_PID_FILE_NAME))
    }

    pub fn skim_document_path_path(&self, session_id: &str) -> Result<PathBuf> {
        Ok(self
            .session_dir(session_id)?
            .join(SKIM_DOCUMENT_PATH_FILE_NAME))
    }
}

fn validate_session_id(session_id: &str) -> Result<()> {
    let mut components = Path::new(session_id).components();

    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => bail!("Invalid session id: {session_id:?}"),
    }
}

fn pdf_path_for_tex_file(session_dir: &Path, tex_file: &Path) -> Result<PathBuf> {
    let file_name = tex_file
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Could not determine file name for {tex_file:?}"))?;

    Ok(session_dir.join(file_name).with_extension("pdf"))
}
