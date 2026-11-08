// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer Abdullah

//! Command line driver for the Concat engine.
//!
//! This exists so the engine can be exercised end to end without a UI. The
//! `render` command is the vertical slice: probe, build a timeline, plan every
//! frame, decode, composite, encode. The `api` command is the Concat API's
//! first transport: JSON-RPC requests in, responses and events out, one per
//! line, so a script in any language edits and exports a project; `serve`
//! is the same API on a socket, for callers that are other processes.
//! `preview` draws one effect's card the way the window draws the
//! catalogue's: a still through the package's own shader on the GPU. `check` is what an author runs on
//! an effect package before sharing it: the same load the window does, its
//! fixtures, and its probes on the GPU, with every fault named.

use std::error::Error;
use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::{Parser, Subcommand};
use concat_api::rpc::{Call, Message};
use concat_core::time::{FrameRate, Rational};
use concat_core::timeline::{Clip, MediaRef, Timeline, Track, TrackKind};
use concat_media::{EncodeOptions, Encoder, FrameSink, ReaderPool};
use concat_render::{Compositor, WgpuCompositor, plan_frame};

#[derive(Parser)]
#[command(name = "concat-cli", version, about = "CutZone engine command line")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Report what is inside a media file.
    Probe {
        /// The file to inspect.
        path: PathBuf,
    },

    /// Decode, composite and re-encode a clip - the end-to-end slice.
    Render {
        /// Source video.
        input: PathBuf,
        /// Where to write the result.
        output: PathBuf,
        /// How many frames to render.
        #[arg(long, default_value_t = 90)]
        frames: u64,
        /// Fade up from black over this many frames. Zero disables the fade.
        #[arg(long, default_value_t = 15)]
        fade: u64,
    },

    /// Speak the Concat API: a JSON-RPC request per line on stdin, a
    /// response per line on stdout, with events from running jobs as they
    /// happen. A bare `{"method": ..., ...}` without the envelope works too.
    Api {
        /// One request to run instead of reading stdin.
        request: Option<String>,
    },

    /// Serve the Concat API on a socket until the process is stopped:
    /// JSON-RPC lines over TCP or a Unix socket, and gRPC in a build that
    /// has it. With no address given, JSON-RPC on 127.0.0.1:7420. Every
    /// connection presents a token first; with none given, one is minted
    /// and printed with the addresses.
    Serve {
        /// The TCP address for JSON-RPC lines, e.g. 127.0.0.1:7420.
        #[arg(long)]
        json: Option<SocketAddr>,
        /// A Unix socket path for JSON-RPC lines.
        #[arg(long)]
        socket: Option<PathBuf>,
        /// The TCP address for gRPC. Needs a build with the `grpc` feature.
        #[arg(long)]
        grpc: Option<SocketAddr>,
        /// The token every connection presents first. Minted when not
        /// given, and printed either way.
        #[arg(long, env = "CONCAT_API_TOKEN")]
        token: Option<String>,
        /// A folder the API may write under: created projects, exports,
        /// preview files. Repeatable. Your home when none is given; `/`
        /// for anywhere.
        #[arg(long = "root", value_name = "DIR")]
        roots: Vec<PathBuf>,
    },

    /// Draw one effect's card - a still through the package's own shader
    /// on the GPU, at its defaults or its manifest's `[card]` settings,
    /// the way the window draws the catalogue's - and write it as a JPEG:
    /// `concat-cli preview still.jpg out.jpg --effect concat.vignette`.
    Preview {
        /// A still, or the first frame of a video.
        input: PathBuf,
        /// Where to write the JPEG.
        output: PathBuf,
        /// The package's catalogue id or alias, e.g. `concat.vignette`.
        #[arg(long)]
        effect: String,
        /// Card width in pixels; the picture is scaled before the effect
        /// runs, so pixel-sized effects look as they will on the card.
        #[arg(long, default_value_t = concat_host::cards::WIDTH)]
        width: u32,
        /// Card height in pixels.
        #[arg(long, default_value_t = concat_host::cards::HEIGHT)]
        height: u32,
    },

    /// Check effect packages before sharing them: load each folder the way
    /// the window does, hold its id against the built-ins, run its
    /// fixtures and its probes, and name every fault. `path` is one package folder - the
    /// one with `effect.toml` in it - or a folder of them, such as the
    /// app's own effects folder. Exits non-zero when any package fails.
    Check {
        /// A package folder, or a folder of package folders.
        path: PathBuf,
    },

    /// Turns talking footage into a vertical video with the silence cut out
    /// and, when given the words, captions that follow the cut.
    Autopilot {
        /// The recording.
        input: PathBuf,
        /// Where to write the finished video. Without it, nothing is made.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Report what would be cut and change nothing.
        #[arg(long)]
        plan_only: bool,
        /// Where the speech is, as JSON `[[start, end], ...]` in seconds,
        /// instead of listening for it.
        #[arg(long)]
        spans: Option<PathBuf>,
        /// What was said, as JSON `[{"start":0,"end":2,"text":"..."}]` in the
        /// recording's own time. It is moved to follow the cut.
        #[arg(long)]
        segments: Option<PathBuf>,
        /// Listen for the words with Whisper and caption them. The model is
        /// downloaded the first time.
        #[arg(long, conflicts_with = "segments")]
        transcribe: bool,
        /// The language spoken, as Whisper's code: `ar` for Arabic. Whisper
        /// hears it when absent, which is less reliable than saying.
        #[arg(long)]
        language: Option<String>,
        /// The Whisper model: tiny, base, small or large-v3-turbo-q5_0 (the
        /// `.en` ones hear English only). Arabic wants `small` at least; the
        /// large one is far better on Arabic and dialects, and slower.
        #[arg(long, default_value = "small")]
        model: String,
        /// Write what was heard to this file, in the `--segments` format, so
        /// it can be corrected by hand and given back with `--segments`.
        #[arg(long)]
        words_out: Option<PathBuf>,
        /// Corrections as JSON `{"wrong": "right"}`, applied to whole words
        /// of what was heard or given.
        #[arg(long)]
        fix: Option<PathBuf>,
        /// Keep every pause.
        #[arg(long)]
        no_cut: bool,
        /// A 16:9 frame instead of the vertical 9:16.
        #[arg(long)]
        landscape: bool,
        /// A pause shorter than this many seconds is kept.
        #[arg(long, default_value_t = 0.45)]
        keep_pauses: f64,
        /// Where the project folder is made. A new folder in the temporary
        /// directory when absent.
        #[arg(long)]
        work_dir: Option<PathBuf>,
        /// A caption look built into the program: cairo (heavy, the brand
        /// face), tajawal (clean, modern) or almarai (rounder).
        #[arg(long, default_value = "cairo")]
        font_style: String,
        /// Use an installed or supplied family instead of a built-in style.
        #[arg(long)]
        font_family: Option<String>,
        /// A font file for the captions, registered under `--font-family`.
        /// Give a static face per weight: a variable font's weight axis is
        /// ignored.
        #[arg(long = "font")]
        fonts: Vec<PathBuf>,
    },
}

