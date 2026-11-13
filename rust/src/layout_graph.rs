//! Layered layout for graph diagrams (all types other than seq).
//!
//! Each container (the root, or a group) gets its own layered layout, from the
//! deepest group up. An edge that leaves a group gets a port on the border of
//! that group, so a route is a chain of pieces, one for each container.
//! The layout works in (main, cross) coordinates. Layers follow the main axis.
//! All values are integers, so each machine gives the same bytes.

use std::collections::HashMap;

use crate::model::{idiv, rdiv, Diagram};
use crate::svg_shapes;
use crate::text::{measure, measure_mono};

const NODE_GAP: i64 = 28; // cross gap between two real nodes
const THIN_GAP: i64 = 12; // cross gap next to an edge that passes a layer
const DUMMY: i64 = 8; // cross size of an edge that passes a layer
const SLOT: i64 = 20; // cross room for one edge end on a node side
const TRACK: i64 = 10; // main distance between two jog tracks
const ARROW: i64 = 16; // straight run before an edge end
const MIN_GAP: i64 = 48; // main gap between two layers
const PAD: i64 = 24; // group padding
const HEAD: i64 = 44; // group padding on the side that holds the label
const TILE: i64 = 60; // side of the icon tile of an arch node

pub struct GEdge {
    pub edge: usize,
    /// x0, y0, x1, y1, ...
    pub pts: Vec<i64>,
    /// 0 when the edge has no label.
    pub label_w: i64,
    /// Centre of the label.
    pub label_x: i64,
    pub label_y: i64,
    pub arrow_start: bool,
    pub arrow_end: bool,
}

pub struct GraphLayout {
    pub w: i64,
    pub h: i64,
    /// One box for each item.
    pub x: Vec<i64>,
    pub y: Vec<i64>,
    pub bw: Vec<i64>,
    pub bh: Vec<i64>,
    /// The size of the icon tile across the layers. 0 for an item with no tile.
    pub tile: Vec<i64>,
    pub edges: Vec<GEdge>,
}

#[derive(Clone, Copy)]
struct Seg {
    e: usize,
    /// Child that holds the edge end. `None` when the group is the end.
    child: Option<usize>,
}

/// Lists of indexes, grouped by a key. The lists share one allocation, and
/// each list keeps the order of the input.
#[derive(Default)]
struct Groups {
    start: Vec<usize>,
    list: Vec<usize>,
}

impl Groups {
    /// Puts index `i` in the list of the i-th key. Each key is less than `n`.
    fn build(&mut self, n: usize, keys: impl Iterator<Item = usize> + Clone) {
        self.start.clear();
        self.start.resize(n + 1, 0);
        let mut len = 0;
        for k in keys.clone() {
            self.start[k + 1] += 1;
            len += 1;
        }
        for k in 0..n {
            self.start[k + 1] += self.start[k];
        }
        self.list.clear();
        self.list.resize(len, 0);
        for (i, k) in keys.enumerate() {
            self.list[self.start[k]] = i;
            self.start[k] += 1;
        }
        // Each start is now the end of its list. Move the values back.
        for k in (0..n).rev() {
            self.start[k + 1] = self.start[k];
        }
        self.start[0] = 0;
    }

    fn get(&self, k: usize) -> &[usize] {
        &self.list[self.start[k]..self.start[k + 1]]
    }
}

// Kinds of layer node.
const CHILD: u8 = 0;
const PASS: u8 = 1; // an edge that passes a layer
const PORT_IN: u8 = 2;
const PORT_OUT: u8 = 3;

/// The layer graph of one container. One value serves all containers, so
/// the vectors keep their memory.
#[derive(Default)]
struct Layers {
    nl: usize,

    // Layer nodes.
    lk: Vec<u8>,
    li: Vec<usize>, // item for a child, edge for a port
    ll: Vec<usize>,
    lms: Vec<i64>,
    lcs: Vec<i64>,
    lpos: Vec<i64>,
    lord: Vec<usize>,
    leaf: Vec<bool>, // a child that is not a group
    // The cross range on a leaf where edges come in, and where they go out.
    ib: Vec<i64>,
    ibs: Vec<i64>,
    ob: Vec<i64>,
    obs: Vec<i64>,
    inset: Vec<i64>, // main distance from the box side to the edge end
    in_e: Groups,
    out_e: Groups,

    // Layer edges.
    es: Vec<usize>,
    et: Vec<usize>,
    ee: Vec<usize>,
    esp: Vec<i64>, // cross offset of the port on the source, -1 = not set
    etp: Vec<i64>,
    ew: Vec<i64>,
    efirst: Vec<bool>,
    etrack: Vec<i64>,
    slot: Vec<i64>,              // cross room for the source end
    chains: Vec<(usize, usize)>, // ranges of layer edges

    lay: Groups,      // nodes of each layer, in cross order
    by_layer: Groups, // edges that start in each layer

    // Work space.
    bsum: Vec<i64>,
    bcnt: Vec<i64>,
    best: Vec<usize>,
    keyed: Vec<(i64, usize)>,
    keyed2: Vec<(i64, usize)>,
    ks: Vec<i64>,
    kt: Vec<i64>,
    offs: Vec<i64>,
    bw: Vec<i64>, // blocks: weight, weight * target, node count
    bwd: Vec<i64>,
    bn: Vec<usize>,
    lmain: Vec<i64>,
    lsize: Vec<i64>,
}

