//! Record nodes of `erd` and `class`: a header and rows, and the marks at
//! the ends of their edges. The layout asks for the size of a record. The
//! SVG writer asks for the record and the edge marks.

use std::fmt::Write;

use crate::model::{idiv, Diagram, DiagramType, Edge, Item, Member, CLASS_ARROWS};
use crate::svg::{esc, quiet, route_path, st};
use crate::text::{measure, measure_mono};

/// Straight run before the box at the end where an edge arrives. The
/// biggest mark needs 24 pixels.
pub const RUN: i64 = 28;
/// Extra straight run after the box where an edge starts, for the mark there.
pub const LEAD: i64 = 20;

const ROW: i64 = 20;
const SECTION_PAD: i64 = 4;
const SIDE: i64 = 10;
const GAP: i64 = 4; // clear space between a line end and the box

/// The style of records and marks, light and dark. The SVG writer adds it
/// only to a diagram with records, so the other diagrams keep their bytes.
pub const LIGHT: &str = concat!(
    // The header is a shade, so the status tint of the card shows through.
    ".rh{fill:#192029;fill-opacity:.05}",
    ".rl{fill:none;stroke:#c3c8ce}",
    ".rn{font-size:12px}",
    ".rkb{font-weight:600}",
    ".rk{font-size:8.5px;font-weight:700;fill:#1f6feb}",
    ".ty{font-size:10.5px;fill:#6f757c;text-anchor:end}",
    ".rm{font-size:10.5px}",
    ".rs{font-size:9.5px;fill:#6f757c;text-anchor:middle}",
    ".it{font-style:italic}",
    ".mk{fill:#fff}",
    ".rw{fill:none}",
    ".rw.s1,.rw.s4{fill:#1f8a4c;fill-opacity:.1}",
    ".rw.s2{fill:#6f52e0;fill-opacity:.1}",
    ".rw.s5{fill:#b06d08;fill-opacity:.1}",
    ".rw.s6{fill:#cc3340;fill-opacity:.1}",
);
/// A hollow class mark has the paper colour, so the line does not show
/// through. Only a class diagram gets these rules.
pub const CLASS_LIGHT: &str = ".e.mk{fill:#f8fafd}";
pub const CLASS_DARK: &str = ".e.mk{fill:#0e1217}";
pub const DARK: &str = concat!(
    ".rh{fill:#fff;fill-opacity:.05}",
    ".rl{stroke:#3c434c}",
    ".rk{fill:#569fff}",
    ".ty,.rs{fill:#8a9098}",
    ".mk{fill:#161b21}",
    ".rw.s1,.rw.s4{fill:#4cc27a;fill-opacity:.16}",
    ".rw.s2{fill:#a48cff;fill-opacity:.16}",
    ".rw.s5{fill:#dba13a;fill-opacity:.16}",
    ".rw.s6{fill:#f0626c;fill-opacity:.16}",
);

fn has_flag(m: &Member, flag: &str) -> bool {
    m.words.iter().skip(2).any(|w| w == flag)
}

/// The key mark of an `erd` field, or an empty text.
fn key_mark(m: &Member) -> String {
    let marks: Vec<&str> = [("pk", "PK"), ("fk", "FK"), ("uniq", "UQ")]
        .iter()
        .filter(|(f, _)| has_flag(m, f))
        .map(|&(_, mark)| mark)
        .collect();
    marks.join(" ")
}

fn is_key(m: &Member) -> bool {
    has_flag(m, "pk") || has_flag(m, "fk")
}

/// The type of an `erd` field. A field that can be null gets a `?`.
fn field_type(m: &Member) -> String {
    let mut t = m.words.get(1).cloned().unwrap_or_default();
    if has_flag(m, "null") {
        t.push('?');
    }
    t
}

fn is_method(m: &Member) -> bool {
    m.text.contains('(')
}

/// The text above the name of a class: the kind, if it is not `class`.
fn stereotype(dt: DiagramType, it: &Item) -> Option<String> {
    (dt == DiagramType::Class && it.kind > 0).then(|| format!("«{}»", dt.kinds()[it.kind]))
}

fn header_h(dt: DiagramType, it: &Item) -> i64 {
    if stereotype(dt, it).is_some() {
        46
    } else {
        34
    }
}

/// The row counts of the sections under the header, in draw order.
fn sections(dt: DiagramType, it: &Item) -> Vec<Vec<&Member>> {
    if dt == DiagramType::Erd {
        return if it.members.is_empty() { Vec::new() } else { vec![it.members.iter().collect()] };
    }
    let fields: Vec<&Member> = it.members.iter().filter(|m| !is_method(m)).collect();
    let methods: Vec<&Member> = it.members.iter().filter(|m| is_method(m)).collect();
    [fields, methods].into_iter().filter(|s| !s.is_empty()).collect()
}

