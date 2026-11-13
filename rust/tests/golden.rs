// Golden tests. Set UPDATE_GOLDEN=1 to write the expected files again.
// The logos come from spec/icons, the vendor packages come from the fake
// zips in spec/vendor, and the network is off.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use swe_diagrams::{render_path, IconOptions, Rendered};

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

/// A copy of spec/icons with the zips of spec/vendor in its `vendor` folder.
/// The render writes the vendor index there, so the spec stays clean.
fn icon_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("swe-diagrams-golden-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("vendor")).unwrap();
        for (from, to) in [("icons", dir.clone()), ("vendor", dir.join("vendor"))] {
            for entry in fs::read_dir(spec(from)).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().is_some_and(|ext| ext == "svg" || ext == "png" || ext == "zip") {
                    fs::copy(&path, to.join(path.file_name().unwrap())).unwrap();
                }
            }
        }
        dir
    })
    .clone()
}

/// Renders the file at `path`. A file link is relative to `path`.
fn draw(path: &Path) -> Result<Rendered, swe_diagrams::DiagramError> {
    render_path(&read(path), Some(path), &IconOptions { dir: Some(icon_dir()), offline: true })
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
        match draw(&path) {
            Ok(r) => {
                // Each detail level has its own file: `name.cart.svc.svg`.
                for lv in &r.levels {
                    let file =
                        if lv.path.is_empty() { name.clone() } else { format!("{name}.{}", lv.path.replace('/', ".")) };
                    if let Some(diff) = difference(&lv.svg, &expected(&golden.join(format!("{file}.svg")), &lv.svg)) {
                        failures.push(format!("{file}.svg: {diff}"));
                    }
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
        match draw(&path) {
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
