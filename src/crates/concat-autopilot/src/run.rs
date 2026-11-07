//! Building the edit through the engine's API, and exporting it.
//!
//! The autopilot does nothing a person could not do in the window: it
//! creates a project, drops the recording in, cuts the silence out by
//! splitting and rippling away each pause, lays the captions down as titles
//! and exports. Going through [`concat_api::Api`] means the refusals,
//! the undo history and the file format are the editor's own.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use concat_api::Api;
use concat_api::message::Request;
use serde_json::{Value, json};

use crate::captions::Caption;
use crate::cuts::{Plan, Span};

/// The output frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Video {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Frames per second.
    pub fps: u32,
}

impl Video {
    /// 1080 x 1920 at 30: what a phone held upright is shown at, and the
    /// frame TikTok, Reels, Snapchat and Shorts all take.
    pub const VERTICAL: Video = Video {
        width: 1080,
        height: 1920,
        fps: 30,
    };
    /// 1920 x 1080 at 30.
    pub const LANDSCAPE: Video = Video {
        width: 1920,
        height: 1080,
        fps: 30,
    };
}

/// How a caption looks. Sizes and offsets are fractions of the frame, as
/// everywhere in the engine's API.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptionStyle {
    /// The family; faces are registered under it with [`Job::fonts`].
    pub font_family: String,
    /// 400 regular, 700 bold. A variable font's weight axis is not applied
    /// by the engine, so a weight needs a static face registered for it.
    pub font_weight: f64,
    /// Height of the letters, as a fraction of the frame's height.
    pub font_size: f64,
    /// Where the caption sits, as a fraction of the frame's height from its
    /// middle; positive is lower.
    pub offset_y: f64,
    /// Letter colour.
    pub color: String,
}

impl Default for CaptionStyle {
    fn default() -> Self {
        CaptionStyle {
            font_family: "Cairo".to_owned(),
            font_weight: 700.0,
            font_size: 0.040,
            offset_y: 0.17,
            color: "#F8F7FC".to_owned(),
        }
    }
}

/// Everything one run needs.
pub struct Job<'a> {
    /// The recording.
    pub input: &'a Path,
    /// Where the video is written.
    pub output: &'a Path,
    /// The folder the project folder is made in.
    pub work_dir: &'a Path,
    /// The output frame.
    pub video: Video,
    /// Cover the frame with the picture, cropping the sides (what a landscape
    /// recording needs to fill a vertical frame), rather than fitting it whole.
    pub fill: bool,
    /// What to cut, in the recording's own time; `None` keeps everything.
    pub plan: Option<&'a Plan>,
    /// Captions in the time of the cut footage.
    pub captions: &'a [Caption],
    /// How they look.
    pub style: &'a CaptionStyle,
    /// Font files to register for the captions, as `(family, file)`.
    pub fonts: &'a [(String, PathBuf)],
}

/// What a run did.
#[derive(Debug)]
pub struct Summary {
    /// The project folder, which can be opened in the window.
    pub project: PathBuf,
    /// Pauses that were cut.
    pub cuts_applied: usize,
    /// Pauses left in, because cutting them could not be done safely.
    pub cuts_skipped: Vec<Span>,
    /// Captions placed.
    pub captions: usize,
}

/// Why a run stopped.
#[derive(Debug)]
pub enum RunError {
    /// The engine refused or failed a step.
    Engine(String),
    /// The engine answered in a shape this code does not know.
    Unexpected(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Engine(why) => write!(f, "{why}"),
            RunError::Unexpected(why) => write!(f, "unexpected reply from the engine: {why}"),
        }
    }
}

impl std::error::Error for RunError {}

/// Sends one API request and returns its reply as JSON.
fn call(api: &mut Api, method: &str, params: Value) -> Result<Value, RunError> {
    let mut object = params.as_object().cloned().unwrap_or_default();
    object.insert("method".to_owned(), json!(method));
    let request: Request = serde_json::from_value(Value::Object(object))
        .map_err(|error| RunError::Unexpected(format!("{method}: {error}")))?;
    let response = serde_json::to_value(api.dispatch(request))
        .map_err(|error| RunError::Unexpected(error.to_string()))?;
    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("failed");
        return Err(RunError::Engine(format!("{method}: {message}")));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| RunError::Unexpected(format!("{method} had no result")))
}

