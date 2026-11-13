//! Command line entry: swe-diagrams render <file|-> [-o out] [--format F]
//! [--theme T] [--scale N] [--animate] [--story T] [--fps N] [--speed X]
//! [--icons dir] [--offline]

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use swe_diagrams::anim::{self, FrameOptions};
use swe_diagrams::video::{encode_gif, encode_video};
use swe_diagrams::{apply_theme, html, png, render, Format, IconOptions, Rendered, Theme};

const USAGE: &str = "usage: swe-diagrams render <file.sd|-> [-o out] [--format svg|html|png|txt|gif|mp4|webm] [--theme auto|light|dark] [--scale N] [--animate] [--story title] [--fps N] [--speed X] [--icons dir] [--offline]\n";
const VERSION: &str = concat!("swe-diagrams ", env!("CARGO_PKG_VERSION"), "\n");

/// The options of the output.
struct Out<'a> {
    format: Format,
    theme: Option<Theme>,
    scale: Option<f32>,
    animate: bool,
    story: Option<&'a str>,
    fps: u32,
    speed: f64,
    path: Option<&'a std::path::Path>,
}

/// The bytes of the output, or `None` when ffmpeg wrote the file, and a
/// note for the summary.
fn produce(r: &Rendered, o: &Out) -> Result<(Option<Vec<u8>>, String), String> {
    let animated = matches!(o.format, Format::Gif | Format::Mp4 | Format::Webm) || o.animate;
    let story = if animated { anim::pick(r, o.story)? } else { None };
    let note = match story {
        Some(s) => format!(" story=\"{}\"", s.title),
        None if animated => " story=none (a still picture: the diagram has no story and no => edges)".to_string(),
        None => String::new(),
    };
    match o.format {
        Format::Svg => {
            let svg = match story {
                Some(s) if o.animate => {
                    let routes = anim::routes(&r.svg);
                    anim::animate_svg(&r.svg, &r.variants, &routes, s, &anim::timing(s, &routes, o.speed))
                }
                _ => r.svg.clone(),
            };
            Ok((Some(apply_theme(&svg, o.theme.unwrap_or(Theme::Auto)).into_bytes()), note))
        }
        Format::Html => Ok((Some(html(r, o.theme.unwrap_or(Theme::Auto)).into_bytes()), String::new())),
        // A PNG has one palette. Light is the default.
        Format::Png => Ok((
            Some(png(&apply_theme(&r.svg, o.theme.unwrap_or(Theme::Light)), o.scale.unwrap_or(2.0))?),
            String::new(),
        )),
        Format::Gif | Format::Mp4 | Format::Webm => {
            let fo = FrameOptions {
                fps: o.fps,
                speed: o.speed,
                scale: o.scale.unwrap_or(1.0),
                theme: o.theme.unwrap_or(Theme::Light),
            };
            let frames = anim::frames(r, story, &fo)?;
            let note = format!(" frames={}{note}", frames.len());
            if o.format == Format::Gif {
                return Ok((Some(encode_gif(&frames, o.fps)?), note));
            }
            let path = o.path.ok_or("a video needs -o <file>")?;
            encode_video(&frames, o.fps, path)?;
            Ok((None, note))
        }
        f => Err(format!("the {} format is not ready", f.name())),
    }
}

/// True for the formats that this build can write.
fn ready(format: Format) -> bool {
    matches!(format, Format::Svg | Format::Png | Format::Html | Format::Gif | Format::Mp4 | Format::Webm)
}

// A closed pipe must not cause a panic, so the two functions report a
// failure to the caller.
fn to_stdout(text: &str) -> bool {
    let mut out = io::stdout().lock();
    out.write_all(text.as_bytes()).and_then(|()| out.flush()).is_ok()
}

fn to_stderr(text: &str) {
    let _ = io::stderr().lock().write_all(text.as_bytes());
}

fn read_input(input: &OsString) -> io::Result<Vec<u8>> {
    if input == "-" {
        let mut bytes = Vec::new();
        io::stdin().lock().read_to_end(&mut bytes)?;
        Ok(bytes)
    } else {
        std::fs::read(input)
    }
}

