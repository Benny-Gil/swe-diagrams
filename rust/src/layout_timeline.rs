//! Layout for timeline diagrams. The items sit on one horizontal axis in
//! line order. The labels go above and below the axis in turn. A group is
//! a phase band under the labels.

use crate::model::{idiv, Diagram};
use crate::text::{measure, measure_mono};

/// The y of the axis. The labels above the axis need this space.
pub const AXIS_Y: i64 = 46;
/// The y of the top of the phase bands.
pub const BAND_Y: i64 = AXIS_Y + 58;
pub const BAND_H: i64 = 26;
/// The space between the axis ends and the first and the last item.
pub const AXIS_PAD: i64 = 28;

/// The smallest space between two neighbour items.
const MIN_GAP: i64 = 64;
/// The clear space at each side of a label. The text widths are estimates,
/// so the space is large.
const LABEL_PAD: i64 = 24;
/// The extra space next to an item in progress. Its "now" text is on the
/// other side of the axis, in the row of the neighbour labels.
const NOW_PAD: i64 = 20;
/// The space between a band end and its first or last item.
const BAND_PAD: i64 = 22;
/// The clear space between two bands.
const BAND_GAP: i64 = 8;

pub struct Band {
    /// The item index of the group.
    pub item: usize,
    pub x: i64,
    pub w: i64,
}

pub struct TimelineLayout {
    pub w: i64,
    pub h: i64,
    /// The item indices of the markers, from left to right.
    pub order: Vec<usize>,
    /// The centre of each marker, by item index.
    pub x: Vec<i64>,
    /// True when the label of the item is above the axis, by item index.
    pub above: Vec<bool>,
    pub bands: Vec<Band>,
    pub axis_x0: i64,
    pub axis_x1: i64,
    /// The x of the last item with `done` or `wip`. The axis is solid up to
    /// this point and dashed after it.
    pub solid_to: Option<i64>,
}

/// The width of the label block of an item: the label and the second line.
fn label_w(d: &Diagram, i: usize) -> i64 {
    let it = &d.items[i];
    measure(&it.label, 13, true).max(measure_mono(&it.sub, 95))
}

/// The width that the label of a phase band needs.
fn band_min_w(d: &Diagram, g: usize) -> i64 {
    let it = &d.items[g];
    let badge = if it.status > 0 { 14 } else { 0 };
    measure(&it.label, 12, true) + 24 + badge
}

pub fn layout_timeline(d: &Diagram) -> TimelineLayout {
    let n = d.items.len();
    let order: Vec<usize> = (0..n).filter(|&i| !d.items[i].group).collect();
    let mut x = vec![0i64; n];
    let mut above = vec![false; n];
    let lw: Vec<i64> = (0..n).map(|i| if d.items[i].group { 0 } else { label_w(d, i) }).collect();

    // One pass from left to right. A constraint looks only at the items to
    // the left, so a push moves the item and all items after it.
    let mut bands: Vec<Band> = Vec::new();
    let mut open: Option<(usize, i64)> = None; // (group, band start)
    let mut last_band_end: Option<i64> = None;
    let mut cur = 0i64;
    for (k, &i) in order.iter().enumerate() {
        above[i] = k % 2 == 0;
        if k > 0 {
            let p = order[k - 1];
            cur += MIN_GAP.max(idiv(lw[p].max(lw[i]) + 1, 2) + LABEL_PAD);
            if d.items[p].status == 2 || d.items[i].status == 2 {
                cur += NOW_PAD;
            }
        }
        let phase = d.items[i].parent;
        if open.map(|(g, _)| Some(g)) != Some(phase) {
            if let Some((g, s)) = open.take() {
                let e = (x[order[k - 1]] + BAND_PAD).max(s + band_min_w(d, g));
                bands.push(Band { item: g, x: s, w: e - s });
                last_band_end = Some(e);
            }
            if let Some(g) = phase {
                if let Some(e) = last_band_end {
                    cur = cur.max(e + BAND_GAP + BAND_PAD);
                }
                open = Some((g, cur - BAND_PAD));
            }
        }
        x[i] = cur;
    }
    if let Some((g, s)) = open {
        let e = (cur + BAND_PAD).max(s + band_min_w(d, g));
        bands.push(Band { item: g, x: s, w: e - s });
    }

    let (first, last) = match (order.first(), order.last()) {
        (Some(&a), Some(&b)) => (a, b),
        _ => {
            return TimelineLayout { w: 0, h: 0, order, x, above, bands, axis_x0: 0, axis_x1: 0, solid_to: None };
        }
    };
    let mut axis_x0 = x[first] - AXIS_PAD;
    let mut axis_x1 = x[last] + AXIS_PAD;
    let solid_to = order.iter().rev().find(|&&i| matches!(d.items[i].status, 1 | 2)).map(|&i| x[i]);

    // Move all parts right so that the leftmost part starts at 0.
    let mut left = axis_x0;
    let mut right = axis_x1;
    for &i in &order {
        left = left.min(x[i] - idiv(lw[i] + 1, 2));
        right = right.max(x[i] + idiv(lw[i] + 1, 2));
    }
    for b in &bands {
        left = left.min(b.x);
        right = right.max(b.x + b.w);
    }
    for &i in &order {
        x[i] -= left;
    }
    for b in &mut bands {
        b.x -= left;
    }
    axis_x0 -= left;
    axis_x1 -= left;
    let solid_to = solid_to.map(|s| s - left);
    let h = if bands.is_empty() { AXIS_Y + 46 } else { BAND_Y + BAND_H };
    TimelineLayout { w: right - left, h, order, x, above, bands, axis_x0, axis_x1, solid_to }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn labels_alternate_and_items_keep_the_line_order() {
        let d = parse("timeline\na \"A\"\nb \"B\"\nc \"C\"").unwrap();
        let l = layout_timeline(&d);
        assert_eq!(l.order, [0, 1, 2]);
        assert_eq!(l.above, [true, false, true]);
        assert!(l.x[0] < l.x[1] && l.x[1] < l.x[2]);
        assert_eq!(l.x[1] - l.x[0], MIN_GAP);
    }

    #[test]
    fn a_wide_label_pushes_its_neighbours() {
        let long = "A label that is much wider than the smallest gap";
        let d = parse(&format!("timeline\na\nb \"{long}\"\nc")).unwrap();
        let l = layout_timeline(&d);
        let need = idiv(measure(long, 13, true) + 1, 2) + LABEL_PAD;
        assert_eq!(l.x[1] - l.x[0], need);
        assert_eq!(l.x[2] - l.x[1], need);
    }

    #[test]
    fn bands_do_not_overlap() {
        let d = parse(
            "timeline
group p \"A very long phase label\"
  a
group q \"Another long phase label\"
  b",
        )
        .unwrap();
        let l = layout_timeline(&d);
        assert_eq!(l.bands.len(), 2);
        assert!(l.bands[0].x + l.bands[0].w + BAND_GAP <= l.bands[1].x);
        assert!(l.bands[0].w >= measure("A very long phase label", 12, true) + 24);
    }

    #[test]
    fn the_axis_is_solid_to_the_last_started_item() {
        let d = parse("timeline\na done\nb wip\nc plan\nd").unwrap();
        let l = layout_timeline(&d);
        assert_eq!(l.solid_to, Some(l.x[1]));
        let d = parse("timeline\na plan\nb").unwrap();
        assert_eq!(layout_timeline(&d).solid_to, None);
    }
}
