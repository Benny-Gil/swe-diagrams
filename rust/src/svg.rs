//! SVG writer. The look: a graph-paper canvas, white icon tiles with the
//! name below, tinted group boxes, right-angle edges, and mono labels.
//! The style sheet holds literal colours, not CSS variables, so a rasterizer
//! without variable support gives the light theme.

use std::fmt::Write;

use crate::cycles::find_cycles;
use crate::icons::Icon;
use crate::layout_graph::GraphLayout;
use crate::layout_seq::SeqLayout;
use crate::model::{idiv, Diagram, DiagramType, STATUS_NAMES, STATUS_WORDS};
use crate::svg_shapes;
use crate::text::{measure, measure_mono};

const CSS: &str = concat!(
    "text{font-family:Geist,ui-sans-serif,system-ui,-apple-system,\"Segoe UI\",Helvetica,Arial,sans-serif;fill:#192029}",
    ".mo{font-family:\"JetBrains Mono\",ui-monospace,\"SF Mono\",Menlo,Consolas,monospace;letter-spacing:.04em}",
    ".pg{fill:#f8fafd}",
    ".gr{fill:none;stroke:#e5e8ed}",
    ".cv{fill:none;stroke:#c3c8ce}",
    ".kk{font-size:11px;fill:#1f6feb}",
    ".t{font-size:22px;font-weight:700}",
    ".lg{font-size:10px;fill:#6f757c}",
    ".g{fill:#e6f0fd;fill-opacity:.5;stroke:#1f6feb;stroke-opacity:.45;stroke-width:1.5}",
    ".gv{fill:#fbeadc;fill-opacity:.5;stroke:#dd7b2b;stroke-opacity:1}",
    ".gn{fill:none;stroke-dasharray:5 4}",
    ".gl{font-size:12px;font-weight:600;fill:#1f6feb}",
    ".glv{fill:#a64b00}",
    ".gp{fill:none}",
    ".sh{fill:#192029;fill-opacity:.06}",
    ".ti{fill:#fff;stroke:#c3c8ce}",
    ".td{fill:#1a2028;stroke:#1a2028}",
    ".x{stroke-dasharray:4 3}",
    ".ic{fill:none;stroke:#323841;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round}",
    ".nm{font-size:13px;font-weight:600;text-anchor:middle}",
    ".ro{font-size:9.5px;fill:#6f757c;text-anchor:middle}",
    ".vl{fill:#f8fafd;fill-opacity:.78}",
    ".cd{fill:#fff;stroke:#c3c8ce}",
    ".e{fill:none;stroke:#535961;stroke-width:1.4}",
    ".em{stroke:#1f6feb;stroke-width:1.6}",
    ".d{stroke-dasharray:4 5}",
    ".a{fill:#535961}",
    ".am{fill:#1f6feb}",
    ".el{font-size:10.5px;fill:#535961;text-anchor:middle;paint-order:stroke;stroke:#f8fafd;stroke-width:4px;stroke-linejoin:round}",
    ".lm{fill:#1f6feb}",
    ".ls{text-anchor:start}",
    ".ll{stroke:#c3c8ce;stroke-dasharray:3 4}",
    ".k{fill:none;stroke:#fff;stroke-width:1.8;stroke-linecap:round;stroke-linejoin:round}",
    ".kf{fill:#fff}",
    ".bq{fill:#fff;stroke:#c3c8ce;stroke-width:1.4}",
    ".bp{fill:#fff;stroke:#9aa1a9;stroke-width:1.4;stroke-dasharray:2.6 2.2}",
    ".ti.s1,.ti.s4,.cd.s1,.cd.s4,.e.s1,.e.s4,.g.s1,.g.s4{stroke:#1f8a4c;stroke-opacity:1}",
    ".cd.s1,.cd.s4{fill:#edf8f1}",
    ".a.s1,.a.s4,.b.s1,.b.s4{fill:#1f8a4c}",
    ".ti.s2,.cd.s2,.e.s2,.g.s2{stroke:#6f52e0;stroke-opacity:1}",
    ".cd.s2{fill:#f1eefe}",
    ".a.s2,.b.s2{fill:#6f52e0}",
    ".ti.s3,.cd.s3,.e.s3,.g.s3{stroke:#9aa1a9;stroke-opacity:1;stroke-dasharray:3 3}",
    ".a.s3{fill:#9aa1a9}",
    ".ti.s5,.cd.s5,.e.s5,.g.s5{stroke:#b06d08;stroke-opacity:1}",
    ".cd.s5{fill:#fdf4e3}",
    ".a.s5,.b.s5{fill:#b06d08}",
    ".ti.s6,.cd.s6,.e.s6,.g.s6{stroke:#cc3340;stroke-opacity:1}",
    ".cd.s6{fill:#fdeeee}",
    ".a.s6,.b.s6{fill:#cc3340}",
    ".ti.s1,.ti.s2,.ti.s4,.ti.s5,.ti.s6{stroke-width:1.5}",
    ".p3{fill:#6f757c}",
    ".p6{fill:#6f757c;text-decoration:line-through}",
    ".oa{stroke-dasharray:none}",
    "@media(prefers-color-scheme:dark){",
    "text{fill:#e8ebf1}",
    ".pg{fill:#0e1217}",
    ".gr{stroke:#1b2025}",
    ".cv{stroke:#3c434c}",
    ".kk,.gl{fill:#569fff}",
    ".lg,.ro,.p3,.p6{fill:#8a9098}",
    ".g{fill:#1a2a3f;fill-opacity:.35;stroke:#569fff;stroke-opacity:.5}",
    ".gv{fill:#2f2115;fill-opacity:.4;stroke:#e0843e;stroke-opacity:1}",
    ".gn{fill:none}",
    ".glv{fill:#f2a26a}",
    ".gp{fill:#fff}",
    ".vl{fill:#0e1217}",
    ".cd{fill:#161b21;stroke:#3c434c}",
    ".e{stroke:#a0a5ac}",
    ".em{stroke:#569fff}",
    ".a{fill:#a0a5ac}",
    ".am{fill:#569fff}",
    ".el{fill:#a0a5ac;stroke:#0e1217}",
    ".lm{fill:#569fff}",
    ".ll{stroke:#3c434c}",
    ".k{stroke:#0e1217}",
    ".kf{fill:#0e1217}",
    ".bq{fill:#161b21;stroke:#3c434c}",
    ".bp{fill:#161b21;stroke:#6c737c}",
    ".ti.s1,.ti.s4,.cd.s1,.cd.s4,.e.s1,.e.s4,.g.s1,.g.s4{stroke:#4cc27a}",
    ".cd.s1,.cd.s4{fill:#13281c}",
    ".a.s1,.a.s4,.b.s1,.b.s4{fill:#4cc27a}",
    ".ti.s2,.cd.s2,.e.s2,.g.s2{stroke:#a48cff}",
    ".cd.s2{fill:#211a3d}",
    ".a.s2,.b.s2{fill:#a48cff}",
    ".ti.s3,.cd.s3,.e.s3,.g.s3{stroke:#6c737c}",
    ".a.s3{fill:#6c737c}",
    ".ti.s5,.cd.s5,.e.s5,.g.s5{stroke:#dba13a}",
    ".cd.s5{fill:#2b2212}",
    ".a.s5,.b.s5{fill:#dba13a}",
    ".ti.s6,.cd.s6,.e.s6,.g.s6{stroke:#f0626c}",
    ".cd.s6{fill:#2e1618}",
    ".a.s6,.b.s6{fill:#f0626c}",
    "}",
);

