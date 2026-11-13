//! Node shapes of the graph types that draw a shape for each kind. The
//! layout asks this module for the size of a shape, and the SVG writer asks
//! it for the drawing.

use std::fmt::Write;

use crate::model::{idiv, Diagram, DiagramType, Item};
use crate::svg::{card_text, esc, st};
use crate::text::{measure, measure_mono};

const SKEW: i64 = 14; // horizontal offset of the top side of an `:io` shape
const LOOP: i64 = 26; // height of a self-transition loop
const LOOP_W: i64 = 24; // width of a self-transition loop on the right side
const CAP: i64 = 7; // height of the top ellipse of a cylinder

/// The style rules of the thick side of a source or a sink, light theme
/// then dark theme.
pub const DATAFLOW_STYLE: (&str, &str) = (
    ".tk{fill:none;stroke:#535961;stroke-width:4;stroke-linecap:round}.pl{fill:#fff;stroke:#c3c8ce}.el.ip{stroke-width:0}",
    ".tk{stroke:#a0a5ac}.pl{fill:#161b21;stroke:#3c434c}",
);

/// The style rules of the state dots, light theme then dark theme.
pub const STATE_STYLE: (&str, &str) =
    (".dt{fill:#192029}.ri{fill:none;stroke:#192029;stroke-width:1.5}", ".dt{fill:#e8ebf1}.ri{stroke:#e8ebf1}");

/// True for the types that this module draws.
pub fn draws(t: DiagramType) -> bool {
    matches!(t, DiagramType::Deps | DiagramType::Flow | DiagramType::State | DiagramType::Dataflow)
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

/// The labels of the self-transitions of item `i`, joined, or empty.
pub fn loop_label(d: &Diagram, i: usize) -> String {
    let labels: Vec<&str> =
        d.edges.iter().filter(|e| e.from == i && e.to == i && !e.label.is_empty()).map(|e| e.label.as_str()).collect();
    labels.join(", ")
}

/// The first self-transition of item `i`.
pub fn first_loop(d: &Diagram, i: usize) -> Option<usize> {
    d.edges.iter().position(|e| e.from == i && e.to == i)
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
        (DiagramType::State, "initial") => (22, 22),
        (DiagramType::State, "final") => (26, 26),
        (DiagramType::State, "choice") => (30, 30),
        (DiagramType::State, _) => {
            let mut w = (tw + 36).max(88);
            if !d.down && first_loop(d, i).is_some() {
                w = w.max(measure_mono(&loop_label(d, i), 105) + 12);
            }
            (w, th)
        }
        (DiagramType::Dataflow, "store") => ((tw + 32).max(96), th + 2 * CAP),
        (DiagramType::Dataflow, "source" | "sink" | "queue") => ((tw + 44).max(96), th),
        _ => ((tw + 32).max(96), th),
    };
    (w + (w & 1), h + (h & 1))
}

/// Cross room before and after the shape. A self-transition needs room on
/// the top side, or on the right side when the layers go down.
pub fn room(d: &Diagram, i: usize) -> (i64, i64) {
    if d.dtype != DiagramType::State || first_loop(d, i).is_none() {
        return (0, 0);
    }
    let label = loop_label(d, i);
    if d.down {
        let lw = if label.is_empty() { 0 } else { measure_mono(&label, 105) + 8 };
        (0, LOOP_W + 8 + lw)
    } else {
        (LOOP + if label.is_empty() { 4 } else { 18 }, 0)
    }
}