fn main() {
    // A crash names the module it happened in on standard error, as the
    // window's does; see concat-host's crash.rs.
    concat_host::crash::catch();
    // A failure is told in words. Returning the error from `main` would print its
    // debug form, `Error: NoSpeech`, where its message says what happened.
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Command::Probe { path } => probe(&path),
        Command::Render {
            input,
            output,
            frames,
            fade,
        } => render(&input, &output, frames, fade),
        Command::Api { request } => api(request),
        Command::Serve {
            json,
            socket,
            grpc,
            token,
            roots,
        } => serve(json, socket, grpc, token, roots),
        Command::Preview {
            input,
            output,
            effect,
            width,
            height,
        } => preview(&input, &output, &effect, width, height),
        Command::Check { path } => check(&path),
        Command::Autopilot {
            input,
            out,
            plan_only,
            spans,
            segments,
            transcribe,
            language,
            model,
            words_out,
            fix,
            no_cut,
            landscape,
            keep_pauses,
            work_dir,
            font_style,
            font_family,
            fonts,
        } => autopilot(&AutopilotArgs {
            input: &input,
            out: out.as_deref(),
            plan_only,
            spans: spans.as_deref(),
            segments: segments.as_deref(),
            transcribe,
            language: language.as_deref(),
            model: &model,
            words_out: words_out.as_deref(),
            fix: fix.as_deref(),
            no_cut,
            landscape,
            keep_pauses,
            work_dir: work_dir.as_deref(),
            font_style: &font_style,
            font_family: font_family.as_deref(),
            fonts: &fonts,
        }),
    }
}

