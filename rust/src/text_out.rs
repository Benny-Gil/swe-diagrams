//! Terminal text output. It draws a diagram with Unicode box drawing.
//!
//! The layout functions give pixel positions. A monotone map moves each pixel
//! coordinate to a column or a row, and constraints give each box the room for
//! its text. A drawing that is wider than the limit becomes an outline.

use crate::cycles::find_cycles;
use crate::layout_graph::layout_graph;
use crate::layout_seq::layout_seq;
use crate::layout_timeline::layout_timeline;
use crate::model::{idiv, rdiv, Diagram, DiagramType, Edge, Member, CLASS_ARROWS, STATUS_NAMES};
use crate::records::{field_type, key_mark, marks, sections, stereotype};
use crate::svg::change_counts;

const SYMBOLS: [&str; 7] = ["", "✓", "◐", "○", "+", "~", "−"];
/// The ANSI colours of the statuses. They match the SVG: green, purple, grey,
/// green, amber, red.
const STATUS_COLORS: [u8; 7] = [0, 32, 35, 90, 32, 33, 31];
const BLUE: u8 = 34;
const RED: u8 = 31;
const GREY: u8 = 90;

/// Pixels for one column and for one row, before the constraints add room.
const SX: i64 = 12;
const SY: i64 = 24;
/// The longest text in a box. A longer text gets an ellipsis.
const MAX_TEXT: usize = 28;

const U: u8 = 1;
const D: u8 = 2;
const L: u8 = 4;
const R: u8 = 8;

/// Owners of the line cells. An edge owns its cells with its index.
const NONE: i32 = -1;
const MANY: i32 = -2;
const BORDER: i32 = -3;

/// A text with one colour.
type Run = (String, u8);

