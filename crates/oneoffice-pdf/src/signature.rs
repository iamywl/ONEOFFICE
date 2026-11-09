//! Electronic signature, traditional Korean seal (`직인` / `도장`), and chroma-key matting engines.
//!
//! Provides:
//! 1. [`FreehandBezierSignature`]: Ramer-Douglas-Peucker simplification + smooth cubic Bezier
//!    fitting (`kurbo::BezPath`) + velocity/pressure-sensitive variable-width fountain-pen ribbons,
//!    renderable both as vector Bezier paths and supersampled anti-aliased [`RgbaStamp`] bitmaps.
//! 2. [`ChromaKeyAlphaMatting`] / [`remove_white_background`]: corner paper white-point estimation,
//!    luminance + chroma foreground ink intensity, cubic Hermite alpha ramp, and background color
//!    decontamination (zero white halo on vermilion Korean seal ink).
//! 3. [`QuickSignText`]: shaped Korean/Unicode text stamp rendered via `krilla` + `hayro`.
//! 4. [`SignatureLayerStack`]: interactive placement, anchor alignment, move/resize/rotate/opacity,
//!    and bounded Undo/Redo snapshot stack.

use std::io::Cursor;

use base64::Engine as _;
use craft_pdf::raster::{Options as RasterOptions, RenderCache};
use craft_pdf::write::{Metadata, Writer};
use image::{ExtendedColorType, ImageEncoder, ImageFormat, codecs::png::PngEncoder};
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point as KrillaPoint};
use krilla::page::PageSettings;
use krilla::paint::{Fill, Paint, Stroke};
use krilla::text::TextDirection;
use kurbo::{Affine, BezPath, ParamCurve, PathEl, Point, Vec2};
use serde::{Deserialize, Serialize};

use crate::PdfError;
use crate::font_fallback::{build_interpreter_settings, resolve_krilla_font};

/// Maximum Undo/Redo history snapshots retained per document session.
const MAX_UNDO_HISTORY: usize = 128;

/// Single sampled pointer event on a signature pad.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignaturePoint {
    /// X coordinate in signature canvas points (origin top-left).
    pub x: f32,
    /// Y coordinate in signature canvas points (origin top-left).
    pub y: f32,
    /// Monotonic event timestamp in milliseconds (used for velocity-sensitive stroke width).
    pub timestamp_ms: f64,
    /// Optional stylus pressure in `0.0..=1.0`.
    pub pressure: Option<f32>,
}

impl SignaturePoint {
    /// Construct a pointer sample without explicit stylus pressure.
    #[must_use]
    pub const fn new(x: f32, y: f32, timestamp_ms: f64) -> Self {
        Self {
            x,
            y,
            timestamp_ms,
            pressure: None,
        }
    }

    /// Construct a pointer sample with explicit stylus pressure (`0.0..=1.0`).
    #[must_use]
    pub const fn with_pressure(x: f32, y: f32, timestamp_ms: f64, pressure: f32) -> Self {
        Self {
            x,
            y,
            timestamp_ms,
            pressure: Some(pressure),
        }
    }

    #[must_use]
    fn to_kurbo(self) -> Point {
        Point::new(f64::from(self.x), f64::from(self.y))
    }
}

/// A single continuous pen-down to pen-up stroke.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FreehandStroke {
    /// Raw or pre-filtered pointer trajectory.
    pub points: Vec<SignaturePoint>,
    /// Nominal stroke width in points at medium writing speed.
    pub base_width_pt: f32,
}

impl FreehandStroke {
    /// Create a freehand stroke with a nominal stroke width in points.
    #[must_use]
    pub fn new(points: Vec<SignaturePoint>, base_width_pt: f32) -> Self {
        Self {
            points,
            base_width_pt: base_width_pt.clamp(0.4, 24.0),
        }
    }

    /// Simplify raw pointer samples via iterative Ramer-Douglas-Peucker (`epsilon` in points).
    #[must_use]
    pub fn simplify(&self, epsilon: f32) -> Vec<SignaturePoint> {
        let valid: Vec<SignaturePoint> = self
            .points
            .iter()
            .copied()
            .filter(|p| p.x.is_finite() && p.y.is_finite() && p.timestamp_ms.is_finite())
            .collect();
        rdp_simplify(&valid, f64::from(epsilon.max(0.05)))
    }

    /// Fit a smooth C1-continuous cubic Bezier centerline (`kurbo::BezPath`) through the stroke.
    #[must_use]
    pub fn to_centerline_bezier(&self, epsilon: f32) -> BezPath {
        let pts = self.simplify(epsilon);
        let mut path = BezPath::new();
        match pts.as_slice() {
            [] => {}
            [only] => {
                let c = only.to_kurbo();
                path.move_to(c);
                path.line_to(Point::new(c.x + 0.01, c.y));
            }
            [a, b] => {
                path.move_to(a.to_kurbo());
                path.line_to(b.to_kurbo());
            }
            many => {
                path.move_to(many[0].to_kurbo());
                let n = many.len();
                for i in 0..(n - 1) {
                    let p0 = if i == 0 {
                        many[0].to_kurbo()
                    } else {
                        many[i - 1].to_kurbo()
                    };
                    let p1 = many[i].to_kurbo();
                    let p2 = many[i + 1].to_kurbo();
                    let p3 = if i + 2 < n {
                        many[i + 2].to_kurbo()
                    } else {
                        many[n - 1].to_kurbo()
                    };
                    // Catmull-Rom to cubic Bezier conversion (tension = 1/6) with chord clamping
                    // to prevent overshoot loops on sharp corner turns.
                    let chord = (p2 - p1).hypot();
                    let mut t1 = (p2 - p0) * (1.0 / 6.0);
                    let mut t2 = (p3 - p1) * (1.0 / 6.0);
                    let max_handle = chord * 0.45;
                    if t1.hypot() > max_handle && t1.hypot() > 1e-6 {
                        t1 *= max_handle / t1.hypot();
                    }
                    if t2.hypot() > max_handle && t2.hypot() > 1e-6 {
                        t2 *= max_handle / t2.hypot();
                    }
                    path.curve_to(p1 + t1, p2 - t2, p2);
                }
            }
        }
        path
    }

