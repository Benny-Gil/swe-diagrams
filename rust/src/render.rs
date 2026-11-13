//! Source text in, SVG and a one-line summary out.

use std::path::Path;

use crate::cycles::find_cycles;
use crate::icons::{resolve, Icon, IconOptions};
use crate::layout_graph::layout_graph;
use crate::layout_seq::layout_seq;
use crate::layout_timeline::layout_timeline;
use crate::levels::load;
use crate::model::{Diagram, DiagramError, DiagramType};
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
    /// All levels, the top level first, in depth-first order.
    pub levels: Vec<RenderedLevel>,
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

/// The SVG of one diagram. A logo that no source has goes in `missing`.
fn draw(d: &Diagram, opts: &IconOptions, missing: &mut Vec<String>) -> String {
    if d.dtype == DiagramType::Seq {
        return render_seq(d, &layout_seq(d));
    }
    if d.dtype == DiagramType::Timeline {
        return render_timeline(d, &layout_timeline(d));
    }
    // Only a diagram with tiles draws logos. The other types draw cards.
    let mut icons: Vec<Option<Icon>> = d.items.iter().map(|_| None).collect();
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
    render_graph(d, &layout_graph(d), &icons)
}

/// Renders a source with no file path. A file link is relative to the
/// current directory.
pub fn render(src: &str, opts: &IconOptions) -> Result<Rendered, DiagramError> {
    render_path(src, None, opts)
}

/// Renders a source and all its levels. `file` is the path of the source,
/// for the file links.
pub fn render_path(src: &str, file: Option<&Path>, opts: &IconOptions) -> Result<Rendered, DiagramError> {
    let levels = load(src, file)?;
    let mut missing: Vec<String> = Vec::new();
    let mut out: Vec<RenderedLevel> = Vec::with_capacity(levels.len());
    for lv in &levels {
        let svg = draw(&lv.d, opts, &mut missing);
        out.push(RenderedLevel {
            path: lv.key(),
            title: lv.d.title.clone(),
            dtype: lv.d.dtype,
            svg,
            order: order(&lv.d),
        });
    }
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
    })
}

impl Rendered {
    /// The level at `path` (`cart/svc`). An empty path gives the top level.
    pub fn level(&self, path: &str) -> Option<&RenderedLevel> {
        let path = path.trim_matches('/');
        self.levels.iter().find(|l| l.path == path)
    }
}