/// Applies one edit command.
fn edit(api: &mut Api, project: &str, command: Value) -> Result<Value, RunError> {
    call(
        api,
        "edit.apply",
        json!({ "path": project, "command": command }),
    )
}

/// The clips of the active timeline, from a reply carrying the project.
fn clips(reply: &Value) -> Vec<&Value> {
    let project = &reply["project"];
    let active = project["activeTimelineId"].as_str();
    project["timelines"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|timeline| active.is_none_or(|id| timeline["id"].as_str() == Some(id)))
        .and_then(|timeline| timeline["clips"].as_array())
        .into_iter()
        .flatten()
        .collect()
}

fn number(value: &Value, key: &str) -> f64 {
    value[key].as_f64().unwrap_or(f64::NAN)
}

/// Cuts `plan`'s pauses out of the clip `head`, working from the last pause
/// to the first so that the times of those not yet cut do not move. Each
/// pause is split off at both ends and rippled away. A pause is left in,
/// and reported, rather than risk removing more than it: the piece about to
/// be deleted must be as long as the pause.
fn cut(
    api: &mut Api,
    project: &str,
    track: &str,
    head: &str,
    plan: &Plan,
) -> Result<(usize, Vec<Span>), RunError> {
    let mut applied = 0;
    let mut skipped = Vec::new();
    for pause in plan.cuts().into_iter().rev() {
        if pause.end < plan.total {
            edit(
                api,
                project,
                json!({ "op": "splitClips", "clipIds": [head], "time": pause.end }),
            )?;
        }
        let reply = if pause.start > 0.0 {
            edit(
                api,
                project,
                json!({ "op": "splitClips", "clipIds": [head], "time": pause.start }),
            )?
        } else {
            call(api, "project.get", json!({ "path": project }))?
        };
        let middle = clips(&reply)
            .into_iter()
            .filter(|c| c["trackId"].as_str() == Some(track))
            .find(|c| (number(c, "start") - pause.start).abs() < 1e-3)
            .map(|c| {
                (
                    c["id"].as_str().unwrap_or_default().to_owned(),
                    number(c, "duration"),
                )
            });
        match middle {
            Some((id, length)) if (length - pause.len()).abs() < 0.02 => {
                edit(
                    api,
                    project,
                    json!({ "op": "removeClips", "clipIds": [id], "ripple": true }),
                )?;
                applied += 1;
            }
            _ => skipped.push(pause),
        }
    }
    Ok((applied, skipped))
}

/// The caption as a title command.
fn caption_command(caption: &Caption, style: &CaptionStyle) -> Value {
    json!({
        "op": "addTextClip",
        "start": caption.start,
        "duration": caption.end - caption.start,
        "above": true,
        "offsetY": style.offset_y,
        "style": {
            "content": caption.text,
            "fontFamily": style.font_family,
            "fontSize": style.font_size,
            "fontWeight": style.font_weight,
            "color": style.color,
            "align": "center",
            "shadow": true,
            "background": "",
            "maxWidth": 0.88,
        }
    })
}