fn mark_w(it: &Item) -> i64 {
    it.members.iter().map(|m| measure_mono(&key_mark(m), 85)).max().unwrap_or(0)
}

/// Width and height of a record.
pub fn record_size(dt: DiagramType, it: &Item) -> (i64, i64) {
    let mut w = measure(&it.label, 13, true) + 48;
    if let Some(s) = stereotype(dt, it) {
        w = w.max(measure_mono(&s, 95) + 2 * SIDE);
    }
    let mw = mark_w(it);
    for m in &it.members {
        let row = if dt == DiagramType::Erd {
            let name = m.words.first().map_or("", String::as_str);
            let lead = if mw > 0 { mw + 6 } else { 0 };
            lead + measure(name, 12, is_key(m)) + 24 + measure_mono(&field_type(m), 105)
        } else {
            measure_mono(&m.text, 105)
        };
        w = w.max(row + 2 * SIDE);
    }
    w = w.max(120);
    w += w & 1;
    let mut h = header_h(dt, it);
    for s in sections(dt, it) {
        h += 2 * SECTION_PAD + ROW * s.len() as i64;
    }
    (w, h)
}

/// Draws a record. Returns the statuses of the rows, for the legend.
pub fn draw_record(out: &mut String, dt: DiagramType, it: &Item, x: i64, y: i64, w: i64, h: i64) -> Vec<usize> {
    let mut used = Vec::new();
    let hh = header_h(dt, it);
    let _ = write!(
        out,
        "<rect class=\"sh\" x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{h}\" rx=\"6\"/><rect class=\"cd{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"6\"/>",
        y + 1,
        st(it.status)
    );
    // A record with no rows is a plain card, so it has no header shade.
    if hh < h {
        // The header keeps the round corners of the box at the top only.
        let _ = write!(
            out,
            "<path class=\"rh\" d=\"M{} {}V{}Q{} {} {} {}H{}Q{} {} {} {}V{}z\"/>",
            x + 1,
            y + hh,
            y + 6,
            x + 1,
            y + 1,
            x + 6,
            y + 1,
            x + w - 6,
            x + w - 1,
            y + 1,
            x + w - 1,
            y + 6,
            y + hh
        );
    }
    let cx = x + idiv(w, 2);
    let p = quiet(it.status);
    let mut ny = y + 22;
    if let Some(s) = stereotype(dt, it) {
        let _ = write!(out, "<text class=\"rs mo\" x=\"{cx}\" y=\"{}\">{}</text>", y + 18, esc(&s));
        ny = y + 34;
    }
    let italic = if dt == DiagramType::Class && dt.kinds()[it.kind] == "abstract" { " it" } else { "" };
    let _ = write!(out, "<text class=\"nm{italic}{p}\" x=\"{cx}\" y=\"{ny}\">{}</text>", esc(&it.label));

    let mw = mark_w(it);
    let mut sy = y + hh;
    for rows in sections(dt, it) {
        let _ = write!(out, "<path class=\"rl\" d=\"M{x} {sy}h{w}\"/>");
        let mut ry = sy + SECTION_PAD;
        for m in rows {
            if m.status > 0 {
                used.push(m.status);
                let _ = write!(
                    out,
                    "<rect class=\"rw{}\" x=\"{}\" y=\"{ry}\" width=\"{}\" height=\"{ROW}\"/>",
                    st(m.status),
                    x + 1,
                    w - 2
                );
                if m.status != 3 {
                    let _ = write!(
                        out,
                        "<rect class=\"b{}\" x=\"{}\" y=\"{ry}\" width=\"3\" height=\"{ROW}\"/>",
                        st(m.status),
                        x + 1
                    );
                }
            }
            let q = quiet(m.status);
            let ty = ry + 14;
            if dt == DiagramType::Erd {
                let mut tx = x + SIDE;
                if mw > 0 {
                    let mark = key_mark(m);
                    if !mark.is_empty() {
                        let _ = write!(out, "<text class=\"rk mo\" x=\"{tx}\" y=\"{}\">{mark}</text>", ty - 1);
                    }
                    tx += mw + 6;
                }
                let bold = if is_key(m) { " rkb" } else { "" };
                let name = m.words.first().map_or("", String::as_str);
                let _ = write!(out, "<text class=\"rn{bold}{q}\" x=\"{tx}\" y=\"{ty}\">{}</text>", esc(name));
                let _ = write!(
                    out,
                    "<text class=\"ty mo{q}\" x=\"{}\" y=\"{ty}\">{}</text>",
                    x + w - SIDE,
                    esc(&field_type(m))
                );
            } else {
                let _ = write!(out, "<text class=\"rm mo{q}\" x=\"{}\" y=\"{ty}\">{}</text>", x + SIDE, esc(&m.text));
            }
            ry += ROW;
        }
        sy = ry + SECTION_PAD;
    }
    used
}

