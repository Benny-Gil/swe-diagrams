// Tests of the logo cache. The tests use only files in a temporary directory,
// and no test uses the network.

use std::fs;
use std::path::{Path, PathBuf};

use swe_diagrams::icons::{default_dir, resolve, Icon, IconOptions};

const SVG_URI: &str = "data:image/svg+xml;base64,";
const PNG_URI: &str = "data:image/png;base64,";

/// Each test gets its own empty directory, because the tests run in parallel.
fn fresh(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("swe-diagrams-icons-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn put(dir: &Path, file: &str, body: &[u8]) {
    fs::write(dir.join(file), body).unwrap();
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn offline(dir: &Path) -> IconOptions {
    IconOptions { dir: Some(dir.to_path_buf()), offline: true }
}

fn one(dir: &Path, name: &str) -> Option<Icon> {
    resolve(&names(&[name]), &offline(dir)).pop().unwrap()
}

#[test]
fn svg_hit_gives_a_data_uri_and_the_local_source() {
    let dir = fresh("svg");
    put(&dir, "react.svg", b"<svg/>");
    let icon = one(&dir, "react").unwrap();
    assert_eq!(icon.uri, format!("{SVG_URI}PHN2Zy8+"));
    assert_eq!(icon.source, "local");
    assert!(!icon.dark_tile);
}

#[test]
fn png_hit_gives_a_png_data_uri() {
    let dir = fresh("png");
    put(&dir, "logo.png", b"foobar");
    let icon = one(&dir, "logo").unwrap();
    assert_eq!(icon, Icon { uri: format!("{PNG_URI}Zm9vYmFy"), dark_tile: false, source: "local".into() });
}

#[test]
fn svg_wins_over_png() {
    let dir = fresh("both");
    put(&dir, "x.svg", b"<svg/>");
    put(&dir, "x.png", b"foobar");
    assert!(one(&dir, "x").unwrap().uri.starts_with(SVG_URI));
}

#[test]
fn alias_reads_the_cache_file_of_the_real_slug() {
    let dir = fresh("alias");
    put(&dir, "postgresql.svg", b"<svg/>");
    put(&dir, "postgres.svg", b"foobar");
    let got = resolve(&names(&["postgres", "@PG", "postgresql"]), &offline(&dir));
    assert_eq!(got[0].as_ref().unwrap().uri, format!("{SVG_URI}PHN2Zy8+"));
    assert_eq!(got[1], got[0]);
    assert_eq!(got[2], got[0]);
}

#[test]
fn src_file_gives_the_source() {
    let dir = fresh("src");
    put(&dir, "postgresql.svg", b"<svg/>");
    put(&dir, "postgresql.src", b"devicon\n");
    put(&dir, "blank.svg", b"<svg/>");
    put(&dir, "blank.src", b"  \n");
    assert_eq!(one(&dir, "postgresql").unwrap().source, "devicon");
    assert_eq!(one(&dir, "blank").unwrap().source, "local");
}

#[test]
fn miss_file_gives_none_without_a_fetch() {
    let dir = fresh("miss");
    put(&dir, "react.miss", b"");
    // The network is allowed here, but the `.miss` file must stop the fetch.
    let opts = IconOptions { dir: Some(dir.clone()), offline: false };
    assert_eq!(resolve(&names(&["react"]), &opts), vec![None]);
    assert!(!dir.join("react.svg").exists());
}

#[test]
fn offline_miss_writes_no_file() {
    let dir = fresh("offline");
    assert_eq!(one(&dir, "not-a-real-logo"), None);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
}

#[test]
fn name_becomes_a_safe_slug() {
    let dir = fresh("slug").join("cache");
    fs::create_dir_all(&dir).unwrap();
    put(&dir, "react.svg", b"<svg/>");
    put(&dir, "a.b_c-1.svg", b"<svg/>");
    put(dir.parent().unwrap(), "secret.svg", b"<svg/>");
    put(&dir, "..svg", b"<svg/>");
    let got = resolve(
        &names(&["@React!", "Re act", "A.B_C-1", "", "@@@", "../secret", "..", "a..b", "日本"]),
        &offline(&dir),
    );
    let hit: Vec<bool> = got.iter().map(Option::is_some).collect();
    assert_eq!(hit, [true, true, true, false, false, false, false, false, false]);
}

#[test]
fn results_keep_the_order_and_repeat_for_a_repeated_name() {
    let dir = fresh("order");
    put(&dir, "a.svg", b"<svg/>");
    put(&dir, "b.png", b"foobar");
    let got = resolve(&names(&["b", "nothing", "a", "B", ""]), &offline(&dir));
    assert_eq!(got.len(), 5);
    assert!(got[0].as_ref().unwrap().uri.starts_with(PNG_URI));
    assert_eq!(got[1], None);
    assert!(got[2].as_ref().unwrap().uri.starts_with(SVG_URI));
    assert_eq!(got[3], got[0]);
    assert_eq!(got[4], None);
    assert_eq!(resolve(&[], &offline(&dir)), vec![]);
}

#[test]
fn dark_tile_needs_white_paint_and_no_other_colour() {
    let cases: [(&str, bool); 14] = [
        (r##"<svg><path fill="#FFF" d="M0 0"/></svg>"##, true),
        (r##"<svg><path fill = '#ffffff'/></svg>"##, true),
        (r##"<svg><path style="fill:White;stroke:none"/></svg>"##, true),
        (r##"<svg><style>.a{fill:#fff !important}</style></svg>"##, true),
        (r##"<svg fill="none"><path fill-rule="evenodd" fill="#fff"/></svg>"##, true),
        (r##"<svg><path stroke-width="2" stroke="#FFFFFF" fill="transparent"/></svg>"##, true),
        (r##"<svg><path fill="url(#g)"/><path fill="#fff"/></svg>"##, true),
        (r##"<svg><path fill="#fff"/><path fill="#000"/></svg>"##, false),
        (r##"<svg><path fill="#fff" stroke="#336791"/></svg>"##, false),
        (r##"<svg><path style="fill:#fff;stroke:red"/></svg>"##, false),
        (r##"<svg><path fill="currentColor"/></svg>"##, false),
        (r##"<svg><path fill="none"/></svg>"##, false),
        (r##"<svg><path d="M0 0"/></svg>"##, false),
        (r##"<svg><path id="refill" class="fill"/></svg>"##, false),
    ];
    let dir = fresh("dark");
    let mut list = Vec::new();
    for (i, (svg, _)) in cases.iter().enumerate() {
        put(&dir, &format!("c{i}.svg"), svg.as_bytes());
        list.push(format!("c{i}"));
    }
    put(&dir, "white.png", b"#fff");
    list.push("white".into());
    let got = resolve(&list, &offline(&dir));
    for (i, (svg, want)) in cases.iter().enumerate() {
        assert_eq!(got[i].as_ref().unwrap().dark_tile, *want, "{svg}");
    }
    assert!(!got[cases.len()].as_ref().unwrap().dark_tile);
}

#[test]
fn base64_matches_the_rfc_4648_vectors() {
    let vectors = [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ];
    let dir = fresh("b64");
    for (i, (plain, _)) in vectors.iter().enumerate() {
        put(&dir, &format!("v{i}.png"), plain.as_bytes());
    }
    let got = resolve(&(0..vectors.len()).map(|i| format!("v{i}")).collect::<Vec<_>>(), &offline(&dir));
    for (i, (_, code)) in vectors.iter().enumerate() {
        assert_eq!(got[i].as_ref().unwrap().uri, format!("{PNG_URI}{code}"));
    }
    // Each byte value must survive.
    let all: Vec<u8> = (0..=255).collect();
    put(&dir, "all.png", &all);
    let uri = one(&dir, "all").unwrap().uri;
    assert!(uri.ends_with("+/w=="), "{uri}");
}

#[test]
fn default_dir_follows_the_variables_in_order() {
    // Only this test reads these variables, so the change does not race.
    let keep: Vec<_> = ["SWE_DIAGRAMS_ICONS", "XDG_CACHE_HOME", "HOME"].map(|k| (k, std::env::var_os(k))).into();
    std::env::set_var("SWE_DIAGRAMS_ICONS", "/a/icons");
    std::env::set_var("XDG_CACHE_HOME", "/b");
    std::env::set_var("HOME", "/c");
    assert_eq!(default_dir(), Some(PathBuf::from("/a/icons")));
    std::env::set_var("SWE_DIAGRAMS_ICONS", "");
    assert_eq!(default_dir(), Some(PathBuf::from("/b/swe-diagrams/icons")));
    std::env::remove_var("XDG_CACHE_HOME");
    assert_eq!(default_dir(), Some(PathBuf::from("/c/.cache/swe-diagrams/icons")));
    std::env::remove_var("HOME");
    assert_eq!(default_dir(), None);
    for (k, v) in keep {
        match v {
            Some(v) => std::env::set_var(k, v),
            None => std::env::remove_var(k),
        }
    }
}
