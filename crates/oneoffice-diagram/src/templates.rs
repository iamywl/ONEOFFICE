//! 10 consulting-grade Korean proposal templates for `DiagramSpec` v1.

use serde::{Deserialize, Serialize};

use crate::spec::{
    ArchitectureLayer, AsIsToBeItem, CitationEntry, ConnectorStyle, DiagramCanvas,
    DiagramError, DiagramNode, DiagramSpec, DiagramTheme, EvidenceBullet, FunnelStage, GanttItem,
    GanttTrack, GluedConnector, ImplicationBox, KpiCallout, KpiMetricCard, PortRef, ProcessStep,
    StepGroupLabel,
};

/// Alias for [`TemplateDescriptor`].
pub type DiagramTemplateInfo = TemplateDescriptor;

/// Canonical IDs of all 10 built-in proposal diagram templates.
pub static DIAGRAM_TEMPLATES: &[&str] = &[
    "problem-comparison-to-solution",
    "oneoffice-proposal-showcase",
    "financial-switch-proposal",
    "kpi-executive-summary",
    "step-process-pipeline",
    "as-is-to-be-bridge",
    "layered-architecture-stack",
    "gantt-milestone-roadmap",
    "swot-4quadrant",
    "funnel-conversion",
];

/// Lists all registered diagram template metadata descriptors.
#[must_use]
pub fn list_diagram_templates() -> Vec<DiagramTemplateInfo> {
    list_templates()
}

/// Retrieves and instantiates a diagram template by ID.
pub fn get_diagram_template(template_id: &str) -> Result<DiagramSpec, DiagramError> {
    instantiate_template(template_id)
}

/// Metadata descriptor for a built-in consulting diagram template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDescriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub recommended_width_pt: f32,
}

/// Lists all 10 registered consulting proposal diagram templates.
#[must_use]
pub fn list_templates() -> Vec<TemplateDescriptor> {
    vec![
        TemplateDescriptor {
            id: "problem-comparison-to-solution",
            name: "문제 비교 및 통합 솔루션 원페이저",
            category: "proposal",
            description: "2열 비교 카드(KPI/근거/시사점) + 하강 연결선 + 다크네이비 4단계 실행 체계",
            recommended_width_pt: 425.2,
        },
        TemplateDescriptor {
            id: "oneoffice-proposal-showcase",
            name: "ONEOFFICE 통합 제안서 쇼케이스",
            category: "proposal",
            description: "도식화 단절과 포맷 파편화를 수식표/시맨틱도식/PDF직인/MCP로 해결하는 원페이저",
            recommended_width_pt: 425.2,
        },
        TemplateDescriptor {
            id: "financial-switch-proposal",
            name: "금융 주거래 전환 및 AI 자산관리 제안",
            category: "proposal",
            description: "전환 장벽과 정착 장벽을 분석하고 4단계 AI 맞춤 서비스로 해결하는 금융 장표",
            recommended_width_pt: 425.2,
        },
        TemplateDescriptor {
            id: "kpi-executive-summary",
            name: "경영진 핵심 KPI 요약 대시보드",
            category: "dashboard",
            description: "4대 핵심 성과 지표(목표 달성률, 절감액, 전환율, 만족도) 가로 그리드",
            recommended_width_pt: 425.2,
        },
        TemplateDescriptor {
            id: "step-process-pipeline",
            name: "5단계 실행 프로세스 파이프라인",
            category: "process",
            description: "진단 → 설계 → 개발 → 검증 → 배포의 순차적 단계와 마일스톤 산출물",
            recommended_width_pt: 425.2,
        },
        TemplateDescriptor {
            id: "as-is-to-be-bridge",
            name: "As-Is / To-Be 변화 및 혁신 모델",
            category: "strategy",
            description: "현행 문제점과 미래 개선 목표를 촉매제(AI/플랫폼)로 연결하는 3단 대비 장표",
            recommended_width_pt: 425.2,
        },
        TemplateDescriptor {
            id: "layered-architecture-stack",
            name: "4계층 통합 시스템 아키텍처",
            category: "architecture",
            description: "인프라 / 코어 엔진 / AI 프로토콜(MCP) / 사용자 경험(Studio) 계층 구조",
            recommended_width_pt: 425.2,
        },
        TemplateDescriptor {
            id: "gantt-milestone-roadmap",
            name: "추진 일정 및 마일스톤 로드맵",
            category: "roadmap",
            description: "스프린트/분기별 트랙(코어, UX, MCP, 패키징) 진행 바와 주요 릴리즈 게이트",
            recommended_width_pt: 425.2,
        },
        TemplateDescriptor {
            id: "swot-4quadrant",
            name: "SWOT 4사분면 전략 매트릭스",
            category: "strategy",
            description: "강점(S), 약점(W), 기회(O), 위협(T) 분석 및 교차 대응 전략",
            recommended_width_pt: 425.2,
        },
        TemplateDescriptor {
            id: "funnel-conversion",
            name: "고객 여정 및 전환 깔때기 (Funnel)",
            category: "marketing",
            description: "유입 → 탐색 → 제안서 작성 → 직인 날인 → 전환율 단계별 레버리지",
            recommended_width_pt: 425.2,
        },
    ]
}

