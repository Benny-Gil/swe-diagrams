//! The stories of a diagram: the `story` blocks of the source, then the
//! default story of the type. The first story is the default for an
//! animated output.

use crate::model::{Diagram, DiagramType};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Tokens move along edges, one beat after the other.
    Steps,
    /// Tokens move along each edge without stop.
    Stream,
    /// The nodes change from planned to in progress to done.
    Progress,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Steps => "steps",
            Mode::Stream => "stream",
            Mode::Progress => "progress",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Move {
    /// The edge index, which is `data-i` in the SVG.
    pub edge: usize,
    /// True when the token moves against the direction of the edge.
    pub back: bool,
    pub hollow: bool,
    pub caption: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Beat {
    /// The moves that run at the same time.
    pub moves: Vec<Move>,
    /// A progress beat: the node id and its last status (1 done, 2 in
    /// progress).
    pub node: Option<(String, usize)>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Story {
    pub title: String,
    pub mode: Mode,
    pub beats: Vec<Beat>,
}

fn step(d: &Diagram, k: usize) -> Beat {
    let e = &d.edges[k];
    Beat { moves: vec![Move { edge: k, back: false, hollow: e.dashed, caption: e.label.clone() }], node: None }
}

/// The source stories, then the default story of the type, if it has one.
pub fn stories(d: &Diagram) -> Vec<Story> {
    let mut out: Vec<Story> = d
        .stories
        .iter()
        .map(|s| Story {
            title: s.title.clone(),
            mode: Mode::Steps,
            beats: s
                .beats
                .iter()
                .map(|b| Beat {
                    moves: b
                        .iter()
                        .map(|m| Move { edge: m.edge, back: m.back, hollow: m.hollow, caption: m.caption.clone() })
                        .collect(),
                    node: None,
                })
                .collect(),
        })
        .collect();
    if let Some(s) = default_story(d) {
        if !s.beats.is_empty() {
            out.push(s);
        }
    }
    out
}

/// The motion of a diagram with no story block.
pub fn default_story(d: &Diagram) -> Option<Story> {
    let main = || -> Vec<Beat> { (0..d.edges.len()).filter(|&k| d.edges[k].main).map(|k| step(d, k)).collect() };
    let (title, mode, beats) = match d.dtype {
        DiagramType::Seq => ("Messages", Mode::Steps, (0..d.edges.len()).map(|k| step(d, k)).collect()),
        DiagramType::Plan | DiagramType::Timeline => ("Progress", Mode::Progress, progress(d)),
        DiagramType::Dataflow => {
            let moves = (0..d.edges.len()).map(|k| step(d, k).moves.remove(0)).collect();
            ("Stream", Mode::Stream, vec![Beat { moves, node: None }])
        }
        DiagramType::Flow => {
            let m = main();
            ("Main path", Mode::Steps, if m.is_empty() { walk(d, "start") } else { m })
        }
        DiagramType::State => ("Walk", Mode::Steps, walk(d, "initial")),
        _ => ("Main path", Mode::Steps, main()),
    };
    if d.edges.is_empty() && mode != Mode::Progress {
        return None;
    }
    Some(Story { title: title.to_string(), mode, beats })
}

/// The tasks in line order. A done task or a task with no status goes to
/// done. A task in progress stops there. A planned task does not move.
fn progress(d: &Diagram) -> Vec<Beat> {
    let mut nodes: Vec<&crate::model::Item> = d.items.iter().filter(|it| !it.group && it.status != 3).collect();
    nodes.sort_by_key(|it| it.line);
    nodes
        .into_iter()
        .map(|it| Beat { moves: Vec::new(), node: Some((it.id.clone(), if it.status == 2 { 2 } else { 1 })) })
        .collect()
}

/// A walk from the first node of kind `first` (else the first node in
/// line order). At each node it takes the first unused edge out, in line
/// order, and it stops at a node with no unused edge out.
fn walk(d: &Diagram, first: &str) -> Vec<Beat> {
    let kind = d.dtype.kinds().iter().position(|&k| k == first);
    let mut nodes: Vec<usize> = (0..d.items.len()).filter(|&i| !d.items[i].group).collect();
    nodes.sort_by_key(|&i| d.items[i].line);
    let Some(mut at) = nodes.iter().copied().find(|&i| Some(d.items[i].kind) == kind).or(nodes.first().copied()) else {
        return Vec::new();
    };
    let mut used = vec![false; d.edges.len()];
    let mut out = Vec::new();
    while let Some(k) = (0..d.edges.len()).find(|&k| !used[k] && d.edges[k].from == at) {
        used[k] = true;
        out.push(step(d, k));
        at = d.edges[k].to;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    fn edges(s: &Story) -> Vec<usize> {
        s.beats.iter().flat_map(|b| b.moves.iter().map(|m| m.edge)).collect()
    }

    #[test]
    fn the_main_edges_make_the_default_story() {
        let d = parse("arch\na => b -> c\nc => d\n").unwrap();
        let s = stories(&d);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].title, "Main path");
        assert_eq!(edges(&s[0]), [0, 2]);
    }

    #[test]
    fn source_stories_come_first() {
        let d = parse("arch\na => b\nstory \"Back\"\n  b --> a\n").unwrap();
        let titles: Vec<String> = stories(&d).into_iter().map(|s| s.title).collect();
        assert_eq!(titles, ["Back", "Main path"]);
    }

    #[test]
    fn an_arch_with_no_main_edge_has_no_story() {
        assert!(stories(&parse("arch\na -> b\n").unwrap()).is_empty());
    }

    #[test]
    fn a_state_walk_takes_each_edge_once() {
        let d = parse("state\ni :initial\ni -> a\na -> a \"edit\"\na -> b \"go\"\nb -> a\n").unwrap();
        let s = &stories(&d)[0];
        assert_eq!(edges(s), [0, 1, 2, 3]);
        assert_eq!(s.beats[2].moves[0].caption, "go");
    }

    #[test]
    fn a_flow_walks_from_the_start() {
        let d = parse("flow\nx\ns :start\ns -> x\nx -> e\n").unwrap();
        assert_eq!(edges(&stories(&d)[0]), [0, 1]);
    }

    #[test]
    fn a_plan_shows_progress_up_to_the_real_status() {
        let d = parse("plan\na done\nb wip\nc plan\nd\n").unwrap();
        let s = &stories(&d)[0];
        assert_eq!(s.mode, Mode::Progress);
        let nodes: Vec<(String, usize)> = s.beats.iter().filter_map(|b| b.node.clone()).collect();
        assert_eq!(nodes, [("a".into(), 1), ("b".into(), 2), ("d".into(), 1)]);
    }

    #[test]
    fn a_dataflow_streams_all_edges_at_once() {
        let d = parse("dataflow\na -> b\nb --> c\n").unwrap();
        let s = &stories(&d)[0];
        assert_eq!(s.mode, Mode::Stream);
        assert_eq!(s.beats.len(), 1);
        assert!(s.beats[0].moves[1].hollow);
    }

    #[test]
    fn a_seq_shows_each_message() {
        let d = parse("seq\na -> b\nb --> a\n").unwrap();
        assert_eq!(edges(&stories(&d)[0]), [0, 1]);
    }
}
