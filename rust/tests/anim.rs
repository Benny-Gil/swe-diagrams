// Tests of the animated outputs: the SVG with motion, the GIF, and the
// video error without ffmpeg.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn spec(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("spec").join(rel)
}

fn tmp(name: &str) -> String {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name).to_str().unwrap().to_string()
}

fn run(args: &[&str], path: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_swe-diagrams"));
    cmd.env("SWE_DIAGRAMS_ICONS", spec("icons")).env("SWE_DIAGRAMS_OFFLINE", "1");
    if let Some(p) = path {
        cmd.env("PATH", p);
    }
    cmd.args(args).stdin(Stdio::null()).output().unwrap()
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}

/// The size and the frame count of a GIF, and true when it loops.
fn gif_info(bytes: &[u8]) -> (u16, u16, usize, bool) {
    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::Indexed);
    let mut dec = opts.read_info(bytes).unwrap();
    let mut n = 0;
    while dec.read_next_frame().unwrap().is_some() {
        n += 1;
    }
    (dec.width(), dec.height(), n, dec.repeat() == gif::Repeat::Infinite)
}

#[test]
fn without_animate_the_svg_does_not_change() {
    let file = spec("fixtures/arch-story.sd");
    let out = run(&["render", file.to_str().unwrap()], None);
    assert_eq!(out.stdout, fs::read(spec("golden/arch-story.svg")).unwrap());
    assert!(!text(&out.stdout).contains("animate"));
}

#[test]
fn animate_adds_motion_without_a_script() {
    let file = spec("fixtures/arch-story.sd");
    let out = run(&["render", file.to_str().unwrap(), "--animate"], None);
    assert_eq!(out.status.code(), Some(0));
    let svg = text(&out.stdout);
    assert!(!svg.contains("<script"));
    assert_eq!(svg.matches("<animateMotion ").count(), 7);
    assert!(svg.contains("@media(prefers-reduced-motion:reduce){.sd-a{display:none}}"));
    assert!(svg.contains("<text class=\"tc mo\" y=\"20\">click Add</text>"));
    assert!(text(&out.stderr).contains(" story=\"Add an item\""));
    let other = run(&["render", file.to_str().unwrap(), "--animate", "--story", "Cache read", "--speed", "2"], None);
    let other = text(&other.stdout);
    assert_eq!(other.matches("<animateMotion ").count(), 4);
}

#[test]
fn a_progress_svg_keeps_the_still_nodes() {
    let file = spec("fixtures/plan-ratelimit.sd");
    let out = run(&["render", file.to_str().unwrap(), "--animate"], None);
    let svg = text(&out.stdout);
    // Two tasks go to done (three variants) and two stop in progress (two).
    assert_eq!(svg.matches("<g class=\"sd-st\">").count(), 4);
    assert_eq!(svg.matches("<g opacity=\"0\"><animate ").count(), 10);
}

#[test]
fn a_gif_loops_and_has_one_frame_for_each_time_step() {
    let target = tmp("story.gif");
    let file = spec("fixtures/arch-story.sd");
    let out = run(&["render", file.to_str().unwrap(), "-o", &target, "--fps", "10"], None);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    // Six steps of 400 to 583 ms, and a pause of 1000 ms: 4350 ms.
    assert!(text(&out.stdout).starts_with("ok arch nodes=5 edges=4 frames=44 story=\"Add an item\" "));
    let (w, h, n, looped) = gif_info(&fs::read(&target).unwrap());
    assert_eq!((w, h, n, looped), (700, 256, 44, true));
    let still = fs::read(spec("golden/arch-story.svg")).unwrap();
    assert!(text(&still).contains("width=\"700\" height=\"256\""));
}

#[test]
fn a_diagram_with_no_story_gives_a_still_gif() {
    let target = tmp("still.gif");
    let file = spec("fixtures/erd-orders.sd");
    let out = run(&["render", file.to_str().unwrap(), "-o", &target, "--scale", "0.5"], None);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains(" frames=1 story=none "));
    assert_eq!(gif_info(&fs::read(&target).unwrap()).2, 1);
}

#[test]
fn a_video_without_ffmpeg_names_the_gif_option() {
    let file = spec("fixtures/arch-story.sd");
    let target = tmp("story.mp4");
    let out = run(&["render", file.to_str().unwrap(), "-o", &target, "--fps", "2"], Some(""));
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        text(&out.stderr),
        "error: a video needs ffmpeg on the PATH; install ffmpeg, or write a GIF with -o out.gif\n"
    );
}
