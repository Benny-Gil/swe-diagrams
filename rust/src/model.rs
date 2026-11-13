//! The diagram model. The structures are flat and index-based.

use std::fmt;

pub const TYPES: [&str; 12] =
    ["arch", "seq", "plan", "deps", "flow", "state", "dataflow", "change", "erd", "class", "timeline", "journey"];
pub const STATUS_WORDS: [&str; 7] = ["", "done", "wip", "plan", "add", "mod", "del"];
pub const STATUS_NAMES: [&str; 7] = ["", "done", "in progress", "planned", "added", "changed", "removed"];
/// The kinds of `arch`, `change`, `seq`, and `plan`.
pub const KINDS: [&str; 5] = ["service", "store", "queue", "actor", "ext"];
pub const DEPS_KINDS: [&str; 2] = ["module", "ext"];
pub const FLOW_KINDS: [&str; 6] = ["step", "start", "end", "decision", "io", "sub"];
pub const STATE_KINDS: [&str; 4] = ["state", "initial", "final", "choice"];
pub const DATAFLOW_KINDS: [&str; 5] = ["transform", "source", "sink", "store", "queue"];
pub const ERD_KINDS: [&str; 1] = ["entity"];
pub const CLASS_KINDS: [&str; 4] = ["class", "interface", "abstract", "enum"];
pub const TIMELINE_KINDS: [&str; 2] = ["event", "milestone"];
pub const JOURNEY_KINDS: [&str; 3] = ["page", "modal", "external"];

pub const ARROWS: [&str; 4] = ["=>", "->", "-->", "<->"];
pub const LINK_ARROWS: [&str; 2] = ["->", "-->"];
pub const FLOW_ARROWS: [&str; 3] = ["=>", "->", "-->"];
/// `<left>--<right>`. An end is `1` (one), `?` (zero or one), `*` (zero or
/// many), or `+` (one or many).
#[rustfmt::skip]
pub const ERD_ARROWS: [&str; 16] = [
    "1--1", "1--?", "1--*", "1--+", "?--1", "?--?", "?--*", "?--+",
    "*--1", "*--?", "*--*", "*--+", "+--1", "+--?", "+--*", "+--+",
];
pub const CLASS_ARROWS: [&str; 6] = ["--|>", "..|>", "*--", "o--", "->", "..>"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiagramType {
    Arch,
    Seq,
    Plan,
    Deps,
    Flow,
    State,
    Dataflow,
    Change,
    Erd,
    Class,
    Timeline,
    Journey,
}

impl DiagramType {
    /// The word in `TYPES` for this type.
    pub fn name(self) -> &'static str {
        TYPES[self as usize]
    }

    pub fn from_name(name: &str) -> Option<DiagramType> {
        use DiagramType::*;
        const ALL: [DiagramType; 12] =
            [Arch, Seq, Plan, Deps, Flow, State, Dataflow, Change, Erd, Class, Timeline, Journey];
        TYPES.iter().position(|&t| t == name).map(|i| ALL[i])
    }

    /// The kind words of this type. `Item::kind` is an index in this list.
    pub fn kinds(self) -> &'static [&'static str] {
        match self {
            DiagramType::Arch | DiagramType::Seq | DiagramType::Plan | DiagramType::Change => &KINDS,
            DiagramType::Deps => &DEPS_KINDS,
            DiagramType::Flow => &FLOW_KINDS,
            DiagramType::State => &STATE_KINDS,
            DiagramType::Dataflow => &DATAFLOW_KINDS,
            DiagramType::Erd => &ERD_KINDS,
            DiagramType::Class => &CLASS_KINDS,
            DiagramType::Timeline => &TIMELINE_KINDS,
            DiagramType::Journey => &JOURNEY_KINDS,
        }
    }

    /// The arrows of this type. A `timeline` has no edges.
    pub fn arrows(self) -> &'static [&'static str] {
        match self {
            DiagramType::Arch | DiagramType::Seq | DiagramType::Plan | DiagramType::Change => &ARROWS,
            DiagramType::Deps | DiagramType::State | DiagramType::Dataflow => &LINK_ARROWS,
            DiagramType::Flow | DiagramType::Journey => &FLOW_ARROWS,
            DiagramType::Erd => &ERD_ARROWS,
            DiagramType::Class => &CLASS_ARROWS,
            DiagramType::Timeline => &[],
        }
    }

    /// The direction of the layers when the source has no `dir` line.
    pub fn default_down(self) -> bool {
        matches!(self, DiagramType::Plan | DiagramType::Deps | DiagramType::Flow | DiagramType::Class)
    }

    /// True when a node line can hold member lines (fields and methods).
    pub fn has_members(self) -> bool {
        matches!(self, DiagramType::Erd | DiagramType::Class)
    }

    /// True when a node is an icon tile with the label below it.
    pub fn has_tiles(self) -> bool {
        matches!(self, DiagramType::Arch | DiagramType::Change)
    }
}

