// PNG tests. Each fixture gives a PNG that decodes, has the size of the
// SVG times the scale, and is not blank. The bytes are not compared.

use std::fs;
use std::path::{Path, PathBuf};

use swe_diagrams::tiny_skia::Pixmap;
use swe_diagrams::{apply_theme, png, render, render_path, IconOptions, Theme};

fn spec(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("spec").join(dir)
}

/// The width and the height in the root tag of an SVG.
fn svg_size(svg: &str) -> (u32, u32) {
    let attr = |name: &str| -> u32 {
        let at = svg.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
        svg[at..].split('"').next().unwrap().parse().unwrap()
    };
    (attr("width"), attr("height"))
}

#[test]
fn each_fixture_gives_a_png() {
    let mut list: Vec<PathBuf> = fs::read_dir(spec("fixtures"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "sd"))
        .collect();
    list.sort();
    let opts = IconOptions { dir: Some(spec("icons")), offline: true };
    for path in list {
        let r = render_path(&fs::read_to_string(&path).unwrap(), Some(&path), &opts).unwrap();
        let svg = apply_theme(&r.svg, Theme::Light);
        let bytes = png(&svg, 1.0).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let pixmap = Pixmap::decode_png(&bytes).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), svg_size(&r.svg), "{}", path.display());
        let first = pixmap.pixels()[0];
        let differ = pixmap.pixels().iter().filter(|&&p| p != first).count();
        assert!(differ > pixmap.pixels().len() / 20, "{} looks blank", path.display());
    }
}

#[test]
fn the_scale_sets_the_size() {
    let opts = IconOptions { dir: Some(spec("icons")), offline: true };
    let r = render(&fs::read_to_string(spec("fixtures/seq-login.sd")).unwrap(), &opts).unwrap();
    let (w, h) = svg_size(&r.svg);
    let pixmap = Pixmap::decode_png(&png(&r.svg, 2.0).unwrap()).unwrap();
    assert_eq!((pixmap.width(), pixmap.height()), (2 * w, 2 * h));
}

#[test]
fn a_logo_shows_in_the_png() {
    // The no-logo tile and the logo tile differ only by the logo, so a logo
    // must put colour on the tile. The arch-logos App tile has a blue logo.
    let opts = IconOptions { dir: Some(spec("icons")), offline: true };
    let r = render(&fs::read_to_string(spec("fixtures/arch-logos.sd")).unwrap(), &opts).unwrap();
    let pixmap = swe_diagrams::rasterize(&apply_theme(&r.svg, Theme::Light), 1.0).unwrap();
    let blue = pixmap.pixels().iter().filter(|p| p.blue() > 150 && p.red() < 120 && p.green() < 140).count();
    assert!(blue > 100, "only {blue} blue pixels");
}