/// Instantiates a consulting proposal template by its canonical template ID.
pub fn instantiate_template(template_id: &str) -> Result<DiagramSpec, DiagramError> {
    match template_id {
        "problem-comparison-to-solution" => Ok(template_problem_comparison_to_solution()),
        "oneoffice-proposal-showcase" => Ok(template_oneoffice_proposal_showcase()),
        "financial-switch-proposal" => Ok(template_financial_switch_proposal()),
        "kpi-executive-summary" => Ok(template_kpi_executive_summary()),
        "step-process-pipeline" => Ok(template_step_process_pipeline()),
        "as-is-to-be-bridge" => Ok(template_as_is_to_be_bridge()),
        "layered-architecture-stack" => Ok(template_layered_architecture_stack()),
        "gantt-milestone-roadmap" => Ok(template_gantt_milestone_roadmap()),
        "swot-4quadrant" => Ok(template_swot_4quadrant()),
        "funnel-conversion" => Ok(template_funnel_conversion()),
        other => Err(DiagramError::UnknownTemplate(other.to_owned())),
    }
}

fn make_theme_consulting_navy() -> DiagramTheme {
    DiagramTheme::default()
}

fn template_problem_comparison_to_solution() -> DiagramSpec {
    DiagramSpec {
        schema_version: crate::spec::DIAGRAM_SCHEMA_V1.to_owned(),
        id: "diag_problem_comparison_01".to_owned(),
        title: Some("문제 비교 및 통합 솔루션 원페이저".to_owned()),
        theme: make_theme_consulting_navy(),
        canvas: DiagramCanvas {
            target_width_pt: 425.2,
            padding_pt: 0.0,
            bg_color: None,
        },
        root: DiagramNode::VStack {
            id: Some("root_stack".to_owned()),
            gap_pt: 12.0,
            padding_pt: 0.0,
            children: vec![
                DiagramNode::HeaderBar {
                    id: Some("hdr".to_owned()),
                    left_text: "WOORI × SSAFY 2026 / WON주민".to_owned(),
                    right_badge: Some("BACKGROUND / NEED".to_owned()),
                    show_divider: true,
                },
                DiagramNode::SectionHeading {
                    id: Some("heading".to_owned()),
                    eyebrow: Some("00 / WHY NOW".to_owned()),
                    title: "기획 배경 및 필요성".to_owned(),
                    lead: Some("은행을 옮길 수는 있지만, '옮기는 수고'와 '머물 이유 부족' 때문에 주거래는 쉽게 바뀌지 않는다.\n금융 이사의 부담을 낮추고, 개인화·보상 경험으로 머물 이유를 만드는 서비스가 필요하다.".to_owned()),
                },
                DiagramNode::Grid {
                    id: Some("problem_grid".to_owned()),
                    columns: 2,
                    gap_pt: 10.0,
                    equalize_card_heights: true,
                    connectors: Vec::new(),
                    children: vec![
                        DiagramNode::ComparisonCard {
                            id: Some("card_prob_a".to_owned()),
                            badge: "문제 A".to_owned(),
                            category: Some("전환 장벽".to_owned()),
                            headline: "옮기기 어렵다".to_owned(),
                            kpi: Some(KpiCallout {
                                metric: "4.6개".to_owned(),
                                label: "평균 거래 은행 수".to_owned(),
                                sublabel: Some("자산 53~54%는 주거래 한 곳에".to_owned()),
                                cite: Some("[1]".to_owned()),
                            }),
                            bullets_title: Some("근거".to_owned()),
                            bullets: vec![
                                EvidenceBullet::new("거래 축소·중단 이유의 **42%**가 '타행 비교'", Some("[1]")),
                                EvidenceBullet::new("**10명 중 6명**이 은행 거래를 줄이거나 중단한 경험", Some("[1]")),
                                EvidenceBullet::new("계좌를 옮기면 기존 **우대금리 혜택**이 사라질 수 있음", Some("[3]")),
                            ],
                            implication: Some(ImplicationBox {
                                title: "시사점".to_owned(),
                                body: "이익은 안 보이고 절차는 번거로워 '관성'으로 남는다.\n옮길 때의 손익과 순서를 대신 계산·계획해 줘야 한다.".to_owned(),
                            }),
                            accent_color: None,
                        },
                        DiagramNode::ComparisonCard {
                            id: Some("card_prob_b".to_owned()),
                            badge: "문제 B".to_owned(),
                            category: Some("정착 장벽".to_owned()),
                            headline: "옮겨도 머물지 않는다".to_owned(),
                            kpi: Some(KpiCallout {
                                metric: "16%".to_owned(),
                                label: "새 거래은행 거래 확대 의향".to_owned(),
                                sublabel: Some("유지 의향도 41%에 그침".to_owned()),
                                cite: Some("[2]".to_owned()),
                            }),
                            bullets_title: Some("근거".to_owned()),
                            bullets: vec![
                                EvidenceBullet::new("최근 1년 새 **10명 중 4명**이 신규 은행과 거래 시작", Some("[2]")),
                                EvidenceBullet::new("디지털 자산관리 경험 **80% 이상**·맞춤 추천 기대", Some("[2]")),
                                EvidenceBullet::new("앱테크 이용자 월평균 **6,947P** 적립·73.5% 현금화", Some("[4]")),
                            ],
                            implication: Some(ImplicationBox {
                                title: "시사점".to_owned(),
                                body: "옮긴 뒤에도 쓸 이유가 있어야 머문다.\n내 소비에 맞춘 추천과 반복 이용할 보상이 필요하다.".to_owned(),
                            }),
                            accent_color: None,
                        },
                    ],
                },
                DiagramNode::HeroContainer {
                    id: Some("solution_hero".to_owned()),
                    eyebrow: Some("SERVICE PROPOSAL / WON이사".to_owned()),
                    headline: "WON이사는 타행에서 쓰던 금융 습관을 분석해 유리한 주거래 전환을 계획하고,\n개인화·보상 경험으로 우리WON뱅킹에 계속 머무르게 하는 AI 금융 이사 매니저다.".to_owned(),
                    subheadline: None,
                    group_labels: vec![
                        StepGroupLabel { start_step: 1, end_step: 1, label: "문제 A 해결 · 전환 장벽 해소".to_owned() },
                        StepGroupLabel { start_step: 2, end_step: 4, label: "문제 B 해결 · 정착 동기 설계".to_owned() },
                    ],
                    step_chain: vec![
                        ProcessStep {
                            step: 1,
                            title: "금융 이사 견적·이전 계획".to_owned(),
                            badge: Some("2p".to_owned()),
                            weight: 2.0,
                            summary: None,
                            bullets: vec![
                                "옮기면 얼마나 이득인지 손익 견적".to_owned(),
                                "자동이체·급여 등 이전 대상 자동 분류".to_owned(),
                                "결제일·만기 고려한 순서별 이사 계획".to_owned(),
                            ],
                        },
                        ProcessStep {
                            step: 2,
                            title: "소비 분석·위비 매니저".to_owned(),
                            badge: Some("3p".to_owned()),
                            weight: 1.0,
                            summary: Some("소비 프로필로 캐릭터 매칭".to_owned()),
                            bullets: Vec::new(),
                        },
                        ProcessStep {
                            step: 3,
                            title: "맞춤 상품 추천".to_owned(),
                            badge: Some("4p".to_owned()),
                            weight: 1.0,
                            summary: Some("실제 순혜택 기준 추천".to_owned()),
                            bullets: Vec::new(),
                        },
                        ProcessStep {
                            step: 4,
                            title: "퀘스트·꿀머니·꾸미기".to_owned(),
                            badge: Some("5p".to_owned()),
                            weight: 1.0,
                            summary: Some("미션·보상으로 반복 이용".to_owned()),
                            bullets: Vec::new(),
                        },
                    ],
                },
                DiagramNode::CitationFooter {
                    id: Some("footer".to_owned()),
                    citations: vec![
                        CitationEntry { index: "[1]".to_owned(), source: "하나금융연구소, 금융소비자 보고서 2025".to_owned() },
                        CitationEntry { index: "[2]".to_owned(), source: "하나금융연구소, 금융소비자 보고서 2024".to_owned() },
                        CitationEntry { index: "[3]".to_owned(), source: "금융위원회, 계좌이동서비스 이용 안내".to_owned() },
                        CitationEntry { index: "[4]".to_owned(), source: "한국소비자원 조사 (뉴시스 보도, 2024)".to_owned() },
                    ],
                    page_indicator: Some("01 / 05".to_owned()),
                },
            ],
            connectors: Vec::new(),
        },
        connectors: vec![
            GluedConnector {
                id: None,
                from: PortRef { node_id: "card_prob_a".to_owned(), port: crate::spec::ConnectorPort::BottomCenter },
                to: PortRef { node_id: "solution_hero".to_owned(), port: crate::spec::ConnectorPort::TopQuarterLeft },
                style: ConnectorStyle::StraightArrow,
                label: None,
                color: None,
            },
            GluedConnector {
                id: None,
                from: PortRef { node_id: "card_prob_b".to_owned(), port: crate::spec::ConnectorPort::BottomCenter },
                to: PortRef { node_id: "solution_hero".to_owned(), port: crate::spec::ConnectorPort::TopQuarterRight },
                style: ConnectorStyle::StraightArrow,
                label: None,
                color: None,
            },
        ],
    }
}

