//! Source text in, SVG and a one-line summary out.

use crate::icons::{resolve, Icon, IconOptions};
use crate::layout_graph::layout_graph;
use crate::layout_seq::layout_seq;
use crate::layout_timeline::layout_timeline;
use crate::model::{DiagramError, DiagramType};
use crate::parse::parse;
use crate::svg::{render_graph, render_seq};
use crate::svg_timeline::render_timeline;

pub struct Rendered {
    pub svg: String,
    pub summary: String,
}

pub fn render(src: &str, opts: &IconOptions) -> Result<Rendered, DiagramError> {
    let d = parse(src)?;
    let mut missing: Vec<&str> = Vec::new();
    let svg = if d.dtype == DiagramType::Seq {
        render_seq(&d, &layout_seq(&d))
    } else if d.dtype == DiagramType::Timeline {
        render_timeline(&d, &layout_timeline(&d))
    } else {
        // Only a diagram with tiles draws logos. The other types draw cards.
        let mut icons: Vec<Option<Icon>> = d.items.iter().map(|_| None).collect();
        if d.dtype.has_tiles() {
            let wanted: Vec<usize> = (0..d.items.len()).filter(|&i| !d.items[i].icon.is_empty()).collect();
            let names: Vec<String> = wanted.iter().map(|&i| d.items[i].icon.clone()).collect();
            for (&i, found) in wanted.iter().zip(resolve(&names, opts)) {
                if found.is_none() && !missing.contains(&d.items[i].icon.as_str()) {
                    missing.push(&d.items[i].icon);
                }
                icons[i] = found;
            }
        }
        render_graph(&d, &layout_graph(&d), &icons)
    };
    let nodes = d.items.iter().filter(|it| !it.group).count();
    // The summary lists the ids that only an edge names, so a typing error
    // in an id is visible. It also lists the logos that no source has.
    let implicit: Vec<&str> = d.items.iter().filter(|it| !it.group && it.implicit).map(|it| it.id.as_str()).collect();
    let mut summary = format!("ok {} nodes={} edges={}", d.dtype.name(), nodes, d.edges.len());
    if !implicit.is_empty() {
        summary.push_str(" implicit=");
        summary.push_str(&implicit.join(","));
    }
    if !missing.is_empty() {
        summary.push_str(" noicon=");
        summary.push_str(&missing.join(","));
    }
    Ok(Rendered { svg, summary })
}
