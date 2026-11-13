//! Node shapes of the graph types that draw a shape for each kind. The
//! layout asks this module for the size of a shape, and the SVG writer asks
//! it for the drawing.

use std::fmt::Write;

use crate::model::{idiv, Diagram, DiagramType, Item};
use crate::svg::{card_text, st};
use crate::text::{measure, measure_mono};

const SKEW: i64 = 14; // horizontal offset of the top side of an `:io` shape

/// True for the types that this module draws.
pub fn draws(t: DiagramType) -> bool {
    matches!(t, DiagramType::Deps | DiagramType::Flow)
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
    let (w, h) = match (d.dtype, kind(d, it)) {
        (DiagramType::Flow, "start" | "end") => ((tw + 40).max(96), th),
        (DiagramType::Flow, "io") => ((tw + 32 + SKEW).max(96), th),
        (DiagramType::Flow, "sub") => ((tw + 48).max(96), th),
        (DiagramType::Flow, "decision") => {
            // A text box fits in the diamond when tw / w + ih / h <= 1.
            let h = th + 26;
            let ih = th - 20;
            ((idiv(tw * h + h - ih - 1, h - ih) + 16).max(96), h)
        }
        _ => ((tw + 32).max(96), th),
    };
    (w + (w & 1), h + (h & 1))
}

/// True for a shape with slanted sides, where an edge end must move to the
/// outline.
pub fn slanted(d: &Diagram, i: usize) -> bool {
    matches!((d.dtype, kind(d, &d.items[i])), (DiagramType::Flow, "decision" | "io"))
}

/// Main distance from the side of the box `b` to the outline, at the edge
/// end `(px, py)`.
pub fn outline_inset(d: &Diagram, i: usize, b: [i64; 4], px: i64, py: i64) -> i64 {
    let [x, y, w, h] = b;
    if kind(d, &d.items[i]) == "io" {
        // Only the left and the right sides slant.
        if d.down {
            return 0;
        }
        return if px <= x + idiv(w, 2) { idiv(SKEW * (y + h - py), h) } else { idiv(SKEW * (py - y), h) };
    }
    // A diamond: the outline is at |dx| / (w / 2) + |dy| / (h / 2) = 1.
    if d.down {
        idiv(h * (px - x - idiv(w, 2)).abs(), w)
    } else {
        idiv(w * (py - y - idiv(h, 2)).abs(), h)
    }
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
    let poly = |out: &mut String, pts: &str| {
        let _ = write!(
            out,
            "<path class=\"sh\" transform=\"translate(0 1)\" d=\"{pts}\"/><path class=\"cd{s}\" d=\"{pts}\"/>"
        );
    };
    let badge = (x + w - 2, y + 2);
    match (d.dtype, kind(d, it)) {
        (DiagramType::Deps, "ext") => {
            rect(&mut out, " x", 8);
            out.push_str(&text);
        }
        (DiagramType::Flow, "start" | "end") => {
            rect(&mut out, "", idiv(h, 2));
            out.push_str(&text);
        }
        (DiagramType::Flow, "decision") => {
            let pts = format!("M{cx} {y}L{} {cy}L{cx} {}L{x} {cy}z", x + w, y + h);
            poly(&mut out, &pts);
            out.push_str(&text);
            return (out, x + idiv(3 * w, 4), y + idiv(h, 4));
        }
        (DiagramType::Flow, "io") => {
            let pts = format!("M{} {y}H{}L{} {}H{x}z", x + SKEW, x + w, x + w - SKEW, y + h);
            poly(&mut out, &pts);
            out.push_str(&text);
        }
        (DiagramType::Flow, "sub") => {
            rect(&mut out, "", 4);
            let _ = write!(out, "<path class=\"cd{s}\" d=\"M{} {y}v{h}M{} {y}v{h}\"/>", x + 8, x + w - 8);
            out.push_str(&text);
        }
        (DiagramType::Flow, _) => {
            rect(&mut out, "", 4);
            out.push_str(&text);
        }
        _ => {
            rect(&mut out, "", 8);
            out.push_str(&text);
        }
    }
    (out, badge.0, badge.1)
}