    /// Build a closed vector ribbon outline (`kurbo::BezPath`) whose half-width modulates
    /// smoothly with writing velocity and optional stylus pressure.
    #[must_use]
    pub fn to_variable_width_ribbon(&self, epsilon: f32) -> BezPath {
        let pts = self.simplify(epsilon);
        let base_r = (f64::from(self.base_width_pt.max(0.4)) * 0.5).max(0.2);
        if pts.is_empty() {
            return BezPath::new();
        }
        if pts.len() == 1 {
            return circle_bezpath(pts[0].to_kurbo(), base_r);
        }

        // Compute target half-width at each simplified knot from velocity & pressure.
        let mut knot_radii = Vec::with_capacity(pts.len());
        for i in 0..pts.len() {
            let speed_pt_per_ms = if i == 0 {
                segment_speed(&pts[0], &pts[1])
            } else if i + 1 == pts.len() {
                segment_speed(&pts[i - 1], &pts[i])
            } else {
                0.5 * (segment_speed(&pts[i - 1], &pts[i]) + segment_speed(&pts[i], &pts[i + 1]))
            };
            // Slow deliberate strokes -> ~1.30x width; brisk flicks -> ~0.55x width.
            let norm_speed = (speed_pt_per_ms / 1.4).clamp(0.0, 1.0);
            let vel_factor = 1.30 - 0.75 * norm_speed;
            let pressure_factor = pts[i]
                .pressure
                .map_or(1.0, |p| 0.55 + 0.90 * f64::from(p.clamp(0.0, 1.0)));
            let r = (base_r * vel_factor * pressure_factor).clamp(base_r * 0.35, base_r * 1.65);
            knot_radii.push(r);
        }

        // Sample cubic Bezier segments + interpolated radii into dense spine stations.
        let mut stations: Vec<(Point, f64)> = Vec::new();
        let n = pts.len();
        for i in 0..(n - 1) {
            let p0 = if i == 0 {
                pts[0].to_kurbo()
            } else {
                pts[i - 1].to_kurbo()
            };
            let p1 = pts[i].to_kurbo();
            let p2 = pts[i + 1].to_kurbo();
            let p3 = if i + 2 < n {
                pts[i + 2].to_kurbo()
            } else {
                pts[n - 1].to_kurbo()
            };
            let chord = (p2 - p1).hypot();
            let max_handle = chord * 0.45;
            let mut t1 = (p2 - p0) * (1.0 / 6.0);
            let mut t2 = (p3 - p1) * (1.0 / 6.0);
            if t1.hypot() > max_handle && t1.hypot() > 1e-6 {
                t1 *= max_handle / t1.hypot();
            }
            if t2.hypot() > max_handle && t2.hypot() > 1e-6 {
                t2 *= max_handle / t2.hypot();
            }
            let seg = kurbo::CubicBez::new(p1, p1 + t1, p2 - t2, p2);
            let steps = ((chord / 1.25).ceil() as usize).clamp(2, 24);
            let r_start = knot_radii[i];
            let r_end = knot_radii[i + 1];
            let start_step = usize::from(i > 0);
            for s in start_step..=steps {
                let t = (s as f64) / (steps as f64);
                let pos = seg.eval(t);
                let radius = r_start + (r_end - r_start) * t;
                if stations
                    .last()
                    .is_none_or(|&(prev, _)| (pos - prev).hypot() > 0.05)
                {
                    stations.push((pos, radius));
                }
            }
        }

        if stations.len() < 2 {
            return circle_bezpath(pts[0].to_kurbo(), knot_radii[0]);
        }

        // Construct left and right offset rails + semicircular endcaps.
        let m = stations.len();
        let mut left = Vec::with_capacity(m);
        let mut right = Vec::with_capacity(m);
        for i in 0..m {
            let tangent = if i == 0 {
                stations[1].0 - stations[0].0
            } else if i + 1 == m {
                stations[m - 1].0 - stations[m - 2].0
            } else {
                stations[i + 1].0 - stations[i - 1].0
            };
            let len = tangent.hypot().max(1e-6);
            let unit = tangent / len;
            let normal = Vec2::new(-unit.y, unit.x);
            let r = stations[i].1;
            left.push(stations[i].0 + normal * r);
            right.push(stations[i].0 - normal * r);
        }

        let mut ribbon = BezPath::new();
        ribbon.move_to(left[0]);
        for &pt in &left[1..] {
            ribbon.line_to(pt);
        }
        // End cap around `stations[m - 1]` from `left[m - 1]` to `right[m - 1]`.
        append_semicircle_cap(&mut ribbon, stations[m - 1].0, left[m - 1], right[m - 1]);
        for &pt in right.iter().rev().skip(1) {
            ribbon.line_to(pt);
        }
        // Start cap around `stations[0]` from `right[0]` back to `left[0]`.
        append_semicircle_cap(&mut ribbon, stations[0].0, right[0], left[0]);
        ribbon.close_path();
        ribbon
    }
}

/// Complete multi-stroke freehand signature captured from a signature pad.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FreehandBezierSignature {
    /// Ordered pen strokes.
    pub strokes: Vec<FreehandStroke>,
    /// Straight RGBA8 ink color (e.g. `[15, 23, 42, 255]` black-navy, `[29, 78, 216, 255]` blue).
    pub ink_color: [u8; 4],
    /// Logical signature pad size `[width_pt, height_pt]`.
    pub canvas_size_pt: [f32; 2],
    /// RDP simplification tolerance in points.
    pub simplify_epsilon: f32,
}

impl FreehandBezierSignature {
    /// Create a freehand Bezier signature over `canvas_size_pt`.
    #[must_use]
    pub fn new(strokes: Vec<FreehandStroke>, ink_color: [u8; 4], canvas_size_pt: [f32; 2]) -> Self {
        Self {
            strokes,
            ink_color,
            canvas_size_pt: [canvas_size_pt[0].max(8.0), canvas_size_pt[1].max(8.0)],
            simplify_epsilon: 0.45,
        }
    }

    /// Return smooth cubic Bezier centerlines for every stroke.
    #[must_use]
    pub fn centerline_paths(&self) -> Vec<BezPath> {
        self.strokes
            .iter()
            .map(|s| s.to_centerline_bezier(self.simplify_epsilon))
            .filter(|p| !p.elements().is_empty())
            .collect()
    }

    /// Return closed variable-width vector fountain-pen ribbon outlines for every stroke.
    #[must_use]
    pub fn variable_width_ribbon_paths(&self) -> Vec<BezPath> {
        self.strokes
            .iter()
            .map(|s| s.to_variable_width_ribbon(self.simplify_epsilon))
            .filter(|p| !p.elements().is_empty())
            .collect()
    }

    /// Compute tight bounding box `[x, y, w, h]` in canvas points across all strokes.
    #[must_use]
    pub fn bounding_box_pt(&self) -> Option<[f32; 4]> {
        let mut union_rect: Option<kurbo::Rect> = None;
        for path in self.variable_width_ribbon_paths() {
            let bb = kurbo::Shape::bounding_box(&path);
            if bb.width() > 0.0 || bb.height() > 0.0 {
                union_rect = Some(match union_rect {
                    Some(acc) => acc.union(bb),
                    None => bb,
                });
            }
        }
        union_rect.map(|r| {
            [
                r.x0 as f32,
                r.y0 as f32,
                r.width().max(1.0) as f32,
                r.height().max(1.0) as f32,
            ]
        })
    }

    /// Rasterize the vector Bezier ribbons into a supersampled, straight-alpha [`RgbaStamp`].
    pub fn render_rgba_stamp(&self, pixels_per_pt: f32) -> Result<RgbaStamp, PdfError> {
        let ppp = pixels_per_pt.clamp(0.5, 8.0);
        let width = ((self.canvas_size_pt[0] * ppp).ceil() as u32).clamp(1, 4096);
        let height = ((self.canvas_size_pt[1] * ppp).ceil() as u32).clamp(1, 4096);
        let scale = Affine::scale(f64::from(ppp));
        let ribbons: Vec<BezPath> = self
            .variable_width_ribbon_paths()
            .into_iter()
            .map(|mut p| {
                p.apply_affine(scale);
                p
            })
            .collect();

        let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
        rasterize_filled_bezpaths_rgba(&mut rgba, width, height, &ribbons, self.ink_color);
        RgbaStamp::new(width, height, rgba)
    }
}

/// Straight (unpremultiplied) RGBA8 stamp bitmap used for transparent PNG seals,
/// chroma-key matted stamps, and live viewport compositing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbaStamp {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Straight RGBA8 pixel buffer (`width * height * 4` bytes).
    pub rgba: Vec<u8>,
}

