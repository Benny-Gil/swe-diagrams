//! Motion along the stories of a diagram. The routes come from the `rt`
//! paths of the SVG, so each diagram type works the same way. All times
//! are whole milliseconds, so the output stays deterministic.

use std::fmt::Write;

use crate::raster::rasterize;
use crate::render::{node_spans, Rendered, Variant};
use crate::story::{Mode, Story};
use crate::svg::{esc, BADGES};
use crate::theme::{apply_theme, Theme};
use crate::tiny_skia::Pixmap;

/// Pixels each second of a token.
pub const SPEED_PX: i64 = 120;
pub const MIN_MS: i64 = 400;
pub const MAX_MS: i64 = 1600;
/// The time of a progress beat.
pub const PROGRESS_MS: i64 = 800;
/// The pause at the end of a story, before the loop starts again.
pub const PAUSE_MS: i64 = 1000;

/// The tokens, the bright edge, and the caption. The reduced-motion rule
/// hides the motion group and shows the still parts.
const STYLE: (&str, &str) = (
    ".sd-hl{fill:none;stroke:#1f6feb;stroke-width:2.6;stroke-linecap:round}.tkf{fill:#1f6feb;stroke:#f8fafd;stroke-width:2}.tkh{fill:#f8fafd;stroke:#1f6feb;stroke-width:2.2}.tc{font-size:10.5px;fill:#1f6feb;text-anchor:middle;paint-order:stroke;stroke:#f8fafd;stroke-width:4px;stroke-linejoin:round}",
    ".sd-hl{stroke:#569fff}.tkf{fill:#569fff;stroke:#0e1217}.tkh{fill:#0e1217;stroke:#569fff}.tc{fill:#569fff;stroke:#0e1217}",
);
const MOTION_RULES: &str =
    "@media(prefers-reduced-motion:reduce){.sd-a{display:none}}@media(prefers-reduced-motion:no-preference){.sd-st{display:none}}";

/// The path of an edge, as a line through points.
#[derive(Clone, Debug)]
pub struct Route {
    /// The `d` attribute of the SVG path.
    pub d: String,
    pts: Vec<(f64, f64)>,
    /// The length of the path up to each point.
    cum: Vec<f64>,
}

impl Route {
    pub fn from_d(d: &str) -> Option<Route> {
        let pts = flatten(d)?;
        let mut cum = vec![0.0];
        for w in pts.windows(2) {
            let last = cum[cum.len() - 1];
            cum.push(last + (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1));
        }
        Some(Route { d: d.to_string(), pts, cum })
    }

    pub fn len(&self) -> f64 {
        self.cum[self.cum.len() - 1]
    }

    pub fn is_empty(&self) -> bool {
        self.len() <= 0.0
    }

    /// The point at the fraction `u` of the length.
    pub fn at(&self, u: f64) -> (f64, f64) {
        let target = u.clamp(0.0, 1.0) * self.len();
        let k = self.cum.partition_point(|&c| c < target).clamp(1, self.pts.len() - 1);
        let (a, b) = (self.pts[k - 1], self.pts[k]);
        let seg = self.cum[k] - self.cum[k - 1];
        let f = if seg > 0.0 { (target - self.cum[k - 1]) / seg } else { 0.0 };
        (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
    }
}

/// The points of a path with the absolute commands M L H V Q C. A curve
/// becomes 12 lines.
fn flatten(d: &str) -> Option<Vec<(f64, f64)>> {
    let b = d.as_bytes();
    let mut i = 0;
    let nums = |i: &mut usize, n: usize| -> Option<Vec<f64>> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            while *i < b.len() && (b[*i] == b' ' || b[*i] == b',') {
                *i += 1;
            }
            let start = *i;
            if *i < b.len() && (b[*i] == b'-' || b[*i] == b'+') {
                *i += 1;
            }
            let mut dot = false;
            while *i < b.len() && (b[*i].is_ascii_digit() || (b[*i] == b'.' && !dot)) {
                dot |= b[*i] == b'.';
                *i += 1;
            }
            out.push(d[start..*i].parse::<f64>().ok()?);
        }
        Some(out)
    };
    let mut pts: Vec<(f64, f64)> = Vec::new();
    let mut cur = (0.0, 0.0);
    while i < b.len() {
        let c = b[i];
        i += 1;
        match c {
            b'M' | b'L' => {
                let v = nums(&mut i, 2)?;
                cur = (v[0], v[1]);
                pts.push(cur);
            }
            b'H' => {
                cur.0 = nums(&mut i, 1)?[0];
                pts.push(cur);
            }
            b'V' => {
                cur.1 = nums(&mut i, 1)?[0];
                pts.push(cur);
            }
            b'Q' | b'C' => {
                let v = nums(&mut i, if c == b'Q' { 4 } else { 6 })?;
                let p0 = cur;
                for s in 1..=12 {
                    let t = f64::from(s) / 12.0;
                    let m = 1.0 - t;
                    cur = if c == b'Q' {
                        (
                            m * m * p0.0 + 2.0 * m * t * v[0] + t * t * v[2],
                            m * m * p0.1 + 2.0 * m * t * v[1] + t * t * v[3],
                        )
                    } else {
                        (
                            m * m * m * p0.0 + 3.0 * m * m * t * v[0] + 3.0 * m * t * t * v[2] + t * t * t * v[4],
                            m * m * m * p0.1 + 3.0 * m * m * t * v[1] + 3.0 * m * t * t * v[3] + t * t * t * v[5],
                        )
                    };
                    pts.push(cur);
                }
            }
            b' ' => {}
            _ => return None,
        }
    }
    (pts.len() >= 2).then_some(pts)
}

