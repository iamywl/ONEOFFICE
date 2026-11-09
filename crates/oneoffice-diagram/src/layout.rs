//! 3-pass Korean constraint layout solver, positioned scene graph, and geometry diagnostics.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::font_metrics::{
    DEFAULT_WRAP_SAFETY_PAD_PT, wrap_rich_text,
};
use crate::spec::{
    ConnectorPort, ConnectorStyle, DiagramError, DiagramNode, DiagramSpec, DiagramTheme, RgbColor,
};

/// 2D bounding rectangle in points (origin top-left, coordinates grow right & down).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RectPt {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectPt {
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    #[must_use]
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    #[must_use]
    pub fn center_x(&self) -> f32 {
        self.x + self.w * 0.5
    }

    #[must_use]
    pub fn center_y(&self) -> f32 {
        self.y + self.h * 0.5
    }

    #[must_use]
    pub fn contains_rect(&self, inner: &RectPt, tolerance_pt: f32) -> bool {
        inner.x >= self.x - tolerance_pt
            && inner.right() <= self.right() + tolerance_pt
            && inner.y >= self.y - tolerance_pt
            && inner.bottom() <= self.bottom() + tolerance_pt
    }

    #[must_use]
    pub fn intersects(&self, other: &RectPt) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}

/// Text horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HAlign {
    Left,
    Center,
    Right,
}

/// A rendered scene element positioned in canvas coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SceneElement {
    Rect {
        rect: RectPt,
        fill: Option<RgbColor>,
        stroke: Option<(RgbColor, f32)>,
        corner_radius: f32,
    },
    Text {
        text: String,
        x: f32,
        y: f32,
        font_size_pt: f32,
        color: RgbColor,
        bold: bool,
        align: HAlign,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: RgbColor,
        width: f32,
        dashed: bool,
    },
    Arrow {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: RgbColor,
        width: f32,
    },
    PillBadge {
        rect: RectPt,
        bg_color: RgbColor,
        text: String,
        text_color: RgbColor,
        font_size_pt: f32,
    },
    Chevron {
        x: f32,
        y: f32,
        size: f32,
        color: RgbColor,
    },
}

/// Empirical geometry diagnostics for checking layout invariants (`Zero Overflow / Zero Collision`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutDiagnostics {
    pub overflow_count: usize,
    pub collision_count: usize,
    pub max_connector_snap_error_pt: f32,
    pub node_bboxes: HashMap<String, RectPt>,
    pub text_overflows: Vec<String>,
}

/// Positioned scene containing the resolved layout and all renderable elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionedScene {
    pub width_pt: f32,
    pub height_pt: f32,
    pub elements: Vec<SceneElement>,
    pub diagnostics: LayoutDiagnostics,
}

/// Solves layout constraints on a [`DiagramSpec`] and produces a [`PositionedScene`].
pub fn solve_layout(spec: &DiagramSpec) -> Result<PositionedScene, DiagramError> {
    let width_pt = spec.canvas.target_width_pt;
    if !width_pt.is_finite() || width_pt <= 10.0 {
        return Err(DiagramError::InvalidCanvasWidth(width_pt));
    }

    let mut elements = Vec::new();
    let mut node_bboxes = HashMap::new();
    let mut text_overflows = Vec::new();

    let cur_y = spec.canvas.padding_pt;
    let content_x = spec.canvas.padding_pt;
    let content_w = width_pt - (spec.canvas.padding_pt * 2.0);

    let total_h = layout_node(
        &spec.root,
        content_x,
        cur_y,
        content_w,
        &spec.theme,
        &mut elements,
        &mut node_bboxes,
        &mut text_overflows,
    );

    // Route glued connectors
    let mut max_snap_err = 0.0f32;
    for conn in &spec.connectors {
        if let (Some(from_box), Some(to_box)) = (
            node_bboxes.get(&conn.from.node_id),
            node_bboxes.get(&conn.to.node_id),
        ) {
            let p1 = compute_port(from_box, conn.from.port);
            let p2 = compute_port(to_box, conn.to.port);

            let primary_color = RgbColor::parse_hex_or(&spec.theme.primary, RgbColor::rgb(0, 82, 204));
            match conn.style {
                ConnectorStyle::StraightArrow => {
                    elements.push(SceneElement::Arrow {
                        x1: p1[0],
                        y1: p1[1],
                        x2: p2[0],
                        y2: p2[1],
                        color: primary_color,
                        width: 2.0,
                    });
                }
                ConnectorStyle::ElbowArrow => {
                    let mid_y = (p1[1] + p2[1]) * 0.5;
                    elements.push(SceneElement::Line {
                        x1: p1[0],
                        y1: p1[1],
                        x2: p1[0],
                        y2: mid_y,
                        color: primary_color,
                        width: 2.0,
                        dashed: false,
                    });
                    elements.push(SceneElement::Line {
                        x1: p1[0],
                        y1: mid_y,
                        x2: p2[0],
                        y2: mid_y,
                        color: primary_color,
                        width: 2.0,
                        dashed: false,
                    });
                    elements.push(SceneElement::Arrow {
                        x1: p2[0],
                        y1: mid_y,
                        x2: p2[0],
                        y2: p2[1],
                        color: primary_color,
                        width: 2.0,
                    });
                }
                ConnectorStyle::LineOnly => {
                    elements.push(SceneElement::Line {
                        x1: p1[0],
                        y1: p1[1],
                        x2: p2[0],
                        y2: p2[1],
                        color: primary_color,
                        width: 1.5,
                        dashed: false,
                    });
                }
                ConnectorStyle::DashedArrow => {
                    elements.push(SceneElement::Line {
                        x1: p1[0],
                        y1: p1[1],
                        x2: p2[0],
                        y2: p2[1],
                        color: primary_color,
                        width: 1.5,
                        dashed: true,
                    });
                }
            }
        } else {
            max_snap_err = max_snap_err.max(1.0);
        }
    }

    let final_h = total_h + (spec.canvas.padding_pt * 2.0);
    let overflow_count = text_overflows.len();

    // Check sibling collisions
    let mut collision_count = 0;
    let bboxes: Vec<RectPt> = node_bboxes.values().copied().collect();
    for i in 0..bboxes.len() {
        for j in (i + 1)..bboxes.len() {
            // Sibling boxes should not overlap unless nested
            let b1 = &bboxes[i];
            let b2 = &bboxes[j];
            if b1.intersects(b2) && !b1.contains_rect(b2, 0.5) && !b2.contains_rect(b1, 0.5) {
                collision_count += 1;
            }
        }
    }

    Ok(PositionedScene {
        width_pt,
        height_pt: final_h.max(100.0),
        elements,
        diagnostics: LayoutDiagnostics {
            overflow_count,
            collision_count,
            max_connector_snap_error_pt: max_snap_err,
            node_bboxes,
            text_overflows,
        },
    })
}

