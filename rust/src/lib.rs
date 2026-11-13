//! Source text in the diagram language in, SVG out.
//!
//! All layout values are `i64`, and all layout math is integer math, so the
//! output is the same on each machine.

pub mod cycles;
pub mod icons;
pub mod layout_graph;
pub mod layout_seq;
pub mod model;
pub mod parse;
pub mod records;
pub mod render;
pub mod svg;
pub mod svg_shapes;
pub mod text;

pub use icons::IconOptions;
pub use model::DiagramError;
pub use render::{render, Rendered};