impl Layers {
    fn reset(&mut self) {
        self.lk.clear();
        self.li.clear();
        self.ll.clear();
        self.lms.clear();
        self.lcs.clear();
        self.lpos.clear();
        self.lord.clear();
        self.leaf.clear();
        self.ib.clear();
        self.ibs.clear();
        self.ob.clear();
        self.obs.clear();
        self.inset.clear();
        self.es.clear();
        self.et.clear();
        self.ee.clear();
        self.esp.clear();
        self.etp.clear();
        self.ew.clear();
        self.efirst.clear();
        self.etrack.clear();
        self.slot.clear();
        self.chains.clear();
    }

    fn add_node(&mut self, kind: u8, item: usize, layer: usize, m: i64, cz: i64, leaf: bool) -> usize {
        self.lk.push(kind);
        self.li.push(item);
        self.ll.push(layer);
        self.lms.push(m);
        self.lcs.push(cz);
        self.lpos.push(0);
        self.lord.push(0);
        self.leaf.push(leaf);
        self.ib.push(0);
        self.ibs.push(cz);
        self.ob.push(0);
        self.obs.push(cz);
        self.inset.push(0);
        self.lk.len() - 1
    }

    /// Builds the lists of edges for each node and the lists for each layer.
    fn index(&mut self, nl: usize) {
        let nn = self.lk.len();
        self.nl = nl;
        self.in_e.build(nn, self.et.iter().copied());
        self.out_e.build(nn, self.es.iter().copied());
        self.lay.build(nl, self.ll.iter().copied());
        let (es, ll) = (&self.es, &self.ll);
        self.by_layer.build(nl, es.iter().map(|&s| ll[s]));
        self.bsum.clear();
        self.bsum.resize(nn, 0);
        self.bcnt.clear();
        self.bcnt.resize(nn, 1);
    }

    fn set_order(&mut self) {
        for l in 0..self.nl {
            for (k, &v) in self.lay.get(l).iter().enumerate() {
                self.lord[v] = k;
            }
        }
    }

    /// Sort key of an edge end: the order of the node, then the port.
    #[inline]
    fn key(&self, v: usize, port: i64) -> i64 {
        self.lord[v] as i64 * 8192 + (if port >= 0 { port } else { self.lcs[v] >> 1 }).min(8191)
    }

    #[inline]
    fn sep(&self, a: usize, b: usize) -> i64 {
        if self.lk[a] == CHILD && self.lk[b] == CHILD {
            NODE_GAP
        } else {
            THIN_GAP
        }
    }

    fn sweep(&mut self, l: usize, from_left: bool) {
        let (a, b) = (self.lay.start[l], self.lay.start[l + 1]);
        for i in a..b {
            let v = self.lay.list[i];
            let mut sum = 0;
            let mut cnt = 0;
            if from_left {
                for &le in self.in_e.get(v) {
                    sum += self.key(self.es[le], self.esp[le]);
                    cnt += 1;
                }
            } else {
                for &le in self.out_e.get(v) {
                    sum += self.key(self.et[le], self.etp[le]);
                    cnt += 1;
                }
            }
            if cnt == 0 {
                sum = self.key(v, -1);
                cnt = 1;
            }
            self.bsum[v] = sum;
            self.bcnt[v] = cnt;
        }
        // The sort compares the means sum / cnt without a division.
        let (bsum, bcnt) = (&self.bsum, &self.bcnt);
        self.lay.list[a..b].sort_by(|&x, &y| (bsum[x] * bcnt[y]).cmp(&(bsum[y] * bcnt[x])));
        for (k, &v) in self.lay.list[a..b].iter().enumerate() {
            self.lord[v] = k;
        }
    }

    fn crossings(&mut self) -> i64 {
        let mut total = 0;
        for l in 0..self.nl {
            let mut ks = std::mem::take(&mut self.ks);
            let mut kt = std::mem::take(&mut self.kt);
            ks.clear();
            kt.clear();
            for &le in self.by_layer.get(l) {
                ks.push(self.key(self.es[le], self.esp[le]));
                kt.push(self.key(self.et[le], self.etp[le]));
            }
            // Two edges cross when their ends have opposite orders. The test
            // has no branch, so the compiler can use vector instructions.
            for a in 0..ks.len() {
                let (sa, ta) = (ks[a], kt[a]);
                let rest = ks[a + 1..].iter().zip(&kt[a + 1..]);
                total += rest.filter(|&(&sb, &tb)| ((sa < sb) & (ta > tb)) | ((sa > sb) & (ta < tb))).count() as i64;
            }
            self.ks = ks;
            self.kt = kt;
        }
        total
    }

    /// Orders the nodes in each layer to get few crossings.
    fn order_layers(&mut self) {
        let nl = self.nl;
        self.set_order();
        let mut best = self.crossings();
        self.best.clone_from(&self.lay.list);
        let mut it = 0;
        while it < 4 && best > 0 {
            for l in 1..nl {
                self.sweep(l, true);
            }
            for l in (0..nl - 1).rev() {
                self.sweep(l, false);
            }
            let cnt = self.crossings();
            if cnt < best {
                best = cnt;
                self.best.clone_from(&self.lay.list);
            }
            it += 1;
        }
        self.lay.list.clone_from(&self.best);
        self.set_order();
    }

