//! Source text in the diagram language in, SVG out.
//!
//! All layout values are `i64`, and all layout math is integer math, so the
//! output is the same on each machine.

pub mod cycles;
pub mod html;
pub mod icons;
pub mod labels;
pub mod layout_graph;
pub mod layout_seq;
pub mod layout_timeline;
pub mod model;
pub mod output;
pub mod parse;
pub mod raster;
pub mod records;
pub mod render;
pub mod svg;
pub mod svg_shapes;
pub mod svg_timeline;
pub mod text;
pub mod theme;

pub use html::html;
pub use icons::IconOptions;
pub use model::DiagramError;
pub use output::Format;
pub use raster::{png, rasterize};
pub use render::{render, Rendered, Step};
pub use resvg::tiny_skia;
pub use theme::{apply_theme, Theme};
