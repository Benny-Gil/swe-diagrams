//! Source text in, SVG and a one-line summary out.

use crate::cycles::find_cycles;
use crate::icons::{resolve, Icon, IconOptions};
use crate::layout_graph::layout_graph;
use crate::layout_seq::layout_seq;
use crate::layout_timeline::layout_timeline;
use crate::model::{Diagram, DiagramError, DiagramType};
use crate::parse::parse;
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
    let mut nodes: Vec<&crate::model::Item> = d.items.iter().filter(|it| !it.group).collect();
    nodes.sort_by_key(|it| it.line);
    nodes.into_iter().map(|it| Step::Node(it.id.clone())).collect()
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
    let mut texts: [Vec<String>; 3] = Default::default();
    for (k, status) in [3, 2, 1].into_iter().enumerate() {
        for it in d.items.iter_mut().filter(|it| !it.group) {
            it.status = status;
        }
        let svg = draw(d, icons);
        texts[k] = ids.iter().map(|id| node_groups(&svg, id).concat()).collect();
    }
    for (it, s) in d.items.iter_mut().zip(real) {
        it.status = s;
    }
    let [planned, wip, done] = texts;
    let mut out: Vec<Variant> = Vec::new();
    for (k, id) in ids.into_iter().enumerate() {
        if !out.iter().any(|v| v.id == id) {
            out.push(Variant { id, planned: planned[k].clone(), wip: wip[k].clone(), done: done[k].clone() });
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

pub fn render(src: &str, opts: &IconOptions) -> Result<Rendered, DiagramError> {
    let mut d = parse(src)?;
    let mut missing: Vec<String> = Vec::new();
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
    let svg = draw(&d, &icons);
    let stories = stories(&d);
    let variants = variants(&mut d, &stories, &icons);
    let nodes = d.items.iter().filter(|it| !it.group).count();
    // The summary lists the ids that only an edge names, so a typing error
    // in an id is visible. It also lists the logos that no source has.
    let implicit: Vec<&str> = d.items.iter().filter(|it| !it.group && it.implicit).map(|it| it.id.as_str()).collect();
    let mut summary = format!("ok {} nodes={} edges={}", d.dtype.name(), nodes, d.edges.len());
    if d.dtype == DiagramType::Change {
        let [add, m, del] = change_counts(&d);
        summary.push_str(&format!(" added={add} changed={m} removed={del}"));
    }
    if d.dtype == DiagramType::Deps {
        summary.push_str(&format!(" cycles={}", find_cycles(&d).count));
    }
    if !implicit.is_empty() {
        summary.push_str(" implicit=");
        summary.push_str(&implicit.join(","));
    }
    if !missing.is_empty() {
        summary.push_str(" noicon=");
        summary.push_str(&missing.join(","));
    }
    Ok(Rendered { svg, summary, dtype: d.dtype, title: d.title.clone(), order: order(&d), stories, variants })
}