    /// Sets the ports on the leaves, in the order of the nodes at the other end.
    fn place_ports(&mut self) {
        let mut keyed = std::mem::take(&mut self.keyed);
        for v in 0..self.lk.len() {
            if !self.leaf[v] {
                continue;
            }
            keyed.clear();
            for &le in self.out_e.get(v) {
                keyed.push((self.key(self.et[le], self.etp[le]), le));
            }
            keyed.sort_by_key(|p| p.0);
            let total: i64 = keyed.iter().map(|p| self.slot[p.1]).sum();
            let mut at = self.ob[v] + idiv(self.obs[v] - total, 2);
            for &(_, le) in &keyed {
                let s = self.slot[le];
                self.esp[le] = at + (s >> 1);
                at += s;
            }
            keyed.clear();
            for &le in self.in_e.get(v) {
                keyed.push((self.key(self.es[le], self.esp[le]), le));
            }
            keyed.sort_by_key(|p| p.0);
            let mut at = self.ib[v] + idiv(self.ibs[v] - keyed.len() as i64 * SLOT, 2);
            for &(_, le) in &keyed {
                self.etp[le] = at + (SLOT >> 1);
                at += SLOT;
            }
        }
        self.keyed = keyed;
    }

    /// Puts the nodes of each layer side by side, from cross position 0.
    fn pack(&mut self) {
        for l in 0..self.nl {
            let (a, b) = (self.lay.start[l], self.lay.start[l + 1]);
            let mut at = 0;
            for i in a..b {
                let v = self.lay.list[i];
                self.lpos[v] = at;
                if i + 1 < b {
                    at += self.lcs[v] + self.sep(v, self.lay.list[i + 1]);
                }
            }
        }
    }

    /// Moves the nodes of one layer to the mean of their neighbours and
    /// keeps the order and the gaps. Mode: 0 = the layer before, 1 = the
    /// layer after, 2 = the two layers.
    fn place(&mut self, l: usize, mode: u8) {
        let (a, b) = (self.lay.start[l], self.lay.start[l + 1]);
        if a == b {
            return;
        }
        self.offs.clear();
        self.bw.clear();
        self.bwd.clear();
        self.bn.clear();
        let mut acc = 0;
        for i in a..b {
            let v = self.lay.list[i];
            let mut sw = 0;
            let mut swx = 0;
            if mode != 1 {
                for &le in self.in_e.get(v) {
                    let w = self.ew[le] * 4;
                    sw += w;
                    swx += w * (self.lpos[self.es[le]] + self.esp[le] - self.etp[le]);
                }
            }
            if mode != 0 {
                for &le in self.out_e.get(v) {
                    let w = self.ew[le] * 4;
                    sw += w;
                    swx += w * (self.lpos[self.et[le]] + self.etp[le] - self.esp[le]);
                }
            }
            let mut want = self.lpos[v];
            if sw == 0 {
                sw = 1;
            } else {
                want = rdiv(swx, sw);
            }
            self.offs.push(acc);
            self.bw.push(sw);
            self.bwd.push(sw * (want - acc));
            self.bn.push(1);
            if i + 1 < b {
                acc += self.lcs[v] + self.sep(v, self.lay.list[i + 1]);
            }
            // Join two blocks when the first one wants a position after the second.
            while self.bw.len() > 1 {
                let z = self.bw.len() - 1;
                if self.bwd[z - 1] * self.bw[z] <= self.bwd[z] * self.bw[z - 1] {
                    break;
                }
                let (w, wd, cnt) = (self.bw[z], self.bwd[z], self.bn[z]);
                self.bw[z - 1] += w;
                self.bwd[z - 1] += wd;
                self.bn[z - 1] += cnt;
                self.bw.pop();
                self.bwd.pop();
                self.bn.pop();
            }
        }
        let mut k = 0;
        for blk in 0..self.bw.len() {
            let p = rdiv(self.bwd[blk], self.bw[blk]);
            for _ in 0..self.bn[blk] {
                self.lpos[self.lay.list[a + k]] = p + self.offs[k];
                k += 1;
            }
        }
    }

    /// True when node `v` has room at cross position `np`.
    fn room(&self, v: usize, np: i64) -> bool {
        let lay = self.lay.get(self.ll[v]);
        let k = self.lord[v];
        if k > 0 && self.lpos[lay[k - 1]] + self.lcs[lay[k - 1]] + self.sep(lay[k - 1], v) - 4 > np {
            return false;
        }
        if k + 1 < lay.len() && np + self.lcs[v] + self.sep(v, lay[k + 1]) - 4 > self.lpos[lay[k + 1]] {
            return false;
        }
        true
    }

    /// Cross distance between the two ends of a layer edge.
    #[inline]
    fn bend(&self, le: usize) -> i64 {
        self.lpos[self.es[le]] + self.esp[le] - (self.lpos[self.et[le]] + self.etp[le])
    }