struct AutopilotArgs<'a> {
    input: &'a Path,
    out: Option<&'a Path>,
    plan_only: bool,
    spans: Option<&'a Path>,
    segments: Option<&'a Path>,
    transcribe: bool,
    language: Option<&'a str>,
    model: &'a str,
    words_out: Option<&'a Path>,
    fix: Option<&'a Path>,
    no_cut: bool,
    landscape: bool,
    keep_pauses: f64,
    work_dir: Option<&'a Path>,
    font_style: &'a str,
    font_family: Option<&'a str>,
    fonts: &'a [PathBuf],
}

/// A JSON file's text. Windows PowerShell and Notepad begin a UTF-8 file with a byte-order mark,
/// which is not JSON, so it is dropped.
fn read_json(path: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(text.trim_start_matches('\u{feff}'))?)
}

/// Reads `[[start, end], ...]`.
fn read_spans(path: &Path) -> Result<Vec<concat_autopilot::cuts::Span>, Box<dyn Error>> {
    let value = read_json(path)?;
    value
        .as_array()
        .ok_or("the spans file must be a list")?
        .iter()
        .map(|pair| match (pair[0].as_f64(), pair[1].as_f64()) {
            (Some(a), Some(b)) => Ok(concat_autopilot::cuts::Span::new(a, b)),
            _ => Err("each span must be [start, end] in seconds".into()),
        })
        .collect()
}

/// Reads `{"wrong": "right", ...}`.
fn read_fixes(path: &Path) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    read_json(path)?
        .as_object()
        .ok_or("the fix file must be an object of \"wrong\": \"right\"")?
        .iter()
        .map(|(wrong, right)| {
            right
                .as_str()
                .map(|r| (wrong.clone(), r.to_owned()))
                .ok_or_else(|| format!("the correction for {wrong:?} must be text").into())
        })
        .collect()
}

/// Reads `[{"start": 0, "end": 2, "text": "..."}]`.
fn read_segments(path: &Path) -> Result<Vec<concat_autopilot::captions::Segment>, Box<dyn Error>> {
    let value = read_json(path)?;
    value
        .as_array()
        .ok_or("the segments file must be a list")?
        .iter()
        .map(|item| {
            match (
                item["start"].as_f64(),
                item["end"].as_f64(),
                item["text"].as_str(),
            ) {
                (Some(start), Some(end), Some(text)) => Ok(concat_autopilot::captions::Segment {
                    start,
                    end,
                    text: text.to_owned(),
                }),
                _ => Err("each segment needs a start, an end and a text".into()),
            }
        })
        .collect()
}