/// Draws the diagram for a terminal. `color` adds ANSI colour codes. When the
/// drawing needs more than `width` columns, the result is an outline.
pub fn to_text(d: &Diagram, color: bool, width: usize) -> String {
    let (grid, notes) = match d.dtype {
        DiagramType::Seq => seq_grid(d),
        DiagramType::Timeline => timeline_grid(d),
        _ => graph_grid(d),
    };
    let mut body = grid.render(color);
    let need = body.iter().map(|l| visible_len(l)).max().unwrap_or(0);
    if need > width {
        let why = format!("outline: the drawing needs {need} columns, and the limit is {width}.");
        body = vec![paint(&clip(&[(why, GREY)], width), color), String::new()];
        body.extend(outline(d).iter().map(|line| paint(&clip(line, width), color)));
    } else {
        body.extend(notes.iter().map(|line| paint(line, color)));
    }

    let mut out = String::new();
    out.push_str(&paint(&[(kicker(d), GREY)], color));
    out.push('\n');
    if !d.title.is_empty() {
        out.push_str(&if color { format!("\x1b[1m{}\x1b[0m", d.title) } else { d.title.clone() });
        out.push('\n');
    }
    out.push('\n');
    for line in &body {
        out.push_str(line);
        out.push('\n');
    }
    let legend = pack(legend(d), width);
    if !legend.is_empty() {
        out.push('\n');
    }
    for line in &legend {
        out.push_str(&paint(line, color));
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------- text

/// The columns of a character: 2 for the wide East Asian ranges and most
/// emoji, 0 for combining marks, 1 for all others.
fn char_w(ch: char) -> i64 {
    let c = ch as u32;
    const ZERO: [(u32, u32); 4] = [(0x300, 0x36F), (0x200B, 0x200F), (0xFE00, 0xFE0F), (0x20D0, 0x20FF)];
    const WIDE: [(u32, u32); 14] = [
        (0x1100, 0x115F),
        (0x2E80, 0x303E),
        (0x3041, 0x33FF),
        (0x3400, 0x4DBF),
        (0x4E00, 0x9FFF),
        (0xA000, 0xA4CF),
        (0xAC00, 0xD7A3),
        (0xF900, 0xFAFF),
        (0xFE30, 0xFE4F),
        (0xFF00, 0xFF60),
        (0xFFE0, 0xFFE6),
        (0x1F300, 0x1F64F),
        (0x1F900, 0x1F9FF),
        (0x20000, 0x3FFFD),
    ];
    if ZERO.iter().any(|&(a, b)| (a..=b).contains(&c)) {
        0
    } else if WIDE.iter().any(|&(a, b)| (a..=b).contains(&c)) {
        2
    } else {
        1
    }
}

fn len(s: &str) -> i64 {
    s.chars().map(char_w).sum()
}

/// The number of columns of a line, without the ANSI codes.
fn visible_len(s: &str) -> usize {
    let mut n = 0;
    let mut esc = false;
    for ch in s.chars() {
        if esc {
            esc = ch != 'm';
        } else if ch == '\x1b' {
            esc = true;
        } else {
            n += char_w(ch) as usize;
        }
    }
    n
}

fn fit(s: &str, max: usize) -> String {
    let max = max as i64;
    if len(s) <= max {
        return s.to_string();
    }
    let mut t = String::new();
    let mut w = 0;
    for ch in s.chars() {
        w += char_w(ch);
        if w > max - 1 {
            break;
        }
        t.push(ch);
    }
    t.push('…');
    t
}

/// A label and its status symbol.
fn labelled(text: &str, status: usize) -> Vec<Run> {
    let mut runs = whole(text, status);
    runs[0].0 = fit(text, MAX_TEXT);
    runs
}

/// A label and its status symbol, without a limit on the length.
fn whole(text: &str, status: usize) -> Vec<Run> {
    let mut runs = vec![(text.to_string(), 0)];
    if status > 0 {
        let sym = if text.is_empty() { SYMBOLS[status].to_string() } else { format!(" {}", SYMBOLS[status]) };
        runs.push((sym, STATUS_COLORS[status]));
    }
    runs
}

fn runs_len(runs: &[Run]) -> i64 {
    runs.iter().map(|r| len(&r.0)).sum()
}

fn plain(runs: &[Run]) -> String {
    runs.iter().map(|r| r.0.as_str()).collect()
}

fn paint(runs: &[Run], color: bool) -> String {
    let mut out = String::new();
    for (text, c) in runs {
        if color && *c > 0 {
            out.push_str(&format!("\x1b[{c}m{text}\x1b[0m"));
        } else {
            out.push_str(text);
        }
    }
    out
}

/// Cuts a line to `width` columns. The last column gets an ellipsis.
fn clip(runs: &[Run], width: usize) -> Vec<Run> {
    let mut out = Vec::new();
    let mut left = width as i64;
    for (s, c) in runs {
        if left <= 0 {
            break;
        }
        if len(s) > left || (len(s) == left && out.len() + 1 < runs.len()) {
            out.push((fit(s, left as usize), *c));
            break;
        }
        left -= len(s);
        out.push((s.clone(), *c));
    }
    out
}

fn kicker(d: &Diagram) -> String {
    let nodes = || d.items.iter().filter(|it| !it.group);
    match d.dtype {
        DiagramType::Plan => {
            format!("plan / {} of {} done", nodes().filter(|it| it.status == 1).count(), nodes().count())
        }
        DiagramType::Timeline if nodes().any(|it| matches!(it.status, 1..=3)) => {
            format!("timeline / {} of {} done", nodes().filter(|it| it.status == 1).count(), nodes().count())
        }
        DiagramType::Change => {
            let [add, m, del] = change_counts(d);
            format!("change / +{add} ~{m} -{del}")
        }
        DiagramType::Seq => "sequence".to_string(),
        DiagramType::Deps => "dependencies".to_string(),
        DiagramType::State => "state machine".to_string(),
        DiagramType::Dataflow => "data flow".to_string(),
        DiagramType::Erd => "entities".to_string(),
        DiagramType::Class => "classes".to_string(),
        DiagramType::Arch => "architecture".to_string(),
        t => t.name().to_string(),
    }
}

const CLASS_NAMES: [&str; 6] = ["extends", "implements", "made of", "has", "uses", "depends on"];
const CLASS_GLYPHS: [&str; 6] = ["▷", "┄▷", "◆", "◇", "▶", "┄▶"];

/// The legend entries. Each entry is one unit for the line break.
fn legend(d: &Diagram) -> Vec<Vec<Run>> {
    let mut used = [false; 7];
    for it in &d.items {
        used[it.status] = true;
        for m in &it.members {
            used[m.status] = true;
        }
    }
    for e in &d.edges {
        used[e.status] = true;
    }
    let mut items: Vec<Vec<Run>> = Vec::new();
    for s in 1..7 {
        if used[s] {
            items.push(vec![(SYMBOLS[s].to_string(), STATUS_COLORS[s]), (format!(" {}", STATUS_NAMES[s]), 0)]);
        }
    }
    if d.dtype == DiagramType::Erd && !d.edges.is_empty() {
        for t in ["1 one", "? zero or one", "* many", "+ one or more"] {
            items.push(vec![(t.to_string(), 0)]);
        }
    }
    if d.dtype == DiagramType::Class {
        for (k, arrow) in CLASS_ARROWS.iter().enumerate() {
            if d.edges.iter().any(|e| e.arrow == *arrow) {
                items.push(vec![(format!("{} {}", CLASS_GLYPHS[k], CLASS_NAMES[k]), 0)]);
            }
        }
    }
    items
}

/// Puts the entries on lines of `width` columns or fewer.
fn pack(items: Vec<Vec<Run>>, width: usize) -> Vec<Vec<Run>> {
    let mut lines: Vec<Vec<Run>> = Vec::new();
    let mut cur: Vec<Run> = Vec::new();
    for item in items {
        let w = runs_len(&item);
        if !cur.is_empty() && runs_len(&cur) + 3 + w > width as i64 {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(("   ".to_string(), 0));
        }
        cur.extend(item);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

// ---------------------------------------------------------------- grid

#[derive(Clone, Copy)]
struct Cell {
    /// The text character, or `\0` when the cell shows its line bits.
    ch: char,
    bits: u8,
    solid: bool,
    dash: bool,
    round: bool,
    color: u8,
    owner: i32,
    /// True inside a node box. A line or a label must not go there.
    block: bool,
}

/// The second cell of a wide character. The output skips it.
const TAIL: char = '\u{E000}';

const EMPTY: Cell =
    Cell { ch: '\0', bits: 0, solid: false, dash: false, round: false, color: 0, owner: NONE, block: false };

struct Grid {
    w: i64,
    h: i64,
    cells: Vec<Cell>,
}

impl Grid {
    fn new(w: i64, h: i64) -> Grid {
        let w = w.max(1);
        let h = h.max(1);
        Grid { w, h, cells: vec![EMPTY; (w * h) as usize] }
    }

    fn get(&self, x: i64, y: i64) -> Option<&Cell> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| &self.cells[(y * self.w + x) as usize])
    }

    fn at(&mut self, x: i64, y: i64) -> Option<&mut Cell> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| &mut self.cells[(y * self.w + x) as usize])
    }

    fn link(&mut self, x: i64, y: i64, bits: u8, dash: bool, color: u8, owner: i32) {
        if let Some(c) = self.at(x, y) {
            c.bits |= bits;
            if dash {
                c.dash = true;
            } else {
                c.solid = true;
            }
            if color > 0 || c.owner == NONE {
                c.color = color;
            }
            c.owner = match (c.owner, owner) {
                (BORDER, _) | (_, BORDER) => BORDER,
                (NONE, o) => o,
                (o, p) if o == p => o,
                _ => MANY,
            };
        }
    }

    /// An orthogonal line from `a` to `b`. A slanted pair becomes an L shape.
    fn seg(&mut self, a: (i64, i64), b: (i64, i64), dash: bool, color: u8, owner: i32) {
        let ((x0, y0), (x1, y1)) = (a, b);
        if y0 == y1 {
            let (lo, hi) = (x0.min(x1), x0.max(x1));
            for x in lo..=hi {
                let bits = if x > lo { L } else { 0 } | if x < hi { R } else { 0 };
                if bits != 0 {
                    self.link(x, y0, bits, dash, color, owner);
                }
            }
        } else if x0 == x1 {
            let (lo, hi) = (y0.min(y1), y0.max(y1));
            for y in lo..=hi {
                let bits = if y > lo { U } else { 0 } | if y < hi { D } else { 0 };
                if bits != 0 {
                    self.link(x0, y, bits, dash, color, owner);
                }
            }
        } else {
            self.seg(a, (x1, y0), dash, color, owner);
            self.seg((x1, y0), b, dash, color, owner);
        }
    }

    fn rect(&mut self, b: [i64; 4], dash: bool, round: bool, color: u8) {
        let [x0, y0, x1, y1] = b;
        self.seg((x0, y0), (x1, y0), dash, color, BORDER);
        self.seg((x0, y1), (x1, y1), dash, color, BORDER);
        self.seg((x0, y0), (x0, y1), dash, color, BORDER);
        self.seg((x1, y0), (x1, y1), dash, color, BORDER);
        if round {
            for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                if let Some(c) = self.at(x, y) {
                    c.round = true;
                }
            }
        }
    }

    fn block(&mut self, b: [i64; 4]) {
        for y in b[1] + 1..b[3] {
            for x in b[0] + 1..b[2] {
                if let Some(c) = self.at(x, y) {
                    *c = Cell { block: true, ..EMPTY };
                }
            }
        }
    }

    fn put(&mut self, x: i64, y: i64, ch: char, color: u8) {
        if let Some(c) = self.at(x, y) {
            c.ch = ch;
            c.color = color;
        }
    }

    fn text(&mut self, x: i64, y: i64, runs: &[Run]) {
        let mut x = x;
        for (s, color) in runs {
            for ch in s.chars() {
                match char_w(ch) {
                    0 => continue,
                    2 => {
                        self.put(x, y, ch, *color);
                        self.put(x + 1, y, TAIL, *color);
                        x += 2;
                    }
                    _ => {
                        self.put(x, y, ch, *color);
                        x += 1;
                    }
                }
            }
        }
    }

    /// True when a label can go on the cells: no text, no box, and no line
    /// of another owner.
    fn free(&self, x: i64, y: i64, n: i64, owner: i32) -> bool {
        (x..x + n)
            .all(|x| self.get(x, y).is_some_and(|c| c.ch == '\0' && !c.block && (c.bits == 0 || c.owner == owner)))
    }

    fn render(&self, color: bool) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for y in 0..self.h {
            let row = &self.cells[(y * self.w) as usize..((y + 1) * self.w) as usize];
            let chars: Vec<(char, u8)> = row.iter().map(|c| (glyph(c), c.color)).collect();
            let end = chars.iter().rposition(|&(ch, _)| ch != ' ').map_or(0, |p| p + 1);
            let mut line = String::new();
            let mut cur = 0u8;
            for &(ch, c) in &chars[..end] {
                if ch == TAIL {
                    continue;
                }
                let c = if ch == ' ' { cur } else { c };
                if color && c != cur {
                    line.push_str(if c == 0 { "\x1b[0m".to_string() } else { format!("\x1b[{c}m") }.as_str());
                    cur = c;
                }
                line.push(ch);
            }
            if color && cur != 0 {
                line.push_str("\x1b[0m");
            }
            lines.push(line);
        }
        // Drop the empty rows at the end.
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        lines
    }
}