fn template_oneoffice_proposal_showcase() -> DiagramSpec {
    DiagramSpec {
        schema_version: crate::spec::DIAGRAM_SCHEMA_V1.to_owned(),
        id: "diag_oneoffice_showcase".to_owned(),
        title: Some("차세대 오피스 통합 제안 쇼케이스".to_owned()),
        theme: make_theme_consulting_navy(),
        canvas: DiagramCanvas {
            target_width_pt: 425.2,
            padding_pt: 0.0,
            bg_color: None,
        },
        root: DiagramNode::VStack {
            id: Some("root".to_owned()),
            gap_pt: 12.0,
            padding_pt: 0.0,
            children: vec![
                DiagramNode::HeaderBar {
                    id: Some("hdr".to_owned()),
                    left_text: "ONEOFFICE × RFP 2026 / 차세대 오피스 통합".to_owned(),
                    right_badge: Some("EXECUTIVE SUMMARY".to_owned()),
                    show_divider: true,
                },
                DiagramNode::SectionHeading {
                    id: Some("head".to_owned()),
                    eyebrow: Some("01 / PROBLEM & SOLUTION".to_owned()),
                    title: "도식화 단절과 포맷 파편화의 근본적 해결".to_owned(),
                    lead: Some("기존 한글·워드는 도식과 표 수정을 지원하지 못해 PPT 캡처 이미지를 붙여넣는 비효율이 발생한다.\nONEOFFICE는 인플레이스 수식 표, 시맨틱 도식화, 비파괴 PDF 직인을 하나의 플랫폼으로 통합한다.".to_owned()),
                },
                DiagramNode::Grid {
                    id: Some("problems".to_owned()),
                    columns: 2,
                    gap_pt: 10.0,
                    equalize_card_heights: true,
                    connectors: Vec::new(),
                    children: vec![
                        DiagramNode::ComparisonCard {
                            id: Some("card_a".to_owned()),
                            badge: "문제 A".to_owned(),
                            category: Some("도식화 단절".to_owned()),
                            headline: "그림 하나에 3시간".to_owned(),
                            kpi: Some(KpiCallout {
                                metric: "3.4시간".to_owned(),
                                label: "장표당 도식 재작업 시간".to_owned(),
                                sublabel: Some("문구 수정 시 줄바꿈·화살표 어긋남".to_owned()),
                                cite: Some("[1]".to_owned()),
                            }),
                            bullets_title: Some("실무 현황".to_owned()),
                            bullets: vec![
                                EvidenceBullet::new("제안서 작성 시간의 **45%**가 서식 및 도식 정렬에 소모", Some("[1]")),
                                EvidenceBullet::new("캡처된 이미지는 **텍스트 검색 불가** 및 인쇄 화질 저하", Some("[1]")),
                                EvidenceBullet::new("수치 변경 시 표와 차트를 **수동으로 재캡처**해야 함", Some("[2]")),
                            ],
                            implication: Some(ImplicationBox {
                                title: "시사점".to_owned(),
                                body: "단순 정적 이미지 붙여넣기는 죽은 데이터다.\n문서 안에서 즉시 크기 균등화와 자동 재배치가 되어야 한다.".to_owned(),
                            }),
                            accent_color: None,
                        },
                        DiagramNode::ComparisonCard {
                            id: Some("card_b".to_owned()),
                            badge: "문제 B".to_owned(),
                            category: Some("포맷 파편화".to_owned()),
                            headline: "5개 포맷 분열".to_owned(),
                            kpi: Some(KpiCallout {
                                metric: "68%".to_owned(),
                                label: "포맷 변환 시 서식 깨짐 경험".to_owned(),
                                sublabel: Some("HWPX·DOCX·XLSX·PDF 도구 단절".to_owned()),
                                cite: Some("[2]".to_owned()),
                            }),
                            bullets_title: Some("실무 현황".to_owned()),
                            bullets: vec![
                                EvidenceBullet::new("공문서(`.hwpx`)와 계약서(`.pdf`) 서명 날인을 위해 **별도 툴 3개 구동**", Some("[2]")),
                                EvidenceBullet::new("도장 이미지는 **흰 배경이 글씨를 가려** 계약서 검토 방해", Some("[3]")),
                                EvidenceBullet::new("AI 에이전트(MCP)가 기존 문서를 읽거나 **수정하지 못함**", Some("[4]")),
                            ],
                            implication: Some(ImplicationBox {
                                title: "시사점".to_owned(),
                                body: "포맷별 전용 뷰어 전환을 멈춰야 한다.\n단일 캔버스에서 엑셀 수식과 투명 직인이 완결되어야 한다.".to_owned(),
                            }),
                            accent_color: None,
                        },
                    ],
                },
                DiagramNode::HeroContainer {
                    id: Some("hero_sol".to_owned()),
                    eyebrow: Some("CORE ARCHITECTURE / ONEOFFICE 통합 플랫폼".to_owned()),
                    headline: "문서 작성부터 엑셀 수식 계산, 26종 벡터 차트, 비파괴 PDF 직인 날인,\n그리고 AI Agent(MCP) 폐루프 자가 검증까지 단 하나의 플랫폼으로 완주한다.".to_owned(),
                    subheadline: None,
                    group_labels: vec![
                        StepGroupLabel { start_step: 1, end_step: 2, label: "인플레이스 복합 개체".to_owned() },
                        StepGroupLabel { start_step: 3, end_step: 4, label: "서명 & AI 자가 검증".to_owned() },
                    ],
                    step_chain: vec![
                        ProcessStep {
                            step: 1,
                            title: "513개 엑셀 수식 표".to_owned(),
                            badge: Some("2p".to_owned()),
                            weight: 1.5,
                            summary: None,
                            bullets: vec![
                                "=SUM, =CAGR 실시간 계산".to_owned(),
                                "한컴 단축키 F5, M, S 100% 호환".to_owned(),
                                ".xlsx 사이드카 무손실 동기화".to_owned(),
                            ],
                        },
                        ProcessStep {
                            step: 2,
                            title: "시맨틱 도식화 DSL".to_owned(),
                            badge: Some("3p".to_owned()),
                            weight: 1.5,
                            summary: None,
                            bullets: vec![
                                "한글 글자 잘림 0% 자동 레이아웃".to_owned(),
                                "좌우 카드 높이 칼정렬 균등화".to_owned(),
                                "OWPML + DrawingML + SVG 출력".to_owned(),
                            ],
                        },
                        ProcessStep {
                            step: 3,
                            title: "비파괴 PDF 직인 날인".to_owned(),
                            badge: Some("4p".to_owned()),
                            weight: 1.0,
                            summary: Some("흰 배경 도장 자동 투명화\n(인) 앵커 자동 탐색 날인".to_owned()),
                            bullets: Vec::new(),
                        },
                        ProcessStep {
                            step: 4,
                            title: "AI MCP 자율 검증".to_owned(),
                            badge: Some("5p".to_owned()),
                            weight: 1.0,
                            summary: Some("visual.verify_page 자가 검증\n오버플로우 0% 무결점 완주".to_owned()),
                            bullets: Vec::new(),
                        },
                    ],
                },
                DiagramNode::CitationFooter {
                    id: Some("foot".to_owned()),
                    citations: vec![
                        CitationEntry { index: "[1]".to_owned(), source: "한국공공SW협회, 공공 제안서 작성 생산성 실태조사 (2025)".to_owned() },
                        CitationEntry { index: "[2]".to_owned(), source: "한국IT서비스산업협회, 공문서 업무 효율화 리포트 (2024)".to_owned() },
                        CitationEntry { index: "[3]".to_owned(), source: "전자문서산업협회, 비대면 전자계약 직인 도입 실태 (2025)".to_owned() },
                        CitationEntry { index: "[4]".to_owned(), source: "ONEOFFICE 아키텍처 및 벤치마크 백서 (2026)".to_owned() },
                    ],
                    page_indicator: Some("01 / 05".to_owned()),
                },
            ],
            connectors: Vec::new(),
        },
        connectors: vec![
            GluedConnector {
                id: None,
                from: PortRef { node_id: "card_a".to_owned(), port: crate::spec::ConnectorPort::BottomCenter },
                to: PortRef { node_id: "hero_sol".to_owned(), port: crate::spec::ConnectorPort::TopQuarterLeft },
                style: ConnectorStyle::StraightArrow,
                label: None,
                color: None,
            },
            GluedConnector {
                id: None,
                from: PortRef { node_id: "card_b".to_owned(), port: crate::spec::ConnectorPort::BottomCenter },
                to: PortRef { node_id: "hero_sol".to_owned(), port: crate::spec::ConnectorPort::TopQuarterRight },
                style: ConnectorStyle::StraightArrow,
                label: None,
                color: None,
            },
        ],
    }
}