/// True when the word is an arrow of some type.
pub fn is_arrow_word(w: &str) -> bool {
    ARROWS.contains(&w) || ERD_ARROWS.contains(&w) || CLASS_ARROWS.contains(&w)
}

/// The flags of an `erd` field, after the name and the type.
pub const ERD_FLAGS: [&str; 4] = ["pk", "fk", "uniq", "null"];

/// An indented line under a node line of `erd` or `class`.
#[derive(Debug)]
pub struct Member {
    /// The words of the line, joined with one space, without the status.
    pub text: String,
    /// The words of the line, without the status. A quoted string is one word.
    pub words: Vec<String>,
    /// Index in `STATUS_WORDS`.
    pub status: usize,
    pub line: usize,
}

#[derive(Debug)]
pub struct Item {
    pub id: String,
    pub label: String,
    pub sub: String,
    /// Index in `Diagram::dtype.kinds()`.
    pub kind: usize,
    /// Index in `STATUS_WORDS`.
    pub status: usize,
    pub group: bool,
    /// Item index. `None` for an item of the root.
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub line: usize,
    /// True when only an edge names the item.
    pub implicit: bool,
    /// The logo name after `@`, or empty.
    pub icon: String,
    /// A group with the `:vendor` kind gets the warm palette.
    pub vendor: bool,
    /// The fields and methods of an `erd` or `class` node, in line order.
    pub members: Vec<Member>,
    pub zoom: Zoom,
}

/// The place of an item in a diagram with levels.
#[derive(Debug, Default, Clone)]
pub struct Zoom {
    /// The file after `>` on the node line, or empty.
    pub link: String,
    /// True when the node has a detail level.
    pub detail: bool,
    /// True for a node of the parent level in a detail level (`^name`).
    pub context: bool,
}

#[derive(Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub dashed: bool,
    pub bidir: bool,
    /// `=>`: a step of the main flow. It gets the accent colour.
    pub main: bool,
    pub status: usize,
    pub line: usize,
    /// The arrow word of the line, from `Diagram::dtype.arrows()`.
    pub arrow: &'static str,
}

#[derive(Debug)]
pub struct Diagram {
    pub dtype: DiagramType,
    pub title: String,
    /// Layer direction: false = left to right, true = top to bottom.
    pub down: bool,
    pub items: Vec<Item>,
    pub edges: Vec<Edge>,
    pub roots: Vec<usize>,
}

/// All errors of one source text. Each error starts with the line number.
#[derive(Debug)]
pub struct DiagramError {
    pub errors: Vec<String>,
}

impl fmt::Display for DiagramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.errors.join("\n"))
    }
}

impl std::error::Error for DiagramError {}

/// Floor division. The divisor must be positive.
#[inline]
pub fn idiv(a: i64, b: i64) -> i64 {
    a.div_euclid(b)
}

/// Division that rounds to the nearest integer, and rounds a half up.
/// The divisor must be positive.
#[inline]
pub fn rdiv(a: i64, b: i64) -> i64 {
    (2 * a + b).div_euclid(2 * b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idiv_rounds_down() {
        assert_eq!(idiv(7, 2), 3);
        assert_eq!(idiv(-7, 2), -4);
        assert_eq!(idiv(-8, 2), -4);
        assert_eq!(idiv(0, 5), 0);
    }

    #[test]
    fn rdiv_rounds_a_half_up() {
        assert_eq!(rdiv(7, 2), 4);
        assert_eq!(rdiv(-7, 2), -3);
        assert_eq!(rdiv(5, 3), 2);
        assert_eq!(rdiv(4, 3), 1);
        assert_eq!(rdiv(-5, 3), -2);
        assert_eq!(rdiv(-4, 3), -1);
    }
}