fn glyph(c: &Cell) -> char {
    if c.ch != '\0' {
        return c.ch;
    }
    let dashed = c.dash && !c.solid;
    let b = c.bits;
    if b == 0 {
        ' '
    } else if b & (U | D) == 0 {
        if dashed {
            '┄'
        } else {
            '─'
        }
    } else if b & (L | R) == 0 {
        if dashed {
            '┆'
        } else {
            '│'
        }
    } else if b == D | R {
        if c.round {
            '╭'
        } else {
            '┌'
        }
    } else if b == D | L {
        if c.round {
            '╮'
        } else {
            '┐'
        }
    } else if b == U | R {
        if c.round {
            '╰'
        } else {
            '└'
        }
    } else if b == U | L {
        if c.round {
            '╯'
        } else {
            '┘'
        }
    } else if b == U | D | R {
        '├'
    } else if b == U | D | L {
        '┤'
    } else if b == L | R | D {
        '┬'
    } else if b == L | R | U {
        '┴'
    } else {
        '┼'
    }
}

/// The arrowhead that points in the direction `(dx, dy)`.
fn arrow(dx: i64, dy: i64) -> char {
    match (dx.signum(), dy.signum()) {
        (1, _) => '▶',
        (-1, _) => '◀',
        (_, 1) => '▼',
        _ => '▲',
    }
}