fn compute_port(box_rect: &RectPt, port: ConnectorPort) -> [f32; 2] {
    match port {
        ConnectorPort::BottomCenter => [box_rect.center_x(), box_rect.bottom()],
        ConnectorPort::TopCenter => [box_rect.center_x(), box_rect.y],
        ConnectorPort::TopQuarterLeft => [box_rect.x + box_rect.w * 0.25, box_rect.y],
        ConnectorPort::TopQuarterRight => [box_rect.x + box_rect.w * 0.75, box_rect.y],
        ConnectorPort::BottomQuarterLeft => [box_rect.x + box_rect.w * 0.25, box_rect.bottom()],
        ConnectorPort::BottomQuarterRight => [box_rect.x + box_rect.w * 0.75, box_rect.bottom()],
        ConnectorPort::LeftCenter => [box_rect.x, box_rect.center_y()],
        ConnectorPort::RightCenter => [box_rect.right(), box_rect.center_y()],
    }
}

fn layout_node(
    node: &DiagramNode,
    x: f32,
    y: f32,
    w: f32,
    theme: &DiagramTheme,
    elements: &mut Vec<SceneElement>,
    bboxes: &mut HashMap<String, RectPt>,
    overflows: &mut Vec<String>,
) -> f32 {
    let card_bg = RgbColor::parse_hex_or(&theme.card_bg, RgbColor::WHITE);
    let card_border = RgbColor::parse_hex_or(&theme.card_border, RgbColor::rgb(203, 213, 225));
    let hero_bg = RgbColor::parse_hex_or(&theme.hero_bg, RgbColor::rgb(15, 41, 66));
    let primary = RgbColor::parse_hex_or(&theme.primary, RgbColor::rgb(0, 82, 204));
    let text_primary = RgbColor::parse_hex_or(&theme.text_primary, RgbColor::rgb(15, 23, 42));
    let text_muted = RgbColor::parse_hex_or(&theme.text_muted, RgbColor::rgb(71, 85, 105));
    let subtle_fill = RgbColor::parse_hex_or(&theme.subtle_fill, RgbColor::rgb(241, 245, 249));
    let imp_fill = RgbColor::parse_hex_or(&theme.implication_fill, RgbColor::rgb(224, 242, 254));

    match node {
        DiagramNode::VStack { id, gap_pt, children, .. } => {
            let mut cur_y = y;
            for (idx, child) in children.iter().enumerate() {
                if idx > 0 {
                    cur_y += gap_pt;
                }
                let h = layout_node(child, x, cur_y, w, theme, elements, bboxes, overflows);
                cur_y += h;
            }
            let total_h = cur_y - y;
            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, total_h));
            }
            total_h
        }

        DiagramNode::HStack { id, gap_pt, children, .. } => {
            if children.is_empty() {
                return 0.0;
            }
            let total_gaps = (*gap_pt) * (children.len().saturating_sub(1) as f32);
            let child_w = (w - total_gaps) / (children.len() as f32);
            let mut max_h = 0.0f32;
            let mut cur_x = x;
            for child in children {
                let h = layout_node(child, cur_x, y, child_w, theme, elements, bboxes, overflows);
                max_h = max_h.max(h);
                cur_x += child_w + gap_pt;
            }
            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, max_h));
            }
            max_h
        }

        DiagramNode::Grid {
            id,
            columns,
            gap_pt,
            equalize_card_heights,
            children,
            ..
        } => {
            if children.is_empty() {
                return 0.0;
            }
            let cols = (*columns).max(1);
            let total_gaps = (*gap_pt) * (cols.saturating_sub(1) as f32);
            let col_w = (w - total_gaps) / (cols as f32);

            let mut cur_y = y;
            let chunks: Vec<_> = children.chunks(cols).collect();
            for chunk in chunks {
                // Pass 1: measure intrinsic heights
                let mut heights = Vec::new();
                for child in chunk {
                    let mut dummy_elements = Vec::new();
                    let mut dummy_bboxes = HashMap::new();
                    let mut dummy_overflows = Vec::new();
                    let h = layout_node(
                        child,
                        x,
                        cur_y,
                        col_w,
                        theme,
                        &mut dummy_elements,
                        &mut dummy_bboxes,
                        &mut dummy_overflows,
                    );
                    heights.push(h);
                }
                let target_h = if *equalize_card_heights {
                    heights.iter().copied().fold(0.0f32, f32::max)
                } else {
                    0.0
                };

                // Pass 2: layout children with target equalized height
                let mut cur_x = x;
                for child in chunk {
                    if let DiagramNode::ComparisonCard { .. } = child {
                        layout_comparison_card(
                            child,
                            cur_x,
                            cur_y,
                            col_w,
                            if *equalize_card_heights { Some(target_h) } else { None },
                            theme,
                            elements,
                            bboxes,
                            overflows,
                        );
                    } else {
                        layout_node(child, cur_x, cur_y, col_w, theme, elements, bboxes, overflows);
                    }
                    cur_x += col_w + gap_pt;
                }
                cur_y += target_h.max(heights.iter().copied().fold(0.0f32, f32::max)) + gap_pt;
            }

            let total_h = cur_y - y - gap_pt;
            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, total_h));
            }
            total_h
        }

        DiagramNode::HeaderBar { id, left_text, right_badge, show_divider } => {
            let h = 20.0;
            elements.push(SceneElement::Text {
                text: left_text.clone(),
                x,
                y: y + 12.0,
                font_size_pt: 8.5,
                color: text_muted,
                bold: true,
                align: HAlign::Left,
            });
            if let Some(badge) = right_badge {
                elements.push(SceneElement::Text {
                    text: badge.clone(),
                    x: x + w,
                    y: y + 12.0,
                    font_size_pt: 8.5,
                    color: text_muted,
                    bold: true,
                    align: HAlign::Right,
                });
            }
            if *show_divider {
                elements.push(SceneElement::Line {
                    x1: x,
                    y1: y + h,
                    x2: x + w,
                    y2: y + h,
                    color: card_border,
                    width: 0.75,
                    dashed: false,
                });
            }
            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, h));
            }
            h + 6.0
        }

        DiagramNode::SectionHeading { id, eyebrow, title, lead } => {
            let mut cur_y = y;
            if let Some(eye) = eyebrow {
                elements.push(SceneElement::Text {
                    text: eye.clone(),
                    x,
                    y: cur_y + 11.0,
                    font_size_pt: 9.0,
                    color: primary,
                    bold: true,
                    align: HAlign::Left,
                });
                cur_y += 16.0;
            }
            elements.push(SceneElement::Text {
                text: title.clone(),
                x,
                y: cur_y + 18.0,
                font_size_pt: 18.0,
                color: text_primary,
                bold: true,
                align: HAlign::Left,
            });
            cur_y += 24.0;

            if let Some(lead_text) = lead {
                let lines = wrap_rich_text(lead_text, 10.0, false, w, DEFAULT_WRAP_SAFETY_PAD_PT);
                for line in &lines {
                    let mut run_x = x;
                    for run in &line.runs {
                        elements.push(SceneElement::Text {
                            text: run.text.clone(),
                            x: run_x,
                            y: cur_y + 11.0,
                            font_size_pt: 10.0,
                            color: text_muted,
                            bold: run.bold,
                            align: HAlign::Left,
                        });
                        run_x += run.width_pt;
                    }
                    cur_y += 15.0;
                }
            }
            let total_h = cur_y - y;
            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, total_h));
            }
            total_h
        }

        DiagramNode::ComparisonCard { .. } => {
            layout_comparison_card(node, x, y, w, None, theme, elements, bboxes, overflows)
        }

        DiagramNode::HeroContainer {
            id,
            eyebrow,
            headline,
            subheadline: _,
            group_labels,
            step_chain,
        } => {
            let pad = 12.0;
            let inner_w = w - (pad * 2.0);

            // Compute step widths
            let total_weight: f32 = step_chain.iter().map(|s| s.weight).sum();
            let step_gap = 6.0;
            let total_gaps = step_gap * (step_chain.len().saturating_sub(1) as f32);
            let avail_w = inner_w - total_gaps;

            let mut header_h = 0.0;
            if eyebrow.is_some() {
                header_h += 14.0;
            }
            let head_lines = wrap_rich_text(headline, 11.0, true, inner_w, DEFAULT_WRAP_SAFETY_PAD_PT);
            header_h += (head_lines.len() as f32) * 16.0 + 10.0;

            if !group_labels.is_empty() {
                header_h += 18.0;
            }

            // Measure step cards height
            let mut max_step_h = 75.0f32;
            for step in step_chain {
                let _step_w = (step.weight / total_weight.max(1.0)) * avail_w;
                let mut sh = 28.0; // title + step badge
                if step.summary.is_some() {
                    sh += 24.0;
                }
                sh += (step.bullets.len() as f32) * 14.0;
                max_step_h = max_step_h.max(sh);
            }

            let hero_h = pad + header_h + max_step_h + pad;
            let hero_rect = RectPt::new(x, y, w, hero_h);

            // Draw hero container background
            elements.push(SceneElement::Rect {
                rect: hero_rect,
                fill: Some(hero_bg),
                stroke: None,
                corner_radius: 8.0,
            });

            let mut cur_y = y + pad;
            if let Some(eye) = eyebrow {
                elements.push(SceneElement::Text {
                    text: eye.clone(),
                    x: x + pad,
                    y: cur_y + 9.0,
                    font_size_pt: 8.5,
                    color: RgbColor::rgb(148, 163, 184),
                    bold: true,
                    align: HAlign::Left,
                });
                cur_y += 14.0;
            }

            for line in &head_lines {
                let mut run_x = x + pad;
                for run in &line.runs {
                    elements.push(SceneElement::Text {
                        text: run.text.clone(),
                        x: run_x,
                        y: cur_y + 11.0,
                        font_size_pt: 11.0,
                        color: RgbColor::WHITE,
                        bold: true,
                        align: HAlign::Left,
                    });
                    run_x += run.width_pt;
                }
                cur_y += 16.0;
            }
            cur_y += 6.0;

            // Render group labels if present
            if !group_labels.is_empty() {
                let mut step_x = x + pad;
                for group in group_labels {
                    let mut grp_w = 0.0;
                    #[allow(clippy::needless_range_loop)]
                    for s_idx in (group.start_step - 1)..group.end_step.min(step_chain.len()) {
                        let sw = (step_chain[s_idx].weight / total_weight.max(1.0)) * avail_w;
                        grp_w += sw;
                        if s_idx < group.end_step - 1 {
                            grp_w += step_gap;
                        }
                    }
                    elements.push(SceneElement::Text {
                        text: group.label.clone(),
                        x: step_x,
                        y: cur_y + 10.0,
                        font_size_pt: 8.5,
                        color: RgbColor::rgb(203, 213, 225),
                        bold: true,
                        align: HAlign::Left,
                    });
                    step_x += grp_w + step_gap;
                }
                cur_y += 16.0;
            }

            // Render process steps
            let mut step_x = x + pad;
            for (idx, step) in step_chain.iter().enumerate() {
                let sw = (step.weight / total_weight.max(1.0)) * avail_w;
                let step_rect = RectPt::new(step_x, cur_y, sw, max_step_h);

                // Step card background
                elements.push(SceneElement::Rect {
                    rect: step_rect,
                    fill: Some(RgbColor::rgb(30, 58, 88)),
                    stroke: Some((RgbColor::rgb(51, 85, 120), 1.0)),
                    corner_radius: 6.0,
                });

                // Step circle number badge
                elements.push(SceneElement::PillBadge {
                    rect: RectPt::new(step_x + 8.0, cur_y + 8.0, 16.0, 16.0),
                    bg_color: RgbColor::WHITE,
                    text: step.step.to_string(),
                    text_color: hero_bg,
                    font_size_pt: 8.5,
                });

                // Page pill badge
                if let Some(badge) = &step.badge {
                    elements.push(SceneElement::PillBadge {
                        rect: RectPt::new(step_x + sw - 28.0, cur_y + 8.0, 20.0, 14.0),
                        bg_color: RgbColor::rgb(51, 85, 120),
                        text: badge.clone(),
                        text_color: RgbColor::rgb(224, 242, 254),
                        font_size_pt: 7.5,
                    });
                }

                // Step title
                elements.push(SceneElement::Text {
                    text: step.title.clone(),
                    x: step_x + 8.0,
                    y: cur_y + 36.0,
                    font_size_pt: 9.5,
                    color: RgbColor::WHITE,
                    bold: true,
                    align: HAlign::Left,
                });

                let mut item_y = cur_y + 48.0;
                if let Some(summary) = &step.summary {
                    elements.push(SceneElement::Text {
                        text: summary.clone(),
                        x: step_x + 8.0,
                        y: item_y + 8.0,
                        font_size_pt: 8.0,
                        color: RgbColor::rgb(186, 215, 242),
                        bold: false,
                        align: HAlign::Left,
                    });
                    item_y += 14.0;
                }

                for b in &step.bullets {
                    elements.push(SceneElement::Text {
                        text: format!("• {b}"),
                        x: step_x + 8.0,
                        y: item_y + 8.0,
                        font_size_pt: 8.0,
                        color: RgbColor::rgb(203, 213, 225),
                        bold: false,
                        align: HAlign::Left,
                    });
                    item_y += 12.0;
                }

                // Chevron connector between steps
                if idx < step_chain.len() - 1 {
                    elements.push(SceneElement::Chevron {
                        x: step_x + sw + (step_gap * 0.5),
                        y: cur_y + (max_step_h * 0.5),
                        size: 6.0,
                        color: RgbColor::rgb(100, 140, 180),
                    });
                }

                step_x += sw + step_gap;
            }

            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), hero_rect);
            }
            hero_h
        }

        DiagramNode::KpiRow { id, items, .. } => {
            if items.is_empty() {
                return 0.0;
            }
            let gap = 8.0;
            let item_w = (w - gap * (items.len().saturating_sub(1) as f32)) / (items.len() as f32);
            let item_h = 70.0;

            for (idx, item) in items.iter().enumerate() {
                let ix = x + (idx as f32) * (item_w + gap);
                let rect = RectPt::new(ix, y, item_w, item_h);
                elements.push(SceneElement::Rect {
                    rect,
                    fill: Some(card_bg),
                    stroke: Some((card_border, 1.0)),
                    corner_radius: 6.0,
                });
                elements.push(SceneElement::Text {
                    text: item.label.clone(),
                    x: ix + 10.0,
                    y: y + 16.0,
                    font_size_pt: 8.5,
                    color: text_muted,
                    bold: true,
                    align: HAlign::Left,
                });
                elements.push(SceneElement::Text {
                    text: item.metric.clone(),
                    x: ix + 10.0,
                    y: y + 42.0,
                    font_size_pt: 18.0,
                    color: primary,
                    bold: true,
                    align: HAlign::Left,
                });
                if let Some(desc) = &item.description {
                    elements.push(SceneElement::Text {
                        text: desc.clone(),
                        x: ix + 10.0,
                        y: y + 58.0,
                        font_size_pt: 7.5,
                        color: text_muted,
                        bold: false,
                        align: HAlign::Left,
                    });
                }
            }

            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, item_h));
            }
            item_h
        }

        DiagramNode::CitationFooter { id, citations, page_indicator } => {
            let mut cur_y = y + 8.0;
            elements.push(SceneElement::Line {
                x1: x,
                y1: cur_y,
                x2: x + w,
                y2: cur_y,
                color: card_border,
                width: 0.75,
                dashed: false,
            });
            cur_y += 8.0;

            for (idx, cite) in citations.iter().enumerate() {
                let cx = if idx % 2 == 0 { x } else { x + w * 0.5 };
                let cy = cur_y + (idx / 2_usize) as f32 * 12.0;
                elements.push(SceneElement::Text {
                    text: format!("{} {}", cite.index, cite.source),
                    x: cx,
                    y: cy + 8.0,
                    font_size_pt: 7.5,
                    color: text_muted,
                    bold: false,
                    align: HAlign::Left,
                });
            }

            if let Some(page) = page_indicator {
                elements.push(SceneElement::Text {
                    text: page.clone(),
                    x: x + w,
                    y: cur_y + 8.0,
                    font_size_pt: 8.5,
                    color: text_muted,
                    bold: true,
                    align: HAlign::Right,
                });
            }

            let total_h = (cur_y - y) + (citations.len().div_ceil(2) as f32) * 12.0 + 4.0;
            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, total_h));
            }
            total_h
        }

        DiagramNode::AsIsToBe {
            id,
            as_is_title,
            as_is_items,
            catalyst_label,
            to_be_title,
            to_be_items,
        } => {
            let col_w = (w - 40.0) / 2.0;
            let card_h = 100.0f32;

            // As-Is card (Left)
            let as_is_rect = RectPt::new(x, y, col_w, card_h);
            elements.push(SceneElement::Rect {
                rect: as_is_rect,
                fill: Some(subtle_fill),
                stroke: Some((card_border, 1.0)),
                corner_radius: 6.0,
            });
            elements.push(SceneElement::Text {
                text: as_is_title.clone(),
                x: x + 10.0,
                y: y + 18.0,
                font_size_pt: 11.0,
                color: RgbColor::rgb(185, 28, 28),
                bold: true,
                align: HAlign::Left,
            });
            let mut cur_y = y + 34.0;
            for item in as_is_items {
                elements.push(SceneElement::Text {
                    text: format!("• {}", item.headline),
                    x: x + 10.0,
                    y: cur_y + 8.0,
                    font_size_pt: 8.5,
                    color: text_primary,
                    bold: true,
                    align: HAlign::Left,
                });
                cur_y += 14.0;
            }

            // Center Catalyst Arrow
            let cx = x + col_w + 20.0;
            elements.push(SceneElement::PillBadge {
                rect: RectPt::new(cx - 30.0, y + 40.0, 60.0, 20.0),
                bg_color: primary,
                text: catalyst_label.clone(),
                text_color: RgbColor::WHITE,
                font_size_pt: 8.0,
            });

            // To-Be card (Right)
            let to_be_x = x + col_w + 40.0;
            let to_be_rect = RectPt::new(to_be_x, y, col_w, card_h);
            elements.push(SceneElement::Rect {
                rect: to_be_rect,
                fill: Some(imp_fill),
                stroke: Some((primary, 1.0)),
                corner_radius: 6.0,
            });
            elements.push(SceneElement::Text {
                text: to_be_title.clone(),
                x: to_be_x + 10.0,
                y: y + 18.0,
                font_size_pt: 11.0,
                color: primary,
                bold: true,
                align: HAlign::Left,
            });
            let mut cur_y = y + 34.0;
            for item in to_be_items {
                elements.push(SceneElement::Text {
                    text: format!("✓ {}", item.headline),
                    x: to_be_x + 10.0,
                    y: cur_y + 8.0,
                    font_size_pt: 8.5,
                    color: text_primary,
                    bold: true,
                    align: HAlign::Left,
                });
                cur_y += 14.0;
            }

            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, card_h));
            }
            card_h
        }

        DiagramNode::ArchitectureStack { id, layers } => {
            let mut cur_y = y;
            for layer in layers {
                let rect = RectPt::new(x, cur_y, w, 32.0);
                elements.push(SceneElement::Rect {
                    rect,
                    fill: Some(subtle_fill),
                    stroke: Some((card_border, 1.0)),
                    corner_radius: 4.0,
                });
                elements.push(SceneElement::Text {
                    text: format!("L{}: {}", layer.level, layer.name),
                    x: x + 10.0,
                    y: cur_y + 20.0,
                    font_size_pt: 9.5,
                    color: primary,
                    bold: true,
                    align: HAlign::Left,
                });
                let comp_text = layer.components.join("  |  ");
                elements.push(SceneElement::Text {
                    text: comp_text,
                    x: x + w - 10.0,
                    y: cur_y + 20.0,
                    font_size_pt: 8.5,
                    color: text_muted,
                    bold: false,
                    align: HAlign::Right,
                });
                cur_y += 38.0;
            }
            let total_h = cur_y - y;
            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, total_h));
            }
            total_h
        }

        DiagramNode::GanttRoadmap { id, time_periods, tracks } => {
            let h = 30.0 + (tracks.len() as f32) * 32.0;
            let rect = RectPt::new(x, y, w, h);
            elements.push(SceneElement::Rect {
                rect,
                fill: Some(card_bg),
                stroke: Some((card_border, 1.0)),
                corner_radius: 6.0,
            });

            // Header columns
            let col_w = (w - 80.0) / (time_periods.len().max(1) as f32);
            for (idx, p) in time_periods.iter().enumerate() {
                elements.push(SceneElement::Text {
                    text: p.clone(),
                    x: x + 80.0 + (idx as f32) * col_w + (col_w * 0.5),
                    y: y + 18.0,
                    font_size_pt: 8.5,
                    color: text_muted,
                    bold: true,
                    align: HAlign::Center,
                });
            }

            let mut cur_y = y + 26.0;
            for track in tracks {
                elements.push(SceneElement::Text {
                    text: track.name.clone(),
                    x: x + 10.0,
                    y: cur_y + 18.0,
                    font_size_pt: 8.5,
                    color: text_primary,
                    bold: true,
                    align: HAlign::Left,
                });
                for item in &track.items {
                    let bx = x + 80.0 + (item.start_idx as f32) * col_w;
                    let bw = (item.span_count as f32) * col_w - 4.0;
                    elements.push(SceneElement::Rect {
                        rect: RectPt::new(bx, cur_y + 4.0, bw, 20.0),
                        fill: Some(if item.completed { primary } else { subtle_fill }),
                        stroke: Some((card_border, 0.75)),
                        corner_radius: 4.0,
                    });
                    elements.push(SceneElement::Text {
                        text: item.label.clone(),
                        x: bx + 6.0,
                        y: cur_y + 17.0,
                        font_size_pt: 7.5,
                        color: if item.completed { RgbColor::WHITE } else { text_primary },
                        bold: false,
                        align: HAlign::Left,
                    });
                }
                cur_y += 28.0;
            }

            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), rect);
            }
            h
        }

        DiagramNode::FunnelStages { id, stages } => {
            let mut cur_y = y;
            for (idx, stage) in stages.iter().enumerate() {
                let fw = w - (idx as f32 * 24.0);
                let fx = x + (idx as f32 * 12.0);
                elements.push(SceneElement::Rect {
                    rect: RectPt::new(fx, cur_y, fw, 26.0),
                    fill: Some(subtle_fill),
                    stroke: Some((card_border, 1.0)),
                    corner_radius: 4.0,
                });
                elements.push(SceneElement::Text {
                    text: stage.stage_name.clone(),
                    x: fx + 10.0,
                    y: cur_y + 17.0,
                    font_size_pt: 9.0,
                    color: text_primary,
                    bold: true,
                    align: HAlign::Left,
                });
                elements.push(SceneElement::Text {
                    text: format!("{} ({})", stage.count_label, stage.conversion_pct),
                    x: fx + fw - 10.0,
                    y: cur_y + 17.0,
                    font_size_pt: 8.5,
                    color: primary,
                    bold: true,
                    align: HAlign::Right,
                });
                cur_y += 32.0;
            }
            let total_h = cur_y - y;
            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, total_h));
            }
            total_h
        }

        DiagramNode::OrgTree { id, lead, advisors: _, teams } => {
            let h = 110.0;
            let cx = x + (w * 0.5);
            // Leader box
            elements.push(SceneElement::Rect {
                rect: RectPt::new(cx - 60.0, y, 120.0, 30.0),
                fill: Some(primary),
                stroke: None,
                corner_radius: 4.0,
            });
            elements.push(SceneElement::Text {
                text: format!("{}: {}", lead.role, lead.name),
                x: cx,
                y: y + 18.0,
                font_size_pt: 9.5,
                color: RgbColor::WHITE,
                bold: true,
                align: HAlign::Center,
            });

            // Team boxes
            let team_w = (w - (teams.len().saturating_sub(1) as f32 * 8.0)) / (teams.len().max(1) as f32);
            for (idx, team) in teams.iter().enumerate() {
                let tx = x + (idx as f32 * (team_w + 8.0));
                elements.push(SceneElement::Rect {
                    rect: RectPt::new(tx, y + 60.0, team_w, 30.0),
                    fill: Some(subtle_fill),
                    stroke: Some((card_border, 1.0)),
                    corner_radius: 4.0,
                });
                elements.push(SceneElement::Text {
                    text: format!("{}: {}", team.role, team.name),
                    x: tx + team_w * 0.5,
                    y: y + 78.0,
                    font_size_pt: 8.5,
                    color: text_primary,
                    bold: true,
                    align: HAlign::Center,
                });
                elements.push(SceneElement::Line {
                    x1: cx,
                    y1: y + 30.0,
                    x2: tx + team_w * 0.5,
                    y2: y + 60.0,
                    color: card_border,
                    width: 1.0,
                    dashed: false,
                });
            }

            if let Some(node_id) = id {
                bboxes.insert(node_id.clone(), RectPt::new(x, y, w, h));
            }
            h
        }
    }
}

