//! Vector SVG compiler for `PositionedScene`.

use std::fmt::Write as _;

use crate::layout::{HAlign, PositionedScene, SceneElement};
use crate::spec::RgbColor;

/// Escapes XML special characters (`&`, `<`, `>`, `"`, `'`).
#[must_use]
pub fn escape_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Standard CJK/Hangul-first CSS font stack for SVG rendering.
pub const SVG_FONT_STACK: &str =
    "-apple-system, BlinkMacSystemFont, 'Pretendard', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif";

/// Compiles a [`PositionedScene`] into a clean, standalone vector SVG string.
#[must_use]
pub fn compile_svg(scene: &PositionedScene) -> String {
    let mut svg = String::with_capacity(4096);
    let w = scene.width_pt;
    let h = scene.height_pt;

    let _ = writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:.2} {h:.2}" width="{w:.2}pt" height="{h:.2}pt">"#
    );
    let _ = writeln!(
        svg,
        r#"  <defs>
    <style>
      text {{
        font-family: {SVG_FONT_STACK};
        user-select: none;
      }}
    </style>
  </defs>"#
    );

    for element in &scene.elements {
        match element {
            SceneElement::Rect {
                rect,
                fill,
                stroke,
                corner_radius,
            } => {
                let fill_attr = match fill {
                    Some(c) => format_color_attr("fill", *c),
                    None => "fill=\"none\"".to_string(),
                };
                let stroke_attr = match stroke {
                    Some((c, sw)) => format!("{} stroke-width=\"{:.2}\"", format_color_attr("stroke", *c), sw),
                    None => "stroke=\"none\"".to_string(),
                };
                let rx_attr = if *corner_radius > 0.0 {
                    format!(" rx=\"{:.2}\" ry=\"{:.2}\"", corner_radius, corner_radius)
                } else {
                    String::new()
                };

                let _ = writeln!(
                    svg,
                    r#"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"{rx_attr} {fill_attr} {stroke_attr}/>"#,
                    rect.x, rect.y, rect.w, rect.h
                );
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
                let anchor = match align {
                    HAlign::Left => "start",
                    HAlign::Center => "middle",
                    HAlign::Right => "end",
                };
                let weight = if *bold { "700" } else { "400" };
                let color_attr = format_color_attr("fill", *color);
                let escaped = escape_xml(text);

                let _ = writeln!(
                    svg,
                    r#"  <text x="{x:.2}" y="{y:.2}" font-size="{font_size_pt:.2}" font-weight="{weight}" text-anchor="{anchor}" {color_attr}>{escaped}</text>"#
                );
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
                let color_attr = format_color_attr("stroke", *color);
                let dash_attr = if *dashed {
                    " stroke-dasharray=\"4 3\""
                } else {
                    ""
                };
                let _ = writeln!(
                    svg,
                    r#"  <line x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}" stroke-width="{width:.2}"{dash_attr} stroke-linecap="round" {color_attr}/>"#
                );
            }
            SceneElement::Arrow {
                x1,
                y1,
                x2,
                y2,
                color,
                width,
            } => {
                let dx = x2 - x1;
                let dy = y2 - y1;
                let len = (dx * dx + dy * dy).sqrt();
                let color_attr = format_color_attr("stroke", *color);
                let fill_attr = format_color_attr("fill", *color);

                if len <= 0.001 {
                    continue;
                }

                let nx = dx / len;
                let ny = dy / len;
                let head_len = (*width * 2.8).clamp(6.0, 14.0);
                let head_half_w = head_len * 0.45;

                // Stop line slightly before the arrowhead tip
                let end_x = x2 - nx * head_len * 0.7;
                let end_y = y2 - ny * head_len * 0.7;

                let _ = writeln!(
                    svg,
                    r#"  <line x1="{x1:.2}" y1="{y1:.2}" x2="{end_x:.2}" y2="{end_y:.2}" stroke-width="{width:.2}" stroke-linecap="round" {color_attr}/>"#
                );

                // Arrowhead triangle polygon
                let base_x = x2 - nx * head_len;
                let base_y = y2 - ny * head_len;
                let p1_x = base_x - ny * head_half_w;
                let p1_y = base_y + nx * head_half_w;
                let p2_x = base_x + ny * head_half_w;
                let p2_y = base_y - nx * head_half_w;

                let _ = writeln!(
                    svg,
                    r#"  <polygon points="{x2:.2},{y2:.2} {p1_x:.2},{p1_y:.2} {p2_x:.2},{p2_y:.2}" {fill_attr}/>"#
                );
            }
            SceneElement::PillBadge {
                rect,
                bg_color,
                text,
                text_color,
                font_size_pt,
            } => {
                let rx = rect.h / 2.0;
                let bg_attr = format_color_attr("fill", *bg_color);
                let text_color_attr = format_color_attr("fill", *text_color);
                let escaped = escape_xml(text);
                let center_x = rect.x + rect.w / 2.0;
                let baseline_y = rect.y + rect.h * 0.7;

                let _ = writeln!(
                    svg,
                    r#"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{rx:.2}" ry="{rx:.2}" {bg_attr}/>"#,
                    rect.x, rect.y, rect.w, rect.h
                );
                let _ = writeln!(
                    svg,
                    r#"  <text x="{center_x:.2}" y="{baseline_y:.2}" font-size="{font_size_pt:.2}" font-weight="700" text-anchor="middle" {text_color_attr}>{escaped}</text>"#
                );
            }
            SceneElement::Chevron { x, y, size, color } => {
                let color_attr = format_color_attr("stroke", *color);
                let half_h = size * 0.8;
                let tip_x = x + size;
                let _ = writeln!(
                    svg,
                    r#"  <path d="M {x:.2} {:.2} L {tip_x:.2} {y:.2} L {x:.2} {:.2}" fill="none" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" {color_attr}/>"#,
                    y - half_h,
                    y + half_h
                );
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn format_color_attr(attr_name: &str, color: RgbColor) -> String {
    if color.a == 255 {
        format!("{attr_name}=\"{}\"", color.to_hex())
    } else {
        let opacity = color.a as f32 / 255.0;
        format!(
            "{attr_name}=\"{}\" {attr_name}-opacity=\"{opacity:.3}\"",
            color.to_hex()
        )
    }
}
