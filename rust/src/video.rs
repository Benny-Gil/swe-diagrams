//! Frames to a GIF file, or to a video file with ffmpeg.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use color_quant::NeuQuant;
use gif::{DisposalMethod, Encoder, Frame, Repeat};

use crate::tiny_skia::Pixmap;

/// The palette index of a clear pixel, and of a pixel that does not change.
const CLEAR: u8 = 255;

/// The colour of a pixel without the premultiplied alpha, or `None` for a
/// clear pixel.
fn straight(p: &[u8]) -> Option<[u8; 3]> {
    let a = u32::from(p[3]);
    if a < 128 {
        return None;
    }
    let un = |c: u8| ((u32::from(c) * 255 + a / 2) / a).min(255) as u8;
    Some([un(p[0]), un(p[1]), un(p[2])])
}

/// One palette for all frames. A picture with 255 colours or fewer keeps
/// them all. Else NeuQuant learns the palette from a sample of the pixels.
fn palette(frames: &[Pixmap]) -> (Vec<[u8; 3]>, Option<NeuQuant>) {
    let mut seen: Vec<[u8; 3]> = Vec::new();
    let mut set: HashMap<[u8; 3], ()> = HashMap::new();
    let mut many = false;
    'outer: for f in frames {
        for p in f.data().as_chunks::<4>().0.iter() {
            if let Some(c) = straight(p) {
                if set.insert(c, ()).is_none() {
                    seen.push(c);
                    if seen.len() > 255 {
                        many = true;
                        break 'outer;
                    }
                }
            }
        }
    }
    if !many {
        return (seen, None);
    }
    // A step through the frames keeps the sample small for a long GIF.
    let step = frames.len().div_ceil(16);
    let mut sample: Vec<u8> = Vec::new();
    for f in frames.iter().step_by(step) {
        for p in f.data().as_chunks::<4>().0.iter().step_by(3) {
            if let Some(c) = straight(p) {
                sample.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
    }
    let nq = NeuQuant::new(10, 255, &sample);
    let pal: Vec<[u8; 3]> = nq.color_map_rgb().as_chunks::<3>().0.iter().map(|c| [c[0], c[1], c[2]]).collect();
    (pal, Some(nq))
}

/// The palette index of each pixel.
fn indices(f: &Pixmap, pal: &[[u8; 3]], nq: Option<&NeuQuant>, cache: &mut HashMap<[u8; 3], u8>) -> Vec<u8> {
    f.data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| match straight(p) {
            None => CLEAR,
            Some(c) => *cache.entry(c).or_insert_with(|| match nq {
                Some(nq) => nq.index_of(&[c[0], c[1], c[2], 255]) as u8,
                None => pal.iter().position(|&x| x == c).unwrap_or(0) as u8,
            }),
        })
        .collect()
}

/// A GIF that loops. Each frame after the first holds only the box of the
/// pixels that change, and the other pixels in the box are clear.
pub fn encode_gif(frames: &[Pixmap], fps: u32) -> Result<Vec<u8>, String> {
    let first = frames.first().ok_or("the GIF has no frames")?;
    let (w, h) = (first.width(), first.height());
    if frames.iter().any(|f| f.width() != w || f.height() != h) {
        return Err("the frames of the GIF have different sizes".into());
    }
    let (w16, h16) = (u16::try_from(w), u16::try_from(h));
    let (Ok(w16), Ok(h16)) = (w16, h16) else {
        return Err(format!("the GIF is too large: {w} x {h}; use a smaller --scale"));
    };
    let fps = fps.max(1);
    let (pal, nq) = palette(frames);
    let mut flat: Vec<u8> = pal.iter().flatten().copied().collect();
    flat.resize(256 * 3, 0);
    let mut out = Vec::new();
    {
        let mut enc = Encoder::new(&mut out, w16, h16, &flat).map_err(|e| format!("cannot write the GIF: {e}"))?;
        enc.set_repeat(Repeat::Infinite).map_err(|e| format!("cannot write the GIF: {e}"))?;
        let mut cache = HashMap::new();
        let mut prev: Option<Vec<u8>> = None;
        let wu = w as usize;
        for (i, f) in frames.iter().enumerate() {
            let idx = indices(f, &pal, nq.as_ref(), &mut cache);
            // The delays are in hundredths of a second. The sum of the
            // rounded times keeps the total time exact.
            let at = |k: usize| (k as u64 * 100 + u64::from(fps) / 2) / u64::from(fps);
            let delay = (at(i + 1) - at(i)) as u16;
            let mut frame = Frame {
                width: w16,
                height: h16,
                delay,
                dispose: DisposalMethod::Keep,
                transparent: Some(CLEAR),
                ..Frame::default()
            };
            match &prev {
                None => frame.buffer = idx.clone().into(),
                Some(p) => {
                    let changed: Vec<usize> = (0..idx.len()).filter(|&k| idx[k] != p[k]).collect();
                    let (mut x0, mut y0, mut x1, mut y1) = (wu, h as usize, 0, 0);
                    for &k in &changed {
                        let (x, y) = (k % wu, k / wu);
                        x0 = x0.min(x);
                        y0 = y0.min(y);
                        x1 = x1.max(x);
                        y1 = y1.max(y);
                    }
                    if changed.is_empty() {
                        (x0, y0, x1, y1) = (0, 0, 0, 0);
                    }
                    let mut buf = Vec::with_capacity((x1 - x0 + 1) * (y1 - y0 + 1));
                    for y in y0..=y1 {
                        for x in x0..=x1 {
                            let k = y * wu + x;
                            buf.push(if idx[k] == p[k] { CLEAR } else { idx[k] });
                        }
                    }
                    frame.left = x0 as u16;
                    frame.top = y0 as u16;
                    frame.width = (x1 - x0 + 1) as u16;
                    frame.height = (y1 - y0 + 1) as u16;
                    frame.buffer = buf.into();
                }
            }
            enc.write_frame(&frame).map_err(|e| format!("cannot write the GIF: {e}"))?;
            prev = Some(idx);
        }
    }
    Ok(out)
}

