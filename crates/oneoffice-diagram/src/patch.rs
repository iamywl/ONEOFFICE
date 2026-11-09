//! In-place incremental patching for [`DiagramSpec`].

use serde::{Deserialize, Serialize};

use crate::spec::{
    DiagramError, DiagramNode, DiagramSpec, DiagramTheme, EvidenceBullet, GluedConnector,
    RgbColor,
};

/// A patch operation to update fields or nodes within a [`DiagramSpec`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canvas_width_pt: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_patches: Vec<NodePatch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_connectors: Vec<GluedConnector>,
}

/// Patch targeting a specific node by its unique ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodePatch {
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bullets: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint_color: Option<RgbColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<RgbColor>,
}

/// Applies a [`DiagramPatch`] to an existing [`DiagramSpec`].
pub fn patch_diagram(spec: &mut DiagramSpec, patch: &DiagramPatch) -> Result<(), DiagramError> {
    if let Some(ref title) = patch.title {
        spec.title = Some(title.clone());
    }

    if let Some(ref preset) = patch.theme_preset {
        spec.theme = DiagramTheme::from_preset(preset)
            .ok_or_else(|| DiagramError::UnknownThemePreset(preset.clone()))?;
    }

    if let Some(width_pt) = patch.canvas_width_pt {
        if !width_pt.is_finite() || width_pt <= 10.0 {
            return Err(DiagramError::InvalidCanvasWidth(width_pt));
        }
        spec.canvas.target_width_pt = width_pt;
    }

    for node_patch in &patch.node_patches {
        let found = apply_node_patch(&mut spec.root, node_patch)?;
        if !found {
            return Err(DiagramError::NodeNotFound(node_patch.target_id.clone()));
        }
    }

    if !patch.add_connectors.is_empty() {
        spec.connectors.extend(patch.add_connectors.clone());
    }

    Ok(())
}

fn apply_node_patch(node: &mut DiagramNode, patch: &NodePatch) -> Result<bool, DiagramError> {
    match node {
        DiagramNode::VStack { id, children, .. }
        | DiagramNode::HStack { id, children, .. }
        | DiagramNode::Grid { id, children, .. } => {
            if id.as_deref() == Some(&patch.target_id) {
                return Ok(true);
            }
            for child in children {
                if apply_node_patch(child, patch)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        DiagramNode::HeaderBar { id, left_text, right_badge, .. } => {
            if id.as_deref() == Some(&patch.target_id) {
                if let Some(ref t) = patch.title {
                    *left_text = t.clone();
                }
                if let Some(ref s) = patch.subtitle {
                    *right_badge = Some(s.clone());
                }
                return Ok(true);
            }
            Ok(false)
        }
        DiagramNode::SectionHeading { id, title, lead, .. } => {
            if id.as_deref() == Some(&patch.target_id) {
                if let Some(ref t) = patch.title {
                    *title = t.clone();
                }
                if let Some(ref s) = patch.subtitle {
                    *lead = Some(s.clone());
                }
                return Ok(true);
            }
            Ok(false)
        }
        DiagramNode::ComparisonCard {
            id,
            headline,
            bullets,
            badge,
            ..
        } => {
            if id.as_deref() == Some(&patch.target_id) {
                if let Some(ref t) = patch.title {
                    *headline = t.clone();
                }
                if let Some(ref s) = patch.subtitle {
                    *badge = s.clone();
                }
                if let Some(ref b) = patch.bullets {
                    bullets.clear();
                    for bullet in b {
                        bullets.push(EvidenceBullet {
                            text: bullet.clone(),
                            cite: None,
                        });
                    }
                }
                return Ok(true);
            }
            Ok(false)
        }
        DiagramNode::HeroContainer {
            id,
            headline,
            subheadline,
            step_chain,
            ..
        } => {
            if id.as_deref() == Some(&patch.target_id) {
                if let Some(ref t) = patch.title {
                    *headline = t.clone();
                }
                if let Some(ref s) = patch.subtitle {
                    *subheadline = Some(s.clone());
                }
                return Ok(true);
            }
            for step in step_chain {
                if step.title == patch.target_id {
                    if let Some(ref t) = patch.title {
                        step.title = t.clone();
                    }
                    if let Some(ref s) = patch.subtitle {
                        step.summary = Some(s.clone());
                    }
                    return Ok(true);
                }
            }
            Ok(false)
        }
        DiagramNode::KpiRow { id, items, .. } => {
            if id.as_deref() == Some(&patch.target_id) {
                return Ok(true);
            }
            for item in items {
                if item.id.as_deref() == Some(&patch.target_id) {
                    if let Some(ref m) = patch.metric {
                        item.metric = m.clone();
                    }
                    if let Some(ref t) = patch.title {
                        item.label = t.clone();
                    }
                    if let Some(ref d) = patch.delta {
                        item.delta = Some(d.clone());
                    }
                    return Ok(true);
                }
            }
            Ok(false)
        }
        DiagramNode::AsIsToBe {
            id,
            as_is_title,
            to_be_title,
            ..
        } => {
            if id.as_deref() == Some(&patch.target_id) {
                if let Some(ref t) = patch.title {
                    *as_is_title = t.clone();
                }
                if let Some(ref s) = patch.subtitle {
                    *to_be_title = s.clone();
                }
                return Ok(true);
            }
            Ok(false)
        }
        DiagramNode::ArchitectureStack { id, layers } => {
            if id.as_deref() == Some(&patch.target_id) {
                return Ok(true);
            }
            for layer in layers {
                if layer.name == patch.target_id {
                    if let Some(ref t) = patch.title {
                        layer.name = t.clone();
                    }
                    return Ok(true);
                }
            }
            Ok(false)
        }
        DiagramNode::GanttRoadmap { id, tracks, .. } => {
            if id.as_deref() == Some(&patch.target_id) {
                return Ok(true);
            }
            for track in tracks {
                if track.name == patch.target_id {
                    if let Some(ref t) = patch.title {
                        track.name = t.clone();
                    }
                    return Ok(true);
                }
            }
            Ok(false)
        }
        DiagramNode::CitationFooter { id, .. } => {
            if id.as_deref() == Some(&patch.target_id) {
                return Ok(true);
            }
            Ok(false)
        }
        DiagramNode::FunnelStages { id, stages } => {
            if id.as_deref() == Some(&patch.target_id) {
                return Ok(true);
            }
            for stage in stages {
                if stage.stage_name == patch.target_id {
                    if let Some(ref t) = patch.title {
                        stage.stage_name = t.clone();
                    }
                    return Ok(true);
                }
            }
            Ok(false)
        }
        DiagramNode::OrgTree { id, .. } => {
            if id.as_deref() == Some(&patch.target_id) {
                return Ok(true);
            }
            Ok(false)
        }
    }
}