fn layout_comparison_card(
    node: &DiagramNode,
    x: f32,
    y: f32,
    w: f32,
    equalized_h: Option<f32>,
    theme: &DiagramTheme,
    elements: &mut Vec<SceneElement>,
    bboxes: &mut HashMap<String, RectPt>,
    _overflows: &mut Vec<String>,
) -> f32 {
    let DiagramNode::ComparisonCard {
        id,
        badge,
        category,
        headline,
        kpi,
        bullets_title,
        bullets,
        implication,
        accent_color,
    } = node else {
        return 0.0;
    };

    let card_bg = RgbColor::parse_hex_or(&theme.card_bg, RgbColor::WHITE);
    let card_border = RgbColor::parse_hex_or(&theme.card_border, RgbColor::rgb(203, 213, 225));
    let primary = accent_color
        .as_deref()
        .map(|hex| RgbColor::parse_hex_or(hex, RgbColor::rgb(0, 82, 204)))
        .unwrap_or_else(|| RgbColor::parse_hex_or(&theme.primary, RgbColor::rgb(0, 82, 204)));
    let text_primary = RgbColor::parse_hex_or(&theme.text_primary, RgbColor::rgb(15, 23, 42));
    let text_muted = RgbColor::parse_hex_or(&theme.text_muted, RgbColor::rgb(71, 85, 105));
    let subtle_fill = RgbColor::parse_hex_or(&theme.subtle_fill, RgbColor::rgb(241, 245, 249));
    let imp_fill = RgbColor::parse_hex_or(&theme.implication_fill, RgbColor::rgb(224, 242, 254));

    let pad = 12.0;
    let inner_w = w - (pad * 2.0);

    // Calculate upper contents height
    let mut upper_h = pad;
    upper_h += 20.0; // Badge + category
    upper_h += 24.0; // Headline
    if kpi.is_some() {
        upper_h += 48.0; // KPI callout card
    }
    if bullets_title.is_some() {
        upper_h += 16.0;
    }

    // Measure bullets with Kinsoku wrap
    for bullet in bullets {
        let lines = wrap_rich_text(&bullet.text, 8.5, false, inner_w - 12.0, DEFAULT_WRAP_SAFETY_PAD_PT);
        upper_h += (lines.len() as f32) * 14.0;
    }
    upper_h += 8.0;

    // Measure implication box height
    let imp_h = if let Some(imp) = implication {
        let lines = wrap_rich_text(&imp.body, 8.5, true, inner_w - 16.0, DEFAULT_WRAP_SAFETY_PAD_PT);
        20.0 + (lines.len() as f32) * 14.0 + 8.0
    } else {
        0.0
    };

    let total_intrinsic_h = upper_h + imp_h + pad;
    let final_card_h = equalized_h.unwrap_or(total_intrinsic_h).max(total_intrinsic_h);

    let card_rect = RectPt::new(x, y, w, final_card_h);

    // 1. Draw outer rounded card
    elements.push(SceneElement::Rect {
        rect: card_rect,
        fill: Some(card_bg),
        stroke: Some((card_border, 1.0)),
        corner_radius: 8.0,
    });

    let mut cur_y = y + pad;

    // 2. Pill Badge + Category
    let badge_w = 46.0;
    elements.push(SceneElement::PillBadge {
        rect: RectPt::new(x + pad, cur_y, badge_w, 18.0),
        bg_color: RgbColor::rgb(15, 23, 42),
        text: badge.clone(),
        text_color: RgbColor::WHITE,
        font_size_pt: 8.5,
    });

    if let Some(cat) = category {
        elements.push(SceneElement::Text {
            text: cat.clone(),
            x: x + pad + badge_w + 8.0,
            y: cur_y + 12.0,
            font_size_pt: 8.5,
            color: primary,
            bold: true,
            align: HAlign::Left,
        });
    }
    cur_y += 24.0;

    // 3. Headline
    elements.push(SceneElement::Text {
        text: headline.clone(),
        x: x + pad,
        y: cur_y + 16.0,
        font_size_pt: 16.0,
        color: text_primary,
        bold: true,
        align: HAlign::Left,
    });
    cur_y += 24.0;

    // 4. KPI Callout Card
    if let Some(kpi_callout) = kpi {
        let kpi_box = RectPt::new(x + pad, cur_y, inner_w, 42.0);
        elements.push(SceneElement::Rect {
            rect: kpi_box,
            fill: Some(subtle_fill),
            stroke: None,
            corner_radius: 6.0,
        });

        // Large KPI Metric
        elements.push(SceneElement::Text {
            text: kpi_callout.metric.clone(),
            x: x + pad + 10.0,
            y: cur_y + 28.0,
            font_size_pt: 24.0,
            color: primary,
            bold: true,
            align: HAlign::Left,
        });

        // Label & sublabel
        let text_start_x = x + pad + 72.0;
        elements.push(SceneElement::Text {
            text: kpi_callout.label.clone(),
            x: text_start_x,
            y: cur_y + 16.0,
            font_size_pt: 8.5,
            color: text_primary,
            bold: true,
            align: HAlign::Left,
        });
        if let Some(sub) = &kpi_callout.sublabel {
            elements.push(SceneElement::Text {
                text: sub.clone(),
                x: text_start_x,
                y: cur_y + 30.0,
                font_size_pt: 7.5,
                color: text_muted,
                bold: false,
                align: HAlign::Left,
            });
        }
        if let Some(cite) = &kpi_callout.cite {
            elements.push(SceneElement::Text {
                text: cite.clone(),
                x: x + pad + inner_w - 6.0,
                y: cur_y + 14.0,
                font_size_pt: 7.0,
                color: text_muted,
                bold: false,
                align: HAlign::Right,
            });
        }
        cur_y += 48.0;
    }

    // 5. Bullets Title
    if let Some(title) = bullets_title {
        elements.push(SceneElement::Text {
            text: title.clone(),
            x: x + pad,
            y: cur_y + 10.0,
            font_size_pt: 8.5,
            color: text_muted,
            bold: true,
            align: HAlign::Left,
        });
        cur_y += 16.0;
    }

    // 6. Evidence Bullets
    for bullet in bullets {
        let lines = wrap_rich_text(&bullet.text, 8.5, false, inner_w - 20.0, DEFAULT_WRAP_SAFETY_PAD_PT);
        for line in &lines {
            elements.push(SceneElement::Text {
                text: "•".to_owned(),
                x: x + pad,
                y: cur_y + 10.0,
                font_size_pt: 8.5,
                color: primary,
                bold: true,
                align: HAlign::Left,
            });

            let mut run_x = x + pad + 8.0;
            for run in &line.runs {
                elements.push(SceneElement::Text {
                    text: run.text.clone(),
                    x: run_x,
                    y: cur_y + 10.0,
                    font_size_pt: 8.5,
                    color: text_primary,
                    bold: run.bold,
                    align: HAlign::Left,
                });
                run_x += run.width_pt;
            }
            cur_y += 14.0;
        }

        if let Some(cite) = &bullet.cite {
            elements.push(SceneElement::Text {
                text: cite.clone(),
                x: x + pad + inner_w,
                y: cur_y - 4.0,
                font_size_pt: 7.0,
                color: text_muted,
                bold: false,
                align: HAlign::Right,
            });
        }
    }

    // 7. Implication Box pinned to bottom of the card
    if let Some(imp) = implication {
        let imp_y = (y + final_card_h) - imp_h - pad;
        let imp_rect = RectPt::new(x + pad, imp_y, inner_w, imp_h);

        elements.push(SceneElement::Rect {
            rect: imp_rect,
            fill: Some(imp_fill),
            stroke: None,
            corner_radius: 6.0,
        });

        elements.push(SceneElement::Text {
            text: imp.title.clone(),
            x: x + pad + 8.0,
            y: imp_y + 14.0,
            font_size_pt: 8.5,
            color: primary,
            bold: true,
            align: HAlign::Left,
        });

        let mut body_y = imp_y + 26.0;
        let lines = wrap_rich_text(&imp.body, 8.5, true, inner_w - 16.0, DEFAULT_WRAP_SAFETY_PAD_PT);
        for line in &lines {
            let mut run_x = x + pad + 8.0;
            for run in &line.runs {
                elements.push(SceneElement::Text {
                    text: run.text.clone(),
                    x: run_x,
                    y: body_y + 8.0,
                    font_size_pt: 8.5,
                    color: text_primary,
                    bold: run.bold,
                    align: HAlign::Left,
                });
                run_x += run.width_pt;
            }
            body_y += 13.0;
        }
    }

    if let Some(node_id) = id {
        bboxes.insert(node_id.clone(), card_rect);
    }

    final_card_h
}