/// The RGB bytes of a frame on its page colour. The page colour is the
/// pixel at the middle of the top row, inside the round corners.
fn flat_rgb(f: &Pixmap, out: &mut Vec<u8>) {
    let d = f.data();
    let mid = (f.width() / 2) as usize * 4;
    let bg = [d[mid], d[mid + 1], d[mid + 2]];
    for p in d.as_chunks::<4>().0.iter() {
        let a = u32::from(p[3]);
        for c in 0..3 {
            // Premultiplied: colour + background * (1 - alpha).
            out.push((u32::from(p[c]) + (u32::from(bg[c]) * (255 - a) + 127) / 255).min(255) as u8);
        }
    }
}

/// Writes an MP4 or a WebM file with ffmpeg. The extension of `path`
/// selects the codec.
pub fn encode_video(frames: &[Pixmap], fps: u32, path: &Path) -> Result<(), String> {
    let first = frames.first().ok_or("the video has no frames")?;
    let webm = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("webm"));
    let codec: &[&str] = if webm {
        &["-c:v", "libvpx-vp9", "-b:v", "0", "-crf", "32"]
    } else {
        &["-c:v", "libx264", "-movflags", "+faststart"]
    };
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s"])
        .arg(format!("{}x{}", first.width(), first.height()))
        .args(["-framerate", &fps.max(1).to_string(), "-i", "-"])
        // The codecs need an even width and height.
        .args(["-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2", "-pix_fmt", "yuv420p"])
        .args(codec)
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "a video needs ffmpeg on the PATH; install ffmpeg, or write a GIF with -o out.gif".to_string()
        } else {
            format!("cannot start ffmpeg: {e}")
        }
    })?;
    let mut stdin = child.stdin.take().ok_or("cannot write to ffmpeg")?;
    let mut buf = Vec::new();
    let mut failed = false;
    for f in frames {
        if f.width() != first.width() || f.height() != first.height() {
            return Err("the frames of the video have different sizes".into());
        }
        buf.clear();
        flat_rgb(f, &mut buf);
        if stdin.write_all(&buf).is_err() {
            failed = true;
            break;
        }
    }
    drop(stdin);
    let res = child.wait_with_output().map_err(|e| format!("ffmpeg failed: {e}"))?;
    if failed || !res.status.success() {
        let msg = String::from_utf8_lossy(&res.stderr);
        return Err(format!("ffmpeg failed: {}", msg.trim()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_premultiplied_pixel_gets_its_colour_back() {
        assert_eq!(straight(&[100, 50, 0, 200]), Some([128, 64, 0]));
        assert_eq!(straight(&[10, 10, 10, 20]), None);
    }

    #[test]
    fn the_delays_add_up_to_the_time() {
        let mut a = Pixmap::new(4, 3).unwrap();
        a.fill(crate::tiny_skia::Color::WHITE);
        let mut b = a.clone();
        b.data_mut()[0] = 0;
        let frames = vec![a, b, a_copy(), a_copy(), a_copy(), a_copy()];
        let gif = encode_gif(&frames, 15).unwrap();
        let mut opts = gif::DecodeOptions::new();
        opts.set_color_output(gif::ColorOutput::Indexed);
        let mut dec = opts.read_info(gif.as_slice()).unwrap();
        let mut delays = Vec::new();
        while let Some(f) = dec.read_next_frame().unwrap() {
            delays.push(f.delay);
        }
        assert_eq!(delays, [7, 6, 7, 7, 6, 7]);
    }

    fn a_copy() -> Pixmap {
        let mut a = Pixmap::new(4, 3).unwrap();
        a.fill(crate::tiny_skia::Color::WHITE);
        a
    }
}