fn template_financial_switch_proposal() -> DiagramSpec {
    template_problem_comparison_to_solution()
}

fn template_kpi_executive_summary() -> DiagramSpec {
    DiagramSpec {
        schema_version: crate::spec::DIAGRAM_SCHEMA_V1.to_owned(),
        id: "diag_kpi_summary".to_owned(),
        title: Some("주요 경영 성과 지표 요약".to_owned()),
        theme: make_theme_consulting_navy(),
        canvas: DiagramCanvas {
            target_width_pt: 425.2,
            padding_pt: 0.0,
            bg_color: None,
        },
        root: DiagramNode::VStack {
            id: Some("root".to_owned()),
            gap_pt: 12.0,
            padding_pt: 0.0,
            children: vec![
                DiagramNode::HeaderBar {
                    id: Some("hdr".to_owned()),
                    left_text: "2026 비즈니스 성과 보고".to_owned(),
                    right_badge: Some("EXECUTIVE DASHBOARD".to_owned()),
                    show_divider: true,
                },
                DiagramNode::SectionHeading {
                    id: Some("head".to_owned()),
                    eyebrow: Some("KEY METRICS".to_owned()),
                    title: "주요 경영 성과 지표 요약".to_owned(),
                    lead: Some("플랫폼 전환 후 업무 처리 시간 단축 및 문서 작성 생산성 개선 추이를 요약합니다.".to_owned()),
                },
                DiagramNode::KpiRow {
                    id: Some("kpi_grid".to_owned()),
                    gap_pt: 12.0,
                    items: vec![
                        KpiMetricCard {
                            id: Some("k1".to_owned()),
                            label: "문서 작성 소요 시간".to_owned(),
                            metric: "68% 감소".to_owned(),
                            delta: Some("▼ 2.4시간 단축".to_owned()),
                            description: Some("인플레이스 도식화 효과".to_owned()),
                            cite: None,
                        },
                        KpiMetricCard {
                            id: Some("k2".to_owned()),
                            label: "수식 오류율".to_owned(),
                            metric: "0.0%".to_owned(),
                            delta: Some("완벽한 엑셀 계산 보장".to_owned()),
                            description: Some("513 함수 엔진 검증".to_owned()),
                            cite: None,
                        },
                        KpiMetricCard {
                            id: Some("k3".to_owned()),
                            label: "전자서명 처리 속도".to_owned(),
                            metric: "1.2초".to_owned(),
                            delta: Some("▲ 10배 고속화".to_owned()),
                            description: Some("자동 앵커 탐색 날인".to_owned()),
                            cite: None,
                        },
                    ],
                },
            ],
            connectors: Vec::new(),
        },
        connectors: Vec::new(),
    }
}