// ---------------------------------------------------------------- axis

/// A monotone map from pixel coordinates to cells. Two different keys get
/// two different cells, so the order of all parts stays the same.
struct Axis {
    keys: Vec<i64>,
    pos: Vec<i64>,
}

impl Axis {
    /// `cons` holds `(a, b, n)`: the cell of key `b` is at least `n` cells
    /// after the cell of key `a`. Each key of `cons` must be in `keys`.
    fn new(mut keys: Vec<i64>, scale: i64, mut cons: Vec<(i64, i64, i64)>) -> Axis {
        keys.sort_unstable();
        keys.dedup();
        let mut pos = Vec::with_capacity(keys.len());
        for (k, &v) in keys.iter().enumerate() {
            pos.push(if k == 0 { 0 } else { pos[k - 1] + rdiv(v - keys[k - 1], scale).max(1) });
        }
        let index = |v: i64| keys.binary_search(&v).expect("a constraint key is a key");
        // A push moves a key and all keys after it, so it never makes a
        // distance smaller. One pass is enough.
        cons.sort_unstable();
        for (a, b, n) in cons {
            let (ia, ib) = (index(a), index(b));
            if ia >= ib {
                continue;
            }
            let short = n - (pos[ib] - pos[ia]);
            if short > 0 {
                for p in &mut pos[ib..] {
                    *p += short;
                }
            }
        }
        Axis { keys, pos }
    }

    fn at(&self, v: i64) -> i64 {
        match self.keys.binary_search(&v) {
            Ok(i) => self.pos[i],
            Err(0) => self.pos.first().copied().unwrap_or(0),
            Err(i) if i == self.keys.len() => self.pos[i - 1],
            Err(i) => {
                let (k0, k1, p0, p1) = (self.keys[i - 1], self.keys[i], self.pos[i - 1], self.pos[i]);
                p0 + idiv((v - k0) * (p1 - p0), k1 - k0)
            }
        }
    }

    fn last(&self) -> i64 {
        self.pos.last().copied().unwrap_or(0)
    }
}

// ---------------------------------------------------------------- graph

enum Row {
    Text(Vec<Run>, bool),
    Sep,
}

fn grey(s: &str) -> Vec<Run> {
    vec![(fit(s, MAX_TEXT), GREY)]
}

/// The text of a member row of a record.
fn member_row(dt: DiagramType, m: &Member, mark_w: usize, name_w: usize) -> Vec<Run> {
    let text = if dt == DiagramType::Erd {
        let name = m.words.first().map_or("", String::as_str);
        let mark = key_mark(m);
        let lead = if mark_w > 0 { format!("{mark:<mark_w$} ") } else { String::new() };
        format!("{lead}{name:<name_w$}  {}", field_type(m))
    } else {
        m.text.clone()
    };
    labelled(&text, m.status)
}

fn node_rows(d: &Diagram, i: usize) -> Vec<Row> {
    let it = &d.items[i];
    let kind = d.dtype.kinds()[it.kind];
    let mut rows = Vec::new();
    if d.dtype.has_members() {
        if let Some(s) = stereotype(d.dtype, it) {
            rows.push(Row::Text(grey(&s), true));
        }
        rows.push(Row::Text(labelled(&it.label, it.status), true));
        let mark_w = it.members.iter().map(|m| len(&key_mark(m)) as usize).max().unwrap_or(0);
        let name_w = it.members.iter().map(|m| m.words.first().map_or(0, |w| len(w) as usize)).max().unwrap_or(0);
        for sec in sections(d.dtype, it) {
            rows.push(Row::Sep);
            for m in sec {
                rows.push(Row::Text(member_row(d.dtype, m, mark_w, name_w), false));
            }
        }
    } else {
        let text = match (d.dtype, kind) {
            (DiagramType::State, "initial") => "●".to_string(),
            (DiagramType::State, "final") => "◉".to_string(),
            (DiagramType::State, "choice") => "◇".to_string(),
            (DiagramType::Flow, "decision") => format!("◇ {}", it.label),
            (DiagramType::Flow, "io") => format!("▱ {}", it.label),
            _ => it.label.clone(),
        };
        rows.push(Row::Text(labelled(&text, it.status), true));
        if !it.sub.is_empty() {
            rows.push(Row::Text(grey(&it.sub), true));
        }
    }
    // A self-transition has no route in a graph, so it is a row of the box.
    let graph = !matches!(d.dtype, DiagramType::Seq | DiagramType::Timeline);
    for e in d.edges.iter().filter(|e| graph && e.from == i && e.to == i) {
        let text = if e.label.is_empty() { "↻".to_string() } else { format!("↻ {}", e.label) };
        rows.push(Row::Text(labelled(&text, e.status), true));
    }
    rows
}

fn row_w(r: &Row) -> i64 {
    match r {
        Row::Text(runs, _) => runs_len(runs),
        Row::Sep => 0,
    }
}

/// The distance outside the box `b` (x, y, w, h) to the point.
fn dist(b: [i64; 4], p: (i64, i64)) -> i64 {
    let dx = (b[0] - p.0).max(p.0 - b[0] - b[2]).max(0);
    let dy = (b[1] - p.1).max(p.1 - b[1] - b[3]).max(0);
    dx + dy
}

