//! ECMA-376 DrawingML (`<wgp:wgp>`) compiler for `PositionedScene`.

use std::fmt::Write as _;

use crate::compile_svg::escape_xml;
use crate::layout::{HAlign, PositionedScene, SceneElement};
use crate::spec::RgbColor;

/// Points to English Metric Units (EMU) conversion ratio (`1 pt = 12,700 EMU`, ECMA-376).
pub const PT_TO_EMU: f64 = 12_700.0;

/// Points to Half-Points / Twips for OpenXML text sizes (`1 pt = 2 half-points` in Word `w:sz`).
pub const PT_TO_HALF_POINTS: f32 = 2.0;

/// Converts points to integer EMUs.
#[must_use]
pub fn pt_to_emu(pt: f32) -> i64 {
    (pt as f64 * PT_TO_EMU).round() as i64
}

/// Formats an [`RgbColor`] as an ECMA-376 hex string (`RRGGBB`).
#[must_use]
pub fn color_to_drawingml_hex(c: RgbColor) -> String {
    format!("{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

/// Compiles a [`PositionedScene`] into standard ECMA-376 DrawingML `<wgp:wgp>` XML.
#[must_use]
pub fn compile_drawingml(scene: &PositionedScene) -> String {
    let mut xml = String::with_capacity(8192);
    let total_cx = pt_to_emu(scene.width_pt);
    let total_cy = pt_to_emu(scene.height_pt);

    let _ = writeln!(
        xml,
        r#"<wgp:wgp xmlns:wgp="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#
    );
    let _ = writeln!(xml, r#"  <wgp:grpSpPr>"#);
    let _ = writeln!(
        xml,
        r#"    <a:xfrm><a:off x="0" y="0"/><a:ext cx="{total_cx}" cy="{total_cy}"/><a:chOff x="0" y="0"/><a:chExt cx="{total_cx}" cy="{total_cy}"/></a:xfrm>"#
    );
    let _ = writeln!(xml, r#"  </wgp:grpSpPr>"#);

    for (idx, elem) in scene.elements.iter().enumerate() {
        let shape_id = idx + 100;
        match elem {
            SceneElement::Rect {
                rect,
                fill,
                stroke,
                corner_radius,
            } => {
                let off_x = pt_to_emu(rect.x);
                let off_y = pt_to_emu(rect.y);
                let ext_cx = pt_to_emu(rect.w);
                let ext_cy = pt_to_emu(rect.h);
                let geom_prst = if *corner_radius > 0.0 { "roundRect" } else { "rect" };

                let _ = writeln!(xml, r#"  <wps:wsp>"#);
                let _ = writeln!(
                    xml,
                    r#"    <wps:cNvSpPr id="{shape_id}" name="Card {idx}"/>"#
                );
                let _ = writeln!(xml, r#"    <wps:spPr>"#);
                let _ = writeln!(
                    xml,
                    r#"      <a:xfrm><a:off x="{off_x}" y="{off_y}"/><a:ext cx="{ext_cx}" cy="{ext_cy}"/></a:xfrm>"#
                );
                let _ = writeln!(xml, r#"      <a:prstGeom prst="{geom_prst}"><a:avLst/></a:prstGeom>"#);

                if let Some(fill_c) = fill {
                    let hex = color_to_drawingml_hex(*fill_c);
                    let _ = writeln!(xml, r#"      <a:solidFill><a:srgbClr val="{hex}"/></a:solidFill>"#);
                } else {
                    let _ = writeln!(xml, r#"      <a:noFill/>"#);
                }

                if let Some((stroke_c, sw)) = stroke {
                    let hex = color_to_drawingml_hex(*stroke_c);
                    let line_emu = pt_to_emu(*sw);
                    let _ = writeln!(
                        xml,
                        r#"      <a:ln w="{line_emu}"><a:solidFill><a:srgbClr val="{hex}"/></a:solidFill></a:ln>"#
                    );
                }

                let _ = writeln!(xml, r#"    </wps:spPr>"#);
                let _ = writeln!(xml, r#"  </wps:wsp>"#);
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
                let off_x = pt_to_emu(*x);
                let off_y = pt_to_emu(*y - (*font_size_pt * 0.85));
                let ext_cx = pt_to_emu(300.0);
                let ext_cy = pt_to_emu(*font_size_pt * 1.5);
                let hex = color_to_drawingml_hex(*color);
                let jc_val = match align {
                    HAlign::Left => "left",
                    HAlign::Center => "center",
                    HAlign::Right => "right",
                };
                let sz_val = (*font_size_pt * PT_TO_HALF_POINTS).round() as i32;
                let bold_tag = if *bold { "<w:b/>" } else { "" };
                let escaped = escape_xml(text);

                let _ = writeln!(xml, r#"  <wps:wsp>"#);
                let _ = writeln!(
                    xml,
                    r#"    <wps:cNvSpPr id="{shape_id}" name="Text {idx}" txBox="1"/>"#
                );
                let _ = writeln!(xml, r#"    <wps:spPr>"#);
                let _ = writeln!(
                    xml,
                    r#"      <a:xfrm><a:off x="{off_x}" y="{off_y}"/><a:ext cx="{ext_cx}" cy="{ext_cy}"/></a:xfrm>"#
                );
                let _ = writeln!(xml, r#"      <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>"#);
                let _ = writeln!(xml, r#"      <a:noFill/>"#);
                let _ = writeln!(xml, r#"    </wps:spPr>"#);
                let _ = writeln!(
                    xml,
                    r#"    <wps:txbx><w:txbxContent><w:p><w:pPr><w:jc val="{jc_val}"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Pretendard" w:eastAsia="Pretendard"/>{bold_tag}<w:sz val="{sz_val}"/><w:color w:val="{hex}"/></w:rPr><w:t>{escaped}</w:t></w:r></w:p></w:txbxContent></wps:txbx>"#
                );
                let _ = writeln!(xml, r#"  </wps:wsp>"#);
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
                let off_x = pt_to_emu(x1.min(*x2));
                let off_y = pt_to_emu(y1.min(*y2));
                let ext_cx = pt_to_emu((x2 - x1).abs().max(1.0));
                let ext_cy = pt_to_emu((y2 - y1).abs().max(1.0));
                let hex = color_to_drawingml_hex(*color);
                let line_emu = pt_to_emu(*width);
                let prst_dash = if *dashed { r#"<a:prstDash val="dash"/>"# } else { "" };

                let _ = writeln!(xml, r#"  <wps:wsp>"#);
                let _ = writeln!(
                    xml,
                    r#"    <wps:cNvSpPr id="{shape_id}" name="Line {idx}"/>"#
                );
                let _ = writeln!(xml, r#"    <wps:spPr>"#);
                let _ = writeln!(
                    xml,
                    r#"      <a:xfrm><a:off x="{off_x}" y="{off_y}"/><a:ext cx="{ext_cx}" cy="{ext_cy}"/></a:xfrm>"#
                );
                let _ = writeln!(xml, r#"      <a:prstGeom prst="line"><a:avLst/></a:prstGeom>"#);
                let _ = writeln!(
                    xml,
                    r#"      <a:ln w="{line_emu}"><a:solidFill><a:srgbClr val="{hex}"/></a:solidFill>{prst_dash}</a:ln>"#
                );
                let _ = writeln!(xml, r#"    </wps:spPr>"#);
                let _ = writeln!(xml, r#"  </wps:wsp>"#);
            }
            SceneElement::Arrow {
                x1,
                y1,
                x2,
                y2,
                color,
                width,
            } => {
                let off_x = pt_to_emu(x1.min(*x2));
                let off_y = pt_to_emu(y1.min(*y2));
                let ext_cx = pt_to_emu((x2 - x1).abs().max(1.0));
                let ext_cy = pt_to_emu((y2 - y1).abs().max(1.0));
                let hex = color_to_drawingml_hex(*color);
                let line_emu = pt_to_emu(*width);

                let _ = writeln!(xml, r#"  <wps:wsp>"#);
                let _ = writeln!(
                    xml,
                    r#"    <wps:cNvSpPr id="{shape_id}" name="Arrow {idx}"/>"#
                );
                let _ = writeln!(xml, r#"    <wps:spPr>"#);
                let _ = writeln!(
                    xml,
                    r#"      <a:xfrm><a:off x="{off_x}" y="{off_y}"/><a:ext cx="{ext_cx}" cy="{ext_cy}"/></a:xfrm>"#
                );
                let _ = writeln!(xml, r#"      <a:prstGeom prst="straightConnector1"><a:avLst/></a:prstGeom>"#);
                let _ = writeln!(
                    xml,
                    r#"      <a:ln w="{line_emu}"><a:solidFill><a:srgbClr val="{hex}"/></a:solidFill><a:headEnd type="triangle" w="med" len="med"/></a:ln>"#
                );
                let _ = writeln!(xml, r#"    </wps:spPr>"#);
                let _ = writeln!(xml, r#"  </wps:wsp>"#);
            }
            SceneElement::PillBadge {
                rect,
                bg_color,
                text,
                text_color,
                font_size_pt,
            } => {
                let off_x = pt_to_emu(rect.x);
                let off_y = pt_to_emu(rect.y);
                let ext_cx = pt_to_emu(rect.w);
                let ext_cy = pt_to_emu(rect.h);
                let bg_hex = color_to_drawingml_hex(*bg_color);
                let text_hex = color_to_drawingml_hex(*text_color);
                let sz_val = (*font_size_pt * PT_TO_HALF_POINTS).round() as i32;
                let escaped = escape_xml(text);

                let _ = writeln!(xml, r#"  <wps:wsp>"#);
                let _ = writeln!(
                    xml,
                    r#"    <wps:cNvSpPr id="{shape_id}" name="Badge {idx}"/>"#
                );
                let _ = writeln!(xml, r#"    <wps:spPr>"#);
                let _ = writeln!(
                    xml,
                    r#"      <a:xfrm><a:off x="{off_x}" y="{off_y}"/><a:ext cx="{ext_cx}" cy="{ext_cy}"/></a:xfrm>"#
                );
                let _ = writeln!(xml, r#"      <a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom>"#);
                let _ = writeln!(xml, r#"      <a:solidFill><a:srgbClr val="{bg_hex}"/></a:solidFill>"#);
                let _ = writeln!(xml, r#"    </wps:spPr>"#);
                let _ = writeln!(
                    xml,
                    r#"    <wps:txbx><w:txbxContent><w:p><w:pPr><w:jc val="center"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Pretendard" w:eastAsia="Pretendard"/><w:b/><w:sz val="{sz_val}"/><w:color w:val="{text_hex}"/></w:rPr><w:t>{escaped}</w:t></w:r></w:p></w:txbxContent></wps:txbx>"#
                );
                let _ = writeln!(xml, r#"  </wps:wsp>"#);
            }
            SceneElement::Chevron { x, y, size, color } => {
                let off_x = pt_to_emu(*x);
                let off_y = pt_to_emu(*y - *size);
                let ext_cx = pt_to_emu(*size);
                let ext_cy = pt_to_emu(*size * 2.0);
                let hex = color_to_drawingml_hex(*color);

                let _ = writeln!(xml, r#"  <wps:wsp>"#);
                let _ = writeln!(
                    xml,
                    r#"    <wps:cNvSpPr id="{shape_id}" name="Chevron {idx}"/>"#
                );
                let _ = writeln!(xml, r#"    <wps:spPr>"#);
                let _ = writeln!(
                    xml,
                    r#"      <a:xfrm><a:off x="{off_x}" y="{off_y}"/><a:ext cx="{ext_cx}" cy="{ext_cy}"/></a:xfrm>"#
                );
                let _ = writeln!(xml, r#"      <a:prstGeom prst="chevron"><a:avLst/></a:prstGeom>"#);
                let _ = writeln!(xml, r#"      <a:solidFill><a:srgbClr val="{hex}"/></a:solidFill>"#);
                let _ = writeln!(xml, r#"    </wps:spPr>"#);
                let _ = writeln!(xml, r#"  </wps:wsp>"#);
            }
        }
    }

    let _ = writeln!(xml, r#"</wgp:wgp>"#);
    xml
}
