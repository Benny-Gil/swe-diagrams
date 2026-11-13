// HTML tests. The page holds the SVG, the data, and the viewer, and it
// loads nothing from the network.

use std::fs;
use std::path::{Path, PathBuf};

use swe_diagrams::{html, render, IconOptions, Theme};

fn spec(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("spec").join(dir)
}

fn page(name: &str, theme: Theme) -> String {
    let opts = IconOptions { dir: Some(spec("icons")), offline: true };
    let src = fs::read_to_string(spec(&format!("fixtures/{name}.sd"))).unwrap();
    html(&render(&src, &opts).unwrap(), theme)
}

#[test]
fn the_page_is_self_contained() {
    for name in ["arch-checkout", "arch-logos", "seq-login", "timeline-rollout", "class-shapes"] {
        let out = page(name, Theme::Auto);
        assert!(out.starts_with("<!doctype html>"), "{name}");
        // The only URL is the SVG namespace, which loads nothing.
        let urls = out.matches("http").count();
        assert_eq!(urls, out.matches("http://www.w3.org/2000/svg").count(), "{name} has a URL");
        assert!(!out.contains(" src="), "{name} loads a file");
        assert!(!out.contains("<use "), "{name} keeps a <use>");
        assert_eq!(out.matches("<script").count(), 2, "{name}");
    }
}

#[test]
fn the_data_has_the_type_the_title_and_the_order() {
    let out = page("seq-login", Theme::Auto);
    assert!(out.contains(
        "<script type=\"application/json\" id=\"sd-data\">{\"type\":\"seq\",\"title\":\"Login with refresh token\",\"levels\":[],\"stories\":[],\"order\":[{\"k\":\"msg\",\"i\":0},"
    ));
    let out = page("plan-ratelimit", Theme::Auto);
    assert!(out.contains("\"type\":\"plan\""));
    assert!(out.contains("\"order\":[{\"k\":\"node\",\"id\":"));
}

#[test]
fn the_theme_follows_the_page_attribute() {
    let auto = page("arch-checkout", Theme::Auto);
    assert!(auto.contains(":where(:root:not([data-theme=light])) .pg{fill:#0e1217}"));
    assert!(auto.contains(":where(:root[data-theme=dark]) .pg{fill:#0e1217}"));
    let light = page("arch-checkout", Theme::Light);
    assert!(!light.contains(".pg{fill:#0e1217}"));
    assert!(light.contains("<body class=\"sd-light\">"));
    let dark = page("arch-checkout", Theme::Dark);
    assert!(dark.contains("<body class=\"sd-dark\">"));
}

#[test]
fn the_viewer_script_is_small() {
    let js = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/html/viewer.js")).unwrap();
    assert!(js.len() < 10 * 1024, "the viewer script has {} bytes", js.len());
}

fn levels_page(name: &str) -> String {
    let opts = IconOptions { dir: Some(spec("icons")), offline: true };
    let path = spec(&format!("fixtures/{name}.sd"));
    let src = fs::read_to_string(&path).unwrap();
    html(&swe_diagrams::render_path(&src, Some(&path), &opts).unwrap(), Theme::Auto)
}

#[test]
fn the_data_holds_each_level() {
    let out = levels_page("arch-zoom");
    for path in ["\"path\":\"\"", "\"path\":\"cart\"", "\"path\":\"cart/svc\"", "\"path\":\"pay\""] {
        assert!(out.contains(path), "{path}");
    }
    assert!(out.contains("\"title\":\"Cart types\",\"type\":\"class\""));
    // The top level is in the page, so its data has no copy.
    assert!(out.contains(
        "\"path\":\"\",\"title\":\"Checkout\",\"type\":\"arch\",\"order\":[{\"k\":\"node\",\"id\":\"user\"}"
    ));
    assert!(out.contains("\"svg\":null"));
    // The levels in the data have the page theme and no <use>.
    assert!(!out.contains("<use "));
    assert!(out.contains(":where(:root[data-theme=dark]) .zm{fill:#161b21;stroke:#569fff}"));
    assert_eq!(out.matches("<script").count(), 2);
    assert_eq!(out.matches("</script>").count(), 2);
    assert!(out.contains("<nav class=\"sd-crumbs\" aria-label=\"Levels\" hidden></nav>"));
    assert!(out.contains(" data-z=\"1\""));
}

#[test]
fn a_page_can_open_at_a_level() {
    let opts = IconOptions { dir: Some(spec("icons")), offline: true };
    let path = spec("fixtures/plan-zoom.sd");
    let r = swe_diagrams::render_path(&fs::read_to_string(&path).unwrap(), Some(&path), &opts).unwrap();
    let out = swe_diagrams::html_at(&r, Theme::Auto, "build");
    assert!(out.contains(",\"start\":\"build\"}</script>"));
    assert!(!html(&r, Theme::Auto).contains("\"start\""));
}

#[test]
fn the_levels_script_is_small() {
    let js = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/html/levels.js")).unwrap();
    assert!(js.len() < 8 * 1024, "the levels script has {} bytes", js.len());
}