fn run(argv: &[OsString]) -> u8 {
    if argv.len() == 1 && argv[0] == "--version" {
        return if to_stdout(VERSION) { 0 } else { 1 };
    }
    if argv.len() < 2 || argv[0] != "render" {
        to_stderr(USAGE);
        return 1;
    }
    let input = &argv[1];
    let shown = input.to_string_lossy();
    // SWE_DIAGRAMS_OFFLINE=1 has the same effect as --offline: no logo fetch.
    let mut opts =
        IconOptions { dir: None, offline: std::env::var_os("SWE_DIAGRAMS_OFFLINE").is_some_and(|v| v == "1") };
    let mut output: Option<&OsString> = None;
    let mut format: Option<Format> = None;
    let mut theme: Option<Theme> = None;
    let mut scale: Option<f32> = None;
    let mut animate = false;
    let mut story: Option<&str> = None;
    let mut fps: u32 = 15;
    let mut speed: f64 = 1.0;
    let mut k = 2;
    while k < argv.len() {
        let value = argv.get(k + 1).and_then(|v| v.to_str());
        if argv[k] == "--format" && k + 1 < argv.len() {
            let Some(f) = value.and_then(Format::from_name) else {
                to_stderr(&format!(
                    "error: unknown format {}; use svg, html, png, txt, gif, mp4, or webm\n",
                    argv[k + 1].to_string_lossy()
                ));
                return 1;
            };
            format = Some(f);
            k += 2;
        } else if argv[k] == "--theme" && k + 1 < argv.len() {
            let Some(t) = value.and_then(Theme::from_name) else {
                to_stderr(&format!(
                    "error: unknown theme {}; use auto, light, or dark\n",
                    argv[k + 1].to_string_lossy()
                ));
                return 1;
            };
            theme = Some(t);
            k += 2;
        } else if argv[k] == "--scale" && k + 1 < argv.len() {
            match value.and_then(|v| v.parse::<f32>().ok()) {
                Some(n) if (0.1..=16.0).contains(&n) => scale = Some(n),
                _ => {
                    to_stderr("error: the scale must be a number from 0.1 to 16\n");
                    return 1;
                }
            }
            k += 2;
        } else if argv[k] == "--animate" {
            animate = true;
            k += 1;
        } else if argv[k] == "--story" && k + 1 < argv.len() {
            let Some(t) = value else {
                to_stderr("error: the story title must be UTF-8\n");
                return 1;
            };
            story = Some(t);
            k += 2;
        } else if argv[k] == "--fps" && k + 1 < argv.len() {
            // A GIF delay under 2 hundredths of a second plays slowly in browsers.
            match value.and_then(|v| v.parse::<u32>().ok()) {
                Some(n) if (1..=50).contains(&n) => fps = n,
                _ => {
                    to_stderr("error: the fps must be a whole number from 1 to 50\n");
                    return 1;
                }
            }
            k += 2;
        } else if argv[k] == "--speed" && k + 1 < argv.len() {
            match value.and_then(|v| v.parse::<f64>().ok()) {
                Some(n) if (0.1..=10.0).contains(&n) => speed = n,
                _ => {
                    to_stderr("error: the speed must be a number from 0.1 to 10\n");
                    return 1;
                }
            }
            k += 2;
        } else if argv[k] == "-o" && k + 1 < argv.len() {
            // An empty output name means standard output.
            output = Some(&argv[k + 1]).filter(|o| !o.is_empty());
            k += 2;
        } else if argv[k] == "--icons" && k + 1 < argv.len() {
            opts.dir = Some(std::path::PathBuf::from(&argv[k + 1]));
            k += 2;
        } else if argv[k] == "--offline" {
            opts.offline = true;
            k += 1;
        } else {
            to_stderr(USAGE);
            return 1;
        }
    }
    let format = format.unwrap_or_else(|| output.map_or(Format::Svg, |o| Format::from_path(std::path::Path::new(o))));
    if !ready(format) {
        to_stderr(&format!("error: the {} format is not ready\n", format.name()));
        return 1;
    }
    if animate && format != Format::Svg {
        to_stderr("error: --animate is for an SVG; a GIF, a video, and the HTML page move without it\n");
        return 1;
    }
    if format.binary() && output.is_none() {
        to_stderr(&format!("error: the {} format needs -o <file>\n", format.name()));
        return 1;
    }
    let Ok(bytes) = read_input(input) else {
        to_stderr(&format!("error: cannot read {shown}\n"));
        return 1;
    };
    // A byte sequence that is not UTF-8 becomes U+FFFD.
    let src = String::from_utf8_lossy(&bytes);
    match render(&src, &opts) {
        Ok(r) => {
            let out = Out { format, theme, scale, animate, story, fps, speed, path: output.map(std::path::Path::new) };
            let (data, note) = match produce(&r, &out) {
                Ok(v) => v,
                Err(msg) => {
                    to_stderr(&format!("error: {msg}\n"));
                    return 1;
                }
            };
            write_output(&format!("{}{note}", r.summary), output, data.as_deref())
        }
        Err(err) => {
            let mut text = String::new();
            for m in &err.errors {
                text.push_str(&format!("{shown}:{m}\n"));
            }
            to_stderr(&text);
            2
        }
    }
}

/// Writes the output to the file or to standard output, and the summary.
/// `None` is a file that ffmpeg wrote.
fn write_output(summary: &str, output: Option<&OsString>, data: Option<&[u8]>) -> u8 {
    match output {
        None => {
            let ok = {
                let mut out = io::stdout().lock();
                out.write_all(data.unwrap_or_default()).and_then(|()| out.flush()).is_ok()
            };
            to_stderr(&format!("{summary}\n"));
            if ok {
                0
            } else {
                1
            }
        }
        Some(output) => {
            if let Some(data) = data {
                if std::fs::write(output, data).is_err() {
                    to_stderr(&format!("error: cannot write {}\n", output.to_string_lossy()));
                    return 1;
                }
            }
            if to_stdout(&format!("{summary} {}\n", output.to_string_lossy())) {
                0
            } else {
                1
            }
        }
    }
}

fn main() -> ExitCode {
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    ExitCode::from(run(&argv))
}
