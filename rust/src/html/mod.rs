//! The HTML output: one file with the inline SVG, a stylesheet, and a
//! script. It loads nothing from the network, so it works offline and as a
//! Claude artifact.

use std::fmt::Write;

use crate::render::{Rendered, Step};
use crate::svg::{esc, BADGES};
use crate::theme::{apply_theme, page_theme, Theme};

const CSS: &str = include_str!("viewer.css");
const JS: &str = include_str!("viewer.js");

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

/// The data that the script and later features read. The zoom and the
/// animation features fill `levels` and `stories`.
pub fn data_json(r: &Rendered) -> String {
    let order: Vec<String> = r
        .order
        .iter()
        .map(|s| match s {
            Step::Msg(i) => format!("{{\"k\":\"msg\",\"i\":{i}}}"),
            Step::Node(id) => format!("{{\"k\":\"node\",\"id\":{}}}", json_str(id)),
        })
        .collect();
    format!(
        "{{\"type\":{},\"title\":{},\"levels\":[],\"stories\":[],\"order\":[{}]}}",
        json_str(r.dtype.name()),
        json_str(&r.title),
        order.join(",")
    )
}

/// The HTML page of a diagram. `Auto` follows the system setting and the
/// `data-theme` attribute of the root element. `Light` and `Dark` fix the
/// palette of the diagram and of the page.
pub fn html(r: &Rendered, theme: Theme) -> String {
    let svg = inline_badges(&match theme {
        Theme::Auto => page_theme(&r.svg),
        t => apply_theme(&r.svg, t),
    });
    let fixed = match theme {
        Theme::Auto => "",
        Theme::Light => " class=\"sd-light\"",
        Theme::Dark => " class=\"sd-dark\"",
    };
    let title = if r.title.is_empty() { format!("{} diagram", r.dtype.name()) } else { r.title.clone() };
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>{CSS}</style>\n</head>\n<body{fixed}>\n<main class=\"sd-view\" tabindex=\"0\" aria-label=\"Diagram. Drag to pan, wheel or plus and minus keys to zoom, arrow keys to step.\">\n{}</main>\n<div class=\"sd-tools\" role=\"toolbar\" aria-label=\"Zoom\"><button type=\"button\" data-act=\"in\" aria-label=\"Zoom in\">+</button><button type=\"button\" data-act=\"out\" aria-label=\"Zoom out\">&#8722;</button><button type=\"button\" data-act=\"fit\">fit</button></div>\n<script type=\"application/json\" id=\"sd-data\">{}</script>\n<script>{JS}</script>\n</body>\n</html>\n",
        esc(&title),
        svg.trim_end(),
        data_json(r)
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