/// The route of each edge index. The first `rt` path in the group of an
/// edge or a message is the route.
pub fn routes(svg: &str) -> Vec<Option<Route>> {
    let mut out: Vec<Option<Route>> = Vec::new();
    let mut rest = svg;
    while let Some(at) = rest.find("<g data-k=\"") {
        rest = &rest[at + 11..];
        let kind = rest.split('"').next().unwrap_or("");
        if kind != "edge" && kind != "msg" {
            continue;
        }
        let Some(k) = rest.split("data-i=\"").nth(1).and_then(|t| t.split('"').next()?.parse::<usize>().ok()) else {
            continue;
        };
        let end = rest.find("<g data-k=\"").unwrap_or(rest.len());
        let body = &rest[..end];
        let Some(p) = body.find(" rt\" d=\"") else { continue };
        let d = body[p + 8..].split('"').next().unwrap_or("");
        if out.len() <= k {
            out.resize(k + 1, None);
        }
        if out[k].is_none() {
            out[k] = Route::from_d(d);
        }
    }
    out
}

/// The time of a step along a path of `len` pixels, before the speed.
pub fn step_ms(len: f64) -> i64 {
    ((len * 1000.0 / SPEED_PX as f64).round() as i64).clamp(MIN_MS, MAX_MS)
}

/// A time at the speed `speed`: 2 is two times faster.
pub fn at_speed(ms: i64, speed: f64) -> i64 {
    ((ms as f64 / speed).round() as i64).max(1)
}

/// The times of a story.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Timing {
    /// The start and the time of each beat.
    pub beats: Vec<(i64, i64)>,
    /// The time of each move of each beat. It is the time of the beat, or
    /// of the loop for a stream.
    pub moves: Vec<Vec<i64>>,
    /// The time of one loop.
    pub total: i64,
}

pub fn timing(story: &Story, routes: &[Option<Route>], speed: f64) -> Timing {
    let mut t = 0;
    let mut beats = Vec::new();
    let mut moves = Vec::new();
    for b in &story.beats {
        let ms: Vec<i64> = b
            .moves
            .iter()
            .map(|m| at_speed(step_ms(routes.get(m.edge).and_then(Option::as_ref).map_or(0.0, Route::len)), speed))
            .collect();
        let mut dur = ms.iter().copied().max().unwrap_or(0);
        if b.node.is_some() {
            dur = dur.max(at_speed(PROGRESS_MS, speed));
        }
        beats.push((t, dur));
        // The steps of one beat arrive together.
        moves.push(vec![dur; ms.len()]);
        t += dur;
    }
    if story.mode == Mode::Stream {
        // All tokens of a stream loop in the same time, so the loop has no seam.
        for m in &mut moves {
            m.fill(t);
        }
    } else {
        t += at_speed(PAUSE_MS, speed);
    }
    Timing { beats, moves, total: t.max(1) }
}

/// A token at one time.
#[derive(Clone, PartialEq, Debug)]
pub struct Token {
    pub edge: usize,
    pub x: f64,
    pub y: f64,
    pub hollow: bool,
    pub caption: String,
}

/// The picture of a story at one time.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Scene {
    pub tokens: Vec<Token>,
    /// The edges of the active beat.
    pub bright: Vec<usize>,
    /// The status of each node of a progress beat.
    pub nodes: Vec<(String, usize)>,
}

