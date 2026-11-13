//! The HTML output: one file with the inline SVG, a stylesheet, and a
//! script. It loads nothing from the network, so it works offline and as a
//! Claude artifact.

use std::fmt::Write;

use crate::anim::{routes, timing};
use crate::render::{Rendered, Step};
use crate::svg::{esc, BADGES};
use crate::theme::{apply_theme, page_theme, Theme};

const CSS: &str = include_str!("viewer.css");
const JS: &str = include_str!("viewer.js");
const PLAYER_CSS: &str = include_str!("player.css");
const PLAYER_JS: &str = include_str!("player.js");
const PLAYER: &str = "<div class=\"sd-player\" role=\"group\" aria-label=\"Story player\"><select aria-label=\"Story\"></select><button type=\"button\" data-p=\"back\" aria-label=\"Step back\">&#9664;&#9664;</button><button type=\"button\" data-p=\"play\" aria-label=\"Play\">&#9654;</button><button type=\"button\" data-p=\"next\" aria-label=\"Step forward\">&#9654;&#9654;</button><button type=\"button\" data-p=\"speed\" aria-label=\"Speed\">1x</button><p class=\"sd-cap\" aria-live=\"polite\"></p></div>\n";
const LEVELS_JS: &str = include_str!("levels.js");

/// A JSON string. In a script element, `</` must not occur, so `<` is
/// always an escape.
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Puts a copy of the badge in place of each `<use href="#bN" x y/>`. A
/// selector cannot match an ancestor across the shadow tree of a `<use>`,
/// so the theme rules of the page do not reach the content of a `<use>`.
fn inline_badges(svg: &str) -> String {
    const OPEN: &str = "<use href=\"#b";
    let mut out = String::with_capacity(svg.len() * 2);
    let mut rest = svg;
    while let Some(at) = rest.find(OPEN) {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail.find("/>").map_or(tail.len(), |e| e + 2);
        let tag = &tail[..end];
        let attr = |name: &str| {
            let key = format!(" {name}=\"");
            tag.find(&key).and_then(|k| tag[k + key.len()..].split('"').next())
        };
        let status = tag[OPEN.len()..].split('"').next().and_then(|n| n.parse::<usize>().ok());
        match (status.filter(|&n| n < BADGES.len()), attr("x"), attr("y")) {
            (Some(n), Some(x), Some(y)) => {
                let _ = write!(out, "<g data-sd-b=\"{n}\" transform=\"translate({x} {y})\">{}</g>", BADGES[n]);
            }
            _ => out.push_str(tag),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// The stories, with the times at speed 1.
fn stories_json(r: &Rendered) -> String {
    let routes = routes(&r.svg);
    let mut out = Vec::new();
    for s in &r.stories {
        let tm = timing(s, &routes, 1.0);
        let beats: Vec<String> = s
            .beats
            .iter()
            .zip(&tm.beats)
            .map(|(b, &(t, d))| {
                let moves: Vec<String> = b
                    .moves
                    .iter()
                    .map(|m| {
                        format!(
                            "{{\"i\":{},\"back\":{},\"hollow\":{},\"cap\":{},\"label\":{}}}",
                            m.edge,
                            m.back,
                            m.hollow,
                            json_str(&m.caption),
                            json_str(&m.label)
                        )
                    })
                    .collect();
                let node = match &b.node {
                    Some((id, last)) => format!("{{\"id\":{},\"s\":{last}}}", json_str(id)),
                    None => "null".to_string(),
                };
                format!("{{\"t\":{t},\"d\":{d},\"moves\":[{}],\"node\":{node}}}", moves.join(","))
            })
            .collect();
        out.push(format!(
            "{{\"title\":{},\"mode\":\"{}\",\"total\":{},\"beats\":[{}]}}",
            json_str(&s.title),
            s.mode.name(),
            tm.total,
            beats.join(",")
        ));
    }
    format!("[{}]", out.join(","))
}

/// The node groups of each progress node at the statuses planned, in
/// progress, and done, then the status of the source.
fn variants_json(r: &Rendered) -> String {
    let list: Vec<String> = r
        .variants
        .iter()
        .map(|v| {
            format!(
                "{}:[{},{},{},{}]",
                json_str(&v.id),
                json_str(&inline_badges(&v.planned)),
                json_str(&inline_badges(&v.wip)),
                json_str(&inline_badges(&v.done)),
                v.status
            )
        })
        .collect();
    format!("{{{}}}", list.join(","))
}

fn order_json(order: &[Step]) -> String {
    let steps: Vec<String> = order
        .iter()
        .map(|s| match s {
            Step::Msg(i) => format!("{{\"k\":\"msg\",\"i\":{i}}}"),
            Step::Node(id) => format!("{{\"k\":\"node\",\"id\":{}}}", json_str(id)),
        })
        .collect();
    format!("[{}]", steps.join(","))
}

/// The SVG of a level for the page: the theme rules, and a copy of each badge.
fn page_svg(svg: &str, theme: Theme) -> String {
    inline_badges(&match theme {
        Theme::Auto => page_theme(svg),
        t => apply_theme(svg, t),
    })
}

/// The data that the script and later features read. `levels` holds each
/// level when the diagram has more than one. The page holds the SVG of the
/// top level, so its `svg` is null. The animation feature fills `stories`.
pub fn data_json(r: &Rendered) -> String {
    data_with(r, Theme::Auto, "")
}

fn data_with(r: &Rendered, theme: Theme, start: &str) -> String {
    let mut levels: Vec<String> = Vec::new();
    if r.levels.len() > 1 {
        for lv in &r.levels {
            let svg = if lv.path.is_empty() { "null".to_string() } else { json_str(&page_svg(&lv.svg, theme)) };
            levels.push(format!(
                "{{\"path\":{},\"title\":{},\"type\":{},\"order\":{},\"svg\":{svg}}}",
                json_str(&lv.path),
                json_str(&lv.title),
                json_str(lv.dtype.name()),
                order_json(&lv.order)
            ));
        }
    }
    let start = if start.is_empty() { String::new() } else { format!(",\"start\":{}", json_str(start)) };
    format!(
        "{{\"type\":{},\"title\":{},\"levels\":[{}],\"stories\":{},\"variants\":{},\"order\":{}{start}}}",
        json_str(r.dtype.name()),
        json_str(&r.title),
        levels.join(","),
        stories_json(r),
        variants_json(r),
        order_json(&r.order)
    )
}

/// The HTML page of a diagram. `Auto` follows the system setting and the
/// `data-theme` attribute of the root element. `Light` and `Dark` fix the
/// palette of the diagram and of the page.
pub fn html(r: &Rendered, theme: Theme) -> String {
    html_at(r, theme, "")
}

/// The HTML page that opens at level `start` (`cart/svc`) when the address
/// has no hash. An empty path is the top level.
pub fn html_at(r: &Rendered, theme: Theme, start: &str) -> String {
    let svg = page_svg(&r.svg, theme);
    let fixed = match theme {
        Theme::Auto => "",
        Theme::Light => " class=\"sd-light\"",
        Theme::Dark => " class=\"sd-dark\"",
    };
    // A diagram with no story gets no player.
    let play = !r.stories.is_empty();
    let title = if r.title.is_empty() { format!("{} diagram", r.dtype.name()) } else { r.title.clone() };
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>{CSS}{}</style>\n</head>\n<body{fixed}>\n<main class=\"sd-view\" tabindex=\"0\" aria-label=\"Diagram. Drag to pan, wheel or plus and minus keys to zoom, arrow keys to step.\">\n{}</main>\n<div class=\"sd-tools\" role=\"toolbar\" aria-label=\"Zoom\"><button type=\"button\" data-act=\"in\" aria-label=\"Zoom in\">+</button><button type=\"button\" data-act=\"out\" aria-label=\"Zoom out\">&#8722;</button><button type=\"button\" data-act=\"fit\">fit</button></div>\n<nav class=\"sd-crumbs\" aria-label=\"Levels\" hidden></nav>\n{}<script type=\"application/json\" id=\"sd-data\">{}</script>\n<script>{JS}{LEVELS_JS}{}</script>\n</body>\n</html>\n",
        esc(&title),
        if play { PLAYER_CSS } else { "" },
        svg.trim_end(),
        if play { PLAYER } else { "" },
        data_with(r, theme, start),
        if play { PLAYER_JS } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_badge_becomes_a_copy() {
        let out = inline_badges("<a/><use href=\"#b6\" x=\"3\" y=\"-4\"/><b/>");
        assert_eq!(out, format!("<a/><g data-sd-b=\"6\" transform=\"translate(3 -4)\">{}</g><b/>", BADGES[6]));
    }

    #[test]
    fn a_json_string_cannot_close_the_script() {
        assert_eq!(json_str("a</script>\"\\\n"), "\"a\\u003c/script\\u003e\\\"\\\\\\u000a\"");
    }
}