/// Moves the end point `p` of a route to the border of box `b`. The route
/// comes from `prev`. An arch tile is smaller than its box, so the layout
/// ends the route inside the box.
fn snap(p: (i64, i64), prev: (i64, i64), b: [i64; 4]) -> (i64, i64) {
    let [x, y, w, h] = b;
    if p.1 == prev.1 {
        if prev.0 < x && p.0 > x {
            return (x, p.1);
        }
        if prev.0 > x + w && p.0 < x + w {
            return (x + w, p.1);
        }
    } else if p.0 == prev.0 {
        if prev.1 < y && p.1 > y {
            return (p.0, y);
        }
        if prev.1 > y + h && p.1 < y + h {
            return (p.0, y + h);
        }
    }
    p
}

fn edge_color(d: &Diagram, e: usize, cyc: &[bool]) -> u8 {
    let edge = &d.edges[e];
    if cyc.get(e).copied().unwrap_or(false) {
        RED
    } else if edge.status > 0 {
        STATUS_COLORS[edge.status]
    } else if edge.main {
        BLUE
    } else {
        0
    }
}

/// The mark at the end of an `erd` or `class` line that points in `dir`.
fn end_mark(name: &str, dir: (i64, i64)) -> Option<char> {
    let (dx, dy) = dir;
    match name {
        "" => None,
        "triangle" => Some(match (dx, dy) {
            (1, _) => '▷',
            (-1, _) => '◁',
            (_, 1) => '▽',
            _ => '△',
        }),
        "diamond" => Some('◆'),
        "hollow" => Some('◇'),
        "arrow" | "open" => Some(arrow(dx, dy)),
        s => s.chars().next(),
    }
}

