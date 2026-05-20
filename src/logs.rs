use crate::session_lifecycle::is_active_session;
use crate::session_store::{SessionStore, SessionSummary};
use crate::ui;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

const RERUN_COMPILE_MARKER: &str = "Latexmk: Need to remake files.";
const WATCH_MARKER: &str = "=== Watching for updated files. Use ctrl/C to stop ...";

pub fn show(tex_file: PathBuf, turns: usize, verbose: bool) -> Result<()> {
    if turns == 0 {
        bail!("--turns must be greater than zero");
    }

    let store = SessionStore::livetex_cache();
    let sessions = store.sessions_for_tex_file(&tex_file)?;
    let session = select_session_for_logs(sessions, &tex_file)?;

    print_recent_compile_turns(&session, turns, verbose)
}

fn print_recent_compile_turns(session: &SessionSummary, turns: usize, verbose: bool) -> Result<()> {
    let contents = std::fs::read_to_string(&session.log_path)
        .with_context(|| format!("could not read log file: {:?}", session.log_path))?;
    let log_lines = recent_compile_turn_lines(&contents, turns);

    ui::logs_header(session, turns, verbose);

    for line in log_lines {
        println!("{line}");
    }

    Ok(())
}

pub fn show_session(session: &SessionSummary, turns: usize, verbose: bool) -> Result<()> {
    if turns == 0 {
        bail!("--turns must be greater than zero");
    }

    print_recent_compile_turns(session, turns, verbose)
}

fn select_session_for_logs(
    mut sessions: Vec<SessionSummary>,
    tex_file: &Path,
) -> Result<SessionSummary> {
    if sessions.is_empty() {
        bail!("No session found for {tex_file:?}");
    }

    let active_sessions: Vec<_> = sessions
        .iter()
        .filter(|session| is_active_session(session))
        .cloned()
        .collect();

    match active_sessions.len() {
        1 => Ok(active_sessions.into_iter().next().unwrap()),
        0 if sessions.len() == 1 => Ok(sessions.remove(0)),
        _ => bail!(
            "Multiple sessions found for {tex_file:?}; stop duplicate sessions before reading logs"
        ),
    }
}

fn recent_compile_turn_lines(contents: &str, turns: usize) -> Vec<&str> {
    if turns == 0 {
        return Vec::new();
    }

    let lines: Vec<_> = contents.lines().collect();
    let compile_turns = compile_turn_ranges(&lines);
    let first_selected_turn = compile_turns.len().saturating_sub(turns);
    let mut selected_lines = Vec::new();

    for range in &compile_turns[first_selected_turn..] {
        if !selected_lines.is_empty() {
            selected_lines.push("");
        }

        selected_lines.extend_from_slice(&lines[range.clone()]);
    }

    selected_lines
}

fn compile_turn_ranges(lines: &[&str]) -> Vec<std::ops::Range<usize>> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut starts = vec![0];

    starts.extend(
        lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| (*line == RERUN_COMPILE_MARKER).then_some(index)),
    );

    starts.sort_unstable();
    starts.dedup();

    starts
        .iter()
        .enumerate()
        .filter_map(|(index, start)| {
            let raw_end = starts.get(index + 1).copied().unwrap_or(lines.len());
            let end = compile_turn_end(lines, *start, raw_end);

            (*start < end).then_some(*start..end)
        })
        .collect()
}

