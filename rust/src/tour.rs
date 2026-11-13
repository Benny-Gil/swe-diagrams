//! The zoom tour: frames that visit each detail level, depth first.
//! Each level is drawn one time. The frames are transforms of those
//! pixmaps, so a long tour does not draw the SVG again for each frame.

use crate::anim::frame_count;
use crate::raster::{node_boxes, rasterize};
use crate::render::Rendered;
use crate::theme::{apply_theme, Theme};
use crate::tiny_skia::{Color, FilterQuality, Pixmap, PixmapPaint, Transform};

/// The times of the tour at speed 1, in milliseconds.
const HOLD_TOP_MS: i64 = 1200;
const ZOOM_MS: i64 = 600;
const GROW_MS: i64 = 400;
const HOLD_MS: i64 = 1500;
/// A short rest on the parent after a zoom out, so two visits do not merge.
const BACK_MS: i64 = 400;
/// At the end of the zoom, the node box fills this part of the canvas.
const ZOOM_FILL: f32 = 0.4;

/// The options of the tour frames.
#[derive(Clone, Copy, Debug)]
pub struct TourOptions {
    pub fps: u32,
    pub speed: f64,
    pub scale: f32,
    pub theme: Theme,
}

/// The frames of a tour and the count of levels.
pub struct Tour {
    pub frames: Vec<Pixmap>,
    pub levels: usize,
}

/// Ease in and out. All zooms use this curve.
fn ease(u: f32) -> f32 {
    u * u * (3.0 - 2.0 * u)
}

fn lerp(a: f32, b: f32, e: f32) -> f32 {
    a + (b - a) * e
}

/// A box in canvas pixels.
#[derive(Clone, Copy, Debug)]
struct Box2 {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Box2 {
    fn cx(&self) -> f32 {
        self.x + self.w / 2.0
    }
    fn cy(&self) -> f32 {
        self.y + self.h / 2.0
    }
}

/// One pixmap with a scale and an offset on the canvas.
struct Place<'a> {
    pix: &'a Pixmap,
    s: f32,
    tx: f32,
    ty: f32,
    alpha: f32,
}

struct Level {
    pix: Pixmap,
    /// The scale at rest. A level larger than the canvas becomes smaller.
    fit: f32,
    /// The top left corner at rest. A level smaller than the canvas is in
    /// the middle.
    ox: f32,
    oy: f32,
    /// The child levels: the index and the node box in this pixmap.
    kids: Vec<(usize, Option<Box2>)>,
}

struct Ctx<'a> {
    levels: &'a [Level],
    w: u32,
    h: u32,
    bg: Color,
    fps: u32,
    speed: f64,
}

impl Ctx<'_> {
    fn n(&self, ms: i64) -> usize {
        frame_count(((ms as f64) / self.speed).round() as i64, self.fps)
    }

    fn frame(&self, parts: &[Place]) -> Result<Pixmap, String> {
        let mut out = Pixmap::new(self.w, self.h).ok_or("the tour is too large")?;
        out.fill(self.bg);
        for p in parts.iter().filter(|p| p.alpha > 0.0) {
            let paint = PixmapPaint { opacity: p.alpha, quality: FilterQuality::Bilinear, ..PixmapPaint::default() };
            let t = Transform::from_row(p.s, 0.0, 0.0, p.s, p.tx, p.ty);
            out.draw_pixmap(0, 0, p.pix.as_ref(), &paint, t, None);
        }
        Ok(out)
    }

    fn rest(&self, k: usize) -> Place<'_> {
        let lv = &self.levels[k];
        Place { pix: &lv.pix, s: lv.fit, tx: lv.ox, ty: lv.oy, alpha: 1.0 }
    }

    fn hold(&self, k: usize, ms: i64, out: &mut Vec<Pixmap>) -> Result<(), String> {
        let f = self.frame(&[self.rest(k)])?;
        for _ in 0..self.n(ms) {
            out.push(f.clone());
        }
        Ok(())
    }

    /// The zoom from parent `p` into child `c`. The camera zooms to the
    /// node box, then the child grows from the box and fades in.
    fn zoom_in(&self, p: usize, c: usize, node: Option<Box2>) -> Result<Vec<Pixmap>, String> {
        let (pl, cl) = (&self.levels[p], &self.levels[c]);
        let (w, h) = (self.w as f32, self.h as f32);
        // The size of the child at rest.
        let (cw, ch) = (cl.pix.width() as f32 * cl.fit, cl.pix.height() as f32 * cl.fit);
        // A node with no box zooms to the middle of the parent.
        let pb = node.unwrap_or(Box2 {
            x: pl.pix.width() as f32 * 0.4,
            y: pl.pix.height() as f32 * 0.4,
            w: pl.pix.width() as f32 * 0.2,
            h: pl.pix.height() as f32 * 0.2,
        });
        // The node box on the canvas at rest.
        let f = pl.fit;
        let b = Box2 { x: pb.x * f + pl.ox, y: pb.y * f + pl.oy, w: pb.w * f, h: pb.h * f };
        let s1 = (ZOOM_FILL * (w / b.w).min(h / b.h)).max(1.2);
        let mut out = Vec::new();
        let n = self.n(ZOOM_MS);
        for i in 0..n {
            let e = ease((i + 1) as f32 / n as f32);
            let s = s1.powf(e);
            let (vx, vy) = (lerp(w / 2.0, b.cx(), e) - w / (2.0 * s), lerp(h / 2.0, b.cy(), e) - h / (2.0 * s));
            let place = Place { pix: &pl.pix, s: s * f, tx: (pl.ox - vx) * s, ty: (pl.oy - vy) * s, alpha: 1.0 };
            out.push(self.frame(&[place])?);
        }
        // The node box on the screen at the end of the zoom.
        let v1x = b.cx() - w / (2.0 * s1);
        let v1y = b.cy() - h / (2.0 * s1);
        let b1 = Box2 { x: (b.x - v1x) * s1, y: (b.y - v1y) * s1, w: b.w * s1, h: b.h * s1 };
        let g0 = (b1.w / cw).min(b1.h / ch);
        let (rcx, rcy) = (cl.ox + cw / 2.0, cl.oy + ch / 2.0);
        let n = self.n(GROW_MS);
        for i in 0..n {
            let e = ease((i + 1) as f32 / n as f32);
            let g = g0.powf(1.0 - e);
            let (dx, dy) = (lerp(b1.cx(), rcx, e), lerp(b1.cy(), rcy, e));
            // The parent zooms with the child, so the node box stays under it.
            let sp = s1 * g / g0;
            let parent = Place {
                pix: &pl.pix,
                s: sp * f,
                tx: dx - (b.cx() - pl.ox) * sp,
                ty: dy - (b.cy() - pl.oy) * sp,
                alpha: 1.0 - e,
            };
            let child = Place { pix: &cl.pix, s: g * cl.fit, tx: dx - cw * g / 2.0, ty: dy - ch * g / 2.0, alpha: e };
            out.push(self.frame(&[parent, child])?);
        }
        Ok(out)
    }

    fn visit(&self, p: usize, out: &mut Vec<Pixmap>) -> Result<(), String> {
        for &(c, node) in &self.levels[p].kids {
            let zoom = self.zoom_in(p, c, node)?;
            out.extend(zoom.iter().cloned());
            self.hold(c, HOLD_MS, out)?;
            self.visit(c, out)?;
            out.extend(zoom.into_iter().rev());
            self.hold(p, BACK_MS, out)?;
        }
        Ok(())
    }
}

