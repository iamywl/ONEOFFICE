//! Lossless KS X 6101 OWPML (`.hwpx`) compound object engine with Excel sidecars,
//! 26-type charts, diagrams, and SHA-256 staleness reconciliation.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

pub mod compound;
pub mod error;
pub mod package;

pub use compound::{
    HwpxTableFormulaSidecar, build_hwpx_table_xml, embed_diagram_into_section,
    embed_table_into_section, replace_fields_in_section,
};
pub use error::{HwpxError, HwpxResult};
pub use package::{
    HWPX_MIMETYPE, HwpxPackage, MAX_SINGLE_ENTRY_BYTES, MAX_TOTAL_DECOMPRESSED_BYTES,
};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::BTreeMap;

    use oneoffice_bridge::calc::{CalcTableSpec, CellInput};
    use oneoffice_diagram::instantiate_template;

    use super::*;

    #[test]
    fn test_create_minimal_hwpx_and_roundtrip() {
        let pkg = HwpxPackage::create_minimal_hwpx(
            "ONEOFFICE 차세대 제안서",
            "순수 Rust 기반 올인원 오피스 플랫폼 기술 규격서입니다.",
        )
        .expect("minimal hwpx creation failed");

        assert_eq!(pkg.section_count(), 1);
        let sec0 = pkg.section_xml(0).expect("read section 0 failed");
        assert!(sec0.contains("ONEOFFICE 차세대 제안서"));
        assert!(sec0.contains("순수 Rust 기반 올인원 오피스 플랫폼 기술 규격서입니다."));

        let bytes = pkg.to_bytes().expect("serialization failed");
        assert!(!bytes.is_empty());

        let roundtrip = HwpxPackage::from_bytes(&bytes).expect("roundtrip parse failed");
        assert_eq!(roundtrip.section_count(), 1);
        let roundtrip_sec = roundtrip.section_xml(0).expect("roundtrip sec read failed");
        assert_eq!(roundtrip_sec, sec0);
    }

    #[test]
    fn test_embed_diagram_into_hwpx_section() {
        let mut pkg = HwpxPackage::create_minimal_hwpx(
            "제안서 아키텍처",
            "다음은 제안 시스템 구성도입니다:",
        )
        .expect("minimal hwpx failed");

        let diagram = instantiate_template("problem-comparison-to-solution")
            .expect("template instantiation failed");

        let sec_xml = pkg.section_xml(0).expect("read section failed");
        let updated_sec = embed_diagram_into_section(&sec_xml, &diagram, 1)
            .expect("embed diagram failed");

        assert!(updated_sec.contains("<hp:container"));
        assert!(updated_sec.contains(r#"id="oneoffice-diagram""#));
        assert!(updated_sec.contains(r#"treatAsChar="1""#));

        pkg.set_section_xml(0, &updated_sec).expect("set section failed");
        let pkg_bytes = pkg.to_bytes().expect("package serialize failed");
        assert!(!pkg_bytes.is_empty());
    }

    #[test]
    fn test_embed_calc_table_and_staleness() {
        let mut pkg = HwpxPackage::create_minimal_hwpx(
            "사업 예산 계획",
            "세부 항목별 소요 예산은 아래 표와 같습니다:",
        )
        .expect("minimal hwpx failed");

        let spec = CalcTableSpec {
            sheet_name: "예산안".to_string(),
            cells: vec![
                vec![CellInput::text("항목"), CellInput::text("수량"), CellInput::text("단가"), CellInput::text("금액")],
                vec![CellInput::text("서버 인프라"), CellInput::text("2"), CellInput::text("500000"), CellInput::formula("=B2*C2")],
                vec![CellInput::text("소프트웨어 라이선스"), CellInput::text("1"), CellInput::text("1500000"), CellInput::formula("=B3*C3")],
                vec![CellInput::text("합계"), CellInput::empty(), CellInput::empty(), CellInput::formula("=SUM(D2:D3)")],
            ],
            merges: Vec::new(),
        };

        let mut sidecar = HwpxTableFormulaSidecar::new("tbl-budget-1", spec.clone());
        let output = sidecar.evaluate().expect("table evaluate failed");

        assert_eq!(output.display_cells.len(), 4);
        assert_eq!(output.display_cells[1][3], "1000000"); // 2 * 500,000
        assert_eq!(output.display_cells[2][3], "1500000"); // 1 * 1,500,000
        assert_eq!(output.display_cells[3][3], "2500000"); // SUM = 2,500,000
        assert!(!output.rendered_cells_sha256.is_empty());

        let sec_xml = pkg.section_xml(0).expect("read section failed");
        let updated_sec = embed_table_into_section(&sec_xml, &output, "tbl-budget-1", 1)
            .expect("embed table failed");

        assert!(updated_sec.contains(r#"<hp:tbl id="tbl-budget-1""#));
        assert!(updated_sec.contains("<hp:t>2500000</hp:t>"));

        // Verify sidecar attachment in package
        pkg.add_binary_attachment("BinData/table_1.xlsx", output.xlsx_bytes.clone());
        assert!(pkg.get_attachment("BinData/table_1.xlsx").is_some());

        // Staleness detection
        assert!(!sidecar.is_stale(&output.rendered_cells_sha256));
        assert!(sidecar.is_stale("0000000000000000000000000000000000000000000000000000000000000000"));
    }

    #[test]
    fn test_field_replacement() {
        let template_xml = r#"<hs:sec><hp:p id="0"><hp:run><hp:t>수신: {{RECIPIENT_NAME}} 귀하</hp:t></hp:run></hp:p><hp:p id="1"><hp:fieldBegin id="1" type="clickhere" name="PROJECT_TITLE"/><hp:run><hp:t>기존프로젝트명</hp:t></hp:run><hp:fieldEnd id="1"/></hp:p></hs:sec>"#;

        let mut replacements = BTreeMap::new();
        replacements.insert("RECIPIENT_NAME".to_string(), "홍길동 대표".to_string());
        replacements.insert("PROJECT_TITLE".to_string(), "차세대 ONEOFFICE 사업".to_string());

        let replaced = replace_fields_in_section(template_xml, &replacements);

        assert!(replaced.contains("수신: 홍길동 대표 귀하"));
        assert!(replaced.contains("차세대 ONEOFFICE 사업"));
        assert!(!replaced.contains("기존프로젝트명"));
        assert!(!replaced.contains("{{RECIPIENT_NAME}}"));
    }
}