fn graph_grid(d: &Diagram) -> (Grid, Vec<Vec<Run>>) {
    let l = layout_graph(d);
    let n = d.items.len();
    let rows: Vec<Vec<Row>> = (0..n).map(|i| if d.items[i].group { Vec::new() } else { node_rows(d, i) }).collect();
    let bx = |i: usize| [l.x[i], l.y[i], l.bw[i], l.bh[i]];

    let mut kx = vec![0, l.w];
    let mut ky = vec![0, l.h];
    let mut cx: Vec<(i64, i64, i64)> = Vec::new();
    let mut cy: Vec<(i64, i64, i64)> = Vec::new();
    for (i, it) in d.items.iter().enumerate() {
        let [x, y, w, h] = bx(i);
        kx.extend([x, x + w]);
        ky.extend([y, y + h]);
        if it.group {
            cx.push((x, x + w, runs_len(&labelled(&it.label, it.status)) + 6));
            cy.push((y, y + h, 2));
        } else {
            let tw = rows[i].iter().map(row_w).max().unwrap_or(0);
            cx.push((x, x + w, tw + 3));
            cy.push((y, y + h, rows[i].len() as i64 + 1));
        }
    }

    // Routes with the ends on the box borders.
    let mut routes: Vec<Vec<(i64, i64)>> = Vec::new();
    let mut from_first: Vec<bool> = Vec::new();
    for ge in &l.edges {
        let e = &d.edges[ge.edge];
        let mut p: Vec<(i64, i64)> = ge.pts.chunks(2).map(|c| (c[0], c[1])).collect();
        let first_from = dist(bx(e.from), p[0]) <= dist(bx(e.to), p[0]);
        if p.len() >= 2 {
            let (a, b) = if first_from { (e.from, e.to) } else { (e.to, e.from) };
            p[0] = snap(p[0], p[1], bx(a));
            let z = p.len();
            p[z - 1] = snap(p[z - 1], p[z - 2], bx(b));
            // The end cell before a box holds the arrowhead, so the last
            // straight run needs two cells.
            for (q, r) in [(p[0], p[1]), (p[z - 1], p[z - 2])] {
                if q.1 == r.1 {
                    cx.push((q.0.min(r.0), q.0.max(r.0), 2));
                } else {
                    cy.push((q.1.min(r.1), q.1.max(r.1), 2));
                }
            }
        }
        for &(x, y) in &p {
            kx.push(x);
            ky.push(y);
        }
        if ge.label_w > 0 {
            let half = idiv(ge.label_w, 2);
            let tw = runs_len(&labelled(&e.label, e.status));
            kx.extend([ge.label_x - half, ge.label_x + half]);
            ky.push(ge.label_y);
            cx.push((ge.label_x - half, ge.label_x + half, tw + 1));
        }
        routes.push(p);
        from_first.push(first_from);
    }

    let ax = Axis::new(kx, SX, cx);
    let ay = Axis::new(ky, SY, cy);
    let mut g = Grid::new(ax.last() + 1, ay.last() + 1);
    let cell_box = |i: usize| {
        let [x, y, w, h] = bx(i);
        [ax.at(x), ay.at(y), ax.at(x + w), ay.at(y + h)]
    };

    // Groups first, so that an edge that crosses a border makes a join.
    for (i, it) in d.items.iter().enumerate() {
        if it.group {
            g.rect(cell_box(i), it.status == 3, true, STATUS_COLORS[it.status]);
        }
    }

    let cyc = if d.dtype == DiagramType::Deps { find_cycles(d).edge } else { Vec::new() };
    let mut heads: Vec<(i64, i64, char, u8)> = Vec::new();
    for (k, ge) in l.edges.iter().enumerate() {
        let e = &d.edges[ge.edge];
        let p: Vec<(i64, i64)> = routes[k].iter().map(|&(x, y)| (ax.at(x), ay.at(y))).collect();
        if p.len() < 2 {
            continue;
        }
        let color = edge_color(d, ge.edge, &cyc);
        let dash = e.dashed || e.status == 3;
        let z = p.len();
        let dir = |a: (i64, i64), b: (i64, i64)| ((b.0 - a.0).signum(), (b.1 - a.1).signum());
        // The direction into the box at each end, and the cell before the box.
        let d0 = dir(p[1], p[0]);
        let d1 = dir(p[z - 2], p[z - 1]);
        let c0 = (p[0].0 - d0.0, p[0].1 - d0.1);
        let c1 = (p[z - 1].0 - d1.0, p[z - 1].1 - d1.1);
        let (m0, m1): (Option<char>, Option<char>) = if d.dtype.has_members() {
            let (mf, mt) = marks(d.dtype, e);
            let (a, b) = if from_first[k] { (mf, mt) } else { (mt, mf) };
            (end_mark(a, d0), end_mark(b, d1))
        } else {
            (ge.arrow_start.then(|| arrow(d0.0, d0.1)), ge.arrow_end.then(|| arrow(d1.0, d1.1)))
        };
        // An end with a mark stops before the border, so the border has no join.
        let mut q = p.clone();
        if m0.is_some() {
            q[0] = c0;
        }
        if m1.is_some() {
            q[z - 1] = c1;
        }
        for w in q.windows(2) {
            g.seg(w[0], w[1], dash, color, ge.edge as i32);
        }
        for (c, m) in [(c0, m0), (c1, m1)] {
            if let Some(ch) = m {
                heads.push((c.0, c.1, ch, color));
            }
        }
    }

    for (i, (it, rows_i)) in d.items.iter().zip(&rows).enumerate().take(n) {
        if it.group {
            continue;
        }
        let b = cell_box(i);
        g.block(b);
        let round = !d.dtype.has_tiles() && !d.dtype.has_members();
        g.rect(b, it.status == 3, round, STATUS_COLORS[it.status]);
        let inner = b[2] - b[0] - 1;
        // A card shows its text in the middle. A record starts at the top.
        let free_rows = b[3] - b[1] - 1 - rows_i.len() as i64;
        let top = b[1] + 1 + if d.dtype.has_members() { 0 } else { idiv(free_rows, 2) };
        for (y, r) in (top..).zip(rows_i) {
            match r {
                Row::Sep => {
                    g.seg((b[0], y), (b[2], y), false, STATUS_COLORS[it.status], BORDER);
                }
                Row::Text(runs, centre) => {
                    let x = if *centre { b[0] + 1 + idiv(inner - runs_len(runs), 2) } else { b[0] + 2 };
                    g.text(x, y, runs);
                }
            }
        }
    }
    for (x, y, ch, color) in heads {
        g.put(x, y, ch, color);
    }

    // Group labels go on the top border, where no line crosses it.
    for (i, it) in d.items.iter().enumerate() {
        if !it.group {
            continue;
        }
        let b = cell_box(i);
        let mut runs = labelled(&it.label, it.status);
        runs.insert(0, (" ".to_string(), 0));
        runs.push((" ".to_string(), 0));
        let tw = runs_len(&runs);
        let clear =
            |x0: i64| (x0..x0 + tw).all(|x| g.get(x, b[1]).is_some_and(|c| c.ch == '\0' && c.bits & (U | D) == 0));
        if let Some(x0) = (b[0] + 2..b[2] - tw).chain([b[0] + 1]).find(|&x0| clear(x0) && x0 + tw < b[2]) {
            g.text(x0, b[1], &runs);
        }
    }

    let mut notes = Vec::new();
    for ge in &l.edges {
        let e = &d.edges[ge.edge];
        if ge.label_w == 0 {
            continue;
        }
        let half = idiv(ge.label_w, 2);
        let mut runs = labelled(&e.label, e.status);
        runs.insert(0, (" ".to_string(), 0));
        runs.push((" ".to_string(), 0));
        let tw = runs_len(&runs);
        let mid = idiv(ax.at(ge.label_x - half) + ax.at(ge.label_x + half) + 1, 2);
        let row = ay.at(ge.label_y);
        let owner = ge.edge as i32;
        // Try the layout spot first, then spots near it, nearest first.
        let reach = idiv(tw, 2) + 2;
        let spot = [0, -1, 1, -2, 2, -3, 3].iter().find_map(|&dy| {
            (0..=2 * reach)
                .map(|k| if k % 2 == 0 { idiv(k, 2) } else { -idiv(k + 1, 2) })
                .map(|dx| (mid - idiv(tw, 2) + dx, row + dy))
                .find(|&(x, y)| g.free(x, y, tw, owner))
        });
        match spot {
            Some((x, y)) => g.text(x, y, &runs),
            None => notes.push(format!(
                "label of {} → {}: {}",
                d.items[e.from].label,
                d.items[e.to].label,
                plain(&labelled(&e.label, e.status))
            )),
        }
    }
    let mut out: Vec<Vec<Run>> = Vec::new();
    if !notes.is_empty() {
        out.push(Vec::new());
        out.extend(notes.into_iter().map(|s| vec![(s, GREY)]));
    }
    (g, out)
}

// ---------------------------------------------------------------- seq

