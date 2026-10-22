//! Presentation-only camera zoom: magnify the composed gameplay view around a
//! smoothed camera, keep the status bar (and any other "HUD" cells) at their
//! normal place and size, and resample to any output size.
//!
//! Everything here works on an already composed RGBA frame (HD pack scale,
//! widescreen margins and all), so the game, the compositor and every HD rule
//! are untouched. Sampling is nearest-neighbour from the composed frame's own
//! pixels: at zoom `z` with `k` output pixels per NES pixel and an `s`x frame
//! (pack scale), each source pixel covers `k * z / s` output pixels. When that
//! is a whole number the camera is snapped to whole source pixels, so every
//! source pixel becomes an exact square block with no seams (e.g. a 4x pack at
//! zoom 2 in a 1440-row output: `6 * 2 / 4 = 3`, 3x3 blocks).

/// NES lines in a frame.
const LINES: f64 = 240.0;

/// Allowed zoom range.
pub const MIN_ZOOM: f32 = 1.0;
/// Allowed zoom range.
pub const MAX_ZOOM: f32 = 4.0;

/// Where the zoomed view looks, in NES pixels of the composed frame
/// (widescreen margins included: x = 0 is the left edge of the left margin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Magnification of the play area (1 = none).
    pub zoom: f32,
    /// Camera centre.
    pub center: [f32; 2],
    /// NES rows at the top drawn unzoomed (the status bar); 0 for none.
    pub hud_rows: u32,
}

/// A cell drawn unzoomed at its own screen position on top of the zoomed
/// view (the boss life meter): an 8-pixel-wide sprite cell at `(x, y)` (NES
/// pixels of the composed frame) whose opaque pixels are the set bits of
/// `rows[r]` (bit 7 = left column).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudCell {
    pub x: i32,
    pub y: i32,
    pub rows: [u8; 16],
}

/// The composed source frame.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    pub rgba: &'a [u8],
    pub width: usize,
    pub height: usize,
    /// Source pixels per NES pixel (the effective present scale).
    pub scale: u32,
}

/// Output pixels per source pixel in the zoomed play area for an output
/// `dst_h` rows tall.
#[must_use]
pub fn block_size(scale: u32, dst_h: usize, zoom: f32) -> f64 {
    dst_h as f64 / LINES * f64::from(zoom) / f64::from(scale.max(1))
}

/// The zoomed play area's top-left corner in NES pixels, after clamping to
/// the composed frame and, when [`block_size`] is whole, snapping to whole
/// source pixels. Returns `(left, top, view width, view height)`.
#[must_use]
pub fn view_rect(src: &Source<'_>, dst: (usize, usize), view: &View) -> (f64, f64, f64, f64) {
    let s = f64::from(src.scale.max(1));
    let fw = src.width as f64 / s;
    let k = dst.1 as f64 / LINES;
    let kz = k * f64::from(view.zoom.clamp(MIN_ZOOM, MAX_ZOOM));
    let hud = f64::from(view.hud_rows);
    let vw = dst.0 as f64 / kz;
    let vh = (dst.1 as f64 - hud * k) / kz;
    let clamp = |c: f64, lo: f64, hi: f64, size: f64| {
        if size >= hi - lo {
            lo + (hi - lo - size) / 2.0
        } else {
            (c - size / 2.0).clamp(lo, hi - size)
        }
    };
    let mut left = clamp(f64::from(view.center[0]), 0.0, fw, vw);
    let mut top = clamp(f64::from(view.center[1]), hud, LINES, vh);
    let block = block_size(src.scale, dst.1, view.zoom);
    if (block - block.round()).abs() < 1e-9 {
        // Whole blocks: start on a whole source pixel so the grid is exact.
        left = (left * s).round() / s;
        top = (top * s).round() / s;
    }
    (left, top, vw, vh)
}

