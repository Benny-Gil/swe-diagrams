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
        "<script type=\"application/json\" id=\"sd-data\">{\"type\":\"seq\",\"title\":\"Login with refresh token\",\"levels\":[],\"stories\":[{\"title\":\"Messages\",\"mode\":\"steps\","
    ));
    assert!(out.contains("\"variants\":{},\"order\":[{\"k\":\"msg\",\"i\":0},"));
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

#[test]
fn the_stories_have_steps_with_times() {
    let out = page("arch-story", Theme::Auto);
    assert!(out.contains("\"stories\":[{\"title\":\"Add an item\",\"mode\":\"steps\",\"total\":4350,\"beats\":[{\"t\":0,\"d\":400,\"moves\":[{\"i\":0,\"back\":false,\"hollow\":false,\"cap\":\"click Add\",\"label\":\"\"}],\"node\":null},"));
    // The parallel steps of "Cache read" are one beat.
    assert!(out.contains("{\"title\":\"Cache read\",\"mode\":\"steps\",\"total\":2542,\"beats\":[{\"t\":0,\"d\":542,\"moves\":[{\"i\":2,"));
    assert!(out.contains("<div class=\"sd-player\""));
    assert!(out.contains("Space plays the story."));
}

#[test]
fn a_plan_page_has_the_status_drawings() {
    let out = page("plan-ratelimit", Theme::Auto);
    assert!(out.contains("\"mode\":\"progress\""));
    assert!(out.contains(
        "\"variants\":{\"spec\":[\"\\u003cg data-k=\\\"node\\\" data-id=\\\"spec\\\" data-s=\\\"plan\\\"\\u003e"
    ));
}

#[test]
fn a_diagram_with_no_story_has_no_player() {
    let out = page("erd-orders", Theme::Auto);
    assert!(out.contains("\"stories\":[]"));
    assert!(!out.contains("sd-player"));
}

#[test]
fn the_player_script_is_small() {
    let js = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/html/player.js")).unwrap();
    assert!(js.len() < 10 * 1024, "the player script has {} bytes", js.len());
}
