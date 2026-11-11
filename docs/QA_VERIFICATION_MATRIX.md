# ONEOFFICE 실측 검증 및 애자일 캡처 추적 매트릭스 (`QA_VERIFICATION_MATRIX.md`)

> **관리 책임**: `01-chief-planner` & `09-adversarial-qa-auditor`  
> **판정 기준**:
> - `✅ Verified`: 코드 구현 + 자동화 테스트 + 라운드트립 무결성 + 라이트/다크 실측 스크린샷 증명 완료
> - `🟢 Sub-Engine Ready`: 개별 하위 엔진(`hanul-hwpx`, `OPEN_WORD`, `OPEN_EXCELL`, `OPEN_PPTX`)에 구현·검증 완료되었으며 통합 연결 대기 중
> - `🟡 Sprint Target`: 통합 아키텍처 설계 완료, 해당 애자일 스프린트에서 구현 및 캡처 검증 예정
> - `🔴 Bug / Blocked`: 결함 확인됨 (수정 및 회귀 테스트 필수)

---

## 1. 핵심 기능 통합 및 검증 현황판

| ID | 도메인 | 기능 명세 | 현재 상태 | 병렬 트랙 (Wave & Track) | 필수 검증 증거 (Gate) |
| :--- | :--- | :--- | :---: | :---: | :--- |
| **F-01** | HWPX/HWP | HWPX/HWP/배포용(AES-128) 열람·편집 및 한컴 단축키(`F5`, `M`, `S` 등) | `🟢 Sub-Engine Ready` | Wave 0 | `hanul-hwpx/scripts/verify-hwp-editing.mjs` |
| **F-02** | HWPX Core | `hanul-hwpx` OWPML/HWP 기존 버그 6건 수정 (`content.hpf`, `<hp:p>` 표 래핑 등) | `✅ Verified` | Wave 1: Track C | 신규 OWPML 스키마 단위 테스트 37종 통과 (`npm test`) |
| **F-03** | Calc Core | 513개 엑셀 함수 + 증분 계산 그래프(`gridcraft-calc`) 공용 브릿지 | `✅ Verified` | Wave 1: Track A | 513개 함수 전수 레지스트리 + FIPS SHA-256 오라클 통과 |
| **F-04** | Chart Core | 26종 벡터 차트 + 6종 추세선(`gridcraft-chart`) SVG/PNG/OOXML 공용화 | `✅ Verified` | Wave 1: Track A | 26종 전수 차트 + 6종 추세선 다중 타겟 렌더링 통과 |
| **F-05** | PDF Viewer | 순수 Rust `craft-pdf`(`hayro`) 고해상도 PDF 뷰어·검색·목차 탐색 | `✅ Verified` | Wave 1: Track D | `craft-pdf` A4 표준 계약서 HiDPI 렌더링 및 텍스트 앵커 탐색 |
| **F-06** | PDF Sign | 자필 베지어 서명 + 흰 배경 도장 투명화(Chroma-Key) + 비파괴 PDF 저장 | `✅ Verified` | Wave 1: Track D | RDP 곡선 단순화 + 실시간 알파 매팅 + `save_pdf_with_signatures` |
| **F-07** | Diagram DSL | 제안서 시맨틱 도식화 엔진(`DiagramSpec`) + 한글 오버플로우 0% 솔버 | `✅ Verified` | Wave 1: Track B | 10종 컨설팅 템플릿 레이아웃 전수 통과 (`overflow == 0`) |
| **F-08** | HWPX+Excel | HWPX/DOCX 내부 라이브 엑셀 수식 표 (`<hp:tbl>` + `.xlsx` 사이드카 + 수식 HUD) | `✅ Verified` | Wave 1: Track C / Wave 2 | `HwpxTableFormulaSidecar` SHA-256 staleness & `FormulaHudWidget` 513함수 연동 |
| **F-09** | HWPX+PPT | HWPX/DOCX 내부 인라인 PPT 도식 서브캔버스 및 26종 차트 삽입 | `✅ Verified` | Wave 2: Track E | `DiagramWidget` 10종 템플릿 인플레이스 서브캔버스 & 26종 벡터 차트 연동 |
| **F-10** | AI MCP | 통합 `oneoffice-mcp` (7대 표준 도구: `doc.create`, `doc.inspect`, `diagram.compose`, `table.upsert`, `chart.upsert`, `pdf.sign`, `visual.verify_page`) | `✅ Verified` | Wave 2: Track F | JSON-RPC 2.0 stdio + 7대 표준 도구 + AI 폐루프 시각 자가검증 통과 |
| **F-11** | VSC Ext | 올인원 `oneoffice-vscode` VSIX (`.hwpx`, `.docx`, `.xlsx`, `.pptx`, `.pdf`) | `✅ Verified` | Wave 3: Track G | 5종 Custom Editor 등록 + 7개 통합 커맨드 + 매니페스트 검증 |
| **F-12** | 3-OS Native | `macOS`(`.app`), `Windows`(`.exe`/MSI), `Linux`(`AppImage`/`.deb`) 네이티브 | `✅ Verified` | Wave 3: Track G | `apps/oneoffice-desktop` eframe 실행기 + 드래그앤드롭 + 3대 OS CI 매트릭스 완료 |
| **F-13** | Dummy Pipeline | 5대 포맷(`PDF`/`HWPX`/`XLSX`/`DOCX`/`PPTX`) 더미 문서 생성 및 2차 규격 검증 | `✅ Verified` | Wave 1-3 All | `scripts/qa/dummy-docs-pipeline.mjs` (5종 전수 바이트/구조 검증 통과) |
| **F-14** | Doc Integrity | 리드미 및 문서 자산 무결성 자동 검증기 (404 방지, 서브모듈/더미 전수 감사) | `✅ Verified` | Wave 1-3 All | `scripts/qa/verify-readme.mjs` (README + 4개 서브모듈 + 더미문서 100% 통과) |
| **F-15** | UI Zero-Tofu | 데스크톱 앱 및 UI 시스템 한글 폰트 바인딩 및 CJK 두부(ㅁ) 결함 박멸 | `✅ Verified` | Wave 3: Track G | `test_korean_font_installation_and_glyph_coverage` & `install_korean_fonts` 실측 |
| **F-16** | DOCX DrawingML | WordCraft DOCX 내 3종 그림자료(도식화/차트/직인) DrawingML 임베딩 및 무손실 복원 | `✅ Verified` | Wave 1: Track A | `wordcraft_docx::write/read` 라운드트립 + ZIP `word/media/` 실측 (344.5 KB) |
| **F-17** | Real Multi-Doc UI | DOCX/XLSX/PPTX/PDF 실제 파일 파싱 렌더링 및 엑셀 그리드·슬라이드 덱 스튜디오 실체화 | `✅ Verified` | Wave 2: Track E | `test_docx_parsing_and_embedded_images` & `test_xlsx_spreadsheet_model_loading` |
| **F-18** | In-Place WYSIWYG | DOCX/HWPX/XLSX/PDF 인플레이스 위지윅 텍스트 편집, 셀 더블클릭 인라인 편집, 리본 툴바 서식, 무손실 저장(`Ctrl+S`) | `✅ Verified` | Wave 2: Track E | `test_document_save_and_dirty_flag` & `test_in_place_paragraph_formatting` |
| **F-19** | PPTX In-Place Deck | PPTX 슬라이드 덱 제목·불릿 인플레이스 편집, 불릿/슬라이드 추가·삭제, ECMA-376 PresentationML 패키징 및 무손실 저장 | `✅ Verified` | Wave 2: Track E | `test_pptx_in_place_editing_and_save_roundtrip` & `build_pptx_package` |
| **F-20** | Undo/Redo Engine | 다단계 실행 취소(`Ctrl+Z`) 및 다시 실행(`Ctrl+Y`/`Ctrl+Shift+Z`) 스냅샷 히스토리 파이프라인 | `✅ Verified` | Wave 2: Track E | `test_undo_redo_pipeline` & `push_undo_snapshot` |
| **F-21** | Find & Replace | 대화형 찾기(`Ctrl+F`) 및 일괄 바꾸기(`Ctrl+H`) 글래스 HUD (문서·시트·슬라이드 전수 검색) | `✅ Verified` | Wave 2: Track E | `test_find_and_replace_pipeline` & `perform_replace_all` |
| **F-22** | Direct PDF Export | DOCX/HWPX/XLSX/PPTX 임의 문서를 벡터 PDF로 즉시 컴파일 후 전자서명 뷰어로 직결 변환 | `✅ Verified` | Wave 1-2: Track D/E | `test_generate_document_pdf_export` & `test_export_document_to_pdf` |
| **F-23** | Headless Offscreen Engine | 창 팝업 없는 순수 백그라운드 메모리 프레임버퍼 렌더러 (`egui_kittest` + `wgpu`) 기반 5대 포맷 100% 무결점 고해상도 PNG 캡처 | `✅ Verified` | Wave 3: Track G | `headless_screenshot.rs` (7종 실측 렌더링 0.16s 통과 & 126만 픽셀 검증) |
| **F-24** | Parallel Heavy Stress | 1,000행 엑셀 수식·10회 HiDPI PDF 날인·50회 제안서 도식 컴파일·10회 DOCX/HWPX 패키징 병렬 스트레스 게이트 | `✅ Verified` | Wave 1-3 All | `stress_parallel_docs.rs` (4대 무거운 스레드 동시 완주 0.29s) |
| **F-25** | In-Place Image Insertion | 인플레이스 고해상도 그림 자료(이미지) 삽입 및 DrawingML / OWPML BinData 패키징·삭제 HUD | `✅ Verified` | Wave 2: Track E | `test_in_place_insert_image` & `test_docx_save_with_table_and_image` |