impl RgbaStamp {
    /// Construct a validated [`RgbaStamp`].
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, PdfError> {
        if width == 0 || height == 0 {
            return Err(PdfError::InvalidDimensions { width, height });
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|px| px.checked_mul(4))
            .ok_or(PdfError::InvalidDimensions { width, height })?;
        if rgba.len() != expected {
            return Err(PdfError::BufferLengthMismatch {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    /// Decode a PNG, JPEG, or WebP byte slice into a straight-alpha [`RgbaStamp`].
    pub fn from_encoded_image(image_bytes: &[u8]) -> Result<Self, PdfError> {
        let dyn_img = image::load_from_memory(image_bytes)
            .map_err(|e| PdfError::ImageDecode(e.to_string()))?;
        let rgba8 = dyn_img.to_rgba8();
        let (width, height) = rgba8.dimensions();
        Self::new(width, height, rgba8.into_raw())
    }

    /// Read pixel `(x, y)` as `[r, g, b, a]`.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        let slice = self.rgba.get(idx..idx + 4)?;
        Some([slice[0], slice[1], slice[2], slice[3]])
    }

    /// Encode this stamp losslessly as PNG bytes.
    pub fn to_png_bytes(&self) -> Result<Vec<u8>, PdfError> {
        let mut out = Vec::new();
        let encoder = PngEncoder::new(&mut out);
        encoder
            .write_image(
                &self.rgba,
                self.width,
                self.height,
                ExtendedColorType::Rgba8,
            )
            .map_err(|e| PdfError::ImageEncode(e.to_string()))?;
        Ok(out)
    }

    /// Encode this stamp as a base64 PNG string (ready for Webview/MCP transport).
    pub fn to_base64_png(&self) -> Result<String, PdfError> {
        let png = self.to_png_bytes()?;
        Ok(base64::engine::general_purpose::STANDARD.encode(png))
    }

    /// Split into planar RGB (`width * height * 3`) and grayscale soft-mask alpha (`width * height`)
    /// streams for PDF `/Image` + `/SMask` embedding.
    ///
    /// Fully transparent pixels are normalized to neutral ink `[255, 255, 255]` in the RGB plane
    /// so viewers with bilinear `/SMask` sampling never bleed dark halos at transparent borders.
    #[must_use]
    pub fn split_rgb_and_alpha(&self) -> (Vec<u8>, Vec<u8>) {
        let px_count = (self.width as usize) * (self.height as usize);
        let mut rgb = Vec::with_capacity(px_count * 3);
        let mut alpha = Vec::with_capacity(px_count);
        for px in self.rgba.chunks_exact(4) {
            if px[3] == 0 {
                rgb.extend_from_slice(&[255, 255, 255]);
                alpha.push(0);
            } else {
                rgb.extend_from_slice(&[px[0], px[1], px[2]]);
                alpha.push(px[3]);
            }
        }
        (rgb, alpha)
    }

    /// Crop redundant outer `alpha == 0` margins while keeping `padding_px` border pixels.
    #[must_use]
    pub fn trim_transparent_margins(&self, padding_px: u32) -> Self {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        let mut any = false;

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
                if self.rgba[idx + 3] > 0 {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    any = true;
                }
            }
        }
        if !any {
            return self.clone();
        }
        let x0 = min_x.saturating_sub(padding_px);
        let y0 = min_y.saturating_sub(padding_px);
        let x1 = (max_x + 1 + padding_px).min(self.width);
        let y1 = (max_y + 1 + padding_px).min(self.height);
        let new_w = (x1 - x0).max(1);
        let new_h = (y1 - y0).max(1);
        let mut cropped = Vec::with_capacity((new_w as usize) * (new_h as usize) * 4);
        for y in y0..y1 {
            let row_start = ((y as usize) * (self.width as usize) + (x0 as usize)) * 4;
            let row_end = row_start + (new_w as usize) * 4;
            cropped.extend_from_slice(&self.rgba[row_start..row_end]);
        }
        Self {
            width: new_w,
            height: new_h,
            rgba: cropped,
        }
    }
}

/// Parameters controlling white-paper background removal (`ChromaKeyAlphaMatting`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChromaKeyOptions {
    /// Chroma boost factor $\gamma$ in $I = \max(L_{\text{bg}} - L,\; \gamma \cdot C)$.
    /// Ensures vivid vermilion Korean seal ink (`인주`) stays 100% opaque.
    pub chroma_weight: f32,
    /// Lower foreground intensity threshold $t_0$ below which pixels become `alpha = 0`.
    pub low_threshold: f32,
    /// Upper foreground intensity threshold $t_1$ above which pixels become `alpha = 255`.
    pub high_threshold: f32,
}

impl Default for ChromaKeyOptions {
    fn default() -> Self {
        Self {
            chroma_weight: 1.35,
            low_threshold: 0.08,
            high_threshold: 0.30,
        }
    }
}

/// White/gray paper background removal engine with edge color decontamination.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChromaKeyAlphaMatting {
    /// Tunable thresholds and chroma boost.
    pub options: ChromaKeyOptions,
}

impl ChromaKeyAlphaMatting {
    /// Create a matting engine with custom [`ChromaKeyOptions`].
    #[must_use]
    pub const fn new(options: ChromaKeyOptions) -> Self {
        Self { options }
    }

    /// Remove opaque white/off-white/light-gray paper background from an encoded image.
    pub fn process_encoded(&self, image_bytes: &[u8]) -> Result<RgbaStamp, PdfError> {
        let stamp = RgbaStamp::from_encoded_image(image_bytes)?;
        self.process_stamp(&stamp)
    }

    /// Remove opaque white/off-white/light-gray paper background from an [`RgbaStamp`].
    pub fn process_stamp(&self, input: &RgbaStamp) -> Result<RgbaStamp, PdfError> {
        let w = input.width;
        let h = input.height;
        let bg_rgb = estimate_corner_paper_background(input);
        let bg_norm = [
            f32::from(bg_rgb[0]) / 255.0,
            f32::from(bg_rgb[1]) / 255.0,
            f32::from(bg_rgb[2]) / 255.0,
        ];
        let l_bg = srgb_luminance(bg_norm[0], bg_norm[1], bg_norm[2]);

        let t0 = self.options.low_threshold.clamp(0.01, 0.85);
        let t1 = self.options.high_threshold.max(t0 + 0.02).min(0.99);
        let gamma = self.options.chroma_weight.max(0.5);

        let mut out = vec![0u8; input.rgba.len()];
        for (src, dst) in input.rgba.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
            let orig_a = f32::from(src[3]) / 255.0;
            if orig_a <= 0.001 {
                dst.copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let r = f32::from(src[0]) / 255.0;
            let g = f32::from(src[1]) / 255.0;
            let b = f32::from(src[2]) / 255.0;

            let lum = srgb_luminance(r, g, b);
            let chroma = (r - g).abs().max((g - b).abs()).max((r - b).abs());
            let intensity = (l_bg - lum).max(gamma * chroma);

            let matte_a = smoothstep(t0, t1, intensity) * orig_a;
            let alpha_u8 = (matte_a * 255.0).round().clamp(0.0, 255.0) as u8;
            if alpha_u8 == 0 {
                dst.copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }

            // Decontaminate paper background contribution from semi-transparent edge pixels:
            // P = a * F + (1 - a) * B  =>  F = (P - (1 - a) * B) / a
            let denom = matte_a.max(0.08);
            let inv_a = 1.0 - matte_a;
            let fg_r = ((r - inv_a * bg_norm[0]) / denom).clamp(0.0, 1.0);
            let fg_g = ((g - inv_a * bg_norm[1]) / denom).clamp(0.0, 1.0);
            let fg_b = ((b - inv_a * bg_norm[2]) / denom).clamp(0.0, 1.0);

            dst[0] = (fg_r * 255.0).round() as u8;
            dst[1] = (fg_g * 255.0).round() as u8;
            dst[2] = (fg_b * 255.0).round() as u8;
            dst[3] = alpha_u8;
        }

        RgbaStamp::new(w, h, out)
    }
}

