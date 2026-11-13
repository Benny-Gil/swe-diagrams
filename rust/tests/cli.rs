// Tests of the command line: arguments, output streams, and exit codes.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const USAGE: &str = "usage: swe-diagrams render <file.sd|-> [-o out] [--format svg|html|png|txt|gif|mp4|webm] [--theme auto|light|dark] [--level path] [--scale N] [--animate] [--story title] [--tour] [--fps N] [--speed X] [--color auto|always|never] [--width N] [--icons dir] [--offline]\n";

fn spec(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("spec").join(rel)
}

/// The binary, with the logos of the spec and with the network off.
fn binary() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_swe-diagrams"));
    cmd.env("SWE_DIAGRAMS_ICONS", spec("icons")).env("SWE_DIAGRAMS_OFFLINE", "1");
    cmd
}

fn run(args: &[&str]) -> Output {
    binary().args(args).stdin(Stdio::null()).output().unwrap()
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}

#[test]
fn version_goes_to_stdout() {
    let out = run(&["--version"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stdout), "swe-diagrams 0.1.0\n");
    assert_eq!(text(&out.stderr), "");
}

#[test]
fn wrong_arguments_give_the_usage_line() {
    let file = spec("fixtures/seq-login.sd");
    let file = file.to_str().unwrap();
    let cases: [&[&str]; 8] = [
        &[],
        &["render"],
        &["draw", file],
        &["--version", "x"],
        &["render", file, "-o"],
        &["render", file, "-x", "out.svg"],
        &["render", file, "-o", "out.svg", "more"],
        &["render", file, "--icons"],
    ];
    for args in cases {
        let out = run(args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert_eq!(text(&out.stdout), "", "{args:?}");
        assert_eq!(text(&out.stderr), USAGE, "{args:?}");
    }
}

#[test]
fn a_missing_file_is_a_read_error() {
    let out = run(&["render", "no-such-file.sd"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "");
    assert_eq!(text(&out.stderr), "error: cannot read no-such-file.sd\n");
}

#[test]
fn render_writes_the_svg_to_stdout_and_the_summary_to_stderr() {
    let out = run(&["render", spec("fixtures/arch-checkout.sd").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(out.stdout, fs::read(spec("golden/arch-checkout.svg")).unwrap());
    assert_eq!(out.stderr, fs::read(spec("golden/arch-checkout.txt")).unwrap());
}

#[test]
fn render_reads_stdin() {
    let mut child = binary()
        .args(["render", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let source = fs::read(spec("fixtures/seq-login.sd")).unwrap();
    child.stdin.take().unwrap().write_all(&source).unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(out.stdout, fs::read(spec("golden/seq-login.svg")).unwrap());
    assert_eq!(out.stderr, fs::read(spec("golden/seq-login.txt")).unwrap());
}

#[test]
fn the_output_option_writes_a_file_and_the_summary_to_stdout() {
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-output.svg");
    let target = target.to_str().unwrap();
    let out = run(&["render", spec("fixtures/plan-ratelimit.sd").to_str().unwrap(), "-o", target]);
    assert_eq!(out.status.code(), Some(0));
    let summary = fs::read_to_string(spec("golden/plan-ratelimit.txt")).unwrap();
    assert_eq!(text(&out.stdout), format!("{} {target}\n", summary.trim_end()));
    assert_eq!(text(&out.stderr), "");
    assert_eq!(fs::read(target).unwrap(), fs::read(spec("golden/plan-ratelimit.svg")).unwrap());
}

#[test]
fn diagram_errors_have_the_file_name_and_exit_code_2() {
    let file = spec("errors/many.sd");
    let file = file.to_str().unwrap();
    let out = run(&["render", file]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(text(&out.stdout), "");
    let expected: String =
        fs::read_to_string(spec("errors/many.txt")).unwrap().lines().map(|line| format!("{file}:{line}\n")).collect();
    assert_eq!(text(&out.stderr), expected);
}

fn tmp(name: &str) -> String {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name).to_str().unwrap().to_string()
}

fn run_env(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = binary();
    cmd.env_remove("NO_COLOR").env_remove("COLUMNS");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.args(args).stdin(Stdio::null()).output().unwrap()
}

#[test]
fn text_goes_to_stdout_and_the_summary_to_stderr() {
    let file = spec("fixtures/plan-ratelimit.sd");
    let file = file.to_str().unwrap();
    let out = run_env(&["render", file, "--format", "txt", "--color", "never", "--width", "100"], &[]);
    assert_eq!(out.status.code(), Some(0));
    let golden = fs::read_to_string(spec("golden/plan-ratelimit.term")).unwrap();
    assert_eq!(text(&out.stdout), golden);
    assert!(text(&out.stderr).starts_with("ok plan nodes="), "{}", text(&out.stderr));
    // Standard output is a pipe here, so auto gives no colour.
    let auto = run_env(&["render", file, "--format", "txt", "--width", "100"], &[]);
    assert_eq!(text(&auto.stdout), golden);
}

#[test]
fn color_always_adds_the_escape_codes() {
    let file = spec("fixtures/plan-ratelimit.sd");
    let file = file.to_str().unwrap();
    let out = run_env(&["render", file, "--format", "txt", "--color", "always"], &[("NO_COLOR", "1")]);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).contains("\x1b["));
    // A .txt file gets no colour with auto.
    let target = tmp("plain.txt");
    let out = run_env(&["render", file, "-o", &target], &[]);
    assert_eq!(out.status.code(), Some(0));
    let saved = fs::read_to_string(&target).unwrap();
    assert!(saved.contains("Add rate limiting") && !saved.contains('\x1b'));
    assert!(text(&out.stdout).ends_with(&format!(" {target}\n")));
}

#[test]
fn the_width_sets_the_columns() {
    let file = spec("fixtures/arch-checkout.sd");
    let file = file.to_str().unwrap();
    let widest = |out: &Output| text(&out.stdout).lines().map(|l| l.chars().count()).max().unwrap_or(0);
    let wide = run_env(&["render", file, "--format", "txt", "--width", "200"], &[]);
    let narrow = run_env(&["render", file, "--format", "txt", "--width", "40"], &[]);
    assert!(widest(&wide) > 40, "{}", widest(&wide));
    assert!(widest(&narrow) <= 40, "{}", text(&narrow.stdout));
    // Without --width, $COLUMNS sets the width.
    let cols = run_env(&["render", file, "--format", "txt"], &[("COLUMNS", "40")]);
    assert_eq!(cols.stdout, narrow.stdout);
    let bad = run_env(&["render", file, "--format", "txt", "--width", "5"], &[]);
    assert_eq!(text(&bad.stderr), "error: the width must be a whole number from 20 to 1000\n");
}

#[test]
fn text_shows_a_detail_level() {
    let file = spec("fixtures/plan-zoom.sd");
    let out =
        run_env(&["render", file.to_str().unwrap(), "--format", "txt", "--level", "build", "--color", "never"], &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).contains("Build tasks"));
    assert!(text(&out.stdout).contains("Middleware"));
}

#[test]
fn wrong_option_values_give_an_error() {
    let file = spec("fixtures/seq-login.sd");
    let file = file.to_str().unwrap();
    let cases: [(&[&str], &str); 8] = [
        (&["--format", "jpg"], "error: unknown format jpg; use svg, html, png, txt, gif, mp4, or webm\n"),
        (&["--fps", "0"], "error: the fps must be a whole number from 1 to 50\n"),
        (&["--speed", "20"], "error: the speed must be a number from 0.1 to 10\n"),
        (
            &["--animate", "--format", "html"],
            "error: --animate is for an SVG; a GIF, a video, and the HTML page move without it\n",
        ),
        (&["--animate", "--story", "Nope"], "error: the diagram has no story \"Nope\"; the stories are \"Messages\"\n"),
        (&["--theme", "blue"], "error: unknown theme blue; use auto, light, or dark\n"),
        (&["--scale", "0"], "error: the scale must be a number from 0.1 to 16\n"),
        (&["--scale", "x"], "error: the scale must be a number from 0.1 to 16\n"),
    ];
    for (extra, message) in cases {
        let mut args = vec!["render", file];
        args.extend_from_slice(extra);
        let out = run(&args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert_eq!(text(&out.stderr), message, "{args:?}");
    }
}

#[test]
fn the_theme_option_sets_the_palette() {
    let file = spec("fixtures/arch-checkout.sd");
    let file = file.to_str().unwrap();
    let auto = run(&["render", file, "--theme", "auto"]);
    assert_eq!(auto.stdout, fs::read(spec("golden/arch-checkout.svg")).unwrap());
    let light = run(&["render", file, "--theme", "light"]);
    assert!(!text(&light.stdout).contains("prefers-color-scheme"));
    let dark = run(&["render", file, "--format", "svg", "--theme", "dark"]);
    let dark = text(&dark.stdout);
    assert!(!dark.contains("prefers-color-scheme"));
    assert!(dark.contains(".pg{fill:#0e1217}"));
}

#[test]
fn an_unknown_extension_gives_svg() {
    let target = tmp("cli-output.diagram");
    let out = run(&["render", spec("fixtures/seq-login.sd").to_str().unwrap(), "-o", &target]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(fs::read(&target).unwrap(), fs::read(spec("golden/seq-login.svg")).unwrap());
}

#[test]
fn png_needs_an_output_file_and_writes_one() {
    let file = spec("fixtures/seq-login.sd");
    let file = file.to_str().unwrap();
    let out = run(&["render", file, "--format", "png"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stderr), "error: the png format needs -o <file>\n");
    let target = tmp("cli-output.png");
    let out = run(&["render", file, "-o", &target, "--scale", "1"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(fs::read(&target).unwrap().starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn the_html_extension_writes_a_page() {
    let target = tmp("cli-output.html");
    let out = run(&["render", spec("fixtures/arch-checkout.sd").to_str().unwrap(), "-o", &target]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let page = fs::read_to_string(&target).unwrap();
    assert!(page.starts_with("<!doctype html>"));
    assert!(page.contains("id=\"sd-data\""));
}

#[test]
fn the_level_option_renders_one_level() {
    let file = spec("fixtures/arch-zoom.sd");
    let file = file.to_str().unwrap();
    let icons = spec("icons");
    let base = ["render", file, "--offline", "--icons", icons.to_str().unwrap()];
    let out = run(&base);
    assert_eq!(out.stdout, fs::read(spec("golden/arch-zoom.svg")).unwrap());
    assert_eq!(text(&out.stderr), fs::read_to_string(spec("golden/arch-zoom.txt")).unwrap());
    let mut args = base.to_vec();
    args.extend(["--level", "cart/svc"]);
    let out = run(&args);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(out.stdout, fs::read(spec("golden/arch-zoom.cart.svc.svg")).unwrap());
    let target = tmp("cli-level.png");
    args.extend(["-o", &target, "--scale", "1"]);
    assert_eq!(run(&args).status.code(), Some(0));
    assert!(fs::read(&target).unwrap().starts_with(b"\x89PNG"));
    let out = run(&["render", file, "--offline", "--level", "nope"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stderr), "error: no level 'nope'; levels: cart, cart/svc, pay\n");
    let flat = spec("fixtures/seq-login.sd");
    let out = run(&["render", flat.to_str().unwrap(), "--level", "a"]);
    assert_eq!(text(&out.stderr), "error: no level 'a'; levels: the diagram has no detail levels\n");
}