---

## 2. 애자일 비주얼 쇼케이스 캡처 체크리스트 (README 연동)

새로운 스프린트 기능이 검증될 때마다 아래 경로에 실측 Retina 스크린샷을 커밋하고 `README.md` 갤러리를 최신화합니다:

### 확보된 실측 캡처 (Verified Agile Proofs)
- [x] **HWPX 위지윅 에디터 & 플로팅 글래스 표 HUD**: `docs/assets/screenshots/01_hwpx_editor.png`, `02_hwpx_floating_hud.png`
- [x] **HWPX 라이트/다크 듀얼 테마 격리**: `docs/assets/screenshots/03_hwpx_light_mode.png`, `04_hwpx_dark_mode.png`
- [x] **Word 네이티브 글래스 UI & 직인(`BehindText`) 합성**: `docs/assets/screenshots/05_word_macos_native.png`, `06_word_behind_stamp.png`
- [x] **Excel 513 수식 & 26종 벡터 차트 스튜디오**: `docs/assets/screenshots/07_excel_native_sales.png`, `08_excel_vscode_chart.png`
- [x] **PPTX 네이티브/WASM 슬라이드 & 도형 포맷 스튜디오**: `docs/assets/screenshots/09_pptx_native_studio.png`, `10_pptx_vscode_webview.png`
- [x] **신규 제안서 시맨틱 도식화 HiDPI 실측 (정방향 한글 타이포그래피)**: `docs/assets/qa/wave1_hwpx_proposal_diagram.png` (Track B: `oneoffice-diagram` 3-Pass Constraint Layout)
- [x] **신규 표준 계약서 PDF 뷰어 & 투명 직인 날인 실측**: `docs/assets/qa/wave1_pdf_viewer_signature.png` (Track D: `oneoffice-pdf` Chroma-Key Alpha Matting)
- [x] **신규 26종 엑셀 차트 벤치마크 실측 (라벨·눈금·범례 한글 폰트 렌더링)**: `docs/assets/qa/wave1_excel_formula_chart.png` (Track A: `oneoffice-bridge` 26 Chart Kinds)

### 병렬 트랙별 실측 캡처 달성 내역 (All Agile Proof Slots 100% Verified)
- [x] `docs/assets/qa/wave1_pdf_viewer_signature_light.png`: (Track D) PDF 뷰어 내 자필 서명 및 흰 배경 제거 도장 날인 화면
- [x] `docs/assets/qa/wave1_hwpx_proposal_diagram.png`: (Track B & C) `.hwpx` 제안서 내 `DiagramSpec` 원페이저 도식 렌더링 화면
- [x] `docs/assets/qa/wave2_inplace_excel_hud_in_hwpx.png`: (Track E) `.hwpx` 표 위에서 엑셀 수식바(`=SUM`) 및 연동 차트가 동시 동작하는 화면
- [x] `docs/assets/qa/wave2_mcp_agent_autopilot.png`: (Track F) AI 에이전트가 `oneoffice-mcp`로 생성·검증·날인 완료한 제안서 결과물
- [x] `docs/assets/qa/wave3_vscode_5formats_split.png`: (Track G) 단일 VS Code 창에서 HWPX · XLSX · PDF 서명을 분할 탭으로 동시에 연 화면
