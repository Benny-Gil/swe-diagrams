//! Places the edge labels of a layout with layers that go down. A label
//! sits at one side of its line, or on the line. The placer tries each
//! side, then moves the label along the line until it touches no other
//! line, node, or label. All values are in (main, cross) coordinates.

/// Half the main size of a label box.
pub const HALF: i64 = 9;
/// Cross distance from the line to a label at its side.
pub const SIDE_GAP: i64 = 8;
const STEP: i64 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub m0: i64,
    pub m1: i64,
    pub c0: i64,
    pub c1: i64,
}

impl Rect {
    fn grow(self, by: i64) -> Rect {
        Rect { m0: self.m0 - by, m1: self.m1 + by, c0: self.c0 - by, c1: self.c1 + by }
    }

    fn meets(&self, o: &Rect) -> bool {
        self.m0 < o.m1 && o.m0 < self.m1 && self.c0 < o.c1 && o.c0 < self.c1
    }
}

/// The box of a label with the cross size `w`, for the line at cross
/// position `line` and the centre `m` on the main axis. Side -1 is before
/// the line, 0 is on the line, and 1 is after the line.
pub fn label_rect(line: i64, m: i64, w: i64, side: i64) -> Rect {
    let (c0, c1) = match side {
        0 => (line - (w >> 1), line + w - (w >> 1)),
        s if s < 0 => (line - SIDE_GAP - w, line - SIDE_GAP),
        _ => (line + SIDE_GAP, line + SIDE_GAP + w),
    };
    Rect { m0: m - HALF, m1: m + HALF, c0, c1 }
}

/// One label to place.
pub struct Want {
    /// Cross position of the line.
    pub line: i64,
    /// The first main centre to try.
    pub m: i64,
    /// The range of main centres along the straight run of the line.
    pub lo: i64,
    pub hi: i64,
    /// Cross size of the label.
    pub w: i64,
    /// The sides to try, in order of preference.
    pub sides: Vec<i64>,
    /// Index of the route of the label in `routes`.
    pub route: usize,
}

/// The place of one label. `clear` is false when each place touches
/// something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub m: i64,
    pub side: i64,
    pub rect: Rect,
    pub clear: bool,
}

/// The number of things that the label box `r` touches. The segment of
/// the own route on the line itself does not count.
fn cost(r: &Rect, own: usize, line: i64, routes: &[&[i64]], boxes: &[Rect], placed: &[Placed]) -> i64 {
    let mut n = 0;
    let near = r.grow(3);
    for (k, pts) in routes.iter().enumerate() {
        for s in pts.as_chunks::<2>().0.windows(2) {
            let ([m1, c1], [m2, c2]) = (s[0], s[1]);
            if k == own && c1 == line && c2 == line {
                continue;
            }
            let seg = Rect { m0: m1.min(m2), m1: m1.max(m2) + 1, c0: c1.min(c2), c1: c1.max(c2) + 1 };
            if near.meets(&seg) {
                n += 1;
            }
        }
    }
    let near = r.grow(4);
    n += boxes.iter().filter(|b| near.meets(b)).count() as i64;
    n += placed.iter().filter(|p| near.meets(&p.rect)).count() as i64;
    n
}

/// Places the labels in list order. Each label avoids the routes, the
/// boxes, and the labels before it.
pub fn place(wants: &[Want], routes: &[&[i64]], boxes: &[Rect]) -> Vec<Placed> {
    let mut out: Vec<Placed> = Vec::with_capacity(wants.len());
    for want in wants {
        let mut best: Option<(i64, Placed)> = None;
        // The main centres to try: the first one, then 4, 8, ... after and
        // before it, in the range.
        let mut centres = vec![want.m];
        let mut d = STEP;
        while want.m + d <= want.hi || want.m - d >= want.lo {
            centres.extend([want.m + d, want.m - d].into_iter().filter(|m| (want.lo..=want.hi).contains(m)));
            d += STEP;
        }
        'search: for &side in &want.sides {
            for &m in &centres {
                let rect = label_rect(want.line, m, want.w, side);
                let n = cost(&rect, want.route, want.line, routes, boxes, &out);
                if best.as_ref().is_none_or(|b| n < b.0) {
                    best = Some((n, Placed { m, side, rect, clear: n == 0 }));
                }
                if n == 0 {
                    break 'search;
                }
            }
        }
        let fallback = Placed {
            m: want.m,
            side: want.sides[0],
            rect: label_rect(want.line, want.m, want.w, want.sides[0]),
            clear: false,
        };
        out.push(best.map_or(fallback, |b| b.1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn want(line: i64, m: i64, sides: Vec<i64>, route: usize) -> Want {
        Want { line, m, lo: m - 40, hi: m + 40, w: 50, sides, route }
    }

    #[test]
    fn a_free_label_keeps_its_first_place() {
        let route: &[i64] = &[0, 100, 200, 100];
        let p = place(&[want(100, 50, vec![1, -1], 0)], &[route], &[]);
        assert_eq!(p[0], Placed { m: 50, side: 1, rect: label_rect(100, 50, 50, 1), clear: true });
    }

    #[test]
    fn a_label_takes_the_other_side_of_a_line_at_its_side() {
        let own: &[i64] = &[0, 100, 200, 100];
        let other: &[i64] = &[0, 130, 200, 130];
        let p = place(&[want(100, 50, vec![1, -1], 0)], &[own, other], &[]);
        assert_eq!((p[0].side, p[0].m, p[0].clear), (-1, 50, true));
    }

    #[test]
    fn a_label_moves_along_its_line_past_a_jog() {
        let own: &[i64] = &[0, 100, 200, 100];
        // A jog crosses the two sides at main position 50.
        let jog: &[i64] = &[50, 0, 50, 300];
        let p = place(&[want(100, 50, vec![1, -1], 0)], &[own, jog], &[]);
        assert!(p[0].clear);
        assert_eq!(p[0].side, 1);
        assert_eq!(p[0].m, 38);
    }

    #[test]
    fn a_label_with_no_clear_place_says_so() {
        let own: &[i64] = &[0, 100, 200, 100];
        let wall = Rect { m0: -100, m1: 300, c0: 0, c1: 300 };
        let p = place(&[want(100, 50, vec![1, -1], 0)], &[own], &[wall]);
        assert!(!p[0].clear);
        assert_eq!((p[0].m, p[0].side), (50, 1));
    }
}
