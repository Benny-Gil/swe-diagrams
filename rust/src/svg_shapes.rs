//! Node shapes of the graph types that draw a shape for each kind. The
//! layout asks this module for the size of a shape, and the SVG writer asks
//! it for the drawing.

use std::fmt::Write;

use crate::model::{idiv, Diagram, DiagramType, Item};
use crate::svg::{card_text, st};
use crate::text::{measure, measure_mono};

/// True for the types that this module draws.
pub fn draws(t: DiagramType) -> bool {
    matches!(t, DiagramType::Deps)
}

/// The kind word of an item.
fn kind(d: &Diagram, it: &Item) -> &'static str {
    d.dtype.kinds()[it.kind]
}

fn text_w(it: &Item) -> i64 {
    measure(&it.label, 13, true).max(measure_mono(&it.sub, 95))
}

fn text_h(it: &Item) -> i64 {
    if it.sub.is_empty() {
        38
    } else {
        52
    }
}

/// Width and height of the shape of item `i`.
pub fn size(d: &Diagram, i: usize) -> (i64, i64) {
    let it = &d.items[i];
    let tw = text_w(it);
    let th = text_h(it);
    let (w, h) = ((tw + 32).max(96), th);
    (w + (w & 1), h + (h & 1))
}

/// The shape and the text of item `i` in the box `(x, y, w, h)`. The
/// result also gives the centre of the status badge.
pub fn node(d: &Diagram, i: usize, x: i64, y: i64, w: i64, h: i64) -> (String, i64, i64) {
    let it = &d.items[i];
    let s = st(it.status);
    let cx = x + idiv(w, 2);
    let cy = y + idiv(h, 2);
    let mut out = String::new();
    let text = card_text(&it.label, &it.sub, it.status, cx, cy);
    let rect = |out: &mut String, extra: &str, rx: i64| {
        let _ = write!(
            out,
            "<rect class=\"sh\" x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\"/><rect class=\"cd{extra}{s}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\"/>",
            y + 1
        );
    };
    let badge = (x + w - 2, y + 2);
    match (d.dtype, kind(d, it)) {
        (DiagramType::Deps, "ext") => {
            rect(&mut out, " x", 8);
            out.push_str(&text);
        }
        _ => {
            rect(&mut out, "", 8);
            out.push_str(&text);
        }
    }
    (out, badge.0, badge.1)
}