fn seq_grid(d: &Diagram) -> (Grid, Vec<Vec<Run>>) {
    let l = layout_seq(d);
    let order = &d.roots;
    let np = order.len();
    let mut pos = vec![0usize; d.items.len()];
    for (k, &i) in order.iter().enumerate() {
        pos[i] = k;
    }
    let heads: Vec<Vec<Row>> = order.iter().map(|&i| node_rows(d, i)).collect();
    let texts: Vec<Vec<Run>> = d.edges.iter().map(|e| labelled(&e.label, e.status)).collect();

    // Each head is centred on its lifeline. `hw` is the half width.
    let hw: Vec<i64> = heads.iter().map(|h| idiv(h.iter().map(row_w).max().unwrap_or(0) + 5, 2)).collect();
    let mut kx = Vec::new();
    let mut cons = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        kx.push(l.cx[i]);
        if k + 1 < np {
            cons.push((l.cx[i], l.cx[order[k + 1]], hw[k] + hw[k + 1] + 2));
        }
    }
    if let Some(&first) = order.first() {
        kx.push(l.cx[first] - 1);
        cons.push((l.cx[first] - 1, l.cx[first], hw[0]));
    }
    for (k, e) in d.edges.iter().enumerate() {
        let a = l.cx[e.from];
        let tw = runs_len(&texts[k]);
        if e.from == e.to {
            let reach = a + 52 + l.label_w[k];
            kx.push(reach);
            cons.push((a, reach, tw + 6));
            continue;
        }
        // The label goes in the gap next to the sender, on the side of the
        // receiver.
        let (lo, hi) =
            if pos[e.to] > pos[e.from] { (a, l.cx[order[pos[e.from] + 1]]) } else { (l.cx[order[pos[e.from] - 1]], a) };
        cons.push((lo, hi, tw + 4));
    }
    let ax = Axis::new(kx, SX, cons);

    let head_rows = heads.iter().map(|h| h.len() as i64).max().unwrap_or(1) + 2;
    let mut rows = head_rows + 1;
    for e in &d.edges {
        rows += if e.from == e.to { 3 } else { 2 };
    }
    let shift = -ax.at(l.cx[order.first().copied().unwrap_or(0)] - 1);
    let col = |i: usize| ax.at(l.cx[i]) + shift;
    let mut g = Grid::new(ax.last() + shift + hw.last().copied().unwrap_or(0) + 1, rows + 1);

    for &i in order {
        g.seg((col(i), head_rows - 1), (col(i), rows), true, 0, NONE);
    }
    for (k, &i) in order.iter().enumerate() {
        let it = &d.items[i];
        let b = [col(i) - hw[k], 0, col(i) + hw[k], head_rows - 1];
        g.block(b);
        g.rect(b, it.status == 3, true, STATUS_COLORS[it.status]);
        for (r, row) in heads[k].iter().enumerate() {
            if let Row::Text(runs, _) = row {
                g.text(b[0] + 1 + idiv(b[2] - b[0] - 1 - runs_len(runs), 2), 1 + r as i64, runs);
            }
        }
    }

    let mut y = head_rows + 1;
    for (k, e) in d.edges.iter().enumerate() {
        let color = if e.status > 0 {
            STATUS_COLORS[e.status]
        } else if e.main {
            BLUE
        } else {
            0
        };
        let dash = e.dashed || e.status == 3;
        let a = col(e.from);
        if e.from == e.to {
            g.seg((a, y + 1), (a + 3, y + 1), dash, color, k as i32);
            g.seg((a + 3, y + 1), (a + 3, y + 2), dash, color, k as i32);
            g.seg((a + 3, y + 2), (a + 1, y + 2), dash, color, k as i32);
            g.put(a + 1, y + 2, '◀', color);
            g.text(a + 2, y, &texts[k]);
            y += 3;
            continue;
        }
        let b = col(e.to);
        let dx = (b - a).signum();
        // The arrowhead touches the lifeline of the receiver, without a join.
        let start = if e.bidir { a + dx } else { a };
        g.seg((start, y + 1), (b - dx, y + 1), dash, color, k as i32);
        g.put(b - dx, y + 1, arrow(dx, 0), color);
        let tw = runs_len(&texts[k]);
        let x = if dx > 0 { a + 2 } else { a - 1 - tw };
        g.text(x, y, &texts[k]);
        if e.bidir {
            g.put(a + dx, y + 1, arrow(-dx, 0), color);
        }
        y += 2;
    }
    (g, Vec::new())
}

// ---------------------------------------------------------------- timeline