/// Listens to a recording, plans the cut and, given `--out`, makes the video.
fn autopilot(args: &AutopilotArgs) -> Result<(), Box<dyn Error>> {
    use concat_autopilot::captions::{self, CaptionOptions, Segment};
    use concat_autopilot::cuts::{self, CutOptions};
    use concat_autopilot::run::{self, CaptionStyle, Job, Video};

    let path = args.input.to_str().ok_or("the path is not valid text")?;
    let summary = concat_host::media::probe(path)?;
    let duration = summary
        .duration
        .ok_or("the file does not say how long it is")?;
    let dirs = concat_host::dirs::AppDirs::locate()?;

    let options = CutOptions {
        min_gap: args.keep_pauses,
        ..CutOptions::default()
    };
    let (speech, plan) = if args.no_cut {
        (Vec::new(), None)
    } else {
        let speech = match args.spans {
            Some(file) => read_spans(file)?,
            None => concat_autopilot::analyze::speech_in(&dirs, args.input, duration)?,
        };
        let plan = cuts::plan(&speech, duration, options)?;
        (speech, Some(plan))
    };
    let spans = |list: &[concat_autopilot::cuts::Span]| {
        list.iter()
            .map(|s| serde_json::json!([round(s.start), round(s.end)]))
            .collect::<Vec<_>>()
    };
    let mut report = serde_json::json!({ "file": path, "duration": round(duration) });
    if let Some(plan) = &plan {
        report["speech"] = spans(&speech).into();
        report["keep"] = spans(&plan.keep).into();
        report["cuts"] = spans(&plan.cuts()).into();
        report["keptSeconds"] = round(plan.kept()).into();
        report["removedSeconds"] = round(plan.removed()).into();
        report["removedPercent"] = round(100.0 * plan.removed() / plan.total).into();
    }
    // The words: given, heard, or none. Heard before the early return so that
    // `--plan-only --transcribe --words-out` can hand them over for correcting.
    let mut said: Vec<Segment> = match args.segments {
        Some(file) => read_segments(file)?,
        None if args.transcribe => concat_autopilot::hear::transcribe(
            &dirs,
            args.input,
            duration,
            args.model,
            args.language,
            &|line| eprintln!("autopilot: {line}"),
        )?,
        None => Vec::new(),
    };
    if let Some(file) = args.fix {
        let fixes = read_fixes(file)?;
        let mut changed = 0;
        for segment in &mut said {
            let (text, n) = captions::apply_glossary(&segment.text, &fixes);
            segment.text = text;
            changed += n;
        }
        report["fixed"] = changed.into();
    }
    let heard = serde_json::json!(
        said.iter()
            .map(|s| serde_json::json!({ "start": round(s.start), "end": round(s.end), "text": s.text }))
            .collect::<Vec<_>>()
    );
    if let Some(file) = args.words_out {
        std::fs::write(file, serde_json::to_string_pretty(&heard)?)?;
        report["wordsFile"] = serde_json::json!(file);
    }
    report["heard"] = heard;
    let Some(out) = args.out.filter(|_| !args.plan_only) else {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    };

    let moved: Vec<Segment> = match &plan {
        Some(plan) => said
            .iter()
            .filter_map(|s| {
                plan.remap_range(s.start, s.end).map(|r| Segment {
                    start: r.start,
                    end: r.end,
                    text: s.text.clone(),
                })
            })
            .collect(),
        None => said.clone(),
    };
    let captions = captions::build(&moved, &CaptionOptions::default());

    let work = match args.work_dir {
        Some(dir) => dir.to_path_buf(),
        None => std::env::temp_dir().join("cutzone-autopilot"),
    };
    std::fs::create_dir_all(&work)?;
    let (fonts, style) = match args.font_family {
        // The caller's own family and files.
        Some(family) => (
            args.fonts
                .iter()
                .map(|file| (family.to_owned(), file.clone()))
                .collect::<Vec<_>>(),
            CaptionStyle {
                font_family: family.to_owned(),
                ..CaptionStyle::default()
            },
        ),
        // One of the faces built into the program.
        None => {
            let font = concat_autopilot::fonts::FontStyle::parse(args.font_style)
                .ok_or("--font-style must be cairo, tajawal or almarai")?;
            let mut fonts = vec![font.install(&work.join("fonts"))?];
            fonts.extend(
                args.fonts
                    .iter()
                    .map(|f| (font.family().to_owned(), f.clone())),
            );
            (
                fonts,
                CaptionStyle {
                    font_family: font.family().to_owned(),
                    font_weight: font.weight(),
                    ..CaptionStyle::default()
                },
            )
        }
    };
    let video = if args.landscape {
        Video::LANDSCAPE
    } else {
        Video::VERTICAL
    };

    let mut api = concat_api::Api::with_dirs(dirs, run::silent());
    let made = run::run(
        &mut api,
        &Job {
            input: args.input,
            output: out,
            work_dir: &work,
            video,
            fill: !args.landscape,
            plan: plan.as_ref(),
            captions: &captions,
            style: &style,
            fonts: &fonts,
        },
    )?;
    let finished =
        concat_host::media::probe(out.to_str().ok_or("the output path is not valid text")?)?;
    report["output"] = serde_json::json!({
        "file": out,
        "duration": finished.duration.map(round),
        "width": finished.video.as_ref().map(|v| v.width),
        "height": finished.video.as_ref().map(|v| v.height),
        "hasAudio": finished.audio.is_some(),
    });
    report["project"] = serde_json::json!(made.project);
    report["cutsApplied"] = made.cuts_applied.into();
    report["cutsSkipped"] = spans(&made.cuts_skipped).into();
    report["captions"] = made.captions.into();
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Seconds to the millisecond, for a report a person reads.
fn round(seconds: f64) -> f64 {
    (seconds * 1000.0).round() / 1000.0
}

/// The stdin transport. Every line in is one call; every line out is one
/// JSON object, a response or an event, written whole under stdout's lock
/// and flushed, so a caller reading a pipe sees progress as it happens and
/// never half a line. A line that is not a call gets an error response and
/// the loop goes on. At the end of input the jobs still running - an export
/// begun on the last line - are waited for, so their events are written
/// before the process exits.
fn api(single: Option<String>) -> Result<(), Box<dyn Error>> {
    let mut api = concat_api::Api::new(Arc::new(|event| emit(&Message::Event(event))))?;
    let mut serve = |line: &str| {
        let (id, response) = match Call::parse(line) {
            Ok(call) => (call.id, api.dispatch(call.request)),
            Err((id, error)) => (id, concat_api::Response::Error(error)),
        };
        emit(&Message::Reply { id, response });
    };
    match single {
        Some(line) => serve(&line),
        None => {
            for line in std::io::stdin().lock().lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                serve(&line);
            }
        }
    }
    api.finish();
    Ok(())
}