    /// Removes a small jog on one side of node `v`. A thin node moves if it
    /// has room. The ports on one side of a leaf move together, a short
    /// distance. A child moves a short distance if that does not bend a
    /// straight edge.
    fn snap(&mut self, v: usize, in_side: bool) {
        let (side, other) =
            if in_side { (self.in_e.get(v), self.out_e.get(v)) } else { (self.out_e.get(v), self.in_e.get(v)) };
        let mut delta: i64 = 0;
        for &le in side {
            let b = if in_side { self.bend(le) } else { -self.bend(le) };
            if b == 0 {
                return;
            }
            if delta == 0 || b.abs() < delta.abs() {
                delta = b;
            }
        }
        if delta == 0 {
            return;
        }
        let other_straight = other.iter().any(|&le| self.bend(le) == 0);
        if self.lk[v] != CHILD {
            if (in_side || !other_straight) && self.room(v, self.lpos[v] + delta) {
                self.lpos[v] += delta;
            }
            return;
        }
        if delta.abs() > 12 {
            return;
        }
        if self.leaf[v] {
            let (band, size) = if in_side { (self.ib[v], self.ibs[v]) } else { (self.ob[v], self.obs[v]) };
            let mut lo = i64::MAX;
            let mut hi = i64::MIN;
            for &le in side {
                let p = if in_side { self.etp[le] } else { self.esp[le] };
                lo = lo.min(p);
                hi = hi.max(p);
            }
            if lo + delta >= band + 6 && hi + delta <= band + size - 6 {
                for &le in side {
                    if in_side {
                        self.etp[le] += delta;
                    } else {
                        self.esp[le] += delta;
                    }
                }
                return;
            }
        }
        if !other_straight && self.room(v, self.lpos[v] + delta) {
            self.lpos[v] += delta;
        }
    }

    /// Sets the cross coordinates of all nodes.
    fn place_cross(&mut self) {
        let nl = self.nl;
        self.pack();
        for l in 1..nl {
            self.place(l, 0);
        }
        for l in (0..nl - 1).rev() {
            self.place(l, 1);
        }
        for _ in 0..2 {
            for l in 0..nl {
                self.place(l, 2);
            }
            for l in (0..nl).rev() {
                self.place(l, 2);
            }
        }
        for l in 1..nl {
            for i in self.lay.start[l]..self.lay.start[l + 1] {
                self.snap(self.lay.list[i], true);
            }
        }
        for l in (0..nl - 1).rev() {
            for i in self.lay.start[l]..self.lay.start[l + 1] {
                self.snap(self.lay.list[i], false);
            }
        }
    }
}

/// The values that all containers share.
struct Graph<'a> {
    d: &'a Diagram,
    down: bool,
    // Box sizes on the main and cross axes. Groups get a size from their layout.
    ms: Vec<i64>,
    cs: Vec<i64>,
    label_w: Vec<i64>,
    // An arch node is an icon tile with the text below it. Edges join the tile.
    tile: Vec<bool>,
    text_h: Vec<i64>,
    tsize: Vec<i64>,
    // Lists for each container. Container c is stored at c + 1, the root at 0.
    local: Vec<Vec<usize>>,
    ins: Vec<Vec<Seg>>,
    outs: Vec<Vec<Seg>>,
    rank: Vec<usize>,
    lend: Vec<usize>, // the end on the low-rank side
    cl: Vec<usize>,
    cr: Vec<usize>,
    ln_of: Vec<usize>, // layer node of each item
    // Results of the container layouts, in the frame of each container.
    pm: Vec<i64>, // position of a child in its parent
    pc: Vec<i64>,
    ports: Vec<HashMap<usize, i64>>, // key: edge * 2 + side, 0 = in, 1 = out
    piece_edge: Vec<usize>,
    piece_cont: Vec<Option<usize>>,
    piece_start: Vec<usize>, // start of each piece in `piece_pts`
    piece_pts: Vec<i64>,
    lab_cont: Vec<Option<usize>>,
    lab_m: Vec<i64>,
    lab_c: Vec<i64>,
    root_m: i64,
    root_c: i64,
}

/// Width of the label of a group, with its icon.
fn group_label_w(it: &crate::model::Item) -> i64 {
    measure(&it.label, 12, true) + if it.icon.is_empty() { 0 } else { 20 }
}

fn slot_of(c: Option<usize>) -> usize {
    c.map_or(0, |c| c + 1)
}