/// Remove an opaque white/off-white/gray paper background from an encoded stamp/signature image
/// using default [`ChromaKeyAlphaMatting`] parameters (`docs/SPEC_PDF_VIEWER_AND_SIGNATURE.md` §3.2).
pub fn remove_white_background(image_bytes: &[u8]) -> Result<RgbaStamp, PdfError> {
    ChromaKeyAlphaMatting::default().process_encoded(image_bytes)
}

/// Generate a 100% clean-room realistic Korean vermilion seal (`직인` / `도장`) photographed on
/// opaque warm off-white paper (`[244, 243, 239]` with subtle paper grain), encoded as PNG bytes.
///
/// Designed for end-to-end verification of [`remove_white_background`] and PDF seal stamping
/// without external proprietary image assets.
pub fn generate_cleanroom_seal_on_white_paper_png(label: &str) -> Result<Vec<u8>, PdfError> {
    let size: u32 = 192;
    let mut rgba = vec![0u8; (size as usize) * (size as usize) * 4];

    // 1. Fill opaque off-white textured paper (`alpha = 255` everywhere).
    for y in 0..size {
        for x in 0..size {
            // Deterministic subtle paper fiber variation in [-3, +3].
            let hash = x
                .wrapping_mul(73_856_093)
                .wrapping_add(y.wrapping_mul(19_349_663));
            let grain = ((hash % 7) as i16) - 3;
            let idx = ((y as usize) * (size as usize) + (x as usize)) * 4;
            rgba[idx] = (244i16 + grain).clamp(0, 255) as u8;
            rgba[idx + 1] = (243i16 + grain).clamp(0, 255) as u8;
            rgba[idx + 2] = (239i16 + grain).clamp(0, 255) as u8;
            rgba[idx + 3] = 255;
        }
    }

    // 2. Construct traditional Korean circular corporate/personal seal geometry in vermilion ink.
    let center = Point::new(96.0, 96.0);
    let mut seal_paths: Vec<BezPath> = Vec::new();

    // Outer thick ring (radius 76..67) — winding difference via reversed inner contour.
    seal_paths.push(ring_bezpath(center, 76.0, 67.0));
    // Inner thin framing ring (radius 62..58.5).
    seal_paths.push(ring_bezpath(center, 62.0, 58.5));

    // Traditional central seal crossbars & seal-script (`전서체`) geometric strokes
    // modulated deterministically by `label` while guaranteeing crisp strokes through center.
    seal_paths.push(rect_stroke_bezpath(92.5, 38.0, 99.5, 154.0));
    seal_paths.push(rect_stroke_bezpath(42.0, 92.5, 150.0, 99.5));
    seal_paths.push(rect_stroke_bezpath(54.0, 62.0, 138.0, 68.5));
    seal_paths.push(rect_stroke_bezpath(54.0, 123.5, 138.0, 130.0));

    // Quadrant seal-character strokes derived from `label` codepoints.
    let mut seed: u32 = 0x5A17_9E3B;
    for ch in label.chars() {
        seed = seed.rotate_left(5) ^ (ch as u32).wrapping_mul(2_654_435_761);
    }
    let quadrants = [
        (56.0, 72.0, 86.0, 88.0),
        (106.0, 72.0, 136.0, 88.0),
        (56.0, 104.0, 86.0, 120.0),
        (106.0, 104.0, 136.0, 120.0),
    ];
    for (qi, &(x0, y0, x1, y1)) in quadrants.iter().enumerate() {
        let bits = (seed >> (qi * 6)) & 0x3F;
        let mid_x = 0.5 * (x0 + x1);
        let mid_y = 0.5 * (y0 + y1);
        seal_paths.push(rect_stroke_bezpath(x0, y0, x0 + 5.5, y1));
        seal_paths.push(rect_stroke_bezpath(x1 - 5.5, y0, x1, y1));
        if bits & 1 == 0 {
            seal_paths.push(rect_stroke_bezpath(x0, mid_y - 2.6, x1, mid_y + 2.6));
        } else {
            seal_paths.push(rect_stroke_bezpath(mid_x - 2.6, y0, mid_x + 2.6, y1));
        }
    }

    // Composite vermilion Korean seal ink (`인주`: `[206, 28, 24, 255]`) onto the opaque paper!
    let mut overlay = vec![0u8; rgba.len()];
    rasterize_filled_bezpaths_rgba(&mut overlay, size, size, &seal_paths, [206, 28, 24, 255]);
    for (paper_px, ink_px) in rgba.chunks_exact_mut(4).zip(overlay.chunks_exact(4)) {
        let a = f32::from(ink_px[3]) / 255.0;
        if a > 0.0 {
            paper_px[0] =
                ((f32::from(ink_px[0]) * a) + f32::from(paper_px[0]) * (1.0 - a)).round() as u8;
            paper_px[1] =
                ((f32::from(ink_px[1]) * a) + f32::from(paper_px[1]) * (1.0 - a)).round() as u8;
            paper_px[2] =
                ((f32::from(ink_px[2]) * a) + f32::from(paper_px[2]) * (1.0 - a)).round() as u8;
            paper_px[3] = 255;
        }
    }

    let stamp = RgbaStamp::new(size, size, rgba)?;
    stamp.to_png_bytes()
}

/// Quick text-based Korean/Unicode signature or approval stamp (e.g. `"이영우"`, `"승인"`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuickSignText {
    /// Text string to shape and render.
    pub text: String,
    /// Straight RGBA8 text color.
    pub text_color: [u8; 4],
    /// Optional rectangular/rounded seal border color.
    pub border_color: Option<[u8; 4]>,
    /// Nominal font size in points.
    pub font_size_pt: f32,
}

impl QuickSignText {
    /// Create a new text-based quick stamp.
    #[must_use]
    pub fn new(
        text: impl Into<String>,
        text_color: [u8; 4],
        border_color: Option<[u8; 4]>,
    ) -> Self {
        Self {
            text: text.into(),
            text_color,
            border_color,
            font_size_pt: 16.0,
        }
    }

