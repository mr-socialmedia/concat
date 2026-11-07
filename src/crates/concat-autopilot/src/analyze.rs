//! Listening to a recording: where the speech is, and what to cut.

use std::path::Path;

use concat_host::dirs::AppDirs;

use crate::cuts::{self, CutOptions, Plan, PlanError, Span};

/// What went wrong while analysing a recording.
#[derive(Debug)]
pub enum AnalyzeError {
    /// The file could not be read, or the voice detector could not run.
    Listen(String),
    /// The speech was heard but nothing could be planned from it.
    Plan(PlanError),
}

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyzeError::Listen(why) => write!(f, "could not listen to the recording: {why}"),
            AnalyzeError::Plan(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for AnalyzeError {}

/// Where the speech is in `file`, whose sound lasts `duration` seconds.
pub fn speech_in(dirs: &AppDirs, file: &Path, duration: f64) -> Result<Vec<Span>, AnalyzeError> {
    let path = file
        .to_str()
        .ok_or_else(|| AnalyzeError::Listen(format!("{} is not a usable path", file.display())))?;
    concat_speech::transcribe::find_speech(dirs, path, None, 0.0, Some(duration))
        .map(|spans| spans.into_iter().map(|(a, b)| Span::new(a, b)).collect())
        .map_err(AnalyzeError::Listen)
}

/// Listens to `file` and plans which stretches to keep.
pub fn plan_for(
    dirs: &AppDirs,
    file: &Path,
    duration: f64,
    options: CutOptions,
) -> Result<(Vec<Span>, Plan), AnalyzeError> {
    let speech = speech_in(dirs, file, duration)?;
    let plan = cuts::plan(&speech, duration, options).map_err(AnalyzeError::Plan)?;
    Ok((speech, plan))
}
