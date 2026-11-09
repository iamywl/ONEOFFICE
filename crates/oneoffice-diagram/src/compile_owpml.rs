//! KS X 6101 OWPML `<hp:container>` compiler for `PositionedScene`.

use std::fmt::Write as _;

use crate::compile_svg::escape_xml;
use crate::layout::{HAlign, PositionedScene, SceneElement};
use crate::spec::RgbColor;

/// Points to HWPUNIT conversion ratio (`1 pt = 100 HWPUNIT`, KS X 6101 standard).
pub const PT_TO_HWPUNIT: f32 = 100.0;

/// Converts points to integer HWPUNITs.
#[must_use]
pub fn pt_to_hwpunit(pt: f32) -> i32 {
    (pt * PT_TO_HWPUNIT).round() as i32
}

/// Formats an [`RgbColor`] as a KS X 6101 hex color string (`#RRGGBB`).
#[must_use]
pub fn color_to_owpml_hex(c: RgbColor) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
}

/// Compiles a [`PositionedScene`] into standard KS X 6101 OWPML `<hp:container>` XML.
#[must_use]
pub fn compile_owpml(scene: &PositionedScene) -> String {
    let mut xml = String::with_capacity(8192);
    let total_w = pt_to_hwpunit(scene.width_pt);
    let total_h = pt_to_hwpunit(scene.height_pt);

    let _ = writeln!(
        xml,
        r#"<hp:container xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core" id="oneoffice-diagram" zOrder="0" treatAsChar="1" width="{total_w}" height="{total_h}">"#
    );
    let _ = writeln!(xml, r#"  <hp:sz width="{total_w}" height="{total_h}"/>"#);
    let _ = writeln!(
        xml,
        r#"  <hp:pos treatAsChar="1"><hp:point x="0" y="0"/></hp:pos>"#
    );
    let _ = writeln!(xml, r#"  <hp:children>"#);

    for (idx, elem) in scene.elements.iter().enumerate() {
        let z_order = idx as i32 + 1;
        match elem {
            SceneElement::Rect {
                rect,
                fill,
                stroke,
                corner_radius,
            } => {
                let x = pt_to_hwpunit(rect.x);
                let y = pt_to_hwpunit(rect.y);
                let w = pt_to_hwpunit(rect.w);
                let h = pt_to_hwpunit(rect.h);
                let curve = if *corner_radius > 0.0 { "ROUND" } else { "NORMAL" };

                let _ = writeln!(
                    xml,
                    r#"    <hp:rect id="rect_{idx}" zOrder="{z_order}" curvetype="{curve}" width="{w}" height="{h}">"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:pos treatAsChar="0"><hp:point x="{x}" y="{y}"/></hp:pos>"#
                );
                let _ = writeln!(xml, r#"      <hp:sz width="{w}" height="{h}"/>"#);

                if let Some((stroke_color, stroke_w)) = stroke {
                    let stroke_hex = color_to_owpml_hex(*stroke_color);
                    let line_w = pt_to_hwpunit(*stroke_w);
                    let _ = writeln!(
                        xml,
                        r#"      <hp:lineShape color="{stroke_hex}" width="{line_w}" style="SOLID"/>"#
                    );
                } else {
                    let _ = writeln!(xml, r#"      <hp:lineShape color="none" width="0" style="NONE"/>"#);
                }

                if let Some(fill_color) = fill {
                    let fill_hex = color_to_owpml_hex(*fill_color);
                    let _ = writeln!(
                        xml,
                        r#"      <hc:fillBrush><hc:winBrush faceColor="{fill_hex}"/></hc:fillBrush>"#
                    );
                } else {
                    let _ = writeln!(xml, r#"      <hc:fillBrush><hc:winBrush faceColor="none"/></hc:fillBrush>"#);
                }

                let _ = writeln!(xml, r#"    </hp:rect>"#);
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
                let hwp_x = pt_to_hwpunit(*x);
                let hwp_y = pt_to_hwpunit(*y - (*font_size_pt * 0.85));
                let font_hwp_sz = (*font_size_pt * 100.0).round() as i32;
                let color_hex = color_to_owpml_hex(*color);
                let align_str = match align {
                    HAlign::Left => "LEFT",
                    HAlign::Center => "CENTER",
                    HAlign::Right => "RIGHT",
                };
                let bold_attr = if *bold { r#" bold="1""# } else { "" };
                let escaped = escape_xml(text);

                let _ = writeln!(
                    xml,
                    r#"    <hp:drawText id="txt_{idx}" zOrder="{z_order}">"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:pos treatAsChar="0"><hp:point x="{hwp_x}" y="{hwp_y}"/></hp:pos>"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:subList><hp:p align="{align_str}"><hp:run{bold_attr} sz="{font_hwp_sz}" color="{color_hex}"><hp:t>{escaped}</hp:t></hp:run></hp:p></hp:subList>"#
                );
                let _ = writeln!(xml, r#"    </hp:drawText>"#);
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
                let sx = pt_to_hwpunit(*x1);
                let sy = pt_to_hwpunit(*y1);
                let ex = pt_to_hwpunit(*x2);
                let ey = pt_to_hwpunit(*y2);
                let line_w = pt_to_hwpunit(*width);
                let color_hex = color_to_owpml_hex(*color);
                let style = if *dashed { "DASH" } else { "SOLID" };

                let _ = writeln!(
                    xml,
                    r#"    <hp:line id="line_{idx}" zOrder="{z_order}" isPointList="0">"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:startPt x="{sx}" y="{sy}"/><hp:endPt x="{ex}" y="{ey}"/>"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:lineShape color="{color_hex}" width="{line_w}" style="{style}"/>"#
                );
                let _ = writeln!(xml, r#"    </hp:line>"#);
            }
            SceneElement::Arrow {
                x1,
                y1,
                x2,
                y2,
                color,
                width,
            } => {
                let sx = pt_to_hwpunit(*x1);
                let sy = pt_to_hwpunit(*y1);
                let ex = pt_to_hwpunit(*x2);
                let ey = pt_to_hwpunit(*y2);
                let line_w = pt_to_hwpunit(*width);
                let color_hex = color_to_owpml_hex(*color);

                let _ = writeln!(
                    xml,
                    r#"    <hp:connectLine id="arrow_{idx}" zOrder="{z_order}" type="STRAIGHT">"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:startPt x="{sx}" y="{sy}"/><hp:endPt x="{ex}" y="{ey}"/>"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:lineShape color="{color_hex}" width="{line_w}" style="SOLID" headStyle="ARROW"/>"#
                );
                let _ = writeln!(xml, r#"    </hp:connectLine>"#);
            }
            SceneElement::PillBadge {
                rect,
                bg_color,
                text,
                text_color,
                font_size_pt,
            } => {
                let x = pt_to_hwpunit(rect.x);
                let y = pt_to_hwpunit(rect.y);
                let w = pt_to_hwpunit(rect.w);
                let h = pt_to_hwpunit(rect.h);
                let bg_hex = color_to_owpml_hex(*bg_color);
                let text_hex = color_to_owpml_hex(*text_color);
                let font_sz = (*font_size_pt * 100.0).round() as i32;
                let escaped = escape_xml(text);

                let _ = writeln!(
                    xml,
                    r#"    <hp:rect id="badge_{idx}" zOrder="{z_order}" curvetype="ROUND" width="{w}" height="{h}">"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:pos treatAsChar="0"><hp:point x="{x}" y="{y}"/></hp:pos>"#
                );
                let _ = writeln!(xml, r#"      <hp:sz width="{w}" height="{h}"/>"#);
                let _ = writeln!(xml, r#"      <hp:lineShape color="none" width="0" style="NONE"/>"#);
                let _ = writeln!(
                    xml,
                    r#"      <hc:fillBrush><hc:winBrush faceColor="{bg_hex}"/></hc:fillBrush>"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:drawText><hp:subList><hp:p align="CENTER"><hp:run bold="1" sz="{font_sz}" color="{text_hex}"><hp:t>{escaped}</hp:t></hp:run></hp:p></hp:subList></hp:drawText>"#
                );
                let _ = writeln!(xml, r#"    </hp:rect>"#);
            }
            SceneElement::Chevron { x, y, size, color } => {
                let cx = pt_to_hwpunit(*x);
                let cy = pt_to_hwpunit(*y);
                let s = pt_to_hwpunit(*size);
                let color_hex = color_to_owpml_hex(*color);

                let _ = writeln!(
                    xml,
                    r#"    <hp:polygon id="chv_{idx}" zOrder="{z_order}">"#
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:point x="{cx}" y="{cy}"/><hp:point x="{}" y="{cy}"/><hp:point x="{cx}" y="{}"/>"#,
                    cx + s,
                    cy + s
                );
                let _ = writeln!(
                    xml,
                    r#"      <hp:lineShape color="{color_hex}" width="200" style="SOLID"/>"#
                );
                let _ = writeln!(xml, r#"    </hp:polygon>"#);
            }
        }
    }

    let _ = writeln!(xml, r#"  </hp:children>"#);
    let _ = writeln!(xml, r#"</hp:container>"#);
    xml
}
