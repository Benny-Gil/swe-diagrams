//! SVG to pixels with resvg. The fonts are in the binary, and no system font
//! loads, so each machine gives the same picture.

use std::sync::{Arc, OnceLock};

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, fontdb, FontFamily, FontResolver};

const FONTS: [&[u8]; 4] = [
    include_bytes!("../fonts/Geist-Regular.ttf"),
    include_bytes!("../fonts/Geist-SemiBold.ttf"),
    include_bytes!("../fonts/Geist-Bold.ttf"),
    include_bytes!("../fonts/JetBrainsMono-Regular.ttf"),
];

const SANS: &str = "Geist";
const MONO: &str = "JetBrains Mono";

fn fonts() -> Arc<fontdb::Database> {
    static DB: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        for data in FONTS {
            db.load_font_data(data.to_vec());
        }
        db.set_sans_serif_family(SANS);
        db.set_serif_family(SANS);
        db.set_monospace_family(MONO);
        Arc::new(db)
    })
    .clone()
}

/// True when a family of the style sheet names a mono face.
fn is_mono(family: &FontFamily) -> bool {
    match family {
        FontFamily::Monospace => true,
        FontFamily::Named(name) => {
            let name = name.to_ascii_lowercase();
            name.contains("mono") || name == "menlo" || name == "consolas"
        }
        _ => false,
    }
}

/// Each family of the CSS maps to one of the two bundled faces. The weight
/// picks regular, semibold, or bold.
fn resolver() -> FontResolver<'static> {
    FontResolver {
        select_font: Box::new(|font, db| {
            let family = if font.families().iter().any(is_mono) { MONO } else { SANS };
            db.query(&fontdb::Query {
                families: &[fontdb::Family::Name(family)],
                weight: fontdb::Weight(font.weight()),
                stretch: fontdb::Stretch::Normal,
                style: fontdb::Style::Normal,
            })
        }),
        select_fallback: FontResolver::default_fallback_selector(),
    }
}

/// The pixels of an SVG, `scale` times its size.
pub fn rasterize(svg: &str, scale: f32) -> Result<Pixmap, String> {
    let opts = usvg::Options { fontdb: fonts(), font_resolver: resolver(), ..usvg::Options::default() };
    let tree = usvg::Tree::from_str(svg, &opts).map_err(|e| format!("cannot read the SVG: {e}"))?;
    let size = tree.size().to_int_size().scale_by(scale).ok_or("the picture is too small")?;
    let mut pixmap = Pixmap::new(size.width(), size.height()).ok_or("the picture is too large")?;
    resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    Ok(pixmap)
}

/// The PNG file of an SVG, `scale` times its size.
pub fn png(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    rasterize(svg, scale)?.encode_png().map_err(|e| format!("cannot write the PNG: {e}"))
}