/// One line to stdout. A line that cannot be written is a caller that
/// went away, and the loop learns that from stdin's end.
fn emit(message: &Message) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", message.to_json());
    let _ = out.flush();
}

/// The socket transports, until the process is stopped. Where they listen
/// is printed, one line each, and then the token they take, so a script
/// that started this knows where to connect and what to present.
fn serve(
    json: Option<SocketAddr>,
    socket: Option<PathBuf>,
    grpc: Option<SocketAddr>,
    token: Option<String>,
    roots: Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let nothing_asked = json.is_none() && socket.is_none() && grpc.is_none();
    let config = concat_server::Config {
        json: json.or_else(|| nothing_asked.then(|| DEFAULT_JSON.parse().expect("an address"))),
        socket,
        grpc,
        token,
        roots,
        // The listing is the window's: a server in a terminal prints its
        // address and token for whoever started it.
        discovery: None,
    };
    let server = concat_server::Server::start(config, concat_api::Api::new)?;
    if let Some(address) = server.json_addr() {
        println!(
            "Concat API {}: JSON-RPC on {address}",
            concat_api::API_VERSION
        );
    }
    if let Some(path) = server.socket_path() {
        println!(
            "Concat API {}: JSON-RPC on {}",
            concat_api::API_VERSION,
            path.display()
        );
    }
    if let Some(address) = server.grpc_addr() {
        println!("Concat API {}: gRPC on {address}", concat_api::API_VERSION);
    }
    println!(
        "Concat API {}: token {}",
        concat_api::API_VERSION,
        server.token()
    );
    let roots = server.roots();
    println!(
        "Concat API {}: writes under {}",
        concat_api::API_VERSION,
        if roots.is_empty() {
            "anywhere".to_owned()
        } else {
            roots
                .iter()
                .map(|root| root.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    loop {
        std::thread::park();
    }
}

/// Where `serve` listens when not told: loopback, on a port nothing else
/// is known to use.
const DEFAULT_JSON: &str = "127.0.0.1:7420";

/// `effect`'s card drawn from `input`, scaled to `width` by `height`, as
/// a JPEG at `output`.
fn preview(
    input: &Path,
    output: &Path,
    effect: &str,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn Error>> {
    let catalogue = concat_effects::Catalogue::builtin();
    let package = catalogue
        .get(effect)
        .ok_or_else(|| format!("no package answers to {effect}"))?;
    let card = concat_host::cards::Card::of(catalogue, package, Path::new(""))
        .ok_or_else(|| format!("{effect} draws no picture to make a card of"))?;
    let compositor =
        WgpuCompositor::new().ok_or("no GPU or software renderer is available to draw with")?;
    let mut painter = concat_host::cards::Painter::new(compositor, input, width, height)?;
    painter.draw_to(&card, output)?;
    println!("{} -> {}", package.id(), output.display());
    Ok(())
}

/// Every package at `path` through the loader and its fixtures, one line
/// per package and one per fault. The built-ins are what the ids are held
/// against: a user package cannot take a name the window already knows.
fn check(path: &Path) -> Result<(), Box<dyn Error>> {
    let folders = if path.join("effect.toml").is_file() {
        vec![path.to_path_buf()]
    } else {
        let folders = concat_effects::package_folders(path)?;
        if folders.is_empty() {
            return Err(format!(
                "{}: no package here - a package is a folder with an effect.toml in it",
                path.display()
            )
            .into());
        }
        folders
    };
    let taken = concat_effects::Catalogue::builtin();
    let mut failed = 0;
    let mut gpu = None;
    for folder in &folders {
        let mut problems = concat_effects::Package::check_folder(folder, taken);
        if problems.is_empty() {
            problems = check_probes(folder, &mut gpu);
        }
        if problems.is_empty() {
            println!("ok    {}", folder.display());
        } else {
            failed += 1;
            println!("FAIL  {}", folder.display());
            for problem in problems {
                for line in problem.lines() {
                    println!("      {line}");
                }
            }
        }
    }
    if failed > 0 {
        return Err(format!("{failed} of {} package(s) failed", folders.len()).into());
    }
    println!("{} package(s) checked", folders.len());
    Ok(())
}

/// The probes of the package in `folder`, each run on the GPU, one line
/// per probe that does not hold. The compositor is made on the first
/// package that has any, and a machine that cannot make one says so as a
/// fault: a probe that did not run has not passed.
fn check_probes(folder: &Path, gpu: &mut Option<WgpuCompositor>) -> Vec<String> {
    // A folder that does not load was reported by `check_folder`.
    let Ok(package) = concat_effects::Package::from_folder(folder) else {
        return Vec::new();
    };
    if package.probes.is_empty() {
        return Vec::new();
    }
    if gpu.is_none() {
        *gpu = WgpuCompositor::new();
    }
    let Some(gpu) = gpu.as_mut() else {
        return vec![format!(
            "{}: no GPU or software renderer is available to run its probes on",
            package.id()
        )];
    };
    let mut problems = Vec::new();
    for (n, probe) in package.probes.iter().enumerate() {
        let label = probe.label(n);
        let got = match (package.probe_pass(probe), package.probe_transition(probe)) {
            (Some(pass), _) => gpu.probe(&[pass], probe.input, 8, 0.0),
            (None, Some(cut)) => {
                gpu.probe_transition(&cut, probe.input, probe.to.unwrap_or_default(), 8, 0.0)
            }
            (None, None) => continue,
        };
        match got {
            Some(got) => {
                if let Err(error) = probe.check(got) {
                    problems.push(format!("{}: {label}\n  {error}", package.id()));
                }
            }
            None => problems.push(format!("{}: {label}: the GPU did not answer", package.id())),
        }
    }
    problems
}

fn probe(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let info = concat_media::probe(path)?;

    println!("{}", info.path.display());
    match info.duration {
        Some(duration) => println!("  duration  {:.3}s ({duration})", duration.as_f64()),
        None => println!("  duration  unknown"),
    }

    match &info.video {
        Some(video) => println!(
            "  video     #{} {} {}x{} @ {} ({})",
            video.index,
            video.codec,
            video.width,
            video.height,
            video.frame_rate,
            video.frame_rate.fps()
        ),
        None => println!("  video     none"),
    }

    // Every audio stream, the default first: a recording with its tracks
    // apart lists them all, and a clip may play any one.
    if info.audio_streams.is_empty() {
        println!("  audio     none");
    }
    for audio in &info.audio_streams {
        let name = match (audio.title.is_empty(), audio.language.is_empty()) {
            (true, true) => String::new(),
            (false, true) => format!(" \"{}\"", audio.title),
            (true, false) => format!(" [{}]", audio.language),
            (false, false) => format!(" \"{}\" [{}]", audio.title, audio.language),
        };
        println!(
            "  audio     #{} {} {} Hz, {} channels{name}",
            audio.index, audio.codec, audio.sample_rate, audio.channels
        );
    }

    Ok(())
}

/// The vertical slice.
///
/// Frames come from the reader pool by (media, source time), which is what
/// makes this loop correct for *any* plan - overlapping clips, gaps, jumps -
/// not just one clip played start to finish. The shortcut this function
/// carried for its whole early life is gone; the pool it was waiting for
/// exists (`concat_media::pool`).
fn render(input: &PathBuf, output: &PathBuf, frames: u64, fade: u64) -> Result<(), Box<dyn Error>> {
    let info = concat_media::probe(input)?;
    let video = info.require_video()?;
    let (width, height) = (video.width, video.height);
    let rate = video.frame_rate;

    let timeline = single_clip_timeline(input, width, height, rate, frames);

    let pool = ReaderPool::with_defaults();
    let mut encoder = Encoder::create(output, width, height, rate, &EncodeOptions::default())?;
    // The GPU, or the software adapter where the machine has none.
    let mut compositor =
        WgpuCompositor::new().ok_or("no GPU or software renderer is available to draw with")?;

    println!("rendering {frames} frames at {width}x{height} {rate}");

    for index in 0..frames {
        let mut plan = plan_frame(&timeline, rate.time_of_frame(index as i64));
        // The plan says what is on screen; the pool fills in the pictures.
        // A gap in the timeline is a plan with no layers, and black.
        for layer in &mut plan.layers {
            let frame = pool.frame_at(
                &layer.media,
                layer.source_time,
                width,
                height,
                false,
                None,
                None,
                None,
                // Straight to the GPU compositor: deep for an HDR source.
                true,
            )?;
            layer.source = Some(frame);
            layer.opacity *= fade_in(index, fade);
        }
        let composed = compositor.render(&plan);

        encoder.write_frame(&composed)?;
    }

    encoder.finish()?;
    println!("wrote {} frames to {}", encoder.written(), output.display());
    Ok(())
}

fn single_clip_timeline(
    input: &PathBuf,
    width: u32,
    height: u32,
    rate: FrameRate,
    frames: u64,
) -> Timeline {
    let mut timeline = Timeline::new(width, height, rate);
    let track = timeline.add_track(Track::new("V1", TrackKind::Video));
    let duration = rate.time_of_frame(frames as i64);
    timeline
        .add_clip(
            track,
            Clip::new(MediaRef::new(input), Rational::ZERO, duration),
        )
        .expect("the track was just added");
    timeline
}

/// Ramps from 0.0 to 1.0 over the first `fade` frames.
fn fade_in(index: u64, fade: u64) -> f32 {
    if fade == 0 || index >= fade {
        1.0
    } else {
        index as f32 / fade as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fade_ramps_then_holds() {
        assert_eq!(fade_in(0, 10), 0.0);
        assert_eq!(fade_in(5, 10), 0.5);
        assert_eq!(fade_in(10, 10), 1.0);
        assert_eq!(fade_in(99, 10), 1.0);
    }

    #[test]
    fn a_zero_length_fade_is_fully_opaque_immediately() {
        assert_eq!(fade_in(0, 0), 1.0);
    }

    #[test]
    fn the_timeline_covers_exactly_the_requested_frames() {
        let timeline =
            single_clip_timeline(&PathBuf::from("a.mp4"), 1920, 1080, FrameRate::NTSC_30, 90);
        assert_eq!(timeline.frame_count(), 90);
        assert_eq!(timeline.clip_count(), 1);
    }
}