/// The path of the parent level, or `None` for the top level.
fn parent_of(path: &str) -> Option<&str> {
    if path.is_empty() {
        return None;
    }
    Some(path.rsplit_once('/').map_or("", |(a, _)| a))
}

/// The frames of the tour of all levels of `r`.
pub fn tour_frames(r: &Rendered, o: &TourOptions) -> Result<Tour, String> {
    if r.levels.len() < 2 {
        return Err("--tour needs a diagram with a detail level: a 'detail' block or a '>file.sd' link".into());
    }
    let svgs: Vec<String> = r.levels.iter().map(|l| apply_theme(&l.svg, o.theme)).collect();
    let mut levels: Vec<Level> = Vec::with_capacity(svgs.len());
    for svg in &svgs {
        levels.push(Level { pix: rasterize(svg, o.scale)?, fit: 1.0, ox: 0.0, oy: 0.0, kids: Vec::new() });
    }
    // The children of each level, in the order of the levels: the source order.
    for (c, lv) in r.levels.iter().enumerate() {
        let Some(pp) = parent_of(&lv.path) else { continue };
        let Some(p) = r.levels.iter().position(|l| l.path == pp) else { continue };
        levels[p].kids.push((c, None));
    }
    for (p, lv) in levels.iter_mut().enumerate() {
        let ids: Vec<&str> =
            lv.kids.iter().map(|&(c, _)| r.levels[c].path.rsplit('/').next().unwrap_or_default()).collect();
        let boxes = node_boxes(&svgs[p], &ids)?;
        for (kid, bx) in lv.kids.iter_mut().zip(boxes) {
            kid.1 = bx.map(|(x, y, w, h)| Box2 { x: x * o.scale, y: y * o.scale, w: w * o.scale, h: h * o.scale });
        }
    }
    // The top level fills the canvas, so the first frame has no margin.
    let (w, h) = (levels[0].pix.width(), levels[0].pix.height());
    for lv in &mut levels {
        let (pw, ph) = (lv.pix.width() as f32, lv.pix.height() as f32);
        lv.fit = (w as f32 / pw).min(h as f32 / ph).min(1.0);
        lv.ox = ((w as f32 - pw * lv.fit) / 2.0).round();
        lv.oy = ((h as f32 - ph * lv.fit) / 2.0).round();
    }
    // The page colour is the pixel at the middle of the top row, inside the
    // round corners, as for a video.
    let top = &levels[0].pix;
    let bg = top.pixel(top.width() / 2, 0).map_or(Color::WHITE, |c| {
        let c = c.demultiply();
        Color::from_rgba8(c.red(), c.green(), c.blue(), c.alpha())
    });
    let ctx = Ctx { levels: &levels, w, h, bg, fps: o.fps.max(1), speed: o.speed };
    let mut frames = Vec::new();
    ctx.hold(0, HOLD_TOP_MS, &mut frames)?;
    ctx.visit(0, &mut frames)?;
    Ok(Tour { frames, levels: levels.len() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ease_starts_and_ends_at_rest() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert_eq!(ease(0.5), 0.5);
        assert!(ease(0.1) < 0.1);
    }

    #[test]
    fn the_parent_path_drops_the_last_id() {
        assert_eq!(parent_of(""), None);
        assert_eq!(parent_of("cart"), Some(""));
        assert_eq!(parent_of("cart/svc"), Some("cart"));
    }
}
