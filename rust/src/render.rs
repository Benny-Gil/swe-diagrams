//! Source text in, SVG and a one-line summary out.

use crate::cycles::find_cycles;
use crate::icons::{resolve, Icon, IconOptions};
use crate::layout_graph::layout_graph;
use crate::layout_seq::layout_seq;
use crate::layout_timeline::layout_timeline;
use crate::model::{DiagramError, DiagramType};
use crate::parse::parse;
use crate::svg::{change_counts, render_graph, render_seq};
use crate::svg_timeline::render_timeline;
use crate::svg_vendor::group_icon;

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
            // An AWS group with no `@` icon gets the group icon of its kind.
            let name = |i: usize| match group_icon(&d.items, i) {
                Some(g) if d.items[i].icon.is_empty() => g.to_string(),
                _ => d.items[i].icon.clone(),
            };
            let wanted: Vec<usize> = (0..d.items.len()).filter(|&i| !name(i).is_empty()).collect();
            let names: Vec<String> = wanted.iter().map(|&i| name(i)).collect();
            for (&i, found) in wanted.iter().zip(resolve(&names, opts)) {
                let asked = !d.items[i].icon.is_empty();
                if found.is_none() && asked && !missing.contains(&d.items[i].icon.as_str()) {
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
    Ok(Rendered { svg, summary })
}