    /// Render the shaped Korean/Unicode text stamp to a transparent [`RgbaStamp`].
    pub fn render_rgba_stamp(
        &self,
        box_size_pt: [f32; 2],
        pixels_per_pt: f32,
    ) -> Result<RgbaStamp, PdfError> {
        let w_pt = box_size_pt[0].clamp(16.0, 600.0);
        let h_pt = box_size_pt[1].clamp(12.0, 300.0);
        let ppp = pixels_per_pt.clamp(1.0, 6.0);

        let font = resolve_krilla_font(&self.text)?;
        let mut writer = Writer::new(
            krilla::SerializeSettings::default(),
            Metadata::default(),
        );
        let doc = writer.document_mut();
        let mut page = doc.start_page_with(PageSettings::from_wh(w_pt, h_pt).ok_or(
            PdfError::InvalidDimensions {
                width: w_pt as u32,
                height: h_pt as u32,
            },
        )?);
        let mut surface = page.surface();

        if let Some(bc) = self.border_color {
            let inset = 1.5_f32;
            let mut pb = PathBuilder::new();
            pb.move_to(inset, inset);
            pb.line_to(w_pt - inset, inset);
            pb.line_to(w_pt - inset, h_pt - inset);
            pb.line_to(inset, h_pt - inset);
            pb.close();
            if let Some(border_path) = pb.finish() {
                surface.set_stroke(Some(Stroke {
                    paint: Paint::from(rgb::Color::new(bc[0], bc[1], bc[2])),
                    width: 1.5,
                    opacity: krilla::num::NormalizedF32::new(f32::from(bc[3]) / 255.0)
                        .unwrap_or(krilla::num::NormalizedF32::ONE),
                    ..Stroke::default()
                }));
                surface.draw_path(&border_path);
                surface.set_stroke(None);
            }
        }

        let char_count = self.text.chars().count().max(1) as f32;
        let fit_size = ((w_pt - 8.0) / (char_count * 0.92))
            .min(h_pt * 0.62)
            .min(self.font_size_pt)
            .max(6.0);
        let text_w_est = char_count * fit_size * 0.85;
        let tx = ((w_pt - text_w_est) * 0.5).max(3.0);
        let ty = (h_pt * 0.5) + (fit_size * 0.34);

        surface.set_fill(Some(Fill {
            paint: Paint::from(rgb::Color::new(
                self.text_color[0],
                self.text_color[1],
                self.text_color[2],
            )),
            opacity: krilla::num::NormalizedF32::new(f32::from(self.text_color[3]) / 255.0)
                .unwrap_or(krilla::num::NormalizedF32::ONE),
            rule: krilla::paint::FillRule::NonZero,
        }));
        surface.draw_text(
            KrillaPoint::from_xy(tx, ty),
            font,
            fit_size,
            &self.text,
            false,
            TextDirection::LeftToRight,
        );
        surface.finish();
        page.finish();

        let mini_pdf_bytes = writer.finish()?;
        let parsed = craft_pdf::open_slice(&mini_pdf_bytes, None)?;
        let pdf_page = parsed
            .pages()
            .first()
            .ok_or(PdfError::PageOutOfBounds {
                page_index: 0,
                page_count: 0,
            })?;
        let cache = RenderCache::new();
        let settings = build_interpreter_settings();
        let raster = craft_pdf::raster::render(
            pdf_page,
            &cache,
            &settings,
            &RasterOptions {
                scale: [ppp, ppp],
                size: None,
                offset: [0.0, 0.0],
                background: [0, 0, 0, 0],
                color_replacement: None,
            },
        )?;
        let straight = unpremultiply_rgba_bytes(raster.bytes());
        RgbaStamp::new(raster.width, raster.height, straight)
    }
}

/// Payload variants supported by [`PlacedSignature`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SignaturePayload {
    /// Vector fountain-pen Bezier strokes.
    BezierStrokes(FreehandBezierSignature),
    /// Transparent or white-background-removed RGBA8 stamp/seal image.
    RgbaStamp(RgbaStamp),
    /// Shaped Korean/Unicode quick-sign text stamp.
    QuickTextStamp(QuickSignText),
}

impl SignaturePayload {
    /// Rasterize this payload at the requested resolution (`pixels_per_pt`) for live preview
    /// or soft-mask image XObject generation.
    pub fn to_rgba_stamp(
        &self,
        box_size_pt: [f32; 2],
        pixels_per_pt: f32,
    ) -> Result<RgbaStamp, PdfError> {
        match self {
            Self::BezierStrokes(sig) => sig.render_rgba_stamp(pixels_per_pt),
            Self::RgbaStamp(stamp) => Ok(stamp.clone()),
            Self::QuickTextStamp(txt) => txt.render_rgba_stamp(box_size_pt, pixels_per_pt),
        }
    }
}

/// Interactive signature or seal instance placed on a specific PDF page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlacedSignature {
    /// Stable identifier used for move/resize/remove/MCP operations.
    pub id: String,
    /// Zero-based target page index.
    pub page_index: usize,
    /// Bounding box `[x_pt, y_pt, width_pt, height_pt]` in top-left viewer page coordinates.
    pub rect_pt: [f32; 4],
    /// Clockwise rotation in degrees around the center of `rect_pt`.
    pub rotation_deg: f32,
    /// Layer opacity in `0.0..=1.0`.
    pub opacity: f32,
    /// Underlying vector Bezier or RGBA stamp payload.
    pub payload: SignaturePayload,
}

impl PlacedSignature {
    /// Create a placed signature at `rect_pt` (`[x, y, width, height]` in top-left page points).
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        page_index: usize,
        rect_pt: [f32; 4],
        payload: SignaturePayload,
    ) -> Self {
        Self {
            id: id.into(),
            page_index,
            rect_pt: [
                rect_pt[0],
                rect_pt[1],
                rect_pt[2].max(4.0),
                rect_pt[3].max(4.0),
            ],
            rotation_deg: 0.0,
            opacity: 1.0,
            payload,
        }
    }
}

/// Anchor-relative placement alignment when auto-stamping `"(인)"` or `"서명란"`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorPlacementMode {
    /// Center the seal directly over the matched anchor token (traditional Korean `(인)` stamping).
    #[default]
    CenterOverAnchor,
    /// Place immediately to the right of the anchor label (e.g. after `"서명:"`).
    RightOfAnchor,
    /// Place immediately above the anchor label.
    AboveAnchor,
}

impl AnchorPlacementMode {
    /// Compute `[x, y, w, h]` in top-left page points from an anchor `rect_pt` and stamp `[w, h]`.
    #[must_use]
    pub fn compute_rect(self, anchor_rect_pt: [f32; 4], size_pt: [f32; 2]) -> [f32; 4] {
        let [ax, ay, aw, ah] = anchor_rect_pt;
        let sw = size_pt[0].max(8.0);
        let sh = size_pt[1].max(8.0);
        match self {
            Self::CenterOverAnchor => {
                let cx = ax + aw * 0.5;
                let cy = ay + ah * 0.5;
                [cx - sw * 0.5, cy - sh * 0.5, sw, sh]
            }
            Self::RightOfAnchor => {
                let cy = ay + ah * 0.5;
                [ax + aw + 4.0, cy - sh * 0.5, sw, sh]
            }
            Self::AboveAnchor => {
                let cx = ax + aw * 0.5;
                [(cx - sw * 0.5).max(0.0), (ay - sh - 2.0).max(0.0), sw, sh]
            }
        }
    }
}

/// Undo/Redo-capable layer stack of [`PlacedSignature`] overlays for an active PDF session.
#[derive(Clone, Debug, Default)]
pub struct SignatureLayerStack {
    active: Vec<PlacedSignature>,
    undo_stack: Vec<Vec<PlacedSignature>>,
    redo_stack: Vec<Vec<PlacedSignature>>,
    next_seq: u64,
}

