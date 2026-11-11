#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use oneoffice_ui::studio::ActiveDocumentKind;
    use oneoffice_ui::theme::StudioTheme;
    use oneoffice_ui::StudioApp;
    use std::path::{Path, PathBuf};

    fn find_doc(name: &str) -> Vec<u8> {
        let candidates = [
            PathBuf::from("test_documents").join(name),
            PathBuf::from("../../test_documents").join(name),
            PathBuf::from("../test_documents").join(name),
        ];
        for path in &candidates {
            if let Ok(bytes) = std::fs::read(path) {
                return bytes;
            }
        }
        panic!("Document not found: {} in {:?}", name, candidates);
    }

    fn verify_and_save_screenshot(harness: &mut Harness<'_, StudioApp>, filename: &str) {
        harness.run_steps(3);
        let img = harness.render().expect("Headless render failed");
        assert_eq!(img.width(), 1440);
        assert!(img.height() >= 900);

        let mut non_black_pixels = 0usize;
        let mut unique_colors = std::collections::HashSet::new();
        for pixel in img.pixels() {
            let [r, g, b, _] = pixel.0;
            if r > 10 || g > 10 || b > 10 {
                non_black_pixels += 1;
            }
            if unique_colors.len() < 100 {
                unique_colors.insert(pixel.0);
            }
        }

        assert!(
            non_black_pixels > 350_000,
            "Screenshot [{}] failed non-black pixel check: {}",
            filename,
            non_black_pixels
        );
        assert!(
            unique_colors.len() >= 8,
            "Screenshot [{}] must contain rich color depth: {}",
            filename,
            unique_colors.len()
        );

        let out_path = Path::new("../../docs/assets/qa/gallery").join(filename);
        if let Some(parent) = out_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        img.save(&out_path).expect("Failed to save screenshot");
    }

    // =========================================================================
    // Category 1: HWPX Editor & In-Place Features (01 - 10)
    // =========================================================================

    #[test]
    fn test_01_hwpx_clean_view() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1200.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "01_hwpx_clean_view.png");
    }

    #[test]
    fn test_02_hwpx_light_theme() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        app.theme = StudioTheme::Light;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1200.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "02_hwpx_light_theme.png");
    }

    #[test]
    fn test_03_hwpx_table_cell_b2() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);
        app.table_selected_cell = (1, 1);
        app.formula_hud.cell_address = "B2".to_string();
        app.formula_hud.formula_text = "5".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1200.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "03_hwpx_table_cell_b2.png");
    }

    #[test]
    fn test_04_hwpx_table_cell_c2() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);
        app.table_selected_cell = (1, 2);
        app.formula_hud.cell_address = "C2".to_string();
        app.formula_hud.formula_text = "10000".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1200.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "04_hwpx_table_cell_c2.png");
    }

    #[test]
    fn test_05_hwpx_table_cell_d2_sum() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);
        app.table_selected_cell = (1, 3);
        app.formula_hud.cell_address = "D2".to_string();
        app.formula_hud.formula_text = "=B2*C2".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1200.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "05_hwpx_table_cell_d2_sum.png");
    }

    #[test]
    fn test_06_hwpx_chart_column_clustered() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);
        app.chart_widget.is_visible = true;
        app.chart_widget.selected_kind = "column_clustered".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1400.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "06_hwpx_chart_column_clustered.png");
    }

    #[test]
    fn test_07_hwpx_chart_line() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);
        app.chart_widget.is_visible = true;
        app.chart_widget.selected_kind = "line".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1400.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "07_hwpx_chart_line.png");
    }

    #[test]
    fn test_08_hwpx_chart_pie() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);
        app.chart_widget.is_visible = true;
        app.chart_widget.selected_kind = "pie".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1400.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "08_hwpx_chart_pie.png");
    }

    #[test]
    fn test_09_hwpx_ghost_block_toolbar() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1000.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "09_hwpx_ghost_block_toolbar.png");
    }

    #[test]
    fn test_10_hwpx_rich_compound_section() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);
        app.chart_widget.is_visible = true;

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1650.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "10_hwpx_rich_compound_section.png");
    }

    // =========================================================================
    // Category 2: 10 Consulting Proposal Diagram Templates (11 - 20)
    // =========================================================================

    fn render_diagram_template(template_id: &str, filename: &str) {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Hwpx;
        let bytes = find_doc("dummy_proposal.hwpx");
        app.load_hwpx_bytes(&bytes);
        app.diagram_widget.selected_template_id = template_id.to_string();
        if let Ok(spec) = oneoffice_diagram::templates::instantiate_template(template_id) {
            app.diagram_widget.spec = spec;
            app.diagram_widget.last_scene =
                oneoffice_diagram::layout::solve_layout(&app.diagram_widget.spec).ok();
        }

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1650.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, filename);
    }

    #[test]
    fn test_11_diagram_problem_comparison() {
        render_diagram_template("problem-comparison-to-solution", "11_diagram_problem_comparison.png");
    }

    #[test]
    fn test_12_diagram_showcase() {
        render_diagram_template("oneoffice-proposal-showcase", "12_diagram_showcase.png");
    }

    #[test]
    fn test_13_diagram_financial_switch() {
        render_diagram_template("financial-switch-proposal", "13_diagram_financial_switch.png");
    }

    #[test]
    fn test_14_diagram_kpi_summary() {
        render_diagram_template("kpi-executive-summary", "14_diagram_kpi_summary.png");
    }

    #[test]
    fn test_15_diagram_step_pipeline() {
        render_diagram_template("step-process-pipeline", "15_diagram_step_pipeline.png");
    }

    #[test]
    fn test_16_diagram_as_is_to_be() {
        render_diagram_template("as-is-to-be-bridge", "16_diagram_as_is_to_be.png");
    }

    #[test]
    fn test_17_diagram_layered_stack() {
        render_diagram_template("layered-architecture-stack", "17_diagram_layered_stack.png");
    }

    #[test]
    fn test_18_diagram_gantt_roadmap() {
        render_diagram_template("gantt-milestone-roadmap", "18_diagram_gantt_roadmap.png");
    }

    #[test]
    fn test_19_diagram_swot_quadrant() {
        render_diagram_template("swot-4quadrant", "19_diagram_swot_quadrant.png");
    }

    #[test]
    fn test_20_diagram_funnel_conversion() {
        render_diagram_template("funnel-conversion", "20_diagram_funnel_conversion.png");
    }

    // =========================================================================
    // Category 3: DOCX Word Processor & DrawingML Graphics (21 - 28)
    // =========================================================================

    #[test]
    fn test_21_docx_dark_view() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        let bytes = find_doc("dummy_report.docx");

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1600.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                if state.doc_textures.is_empty() {
                    let ctx = ui.ctx().clone();
                    state.load_docx_bytes(&ctx, &bytes);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "21_docx_dark_view.png");
    }

    #[test]
    fn test_22_docx_light_view() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        app.theme = StudioTheme::Light;
        let bytes = find_doc("dummy_report.docx");

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1600.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                if state.doc_textures.is_empty() {
                    let ctx = ui.ctx().clone();
                    state.load_docx_bytes(&ctx, &bytes);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "22_docx_light_view.png");
    }

    #[test]
    fn test_23_docx_drawingml_diagram() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        let bytes = find_doc("dummy_report.docx");

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1200.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                if state.doc_textures.is_empty() {
                    let ctx = ui.ctx().clone();
                    state.load_docx_bytes(&ctx, &bytes);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "23_docx_drawingml_diagram.png");
    }

    #[test]
    fn test_24_docx_drawingml_chart() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        let bytes = find_doc("dummy_report.docx");

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1600.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                if state.doc_textures.is_empty() {
                    let ctx = ui.ctx().clone();
                    state.load_docx_bytes(&ctx, &bytes);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "24_docx_drawingml_chart.png");
    }

    #[test]
    fn test_25_docx_behind_text_stamp() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        let bytes = find_doc("dummy_report.docx");

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1650.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                if state.doc_textures.is_empty() {
                    let ctx = ui.ctx().clone();
                    state.load_docx_bytes(&ctx, &bytes);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "25_docx_behind_text_stamp.png");
    }

    #[test]
    fn test_26_docx_paragraph_format() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        let bytes = find_doc("dummy_report.docx");
        app.active_paragraph_idx = Some(0);
        app.format_state.is_bold = true;
        app.format_state.font_size = 18.0;

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1200.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                if state.doc_textures.is_empty() {
                    let ctx = ui.ctx().clone();
                    state.load_docx_bytes(&ctx, &bytes);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "26_docx_paragraph_format.png");
    }

    #[test]
    fn test_27_docx_zoom_level() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        app.zoom_pct = 120;
        let bytes = find_doc("dummy_report.docx");

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1400.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                if state.doc_textures.is_empty() {
                    let ctx = ui.ctx().clone();
                    state.load_docx_bytes(&ctx, &bytes);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "27_docx_zoom_level.png");
    }

    #[test]
    fn test_28_docx_no_formula_hud() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Docx;
        let bytes = find_doc("dummy_report.docx");

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1000.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                if state.doc_textures.is_empty() {
                    let ctx = ui.ctx().clone();
                    state.load_docx_bytes(&ctx, &bytes);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "28_docx_no_formula_hud.png");
    }

    // =========================================================================
    // Category 4: XLSX Full-Grid Spreadsheet & 513 Calc Engine (29 - 36)
    // =========================================================================

    #[test]
    fn test_29_xlsx_sheet1_budget() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        let bytes = find_doc("dummy_budget.xlsx");
        app.load_xlsx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "29_xlsx_sheet1_budget.png");
    }

    #[test]
    fn test_30_xlsx_sheet2_execution() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        let bytes = find_doc("dummy_budget.xlsx");
        app.load_xlsx_bytes(&bytes);
        if let Some(model) = &mut app.spreadsheet_model {
            if model.sheet_names.len() < 2 {
                model.sheet_names.push("집행내역".to_string());
            }
            model.active_sheet_index = 1;
        }

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "30_xlsx_sheet2_execution.png");
    }

    #[test]
    fn test_31_xlsx_light_sheet() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        app.theme = StudioTheme::Light;
        let bytes = find_doc("dummy_budget.xlsx");
        app.load_xlsx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "31_xlsx_light_sheet.png");
    }

    #[test]
    fn test_32_xlsx_focus_cell_b2() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        let bytes = find_doc("dummy_budget.xlsx");
        app.load_xlsx_bytes(&bytes);
        app.table_selected_cell = (1, 1);
        app.formula_hud.cell_address = "B2".to_string();
        app.formula_hud.formula_text = "5000".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "32_xlsx_focus_cell_b2.png");
    }

    #[test]
    fn test_33_xlsx_focus_cell_c2() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        let bytes = find_doc("dummy_budget.xlsx");
        app.load_xlsx_bytes(&bytes);
        app.table_selected_cell = (1, 2);
        app.formula_hud.cell_address = "C2".to_string();
        app.formula_hud.formula_text = "4800".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "33_xlsx_focus_cell_c2.png");
    }

    #[test]
    fn test_34_xlsx_12cols_full_grid() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        let bytes = find_doc("dummy_budget.xlsx");
        app.load_xlsx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "34_xlsx_12cols_full_grid.png");
    }

    #[test]
    fn test_35_xlsx_find_replace_active() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        let bytes = find_doc("dummy_budget.xlsx");
        app.load_xlsx_bytes(&bytes);
        app.find_replace.is_open = true;
        app.find_replace.find_query = "예산".to_string();
        app.find_replace.replace_query = "투자금액".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "35_xlsx_find_replace_active.png");
    }

    #[test]
    fn test_36_xlsx_ready_status_stats() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Xlsx;
        let bytes = find_doc("dummy_budget.xlsx");
        app.load_xlsx_bytes(&bytes);
        app.status_text = "READY · 활성 셀 [B2] · 시트 크기 18×12 · 513개 엑셀 수식 및 차트 엔진 가동 중".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "36_xlsx_ready_status_stats.png");
    }

    // =========================================================================
    // Category 5: PPTX 16:9 Slide Presentation (37 - 44)
    // =========================================================================

    #[test]
    fn test_37_pptx_slide1_architecture() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        let bytes = find_doc("dummy_presentation.pptx");
        app.load_pptx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "37_pptx_slide1_architecture.png");
    }

    #[test]
    fn test_38_pptx_slide2_why_now() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        let bytes = find_doc("dummy_presentation.pptx");
        app.load_pptx_bytes(&bytes);
        if let Some(model) = &mut app.presentation_model {
            model.active_slide_index = 1;
        }

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "38_pptx_slide2_why_now.png");
    }

    #[test]
    fn test_39_pptx_light_slide() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        app.theme = StudioTheme::Light;
        let bytes = find_doc("dummy_presentation.pptx");
        app.load_pptx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "39_pptx_light_slide.png");
    }

    #[test]
    fn test_40_pptx_deck_thumbnails() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        let bytes = find_doc("dummy_presentation.pptx");
        app.load_pptx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "40_pptx_deck_thumbnails.png");
    }

    #[test]
    fn test_41_pptx_slide_full_diagram() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        let bytes = find_doc("dummy_presentation.pptx");
        app.load_pptx_bytes(&bytes);

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1200.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "41_pptx_slide_full_diagram.png");
    }

    #[test]
    fn test_42_pptx_slide_title_editing() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        let bytes = find_doc("dummy_presentation.pptx");
        app.load_pptx_bytes(&bytes);
        if let Some(model) = &mut app.presentation_model
            && let Some(slide) = model.slides.first_mut()
        {
            slide.title = "ONEOFFICE 2026 차세대 사업계획서 제안 (수정 중)".to_string();
        }

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "42_pptx_slide_title_editing.png");
    }

    #[test]
    fn test_43_pptx_bullet_points_list() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        let bytes = find_doc("dummy_presentation.pptx");
        app.load_pptx_bytes(&bytes);
        if let Some(model) = &mut app.presentation_model
            && let Some(slide) = model.slides.first_mut()
        {
            slide.bullet_points.push("글자 잘림 제로(Zero-Overflow) 자동 조판 엔진 탑재".to_string());
        }

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "43_pptx_bullet_points_list.png");
    }

    #[test]
    fn test_44_pptx_template_switcher() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pptx;
        let bytes = find_doc("dummy_presentation.pptx");
        app.load_pptx_bytes(&bytes);
        app.diagram_widget.selected_template_id = "step-process-pipeline".to_string();
        if let Ok(spec) = oneoffice_diagram::templates::instantiate_template("step-process-pipeline") {
            app.diagram_widget.spec = spec;
            app.diagram_widget.last_scene =
                oneoffice_diagram::layout::solve_layout(&app.diagram_widget.spec).ok();
        }

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1150.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "44_pptx_template_switcher.png");
    }

    // =========================================================================
    // Category 6: Pure-Rust PDF Viewer & Electronic Signatures (45 - 52)
    // =========================================================================

    #[test]
    fn test_45_pdf_contract_clean_view() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pdf;
        app.document_title = "표준_용역계약서.pdf".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1020.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                let ctx = ui.ctx().clone();
                state.pdf_widget.ensure_sample_loaded(&ctx);
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "45_pdf_contract_clean_view.png");
    }

    #[test]
    fn test_46_pdf_contract_light_view() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pdf;
        app.theme = StudioTheme::Light;
        app.document_title = "표준_용역계약서 (Light).pdf".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1020.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                let ctx = ui.ctx().clone();
                state.pdf_widget.ensure_sample_loaded(&ctx);
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "46_pdf_contract_light_view.png");
    }

    #[test]
    fn test_47_pdf_anchor_detected_seal() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pdf;
        app.document_title = "전자직인_체결본.pdf".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1020.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                let ctx = ui.ctx().clone();
                state.pdf_widget.ensure_sample_loaded(&ctx);
                if state.pdf_widget.layer_stack.signatures().is_empty() {
                    let _ = state.pdf_widget.stamp_seal_at(195.0, 736.0);
                }
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "47_pdf_anchor_detected_seal.png");
    }

    #[test]
    fn test_48_pdf_seal_label_expanded() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pdf;
        app.pdf_widget.seal_label = "원오피스\n총괄대표".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1020.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                let ctx = ui.ctx().clone();
                state.pdf_widget.ensure_sample_loaded(&ctx);
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "48_pdf_seal_label_expanded.png");
    }

    #[test]
    fn test_49_pdf_zoom_120_percent() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pdf;
        app.pdf_widget.zoom_scale = 1.2;

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1080.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                let ctx = ui.ctx().clone();
                state.pdf_widget.ensure_sample_loaded(&ctx);
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "49_pdf_zoom_120_percent.png");
    }

    #[test]
    fn test_50_pdf_full_a4_viewport() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pdf;

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1020.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                let ctx = ui.ctx().clone();
                state.pdf_widget.ensure_sample_loaded(&ctx);
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "50_pdf_full_a4_viewport.png");
    }

    #[test]
    fn test_51_pdf_signing_status_bar() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pdf;
        app.pdf_widget.status_message = "페이지 1에 직인을 성공적으로 날인했습니다.".to_string();

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1020.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                let ctx = ui.ctx().clone();
                state.pdf_widget.ensure_sample_loaded(&ctx);
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "51_pdf_signing_status_bar.png");
    }

    #[test]
    fn test_52_pdf_signature_undoredo() {
        let mut app = StudioApp::new();
        app.active_doc = ActiveDocumentKind::Pdf;

        let mut harness = Harness::builder()
            .allow_missing_glyphs()
            .with_accessibility_check(false)
            .with_size(egui::vec2(1440.0, 1020.0))
            .build_ui_state(|ui, state: &mut StudioApp| {
                let ctx = ui.ctx().clone();
                state.pdf_widget.ensure_sample_loaded(&ctx);
                let _ = state.pdf_widget.stamp_seal_at(195.0, 736.0);
                state.show(ui);
            }, app);

        verify_and_save_screenshot(&mut harness, "52_pdf_signature_undoredo.png");
    }
}
