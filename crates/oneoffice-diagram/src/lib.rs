//! Semantic proposal diagramming DSL (`DiagramSpec` v1), 3-pass Korean constraint layout solver,
//! and multi-target compilers (SVG, PNG, OWPML, DrawingML).

#![forbid(unsafe_code)]

pub mod compile_drawingml;
pub mod compile_owpml;
pub mod compile_png;
pub mod compile_svg;
pub mod font_metrics;
pub mod layout;
pub mod patch;
pub mod spec;
pub mod templates;

pub use compile_drawingml::compile_drawingml;
pub use compile_owpml::compile_owpml;
pub use compile_png::compile_png;
pub use compile_svg::compile_svg;
pub use font_metrics::{DEFAULT_WRAP_SAFETY_PAD_PT, DiagramFontContext};
pub use layout::{
    HAlign, LayoutDiagnostics, PositionedScene, RectPt, SceneElement, solve_layout,
};
pub use patch::{DiagramPatch, NodePatch, patch_diagram};
pub use spec::{
    ComparisonVariant, ConnectorAnchor, ConnectorPort, ConnectorStyle, DIAGRAM_SCHEMA_V1,
    DiagramCanvas, DiagramError, DiagramNode, DiagramSpec, DiagramTheme, GluedConnector, PortRef,
    RgbColor,
};
pub use templates::{
    DIAGRAM_TEMPLATES, DiagramTemplateInfo, get_diagram_template, instantiate_template,
    list_diagram_templates,
};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_all_10_templates_instantiate_and_solve_layout() {
        let templates = list_diagram_templates();
        assert_eq!(templates.len(), 10);

        for info in templates {
            let spec = instantiate_template(info.id)
                .unwrap_or_else(|e| panic!("failed to instantiate template '{}': {e}", info.id));

            let scene = solve_layout(&spec)
                .unwrap_or_else(|e| panic!("failed to solve layout for template '{}': {e}", info.id));

            assert!(scene.width_pt > 100.0);
            assert!(scene.height_pt > 50.0);
            assert!(!scene.elements.is_empty());
            assert_eq!(scene.diagnostics.overflow_count, 0, "template '{}' had text overflow", info.id);
        }
    }

    #[test]
    fn test_svg_compilation() {
        let spec = instantiate_template("problem-comparison-to-solution").unwrap();
        let scene = solve_layout(&spec).unwrap();
        let svg = compile_svg(&scene);

        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>\n"));
        assert!(svg.contains("viewBox="));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<text"));
    }

    #[test]
    fn test_png_compilation() {
        let spec = instantiate_template("problem-comparison-to-solution").unwrap();
        let scene = solve_layout(&spec).unwrap();
        let png_bytes = compile_png(&scene, 1.0).expect("compile_png failed");

        assert!(png_bytes.len() > 100);
        // PNG magic bytes: 0x89 0x50 0x4E 0x47 0x0D 0x0A 0x1A 0x0A
        assert_eq!(&png_bytes[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn test_owpml_compilation() {
        let spec = instantiate_template("oneoffice-proposal-showcase").unwrap();
        let scene = solve_layout(&spec).unwrap();
        let owpml = compile_owpml(&scene);

        assert!(owpml.starts_with("<hp:container"));
        assert!(owpml.ends_with("</hp:container>\n"));
        assert!(owpml.contains("xmlns:hp="));
        assert!(owpml.contains("<hp:rect"));
        assert!(owpml.contains("<hp:drawText"));
    }

    #[test]
    fn test_drawingml_compilation() {
        let spec = instantiate_template("oneoffice-proposal-showcase").unwrap();
        let scene = solve_layout(&spec).unwrap();
        let drawingml = compile_drawingml(&scene);

        assert!(drawingml.starts_with("<wgp:wgp"));
        assert!(drawingml.ends_with("</wgp:wgp>\n"));
        assert!(drawingml.contains("<wps:wsp>"));
        assert!(drawingml.contains("<a:xfrm>"));
    }

    #[test]
    fn test_patch_diagram() {
        let mut spec = instantiate_template("problem-comparison-to-solution").unwrap();
        let patch = DiagramPatch {
            title: Some("업데이트된 제안서 도식".to_string()),
            theme_preset: Some("public-rfp-emerald".to_string()),
            canvas_width_pt: Some(500.0),
            node_patches: vec![NodePatch {
                target_id: "card_prob_a".to_string(),
                title: Some("기존 분절 환경".to_string()),
                subtitle: Some("AS-IS 수작업 분석".to_string()),
                metric: None,
                delta: None,
                bullets: Some(vec!["한컴/MS 단절 심각".to_string()]),
                tint_color: None,
                border_color: None,
            }],
            add_connectors: vec![],
        };

        patch_diagram(&mut spec, &patch).expect("patch_diagram failed");

        assert_eq!(spec.title.as_deref(), Some("업데이트된 제안서 도식"));
        assert_eq!(spec.canvas.target_width_pt, 500.0);

        let scene = solve_layout(&spec).unwrap();
        let svg = compile_svg(&scene);
        assert!(svg.contains("기존 분절 환경"));
    }
}