impl SignatureLayerStack {
    /// Create an empty signature layer stack.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a deterministic unique signature ID (`"sig-1"`, `"sig-2"`, ...).
    pub fn allocate_id(&mut self) -> String {
        self.next_seq = self.next_seq.saturating_add(1);
        format!("sig-{}", self.next_seq)
    }

    /// Active placed signatures in z-order (bottom to top).
    #[must_use]
    pub fn signatures(&self) -> &[PlacedSignature] {
        &self.active
    }

    /// Look up a placed signature by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&PlacedSignature> {
        self.active.iter().find(|s| s.id == id)
    }

    /// Whether [`Self::undo`] can revert a previous mutation.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether [`Self::redo`] can re-apply a reverted mutation.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    fn record_mutation(&mut self) {
        if self.undo_stack.len() >= MAX_UNDO_HISTORY {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.active.clone());
        self.redo_stack.clear();
    }

    /// Add (or replace by matching `id`) a [`PlacedSignature`] and record an undo snapshot.
    pub fn add_signature(&mut self, mut sig: PlacedSignature) -> String {
        if sig.id.trim().is_empty() {
            sig.id = self.allocate_id();
        }
        sig.rect_pt[2] = sig.rect_pt[2].max(4.0);
        sig.rect_pt[3] = sig.rect_pt[3].max(4.0);
        sig.opacity = sig.opacity.clamp(0.0, 1.0);
        self.record_mutation();
        let id = sig.id.clone();
        if let Some(existing) = self.active.iter_mut().find(|s| s.id == id) {
            *existing = sig;
        } else {
            self.active.push(sig);
        }
        id
    }

    /// Move an existing signature's top-left origin to `(new_x_pt, new_y_pt)`.
    pub fn move_signature(&mut self, id: &str, new_x_pt: f32, new_y_pt: f32) -> Result<(), PdfError> {
        if !self.active.iter().any(|s| s.id == id) {
            return Err(PdfError::SignatureNotFound(id.to_owned()));
        }
        self.record_mutation();
        if let Some(sig) = self.active.iter_mut().find(|s| s.id == id) {
            sig.rect_pt[0] = new_x_pt;
            sig.rect_pt[1] = new_y_pt;
        }
        Ok(())
    }

    /// Resize an existing signature to `(new_width_pt, new_height_pt)`.
    pub fn resize_signature(
        &mut self,
        id: &str,
        new_width_pt: f32,
        new_height_pt: f32,
    ) -> Result<(), PdfError> {
        if !self.active.iter().any(|s| s.id == id) {
            return Err(PdfError::SignatureNotFound(id.to_owned()));
        }
        self.record_mutation();
        if let Some(sig) = self.active.iter_mut().find(|s| s.id == id) {
            sig.rect_pt[2] = new_width_pt.max(4.0);
            sig.rect_pt[3] = new_height_pt.max(4.0);
        }
        Ok(())
    }

    /// Update rotation in degrees clockwise around the signature's center.
    pub fn rotate_signature(&mut self, id: &str, rotation_deg: f32) -> Result<(), PdfError> {
        if !self.active.iter().any(|s| s.id == id) {
            return Err(PdfError::SignatureNotFound(id.to_owned()));
        }
        self.record_mutation();
        if let Some(sig) = self.active.iter_mut().find(|s| s.id == id) {
            sig.rotation_deg = rotation_deg;
        }
        Ok(())
    }

    /// Update opacity (`0.0..=1.0`) of an existing signature.
    pub fn set_signature_opacity(&mut self, id: &str, opacity: f32) -> Result<(), PdfError> {
        if !self.active.iter().any(|s| s.id == id) {
            return Err(PdfError::SignatureNotFound(id.to_owned()));
        }
        self.record_mutation();
        if let Some(sig) = self.active.iter_mut().find(|s| s.id == id) {
            sig.opacity = opacity.clamp(0.0, 1.0);
        }
        Ok(())
    }

    /// Remove a placed signature by ID and record an undo snapshot.
    pub fn remove_signature(&mut self, id: &str) -> Result<PlacedSignature, PdfError> {
        let pos = self
            .active
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| PdfError::SignatureNotFound(id.to_owned()))?;
        self.record_mutation();
        Ok(self.active.remove(pos))
    }

    /// Revert the most recent signature layer mutation. Returns `true` if state changed.
    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo_stack.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.active, prev);
        self.redo_stack.push(current);
        true
    }

    /// Re-apply a previously undone signature layer mutation. Returns `true` if state changed.
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.active, next);
        self.undo_stack.push(current);
        true
    }
}

/// Convert premultiplied RGBA8 bytes (from `craft_pdf::raster::Raster`) into straight RGBA8 bytes.
#[must_use]
pub fn unpremultiply_rgba_bytes(premul: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(premul.len());
    for px in premul.chunks_exact(4) {
        let a = px[3];
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else if a == 255 {
            out.extend_from_slice(px);
        } else {
            let a_u32 = u32::from(a);
            let r = ((u32::from(px[0]) * 255 + a_u32 / 2) / a_u32).min(255) as u8;
            let g = ((u32::from(px[1]) * 255 + a_u32 / 2) / a_u32).min(255) as u8;
            let b = ((u32::from(px[2]) * 255 + a_u32 / 2) / a_u32).min(255) as u8;
            out.extend_from_slice(&[r, g, b, a]);
        }
    }
    out
}

