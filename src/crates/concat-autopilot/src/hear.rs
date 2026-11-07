//! Hearing the words: the engine's own Whisper, with the model fetched the
//! first time it is wanted.

use std::path::Path;

use concat_host::dirs::AppDirs;
use concat_speech::transcribe::{TranscribeRequest, Transcriber};

use crate::captions::Segment;

/// Transcribes `file`, whose sound lasts `duration` seconds, into segments in
/// the file's own time.
///
/// `language` is Whisper's code (`"ar"` for Arabic); `None` lets Whisper hear
/// it. An English-only model is refused for any other language rather than
/// returning English nonsense. `say` receives a line whenever something slow
/// begins, so a command line can show it.
pub fn transcribe(
    dirs: &AppDirs,
    file: &Path,
    duration: f64,
    model_id: &str,
    language: Option<&str>,
    say: &dyn Fn(&str),
) -> Result<Vec<Segment>, String> {
    let status = Transcriber::status(dirs)?;
    let model = status
        .models
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| {
            let known: Vec<&str> = status.models.iter().map(|m| m.id.as_str()).collect();
            format!(
                "there is no model called {model_id}; the choices are {}",
                known.join(", ")
            )
        })?;
    if model.english_only && language.is_some_and(|l| !l.is_empty() && l != "auto" && l != "en") {
        return Err(format!(
            "{model_id} hears English only; choose {}",
            model_id.trim_end_matches(".en")
        ));
    }

    let transcriber = Transcriber::new();
    if !model.downloaded {
        say(&format!(
            "downloading the {} model, about {} MB, once",
            model.label,
            model.size_bytes / 1_000_000
        ));
        let mut shown = 0;
        transcriber.download_model(dirs, model_id, |progress| {
            let percent = progress
                .received
                .saturating_mul(100)
                .checked_div(progress.total)
                .unwrap_or(0);
            if percent >= shown + 10 {
                shown = percent / 10 * 10;
                say(&format!("  {shown}%"));
            }
        })?;
    }

    say("listening");
    let request = TranscribeRequest {
        path: file
            .to_str()
            .ok_or("the path is not usable text")?
            .to_owned(),
        audio_stream: None,
        source_start: 0.0,
        window: duration,
        model_id: model_id.to_owned(),
        language: language.map(str::to_owned),
    };
    let segments = transcriber.transcribe(dirs, &request, |_| {})?;
    Ok(segments
        .into_iter()
        .map(|s| Segment {
            start: s.start,
            end: s.end,
            text: s.text,
        })
        .collect())
}