fn template_step_process_pipeline() -> DiagramSpec {
    DiagramSpec {
        schema_version: crate::spec::DIAGRAM_SCHEMA_V1.to_owned(),
        id: "diag_pipeline".to_owned(),
        title: Some("5단계 프로젝트 추진 파이프라인".to_owned()),
        theme: make_theme_consulting_navy(),
        canvas: DiagramCanvas {
            target_width_pt: 425.2,
            padding_pt: 0.0,
            bg_color: None,
        },
        root: DiagramNode::VStack {
            id: Some("root".to_owned()),
            gap_pt: 12.0,
            padding_pt: 0.0,
            children: vec![
                DiagramNode::SectionHeading {
                    id: Some("head".to_owned()),
                    eyebrow: Some("METHODOLOGY".to_owned()),
                    title: "5단계 프로젝트 추진 파이프라인".to_owned(),
                    lead: Some("요구사항 정의부터 아키텍처 수립, 병렬 구현, 실측 검증 및 릴리즈까지의 표준 절차입니다.".to_owned()),
                },
                DiagramNode::HeroContainer {
                    id: Some("pipeline_box".to_owned()),
                    eyebrow: Some("DELIVERY WORKFLOW".to_owned()),
                    headline: "체계적인 검증 게이트를 거쳐 무결점 프로덕션 품질을 보장합니다.".to_owned(),
                    subheadline: None,
                    group_labels: Vec::new(),
                    step_chain: vec![
                        ProcessStep { step: 1, title: "1. 요구사항 정의".to_owned(), badge: Some("W1".to_owned()), weight: 1.0, summary: Some("기획 및 인터페이스 확정".to_owned()), bullets: Vec::new() },
                        ProcessStep { step: 2, title: "2. 병렬 코어 구현".to_owned(), badge: Some("W2".to_owned()), weight: 1.0, summary: Some("수식/도식/PDF 엔진".to_owned()), bullets: Vec::new() },
                        ProcessStep { step: 3, title: "3. UI/UX 통합".to_owned(), badge: Some("W3".to_owned()), weight: 1.0, summary: Some("글래스 스튜디오 연동".to_owned()), bullets: Vec::new() },
                        ProcessStep { step: 4, title: "4. 실측 QA 감사".to_owned(), badge: Some("W4".to_owned()), weight: 1.0, summary: Some("6단계 무신뢰 게이트".to_owned()), bullets: Vec::new() },
                        ProcessStep { step: 5, title: "5. 정식 릴리즈".to_owned(), badge: Some("W5".to_owned()), weight: 1.0, summary: Some("3대 OS 및 VSIX 배포".to_owned()), bullets: Vec::new() },
                    ],
                },
            ],
            connectors: Vec::new(),
        },
        connectors: Vec::new(),
    }
}