/// Resample `src` into `dst` (`dst_w x dst_h` RGBA): rows above the HUD line
/// and every [`HudCell`] unzoomed (`k = dst_h / 240` output pixels per NES
/// pixel, centred horizontally), the play area zoomed around `view`. Pixels
/// that fall outside the composed frame are black.
pub fn resample(
    src: &Source<'_>,
    dst: &mut [u8],
    dst_w: usize,
    dst_h: usize,
    view: &View,
    hud_cells: &[HudCell],
) {
    let s = f64::from(src.scale.max(1));
    let fw = src.width as f64 / s;
    let k = dst_h as f64 / LINES;
    let kz = k * f64::from(view.zoom.clamp(MIN_ZOOM, MAX_ZOOM));
    // Unzoomed mapping: the whole frame centred.
    let left1 = (fw - dst_w as f64 / k) / 2.0;
    let (left, top, _, _) = view_rect(src, (dst_w, dst_h), view);
    let hud_px = (f64::from(view.hud_rows) * k).round() as usize;
    // Source column per output column for both mappings (None: outside).
    let column = |origin: f64, per: f64, ox: usize| -> Option<usize> {
        let x = ((origin * s) + (ox as f64 * s) / per + 1e-9).floor();
        (x >= 0.0 && (x as usize) < src.width).then_some(x as usize)
    };
    let cols1: Vec<Option<usize>> = (0..dst_w).map(|ox| column(left1, k, ox)).collect();
    let colsz: Vec<Option<usize>> = (0..dst_w).map(|ox| column(left, kz, ox)).collect();
    for oy in 0..dst_h {
        let (cols, sy) = if oy < hud_px {
            (&cols1, ((oy as f64 * s) / k + 1e-9).floor())
        } else {
            (
                &colsz,
                (top * s + ((oy - hud_px) as f64 * s) / kz + 1e-9).floor(),
            )
        };
        let row = &mut dst[oy * dst_w * 4..(oy + 1) * dst_w * 4];
        let sy = (sy >= 0.0 && (sy as usize) < src.height).then_some(sy as usize);
        for (px, col) in row.as_chunks_mut::<4>().0.iter_mut().zip(cols) {
            *px = match (sy, col) {
                (Some(y), Some(x)) => {
                    let i = (y * src.width + x) * 4;
                    [src.rgba[i], src.rgba[i + 1], src.rgba[i + 2], 0xFF]
                }
                _ => [0, 0, 0, 0xFF],
            };
        }
    }
    // HUD cells on top, unzoomed, at their own place.
    for cell in hud_cells {
        let x0 = ((f64::from(cell.x) - left1) * k).floor().max(0.0) as usize;
        let x1 = (((f64::from(cell.x) + 8.0 - left1) * k).ceil().max(0.0) as usize).min(dst_w);
        let y0 = (f64::from(cell.y) * k).floor().max(0.0) as usize;
        let y1 = (((f64::from(cell.y) + 16.0) * k).ceil().max(0.0) as usize).min(dst_h);
        for oy in y0..y1 {
            let ny = (oy as f64 / k + 1e-9).floor() as i32 - cell.y;
            let sy = ((oy as f64 * s) / k + 1e-9).floor() as usize;
            if !(0..16).contains(&ny) || sy >= src.height {
                continue;
            }
            for (ox, col) in cols1.iter().enumerate().take(x1).skip(x0) {
                let Some(sx) = *col else { continue };
                let nx = (sx as f64 / s).floor() as i32 - cell.x;
                if !(0..8).contains(&nx) || cell.rows[ny as usize] & (0x80 >> nx) == 0 {
                    continue;
                }
                let i = (sy * src.width + sx) * 4;
                let o = (oy * dst_w + ox) * 4;
                dst[o..o + 3].copy_from_slice(&src.rgba[i..i + 3]);
                dst[o + 3] = 0xFF;
            }
        }
    }
}

/// Smoothed camera: follows a target with a deadzone and eased catch-up, and
/// eases the zoom in and out.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Camera {
    center: Option<[f32; 2]>,
    zoom: f32,
}

/// Half-size of the deadzone around the camera centre, in NES pixels:
/// targets inside it do not move the camera (walk bob, small hops).
pub const DEADZONE: [f32; 2] = [12.0, 24.0];
/// Fraction of the remaining distance (beyond the deadzone) covered per
/// emulated frame.
pub const FOLLOW: f32 = 0.18;
/// Zoom eased per emulated frame (fraction of the remaining difference).
pub const ZOOM_EASE: f32 = 0.15;

impl Camera {
    /// Current camera centre (`None` before the first target).
    #[must_use]
    pub fn center(&self) -> Option<[f32; 2]> {
        self.center
    }

    /// Current (eased) zoom; 1 before the first update.
    #[must_use]
    pub fn zoom(&self) -> f32 {
        if self.zoom < MIN_ZOOM {
            MIN_ZOOM
        } else {
            self.zoom
        }
    }