impl Graph<'_> {
    fn fixed_port(&self, w: &Layers, v: usize, e: usize, side: usize) -> i64 {
        match w.lk[v] {
            PASS => DUMMY >> 1,
            PORT_IN | PORT_OUT => w.lcs[v] >> 1,
            _ if w.leaf[v] => -1,
            _ => self.ports[w.li[v]][&(e * 2 + side)],
        }
    }

    /// Adds the layer edges of edge `e` from node `s` to node `t`, with one
    /// thin node in each layer between them.
    fn link(&self, w: &mut Layers, e: usize, s: usize, t: usize) {
        let start = w.es.len();
        let last = w.ll[t];
        let mut prev = s;
        for l in w.ll[s] + 1..=last {
            let next = if l == last { t } else { w.add_node(PASS, usize::MAX, l, 0, DUMMY, false) };
            let thin = u8::from(w.lk[prev] != CHILD) + u8::from(w.lk[next] != CHILD);
            let first = w.lk[prev] == CHILD && w.li[prev] == self.lend[e];
            let sp = self.fixed_port(w, prev, e, 1);
            let tp = self.fixed_port(w, next, e, 0);
            let slot = SLOT;
            w.es.push(prev);
            w.et.push(next);
            w.ee.push(e);
            w.esp.push(sp);
            w.etp.push(tp);
            w.ew.push(match thin {
                2 => 8,
                1 => 2,
                _ => 1,
            });
            w.efirst.push(first);
            w.etrack.push(-1);
            w.slot.push(slot);
            prev = next;
        }
        debug_assert!(w.es.len() > start, "the target layer is after the source layer");
        w.chains.push((start, w.es.len()));
    }

    fn layout_container(&mut self, c: Option<usize>, w: &mut Layers) {
        let d = self.d;
        let items = &d.items;
        let down = self.down;
        let kids: &[usize] = match c {
            None => &d.roots,
            Some(c) => &items[c].children,
        };
        for &k in kids {
            if items[k].group {
                self.layout_container(Some(k), w);
            }
        }

        let cslot = slot_of(c);
        let has_w = !self.ins[cslot].is_empty();
        let has_e = !self.outs[cslot].is_empty();
        let max_rank = kids.iter().map(|&k| self.rank[k]).max().unwrap_or(0);
        let off = usize::from(has_w);
        let nl = off + max_rank + 1 + usize::from(has_e);

        w.reset();
        for &k in kids {
            self.ln_of[k] = w.add_node(CHILD, k, self.rank[k] + off, self.ms[k], self.cs[k], !items[k].group);
        }
        for sg in &self.ins[cslot] {
            let p = w.add_node(PORT_IN, sg.e, 0, 0, 0, false);
            if let Some(child) = sg.child {
                self.link(w, sg.e, p, self.ln_of[child]);
            }
        }
        for &e in &self.local[cslot] {
            self.link(w, e, self.ln_of[self.cl[e]], self.ln_of[self.cr[e]]);
        }
        for sg in &self.outs[cslot] {
            let p = w.add_node(PORT_OUT, sg.e, nl - 1, 0, 0, false);
            if let Some(child) = sg.child {
                self.link(w, sg.e, self.ln_of[child], p);
            }
        }
        let nn = w.lk.len();
        w.index(nl);

        // Room for the edge ends on each leaf.
        for v in 0..nn {
            if !w.leaf[v] {
                continue;
            }
            let so: i64 = w.out_e.get(v).iter().map(|&le| w.slot[le]).sum();
            let si = w.in_e.get(v).len() as i64 * SLOT;
            let i = w.li[v];
            if !self.tile[i] {
                w.lcs[v] = w.lcs[v].max(so).max(si);
                w.ibs[v] = w.lcs[v];
                w.obs[v] = w.lcs[v];
            } else if down {
                // Edges come in at the top of the tile and go out below the
                // text, in the width of the tile.
                let tw = TILE.max(si).max(so);
                w.lcs[v] = w.lcs[v].max(tw);
                w.ib[v] = idiv(w.lcs[v] - tw, 2);
                w.ibs[v] = tw;
                w.ob[v] = w.ib[v];
                w.obs[v] = tw;
                self.tsize[i] = tw;
            } else {
                // Edges join the sides of the tile. The text is below the tile.
                let th = TILE.max(so).max(si);
                w.lcs[v] = th + self.text_h[i];
                w.ibs[v] = th;
                w.obs[v] = th;
                w.inset[v] = idiv(w.lms[v] - TILE, 2);
                self.tsize[i] = th;
            }
            self.cs[i] = w.lcs[v];
        }

        w.order_layers();
        w.place_ports();
        w.place_cross();

        let root = c.is_none();
        let pad_ms = if root {
            0
        } else if down {
            HEAD
        } else {
            PAD
        };
        let pad_me = if root { 0 } else { PAD };
        let pad_cs = if root {
            0
        } else if down {
            PAD
        } else {
            HEAD
        };
        let pad_ce = if root { 0 } else { PAD };
        let mut csize = pad_cs + pad_ce;
        if nn > 0 {
            let mut lo = w.lpos[0];
            let mut hi = w.lpos[0] + w.lcs[0];
            for v in 1..nn {
                lo = lo.min(w.lpos[v]);
                hi = hi.max(w.lpos[v] + w.lcs[v]);
            }
            let mut shift = pad_cs - lo;
            if let (true, true, Some(c)) = (down, has_w, c) {
                // An edge that comes in through the top border must not cross
                // the group label.
                let mut first = hi;
                for v in 0..nn {
                    if w.lk[v] == PORT_IN && w.lpos[v] < first {
                        first = w.lpos[v];
                    }
                }
                let need = group_label_w(&items[c]) + 24;
                if first + shift < need {
                    shift = need - first;
                }
            }
            for p in &mut w.lpos {
                *p += shift;
            }
            csize = hi + shift + pad_ce;
        }

        // Main coordinates. Each gap holds a label zone, the jog tracks, and a
        // straight run for the arrow.
        w.lmain.clear();
        w.lmain.resize(nl, 0);
        w.lsize.clear();
        w.lsize.resize(nl, 0);
        for v in 0..nn {
            let l = w.ll[v];
            if w.lms[v] > w.lsize[l] {
                w.lsize[l] = w.lms[v];
            }
        }
        let mut downs = std::mem::take(&mut w.keyed);
        let mut ups = std::mem::take(&mut w.keyed2);
        let mut m = if has_w { 0 } else { pad_ms };
        for l in 0..nl {
            w.lmain[l] = m;
            m += w.lsize[l];
            if l == nl - 1 {
                break;
            }
            downs.clear();
            ups.clear();
            let mut label_main = 0;
            for &le in w.by_layer.get(l) {
                let c1 = w.lpos[w.es[le]] + w.esp[le];
                let c2 = w.lpos[w.et[le]] + w.etp[le];
                if c2 > c1 {
                    downs.push((c1, le));
                } else if c2 < c1 {
                    ups.push((c1, le));
                }
                if w.efirst[le] && self.label_w[w.ee[le]] > 0 {
                    let lm = if down { 16 } else { self.label_w[w.ee[le]] };
                    if lm > label_main {
                        label_main = lm;
                    }
                }
            }
            // This track order keeps two jogs in the same direction from crossing.
            downs.sort_by_key(|p| std::cmp::Reverse(p.0));
            ups.sort_by_key(|p| p.0);
            let jogs = (downs.len() + ups.len()) as i64;
            let track_zone = if jogs > 0 { (jogs + 1) * TRACK } else { 0 };
            let label_zone = if label_main > 0 { label_main + 24 } else { 0 };
            let mut min_gap = MIN_GAP;
            if has_w && l == 0 {
                min_gap = pad_ms;
            }
            if has_e && l + 2 == nl {
                min_gap = pad_me;
            }
            let gap = min_gap.max(label_zone + track_zone + ARROW);
            let mut track = m + gap - track_zone - ARROW;
            for &(_, le) in downs.iter().chain(ups.iter()) {
                track += TRACK;
                w.etrack[le] = track;
            }
            for &le in w.by_layer.get(l) {
                let e = w.ee[le];
                if w.efirst[le] && self.label_w[e] > 0 {
                    self.lab_cont[e] = c;
                    self.lab_m[e] = m + idiv(gap - track_zone - ARROW, 2);
                    self.lab_c[e] = w.lpos[w.es[le]] + w.esp[le];
                }
            }
            m += gap;
        }
        w.keyed = downs;
        w.keyed2 = ups;
        let mut msize = if has_e { m } else { m + pad_me };

        // The box must be wide enough for the group label.
        if let Some(c) = c {
            let need = group_label_w(&items[c]) + 28;
            if down {
                if csize < need {
                    csize = need;
                }
            } else if msize < need {
                msize = need;
                if has_e {
                    w.lmain[nl - 1] = msize;
                }
            }
        }

        for v in 0..nn {
            let l = w.ll[v];
            match (w.lk[v], c) {
                (CHILD, _) => {
                    self.pm[w.li[v]] = w.lmain[l] + idiv(w.lsize[l] - w.lms[v], 2);
                    self.pc[w.li[v]] = w.lpos[v];
                }
                (PORT_IN, Some(c)) => {
                    self.ports[c].insert(w.li[v] * 2, w.lpos[v] + (w.lcs[v] >> 1));
                }
                (PORT_OUT, Some(c)) => {
                    self.ports[c].insert(w.li[v] * 2 + 1, w.lpos[v] + (w.lcs[v] >> 1));
                }
                _ => {}
            }
        }

        // The main position where an edge leaves a node, and where it enters.
        let pm = &self.pm;
        let exit_m = |v: usize| match w.lk[v] {
            CHILD => pm[w.li[v]] + w.lms[v] - w.inset[v],
            PASS => w.lmain[w.ll[v]] + w.lsize[w.ll[v]],
            _ => 0,
        };
        let entry_m = |v: usize| match w.lk[v] {
            CHILD => pm[w.li[v]] + w.inset[v],
            PASS => w.lmain[w.ll[v]],
            _ => msize,
        };
        let pts = &mut self.piece_pts;
        for &(first, end) in &w.chains {
            self.piece_start.push(pts.len());
            pts.push(exit_m(w.es[first]));
            pts.push(w.lpos[w.es[first]] + w.esp[first]);
            for le in first..end {
                let c1 = w.lpos[w.es[le]] + w.esp[le];
                let c2 = w.lpos[w.et[le]] + w.etp[le];
                if c1 != c2 {
                    pts.extend_from_slice(&[w.etrack[le], c1, w.etrack[le], c2]);
                }
                pts.extend_from_slice(&[entry_m(w.et[le]), c2]);
                if w.lk[w.et[le]] == PASS {
                    pts.extend_from_slice(&[exit_m(w.et[le]), c2]);
                }
            }
            self.piece_edge.push(w.ee[first]);
            self.piece_cont.push(c);
        }

        match c {
            None => {
                self.root_m = msize;
                self.root_c = csize;
            }
            Some(c) => {
                self.ms[c] = msize;
                self.cs[c] = csize;
            }
        }
    }
}