fn template_as_is_to_be_bridge() -> DiagramSpec {
    DiagramSpec {
        schema_version: crate::spec::DIAGRAM_SCHEMA_V1.to_owned(),
        id: "diag_as_is_to_be".to_owned(),
        title: Some("As-Is / To-Be 비교 및 변화 모델".to_owned()),
        theme: make_theme_consulting_navy(),
        canvas: DiagramCanvas {
            target_width_pt: 425.2,
            padding_pt: 0.0,
            bg_color: None,
        },
        root: DiagramNode::VStack {
            id: Some("root".to_owned()),
            gap_pt: 12.0,
            padding_pt: 0.0,
            children: vec![
                DiagramNode::SectionHeading {
                    id: Some("head".to_owned()),
                    eyebrow: Some("TRANSFORMATION".to_owned()),
                    title: "As-Is / To-Be 비교 및 변화 모델".to_owned(),
                    lead: Some("기존 파편화된 오피스 환경에서 ONEOFFICE 통합 플랫폼 도입 후의 비즈니스 가치 변화입니다.".to_owned()),
                },
                DiagramNode::AsIsToBe {
                    id: Some("bridge".to_owned()),
                    as_is_title: "현재 (As-Is)".to_owned(),
                    as_is_items: vec![
                        AsIsToBeItem { headline: "수동 캡처 반복".to_owned(), detail: Some("PPT·엑셀 캡처 후 한글 문서에 붙여넣기".to_owned()) },
                        AsIsToBeItem { headline: "도장 배경 오염".to_owned(), detail: Some("흰 배경 스탬프로 인한 문서 가림".to_owned()) },
                        AsIsToBeItem { headline: "AI 수정 불가".to_owned(), detail: Some("정적 이미지로 텍스트 파싱 불가".to_owned()) },
                    ],
                    catalyst_label: "ONEOFFICE 통합".to_owned(),
                    to_be_title: "미래 (To-Be)".to_owned(),
                    to_be_items: vec![
                        AsIsToBeItem { headline: "인플레이스 복합 편집".to_owned(), detail: Some("문서 안에서 엑셀 수식·차트 즉시 조작".to_owned()) },
                        AsIsToBeItem { headline: "자동 투명 직인".to_owned(), detail: Some("순수 Rust 알파 매팅으로 깨끗한 날인".to_owned()) },
                        AsIsToBeItem { headline: "AI 폐루프 자가 검증".to_owned(), detail: Some("MCP로 문서 생성부터 렌더링 검증 완주".to_owned()) },
                    ],
                },
            ],
            connectors: Vec::new(),
        },
        connectors: Vec::new(),
    }
}

