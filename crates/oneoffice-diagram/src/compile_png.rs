//! Pure-Rust PNG rasterizer for `PositionedScene` using `vello_cpu`, `skrifa`, and `image`.

use image::ImageEncoder as _;
use kurbo::{Affine, BezPath, Point, RoundedRect, Shape};
use vello_cpu::peniko::Color;
use vello_cpu::{PixmapMut, RenderContext, RenderSettings, Resources};

use crate::font_metrics::{DiagramFontContext, DrawableGlyph, build_fallback_glyph_path};
use crate::layout::{HAlign, PositionedScene, SceneElement};
use crate::spec::{DiagramError, RgbColor};

/// Maximum raster dimensions in pixels (`16 MP` cap to prevent OOM).
const MAX_RASTER_PIXELS: usize = 16_777_216;

#[must_use]
fn to_peniko_color(c: RgbColor) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, c.a)
}

/// Rasterizes a [`PositionedScene`] into PNG bytes at the specified scale factor.
///
/// Scale `2.0` produces crisp Retina/HiDPI output.
pub fn compile_png(scene: &PositionedScene, scale: f32) -> Result<Vec<u8>, DiagramError> {
    if !scale.is_finite() || scale <= 0.05 || scale > 8.0 {
        return Err(DiagramError::InvalidRenderScale(scale));
    }

    let w_px = (scene.width_pt * scale).round();
    let h_px = (scene.height_pt * scale).round();

    if w_px < 1.0 || h_px < 1.0 {
        return Err(DiagramError::InvalidCanvasWidth(scene.width_pt));
    }

    let px_w = (w_px as usize).min(u16::MAX as usize);
    let px_h = (h_px as usize).min(u16::MAX as usize);

    if px_w.saturating_mul(px_h) > MAX_RASTER_PIXELS {
        return Err(DiagramError::RasterError(format!(
            "target image resolution {px_w}x{px_h} exceeds safety threshold ({MAX_RASTER_PIXELS} px)"
        )));
    }

    let u16_w = px_w as u16;
    let u16_h = px_h as u16;

    let mut ctx = RenderContext::new_with(
        u16_w,
        u16_h,
        RenderSettings {
            num_threads: 0,
            ..Default::default()
        },
    );

    // Initial clear: pure white background
    ctx.set_transform(Affine::IDENTITY);
    ctx.set_paint(Color::from_rgba8(255, 255, 255, 255));
    ctx.fill_rect(&kurbo::Rect::new(0.0, 0.0, px_w as f64, px_h as f64));

    let s = scale as f64;
    let font_ctx = DiagramFontContext::global();

    for elem in &scene.elements {
        match elem {
            SceneElement::Rect {
                rect,
                fill,
                stroke,
                corner_radius,
            } => {
                let rx = rect.x as f64 * s;
                let ry = rect.y as f64 * s;
                let rw = rect.w as f64 * s;
                let rh = rect.h as f64 * s;
                let cr = (*corner_radius as f64 * s).min(rw / 2.0).min(rh / 2.0);

                let shape = RoundedRect::new(rx, ry, rx + rw, ry + rh, cr);
                let path = shape.to_path(0.1);

                ctx.set_transform(Affine::IDENTITY);
                if let Some(f) = fill {
                    ctx.set_paint(to_peniko_color(*f));
                    ctx.fill_path(&path);
                }
                if let Some((st_c, sw)) = stroke {
                    ctx.set_paint(to_peniko_color(*st_c));
                    let stroke_style = kurbo::Stroke::new(*sw as f64 * s);
                    ctx.set_stroke(stroke_style);
                    ctx.stroke_path(&path);
                }
            }
            SceneElement::Text {
                text,
                x,
                y,
                font_size_pt,
                color,
                bold,
                align,
            } => {
                let glyphs = font_ctx.shape_text(text, *font_size_pt, *bold);
                let total_adv: f32 = glyphs.iter().map(|g| match g {
                    DrawableGlyph::Bundled { advance_pt, .. }
                    | DrawableGlyph::SystemCjk { advance_pt, .. }
                    | DrawableGlyph::FallbackChar { advance_pt, .. } => *advance_pt,
                }).sum();

                let origin_x = match align {
                    HAlign::Left => *x,
                    HAlign::Center => *x - (total_adv / 2.0),
                    HAlign::Right => *x - total_adv,
                };

                let peniko_c = to_peniko_color(*color);
                ctx.set_paint(peniko_c);

                let mut cur_adv = 0.0f32;
                for glyph in glyphs {
                    match glyph {
                        DrawableGlyph::Bundled {
                            face,
                            gid,
                            x_offset_pt,
                            y_offset_pt,
                            advance_pt,
                        } => {
                            let upem = if face.upem.is_finite() && face.upem > 0.0 {
                                face.upem
                            } else {
                                1000.0
                            };
                            let glyph_k = (*font_size_pt as f64 * s) / upem;
                            let gx = (origin_x + cur_adv + x_offset_pt) as f64 * s;
                            let gy = (*y - y_offset_pt) as f64 * s;

                            let transform = Affine::translate((gx, gy))
                                * Affine::scale_non_uniform(glyph_k, -glyph_k);
                            ctx.set_transform(transform);
                            ctx.fill_path(&face.glyph_outline(gid));
                            cur_adv += advance_pt;
                        }
                        DrawableGlyph::SystemCjk {
                            gid,
                            upem,
                            x_offset_pt,
                            y_offset_pt,
                            advance_pt,
                        } => {
                            if let Some(outline) = font_ctx.cjk_outline(gid) {
                                let font_upem = if upem > 0.0 { upem as f64 } else { 1000.0 };
                                let glyph_k = (*font_size_pt as f64 * s) / font_upem;
                                let gx = (origin_x + cur_adv + x_offset_pt) as f64 * s;
                                let gy = (*y - y_offset_pt) as f64 * s;

                                let transform = Affine::translate((gx, gy))
                                    * Affine::scale_non_uniform(glyph_k, -glyph_k);
                                ctx.set_transform(transform);
                                ctx.fill_path(&outline);
                            }
                            cur_adv += advance_pt;
                        }
                        DrawableGlyph::FallbackChar { ch, advance_pt } => {
                            let (fallback_path, is_box) = build_fallback_glyph_path(
                                ch,
                                *font_size_pt * scale,
                                advance_pt * scale,
                            );
                            let gx = (origin_x + cur_adv) as f64 * s;
                            let gy = *y as f64 * s;

                            ctx.set_transform(Affine::translate((gx, gy)));
                            if is_box {
                                let stroke_st = kurbo::Stroke::new(1.0);
                                ctx.set_stroke(stroke_st);
                                ctx.stroke_path(&fallback_path);
                            } else {
                                ctx.fill_path(&fallback_path);
                            }
                            cur_adv += advance_pt;
                        }
                    }
                }
            }
            SceneElement::Line {
                x1,
                y1,
                x2,
                y2,
                color,
                width,
                dashed,
            } => {
                ctx.set_transform(Affine::IDENTITY);
                ctx.set_paint(to_peniko_color(*color));
                let stroke_style = kurbo::Stroke::new(*width as f64 * s);

                if *dashed {
                    let dx = (*x2 - *x1) as f64 * s;
                    let dy = (*y2 - *y1) as f64 * s;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > 0.001 {
                        let nx = dx / dist;
                        let ny = dy / dist;
                        let dash_len = 5.0 * s;
                        let gap_len = 3.0 * s;
                        let mut covered = 0.0;
                        let mut dash_path = BezPath::new();
                        while covered < dist {
                            let seg_end = (covered + dash_len).min(dist);
                            dash_path.move_to(Point::new(*x1 as f64 * s + nx * covered, *y1 as f64 * s + ny * covered));
                            dash_path.line_to(Point::new(*x1 as f64 * s + nx * seg_end, *y1 as f64 * s + ny * seg_end));
                            covered += dash_len + gap_len;
                        }
                        ctx.set_stroke(stroke_style);
                        ctx.stroke_path(&dash_path);
                    }
                } else {
                    let mut path = BezPath::new();
                    path.move_to(Point::new(*x1 as f64 * s, *y1 as f64 * s));
                    path.line_to(Point::new(*x2 as f64 * s, *y2 as f64 * s));
                    ctx.set_stroke(stroke_style);
                    ctx.stroke_path(&path);
                }
            }
            SceneElement::Arrow {
                x1,
                y1,
                x2,
                y2,
                color,
                width,
            } => {
                let dx = (*x2 - *x1) as f64;
                let dy = (*y2 - *y1) as f64;
                let len = (dx * dx + dy * dy).sqrt();
                if len <= 0.001 {
                    continue;
                }

                let nx = dx / len;
                let ny = dy / len;
                let head_len = (*width as f64 * 2.8).clamp(6.0, 14.0) * s;
                let head_half_w = head_len * 0.45;

                let end_x = (*x2 as f64 * s) - nx * head_len * 0.7;
                let end_y = (*y2 as f64 * s) - ny * head_len * 0.7;

                let mut line_path = BezPath::new();
                line_path.move_to(Point::new(*x1 as f64 * s, *y1 as f64 * s));
                line_path.line_to(Point::new(end_x, end_y));

                ctx.set_transform(Affine::IDENTITY);
                ctx.set_paint(to_peniko_color(*color));
                let stroke_style = kurbo::Stroke::new(*width as f64 * s);
                ctx.set_stroke(stroke_style);
                ctx.stroke_path(&line_path);

                let tip_x = *x2 as f64 * s;
                let tip_y = *y2 as f64 * s;
                let base_x = tip_x - nx * head_len;
                let base_y = tip_y - ny * head_len;

                let mut head_path = BezPath::new();
                head_path.move_to(Point::new(tip_x, tip_y));
                head_path.line_to(Point::new(base_x - ny * head_half_w, base_y + nx * head_half_w));
                head_path.line_to(Point::new(base_x + ny * head_half_w, base_y - nx * head_half_w));
                head_path.close_path();

                ctx.fill_path(&head_path);
            }
            SceneElement::PillBadge {
                rect,
                bg_color,
                text,
                text_color,
                font_size_pt,
            } => {
                let rx = rect.x as f64 * s;
                let ry = rect.y as f64 * s;
                let rw = rect.w as f64 * s;
                let rh = rect.h as f64 * s;
                let cr = rh / 2.0;

                let shape = RoundedRect::new(rx, ry, rx + rw, ry + rh, cr);
                let path = shape.to_path(0.1);

                ctx.set_transform(Affine::IDENTITY);
                ctx.set_paint(to_peniko_color(*bg_color));
                ctx.fill_path(&path);

                // Centered badge text
                let glyphs = font_ctx.shape_text(text, *font_size_pt, true);
                let total_adv: f32 = glyphs.iter().map(|g| match g {
                    DrawableGlyph::Bundled { advance_pt, .. }
                    | DrawableGlyph::SystemCjk { advance_pt, .. }
                    | DrawableGlyph::FallbackChar { advance_pt, .. } => *advance_pt,
                }).sum();

                let origin_x = (rect.x + (rect.w - total_adv) / 2.0) as f64 * s;
                let origin_y = (rect.y + rect.h * 0.72) as f64 * s;

                ctx.set_paint(to_peniko_color(*text_color));
                let mut cur_adv = 0.0f32;
                for glyph in glyphs {
                    match glyph {
                        DrawableGlyph::Bundled {
                            face,
                            gid,
                            x_offset_pt,
                            y_offset_pt,
                            advance_pt,
                        } => {
                            let upem = if face.upem.is_finite() && face.upem > 0.0 {
                                face.upem
                            } else {
                                1000.0
                            };
                            let glyph_k = (*font_size_pt as f64 * s) / upem;
                            let gx = origin_x + (cur_adv + x_offset_pt) as f64 * s;
                            let gy = origin_y - y_offset_pt as f64 * s;

                            let transform = Affine::translate((gx, gy))
                                * Affine::scale_non_uniform(glyph_k, -glyph_k);
                            ctx.set_transform(transform);
                            ctx.fill_path(&face.glyph_outline(gid));
                            cur_adv += advance_pt;
                        }
                        DrawableGlyph::SystemCjk {
                            gid,
                            upem,
                            x_offset_pt,
                            y_offset_pt,
                            advance_pt,
                        } => {
                            if let Some(outline) = font_ctx.cjk_outline(gid) {
                                let font_upem = if upem > 0.0 { upem as f64 } else { 1000.0 };
                                let glyph_k = (*font_size_pt as f64 * s) / font_upem;
                                let gx = origin_x + (cur_adv + x_offset_pt) as f64 * s;
                                let gy = origin_y - y_offset_pt as f64 * s;

                                let transform = Affine::translate((gx, gy))
                                    * Affine::scale_non_uniform(glyph_k, -glyph_k);
                                ctx.set_transform(transform);
                                ctx.fill_path(&outline);
                            }
                            cur_adv += advance_pt;
                        }
                        DrawableGlyph::FallbackChar { ch, advance_pt } => {
                            let (fallback_path, is_box) = build_fallback_glyph_path(
                                ch,
                                *font_size_pt * scale,
                                advance_pt * scale,
                            );
                            let gx = origin_x + cur_adv as f64 * s;
                            let gy = origin_y;

                            ctx.set_transform(Affine::translate((gx, gy)));
                            if is_box {
                                let stroke_st = kurbo::Stroke::new(1.0);
                                ctx.set_stroke(stroke_st);
                                ctx.stroke_path(&fallback_path);
                            } else {
                                ctx.fill_path(&fallback_path);
                            }
                            cur_adv += advance_pt;
                        }
                    }
                }
            }
            SceneElement::Chevron { x, y, size, color } => {
                let half_h = *size as f64 * 0.8 * s;
                let cx = *x as f64 * s;
                let cy = *y as f64 * s;
                let tip_x = cx + *size as f64 * s;

                let mut path = BezPath::new();
                path.move_to(Point::new(cx, cy - half_h));
                path.line_to(Point::new(tip_x, cy));
                path.line_to(Point::new(cx, cy + half_h));

                ctx.set_transform(Affine::IDENTITY);
                ctx.set_paint(to_peniko_color(*color));
                let stroke_style = kurbo::Stroke::new(2.5 * s)
                    .with_caps(kurbo::Cap::Round)
                    .with_join(kurbo::Join::Round);
                ctx.set_stroke(stroke_style);
                ctx.stroke_path(&path);
            }
        }
    }

    ctx.flush();
    let mut pixels = vec![0u8; px_w * px_h * 4];
    let mut resources = Resources::new();
    if let Some(pm) = PixmapMut::new(u16_w, u16_h, &mut pixels) {
        ctx.render(pm, &mut resources);
    } else {
        return Err(DiagramError::RasterError(
            "failed to allocate vello_cpu PixmapMut buffer".to_string(),
        ));
    }

    let mut png_bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png_bytes)
        .write_image(
            &pixels,
            px_w as u32,
            px_h as u32,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| DiagramError::RasterError(e.to_string()))?;

    Ok(png_bytes)
}
