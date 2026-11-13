//! Source text in the diagram language in, SVG out.
//!
//! All layout values are `i64`, and all layout math is integer math, so the
//! output is the same on each machine.

pub mod anim;
pub mod cycles;
pub mod html;
pub mod icons;
pub mod layout_graph;
pub mod layout_seq;
pub mod layout_timeline;
pub mod levels;
pub mod model;
pub mod output;
pub mod parse;
mod parse_story;
pub mod raster;
pub mod records;
pub mod render;
pub mod story;
pub mod svg;
pub mod svg_shapes;
pub mod svg_timeline;
pub mod text;
pub mod text_out;
pub mod theme;
pub mod video;

pub use html::{html, html_at};
pub use icons::IconOptions;
pub use model::DiagramError;
pub use output::Format;
pub use raster::{png, rasterize};
pub use render::{render, render_path, Rendered, RenderedLevel, Step, Variant};
pub use resvg::tiny_skia;
pub use theme::{apply_theme, Theme};
