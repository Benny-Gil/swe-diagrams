// Golden tests of the terminal text. Set UPDATE_GOLDEN=1 to write the
// expected files again. The text has no colour and a width of 100.

use std::fs;
use std::path::{Path, PathBuf};

use swe_diagrams::levels::load;
use swe_diagrams::parse::parse;
use swe_diagrams::text_out::to_text;

fn spec(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("spec").join(dir)
}

fn fixtures() -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = fs::read_dir(spec("fixtures"))
        .expect("the spec directory must exist")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sd"))
        .collect();
    list.sort();
    list
}

fn text_of(path: &Path, color: bool, width: usize) -> String {
    let src = fs::read_to_string(path).unwrap();
    // A fixture with detail levels needs the loader. The text shows the top level.
    let levels = load(&src, Some(path)).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    to_text(&levels[0].d, color, width)
}

#[test]
fn fixtures_match_the_term_files() {
    let update = std::env::var_os("UPDATE_GOLDEN").is_some_and(|v| v == "1");
    let mut failures = Vec::new();
    for path in fixtures() {
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let actual = text_of(&path, false, 100);
        let golden = spec("golden").join(format!("{name}.term"));
        if update {
            fs::write(&golden, &actual).unwrap();
        }
        let expected = fs::read_to_string(&golden).unwrap_or_default();
        if actual != expected {
            failures.push(name);
        }
    }
    assert!(failures.is_empty(), "the text differs: {}", failures.join(", "));
}

#[test]
fn text_without_colour_has_no_escape_codes() {
    for path in fixtures() {
        assert!(!text_of(&path, false, 100).contains('\x1b'), "{}", path.display());
    }
}

#[test]
fn colour_marks_the_status_symbols() {
    let d = parse("plan\na \"A\" done\nb \"B\" wip\nc \"C\" del\na -> b\n").unwrap();
    let text = to_text(&d, true, 100);
    assert!(text.contains("\x1b[32m✓"));
    assert!(text.contains("\x1b[35m◐"));
    assert!(text.contains("\x1b[31m−"));
}

#[test]
fn no_line_is_wider_than_the_limit() {
    for path in fixtures() {
        for width in [60, 100] {
            for line in text_of(&path, false, width).lines().skip(3) {
                assert!(line.chars().count() <= width, "{}: {line}", path.display());
            }
        }
    }
}

#[test]
fn a_wide_drawing_becomes_an_outline() {
    let d =
        parse("arch\na \"Alpha\"\nb \"Beta\"\nc \"Gamma\"\nd \"Delta\"\na -> b \"call\"\nb -> c\nc -> d\n").unwrap();
    let text = to_text(&d, false, 40);
    assert!(text.contains("outline: the drawing needs"));
    assert!(text.contains("Alpha → Beta  call"));
}

#[test]
fn a_long_label_gets_an_ellipsis() {
    let d = parse("plan\na \"A label that is much too long for one box\"\n").unwrap();
    assert!(to_text(&d, false, 100).contains('…'));
}
