# ONEOFFICE 52종 전 기능 비주얼 쇼케이스 갤러리 (Showcase Gallery)

> **Benchmarked on High-Impact Open Source UX**  
> 본 갤러리는 ONEOFFICE의 5대 문서 포맷(HWPX, DOCX, XLSX, PPTX, PDF), 10대 제안서 도식화 템플릿, 26종 엑셀 차트 엔진, 라이트/다크 테마, 그리고 순수 Rust 전자서명/직인 날인의 실제 런타임 렌더링 화면을 실측 캡처로 집대성한 공식 쇼케이스입니다.

---

## 목차 (Table of Contents)
1. [Category 1. HWPX 에디터 & 인플레이스 수식표 (10종)](#category-1-hwpx-에디터--인플레이스-수식표-10종)
2. [Category 2. 10대 제안서 인포그래픽 도식화 템플릿 (10종)](#category-2-10대-제안서-인포그래픽-도식화-템플릿-10종)
3. [Category 3. DOCX 워드 & DrawingML 복합 그래픽 (8종)](#category-3-docx-워드--drawingml-복합-그래픽-8종)
4. [Category 4. XLSX 12열 풀 그리드 스프레드시트 (8종)](#category-4-xlsx-12열-풀-그리드-스프레드시트-8종)
5. [Category 5. PPTX 16:9 와이드스크린 슬라이드 덱 (8종)](#category-5-pptx-169-와이드스크린-슬라이드-덱-8종)
6. [Category 6. PDF 뷰어 & 무손실 전자서명/직인 날인 (8종)](#category-6-pdf-뷰어--무손실-전자서명직인-날인-8종)

---

## Category 1. HWPX 에디터 & 인플레이스 수식표 (10종)

HWPX(KS X 6101) 문서를 네이티브 OWPML로 파싱하여 2D 엑셀 수식표, 26종 벡터 차트, 제안서 도식을 인플레이스로 편집합니다.

| 번호 | 기능 명칭 | 실측 캡처 링크 | 주요 특징 및 검증 사항 |
| :---: | :--- | :---: | :--- |
| **01** | HWPX 기본 클린 뷰 | [`01_hwpx_clean_view.png`](assets/qa/gallery/01_hwpx_clean_view.png) | KS X 6101 OWPML 표준 제안서 다크 테마 기본 렌더링 |
| **02** | HWPX 라이트 모드 | [`02_hwpx_light_theme.png`](assets/qa/gallery/02_hwpx_light_theme.png) | 소프트 슬레이트 바탕 + 순백 용지 고대비(WCAG 4.5:1 이상) |
| **03** | B2 수량 셀 선택 | [`03_hwpx_table_cell_b2.png`](assets/qa/gallery/03_hwpx_table_cell_b2.png) | B2 포커스 링(`2px #2563eb`) & Formula HUD `fx B2 [ 5 ]` 동기화 |
| **04** | C2 단가 셀 선택 | [`04_hwpx_table_cell_c2.png`](assets/qa/gallery/04_hwpx_table_cell_c2.png) | C2 단가(`10000`) 활성 선택 및 우측 정렬 숫자 포맷 |
| **05** | D2 수식 합계 셀 | [`05_hwpx_table_cell_d2_sum.png`](assets/qa/gallery/05_hwpx_table_cell_d2_sum.png) | `=B2*C2` 연동 소계 계산(`50000`) 및 실시간 재계산 |
| **06** | 묶은 세로 막대형 차트 | [`06_hwpx_chart_column_clustered.png`](assets/qa/gallery/06_hwpx_chart_column_clustered.png) | 26종 엑셀 차트 엔진의 기본 세로 막대형 인라인 렌더링 |
| **07** | 꺾은선형 차트 | [`07_hwpx_chart_line.png`](assets/qa/gallery/07_hwpx_chart_line.png) | 추세 분석을 위한 고해상도 벡터 꺾은선형 차트 |
| **08** | 원형 차트 (Pie) | [`08_hwpx_chart_pie.png`](assets/qa/gallery/08_hwpx_chart_pie.png) | 예산 배분 비율을 직관적으로 보여주는 원형 파이 차트 |
| **09** | 화이트 고스트 툴바 | [`09_hwpx_ghost_block_toolbar.png`](assets/qa/gallery/09_hwpx_ghost_block_toolbar.png) | 본문 가독성을 해치지 않는 정돈된 화이트 고스트 블록 삽입 바 |
| **10** | 3단 복합 섹션 | [`10_hwpx_rich_compound_section.png`](assets/qa/gallery/10_hwpx_rich_compound_section.png) | 1. 수식표 + 2. 차트 + 3. 도식화 3단 구조 일체형 렌더링 |

---

## Category 2. 10대 제안서 인포그래픽 도식화 템플릿 (10종)

HarfRust 한글 글리프 실측 조판 엔진으로 글자 잘림(Overflow)과 충돌(Collision)이 0건인 컨설팅급 다이어그램입니다.

| 번호 | 템플릿 ID | 실측 캡처 링크 | 다이어그램 구성 및 활용처 |
| :---: | :--- | :---: | :--- |
| **11** | `problem-comparison-to-solution` | [`11_diagram_problem_comparison.png`](assets/qa/gallery/11_diagram_problem_comparison.png) | 문제 A/B 대조 카드 + 해결 솔루션 4단계 프로세스 원페이저 |
| **12** | `oneoffice-proposal-showcase` | [`12_diagram_showcase.png`](assets/qa/gallery/12_diagram_showcase.png) | ONEOFFICE 5대 핵심 가치 및 통합 아키텍처 쇼케이스 |
| **13** | `financial-switch-proposal` | [`13_diagram_financial_switch.png`](assets/qa/gallery/13_diagram_financial_switch.png) | 금융권 플랫폼 전환 제안 및 수수료 절감 비교 인포그래픽 |
| **14** | `kpi-executive-summary` | [`14_diagram_kpi_summary.png`](assets/qa/gallery/14_diagram_kpi_summary.png) | 임원 보고용 3대 핵심 성과 지표(시간 단축, 오류율, 서명 속도) 대시보드 |
| **15** | `step-process-pipeline` | [`15_diagram_step_pipeline.png`](assets/qa/gallery/15_diagram_step_pipeline.png) | 5단계 순차 프로젝트 추진 파이프라인 (요구사항→배포) |
| **16** | `as-is-to-be-bridge` | [`16_diagram_as_is_to_be.png`](assets/qa/gallery/16_diagram_as_is_to_be.png) | 현행(AS-IS) 한계와 목표(TO-BE) 혁신 간의 브릿지 도식 |
| **17** | `layered-architecture-stack` | [`17_diagram_layered_stack.png`](assets/qa/gallery/17_diagram_layered_stack.png) | UI/UX, 브릿지, 코어 엔진, 스토리지 4계층 스택 다이어그램 |
| **18** | `gantt-milestone-roadmap` | [`18_diagram_gantt_roadmap.png`](assets/qa/gallery/18_diagram_gantt_roadmap.png) | 스프린트 및 마일스톤 일정을 한눈에 조망하는 간트 로드맵 |
| **19** | `swot-4quadrant` | [`19_diagram_swot_quadrant.png`](assets/qa/gallery/19_diagram_swot_quadrant.png) | 강점, 약점, 기회, 위협을 분석하는 SWOT 4분면 전략 매트릭스 |
| **20** | `funnel-conversion` | [`20_diagram_funnel_conversion.png`](assets/qa/gallery/20_diagram_funnel_conversion.png) | 사용자 유입부터 계약 체결까지의 단계별 전환 퍼널 |

---

## Category 3. DOCX 워드 & DrawingML 복합 그래픽 (8종)

ONEOFFICE Word 엔진으로 단락 조판, 인라인 DrawingML 도식화/차트, BehindText 전자직인을 무손실 렌더링합니다.

| 번호 | 기능 명칭 | 실측 캡처 링크 | 주요 특징 및 검증 사항 |
| :---: | :--- | :---: | :--- |
| **21** | DOCX 다크 뷰 | [`21_docx_dark_view.png`](assets/qa/gallery/21_docx_dark_view.png) | 기술 보고서 표준 규격 다크 테마 렌더링 |
| **22** | DOCX 라이트 뷰 | [`22_docx_light_view.png`](assets/qa/gallery/22_docx_light_view.png) | 순백색 용지 + 소프트 슬레이트 프레임 라이트 뷰 |
| **23** | 인라인 도식화 카드 | [`23_docx_drawingml_diagram.png`](assets/qa/gallery/23_docx_drawingml_diagram.png) | 본문 내부에 삽입된 고해상도 DrawingML 제안서 도식 텍스처 |
| **24** | 인라인 차트 카드 | [`24_docx_drawingml_chart.png`](assets/qa/gallery/24_docx_drawingml_chart.png) | OOXML 패키지 내 `word/media/`에 바인딩된 26종 벡터 차트 |
| **25** | BehindText 직인 | [`25_docx_behind_text_stamp.png`](assets/qa/gallery/25_docx_behind_text_stamp.png) | 본문 텍스트 뒤에 배경으로 안착되어 텍스트 선택을 방해하지 않는 직인 |
| **26** | 문단 인플레이스 서식 | [`26_docx_paragraph_format.png`](assets/qa/gallery/26_docx_paragraph_format.png) | 헤딩/본문 스타일, 굵게/기울임/밑줄/형광펜 실시간 서식 변경 |
| **27** | 줌 레벨 스케일링 | [`27_docx_zoom_level.png`](assets/qa/gallery/27_docx_zoom_level.png) | 120% 확대 상태에서도 글자 깨짐 없는 고해상도 렌더링 |
| **28** | 부유 HUD 은닉 뷰 | [`28_docx_no_formula_hud.png`](assets/qa/gallery/28_docx_no_formula_hud.png) | 표가 없는 일반 문서 열람 시 불필요한 수식바를 숨겨 정돈된 뷰 유지 |

---

## Category 4. XLSX 12열 풀 그리드 스프레드시트 (8종)

ONEOFFICE Spreadsheet 513개 엑셀 수식 함수와 A~L 12열 × 18행 풀 뷰포트 격자망을 통해 정통 오피스 경험을 제공합니다.

| 번호 | 기능 명칭 | 실측 캡처 링크 | 주요 특징 및 검증 사항 |
| :---: | :--- | :---: | :--- |
| **29** | 시트 1 (예산 시트) | [`29_xlsx_sheet1_budget.png`](assets/qa/gallery/29_xlsx_sheet1_budget.png) | A~L 12열 × 18행 풀 스크린 격자망 및 예산 데이터 렌더링 |
| **30** | 시트 2 (집행내역) | [`30_xlsx_sheet2_execution.png`](assets/qa/gallery/30_xlsx_sheet2_execution.png) | 다중 워크시트 탭 스위칭 및 활성 탭 하이라이트 |
| **31** | 라이트 테마 시트 | [`31_xlsx_light_sheet.png`](assets/qa/gallery/31_xlsx_light_sheet.png) | 표준 엑셀 룩앤필의 라이트 테마 스프레드시트 격자망 |
| **32** | B2 포커스 링 | [`32_xlsx_focus_cell_b2.png`](assets/qa/gallery/32_xlsx_focus_cell_b2.png) | B2 예산 셀(`5000`) 포커스 링 & Formula HUD 동기화 |
| **33** | C2 실집행 셀 선택 | [`33_xlsx_focus_cell_c2.png`](assets/qa/gallery/33_xlsx_focus_cell_c2.png) | C2 실집행 셀(`4800`) 선택 및 숫자 데이터 우측 정렬 |
| **34** | 12열 풀 뷰포트 | [`34_xlsx_12cols_full_grid.png`](assets/qa/gallery/34_xlsx_12cols_full_grid.png) | 화면 80% 공백 낭비를 원천 박멸한 1440px 풀 가로 격자 전개 |
| **35** | 찾기/바꾸기 HUD | [`35_xlsx_find_replace_active.png`](assets/qa/gallery/35_xlsx_find_replace_active.png) | `Ctrl+F` 플로팅 글래스 HUD로 셀 내용 탐색 및 치환 |
| **36** | READY 상태바 | [`36_xlsx_ready_status_stats.png`](assets/qa/gallery/36_xlsx_ready_status_stats.png) | 활성 셀 좌표, 18×12 시트 규격, 513 함수 엔진 가동 상태 표시 |

---

## Category 5. PPTX 16:9 와이드스크린 슬라이드 덱 (8종)

ONEOFFICE Presentation 기반의 16:9 프레젠테이션 슬라이드 덱으로, 도식화 요소가 스크롤 잘림 없이 온전히 수납됩니다.

| 번호 | 기능 명칭 | 실측 캡처 링크 | 주요 특징 및 검증 사항 |
| :---: | :--- | :---: | :--- |
| **37** | 1번 슬라이드 (아키텍처) | [`37_pptx_slide1_architecture.png`](assets/qa/gallery/37_pptx_slide1_architecture.png) | 16:9 와이드스크린 황금비 슬라이드 캔버스 및 타이틀 |
| **38** | 2번 슬라이드 (시장 분석) | [`38_pptx_slide2_why_now.png`](assets/qa/gallery/38_pptx_slide2_why_now.png) | 좌측 썸네일 카드에서 2번 슬라이드 선택 및 내용 전개 |
| **39** | 슬라이드 라이트 모드 | [`39_pptx_light_slide.png`](assets/qa/gallery/39_pptx_light_slide.png) | 밝은 프레젠테이션 환경을 위한 라이트 테마 슬라이드 |
| **40** | 좌측 썸네일 카드 덱 | [`40_pptx_deck_thumbnails.png`](assets/qa/gallery/40_pptx_deck_thumbnails.png) | 번호 뱃지(`01`, `02`)와 미니 16:9 썸네일 슬라이드 목록 |
| **41** | 도식 수납 (클리핑 제로) | [`41_pptx_slide_full_diagram.png`](assets/qa/gallery/41_pptx_slide_full_diagram.png) | 내부 스크롤바 없이 슬라이드 프레임 내에 다이어그램 100% 수납 |
| **42** | 제목 인플레이스 편집 | [`42_pptx_slide_title_editing.png`](assets/qa/gallery/42_pptx_slide_title_editing.png) | 22pt 프레젠테이션 타이틀 실시간 편집 및 Dirty 상태 관리 |
| **43** | 불릿 항목 리스트 | [`43_pptx_bullet_points_list.png`](assets/qa/gallery/43_pptx_bullet_points_list.png) | 원클릭 `+ 새 불릿 항목 추가` 및 `X` 삭제 인터랙션 |
| **44** | 슬라이드 도식 교체 | [`44_pptx_template_switcher.png`](assets/qa/gallery/44_pptx_template_switcher.png) | 드롭다운을 통해 슬라이드 내 도식을 5단계 파이프라인으로 즉시 교체 |

---

## Category 6. PDF 뷰어 & 무손실 전자서명/직인 날인 (8종)

순수 Rust `craft-pdf`(`hayro` + `krilla`) 기반으로 고해상도 PDF를 렌더링하고 `(인)` 앵커에 직인을 무손실 날인합니다.

| 번호 | 기능 명칭 | 실측 캡처 링크 | 주요 특징 및 검증 사항 |
| :---: | :--- | :---: | :--- |
| **45** | 용역계약서 기본 뷰 | [`45_pdf_contract_clean_view.png`](assets/qa/gallery/45_pdf_contract_clean_view.png) | 표준 용역계약서 원본 PDF 다크 테마 고해상도 렌더링 |
| **46** | 계약서 라이트 모드 | [`46_pdf_contract_light_view.png`](assets/qa/gallery/46_pdf_contract_light_view.png) | 용지 중앙 정렬과 드롭 섀도우를 갖춘 라이트 테마 열람 |
| **47** | `(인)` 앵커 정밀 날인 | [`47_pdf_anchor_detected_seal.png`](assets/qa/gallery/47_pdf_anchor_detected_seal.png) | 계약서 본문 `(인)` 위치를 자동 탐색하여 적색 직인(`印`) 정밀 안착 |
| **48** | 직인 라벨 폭 확장 | [`48_pdf_seal_label_expanded.png`](assets/qa/gallery/48_pdf_seal_label_expanded.png) | 140pt 라벨 입력 필드로 `대표이사 원오피스` 문구 말림 완전 해소 |
| **49** | 120% 확대 열람 | [`49_pdf_zoom_120_percent.png`](assets/qa/gallery/49_pdf_zoom_120_percent.png) | 확대 상태에서도 텍스트 벡터 선명도 100% 유지 |
| **50** | A4 전체 수납 뷰포트 | [`50_pdf_full_a4_viewport.png`](assets/qa/gallery/50_pdf_full_a4_viewport.png) | 1020pt 높이로 계약서 최하단 서명 날인란까지 스크롤 없이 수납 |
| **51** | 서명 완료 상태바 | [`51_pdf_signing_status_bar.png`](assets/qa/gallery/51_pdf_signing_status_bar.png) | "페이지 1에 직인을 성공적으로 날인했습니다" 상태 피드백 |
| **52** | 직인 실행취소 스택 | [`52_pdf_signature_undoredo.png`](assets/qa/gallery/52_pdf_signature_undoredo.png) | 직인 날인 후 `서명 취소` 및 레이어 스택 비파괴 복원 |