fn compile_turn_end(lines: &[&str], start: usize, raw_end: usize) -> usize {
    let mut end = lines[start..raw_end]
        .iter()
        .position(|line| *line == WATCH_MARKER)
        .map(|position| start + position)
        .unwrap_or(raw_end);

    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    end
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBSERVED_LATEXMK_LOG: &str = "\
Rc files read:
  NONE
Latexmk: This is Latexmk, John Collins, 27 Dec. 2024. Version 4.86a.
No existing .aux file, so I'll make a simple one, and require run of *latex.
======= Need to update make_preview_continuous for target files
Not using a previewer
Latexmk: applying rule 'pdflatex'...
Rule 'pdflatex':  Reasons for rerun
Category 'other':
  Rerun of 'pdflatex' forced or previously required:
    Reason or flag: 'Initial setup'

------------
Run number 1 of rule 'pdflatex'
------------
Running 'pdflatex  -interaction=nonstopmode -synctex=1 -recorder -output-directory=\"/tmp/session\"  \"main.tex\"'
------------
This is pdfTeX, Version 3.141592653-2.6-1.40.27 (TeX Live 2025)
Output written on /tmp/session/main.pdf (1 page, 13201 bytes).
Latexmk: All targets (/tmp/session/main.pdf) are up-to-date

=== Watching for updated files. Use ctrl/C to stop ...

Latexmk: Need to remake files.
 Reasons for rerun
Changed files or newly in use/created:
  main.tex
Category 'changed_user':
  main.tex
Category 'rules_to_apply':
  pdflatex


Latexmk: applying rule 'pdflatex'...
Rule 'pdflatex':  Reasons for rerun
Changed files or newly in use/created:
  main.tex
Category 'changed_user':
  main.tex

------------
Run number 1 of rule 'pdflatex'
------------
Running 'pdflatex  -interaction=nonstopmode -synctex=1 -recorder -output-directory=\"/tmp/session\"  \"main.tex\"'
------------
! Undefined control sequence.
l.8 \\thiscommanddoesnotexist
Latexmk: Errors, so I did not complete making targets
Latexmk: Failure to make the files correctly
Collected error summary (may duplicate other messages):
  pdflatex: Command for 'pdflatex' gave return code 1

=== Watching for updated files. Use ctrl/C to stop ...

Latexmk: Need to remake files.
 Reasons for rerun
Changed files or newly in use/created:
  main.tex
Category 'changed_user':
  main.tex
Category 'rules_to_apply':
  pdflatex


Latexmk: applying rule 'pdflatex'...
Rule 'pdflatex':  Reasons for rerun
Changed files or newly in use/created:
  main.tex
Category 'changed_user':
  main.tex

------------
Run number 1 of rule 'pdflatex'
------------
Running 'pdflatex  -interaction=nonstopmode -synctex=1 -recorder -output-directory=\"/tmp/session\"  \"main.tex\"'
------------
Output written on /tmp/session/main.pdf (1 page, 13385 bytes).
Latexmk: All targets (/tmp/session/main.pdf) are up-to-date

=== Watching for updated files. Use ctrl/C to stop ...
";

    fn session(id: &str, latexmk_pid: Option<u32>) -> SessionSummary {
        SessionSummary {
            id: id.to_string(),
            tex_file: Some(PathBuf::from("/tmp/project/main.tex")),
            pdf_file: Some(PathBuf::from("/tmp/livetex/main.pdf")),
            latexmk_pid,
            skim_pid: None,
            skim_document_path: None,
            session_dir: PathBuf::from(format!("/tmp/livetex/{id}")),
            log_path: PathBuf::from(format!("/tmp/livetex/{id}/latexmk.log")),
        }
    }

    #[test]
    fn recent_compile_turn_lines_returns_latest_compile_by_default() {
        let lines = recent_compile_turn_lines(OBSERVED_LATEXMK_LOG, 1);
        let joined = lines.join("\n");

        assert!(joined.starts_with(RERUN_COMPILE_MARKER));
        assert!(joined.contains("Output written on /tmp/session/main.pdf"));
        assert!(joined.contains("Latexmk: All targets"));
        assert!(!joined.contains("Undefined control sequence"));
        assert!(!joined.contains(WATCH_MARKER));
    }

    #[test]
    fn recent_compile_turn_lines_returns_requested_number_of_turns() {
        let lines = recent_compile_turn_lines(OBSERVED_LATEXMK_LOG, 2);
        let joined = lines.join("\n");

        assert_eq!(
            lines
                .iter()
                .filter(|line| **line == RERUN_COMPILE_MARKER)
                .count(),
            2
        );
        assert!(joined.contains("Undefined control sequence"));
        assert!(joined.contains("Latexmk: Failure to make the files correctly"));
        assert!(joined.contains("Latexmk: All targets"));
        assert!(!joined.contains("Initial setup"));
        assert!(!joined.contains(WATCH_MARKER));
    }

    #[test]
    fn recent_compile_turn_lines_includes_initial_compile_when_requested() {
        let lines = recent_compile_turn_lines(OBSERVED_LATEXMK_LOG, 10);
        let joined = lines.join("\n");

        assert!(joined.starts_with("Rc files read:"));
        assert!(joined.contains("Initial setup"));
        assert!(joined.contains("Undefined control sequence"));
        assert!(joined.contains("Latexmk: All targets"));
        assert!(!joined.contains(WATCH_MARKER));
    }

    #[test]
    fn recent_compile_turn_lines_treats_unrecognized_nonempty_log_as_one_turn() {
        assert_eq!(
            recent_compile_turn_lines("partial log\nstill compiling\n", 1),
            vec!["partial log", "still compiling"]
        );
    }

    #[test]
    fn recent_compile_turn_lines_handles_zero_turns() {
        assert!(recent_compile_turn_lines("one\ntwo\n", 0).is_empty());
    }

    #[test]
    fn selects_single_inactive_session_for_logs() {
        let selected =
            select_session_for_logs(vec![session("stale", None)], Path::new("/tmp/main.tex"))
                .unwrap();

        assert_eq!(selected.id, "stale");
    }

    #[test]
    fn selects_active_session_when_one_is_active() {
        let selected = select_session_for_logs(
            vec![
                session("stale", None),
                session("active", Some(std::process::id())),
            ],
            Path::new("/tmp/main.tex"),
        )
        .unwrap();

        assert_eq!(selected.id, "active");
    }
}