/// True for a shape with slanted sides, where an edge end must move to the
/// outline.
pub fn slanted(d: &Diagram, i: usize) -> bool {
    matches!((d.dtype, kind(d, &d.items[i])), (DiagramType::Flow, "decision" | "io") | (DiagramType::State, "choice"))
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
        (DiagramType::Flow, "decision") | (DiagramType::State, "choice") => {
            let pts = format!("M{cx} {y}L{} {cy}L{cx} {}L{x} {cy}z", x + w, y + h);
            poly(&mut out, &pts);
            if d.dtype == DiagramType::Flow {
                out.push_str(&text);
            }
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
        (DiagramType::State, "initial") => {
            let _ = write!(out, "<circle class=\"dt\" cx=\"{cx}\" cy=\"{cy}\" r=\"{}\"/>", idiv(w, 2) - 2);
            return (out, x + w, y);
        }
        (DiagramType::State, "final") => {
            let r = idiv(w, 2);
            let _ = write!(
                out,
                "<circle class=\"ri\" cx=\"{cx}\" cy=\"{cy}\" r=\"{}\"/><circle class=\"dt\" cx=\"{cx}\" cy=\"{cy}\" r=\"{}\"/>",
                r - 1,
                r - 6
            );
            return (out, x + w, y);
        }
        (DiagramType::State, _) => {
            rect(&mut out, "", 16);
            out.push_str(&text);
        }
        (DiagramType::Dataflow, "store") => {
            // A cylinder: the body, then the front half of the top ellipse.
            let b = y + h - CAP;
            let rx = idiv(w, 2);
            let pts = format!("M{x} {}A{rx} {CAP} 0 0 1 {} {}V{b}A{rx} {CAP} 0 0 1 {x} {b}z", y + CAP, x + w, y + CAP);
            poly(&mut out, &pts);
            let _ =
                write!(out, "<path class=\"cd{s}\" d=\"M{x} {}A{rx} {CAP} 0 0 0 {} {}\"/>", y + CAP, x + w, y + CAP);
            out.push_str(&card_text(&it.label, &it.sub, it.status, cx, cy + idiv(CAP, 2)));
            return (out, x + w - 2, y + CAP);
        }
        (DiagramType::Dataflow, "source") => {
            rect(&mut out, "", 4);
            let _ = write!(out, "<path class=\"tk\" d=\"M{} {}v{}\"/>", x + 1, y + 3, h - 6);
            out.push_str(&card_text(&it.label, &it.sub, it.status, cx + 4, cy));
        }
        (DiagramType::Dataflow, "sink") => {
            rect(&mut out, "", 4);
            let _ = write!(out, "<path class=\"tk\" d=\"M{} {}v{}\"/>", x + w - 1, y + 3, h - 6);
            out.push_str(&card_text(&it.label, &it.sub, it.status, cx - 4, cy));
        }
        (DiagramType::Dataflow, "queue") => {
            // The two bars at the right end are the slots of a buffer.
            rect(&mut out, "", 4);
            let _ = write!(out, "<path class=\"cd{s}\" d=\"M{} {y}v{h}M{} {y}v{h}\"/>", x + w - 16, x + w - 9);
            out.push_str(&card_text(&it.label, &it.sub, it.status, cx - 8, cy));
        }
        (DiagramType::Dataflow, _) => {
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

/// The loop of the self-transitions of item `i`, on the shape box `b`.
/// The classes give the colour of the line and of the arrowhead.
pub fn self_loop(d: &Diagram, i: usize, b: [i64; 4], line: &str, head: &str, lm: &str) -> String {
    let [x, y, w, h] = b;
    let label = loop_label(d, i);
    let mut out = String::new();
    if d.down {
        // The loop is on the right side, and the label is at its right.
        let r = x + w;
        let cy = y + idiv(h, 2);
        let _ = write!(
            out,
            "<path class=\"{line}\" d=\"M{r} {}C{} {} {} {} {} {}\"/><path class=\"{head}\" d=\"M{} {}l7 -3v6z\"/>",
            cy - 9,
            r + LOOP_W + 4,
            cy - 18,
            r + LOOP_W + 4,
            cy + 18,
            r + 9,
            cy + 9,
            r + 2,
            cy + 9
        );
        if !label.is_empty() {
            let _ = write!(
                out,
                "<text class=\"el ls mo{lm}\" x=\"{}\" y=\"{}\">{}</text>",
                r + LOOP_W + 8,
                cy + 4,
                esc(&label)
            );
        }
    } else {
        // The loop is on the top side, and the label is above it.
        let cx = x + idiv(w, 2);
        let _ = write!(
            out,
            "<path class=\"{line}\" d=\"M{} {y}C{} {} {} {} {} {}\"/><path class=\"{head}\" d=\"M{} {}l-3 -7h6z\"/>",
            cx - 9,
            cx - 18,
            y - LOOP - 4,
            cx + 18,
            y - LOOP - 4,
            cx + 9,
            y - 9,
            cx + 9,
            y - 2
        );
        if !label.is_empty() {
            let _ = write!(out, "<text class=\"el mo{lm}\" x=\"{cx}\" y=\"{}\">{}</text>", y - LOOP - 2, esc(&label));
        }
    }
    out
}