pub fn scene(story: &Story, tm: &Timing, routes: &[Option<Route>], t: i64) -> Scene {
    let t = t.rem_euclid(tm.total);
    let mut sc = Scene::default();
    for (k, b) in story.beats.iter().enumerate() {
        let (s, dur) = tm.beats[k];
        if let Some((id, last)) = &b.node {
            let status = if t < s {
                3
            } else if t < s + dur {
                2
            } else {
                *last
            };
            sc.nodes.push((id.clone(), status));
        }
        let active = story.mode == Mode::Stream || (s <= t && t < s + dur);
        if !active {
            continue;
        }
        for (j, m) in b.moves.iter().enumerate() {
            let Some(r) = routes.get(m.edge).and_then(Option::as_ref) else { continue };
            if story.mode != Mode::Stream {
                sc.bright.push(m.edge);
            }
            let md = tm.moves[k][j];
            // A stream has two tokens on each edge, half a loop apart.
            let phases: Vec<i64> = if story.mode == Mode::Stream { vec![t, (t + md / 2) % md] } else { vec![t - s] };
            for p in phases {
                if p >= md {
                    continue;
                }
                let u = p as f64 / md as f64;
                let (x, y) = r.at(if m.back { 1.0 - u } else { u });
                let caption = if story.mode == Mode::Stream { "" } else { m.token_caption() };
                sc.tokens.push(Token { edge: m.edge, x, y, hollow: m.hollow, caption: caption.to_string() });
            }
        }
    }
    sc
}

/// The number of frames of one loop.
pub fn frame_count(total: i64, fps: u32) -> usize {
    ((total * i64::from(fps) + 999) / 1000).max(1) as usize
}

fn token_svg(out: &mut String, hollow: bool, caption: &str) {
    let _ = write!(out, "<circle r=\"5.5\" class=\"{}\"/>", if hollow { "tkh" } else { "tkf" });
    if !caption.is_empty() {
        let _ = write!(out, "<text class=\"tc mo\" y=\"20\">{}</text>", esc(caption));
    }
}

/// Adds the style rules, before the end of the first style element.
fn with_style(svg: &str, extra: &str) -> String {
    let rules = format!("{}@media(prefers-color-scheme:dark){{{}}}{extra}", STYLE.0, STYLE.1);
    match svg.find("</style>") {
        Some(at) => format!("{}{rules}{}", &svg[..at], &svg[at..]),
        None => svg.to_string(),
    }
}

/// Adds the badges of the statuses 1 to 3 to the defs, for the variants.
fn with_badges(svg: &str) -> String {
    let mut add = String::new();
    for (s, badge) in BADGES.iter().enumerate().take(4).skip(1) {
        if !svg.contains(&format!("<g id=\"b{s}\">")) {
            let _ = write!(add, "<g id=\"b{s}\">{badge}</g>");
        }
    }
    match svg.find("<defs>") {
        Some(at) => format!("{}{add}{}", &svg[..at + 6], &svg[at + 6..]),
        None => svg.to_string(),
    }
}

/// Replaces the node groups of each id with `f(variant)`, at the place of
/// the last group of the id.
fn swap_nodes(svg: &str, variants: &[Variant], ids: &[&str], f: impl Fn(&Variant) -> String) -> String {
    let mut cuts: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for id in ids {
        let Some(v) = variants.iter().find(|v| v.id == *id) else { continue };
        let spans = node_spans(svg, id);
        let n = spans.len();
        for (k, sp) in spans.into_iter().enumerate() {
            cuts.push((sp.clone(), if k + 1 == n { f(v) } else { String::new() }));
        }
    }
    cuts.sort_by_key(|c| c.0.start);
    let mut out = String::with_capacity(svg.len() * 2);
    let mut at = 0;
    for (r, text) in cuts {
        out.push_str(&svg[at..r.start]);
        out.push_str(&text);
        at = r.end;
    }
    out.push_str(&svg[at..]);
    out
}

fn before_end(svg: &str, text: &str) -> String {
    match svg.rfind("</svg>") {
        Some(at) => format!("{}{text}{}", &svg[..at], &svg[at..]),
        None => svg.to_string(),
    }
}