/// Status badges, drawn around the origin. Index = status.
pub(crate) const BADGES: [&str; 7] = [
    "<circle r=\"7.4\" class=\"bq\"/>",
    "<circle r=\"8\" class=\"b s1\"/><path class=\"k\" d=\"M-3.6 .3l2.5 2.5 4.7-5.3\"/>",
    "<circle r=\"8\" class=\"b s2\"/><circle r=\"4.2\" class=\"k\"/><path class=\"kf\" d=\"M0-4.2a4.2 4.2 0 0 1 0 8.4z\"/>",
    "<circle r=\"7.4\" class=\"bp\"/>",
    "<circle r=\"8\" class=\"b s4\"/><path class=\"k\" d=\"M-3.6 0h7.2M0-3.6v7.2\"/>",
    "<circle r=\"8\" class=\"b s5\"/><circle r=\"2.4\" class=\"kf\"/>",
    "<circle r=\"8\" class=\"b s6\"/><path class=\"k\" d=\"M-3.6 0h7.2\"/>",
];

/// Line icons for a node with no logo, in a 24 by 24 box. Index = kind.
const GLYPHS: [&str; 5] = [
    // service: two stacked units
    "M5 4h14a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM5 13h14a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1zM8 7.5h.01M8 16.5h.01",
    // store: a cylinder
    "M4 6c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3",
    // queue: three messages in a row
    "M3 8h4v8H3zM10 8h4v8h-4zM17 8h4v8h-4z",
    // actor: a person
    "M12 4a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM4 21c0-3.9 3.6-6 8-6s8 2.1 8 6",
    // ext: a globe
    "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18M12 3c2.5 2.7 3.8 5.7 3.8 9s-1.3 6.3-3.8 9c-2.5-2.7-3.8-5.7-3.8-9S9.5 5.7 12 3z",
];

/// An edge in a dependency cycle gets the colour of `del`.
const CYCLE_STYLE: (&str, &str) =
    (".e.cy{stroke:#cc3340}.a.cy{fill:#cc3340}", ".e.cy{stroke:#f0626c}.a.cy{fill:#f0626c}");

/// In a change diagram, a part with no status is quiet.
const QUIET_STYLE: (&str, &str) = (".q{opacity:.38}", "");

const LABEL_END_STYLE: (&str, &str) = (".le{text-anchor:end}", "");

const PAD: i64 = 24; // page padding
const INNER: i64 = 32; // canvas padding
const GAP: i64 = 4; // clear space between a line end and its target

pub(crate) fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn st(status: usize) -> String {
    if status > 0 {
        format!(" s{status}")
    } else {
        String::new()
    }
}