/// A point on the line at distance `t` from the box, and `s` to the side.
/// (x, y) is on the box and (dx, dy) points into the box.
fn at(x: i64, y: i64, dx: i64, dy: i64, t: i64, s: i64) -> (i64, i64) {
    (x - t * dx - s * dy, y - t * dy + s * dx)
}

/// The SVG of one end mark, and the distance from the box where the line
/// must stop.
fn end_mark(mark: &str, x: i64, y: i64, dx: i64, dy: i64, line: &str, fill: &str) -> (String, i64) {
    let p = |t: i64, s: i64| at(x, y, dx, dy, t, s);
    let bar = |t: i64| {
        let (a, b) = (p(t, -6), p(t, 6));
        format!("<path class=\"{line}\" d=\"M{} {}L{} {}\"/>", a.0, a.1, b.0, b.1)
    };
    let ring = |t: i64| {
        let c = p(t, 0);
        format!("<circle class=\"{line} mk\" cx=\"{}\" cy=\"{}\" r=\"4\"/>", c.0, c.1)
    };
    let foot = || {
        let (a, b, c) = (p(GAP, -6), p(15, 0), p(GAP, 6));
        format!("<path class=\"{line}\" d=\"M{} {}L{} {}L{} {}\"/>", a.0, a.1, b.0, b.1, c.0, c.1)
    };
    let shape = |pts: &[(i64, i64)], class: &str| {
        let mut d = String::new();
        for (k, q) in pts.iter().enumerate() {
            let _ = write!(d, "{}{} {}", if k == 0 { "M" } else { "L" }, q.0, q.1);
        }
        format!("<path class=\"{class}\" d=\"{d}z\"/>")
    };
    let diamond = [p(GAP, 0), p(12, -6), p(20, 0), p(12, 6)];
    let triangle = [p(GAP, 0), p(16, -7), p(16, 7)];
    match mark {
        "1" => (bar(10) + &bar(14), GAP),
        "?" => (bar(10) + &ring(19), GAP),
        "*" => (foot() + &ring(20), GAP),
        "+" => (foot() + &bar(18), GAP),
        "triangle" => (shape(&triangle, &format!("{line} mk")), 16),
        "diamond" => (shape(&diamond, fill), 20),
        "hollow" => (shape(&diamond, &format!("{line} mk")), 20),
        "arrow" => (shape(&[p(GAP, 0), p(11, -3), p(11, 3)], fill), 9),
        "open" => {
            let (a, b, c) = (p(13, -5), p(GAP, 0), p(13, 5));
            (format!("<path class=\"{line}\" d=\"M{} {}L{} {}L{} {}\"/>", a.0, a.1, b.0, b.1, c.0, c.1), GAP)
        }
        _ => (String::new(), GAP),
    }
}

/// The marks at the `from` end and at the `to` end of an edge.
fn marks(dt: DiagramType, e: &Edge) -> (&str, &str) {
    if dt == DiagramType::Erd {
        // An erd arrow is `<from>--<to>`, and each end is one character.
        return e.arrow.split_once("--").unwrap_or(("", ""));
    }
    match e.arrow {
        "--|>" | "..|>" => ("", "triangle"),
        "*--" => ("diamond", ""),
        "o--" => ("hollow", ""),
        "->" => ("", "arrow"),
        "..>" => ("", "open"),
        _ => ("", ""),
    }
}

/// Moves the end of a route `cut` pixels closer to its previous point.
fn shorten(p: &mut [i64], end: bool, cut: i64) {
    let z = p.len();
    let (i, j) = if end { (z - 2, z - 4) } else { (0, 2) };
    for k in 0..2 {
        let d = p[i + k] - p[j + k];
        let step = cut.min(d.abs());
        p[i + k] -= step * d.signum();
    }
}

/// Draws the line of an edge between records, and its end marks. `p` holds
/// absolute points. `to_first` is true when the route starts at `e.to`.
pub fn draw_edge(out: &mut String, dt: DiagramType, e: &Edge, p: &[i64], to_first: bool) {
    draw_line(out, dt, e, e.status, p, to_first);
}

