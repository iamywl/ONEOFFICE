# ONEOFFICE 기능 결손 및 엣지케이스 추적 원장 (`GAPS.md`)

> **관리 주체**: `01-chief-planner` 및 전체 개발 서브에이전트  
> **목적**: 개발 및 실무 문서 파싱 과정에서 발견된 미지원 태그, 숨은 기능, 포맷 엣지케이스를 기록하고 체계적으로 해결합니다.  
> **상태 범례**: `🔴 Open (미해결)` · `🟡 In Progress (구현 중)` · `🟢 Resolved (회귀 테스트 통과 및 해결)`

---

## 1. 현재 식별된 결손 목록 (Identified Gaps)

| GAP-ID | 도메인 | 기능 및 결손 내용 | 심각도 | 재현 픽스처 / 위치 | 해결 상태 |
| :--- | :--- | :--- | :---: | :--- | :---: |
| **GAP-HWPX-01** | HWPX | `hanul_insert_image` 시 `<hc:img>` 및 `content.hpf` 매니페스트 누락 | `P0` | `hanul-hwpx/src/mcp/server.ts:1210` | `🟢 Resolved` |
| **GAP-HWPX-02** | HWPX | `hanul_append_content` 시 `<hp:tbl>`이 `<hp:p><hp:run>` 없이 `<hs:sec>` 직하위 삽입 | `P0` | `hanul-hwpx/src/mcp/server.ts:1330` | `🟢 Resolved` |
| **GAP-HWPX-03** | HWPX | `find_and_replace`가 raw XML 문자열 정규식으로 치환되어 태그 파손 위험 | `P0` | `hanul-hwpx/src/mcp/server.ts:1039` | `🟢 Resolved` |
| **GAP-HWPX-04** | HWPX | `fill_fields` 시 문단 전체의 형제 `<hp:t>` 텍스트가 공백으로 삭제됨 | `P1` | `hanul-hwpx/src/mcp/server.ts:1538` | `🟢 Resolved` |
| **GAP-HWPX-05** | HWPX | `hwpx-parser.ts`에서 표 셀 내부 텍스트가 본문 `paragraphs`에 중복 수집됨 | `P1` | `hanul-hwpx/src/core/hwpx-parser.ts:223` | `🟢 Resolved` |
| **GAP-HWPX-06** | HWP 5.x | HWP 압축 플래그 시 `BinData` 및 `bodytext` 필드가 압축 해제 없이 raw 읽힘 | `P1` | `hanul-hwpx/src/core/hwp-parser.ts` | `🟢 Resolved` |
| **GAP-PPTX-01** | PPTX | 슬라이드 차트 데이터 수정 시 임베디드 `.xlsx` 워크북 파츠가 증발함 (`#335`) | `P1` | `OPEN_PPTX/LAYOUT_PRECISION_GAP_REPORT.md` | `🟢 Resolved` |
| **GAP-PPTX-02** | PPTX | `src/deckcraftEditor.ts:636`에서 `cmdId`가 `message.engineCmd` 누락으로 `undefined` 전달 | `P1` | `OPEN_PPTX/src/deckcraftEditor.ts:636` | `🟢 Resolved` |
| **GAP-WORD-01** | Word | `BehindText` 직인 위 본문 텍스트 클릭 시 직인 히트테스트가 간섭받거나 선택 불가 | `P1` | `OPEN_WORD/crates/layout/src/hit.rs:418` | `🟢 Resolved` |
| **GAP-DIAG-01** | Diagram | 한글 어절 줄바꿈 시 카드 패딩 경계에서의 1글자 흘림(Orphan line-wrap) 현상 | `P1` | `tests/fixtures/korean_proposal_ref.hwpx` | `🟢 Resolved` |
| **GAP-PDF-01** | PDF | 스캔/공공기관 PDF의 변환 행렬(`cm`) 미복원으로 인한 서명 XObject 오배치 위험 | `P1` | 은행 발급 거래내역서 / 공공기관 신청서 PDF | `🟢 Resolved` |
| **GAP-DIAG-02** | Diagram | `compile_png.rs`에서 `scale_non_uniform(k, -k)` 이중 Y-flip으로 인한 도식화 글리프 상하 반전 | `P0` | `crates/oneoffice-diagram/src/compile_png.rs:138` | `🟢 Resolved` |
| **GAP-EXCEL-01** | Excel/Chart | `gridcraft_chart::rasterize`의 `Prim::Text` 스텁으로 인한 차트 PNG 축/라벨/범례 텍스트 누락 | `P0` | `crates/oneoffice-bridge/src/chart.rs:904` | `🟢 Resolved` |

---

## 2. 신규 결손 등록 템플릿 (New Gap Template)

새로운 결손이나 미지원 기능을 발견한 에이전트는 아래 템플릿을 복사하여 위 표에 추가합니다:

```markdown
| GAP-XXX-NN | [도메인] | [결손 내용 한 줄 요약] | [P0/P1/P2] | [재현 픽스처 파일 경로] | 🔴 Open |
```
- **P0**: 데이터 유실, 파일 손상, 프로세스 크래시 (발견 즉시 최우선 해결)
- **P1**: 레이아웃 깨짐, 서식 누락, 단축키 먹통 (현재 스프린트 내 해결)
- **P2**: 부가 편의 기능, 희귀 특수 효과 (다음 스프린트 로드맵 편입)