/// The done count of the kicker when the shown statuses are `shown`, or
/// `None` when the kicker has no count.
fn kicker_count(svg: &str, variants: &[Variant], shown: &[(String, usize)]) -> Option<(std::ops::Range<usize>, i64)> {
    let open = svg.find("<text class=\"kk mo\"")?;
    let start = open + svg[open..].find('>')? + 1;
    let end = start + svg[start..].find("</text>")?;
    let text = &svg[start..end];
    let slash = text.find(" / ")? + 3;
    let num = slash + text[slash..].find(" of ")?;
    let real: i64 = text[slash..num].parse().ok()?;
    let mut n = real;
    for (id, s) in shown {
        let Some(v) = variants.iter().find(|v| v.id == *id) else { continue };
        n += i64::from(*s == 1) - i64::from(v.status == 1);
    }
    Some((start + slash..start + num, n))
}

/// A still SVG of the scene: the bright edges, the tokens, and the node
/// statuses of a progress story.
pub fn frame_svg(svg: &str, variants: &[Variant], routes: &[Option<Route>], sc: &Scene) -> String {
    let mut out = with_style(svg, "");
    if !sc.nodes.is_empty() {
        // The done count follows the badges of the frame.
        if let Some((r, n)) = kicker_count(&out, variants, &sc.nodes) {
            out.replace_range(r, &n.to_string());
        }
        out = with_badges(&out);
        let ids: Vec<&str> = sc.nodes.iter().map(|n| n.0.as_str()).collect();
        out = swap_nodes(&out, variants, &ids, |v| {
            let s = sc.nodes.iter().find(|n| n.0 == v.id).map_or(3, |n| n.1);
            v.at(s).to_string()
        });
    }
    let mut over = String::new();
    for &e in &sc.bright {
        if let Some(r) = routes.get(e).and_then(Option::as_ref) {
            let _ = write!(over, "<path class=\"sd-hl\" d=\"{}\"/>", r.d);
        }
    }
    for tk in &sc.tokens {
        let _ = write!(over, "<g transform=\"translate({:.1} {:.1})\">", tk.x, tk.y);
        token_svg(&mut over, tk.hollow, &tk.caption);
        over.push_str("</g>");
    }
    before_end(&out, &over)
}

