//! SVG writer for timeline diagrams. It uses the frame and the classes of
//! `svg.rs`, so a timeline has the same look as the other types.

use std::fmt::Write;

use crate::layout_timeline::{TimelineLayout, AXIS_Y, BAND_H, BAND_Y};
use crate::model::Diagram;
use crate::svg::{esc, quiet, st, Frame};

/// The index of `:milestone` in `TIMELINE_KINDS`.
const MILESTONE: usize = 1;

/// The kicker counts the done items when the timeline shows progress.
fn kicker(d: &Diagram) -> String {
    let items = d.items.iter().filter(|it| !it.group);
    let total = items.clone().count();
    if !items.clone().any(|it| matches!(it.status, 1..=3)) {
        return "timeline".to_string();
    }
    let done = items.filter(|it| it.status == 1).count();
    format!("timeline / {done} of {total} done")
}

pub fn render_timeline(d: &Diagram, l: &TimelineLayout) -> String {
    let mut f = Frame::new(&d.title, kicker(d));
    let (ox, oy) = f.origin();
    let ay = oy + AXIS_Y;

    for b in &l.bands {
        let it = &d.items[b.item];
        let x = ox + b.x;
        let y = oy + BAND_Y;
        let gv = if it.vendor { " gv" } else { "" };
        let _ = write!(
            f.body,
            "<rect class=\"g{gv}{}\" x=\"{x}\" y=\"{y}\" width=\"{}\" height=\"{BAND_H}\" rx=\"8\"/>",
            st(it.status),
            b.w
        );
        let gl = if it.vendor { "gl glv" } else { "gl" };
        let _ = write!(f.body, "<text class=\"{gl}\" x=\"{}\" y=\"{}\">{}</text>", x + 12, y + 17, esc(&it.label));
        if it.status > 0 {
            f.badge(it.status, x + b.w - 2, y + 2);
        }
    }

    if !l.order.is_empty() {
        let x0 = ox + l.axis_x0;
        let x1 = ox + l.axis_x1;
        let mut from = x0;
        if let Some(s) = l.solid_to {
            from = ox + s;
            let _ = write!(f.body, "<path class=\"e em\" d=\"M{x0} {ay}H{from}\"/>");
        }
        let _ = write!(f.body, "<path class=\"e d\" d=\"M{from} {ay}H{}\"/>", x1 - 5);
        let _ = write!(f.body, "<path class=\"a\" d=\"M{x1} {ay}l-7 -3v6z\"/>");
    }

    // A "now" mark crosses the axis at each item in progress. Its text is on
    // the other side of the axis from the label of the item.
    for &i in &l.order {
        if d.items[i].status != 2 {
            continue;
        }
        let x = ox + l.x[i];
        let ty = if l.above[i] { ay + 25 } else { ay - 18 };
        let _ = write!(
            f.body,
            "<path class=\"e em\" d=\"M{x} {}V{}\"/><text class=\"el mo lm\" x=\"{x}\" y=\"{ty}\">now</text>",
            ay - 14,
            ay + 14
        );
    }

    for &i in &l.order {
        let it = &d.items[i];
        let x = ox + l.x[i];
        if it.kind == MILESTONE {
            if it.status > 0 {
                f.used[it.status] = true;
            }
            let s = st(it.status);
            let _ = write!(
                f.body,
                "<path class=\"sh\" d=\"M{x} {}l11 11-11 11-11-11z\"/><path class=\"cd{s}\" d=\"M{x} {}l11 11-11 11-11-11z\"/><path class=\"a{s}\" d=\"M{x} {}l4 4-4 4-4-4z\"/>",
                ay - 10,
                ay - 11,
                ay - 4
            );
        } else {
            f.badge(it.status, x, ay);
        }

        // The second line is nearer to the axis than the label.
        let p = quiet(it.status);
        let (name_y, sub_y) = match (l.above[i], it.sub.is_empty()) {
            (true, true) => (ay - 18, 0),
            (true, false) => (ay - 32, ay - 18),
            (false, true) => (ay + 27, 0),
            (false, false) => (ay + 40, ay + 25),
        };
        let _ = write!(f.body, "<text class=\"nm{p}\" x=\"{x}\" y=\"{name_y}\">{}</text>", esc(&it.label));
        if !it.sub.is_empty() {
            let _ = write!(f.body, "<text class=\"ro mo\" x=\"{x}\" y=\"{sub_y}\">{}</text>", esc(&it.sub));
        }
    }
    f.finish(l.w, l.h)
}