fn draw_line(out: &mut String, dt: DiagramType, e: &Edge, status: usize, p: &[i64], to_first: bool) {
    let z = p.len();
    let s = st(status);
    let line = format!("e{s}");
    let fill = format!("a{s}");
    let (from_mark, to_mark) = marks(dt, e);
    let (start_mark, end_mark_name) = if to_first { (to_mark, from_mark) } else { (from_mark, to_mark) };
    let (sx, sy, sdx, sdy) = (p[0], p[1], (p[0] - p[2]).signum(), (p[1] - p[3]).signum());
    let (ex, ey, edx, edy) = (p[z - 2], p[z - 1], (p[z - 2] - p[z - 4]).signum(), (p[z - 1] - p[z - 3]).signum());
    let (start_svg, start_cut) = end_mark(start_mark, sx, sy, sdx, sdy, &line, &fill);
    let (end_svg, end_cut) = end_mark(end_mark_name, ex, ey, edx, edy, &line, &fill);
    let mut pts = p.to_vec();
    // `route_path` takes away `GAP` at each end.
    shorten(&mut pts, false, start_cut - GAP);
    shorten(&mut pts, true, end_cut - GAP);
    let dash = if e.dashed { " d" } else { "" };
    let _ = write!(out, "<path class=\"e{dash}{s} rt\" d=\"{}\"/>", route_path(&pts, false, false));
    out.push_str(&start_svg);
    out.push_str(&end_svg);
}

/// The legend of the class relations, in the order of `CLASS_ARROWS`. Each
/// entry is a sample line from (0, 0) to (34, 0), and its name. The legend
/// shows only when the diagram uses two or more relations.
pub fn class_legend(d: &Diagram) -> Vec<(String, &'static str)> {
    const NAMES: [&str; 6] = ["extends", "implements", "made of", "has", "uses", "depends on"];
    if d.dtype != DiagramType::Class {
        return Vec::new();
    }
    let used: Vec<usize> =
        (0..CLASS_ARROWS.len()).filter(|&k| d.edges.iter().any(|e| e.arrow == CLASS_ARROWS[k])).collect();
    if used.len() < 2 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for k in used {
        let e = d.edges.iter().find(|e| e.arrow == CLASS_ARROWS[k]).expect("the arrow is in use");
        let mut svg = String::new();
        draw_line(&mut svg, d.dtype, e, 0, &[0, 0, 34, 0], false);
        out.push((svg, NAMES[k]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_graph::layout_graph;
    use crate::parse::parse;

    #[test]
    fn a_record_grows_with_its_rows() {
        let d = parse("erd\na\nb\n  id uuid pk\n  name text\n").unwrap();
        let (_, h0) = record_size(d.dtype, &d.items[0]);
        let (_, h2) = record_size(d.dtype, &d.items[1]);
        assert_eq!(h0, 34);
        assert_eq!(h2, 34 + 2 * SECTION_PAD + 2 * ROW);
    }

    #[test]
    fn a_class_has_a_section_for_fields_and_one_for_methods() {
        let d = parse("class\na :interface\n  - x i32\n  + f()\n  + g()\n").unwrap();
        let (_, h) = record_size(d.dtype, &d.items[0]);
        assert_eq!(h, 46 + 4 * SECTION_PAD + 3 * ROW);
    }

    #[test]
    fn a_field_shows_its_key_marks_and_null() {
        let d = parse("erd\na\n  id uuid pk fk\n  note text null\n").unwrap();
        let m = &d.items[0].members;
        assert_eq!(key_mark(&m[0]), "PK FK");
        assert_eq!(field_type(&m[1]), "text?");
        assert!(is_key(&m[0]) && !is_key(&m[1]));
    }

    #[test]
    fn each_arrow_has_its_end_marks() {
        let d = parse("erd\na ?--+ b\n").unwrap();
        assert_eq!(marks(d.dtype, &d.edges[0]), ("?", "+"));
        let d = parse("class\na --|> b\nc *-- a\nc o-- b\na ..> c\n").unwrap();
        let got: Vec<(&str, &str)> = d.edges.iter().map(|e| marks(d.dtype, e)).collect();
        assert_eq!(got, [("", "triangle"), ("diamond", ""), ("hollow", ""), ("", "open")]);
    }

    #[test]
    fn the_class_legend_needs_two_relations() {
        let one = parse("class\na --|> b\nc --|> b\n").unwrap();
        assert!(class_legend(&one).is_empty());
        let two = parse("class\na o-- b\nc --|> b\n").unwrap();
        let names: Vec<&str> = class_legend(&two).iter().map(|g| g.1).collect();
        assert_eq!(names, ["extends", "has"]);
        assert!(class_legend(&two)[1].0.contains("e mk"));
    }

    #[test]
    fn a_parent_class_is_above_its_children() {
        // The lines name the child first, so a plain layering puts it on top.
        let d = parse("class\nchild --|> parent\nimpl ..|> api\nimpl -> parent\n").unwrap();
        let l = layout_graph(&d);
        let y = |id: &str| l.y[d.items.iter().position(|it| it.id == id).unwrap()];
        assert!(y("parent") < y("child"));
        assert!(y("api") < y("impl"));
    }
}