/// Sets the absolute positions, parents before children.
fn walk(d: &Diagram, list: &[usize], bm: i64, bc: i64, g: &Graph, am: &mut [i64], ac: &mut [i64]) {
    for &k in list {
        am[k] = bm + g.pm[k];
        ac[k] = bc + g.pc[k];
        if d.items[k].group {
            walk(d, &d.items[k].children, am[k], ac[k], g, am, ac);
        }
    }
}

pub fn layout_graph(d: &Diagram) -> GraphLayout {
    let items = &d.items;
    let n = items.len();
    let ne = d.edges.len();
    let down = d.down;
    let tiles = d.dtype.has_tiles();

    let mut ms = vec![0i64; n];
    let mut cs = vec![0i64; n];
    let mut tile = vec![false; n];
    let mut text_h = vec![0i64; n];
    for (i, it) in items.iter().enumerate() {
        if it.group {
            continue;
        }
        let tw = measure(&it.label, 13, true).max(measure_mono(&it.sub, 95));
        let (mut w, h);
        if svg_shapes::draws(d.dtype) {
            (w, h) = svg_shapes::size(d, i);
        } else if !tiles {
            // A card has a status mark at the left.
            w = (tw + 28 + 22).max(96);
            h = if it.sub.is_empty() { 38 } else { 52 };
        } else {
            tile[i] = true;
            text_h[i] = if it.sub.is_empty() { 24 } else { 40 };
            w = (tw + 12).max(72);
            h = TILE + text_h[i];
        }
        w += w & 1;
        ms[i] = if down { h } else { w };
        cs[i] = if down { w } else { h };
    }
    let label_w: Vec<i64> =
        d.edges.iter().map(|e| if e.label.is_empty() { 0 } else { measure_mono(&e.label, 105) }).collect();

    // The lowest container that holds both ends of an edge, and the two children
    // of that container that hold the ends.
    let mut lca: Vec<Option<usize>> = vec![None; ne];
    let mut ca = vec![0usize; ne];
    let mut cb = vec![0usize; ne];
    let mut local: Vec<Vec<usize>> = vec![Vec::new(); n + 1];
    let mut ins: Vec<Vec<Seg>> = vec![Vec::new(); n + 1];
    let mut outs: Vec<Vec<Seg>> = vec![Vec::new(); n + 1];
    let mut pa: Vec<usize> = Vec::new();
    let mut pb: Vec<usize> = Vec::new();
    for (e, edge) in d.edges.iter().enumerate() {
        if edge.from == edge.to {
            // A self-transition does not take part in the layering.
            ca[e] = edge.from;
            cb[e] = edge.from;
            continue;
        }
        pa.clear();
        pb.clear();
        let mut p = Some(edge.from);
        while let Some(q) = p {
            pa.push(q);
            p = items[q].parent;
        }
        let mut p = Some(edge.to);
        while let Some(q) = p {
            pb.push(q);
            p = items[q].parent;
        }
        let mut i = pa.len();
        let mut j = pb.len();
        while i > 0 && j > 0 && pa[i - 1] == pb[j - 1] {
            lca[e] = Some(pa[i - 1]);
            i -= 1;
            j -= 1;
        }
        // The parser rejects an edge between a group and its own member, so
        // each end has its own child in the container.
        assert!(i > 0 && j > 0, "line {}: an edge joins a group and its own member", edge.line);
        ca[e] = pa[i - 1];
        cb[e] = pb[j - 1];
        local[slot_of(lca[e])].push(e);
    }

    // Break cycles: a depth-first search reverses each edge that closes a
    // cycle. The stack holds a node and the position in its edge list.
    let mut rev = vec![false; ne];
    let mut state = vec![0u8; n];
    let mut adj = Groups::default();
    adj.build(n, ca.iter().copied());
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        if state[i] != 0 {
            continue;
        }
        state[i] = 1;
        stack.push((i, adj.start[i]));
        while let Some(top) = stack.last_mut() {
            let (u, at) = *top;
            if at == adj.start[u + 1] {
                state[u] = 2;
                stack.pop();
                continue;
            }
            top.1 += 1;
            let e = adj.list[at];
            let v = cb[e];
            if state[v] == 1 {
                rev[e] = true;
            } else if state[v] == 0 {
                state[v] = 1;
                stack.push((v, adj.start[v]));
            }
        }
    }

    let mut lend = vec![0usize; ne]; // the end on the low-rank side
    let mut rend = vec![0usize; ne];
    let mut cl = vec![0usize; ne];
    let mut cr = vec![0usize; ne];
    for (e, edge) in d.edges.iter().enumerate() {
        lend[e] = if rev[e] { edge.to } else { edge.from };
        rend[e] = if rev[e] { edge.from } else { edge.to };
        cl[e] = if rev[e] { cb[e] } else { ca[e] };
        cr[e] = if rev[e] { ca[e] } else { cb[e] };
    }

    // Give each child a rank inside its container.
    let mut rank = vec![0usize; n];
    let mut indeg = vec![0i64; n];
    let mut queue: Vec<usize> = Vec::new();
    for (cslot, loc) in local.iter().enumerate() {
        let kids: &[usize] = match cslot.checked_sub(1) {
            None => &d.roots,
            Some(c) if items[c].group => &items[c].children,
            Some(_) => continue,
        };
        for &k in kids {
            indeg[k] = 0;
        }
        for &e in loc {
            indeg[cr[e]] += 1;
        }
        queue.clear();
        queue.extend(kids.iter().copied().filter(|&k| indeg[k] == 0));
        let mut q = 0;
        while q < queue.len() {
            let u = queue[q];
            q += 1;
            for &e in loc {
                if cl[e] != u {
                    continue;
                }
                let v = cr[e];
                if rank[v] < rank[u] + 1 {
                    rank[v] = rank[u] + 1;
                }
                indeg[v] -= 1;
                if indeg[v] == 0 {
                    queue.push(v);
                }
            }
        }
        // Move a source next to its first target. This makes the edges shorter.
        for &k in kids {
            let mut has_in = false;
            let mut min_out: Option<usize> = None;
            for &e in loc {
                if cr[e] == k {
                    has_in = true;
                }
                if cl[e] == k && min_out.is_none_or(|m| rank[cr[e]] < m) {
                    min_out = Some(rank[cr[e]]);
                }
            }
            if let (false, Some(m)) = (has_in, min_out) {
                if m > 0 {
                    rank[k] = m - 1;
                }
            }
        }
    }

    // Border ports. An edge end inside a group leaves each group between the
    // end and the lowest common container.
    for e in 0..ne {
        if d.edges[e].from == d.edges[e].to {
            continue;
        }
        for (end, list) in [(lend[e], &mut outs), (rend[e], &mut ins)] {
            let mut cur = end;
            if items[cur].group {
                list[cur + 1].push(Seg { e, child: None });
            }
            let mut p = items[cur].parent;
            while p != lca[e] {
                let Some(g) = p else { unreachable!("the container holds the two ends") };
                list[g + 1].push(Seg { e, child: Some(cur) });
                cur = g;
                p = items[g].parent;
            }
        }
    }

    let mut g = Graph {
        d,
        down,
        ms,
        cs,
        label_w,
        tile,
        text_h,
        tsize: vec![0; n],
        local,
        ins,
        outs,
        rank,
        lend,
        cl,
        cr,
        ln_of: vec![0; n],
        pm: vec![0; n],
        pc: vec![0; n],
        ports: vec![HashMap::new(); n],
        piece_edge: Vec::new(),
        piece_cont: Vec::new(),
        piece_start: Vec::new(),
        piece_pts: Vec::new(),
        lab_cont: vec![None; ne],
        lab_m: vec![0; ne],
        lab_c: vec![0; ne],
        root_m: 0,
        root_c: 0,
    };
    g.layout_container(None, &mut Layers::default());
    g.piece_start.push(g.piece_pts.len());

    let mut am = vec![0i64; n];
    let mut ac = vec![0i64; n];
    walk(d, &d.roots, 0, 0, &g, &mut am, &mut ac);

    let mut out = GraphLayout {
        w: if down { g.root_c } else { g.root_m },
        h: if down { g.root_m } else { g.root_c },
        x: if down { ac.clone() } else { am.clone() },
        y: if down { am.clone() } else { ac.clone() },
        bw: if down { g.cs.clone() } else { g.ms.clone() },
        bh: if down { g.ms.clone() } else { g.cs.clone() },
        tile: g.tsize.clone(),
        edges: Vec::with_capacity(ne),
    };

    // Join the pieces of each edge. A route always moves forward on the main
    // axis, so the start position gives the order of the pieces.
    for p in 0..g.piece_edge.len() {
        let (bm, bc) = g.piece_cont[p].map_or((0, 0), |c| (am[c], ac[c]));
        for [m, c] in g.piece_pts[g.piece_start[p]..g.piece_start[p + 1]].as_chunks_mut::<2>().0 {
            *m += bm;
            *c += bc;
        }
    }
    let mut by_edge = Groups::default();
    by_edge.build(ne, g.piece_edge.iter().copied());
    let mut order: Vec<usize> = Vec::new();
    let mut raw: Vec<i64> = Vec::new();
    for (e, edge) in d.edges.iter().enumerate() {
        if edge.from == edge.to {
            continue;
        }
        order.clear();
        order.extend_from_slice(by_edge.get(e));
        order.sort_by_key(|&p| g.piece_pts[g.piece_start[p]]);
        raw.clear();
        for &p in &order {
            raw.extend_from_slice(&g.piece_pts[g.piece_start[p]..g.piece_start[p + 1]]);
        }
        // Drop a point that repeats, or that lies on a straight run.
        let mut pts: Vec<i64> = Vec::with_capacity(raw.len());
        for &[pm0, pc0] in raw.as_chunks::<2>().0 {
            let z = pts.len();
            if z >= 2 && pts[z - 2] == pm0 && pts[z - 1] == pc0 {
                continue;
            }
            if z >= 4 {
                let same_c = pts[z - 3] == pts[z - 1] && pts[z - 1] == pc0;
                let same_m = pts[z - 4] == pts[z - 2] && pts[z - 2] == pm0;
                if same_c || same_m {
                    pts[z - 2] = pm0;
                    pts[z - 1] = pc0;
                    continue;
                }
            }
            pts.push(pm0);
            pts.push(pc0);
        }
        if down {
            for pt in pts.as_chunks_mut::<2>().0 {
                pt.swap(0, 1);
            }
        }
        let mut ge = GEdge {
            edge: e,
            pts,
            label_w: g.label_w[e],
            label_x: 0,
            label_y: 0,
            arrow_start: rev[e] || edge.bidir,
            arrow_end: !rev[e] || edge.bidir,
        };
        if ge.label_w > 0 {
            let (bm, bc) = g.lab_cont[e].map_or((0, 0), |c| (am[c], ac[c]));
            let lm = g.lab_m[e] + bm;
            let lc = g.lab_c[e] + bc;
            ge.label_x = if down { lc } else { lm };
            ge.label_y = if down { lm } else { lc };
        }
        out.edges.push(ge);
    }
    out
}
