//! Layout for sequence diagrams. Participants are columns in the order of
//! declaration. Messages are rows in the order of declaration.

use crate::model::{idiv, Diagram};
use crate::text::{measure, measure_mono};

pub struct SeqLayout {
    pub w: i64,
    pub h: i64,
    pub head_h: i64,
    /// Centre of each item, by item index.
    pub cx: Vec<i64>,
    /// Head width of each item.
    pub pw: Vec<i64>,
    /// y of each message.
    pub my: Vec<i64>,
    pub label_w: Vec<i64>,
}

const ROW: i64 = 34;
const SELF_ROW: i64 = 44;
const LOOP: i64 = 28; // width of the loop of a message to self

pub fn layout_seq(d: &Diagram) -> SeqLayout {
    let order = &d.roots;
    let n = d.items.len();
    let np = order.len();
    let mut pos = vec![0usize; n];
    let mut pw = vec![0i64; n];
    let mut head_h = 38;
    for (k, &i) in order.iter().enumerate() {
        let it = &d.items[i];
        pos[i] = k;
        let mut w = (measure(&it.label, 13, true).max(measure_mono(&it.sub, 95)) + 28).max(96);
        w += w & 1;
        pw[i] = w;
        if !it.sub.is_empty() {
            head_h = 52;
        }
    }

    // Distance between the centres of two adjacent columns.
    let mut gap: Vec<i64> = order.windows(2).map(|p| idiv(pw[p[0]] + pw[p[1]], 2) + 32).collect();
    let mut extra_right = 0;
    let mut label_w: Vec<i64> = Vec::with_capacity(d.edges.len());
    for e in &d.edges {
        let lw = if e.label.is_empty() { 0 } else { measure_mono(&e.label, 105) };
        label_w.push(lw);
        let a = pos[e.from];
        let b = pos[e.to];
        if a == b {
            let need = LOOP + 8 + lw + 16;
            if a == np - 1 {
                if need > extra_right {
                    extra_right = need;
                }
            } else if gap[a] < need {
                gap[a] = need;
            }
            continue;
        }
        let lo = a.min(b);
        let hi = a.max(b);
        let span: i64 = gap[lo..hi].iter().sum();
        let need = lw + 28;
        if span < need {
            let cols = (hi - lo) as i64;
            let per = idiv(need - span + cols - 1, cols);
            for g in &mut gap[lo..hi] {
                *g += per;
            }
        }
    }

    let mut cx = vec![0i64; n];
    let mut x = if np > 0 { idiv(pw[order[0]], 2) } else { 0 };
    for (k, &i) in order.iter().enumerate() {
        cx[i] = x;
        if k + 1 < np {
            x += gap[k];
        }
    }
    let w = if np > 0 { x + idiv(pw[order[np - 1]], 2).max(extra_right) } else { 0 };

    let mut my: Vec<i64> = Vec::with_capacity(d.edges.len());
    let mut y = head_h + 34;
    for e in &d.edges {
        my.push(y);
        y += if e.from == e.to { SELF_ROW } else { ROW };
    }
    SeqLayout { w, h: y - 8, head_h, cx, pw, my, label_w }
}
