//! The diagram model. The structures are flat and index-based.

use std::fmt;

pub const TYPES: [&str; 3] = ["arch", "seq", "plan"];
pub const STATUS_WORDS: [&str; 7] = ["", "done", "wip", "plan", "add", "mod", "del"];
pub const STATUS_NAMES: [&str; 7] = ["", "done", "in progress", "planned", "added", "changed", "removed"];
pub const KINDS: [&str; 5] = ["service", "store", "queue", "actor", "ext"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiagramType {
    Arch,
    Seq,
    Plan,
}

impl DiagramType {
    /// The word in `TYPES` for this type.
    pub fn name(self) -> &'static str {
        TYPES[self as usize]
    }

    pub fn from_name(name: &str) -> Option<DiagramType> {
        match name {
            "arch" => Some(DiagramType::Arch),
            "seq" => Some(DiagramType::Seq),
            "plan" => Some(DiagramType::Plan),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Item {
    pub id: String,
    pub label: String,
    pub sub: String,
    /// Index in `KINDS`.
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
