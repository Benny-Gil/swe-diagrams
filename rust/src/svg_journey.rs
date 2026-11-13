//! Page cards of the `journey` type. A page is a small browser frame: a
//! bar with three dots and the route, and the page title below the bar.
//! A modal is a smaller frame with a close mark. An external page has a
//! dashed frame.

use std::fmt::Write;

use crate::model::{idiv, Item};
use crate::svg::{esc, quiet, st};
use crate::text::{measure, measure_mono};

/// Height of the bar that holds the route.
const BAR: i64 = 20;
/// Cross room before the route field: the three dots.
const DOTS: i64 = 36;

/// The style rules of the bar, light theme then dark theme.
pub const STYLE: (&str, &str) = (
    ".jb{fill:#192029;fill-opacity:.05}.jd{fill:#c3c8ce}.ju{fill:#fff;stroke:#dde1e6}.jr{text-anchor:start}.jx{fill:none;stroke:#535961;stroke-width:1.5;stroke-linecap:round}",
    ".jb{fill:#fff;fill-opacity:.05}.jd{fill:#3c434c}.ju{fill:#0e1217;stroke:#3c434c}.jx{stroke:#a0a5ac}",
);

/// Room at the right end of the bar of a modal, so the status badge does
/// not cover the close mark.
fn badge_room(it: &Item) -> i64 {
    if it.status > 0 {
        10
    } else {
        0
    }
}

/// Width and height of the card of a page.
pub fn size(it: &Item, kind: &str) -> (i64, i64) {
    let title = measure(&it.label, 13, true);
    let route = measure_mono(&it.sub, 95);
    let (w, h) = if kind == "modal" {
        // The route field fills the bar up to the close mark.
        ((title + 32).max(route + 44 + badge_room(it)).max(112), BAR + 34)
    } else {
        ((title + 36).max(route + DOTS + 24).max(144), BAR + 44)
    };
    (w + (w & 1), h)
}

/// The card of a page in the box `(x, y, w, h)`, and the centre of the
/// status badge.
pub fn node(it: &Item, kind: &str, x: i64, y: i64, w: i64, h: i64) -> (String, i64, i64) {
    let s = st(it.status);
    let modal = kind == "modal";
    let ext = if kind == "external" { " x" } else { "" };
    let rx = if modal { 6 } else { 8 };
    let mut out = String::new();
    let _ = write!(
        out,
        "<rect class=\"sh\" x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\"/><rect class=\"cd{ext}{s}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\"/>",
        y + 1
    );
    // The bar: a shade with round top corners, and a line below it.
    let _ = write!(
        out,
        "<path class=\"jb\" d=\"M{} {}V{}a{rx} {rx} 0 0 1 {rx} -{rx}H{}a{rx} {rx} 0 0 1 {rx} {rx}V{}z\"/><path class=\"cd{ext}{s}\" d=\"M{x} {}h{w}\"/>",
        x + 1,
        y + BAR,
        y + rx,
        x + w - rx,
        y + BAR,
        y + BAR
    );
    let (field_x, field_w) = if modal {
        // A close mark at the right end of the bar.
        let (mx, my) = (x + w - 12 - badge_room(it), y + idiv(BAR, 2));
        let _ = write!(out, "<path class=\"jx\" d=\"M{} {}l6 6M{} {}l-6 6\"/>", mx - 3, my - 3, mx + 3, my - 3);
        (x + 6, w - 30 - badge_room(it))
    } else {
        for k in 0..3 {
            let _ =
                write!(out, "<circle class=\"jd\" cx=\"{}\" cy=\"{}\" r=\"2.5\"/>", x + 10 + 8 * k, y + idiv(BAR, 2));
        }
        (x + DOTS, w - DOTS - 8)
    };
    let _ =
        write!(out, "<rect class=\"ju\" x=\"{field_x}\" y=\"{}\" width=\"{field_w}\" height=\"12\" rx=\"6\"/>", y + 4);
    if !it.sub.is_empty() {
        let _ = write!(out, "<text class=\"ro mo jr\" x=\"{}\" y=\"{}\">{}</text>", field_x + 7, y + 13, esc(&it.sub));
    }
    let p = quiet(it.status);
    let _ = write!(
        out,
        "<text class=\"nm{p}\" x=\"{}\" y=\"{}\">{}</text>",
        x + idiv(w, 2),
        y + BAR + idiv(h - BAR, 2) + 5,
        esc(&it.label)
    );
    (out, x + w - 2, y + 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn a_modal_is_smaller_than_a_page() {
        let d = parse("journey\na \"Cart\" \"/cart\"\nb \"Cart\" \"/cart\" :modal\n").unwrap();
        let (pw, ph) = size(&d.items[0], "page");
        let (mw, mh) = size(&d.items[1], "modal");
        assert!(mw < pw && mh < ph);
    }

    #[test]
    fn each_kind_has_its_frame() {
        let d = parse(
            "journey\na \"Home\" \"/\"\nb \"Login\" \"/login\" :modal\nc \"Docs\" \"docs.example.com\" :external\n",
        )
        .unwrap();
        let draw = |i: usize| node(&d.items[i], d.dtype.kinds()[d.items[i].kind], 0, 0, 160, 64).0;
        assert!(draw(0).contains("class=\"jd\"") && !draw(0).contains("jx"));
        assert!(draw(1).contains("class=\"jx\"") && !draw(1).contains("jd"));
        assert!(draw(2).contains("class=\"cd x\""));
        assert!(draw(0).contains(">/</text>"));
    }
}
