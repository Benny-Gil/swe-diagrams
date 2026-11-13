//! Source text in, SVG and a one-line summary out.

use std::path::Path;

use crate::cycles::find_cycles;
use crate::icons::{resolve, Icon, IconOptions};
use crate::layout_graph::layout_graph;
use crate::layout_seq::layout_seq;
use crate::layout_timeline::layout_timeline;
use crate::levels::load;
use crate::model::{Diagram, DiagramError, DiagramType};
use crate::story::{stories, Story};
use crate::svg::{change_counts, render_graph, render_seq};
use crate::svg_timeline::render_timeline;

pub struct Rendered {
    pub svg: String,
    pub summary: String,
    pub dtype: DiagramType,
    pub title: String,
    /// The parts in the order of the diagram, for a viewer that steps
    /// through them.
    pub order: Vec<Step>,
    /// The source stories, then the default story of the type.
    pub stories: Vec<Story>,
    /// The parts of each node of a progress story at the statuses planned,
    /// in progress, and done.
    pub variants: Vec<Variant>,
    /// All levels, the top level first, in depth-first order.
    pub levels: Vec<RenderedLevel>,
}

/// The SVG text of the node groups of one id, drawn at the statuses
/// planned, in progress, and done. One id can have two groups: a timeline
/// draws a "now" mark for an item in progress.
#[derive(Clone, Debug)]
pub struct Variant {
    pub id: String,
    pub planned: String,
    pub wip: String,
    pub done: String,
    /// The status of the source, for the done count of the kicker.
    pub status: usize,
}

impl Variant {
    /// The text for a status: 1 done, 2 in progress, else planned.
    pub fn at(&self, status: usize) -> &str {
        match status {
            1 => &self.done,
            2 => &self.wip,
            _ => &self.planned,
        }
    }
}

/// One part in the order of a diagram.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Step {
    /// A sequence message: the edge index.
    Msg(usize),
    /// A node: the id.
    Node(String),
}

/// A sequence steps through its messages. The other types step through
/// their nodes in line order.
fn order(d: &Diagram) -> Vec<Step> {
    if d.dtype == DiagramType::Seq {
        return (0..d.edges.len()).map(Step::Msg).collect();
    }
    let mut nodes: Vec<&crate::model::Item> = d.items.iter().filter(|it| !it.group && !it.zoom.context).collect();
    nodes.sort_by_key(|it| it.line);
    nodes.into_iter().map(|it| Step::Node(it.id.clone())).collect()
}

/// One level of a diagram with levels. The top level comes first.
pub struct RenderedLevel {
    /// The ids of the zoomed nodes, joined with `/`. Empty for the top level.
    pub path: String,
    pub title: String,
    pub dtype: DiagramType,
    pub svg: String,
    pub order: Vec<Step>,
}

/// The SVG of a diagram, with the logos in `icons`.
fn draw(d: &Diagram, icons: &[Option<Icon>]) -> String {
    if d.dtype == DiagramType::Seq {
        render_seq(d, &layout_seq(d))
    } else if d.dtype == DiagramType::Timeline {
        render_timeline(d, &layout_timeline(d))
    } else {
        render_graph(d, &layout_graph(d), icons)
    }
}

/// Draws the diagram three times, with the nodes of the progress beats at
/// each status, and keeps the node groups.
fn variants(d: &mut Diagram, stories: &[Story], icons: &[Option<Icon>]) -> Vec<Variant> {
    let ids: Vec<String> =
        stories.iter().flat_map(|s| s.beats.iter().filter_map(|b| b.node.as_ref().map(|n| n.0.clone()))).collect();
    if ids.is_empty() {
        return Vec::new();
    }
    let real: Vec<usize> = d.items.iter().map(|it| it.status).collect();
    let status_of = |id: &str| d.items.iter().position(|it| it.id == id).map_or(0, |i| real[i]);
    let statuses: Vec<usize> = ids.iter().map(|id| status_of(id)).collect();
    let mut texts: [Vec<String>; 3] = Default::default();
    for (k, status) in [3, 2, 1].into_iter().enumerate() {
        for it in d.items.iter_mut().filter(|it| !it.group) {
            it.status = status;
        }
        let svg = draw(d, icons);
        texts[k] = ids.iter().map(|id| node_groups(&svg, id).concat()).collect();
    }
    for (it, &s) in d.items.iter_mut().zip(&real) {
        it.status = s;
    }
    let [planned, wip, done] = texts;
    let mut out: Vec<Variant> = Vec::new();
    for (k, id) in ids.into_iter().enumerate() {
        if !out.iter().any(|v| v.id == id) {
            out.push(Variant {
                id,
                planned: planned[k].clone(),
                wip: wip[k].clone(),
                done: done[k].clone(),
                status: statuses[k],
            });
        }
    }
    out
}