/// Opens the group of a node or a group. A script finds the part by these
/// attributes. `kind` is `node` or `group`.
pub(crate) fn open_part(out: &mut String, kind: &str, id: &str, status: usize) {
    let _ = write!(out, "<g data-k=\"{kind}\" data-id=\"{}\" data-s=\"{}\">", esc(id), STATUS_WORDS[status]);
}

/// Opens the group of edge `k`. `kind` is `edge` or `msg`.
pub(crate) fn open_edge(out: &mut String, kind: &str, d: &Diagram, k: usize) {
    let e = &d.edges[k];
    let _ = write!(
        out,
        "<g data-k=\"{kind}\" data-i=\"{k}\" data-from=\"{}\" data-to=\"{}\" data-s=\"{}\">",
        esc(&d.items[e.from].id),
        esc(&d.items[e.to].id),
        STATUS_WORDS[e.status]
    );
}

/// The class of the text of a part with a status that makes the text quiet.
pub(crate) fn quiet(status: usize) -> &'static str {
    match status {
        3 => " p3",
        6 => " p6",
        _ => "",
    }
}

/// The frame that all diagram types share: page, header, canvas, legend.
pub(crate) struct Frame {
    pub(crate) body: String,
    title: String,
    kicker: String,
    pub(crate) used: [bool; 7],
    lines: [bool; 3], // main flow, connection, dashed
    /// The legend names of the three line kinds.
    line_names: [&'static str; 3],
    sources: Vec<String>,
    /// More legend lines after the line kinds: the class of the line, and the name.
    marks: Vec<(&'static str, &'static str)>,
    /// More style rules, light then dark. Only a diagram that uses them gets them.
    styles: Vec<(&'static str, &'static str)>,
    /// Legend entries with a drawn sample: the SVG from (0, 0) to (34, 0), and the name.
    glyphs: Vec<(String, &'static str)>,
}

impl Frame {
    pub(crate) fn new(title: &str, kicker: String) -> Frame {
        Frame {
            body: String::new(),
            title: title.to_string(),
            kicker,
            used: [false; 7],
            lines: [false; 3],
            line_names: ["main flow", "connection", "async / indirect"],
            sources: Vec::new(),
            marks: Vec::new(),
            styles: Vec::new(),
            glyphs: Vec::new(),
        }
    }

    fn top(&self) -> i64 {
        if self.title.is_empty() {
            PAD
        } else {
            84
        }
    }

    /// Origin of the diagram content.
    pub(crate) fn origin(&self) -> (i64, i64) {
        (PAD + INNER, self.top() + INNER)
    }

    fn style(&mut self, rules: (&'static str, &'static str)) {
        if !self.styles.contains(&rules) {
            self.styles.push(rules);
        }
    }

    fn mark(&mut self, class: &'static str, name: &'static str) {
        if !self.marks.contains(&(class, name)) {
            self.marks.push((class, name));
        }
    }

    pub(crate) fn badge(&mut self, status: usize, x: i64, y: i64) {
        self.used[status] = true;
        let _ = write!(self.body, "<use href=\"#b{status}\" x=\"{x}\" y=\"{y}\"/>");
    }

    pub(crate) fn finish(self, inner_w: i64, inner_h: i64) -> String {
        let top = self.top();
        let ch = inner_h + 2 * INNER;

        // Legend: the line kinds when the diagram mixes them, then the statuses.
        let mut legend = String::new();
        let ly = top + ch + 24;
        let mut lx = PAD;
        let kinds = self.lines.iter().filter(|&&b| b).count();
        if kinds > 1 {
            let names = self.line_names;
            let classes = ["e em", "e", "e d"];
            for k in 0..3 {
                if !self.lines[k] {
                    continue;
                }
                let _ = write!(
                    legend,
                    "<path class=\"{}\" d=\"M{} {}h30\"/><text class=\"lg mo\" x=\"{}\" y=\"{}\">{}</text>",
                    classes[k],
                    lx,
                    ly - 3,
                    lx + 36,
                    ly,
                    names[k]
                );
                lx += 36 + measure_mono(names[k], 100) + 24;
            }
        }
        for (svg, name) in &self.glyphs {
            let _ = write!(
                legend,
                "<g transform=\"translate({} {})\">{svg}</g><text class=\"lg mo\" x=\"{}\" y=\"{}\">{name}</text>",
                lx,
                ly - 3,
                lx + 40,
                ly
            );
            lx += 40 + measure_mono(name, 100) + 24;
        }
        for &(class, name) in &self.marks {
            let _ = write!(
                legend,
                "<path class=\"{class}\" d=\"M{} {}h30\"/><text class=\"lg mo\" x=\"{}\" y=\"{}\">{name}</text>",
                lx,
                ly - 3,
                lx + 36,
                ly
            );
            lx += 36 + measure_mono(name, 100) + 24;
        }
        for (s, name) in STATUS_NAMES.iter().enumerate().skip(1) {
            if !self.used[s] {
                continue;
            }
            let _ = write!(
                legend,
                "<use href=\"#b{s}\" x=\"{}\" y=\"{}\"/><text class=\"lg mo\" x=\"{}\" y=\"{}\">{}</text>",
                lx + 8,
                ly - 3,
                lx + 22,
                ly,
                name
            );
            lx += 22 + measure_mono(name, 100) + 24;
        }
        let credit =
            if self.sources.is_empty() { String::new() } else { format!("icons: {}", self.sources.join(" / ")) };
        let credit_w = if credit.is_empty() { 0 } else { measure_mono(&credit, 100) + 24 };
        let has_legend = lx > PAD || !credit.is_empty();

        let title_w = if self.title.is_empty() { 0 } else { measure(&self.title, 22, true) };
        let kicker_w = measure_mono(&self.kicker, 110);
        let cw = (inner_w + 2 * INNER).max(title_w).max(kicker_w).max(lx - PAD - 24 + credit_w).max(240);
        let w = cw + 2 * PAD;
        let h = top + ch + if has_legend { 40 } else { PAD };

        let mut defs = format!(
            "<pattern id=\"gr\" width=\"24\" height=\"24\" patternUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\"><path class=\"gr\" d=\"M24 .5H.5V24\"/></pattern>",
            PAD - 1,
            top - 1
        );
        for (s, badge) in BADGES.iter().enumerate() {
            if self.used[s] {
                let _ = write!(defs, "<g id=\"b{s}\">{badge}</g>");
            }
        }
        let mut out =
            format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" role=\"img\"");
        if !self.title.is_empty() {
            let _ = write!(out, " aria-label=\"{}\"", esc(&self.title));
        }
        let _ = write!(out, "><style>{CSS}");
        if !self.styles.is_empty() {
            for (light, _) in &self.styles {
                out.push_str(light);
            }
            out.push_str("@media(prefers-color-scheme:dark){");
            for (_, dark) in &self.styles {
                out.push_str(dark);
            }
            out.push('}');
        }
        let _ = write!(out, "</style><defs>{defs}</defs>");
        let _ = write!(out, "<rect class=\"pg\" width=\"{w}\" height=\"{h}\" rx=\"12\"/>");
        if !self.title.is_empty() {
            let _ = write!(out, "<text class=\"kk mo\" x=\"{PAD}\" y=\"36\">{}</text>", esc(&self.kicker));
            let _ = write!(out, "<text class=\"t\" x=\"{PAD}\" y=\"66\">{}</text>", esc(&self.title));
        }
        let _ =
            write!(out, "<rect fill=\"url(#gr)\" x=\"{PAD}\" y=\"{top}\" width=\"{cw}\" height=\"{ch}\" rx=\"10\"/>");
        let _ = write!(
            out,
            "<rect class=\"cv\" x=\"{}.5\" y=\"{}.5\" width=\"{}\" height=\"{}\" rx=\"10\"/>",
            PAD,
            top,
            cw - 1,
            ch - 1
        );
        out.push_str(&self.body);
        out.push_str(&legend);
        if !credit.is_empty() {
            let _ = write!(
                out,
                "<text class=\"lg mo\" x=\"{}\" y=\"{}\" text-anchor=\"end\">{}</text>",
                w - PAD,
                ly,
                esc(&credit)
            );
        }
        out.push_str("</svg>\n");
        out
    }
}

/// A path with round corners through points on a right-angle route. The
/// line stops short of both ends, and shorter where an arrowhead sits.
pub(crate) fn route_path(p: &[i64], arrow_start: bool, arrow_end: bool) -> String {
    let mut pts = p.to_vec();
    let z = pts.len();
    let cut_end = GAP + if arrow_end { 5 } else { 0 };
    let cut_start = GAP + if arrow_start { 5 } else { 0 };
    pts[z - 2] -= cut_end * (pts[z - 2] - pts[z - 4]).signum();
    pts[z - 1] -= cut_end * (pts[z - 1] - pts[z - 3]).signum();
    pts[0] -= cut_start * (pts[0] - pts[2]).signum();
    pts[1] -= cut_start * (pts[1] - pts[3]).signum();
    let mut out = format!("M{} {}", pts[0], pts[1]);
    let mut k = 2;
    while k + 2 < z {
        let dx1 = pts[k] - pts[k - 2];
        let dy1 = pts[k + 1] - pts[k - 1];
        let dx2 = pts[k + 2] - pts[k];
        let dy2 = pts[k + 3] - pts[k + 1];
        let r = 6.min((dx1.abs() + dy1.abs()) >> 1).min((dx2.abs() + dy2.abs()) >> 1);
        if r <= 0 {
            let _ = write!(out, "L{} {}", pts[k], pts[k + 1]);
        } else {
            let _ = write!(out, "L{} {}", pts[k] - r * dx1.signum(), pts[k + 1] - r * dy1.signum());
            let _ = write!(
                out,
                "Q{} {} {} {}",
                pts[k],
                pts[k + 1],
                pts[k] + r * dx2.signum(),
                pts[k + 1] + r * dy2.signum()
            );
        }
        k += 2;
    }
    let _ = write!(out, "L{} {}", pts[z - 2], pts[z - 1]);
    out
}

/// A solid arrowhead that points along (dx, dy). The tip is `GAP` before (x, y).
fn arrow_head(x: i64, y: i64, dx: i64, dy: i64) -> String {
    let tx = x - GAP * dx;
    let ty = y - GAP * dy;
    let bx = tx - 7 * dx;
    let by = ty - 7 * dy;
    format!("M{tx} {ty}L{} {}L{} {}z", bx - 3 * dy, by + 3 * dx, bx + 3 * dy, by - 3 * dx)
}

/// The class of an edge line or an arrowhead.
fn edge_class(base: &str, main: bool, dashed: bool, status: usize) -> String {
    let mut c = base.to_string();
    if main {
        c.push_str(if base == "a" { " am" } else { " em" });
    }
    if dashed && base == "e" {
        c.push_str(" d");
    }
    c.push_str(&st(status));
    c
}

/// A card: the box of a plan task or a sequence participant.
fn card(kind: usize, status: usize, x: i64, y: i64, w: i64, h: i64) -> String {
    let rx = if kind == 3 { idiv(h, 2) } else { 8 };
    let ext = if kind == 4 { " x" } else { "" };
    format!(
        "<rect class=\"sh\" x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\"/><rect class=\"cd{ext}{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\"/>",
        y + 1,
        st(status)
    )
}

pub(crate) fn card_text(label: &str, sub: &str, status: usize, cx: i64, cy: i64) -> String {
    let p = quiet(status);
    if sub.is_empty() {
        return format!("<text class=\"nm{p}\" x=\"{cx}\" y=\"{}\">{}</text>", cy + 5, esc(label));
    }
    format!(
        "<text class=\"nm{p}\" x=\"{cx}\" y=\"{}\">{}</text><text class=\"ro mo\" x=\"{cx}\" y=\"{}\">{}</text>",
        cy - 3,
        esc(label),
        cy + 13,
        esc(sub)
    )
}

/// The box of the shape of item `i`, without the room for a loop.
fn shape_box(d: &Diagram, l: &GraphLayout, i: usize) -> [i64; 4] {
    if d.down {
        [l.x[i] + l.pre[i], l.y[i], l.tile[i], l.bh[i]]
    } else {
        [l.x[i], l.y[i] + l.pre[i], l.bw[i], l.tile[i]]
    }
}

/// Moves the end of the route `p` at item `i` from the box side into the
/// box, to the outline of a slanted shape. The last run gets longer, so the
/// arrowhead keeps its gap.
fn to_outline(d: &Diagram, l: &GraphLayout, i: usize, p: &mut [i64]) {
    if !svg_shapes::slanted(d, i) {
        return;
    }
    let b = shape_box(d, l, i);
    let on_box = |x: i64, y: i64| x >= b[0] && x <= b[0] + b[2] && y >= b[1] && y <= b[1] + b[3];
    let z = p.len();
    // (end, the point before it)
    let (k, j) = if on_box(p[0], p[1]) {
        (0, 2)
    } else if on_box(p[z - 2], p[z - 1]) {
        (z - 2, z - 4)
    } else {
        return;
    };
    let m = usize::from(d.down); // index of the main coordinate
    let inset = svg_shapes::outline_inset(d, i, b, p[k], p[k + 1]);
    p[k + m] += inset * (p[k + m] - p[j + m]).signum();
}

/// The count of the nodes and groups with `add`, `mod`, and `del`.
pub fn change_counts(d: &Diagram) -> [usize; 3] {
    let mut out = [0; 3];
    for it in &d.items {
        if (4..=6).contains(&it.status) {
            out[it.status - 4] += 1;
        }
    }
    out
}

pub fn render_graph(d: &Diagram, l: &GraphLayout, icons: &[Option<Icon>]) -> String {
    let tiles = d.dtype.has_tiles();
    let kicker = match d.dtype {
        DiagramType::Plan => {
            let total = d.items.iter().filter(|it| !it.group).count();
            let done = d.items.iter().filter(|it| !it.group && it.status == 1).count();
            format!("plan / {done} of {total} done")
        }
        DiagramType::Deps => "dependencies".to_string(),
        DiagramType::State => "state machine".to_string(),
        DiagramType::Dataflow => "data flow".to_string(),
        DiagramType::Erd => "entities".to_string(),
        DiagramType::Class => "classes".to_string(),
        DiagramType::Arch => "architecture".to_string(),
        DiagramType::Journey => "user journey".to_string(),
        DiagramType::Change => {
            let [add, m, del] = change_counts(d);
            format!("change / +{add} ~{m} -{del}")
        }
        t => t.name().to_string(),
    };
    let mut f = Frame::new(&d.title, kicker);
    let records = d.dtype.has_members();
    if records {
        f.style((crate::records::LIGHT, crate::records::DARK));
        f.glyphs = crate::records::class_legend(d);
        if d.dtype == DiagramType::Class {
            f.style((crate::records::CLASS_LIGHT, crate::records::CLASS_DARK));
        }
    }
    let (ox, oy) = f.origin();
    if d.dtype == DiagramType::State {
        f.style(svg_shapes::STATE_STYLE);
    }
    if d.dtype == DiagramType::Dataflow {
        f.style(svg_shapes::DATAFLOW_STYLE);
    }
    if d.dtype == DiagramType::Journey {
        f.style(crate::svg_journey::STYLE);
    }
    let change = d.dtype == DiagramType::Change;
    if change {
        f.style(QUIET_STYLE);
    }
    if d.dtype == DiagramType::Deps {
        f.line_names[1] = "import";
        f.line_names[2] = "type-only / dev";
    }
    let cycle = if d.dtype == DiagramType::Deps { find_cycles(d).edge } else { vec![false; d.edges.len()] };

    for (i, it) in d.items.iter().enumerate() {
        if !it.group {
            continue;
        }
        let mut class = String::from("g");
        if it.vendor {
            class.push_str(" gv");
        }
        // A group inside a group has no fill, so the tints do not add up.
        if it.parent.is_some() {
            class.push_str(" gn");
        }
        class.push_str(&st(it.status));
        if change && it.status == 0 {
            class.push_str(" q");
        }
        open_part(&mut f.body, "group", &it.id, it.status);
        let _ = write!(
            f.body,
            "<rect class=\"{class}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\"/></g>",
            ox + l.x[i],
            oy + l.y[i],
            l.bw[i],
            l.bh[i]
        );
    }

    for ge in &l.edges {
        let e = &d.edges[ge.edge];
        let mut p: Vec<i64> = ge.pts.clone();
        if svg_shapes::draws(d.dtype) {
            to_outline(d, l, e.from, &mut p);
            to_outline(d, l, e.to, &mut p);
        }
        let p: Vec<i64> = p.as_chunks::<2>().0.iter().flat_map(|c| [c[0] + ox, c[1] + oy]).collect();
        let z = p.len();
        if e.status > 0 {
            f.used[e.status] = true;
        }
        open_edge(&mut f.body, "edge", d, ge.edge);
        if records {
            // The end marks tell the kind of a relation, so no line legend.
            crate::records::draw_edge(&mut f.body, d.dtype, e, &p, ge.arrow_start);
            f.body.push_str("</g>");
            continue;
        }
        f.lines[if e.main {
            0
        } else if e.dashed {
            2
        } else {
            1
        }] = true;
        let mut lc = edge_class("e", e.main, e.dashed, e.status);
        let mut ac = edge_class("a", e.main, false, e.status);
        if cycle[ge.edge] {
            lc.push_str(" cy");
            ac.push_str(" cy");
            f.style(CYCLE_STYLE);
            f.mark("e cy", "cycle");
        }
        let quiet_edge = change && e.status == 0;
        if quiet_edge {
            f.body.push_str("<g class=\"q\">");
        }
        let _ = write!(f.body, "<path class=\"{lc} rt\" d=\"{}\"/>", route_path(&p, ge.arrow_start, ge.arrow_end));
        if ge.arrow_end {
            let head = arrow_head(p[z - 2], p[z - 1], (p[z - 2] - p[z - 4]).signum(), (p[z - 1] - p[z - 3]).signum());
            let _ = write!(f.body, "<path class=\"{ac}\" d=\"{head}\"/>");
        }
        if ge.arrow_start {
            let head = arrow_head(p[0], p[1], (p[0] - p[2]).signum(), (p[1] - p[3]).signum());
            let _ = write!(f.body, "<path class=\"{ac}\" d=\"{head}\"/>");
        }
        if quiet_edge {
            f.body.push_str("</g>");
        }
        f.body.push_str("</g>");
    }

    for (i, it) in d.items.iter().enumerate() {
        let x = ox + l.x[i];
        let y = oy + l.y[i];
        let w = l.bw[i];
        let h = l.bh[i];
        let icon = icons.get(i).and_then(|o| o.as_ref());
        if let Some(ic) = icon {
            if !f.sources.contains(&ic.source) && ic.source != "local" {
                f.sources.push(ic.source.clone());
            }
        }
        open_part(&mut f.body, if it.group { "group" } else { "node" }, &it.id, it.status);
        if it.group {
            let mut tx = x + 12;
            if let Some(ic) = icon {
                // The plate keeps a dark mark visible on the dark theme.
                let _ = write!(
                    f.body,
                    "<rect class=\"gp\" x=\"{}\" y=\"{}\" width=\"18\" height=\"18\" rx=\"3\"/><image href=\"{}\" x=\"{}\" y=\"{}\" width=\"14\" height=\"14\"/>",
                    tx - 2,
                    y + 10,
                    ic.uri,
                    tx,
                    y + 12
                );
                tx += 20;
            }
            let gl = match (it.vendor, change && it.status == 0) {
                (false, false) => "gl",
                (true, false) => "gl glv",
                (false, true) => "gl q",
                (true, true) => "gl glv q",
            };
            let _ = write!(f.body, "<text class=\"{gl}\" x=\"{tx}\" y=\"{}\">{}</text>", y + 24, esc(&it.label));
            if it.status > 0 {
                f.badge(it.status, x + w - 2, y + 2);
            }
            f.body.push_str("</g>");
            continue;
        }
        if records {
            for s in crate::records::draw_record(&mut f.body, d.dtype, it, x, y, w, h) {
                f.used[s] = true;
            }
            if it.status > 0 {
                f.badge(it.status, x + w - 2, y + 2);
            }
            f.body.push_str("</g>");
            continue;
        }
        if svg_shapes::draws(d.dtype) {
            let [sx, sy, sw, sh] = shape_box(d, l, i);
            let (shape, bx, by) = svg_shapes::node(d, i, sx + ox, sy + oy, sw, sh);
            f.body.push_str(&shape);
            if it.status > 0 {
                f.badge(it.status, bx, by);
            }
            f.body.push_str("</g>");
            if let Some(k) = svg_shapes::first_loop(d, i) {
                open_edge(&mut f.body, "edge", d, k);
                let e = &d.edges[k];
                let lm = if e.main { " lm" } else { "" };
                if e.status > 0 {
                    f.used[e.status] = true;
                }
                let [sx, sy, sw, sh] = shape_box(d, l, i);
                f.body.push_str(&svg_shapes::self_loop(
                    d,
                    i,
                    [sx + ox, sy + oy, sw, sh],
                    &edge_class("e", e.main, e.dashed, e.status),
                    &edge_class("a", e.main, false, e.status),
                    lm,
                ));
                f.body.push_str("</g>");
            }
            continue;
        }
        if !tiles {
            // The plan card kinds are the arch kinds. Other types have their own.
            let kind = if d.dtype == DiagramType::Plan { it.kind } else { 0 };
            f.body.push_str(&card(kind, it.status, x, y, w, h));
            let cy = y + idiv(h, 2);
            f.badge(it.status, x + 18, cy);
            f.body.push_str(&card_text(&it.label, &it.sub, it.status, x + 11 + idiv(w, 2), cy));
            f.body.push_str("</g>");
            continue;
        }

        let quiet_node = change && it.status == 0;
        if quiet_node {
            f.body.push_str("<g class=\"q\">");
        }
        // An arch node: the tile, the icon, then the name and the role.
        let (tw, th) = if d.down { (l.tile[i], 60) } else { (60, l.tile[i]) };
        let tx = x + idiv(w - tw, 2);
        let cx = x + idiv(w, 2);
        let dark = icon.is_some_and(|ic| ic.dark_tile);
        let mut class = String::from(if dark { "ti td" } else { "ti" });
        if it.kind == 4 {
            class.push_str(" x");
        }
        class.push_str(&st(it.status));
        let _ = write!(
            f.body,
            "<rect class=\"sh\" x=\"{tx}\" y=\"{}\" width=\"{tw}\" height=\"{th}\" rx=\"12\"/><rect class=\"{class}\" x=\"{tx}\" y=\"{y}\" width=\"{tw}\" height=\"{th}\" rx=\"12\"/>",
            y + 1
        );
        let icy = y + idiv(th, 2);
        match icon {
            Some(ic) => {
                let _ = write!(
                    f.body,
                    "<image href=\"{}\" x=\"{}\" y=\"{}\" width=\"34\" height=\"34\"/>",
                    ic.uri,
                    cx - 17,
                    icy - 17
                );
            }
            None => {
                let _ = write!(
                    f.body,
                    "<path class=\"ic\" transform=\"translate({} {}) scale(1.25)\" d=\"{}\"/>",
                    cx - 15,
                    icy - 15,
                    GLYPHS[it.kind]
                );
            }
        }
        let p = quiet(it.status);
        let nw = measure(&it.label, 13, true);
        let ty = y + th;
        let _ = write!(
            f.body,
            "<rect class=\"vl\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"17\" rx=\"3\"/><text class=\"nm{p}\" x=\"{cx}\" y=\"{}\">{}</text>",
            cx - idiv(nw, 2) - 4,
            ty + 8,
            nw + 8,
            ty + 21,
            esc(&it.label)
        );
        if !it.sub.is_empty() {
            let rw = measure_mono(&it.sub, 95);
            let _ = write!(
                f.body,
                "<rect class=\"vl\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"13\" rx=\"3\"/><text class=\"ro mo\" x=\"{cx}\" y=\"{}\">{}</text>",
                cx - idiv(rw, 2) - 4,
                ty + 27,
                rw + 8,
                ty + 37,
                esc(&it.sub)
            );
        }
        if it.status > 0 {
            f.badge(it.status, tx + tw - 2, y + 2);
        }
        if quiet_node {
            f.body.push_str("</g>");
        }
        f.body.push_str("</g>");
    }

    for ge in &l.edges {
        if ge.label_w == 0 {
            continue;
        }
        let e = &d.edges[ge.edge];
        let lm = if e.main { " lm" } else { "" };
        open_edge(&mut f.body, "edge", d, ge.edge);
        if d.dtype == DiagramType::Dataflow {
            // The label is the data that moves, in a pill on the line.
            let (mut cx, cy) = (ge.label_x + ox, ge.label_y + oy);
            if d.down && ge.label_side != 0 {
                cx += ge.label_side * (8 + idiv(ge.label_w + 14, 2));
            }
            let _ = write!(
                f.body,
                "<rect class=\"pl\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"18\" rx=\"9\"/><text class=\"el ip mo{lm}\" x=\"{cx}\" y=\"{}\">{}</text>",
                cx - idiv(ge.label_w, 2) - 7,
                cy - 9,
                ge.label_w + 14,
                cy + 4,
                esc(&e.label)
            );
            f.body.push_str("</g>");
            continue;
        }
        // Across the layers the label sits above the line. Along a vertical
        // line it sits at the right of the line.
        let (ls, dx, dy) = match (d.down, ge.label_side) {
            (false, _) => ("", 0, -7),
            (true, 0) => ("", 0, 4),
            (true, s) if s < 0 => (" le", -8, 4),
            _ => (" ls", 8, 4),
        };
        if ls == " le" {
            f.style(LABEL_END_STYLE);
        }
        let _ = write!(
            f.body,
            "<text class=\"el mo{ls}{lm}{}\" x=\"{}\" y=\"{}\">{}</text>",
            if change && e.status == 0 { " q" } else { "" },
            ge.label_x + ox + dx,
            ge.label_y + oy + dy,
            esc(&e.label)
        );
        f.body.push_str("</g>");
    }
    f.finish(l.w, l.h)
}

pub fn render_seq(d: &Diagram, l: &SeqLayout) -> String {
    let mut f = Frame::new(&d.title, "sequence".to_string());
    let (ox, oy) = f.origin();
    let bottom = oy + l.h;

    for &i in &d.roots {
        let cx = ox + l.cx[i];
        open_part(&mut f.body, "node", &d.items[i].id, d.items[i].status);
        let _ = write!(f.body, "<path class=\"ll\" d=\"M{cx} {}V{bottom}\"/></g>", oy + l.head_h);
    }
    for (k, e) in d.edges.iter().enumerate() {
        if e.status > 0 {
            f.used[e.status] = true;
        }
        let lc = edge_class("e", e.main, e.dashed, e.status);
        let ac = edge_class("a", e.main, false, e.status);
        let lm = if e.main { " lm" } else { "" };
        let y = oy + l.my[k];
        let x1 = ox + l.cx[e.from];
        open_edge(&mut f.body, "msg", d, k);
        if e.from == e.to {
            let _ = write!(
                f.body,
                "<path class=\"{lc} rt\" d=\"M{x1} {y}H{}Q{} {y} {} {}V{}Q{} {} {} {}H{}\"/>",
                x1 + 22,
                x1 + 28,
                x1 + 28,
                y + 6,
                y + 12,
                x1 + 28,
                y + 18,
                x1 + 22,
                y + 18,
                x1 + 5
            );
            let _ = write!(f.body, "<path class=\"{ac}\" d=\"M{x1} {}l7 -3v6z\"/>", y + 18);
            if !e.label.is_empty() {
                let _ = write!(
                    f.body,
                    "<text class=\"el ls mo{lm}\" x=\"{}\" y=\"{}\">{}</text>",
                    x1 + 36,
                    y + 13,
                    esc(&e.label)
                );
            }
            f.body.push_str("</g>");
            continue;
        }
        let x2 = ox + l.cx[e.to];
        let dir = (x2 - x1).signum();
        if e.dashed {
            // A reply has an open arrowhead.
            let _ = write!(f.body, "<path class=\"{lc} rt\" d=\"M{x1} {y}H{x2}\"/>");
            let _ = write!(
                f.body,
                "<path class=\"{} oa\" d=\"M{} {}L{x2} {y}L{} {}\"/>",
                edge_class("e", e.main, false, e.status),
                x2 - 7 * dir,
                y - 3,
                x2 - 7 * dir,
                y + 3
            );
        } else {
            let _ = write!(f.body, "<path class=\"{lc} rt\" d=\"M{x1} {y}H{}\"/>", x2 - 5 * dir);
            let _ = write!(f.body, "<path class=\"{ac}\" d=\"M{x2} {y}l{} -3v6z\"/>", -7 * dir);
        }
        if e.bidir {
            let _ = write!(f.body, "<path class=\"{ac}\" d=\"M{x1} {y}l{} -3v6z\"/>", 7 * dir);
        }
        if !e.label.is_empty() {
            let _ = write!(
                f.body,
                "<text class=\"el mo{lm}\" x=\"{}\" y=\"{}\">{}</text>",
                idiv(x1 + x2, 2),
                y - 7,
                esc(&e.label)
            );
        }
        f.body.push_str("</g>");
    }
    for &i in &d.roots {
        let it = &d.items[i];
        let x = ox + l.cx[i] - idiv(l.pw[i], 2);
        open_part(&mut f.body, "node", &it.id, it.status);
        f.body.push_str(&card(it.kind, it.status, x, oy, l.pw[i], l.head_h));
        f.body.push_str(&card_text(&it.label, &it.sub, it.status, ox + l.cx[i], oy + idiv(l.head_h, 2)));
        if it.status > 0 {
            f.badge(it.status, x + l.pw[i] - 2, oy + 2);
        }
        f.body.push_str("</g>");
    }
    f.finish(l.w, l.h)
}