/// Builds the project, edits it and exports, waiting for the export.
pub fn run(api: &mut Api, job: &Job) -> Result<Summary, RunError> {
    let name = format!("cutzone-{}", std::process::id());
    let created = call(
        api,
        "project.create",
        json!({
            "location": job.work_dir,
            "name": name,
            "video": { "width": job.video.width, "height": job.video.height,
                       "rateNum": job.video.fps, "rateDen": 1 },
        }),
    )?;
    let project = job.work_dir.join(&name).to_string_lossy().into_owned();
    let track = created["project"]["timelines"][0]["tracks"][0]["id"]
        .as_str()
        .ok_or_else(|| RunError::Unexpected("a new project has no track".into()))?
        .to_owned();

    let imported = call(
        api,
        "media.import",
        json!({ "path": project, "file": job.input }),
    )?;
    let media = imported["createdId"]
        .as_str()
        .ok_or_else(|| RunError::Unexpected("the import made no media".into()))?
        .to_owned();
    for (family, file) in job.fonts {
        edit(
            api,
            &project,
            json!({ "op": "addFont", "family": family, "path": file }),
        )?;
    }

    let placed = edit(
        api,
        &project,
        json!({ "op": "addClip", "mediaId": media, "trackId": track, "start": 0.0 }),
    )?;
    let clip = placed["createdId"]
        .as_str()
        .ok_or_else(|| RunError::Unexpected("the clip was not placed".into()))?
        .to_owned();
    if job.fill {
        edit(
            api,
            &project,
            json!({ "op": "frameClip", "clipId": clip, "framing": "fill" }),
        )?;
    }

    let (cuts_applied, cuts_skipped) = match job.plan {
        Some(plan) => cut(api, &project, &track, &clip, plan)?,
        None => (0, Vec::new()),
    };
    for caption in job.captions {
        edit(api, &project, caption_command(caption, job.style))?;
    }
    call(api, "project.save", json!({ "path": project }))?;

    let started = call(
        api,
        "export.run",
        json!({ "path": project, "output": job.output }),
    )?;
    let id = started["job"]
        .as_str()
        .ok_or_else(|| RunError::Unexpected("the export has no job".into()))?
        .to_owned();
    let deadline = Instant::now() + Duration::from_secs(60 * 60);
    loop {
        let status = call(api, "export.status", json!({ "job": id }))?;
        match status["state"].as_str() {
            Some("done") => break,
            Some("running") if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(150))
            }
            other => {
                return Err(RunError::Engine(format!(
                    "the export ended as {}: {}",
                    other.unwrap_or("unknown"),
                    status["error"]
                        .as_str()
                        .or(status["message"].as_str())
                        .unwrap_or("no reason given")
                )));
            }
        }
    }
    Ok(Summary {
        project: PathBuf::from(project),
        cuts_applied,
        cuts_skipped,
        captions: job.captions.len(),
    })
}

/// An event sink that drops events: the run polls the job instead.
pub fn silent() -> concat_api::EventSink {
    Arc::new(|_| {})
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A caption becomes a title command with its words, time and look, in
    /// the units the engine takes: fractions of the frame, not pixels.
    #[test]
    fn a_caption_is_a_title_in_fractions_of_the_frame() {
        let caption = Caption {
            start: 1.5,
            end: 3.0,
            text: "مرحبًا بكم\nCutZone".to_owned(),
        };
        let command = caption_command(&caption, &CaptionStyle::default());
        assert_eq!(command["op"], "addTextClip");
        assert_eq!(command["start"], 1.5);
        assert_eq!(command["duration"], 1.5);
        assert_eq!(command["style"]["content"], "مرحبًا بكم\nCutZone");
        assert_eq!(command["style"]["fontFamily"], "Cairo");
        let size = command["style"]["fontSize"].as_f64().unwrap();
        assert!(
            size > 0.0 && size < 0.2,
            "a size of {size} is not a fraction of the frame"
        );
        // No filled box: a semi-transparent one stacks over earlier captions.
        assert_eq!(command["style"]["background"], "");
    }

    /// The edits this module sends are accepted by the engine's own types,
    /// so a rename there is caught here and not in a user's export.
    #[test]
    fn the_requests_parse_as_the_apis_own() {
        let parse = |method: &str, params: Value| {
            let mut object = params.as_object().cloned().unwrap();
            object.insert("method".to_owned(), json!(method));
            serde_json::from_value::<Request>(Value::Object(object))
        };
        parse(
            "project.create",
            json!({ "location": "/tmp", "name": "x",
            "video": { "width": 1080, "height": 1920, "rateNum": 30, "rateDen": 1 } }),
        )
        .unwrap();
        parse("media.import", json!({ "path": "/p", "file": "/f.mp4" })).unwrap();
        parse("project.save", json!({ "path": "/p" })).unwrap();
        parse("project.get", json!({ "path": "/p" })).unwrap();
        parse("export.run", json!({ "path": "/p", "output": "/o.mp4" })).unwrap();
        parse("export.status", json!({ "job": "j1" })).unwrap();
        for command in [
            json!({ "op": "addClip", "mediaId": "m1", "trackId": "T1", "start": 0.0 }),
            json!({ "op": "frameClip", "clipId": "c1", "framing": "fill" }),
            json!({ "op": "splitClips", "clipIds": ["c1"], "time": 2.0 }),
            json!({ "op": "removeClips", "clipIds": ["c1"], "ripple": true }),
            json!({ "op": "addFont", "family": "Cairo", "path": "/f.ttf" }),
            caption_command(
                &Caption {
                    start: 0.0,
                    end: 1.0,
                    text: "x".into(),
                },
                &CaptionStyle::default(),
            ),
        ] {
            parse("edit.apply", json!({ "path": "/p", "command": command })).unwrap();
        }
    }
}