fn template_layered_architecture_stack() -> DiagramSpec {
    DiagramSpec {
        schema_version: crate::spec::DIAGRAM_SCHEMA_V1.to_owned(),
        id: "diag_arch_stack".to_owned(),
        title: Some("4계층 시스템 아키텍처 스택".to_owned()),
        theme: make_theme_consulting_navy(),
        canvas: DiagramCanvas {
            target_width_pt: 425.2,
            padding_pt: 0.0,
            bg_color: None,
        },
        root: DiagramNode::VStack {
            id: Some("root".to_owned()),
            gap_pt: 12.0,
            padding_pt: 0.0,
            children: vec![
                DiagramNode::SectionHeading {
                    id: Some("head".to_owned()),
                    eyebrow: Some("SYSTEM ARCHITECTURE".to_owned()),
                    title: "4계층 시스템 아키텍처 스택".to_owned(),
                    lead: Some("하위 코어 엔진부터 프레젠테이션 및 AI 에이전트 인터페이스까지의 계층 구조입니다.".to_owned()),
                },
                DiagramNode::ArchitectureStack {
                    id: Some("stack".to_owned()),
                    layers: vec![
                        ArchitectureLayer {
                            level: 4,
                            name: "프레젠테이션 & UX 계층".to_owned(),
                            components: vec!["VS Code Custom Editors".to_owned(), "Native Desktop (macOS/Win/Linux)".to_owned(), "Frosted Glass HUD".to_owned()],
                        },
                        ArchitectureLayer {
                            level: 3,
                            name: "AI & 프로토콜 계층".to_owned(),
                            components: vec!["Model Context Protocol (MCP)".to_owned(), "visual.verify_page 자율 루프".to_owned(), "Live GUI Bridge".to_owned()],
                        },
                        ArchitectureLayer {
                            level: 2,
                            name: "통합 복합개체 & 도식 엔진 계층".to_owned(),
                            components: vec!["oneoffice-bridge (513함수/26차트)".to_owned(), "oneoffice-diagram (3-Pass 솔버)".to_owned(), "oneoffice-pdf (비파괴 직인)".to_owned()],
                        },
                        ArchitectureLayer {
                            level: 1,
                            name: "기반 포맷 & 폰트 커널 계층".to_owned(),
                            components: vec!["KS X 6101 OWPML / HWP".to_owned(), "ECMA-376 OOXML".to_owned(), "PDF 1.7 (craft-pdf)".to_owned(), "HarfRust / Skrifa".to_owned()],
                        },
                    ],
                },
            ],
            connectors: Vec::new(),
        },
        connectors: Vec::new(),
    }
}

fn template_gantt_milestone_roadmap() -> DiagramSpec {
    DiagramSpec {
        schema_version: crate::spec::DIAGRAM_SCHEMA_V1.to_owned(),
        id: "diag_gantt".to_owned(),
        title: Some("추진 일정 및 마일스톤 로드맵".to_owned()),
        theme: make_theme_consulting_navy(),
        canvas: DiagramCanvas {
            target_width_pt: 425.2,
            padding_pt: 0.0,
            bg_color: None,
        },
        root: DiagramNode::VStack {
            id: Some("root".to_owned()),
            gap_pt: 12.0,
            padding_pt: 0.0,
            children: vec![
                DiagramNode::SectionHeading {
                    id: Some("head".to_owned()),
                    eyebrow: Some("ROADMAP".to_owned()),
                    title: "추진 일정 및 마일스톤 로드맵".to_owned(),
                    lead: Some("스프린트별 핵심 트랙의 병렬 진행 및 실측 릴리즈 목표입니다.".to_owned()),
                },
                DiagramNode::GanttRoadmap {
                    id: Some("roadmap".to_owned()),
                    time_periods: vec!["Wave 0".to_owned(), "Wave 1".to_owned(), "Wave 2".to_owned(), "Wave 3".to_owned()],
                    tracks: vec![
                        GanttTrack {
                            name: "코어 엔진".to_owned(),
                            items: vec![
                                GanttItem { label: "루트 워크스페이스 셋업".to_owned(), start_idx: 0, span_count: 1, completed: true },
                                GanttItem { label: "수식/차트/도식/PDF 브릿지".to_owned(), start_idx: 1, span_count: 1, completed: true },
                            ],
                        },
                        GanttTrack {
                            name: "스튜디오 UX & MCP".to_owned(),
                            items: vec![
                                GanttItem { label: "인플레이스 HUD & MCP 서버".to_owned(), start_idx: 2, span_count: 1, completed: false },
                            ],
                        },
                        GanttTrack {
                            name: "패키징 & 릴리즈".to_owned(),
                            items: vec![
                                GanttItem { label: "3대 OS + VSIX 공식 배포".to_owned(), start_idx: 3, span_count: 1, completed: false },
                            ],
                        },
                    ],
                },
            ],
            connectors: Vec::new(),
        },
        connectors: Vec::new(),
    }
}