fn timeline_grid(d: &Diagram) -> (Grid, Vec<Vec<Run>>) {
    let l = layout_timeline(d);
    let order = &l.order;
    let label = |i: usize| labelled(&d.items[i].label, d.items[i].status);
    let block = |i: usize| runs_len(&label(i)).max(len(&fit(&d.items[i].sub, MAX_TEXT)));
    let band_text = |b: usize| {
        let it = &d.items[l.bands[b].item];
        let mut runs = labelled(&it.label, it.status);
        runs.insert(0, (" ".to_string(), 0));
        runs.push((" ".to_string(), 0));
        runs
    };

    let mut kx = vec![l.axis_x0, l.axis_x1];
    let mut cons = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        kx.push(l.x[i]);
        if k + 1 < order.len() {
            cons.push((l.x[i], l.x[order[k + 1]], 2));
        }
        // Two labels on one side need a space between them.
        if k + 2 < order.len() {
            let j = order[k + 2];
            let (bi, bj) = (block(i), block(j));
            cons.push((l.x[i], l.x[j], bi - idiv(bi, 2) + idiv(bj, 2) + 2));
        }
    }
    if let (Some(&f), Some(&z)) = (order.first(), order.last()) {
        cons.push((l.axis_x0, l.x[f], 2));
        cons.push((l.x[z], l.axis_x1, 2));
    }
    for (b, band) in l.bands.iter().enumerate() {
        kx.extend([band.x, band.x + band.w]);
        cons.push((band.x, band.x + band.w, runs_len(&band_text(b)) + 3));
    }
    if let Some(s) = l.solid_to {
        kx.push(s);
    }
    let ax = Axis::new(kx, SX, cons);
    // A label can reach left of the axis start, so all columns move right.
    let shift = order.iter().map(|&i| idiv(block(i), 2) - ax.at(l.x[i])).max().unwrap_or(0).max(0);
    let col = |x: i64| ax.at(x) + shift;

    let has_sub = order.iter().any(|&i| !d.items[i].sub.is_empty());
    let text_rows = if has_sub { 2 } else { 1 };
    let axis = text_rows + 1;
    let band_row = axis + 2 + text_rows + 1;
    let right = order.iter().map(|&i| col(l.x[i]) + block(i)).max().unwrap_or(0).max(col(l.axis_x1));
    let mut g = Grid::new(right + 1, band_row + 1);

    let (x0, x1) = (col(l.axis_x0), col(l.axis_x1));
    match l.solid_to {
        Some(s) => {
            let s = col(s);
            g.seg((x0, axis), (s, axis), false, 0, NONE);
            if s < x1 {
                g.seg((s, axis), (x1, axis), true, 0, NONE);
            }
        }
        None => g.seg((x0, axis), (x1, axis), true, 0, NONE),
    }
    for &i in order {
        let it = &d.items[i];
        let c = col(l.x[i]);
        let color = STATUS_COLORS[it.status];
        let milestone = it.kind == 1;
        let mark = match (milestone, it.status == 3) {
            (true, false) => '◆',
            (true, true) => '◇',
            (false, false) => '●',
            (false, true) => '○',
        };
        let mut lines = vec![label(i)];
        if !it.sub.is_empty() {
            lines.push(grey(&it.sub));
        }
        if l.above[i] {
            g.seg((c, axis - 1), (c, axis), false, color, NONE);
            let top = axis - 1 - lines.len() as i64;
            for (r, runs) in lines.iter().enumerate() {
                g.text(c - idiv(runs_len(runs), 2), top + r as i64, runs);
            }
        } else {
            g.seg((c, axis), (c, axis + 1), false, color, NONE);
            for (r, runs) in lines.iter().enumerate() {
                g.text(c - idiv(runs_len(runs), 2), axis + 2 + r as i64, runs);
            }
        }
        g.put(c, axis, mark, color);
    }
    for (b, band) in l.bands.iter().enumerate() {
        let it = &d.items[band.item];
        let color = STATUS_COLORS[it.status];
        let (a, z) = (col(band.x), col(band.x + band.w));
        g.seg((a, band_row), (z, band_row), false, color, NONE);
        g.put(a, band_row, '├', color);
        g.put(z, band_row, '┤', color);
        g.text(a + 2, band_row, &band_text(b));
    }
    (g, Vec::new())
}

// ---------------------------------------------------------------- outline

fn outline(d: &Diagram) -> Vec<Vec<Run>> {
    let mut out: Vec<Vec<Run>> = Vec::new();
    let line = |indent: usize, mut runs: Vec<Run>, sub: &str| {
        runs.insert(0, (" ".repeat(indent), 0));
        if !sub.is_empty() {
            runs.push((format!("  {sub}"), GREY));
        }
        runs
    };
    if d.dtype == DiagramType::Seq {
        out.push(vec![("Participants".to_string(), 0)]);
        for &i in &d.roots {
            let it = &d.items[i];
            out.push(line(2, whole(&it.label, it.status), &it.sub));
        }
        out.push(Vec::new());
        out.push(vec![("Messages".to_string(), 0)]);
        for (k, e) in d.edges.iter().enumerate() {
            let mut runs = vec![(format!("{}. ", k + 1), 0)];
            runs.extend(edge_runs(d, e));
            out.push(line(2, runs, ""));
        }
        return out;
    }
    fn walk(d: &Diagram, list: &[usize], depth: usize, out: &mut Vec<Vec<Run>>) {
        for &i in list {
            let it = &d.items[i];
            let pad = " ".repeat(2 * depth);
            if it.group {
                let mut runs = vec![(pad, 0)];
                runs.extend(whole(&it.label, it.status).into_iter().map(|(s, c)| (s, if c == 0 { BLUE } else { c })));
                runs[1].0 = format!("[{}]", runs[1].0);
                out.push(runs);
                walk(d, &it.children, depth + 1, out);
                continue;
            }
            let lead = if d.dtype == DiagramType::Timeline && it.kind == 1 { "◆ " } else { "• " };
            let mut runs = vec![(format!("{pad}{lead}"), 0)];
            runs.extend(whole(&it.label, it.status));
            if !it.sub.is_empty() {
                runs.push((format!("  {}", it.sub), GREY));
            }
            out.push(runs);
            for m in &it.members {
                let mut runs = vec![(format!("{pad}    "), 0)];
                runs.extend(whole(&m.text, m.status));
                out.push(runs);
            }
        }
    }
    walk(d, &d.roots, 0, &mut out);
    if !d.edges.is_empty() {
        out.push(Vec::new());
        out.push(vec![("Edges".to_string(), 0)]);
        for e in &d.edges {
            out.push(line(2, edge_runs(d, e), ""));
        }
    }
    out
}

fn edge_runs(d: &Diagram, e: &Edge) -> Vec<Run> {
    let arrow = if e.bidir {
        "↔"
    } else if e.dashed {
        "⇢"
    } else {
        "→"
    };
    let mut runs = vec![(format!("{} {arrow} {}", d.items[e.from].label, d.items[e.to].label), 0)];
    if !e.label.is_empty() || e.status > 0 {
        runs.push(("  ".to_string(), 0));
        runs.extend(whole(&e.label, e.status));
    }
    runs
}