    /// Jump straight to `target` and `zoom` (first frame, a new room or
    /// scene, a loaded state).
    pub fn snap(&mut self, target: [f32; 2], zoom: f32) {
        self.center = Some(target);
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// One emulated frame toward `target` (keep still when `None`) and
    /// `zoom`.
    pub fn follow(&mut self, target: Option<[f32; 2]>, zoom: f32) {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        if self.zoom < MIN_ZOOM {
            self.zoom = zoom;
        }
        self.zoom += (zoom - self.zoom) * ZOOM_EASE;
        if (self.zoom - zoom).abs() < 0.01 {
            self.zoom = zoom;
        }
        let Some(t) = target else { return };
        let Some(c) = self.center.as_mut() else {
            self.center = Some(t);
            return;
        };
        for axis in 0..2 {
            let d = t[axis] - c[axis];
            let over = d.abs() - DEADZONE[axis];
            if over > 0.0 {
                c[axis] += d.signum() * over * FOLLOW;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: usize, h: usize) -> Vec<u8> {
        // Each pixel encodes its own coordinates.
        let mut v = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                v[i] = x as u8;
                v[i + 1] = y as u8;
                v[i + 2] = (x >> 8) as u8 | ((y >> 8) as u8) << 4;
                v[i + 3] = 0xFF;
            }
        }
        v
    }

    fn at(v: &[u8], w: usize, x: usize, y: usize) -> (usize, usize) {
        let i = (y * w + x) * 4;
        (
            usize::from(v[i]) | (usize::from(v[i + 2] & 15) << 8),
            usize::from(v[i + 1]) | (usize::from(v[i + 2] >> 4) << 8),
        )
    }

    #[test]
    fn zoom_one_without_resize_is_the_identity() {
        let (w, h) = (256 * 2, 240 * 2);
        let src = frame(w, h);
        let s = Source {
            rgba: &src,
            width: w,
            height: h,
            scale: 2,
        };
        let mut out = vec![0; w * h * 4];
        let view = View {
            zoom: 1.0,
            center: [128.0, 120.0],
            hud_rows: 32,
        };
        resample(&s, &mut out, w, h, &view, &[]);
        assert!(out == src, "identity");
    }

    #[test]
    fn zoom_two_maps_each_source_pixel_to_an_exact_block() {
        // A 4x 16:9 widescreen frame (432 x 240 NES px) into 2560 x 1440 at
        // zoom 2: 3 x 3 output pixels per source pixel.
        let (w, h) = (432 * 4, 240 * 4);
        let src = frame(w, h);
        let s = Source {
            rgba: &src,
            width: w,
            height: h,
            scale: 4,
        };
        let (dw, dh) = (2560, 1440);
        assert_eq!(block_size(4, dh, 2.0), 3.0);
        let mut out = vec![0; dw * dh * 4];
        let view = View {
            zoom: 2.0,
            center: [201.37, 150.6],
            hud_rows: 32,
        };
        resample(&s, &mut out, dw, dh, &view, &[]);
        let hud = 32 * 6;
        for oy in (hud..dh).step_by(7) {
            for ox in (0..dw - 3).step_by(5) {
                let a = at(&out, dw, ox, oy);
                // Blocks start on multiples of 3 from the left and HUD edges.
                let bx = ox - ox % 3;
                let by = oy - (oy - hud) % 3;
                assert_eq!(a, at(&out, dw, bx, by), "({ox}, {oy})");
                if ox % 3 == 2 {
                    assert_ne!(a, at(&out, dw, ox + 1, oy), "seam at ({ox}, {oy})");
                }
            }
        }
        // HUD rows are unzoomed: 6 output px per NES px, centred.
        let (x, y) = at(&out, dw, dw / 2, 3 * 6);
        assert_eq!(y / 4, 3);
        assert!((x as i32 / 4 - 216).abs() <= 1);
    }

    #[test]
    fn the_view_is_clamped_inside_the_composed_frame() {
        let (w, h) = (256, 240);
        let src = frame(w, h);
        let s = Source {
            rgba: &src,
            width: w,
            height: h,
            scale: 1,
        };
        let mut out = vec![0; w * h * 4];
        let view = View {
            zoom: 2.0,
            center: [0.0, 239.0],
            hud_rows: 32,
        };
        let (left, top, vw, vh) = view_rect(&s, (w, h), &view);
        assert_eq!((left, top), (0.0, 240.0 - vh));
        assert_eq!(vw, 128.0);
        resample(&s, &mut out, w, h, &view, &[]);
        assert_eq!(at(&out, w, 0, 32), (0, (240.0 - vh) as usize));
        assert_eq!(at(&out, w, w - 1, h - 1), (127, 239));
    }

    #[test]
    fn hud_cells_stay_unzoomed_and_only_their_opaque_pixels_are_drawn() {
        let (w, h) = (256, 240);
        let src = frame(w, h);
        let s = Source {
            rgba: &src,
            width: w,
            height: h,
            scale: 1,
        };
        let mut out = vec![0; w * h * 4];
        let mut rows = [0u8; 16];
        rows[0] = 0b1000_0001;
        let cell = HudCell { x: 16, y: 62, rows };
        let view = View {
            zoom: 2.0,
            center: [200.0, 150.0],
            hud_rows: 32,
        };
        resample(&s, &mut out, w, h, &view, &[cell]);
        assert_eq!(at(&out, w, 16, 62), (16, 62));
        assert_eq!(at(&out, w, 23, 62), (23, 62));
        assert_ne!(at(&out, w, 17, 62), (17, 62), "transparent: zoomed world");
    }

    #[test]
    fn the_camera_ignores_small_moves_and_eases_after_big_ones() {
        let mut c = Camera::default();
        c.snap([100.0, 100.0], 2.0);
        c.follow(Some([108.0, 110.0]), 2.0);
        assert_eq!(c.center(), Some([100.0, 100.0]), "inside the deadzone");
        c.follow(Some([150.0, 100.0]), 2.0);
        let x = c.center().unwrap()[0];
        assert!(x > 100.0 && x < 110.0, "eased, not jumped: {x}");
        for _ in 0..200 {
            c.follow(Some([150.0, 100.0]), 1.0);
        }
        let x = c.center().unwrap()[0];
        assert!((x - (150.0 - DEADZONE[0])).abs() < 0.5, "{x}");
        assert_eq!(c.zoom(), 1.0);
    }
}
