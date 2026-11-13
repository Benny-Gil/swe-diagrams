// Golden tests. Set UPDATE_GOLDEN=1 to write the expected files again.
// The logos come from spec/icons, and the network is off.

use std::fs;
use std::path::{Path, PathBuf};

use swe_diagrams::{render, IconOptions, Rendered};

fn spec(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("spec").join(dir)
}

/// The `.sd` files of a spec directory, in name order.
fn sources(dir: &str) -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = fs::read_dir(spec(dir))
        .expect("the spec directory must exist")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sd"))
        .collect();
    list.sort();
    assert!(!list.is_empty(), "spec/{dir} has no .sd files");
    list
}

fn draw(src: &str) -> Result<Rendered, swe_diagrams::DiagramError> {
    render(src, &IconOptions { dir: Some(spec("icons")), offline: true })
}

/// The expected text. In update mode, the actual text becomes the expected text.
fn expected(path: &Path, actual: &str) -> String {
    if std::env::var_os("UPDATE_GOLDEN").is_some_and(|v| v == "1") {
        fs::write(path, actual).unwrap_or_else(|err| panic!("cannot write {}: {err}", path.display()));
    }
    read(path)
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
}

/// A short description of the first difference between two texts.
fn difference(actual: &str, expected: &str) -> Option<String> {
    if actual == expected {
        return None;
    }
    let at = actual.bytes().zip(expected.bytes()).take_while(|(a, b)| a == b).count();
    let show =
        |s: &str| String::from_utf8_lossy(&s.as_bytes()[at.saturating_sub(30)..s.len().min(at + 50)]).into_owned();
    Some(format!("first difference at byte {at}\n  actual:   ...{}\n  expected: ...{}", show(actual), show(expected)))
}

#[test]
fn fixtures_match_the_golden_files() {
    let mut failures = Vec::new();
    for path in sources("fixtures") {
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let golden = spec("golden");
        match draw(&read(&path)) {
            Ok(r) => {
                if let Some(diff) = difference(&r.svg, &expected(&golden.join(format!("{name}.svg")), &r.svg)) {
                    failures.push(format!("{name}.svg: {diff}"));
                }
                let summary = format!("{}\n", r.summary);
                if let Some(diff) = difference(&summary, &expected(&golden.join(format!("{name}.txt")), &summary)) {
                    failures.push(format!("{name}.txt: {diff}"));
                }
            }
            Err(err) => failures.push(format!("{name}: the source gives errors:\n{err}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn error_sources_give_the_expected_lines() {
    let mut failures = Vec::new();
    for path in sources("errors") {
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        match draw(&read(&path)) {
            Ok(_) => failures.push(format!("{name}: the source must give an error")),
            Err(err) => {
                let text = format!("{err}\n");
                if let Some(diff) = difference(&text, &expected(&path.with_extension("txt"), &text)) {
                    failures.push(format!("{name}.txt: {diff}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