/// Composite all active [`PlacedSignature`]s for `page_index` onto a straight-RGBA8 buffer
/// rendered at `scale` with device-pixel viewport origin `offset_px`.
pub fn composite_signatures_onto_rgba(
    canvas_rgba: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    page_index: usize,
    scale: f32,
    offset_px: [f32; 2],
    signatures: &[PlacedSignature],
) -> Result<(), PdfError> {
    for sig in signatures.iter().filter(|s| s.page_index == page_index) {
        if sig.opacity <= 0.001 {
            continue;
        }
        let [rx, ry, rw, rh] = sig.rect_pt;
        if rw <= 0.0 || rh <= 0.0 {
            continue;
        }
        let stamp = sig
            .payload
            .to_rgba_stamp([rw, rh], scale.clamp(1.0, 4.0) * 1.5)?;
        let dst_cx = (rx + rw * 0.5) * scale - offset_px[0];
        let dst_cy = (ry + rh * 0.5) * scale - offset_px[1];
        let dst_w = (rw * scale).max(1.0);
        let dst_h = (rh * scale).max(1.0);

        let rad = sig.rotation_deg.to_radians();
        let (sin_r, cos_r) = rad.sin_cos();
        let half_diag = 0.5 * dst_w.hypot(dst_h) + 2.0;

        let min_px = ((dst_cx - half_diag).floor().max(0.0)) as u32;
        let max_px = ((dst_cx + half_diag).ceil().max(0.0) as u32).min(canvas_w);
        let min_py = ((dst_cy - half_diag).floor().max(0.0)) as u32;
        let max_py = ((dst_cy + half_diag).ceil().max(0.0) as u32).min(canvas_h);

        for py in min_py..max_py {
            for px in min_px..max_px {
                let dx = (px as f32 + 0.5) - dst_cx;
                let dy = (py as f32 + 0.5) - dst_cy;
                // Inverse-rotate into unrotated signature box `[-dst_w/2 .. +dst_w/2]`.
                let lx = dx * cos_r + dy * sin_r + dst_w * 0.5;
                let ly = -dx * sin_r + dy * cos_r + dst_h * 0.5;
                if lx < 0.0 || ly < 0.0 || lx >= dst_w || ly >= dst_h {
                    continue;
                }
                let u = (lx / dst_w) * (stamp.width as f32) - 0.5;
                let v = (ly / dst_h) * (stamp.height as f32) - 0.5;
                let [sr, sg, sb, sa] = sample_bilinear_straight(&stamp, u, v);
                let eff_a = (sa * sig.opacity).clamp(0.0, 1.0);
                if eff_a <= 0.001 {
                    continue;
                }

                let idx = ((py as usize) * (canvas_w as usize) + (px as usize)) * 4;
                let dst_a = f32::from(canvas_rgba[idx + 3]) / 255.0;
                let out_a = eff_a + dst_a * (1.0 - eff_a);
                if out_a > 0.0 {
                    let dr = f32::from(canvas_rgba[idx]) / 255.0;
                    let dg = f32::from(canvas_rgba[idx + 1]) / 255.0;
                    let db = f32::from(canvas_rgba[idx + 2]) / 255.0;
                    let or = (sr * eff_a + dr * dst_a * (1.0 - eff_a)) / out_a;
                    let og = (sg * eff_a + dg * dst_a * (1.0 - eff_a)) / out_a;
                    let ob = (sb * eff_a + db * dst_a * (1.0 - eff_a)) / out_a;
                    canvas_rgba[idx] = (or * 255.0).round().clamp(0.0, 255.0) as u8;
                    canvas_rgba[idx + 1] = (og * 255.0).round().clamp(0.0, 255.0) as u8;
                    canvas_rgba[idx + 2] = (ob * 255.0).round().clamp(0.0, 255.0) as u8;
                    canvas_rgba[idx + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal geometry, rasterization, and chroma-key helpers
// ---------------------------------------------------------------------------

fn rdp_simplify(points: &[SignaturePoint], epsilon: f64) -> Vec<SignaturePoint> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;

    let mut stack: Vec<(usize, usize)> = vec![(0, points.len() - 1)];
    while let Some((start, end)) = stack.pop() {
        if end <= start + 1 {
            continue;
        }
        let a = points[start].to_kurbo();
        let b = points[end].to_kurbo();
        let mut max_dist = 0.0_f64;
        let mut split_idx = start;
        for (i, pt) in points.iter().enumerate().take(end).skip(start + 1) {
            let d = point_segment_distance(pt.to_kurbo(), a, b);
            if d > max_dist {
                max_dist = d;
                split_idx = i;
            }
        }
        if max_dist > epsilon && split_idx > start && split_idx < end {
            keep[split_idx] = true;
            stack.push((start, split_idx));
            stack.push((split_idx, end));
        }
    }

    points
        .iter()
        .copied()
        .zip(keep)
        .filter_map(|(pt, k)| k.then_some(pt))
        .collect()
}

fn point_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let len_sq = ab.hypot2();
    if len_sq <= 1e-12 {
        return (p - a).hypot();
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = a + ab * t;
    (p - proj).hypot()
}

fn segment_speed(a: &SignaturePoint, b: &SignaturePoint) -> f64 {
    let dist = (b.to_kurbo() - a.to_kurbo()).hypot();
    let dt = (b.timestamp_ms - a.timestamp_ms).abs().max(1.0);
    dist / dt
}

fn circle_bezpath(center: Point, radius: f64) -> BezPath {
    // Magic cubic Bezier constant for circular quadrant: 4/3 * (sqrt(2) - 1)
    let k = 0.552_284_749_830_793_6 * radius;
    let cx = center.x;
    let cy = center.y;
    let r = radius;
    let mut path = BezPath::new();
    path.move_to(Point::new(cx + r, cy));
    path.curve_to(
        Point::new(cx + r, cy + k),
        Point::new(cx + k, cy + r),
        Point::new(cx, cy + r),
    );
    path.curve_to(
        Point::new(cx - k, cy + r),
        Point::new(cx - r, cy + k),
        Point::new(cx - r, cy),
    );
    path.curve_to(
        Point::new(cx - r, cy - k),
        Point::new(cx - k, cy - r),
        Point::new(cx, cy - r),
    );
    path.curve_to(
        Point::new(cx + k, cy - r),
        Point::new(cx + r, cy - k),
        Point::new(cx + r, cy),
    );
    path.close_path();
    path
}

fn ring_bezpath(center: Point, outer_r: f64, inner_r: f64) -> BezPath {
    let mut path = circle_bezpath(center, outer_r);
    // Append opposite-winding inner circle so NonZero fill leaves the center hollow.
    let k = 0.552_284_749_830_793_6 * inner_r;
    let cx = center.x;
    let cy = center.y;
    let r = inner_r;
    path.move_to(Point::new(cx + r, cy));
    path.curve_to(
        Point::new(cx + r, cy - k),
        Point::new(cx + k, cy - r),
        Point::new(cx, cy - r),
    );
    path.curve_to(
        Point::new(cx - k, cy - r),
        Point::new(cx - r, cy - k),
        Point::new(cx - r, cy),
    );
    path.curve_to(
        Point::new(cx - r, cy + k),
        Point::new(cx - k, cy + r),
        Point::new(cx, cy + r),
    );
    path.curve_to(
        Point::new(cx + k, cy + r),
        Point::new(cx + r, cy + k),
        Point::new(cx + r, cy),
    );
    path.close_path();
    path
}

fn rect_stroke_bezpath(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(Point::new(x0, y0));
    path.line_to(Point::new(x1, y0));
    path.line_to(Point::new(x1, y1));
    path.line_to(Point::new(x0, y1));
    path.close_path();
    path
}

fn append_semicircle_cap(path: &mut BezPath, center: Point, from: Point, to: Point) {
    let v = from - center;
    let r = v.hypot();
    if r <= 1e-6 {
        path.line_to(to);
        return;
    }
    let start_angle = v.y.atan2(v.x);
    // Sweep -PI (clockwise in Y-down screen space matches left -> forward tip -> right).
    let steps = 6usize;
    for s in 1..=steps {
        let theta = start_angle - std::f64::consts::PI * (s as f64 / steps as f64);
        path.line_to(Point::new(
            center.x + r * theta.cos(),
            center.y + r * theta.sin(),
        ));
    }
    path.line_to(to);
}

/// 4x4 supersampled NonZero winding-number scanline rasterizer for closed [`BezPath`] shapes.
fn rasterize_filled_bezpaths_rgba(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    paths: &[BezPath],
    color: [u8; 4],
) {
    if paths.is_empty() || color[3] == 0 {
        return;
    }
    // Flatten curves into line segments `[p0, p1]` once.
    let mut segments: Vec<(Point, Point)> = Vec::new();
    let mut min_y = f64::from(height);
    let mut max_y = 0.0_f64;

    for path in paths {
        let mut start = Point::ORIGIN;
        let mut cur = Point::ORIGIN;
        kurbo::flatten(path.iter(), 0.15, |el| match el {
            PathEl::MoveTo(p) => {
                start = p;
                cur = p;
            }
            PathEl::LineTo(p) => {
                if (p.y - cur.y).abs() > 1e-9 {
                    min_y = min_y.min(cur.y.min(p.y));
                    max_y = max_y.max(cur.y.max(p.y));
                    segments.push((cur, p));
                }
                cur = p;
            }
            PathEl::ClosePath => {
                if (start.y - cur.y).abs() > 1e-9 {
                    min_y = min_y.min(cur.y.min(start.y));
                    max_y = max_y.max(cur.y.max(start.y));
                    segments.push((cur, start));
                }
                cur = start;
            }
            PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
        });
    }

    if segments.is_empty() {
        return;
    }

    let y_start = (min_y.floor().max(0.0) as u32).min(height);
    let y_end = (max_y.ceil().max(0.0) as u32).min(height);
    let sub_y_offsets = [0.125_f64, 0.375, 0.625, 0.875];
    let sub_x_offsets = [0.125_f64, 0.375, 0.625, 0.875];
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    let mut coverage_row = vec![0u8; width as usize];

    let base_alpha = f32::from(color[3]) / 255.0;

    for py in y_start..y_end {
        coverage_row.fill(0);
        for &sy_off in &sub_y_offsets {
            let scan_y = f64::from(py) + sy_off;
            crossings.clear();
            for &(p0, p1) in &segments {
                let (y0, y1) = (p0.y, p1.y);
                if (y0 <= scan_y && y1 > scan_y) || (y1 <= scan_y && y0 > scan_y) {
                    let dir = if y1 > y0 { 1 } else { -1 };
                    let t = (scan_y - y0) / (y1 - y0);
                    let ix = p0.x + t * (p1.x - p0.x);
                    crossings.push((ix, dir));
                }
            }
            if crossings.len() < 2 {
                continue;
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));

            // Walk NonZero intervals across `crossings`.
            let mut winding = 0i32;
            let mut interval_start: Option<f64> = None;
            for &(ix, dir) in &crossings {
                let prev = winding;
                winding += dir;
                if prev == 0 && winding != 0 {
                    interval_start = Some(ix);
                } else if prev != 0
                    && winding == 0
                    && let Some(x0) = interval_start.take()
                {
                    let x1 = ix;
                    let px_min = (x0.floor().max(0.0) as u32).min(width);
                    let px_max = (x1.ceil().max(0.0) as u32).min(width);
                    for px in px_min..px_max {
                        let base_x = f64::from(px);
                        let mut hits = 0u8;
                        for &sx_off in &sub_x_offsets {
                            let sx = base_x + sx_off;
                            if sx >= x0 && sx < x1 {
                                hits += 1;
                            }
                        }
                        coverage_row[px as usize] = coverage_row[px as usize].saturating_add(hits);
                    }
                }
            }
        }

        for (px, &cov) in coverage_row.iter().enumerate() {
            if cov == 0 {
                continue;
            }
            let frac = (f32::from(cov) / 16.0) * base_alpha;
            let out_a = (frac * 255.0).round().clamp(0.0, 255.0) as u8;
            if out_a == 0 {
                continue;
            }
            let idx = ((py as usize) * (width as usize) + px) * 4;
            rgba[idx] = color[0];
            rgba[idx + 1] = color[1];
            rgba[idx + 2] = color[2];
            rgba[idx + 3] = rgba[idx + 3].max(out_a);
        }
    }
}

fn estimate_corner_paper_background(stamp: &RgbaStamp) -> [u8; 3] {
    let patch = (stamp.width.min(stamp.height) / 10).clamp(1, 16);
    let mut rs = Vec::new();
    let mut gs = Vec::new();
    let mut bs = Vec::new();

    let corners = [
        (0u32, 0u32),
        (stamp.width.saturating_sub(patch), 0u32),
        (0u32, stamp.height.saturating_sub(patch)),
        (
            stamp.width.saturating_sub(patch),
            stamp.height.saturating_sub(patch),
        ),
    ];

    for &(cx, cy) in &corners {
        for y in cy..(cy + patch).min(stamp.height) {
            for x in cx..(cx + patch).min(stamp.width) {
                if let Some([r, g, b, a]) = stamp.pixel(x, y)
                    && a > 128
                {
                    rs.push(r);
                    gs.push(g);
                    bs.push(b);
                }
            }
        }
    }

    if rs.is_empty() {
        return [255, 255, 255];
    }
    rs.sort_unstable();
    gs.sort_unstable();
    bs.sort_unstable();
    // Pick upper-quartile (75th percentile) of corner samples so an accidental stray stroke in one
    // corner never drags the estimated paper white-point down.
    let q_idx = ((rs.len() * 3) / 4).min(rs.len() - 1);
    [rs[q_idx], gs[q_idx], bs[q_idx]]
}

fn srgb_luminance(r: f32, g: f32, b: f32) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn sample_bilinear_straight(stamp: &RgbaStamp, u: f32, v: f32) -> [f32; 4] {
    let max_x = (stamp.width as i32) - 1;
    let max_y = (stamp.height as i32) - 1;
    let x0 = (u.floor() as i32).clamp(0, max_x) as u32;
    let y0 = (v.floor() as i32).clamp(0, max_y) as u32;
    let x1 = ((u.floor() as i32 + 1).clamp(0, max_x)) as u32;
    let y1 = ((v.floor() as i32 + 1).clamp(0, max_y)) as u32;
    let fx = (u - u.floor()).clamp(0.0, 1.0);
    let fy = (v - v.floor()).clamp(0.0, 1.0);

    let p00 = stamp.pixel(x0, y0).unwrap_or([0; 4]);
    let p10 = stamp.pixel(x1, y0).unwrap_or([0; 4]);
    let p01 = stamp.pixel(x0, y1).unwrap_or([0; 4]);
    let p11 = stamp.pixel(x1, y1).unwrap_or([0; 4]);

    // Interpolate in premultiplied space so transparent texels never bleed RGB into neighbors!
    let premul = |p: [u8; 4]| -> [f32; 4] {
        let a = f32::from(p[3]) / 255.0;
        [
            (f32::from(p[0]) / 255.0) * a,
            (f32::from(p[1]) / 255.0) * a,
            (f32::from(p[2]) / 255.0) * a,
            a,
        ]
    };
    let c00 = premul(p00);
    let c10 = premul(p10);
    let c01 = premul(p01);
    let c11 = premul(p11);

    let w00 = (1.0 - fx) * (1.0 - fy);
    let w10 = fx * (1.0 - fy);
    let w01 = (1.0 - fx) * fy;
    let w11 = fx * fy;

    let pa = c00[3] * w00 + c10[3] * w10 + c01[3] * w01 + c11[3] * w11;
    if pa <= 1e-5 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let pr = c00[0] * w00 + c10[0] * w10 + c01[0] * w01 + c11[0] * w11;
    let pg = c00[1] * w00 + c10[1] * w10 + c01[1] * w01 + c11[1] * w11;
    let pb = c00[2] * w00 + c10[2] * w10 + c01[2] * w01 + c11[2] * w11;
    [
        (pr / pa).clamp(0.0, 1.0),
        (pg / pa).clamp(0.0, 1.0),
        (pb / pa).clamp(0.0, 1.0),
        pa.clamp(0.0, 1.0),
    ]
}

/// Convenience helper to decode PNG bytes back into [`RgbaStamp`] (also used in QA round-trips).
pub fn decode_png_stamp(png_bytes: &[u8]) -> Result<RgbaStamp, PdfError> {
    let reader = image::ImageReader::with_format(Cursor::new(png_bytes), ImageFormat::Png);
    let img = reader
        .decode()
        .map_err(|e| PdfError::ImageDecode(e.to_string()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    RgbaStamp::new(w, h, img.into_raw())
}