/// The byte ranges of the node groups of `id`, in SVG order.
pub fn node_spans(svg: &str, id: &str) -> Vec<std::ops::Range<usize>> {
    let open = format!("<g data-k=\"node\" data-id=\"{}\" ", crate::svg::esc(id));
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = svg[from..].find(&open) {
        let start = from + at;
        // The groups nest, so count the open and the close tags.
        let mut depth = 0;
        let mut k = start;
        let end = loop {
            let next_open = svg[k..].find("<g").map(|x| x + k);
            let next_close = svg[k..].find("</g>").map(|x| x + k);
            match (next_open, next_close) {
                (Some(o), Some(c)) if o < c => {
                    depth += 1;
                    k = o + 2;
                }
                (_, Some(c)) => {
                    depth -= 1;
                    k = c + 4;
                    if depth == 0 {
                        break k;
                    }
                }
                _ => break svg.len(),
            }
        };
        out.push(start..end);
        from = end;
    }
    out
}

fn node_groups<'a>(svg: &'a str, id: &str) -> Vec<&'a str> {
    node_spans(svg, id).into_iter().map(|r| &svg[r]).collect()
}

/// The logos of the nodes of `d`. A logo that no source has goes in
/// `missing`.
fn icons_of(d: &Diagram, opts: &IconOptions, missing: &mut Vec<String>) -> Vec<Option<Icon>> {
    let mut icons: Vec<Option<Icon>> = d.items.iter().map(|_| None).collect();
    // Only a diagram with tiles draws logos. The other types draw cards.
    if d.dtype.has_tiles() {
        let wanted: Vec<usize> = (0..d.items.len()).filter(|&i| !d.items[i].icon.is_empty()).collect();
        let names: Vec<String> = wanted.iter().map(|&i| d.items[i].icon.clone()).collect();
        for (&i, found) in wanted.iter().zip(resolve(&names, opts)) {
            if found.is_none() && !missing.contains(&d.items[i].icon) {
                missing.push(d.items[i].icon.clone());
            }
            icons[i] = found;
        }
    }
    icons
}

/// Renders a source with no file path. A file link is relative to the
/// current directory.
pub fn render(src: &str, opts: &IconOptions) -> Result<Rendered, DiagramError> {
    render_path(src, None, opts)
}

/// Renders a source and all its levels. `file` is the path of the source,
/// for the file links.
pub fn render_path(src: &str, file: Option<&Path>, opts: &IconOptions) -> Result<Rendered, DiagramError> {
    let mut levels = load(src, file)?;
    let mut missing: Vec<String> = Vec::new();
    let mut out: Vec<RenderedLevel> = Vec::with_capacity(levels.len());
    let mut top_icons: Vec<Option<Icon>> = Vec::new();
    for lv in &levels {
        let icons = icons_of(&lv.d, opts, &mut missing);
        let svg = draw(&lv.d, &icons);
        if out.is_empty() {
            top_icons = icons;
        }
        out.push(RenderedLevel {
            path: lv.key(),
            title: lv.d.title.clone(),
            dtype: lv.d.dtype,
            svg,
            order: order(&lv.d),
        });
    }
    // The stories and the progress drawings are of the top level only.
    let stories = stories(&levels[0].d);
    let variants = variants(&mut levels[0].d, &stories, &top_icons);
    let d = &levels[0].d;
    let nodes = d.items.iter().filter(|it| !it.group).count();
    // The summary lists the ids that only an edge names, so a typing error
    // in an id is visible. It also lists the logos that no source has.
    let mut implicit: Vec<&str> = Vec::new();
    for lv in &levels {
        for it in lv.d.items.iter().filter(|it| !it.group && it.implicit) {
            if !implicit.contains(&it.id.as_str()) {
                implicit.push(&it.id);
            }
        }
    }
    let mut summary = format!("ok {} nodes={} edges={}", d.dtype.name(), nodes, d.edges.len());
    if d.dtype == DiagramType::Change {
        let [add, m, del] = change_counts(d);
        summary.push_str(&format!(" added={add} changed={m} removed={del}"));
    }
    if d.dtype == DiagramType::Deps {
        summary.push_str(&format!(" cycles={}", find_cycles(d).count));
    }
    if levels.len() > 1 {
        let ids: Vec<&str> = levels[1..].iter().filter_map(|lv| lv.path.last().map(String::as_str)).collect();
        summary.push_str(&format!(" levels={} details={}", levels.len(), ids.join(",")));
    }
    if !implicit.is_empty() {
        summary.push_str(" implicit=");
        summary.push_str(&implicit.join(","));
    }
    if !missing.is_empty() {
        summary.push_str(" noicon=");
        summary.push_str(&missing.join(","));
    }
    let top = &out[0];
    Ok(Rendered {
        svg: top.svg.clone(),
        summary,
        dtype: d.dtype,
        title: d.title.clone(),
        order: top.order.clone(),
        levels: out,
        stories,
        variants,
    })
}

impl Rendered {
    /// The level at `path` (`cart/svc`). An empty path gives the top level.
    pub fn level(&self, path: &str) -> Option<&RenderedLevel> {
        let path = path.trim_matches('/');
        self.levels.iter().find(|l| l.path == path)
    }
}