fn template_swot_4quadrant() -> DiagramSpec {
    DiagramSpec {
        schema_version: crate::spec::DIAGRAM_SCHEMA_V1.to_owned(),
        id: "diag_swot".to_owned(),
        title: Some("SWOT 4사분면 전략 매트릭스".to_owned()),
        theme: make_theme_consulting_navy(),
        canvas: DiagramCanvas {
            target_width_pt: 425.2,
            padding_pt: 0.0,
            bg_color: None,
        },
        root: DiagramNode::VStack {
            id: Some("root".to_owned()),
            gap_pt: 12.0,
            padding_pt: 0.0,
            children: vec![
                DiagramNode::SectionHeading {
                    id: Some("head".to_owned()),
                    eyebrow: Some("STRATEGIC ANALYSIS".to_owned()),
                    title: "SWOT 4사분면 전략 매트릭스".to_owned(),
                    lead: Some("내부 역량(강점/약점)과 외부 환경(기회/위협)을 종합 분석합니다.".to_owned()),
                },
                DiagramNode::Grid {
                    id: Some("swot_grid".to_owned()),
                    columns: 2,
                    gap_pt: 10.0,
                    equalize_card_heights: true,
                    connectors: Vec::new(),
                    children: vec![
                        DiagramNode::ComparisonCard {
                            id: Some("card_s".to_owned()),
                            badge: "S".to_owned(),
                            category: Some("강점 (Strengths)".to_owned()),
                            headline: "순수 Rust 5대 포맷 통합".to_owned(),
                            kpi: None,
                            bullets_title: Some("핵심 요인".to_owned()),
                            bullets: vec![
                                EvidenceBullet::new("513개 엑셀 수식 및 26종 차트 완비", None),
                                EvidenceBullet::new("순수 Rust 기반 무손실 PDF 뷰어/직인", None),
                            ],
                            implication: None,
                            accent_color: Some("#059669".to_owned()),
                        },
                        DiagramNode::ComparisonCard {
                            id: Some("card_w".to_owned()),
                            badge: "W".to_owned(),
                            category: Some("약점 (Weaknesses)".to_owned()),
                            headline: "신규 브랜드 인지도".to_owned(),
                            kpi: None,
                            bullets_title: Some("핵심 요인".to_owned()),
                            bullets: vec![
                                EvidenceBullet::new("기존 독점 오피스 대비 시장 인지도 확보 필요", None),
                                EvidenceBullet::new("커뮤니티 및 오픈소스 플러그인 생태계 확장 진행 중", None),
                            ],
                            implication: None,
                            accent_color: Some("#DC2626".to_owned()),
                        },
                    ],
                },
            ],
            connectors: Vec::new(),
        },
        connectors: Vec::new(),
    }
}

fn template_funnel_conversion() -> DiagramSpec {
    DiagramSpec {
        schema_version: crate::spec::DIAGRAM_SCHEMA_V1.to_owned(),
        id: "diag_funnel".to_owned(),
        title: Some("서비스 이용 퍼널 및 전환 분석".to_owned()),
        theme: make_theme_consulting_navy(),
        canvas: DiagramCanvas {
            target_width_pt: 425.2,
            padding_pt: 0.0,
            bg_color: None,
        },
        root: DiagramNode::VStack {
            id: Some("root".to_owned()),
            gap_pt: 12.0,
            padding_pt: 0.0,
            children: vec![
                DiagramNode::SectionHeading {
                    id: Some("head".to_owned()),
                    eyebrow: Some("CUSTOMER CONVERSION".to_owned()),
                    title: "서비스 이용 퍼널 및 전환 분석".to_owned(),
                    lead: Some("방문 유입부터 최종 전자서명 체결까지의 단계별 전환율입니다.".to_owned()),
                },
                DiagramNode::FunnelStages {
                    id: Some("funnel".to_owned()),
                    stages: vec![
                        FunnelStage { stage_name: "1. 유입 및 탐색".to_owned(), count_label: "100,000명".to_owned(), conversion_pct: "100%".to_owned(), strategic_action: Some("검색 및 랜딩 최적화".to_owned()) },
                        FunnelStage { stage_name: "2. 문서 템플릿 선택".to_owned(), count_label: "42,000명".to_owned(), conversion_pct: "42%".to_owned(), strategic_action: Some("10종 프리셋 원클릭 제공".to_owned()) },
                        FunnelStage { stage_name: "3. 수식표·도식 작성".to_owned(), count_label: "25,000명".to_owned(), conversion_pct: "60%".to_owned(), strategic_action: Some("인플레이스 편집 UX".to_owned()) },
                        FunnelStage { stage_name: "4. 전자서명 및 날인".to_owned(), count_label: "18,000명".to_owned(), conversion_pct: "72%".to_owned(), strategic_action: Some("자동 앵커 날인".to_owned()) },
                    ],
                },
            ],
            connectors: Vec::new(),
        },
        connectors: Vec::new(),
    }
}