/// The fraction `ms / total` as a short decimal.
fn frac(ms: i64, total: i64) -> String {
    let v = (ms.clamp(0, total) * 10000 + total / 2) / total;
    if v == 10000 {
        return "1".to_string();
    }
    let s = format!("0.{v:04}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Key times with the values at them. A key at the time of the key before
/// it takes its place.
fn keys(list: &[(i64, &str)], total: i64) -> (String, String) {
    let mut ks: Vec<(String, &str)> = Vec::new();
    for &(t, v) in list {
        let f = frac(t, total);
        match ks.last_mut() {
            Some(last) if last.0 == f => last.1 = v,
            _ => ks.push((f, v)),
        }
    }
    let times: Vec<&str> = ks.iter().map(|k| k.0.as_str()).collect();
    let vals: Vec<&str> = ks.iter().map(|k| k.1).collect();
    (vals.join(";"), times.join(";"))
}

/// An opacity animation: 1 from `s` to `e`, else 0.
fn show_between(s: i64, e: i64, total: i64) -> String {
    let mut list = vec![(0, "0"), (s, "1")];
    if e < total {
        list.push((e, "0"));
    }
    let (values, times) = keys(&list, total);
    format!("<animate attributeName=\"opacity\" values=\"{values}\" keyTimes=\"{times}\" dur=\"{total}ms\" calcMode=\"discrete\" repeatCount=\"indefinite\"/>")
}

/// Puts a copy of the kicker for each done count, each shown while the
/// count holds. The still kicker shows without motion.
fn animate_kicker(svg: &str, variants: &[Variant], story: &Story, tm: &Timing) -> String {
    let total = tm.total;
    // The count changes at the end of each beat of a task that goes to done.
    let mut times = vec![0];
    for (b, &(s, d)) in story.beats.iter().zip(&tm.beats) {
        if b.node.as_ref().is_some_and(|n| n.1 == 1) {
            times.push(s + d);
        }
    }
    let Some(open) = svg.find("<text class=\"kk mo\"") else { return svg.to_string() };
    let Some(close) = svg[open..].find("</text>").map(|e| open + e + 7) else { return svg.to_string() };
    let tag = &svg[open..close];
    let mut g = format!("<g class=\"sd-st\">{tag}</g><g class=\"sd-a\">");
    for (k, &t0) in times.iter().enumerate() {
        let t1 = times.get(k + 1).copied().unwrap_or(total);
        let sc = scene(story, tm, &[], t0);
        let Some((r, n)) = kicker_count(tag, variants, &sc.nodes) else { return svg.to_string() };
        let mut text = tag.to_string();
        text.replace_range(r, &n.to_string());
        let _ = write!(g, "<g opacity=\"0\">{}{text}</g>", show_between(t0, t1, total));
    }
    g.push_str("</g>");
    format!("{}{g}{}", &svg[..open], &svg[close..])
}

/// An SVG that plays the story without a script. Without motion, as with
/// `prefers-reduced-motion`, it is the still diagram.
pub fn animate_svg(svg: &str, variants: &[Variant], routes: &[Option<Route>], story: &Story, tm: &Timing) -> String {
    let total = tm.total;
    let mut out = with_style(svg, MOTION_RULES);
    if story.mode == Mode::Progress {
        out = with_badges(&out);
        let nodes: Vec<(&str, i64, i64, usize)> = story
            .beats
            .iter()
            .zip(&tm.beats)
            .filter_map(|(b, &(s, d))| b.node.as_ref().map(|(id, last)| (id.as_str(), s, s + d, *last)))
            .collect();
        out = animate_kicker(&out, variants, story, tm);
        let ids: Vec<&str> = nodes.iter().map(|n| n.0).collect();
        let original = out.clone();
        out = swap_nodes(&out, variants, &ids, |v| {
            let &(_, s, e, last) = nodes.iter().find(|n| n.0 == v.id).unwrap_or(&("", 0, 0, 1));
            let still: String = node_spans(&original, &v.id).into_iter().map(|r| &original[r]).collect();
            let mut g = format!("<g class=\"sd-st\">{still}</g><g class=\"sd-a\">");
            let phases =
                if last == 2 { vec![(0, s, 3), (s, total, 2)] } else { vec![(0, s, 3), (s, e, 2), (e, total, 1)] };
            for (a, b, status) in phases {
                let _ = write!(g, "<g opacity=\"0\">{}{}</g>", show_between(a, b, total), v.at(status));
            }
            g.push_str("</g>");
            g
        });
        return out;
    }
    let mut over = String::from("<g class=\"sd-a\">");
    for (k, b) in story.beats.iter().enumerate() {
        let (s, dur) = tm.beats[k];
        for (j, m) in b.moves.iter().enumerate() {
            let Some(r) = routes.get(m.edge).and_then(Option::as_ref) else { continue };
            let md = tm.moves[k][j];
            if story.mode == Mode::Stream {
                for half in [0, md / 2] {
                    let _ = write!(
                        over,
                        "<g><animateMotion path=\"{}\" dur=\"{md}ms\" begin=\"-{half}ms\" repeatCount=\"indefinite\"/>",
                        r.d
                    );
                    token_svg(&mut over, m.hollow, "");
                    over.push_str("</g>");
                }
                continue;
            }
            let _ = write!(
                over,
                "<path class=\"sd-hl\" d=\"{}\" opacity=\"0\">{}</path>",
                r.d,
                show_between(s, s + dur, total)
            );
            let (a, z) = if m.back { ("1", "0") } else { ("0", "1") };
            let (points, times) = keys(&[(0, a), (s, a), (s + md, z), (total, z)], total);
            let _ = write!(
                over,
                "<g opacity=\"0\">{}<animateMotion path=\"{}\" keyPoints=\"{points}\" keyTimes=\"{times}\" calcMode=\"linear\" dur=\"{total}ms\" repeatCount=\"indefinite\"/>",
                show_between(s, s + md, total),
                r.d
            );
            token_svg(&mut over, m.hollow, m.token_caption());
            over.push_str("</g>");
        }
    }
    over.push_str("</g>");
    before_end(&out, &over)
}

/// The options of the frames of a GIF or a video.
#[derive(Clone, Copy, Debug)]
pub struct FrameOptions {
    pub fps: u32,
    pub speed: f64,
    pub scale: f32,
    pub theme: Theme,
}

/// The pixels of each frame of one loop of `story`. With no story, one
/// frame of the still diagram.
pub fn frames(r: &Rendered, story: Option<&Story>, o: &FrameOptions) -> Result<Vec<Pixmap>, String> {
    let Some(story) = story else {
        return Ok(vec![rasterize(&apply_theme(&r.svg, o.theme), o.scale)?]);
    };
    let routes = routes(&r.svg);
    let tm = timing(story, &routes, o.speed);
    let n = frame_count(tm.total, o.fps);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as i64 * 1000 / i64::from(o.fps);
        let sc = scene(story, &tm, &routes, t);
        out.push(rasterize(&apply_theme(&frame_svg(&r.svg, &r.variants, &routes, &sc), o.theme), o.scale)?);
    }
    Ok(out)
}

/// The story with the title, or the first story.
pub fn pick<'a>(r: &'a Rendered, title: Option<&str>) -> Result<Option<&'a Story>, String> {
    match title {
        None => Ok(r.stories.first()),
        Some(t) => r.stories.iter().find(|s| s.title == t).map(Some).ok_or_else(|| {
            let names: Vec<String> = r.stories.iter().map(|s| format!("\"{}\"", s.title)).collect();
            if names.is_empty() {
                format!("the diagram has no story \"{t}\"; it has no stories")
            } else {
                format!("the diagram has no story \"{t}\"; the stories are {}", names.join(", "))
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::{Beat, Move};

    #[test]
    fn a_path_has_a_length_and_points() {
        let r = Route::from_d("M0 0H100V50").unwrap();
        assert_eq!(r.len(), 150.0);
        assert_eq!(r.at(0.5), (75.0, 0.0));
        assert_eq!(r.at(1.0), (100.0, 50.0));
        let q = Route::from_d("M0 0L10 0Q16 0 16 6L16 20").unwrap();
        assert!(q.len() > 33.0 && q.len() < 34.0, "{}", q.len());
    }

    #[test]
    fn the_time_follows_the_length() {
        assert_eq!(step_ms(10.0), MIN_MS);
        assert_eq!(step_ms(120.0), 1000);
        assert_eq!(step_ms(131.0), 1092);
        assert_eq!(step_ms(1000.0), MAX_MS);
        assert_eq!(at_speed(1000, 2.0), 500);
        assert_eq!(at_speed(1000, 3.0), 333);
    }

    fn one(edge: usize, back: bool) -> Beat {
        Beat {
            moves: vec![Move { edge, back, hollow: false, caption: String::new(), label: String::new() }],
            node: None,
        }
    }

    #[test]
    fn beats_follow_each_other() {
        let routes = vec![Route::from_d("M0 0H120"), Route::from_d("M0 0V60")];
        let s = Story { title: "s".into(), mode: Mode::Steps, beats: vec![one(0, false), one(1, true)] };
        let tm = timing(&s, &routes, 1.0);
        assert_eq!(tm.beats, [(0, 1000), (1000, 500)]);
        assert_eq!(tm.total, 1500 + PAUSE_MS);
        assert_eq!(frame_count(tm.total, 15), 38);
        let sc = scene(&s, &tm, &routes, 1250);
        assert_eq!(sc.bright, [1]);
        // The second step goes back: from the end of the path to the start.
        assert_eq!((sc.tokens[0].x, sc.tokens[0].y), (0.0, 30.0));
        assert!(scene(&s, &tm, &routes, 1600).tokens.is_empty());
    }

    #[test]
    fn a_stream_has_no_pause_and_two_tokens_on_each_edge() {
        let routes = vec![Route::from_d("M0 0H240")];
        let s = Story { title: "s".into(), mode: Mode::Stream, beats: vec![one(0, false)] };
        let tm = timing(&s, &routes, 1.0);
        assert_eq!(tm.total, 1600);
        let sc = scene(&s, &tm, &routes, 400);
        let xs: Vec<f64> = sc.tokens.iter().map(|t| t.x).collect();
        assert_eq!(xs, [60.0, 180.0]);
    }

    #[test]
    fn key_times_are_short_and_unique() {
        assert_eq!(frac(250, 1000), "0.25");
        assert_eq!(frac(1000, 1000), "1");
        assert_eq!(frac(0, 1000), "0");
        assert_eq!(keys(&[(0, "0"), (0, "1"), (500, "0")], 1000), ("1;0".into(), "0;0.5".into()));
    }

    #[test]
    fn routes_come_from_the_edge_groups() {
        let svg = "<g data-k=\"edge\" data-i=\"1\" x=\"\"><path class=\"e rt\" d=\"M0 0H10\"/></g><g data-k=\"edge\" data-i=\"1\"><text/></g><g data-k=\"msg\" data-i=\"0\"><path class=\"e d rt\" d=\"M5 5V9\"/></g>";
        let r = routes(svg);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].as_ref().unwrap().d, "M5 5V9");
        assert_eq!(r[1].as_ref().unwrap().len(), 10.0);
    }
}
