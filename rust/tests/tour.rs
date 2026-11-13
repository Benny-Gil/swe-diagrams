// Tests of the zoom tour: the frame count, the GIF, and the errors.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use swe_diagrams::tour::{tour_frames, TourOptions};
use swe_diagrams::{render, IconOptions, Theme};

/// A diagram with two levels: the top level and the detail of `api`.
const TWO: &str = "arch \"Shop\"\n\nweb \"Web\"\napi \"API\"\nweb => api\n\ndetail api \"API parts\"\n  h \"Handler\"\n  s \"Service\"\n  h => s\n";

fn spec(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("spec").join(rel)
}

fn tmp(name: &str) -> String {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name).to_str().unwrap().to_string()
}

fn two_file() -> String {
    let path = tmp("tour-two.sd");
    fs::write(&path, TWO).unwrap();
    path
}

fn run(args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_swe-diagrams"));
    cmd.env("SWE_DIAGRAMS_ICONS", spec("icons")).env("SWE_DIAGRAMS_OFFLINE", "1");
    cmd.args(args).stdin(Stdio::null()).output().unwrap()
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}

fn options(fps: u32) -> TourOptions {
    TourOptions { fps, speed: 1.0, scale: 0.5, theme: Theme::Light }
}

#[test]
fn a_tour_of_two_levels_has_one_frame_for_each_time_step() {
    let opts = IconOptions { dir: Some(spec("icons")), offline: true };
    let r = render(TWO, &opts).unwrap();
    let t = tour_frames(&r, &options(15)).unwrap();
    assert_eq!(t.levels, 2);
    // At 15 fps: hold 1200 ms (18), zoom 600 ms (9), grow 400 ms (6),
    // hold 1500 ms (23), the zoom back (15), and a rest of 400 ms (6).
    assert_eq!(t.frames.len(), 18 + 9 + 6 + 23 + 15 + 6);
    // The first frame and the last frame show the top level, so the loop is smooth.
    assert_eq!(t.frames[0].data(), t.frames[t.frames.len() - 1].data());
    // All frames have the size of the top level.
    let (w, h) = (t.frames[0].width(), t.frames[0].height());
    assert!(t.frames.iter().all(|f| f.width() == w && f.height() == h));
    // Speed 2 halves the times.
    let fast = tour_frames(&r, &TourOptions { speed: 2.0, ..options(15) }).unwrap();
    assert_eq!(fast.frames.len(), 9 + 5 + 3 + 12 + 8 + 3);
}

#[test]
fn a_tour_gif_has_the_size_and_the_frames() {
    let target = tmp("tour.gif");
    let out = run(&["render", &two_file(), "--tour", "-o", &target, "--fps", "10", "--scale", "0.5"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    // At 10 fps: 12 + 6 + 4 + 15 + 10 + 4 frames.
    assert!(text(&out.stdout).contains(" levels=2 details=api tour=2 frames=51 "), "{}", text(&out.stdout));
    let bytes = fs::read(&target).unwrap();
    let mut dopts = gif::DecodeOptions::new();
    dopts.set_color_output(gif::ColorOutput::Indexed);
    let mut dec = dopts.read_info(bytes.as_slice()).unwrap();
    let mut n = 0;
    while dec.read_next_frame().unwrap().is_some() {
        n += 1;
    }
    let r = render(TWO, &IconOptions { dir: Some(spec("icons")), offline: true }).unwrap();
    let top = swe_diagrams::rasterize(&r.svg, 0.5).unwrap();
    assert_eq!((u32::from(dec.width()), u32::from(dec.height()), n), (top.width(), top.height(), 51));
}

#[test]
fn a_tour_visits_the_levels_depth_first() {
    let file = spec("fixtures/arch-zoom.sd");
    let src = fs::read_to_string(&file).unwrap();
    let opts = IconOptions { dir: Some(spec("icons")), offline: true };
    let r = swe_diagrams::render_path(&src, Some(&file), &opts).unwrap();
    let t = tour_frames(&r, &options(5)).unwrap();
    assert_eq!(t.levels, 4);
    // At 5 fps, one visit with no child: the zoom (3 + 2), the hold (8), the
    // zoom back (5), and the rest (2). The cart visit holds its child svc.
    let leaf = 3 + 2 + 8 + 5 + 2;
    assert_eq!(t.frames.len(), 6 + (leaf + leaf) + leaf);
}

#[test]
fn tour_errors_are_clear() {
    let flat = spec("fixtures/arch-story.sd");
    let flat = flat.to_str().unwrap();
    let two = two_file();
    let gif = tmp("bad-tour.gif");
    let png = tmp("bad-tour.png");
    let cases: [(&[&str], &str); 3] = [
        (
            &["render", flat, "--tour", "-o", &gif],
            "error: --tour needs a diagram with a detail level: a 'detail' block or a '>file.sd' link\n",
        ),
        (&["render", &two, "--tour", "-o", &png], "error: --tour writes gif, mp4, or webm, not png; use -o tour.gif\n"),
        (
            &["render", &two, "--tour", "--story", "x", "-o", &gif],
            "error: use --tour or --story, not both; a tour visits the levels, a story moves on one level\n",
        ),
    ];
    for (args, msg) in cases {
        let out = run(args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert_eq!(text(&out.stderr), msg, "{args:?}");
    }
}
