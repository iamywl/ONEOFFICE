# 시맨틱 도식화(`DiagramSpec`) 및 이중 무손실 복합 개체 기술 명세서

> **연관 에이전트**: `02-rust-core-bridge`, `03-diagram-layout-architect`, `04-hwpx-compound-engineer`, `06-studio-ux-craftsman`  
> **목적**: `.hwpx` 및 `.docx` 제안서·보고서 내부에서 엑셀 수식 표, 26종 차트, PPT 수준의 도식화(인포그래픽)가 **외부 뷰어 호환성(한컴오피스·MS Office)** 과 **재편집성(ONEOFFICE·AI Agent)** 을 동시에 100% 달성하기 위한 바이트·스키마 수준 설계 명세입니다.

---

## 1. 이중 무손실 복합 개체 아키텍처 (Dual-Persistence Package Spec)

### 1.1 HWPX (KS X 6101 / OWPML) 패키지 확장 구조
`.hwpx`는 ZIP 컨테이너 기반 개방형 표준입니다. ONEOFFICE는 표준 한컴오피스가 여는 데 아무런 지장이 없도록 표준 태그를 생성하는 동시에, `Contents/oneoffice/` 네임스페이스 아래에 라이브 재편집용 사이드카를 저장합니다.

```text
proposal.hwpx (ZIP Archive)
├── mimetype                                         # "application/hwp+zip" (무압축 STORE 필수)
├── META-INF/container.xml
├── Contents/
│   ├── content.hpf                                  # 모든 BinData, Chart, oneoffice 사이드카 매니페스트 등록
│   ├── header.xml                                   # 동적 할당된 charPr, paraPr, borderFill 정의
│   ├── section0.xml                                 # 표준 OWPML 본문 (<hp:tbl>, <hp:chart>, <hp:container>)
│   ├── Chart/
│   │   └── chart_01.xml                             # 표준 OOXML c:chartSpace 차트 정의
│   └── oneoffice/                                   # ONEOFFICE 확장 메타데이터 (표준 뷰어는 안전하게 무시)
│       ├── manifest.json                            # 개체 ID ↔ 본문 SHA-256 해시 매핑 (외부 편집 충돌 감지용)
│       ├── tables/tbl_01.xlsx                       # GridCraft 513개 수식·서식·이름정의 원본 워크북
│       ├── charts/chart_01.json                     # UnifiedChartEngine 26종 차트·추세선·테마 바인딩 명세
│       └── diagrams/diag_01.json                    # DiagramSpec v1 시맨틱 도식 트리 원본
└── BinData/
    └── chart_01_fallback.png                        # 구버전 뷰어 호환용 2x 고해상도 폴백 이미지
```

### 1.2 외부 편집 충돌 감지 및 양방향 동기화 (Staleness Reconciliation)
문서를 ONEOFFICE에서 저장한 뒤 **외부 한컴오피스(한글)나 MS Word에서 열어 수치만 직접 수정하고 저장한 경우** 데이터가 유실되지 않아야 합니다:
1. **저장 시**: ONEOFFICE는 렌더링된 `<hp:tbl>` 셀 표시 문자열 행렬의 정규화 해시(`rendered_cells_sha256`)를 `Contents/oneoffice/manifest.json`에 기록합니다.
2. **재열기 시**:
   - 현재 `section0.xml` 내 `<hp:tbl>`의 해시가 `rendered_cells_sha256`와 **일치**하면 → 사이드카(`tbl_01.xlsx`)의 수식과 스타일을 100% 그대로 복원합니다.
   - 해시가 **불일치**하면(외부 프로그램에서 셀 수정 발생) → 변경된 리터럴 셀 값을 `gridcraft-model`에 덮어쓴 뒤 `gridcraft-calc`로 의존 수식(`=SUM(...)` 등)을 즉시 재계산하고 사용자에게 비침습적 동기화 배지(`"외부 편집 셀 2건 반영 및 수식 재계산됨"`)를 표시합니다.

### 1.3 좌표계 정밀 변환 규격 (Coordinate & Unit Precision)
포맷 간 변환 시 부동소수점 누적 오차를 막기 위해 `wordcraft-geom` 단위계를 기준으로 고정밀 정수 변환을 수행합니다:
- **기준 단위**: `1 inch = 72 pt`
- **HWPX (`HWPUNIT`)**: `1 pt = 100 HWPUNIT` (`1 inch = 7,200 HWPUNIT`, `1 mm = 283.4646 HWPUNIT`)
  - A4 세로 기본 용지: `59,528 × 84,186 HWPUNIT` (`210mm × 297mm`)
  - 기본 본문 유효 폭(좌우 여백 각 `30mm = 8,504 HWPUNIT` 제외 시): **`42,520 HWPUNIT` (`425.2 pt`)**
- **DOCX (`Twip` / `EMU`)**: `1 pt = 20 Twip = 12,700 EMU`
- **PPTX (`Slide Pt` / `EMU`)**: 기본 와이드스크린 `960 × 540 pt` (`1 pt = 12,700 EMU`)

---

## 2. 제안서 시맨틱 도식화 DSL (`DiagramSpec` v1) 명세

### 2.1 설계 배경: 왜 좌표 기반이 아닌 시맨틱 트리인가?
실무 제안서 원페이저(예: `00 / WHY NOW 기획 배경 및 필요성`)는 단순 사각형의 나열이 아니라 **의미론적 위계(비교 카드 → KPI 강조 → 근거 불릿 → 시사점 → 연결 화살표 → 해결 아키텍처)** 를 가집니다. 좌표 대신 시맨틱 트리로 정의하면:
1. 문구 1줄을 추가해도 좌우 비교 카드의 높이와 화살표 길이가 자동으로 함께 늘어납니다.
2. 동일 `DiagramSpec`을 **A4 세로 본문 폭(`425.2 pt`)** 과 **16:9 슬라이드 폭(`960 pt`)** 에 맞춰 반응형으로 즉시 재배치할 수 있습니다.
3. AI 에이전트(MCP)가 좌표 계산 실수 없이 완벽한 도식을 1회 호출로 생성·수정합니다.

### 2.2 `DiagramSpec` v1 표준 JSON 스키마 예시
```json
{
  "schemaVersion": "oneoffice-diagram/v1",
  "id": "diag_why_now_01",
  "theme": {
    "preset": "consulting-navy-blue",
    "primary": "#0052CC",
    "heroBg": "#0F2942",
    "cardBg": "#FFFFFF",
    "cardBorder": "#CBD5E1",
    "subtleFill": "#F1F5F9",
    "implicationFill": "#E0F2FE",
    "fontFamily": "Pretendard"
  },
  "canvas": { "targetWidthPt": 425.2, "paddingPt": 0 },
  "root": {
    "kind": "VStack",
    "gapPt": 14,
    "children": [
      {
        "kind": "SectionHeading",
        "eyebrow": "00 / WHY NOW",
        "title": "기획 배경 및 필요성",
        "lead": "은행을 옮길 수는 있지만, '옮기는 수고'와 '머물 이유 부족' 때문에 주거래는 쉽게 바뀌지 않는다."
      },
      {
        "kind": "Grid",
        "id": "problem_grid",
        "columns": 2,
        "gapPt": 12,
        "equalizeCardHeights": true,
        "children": [
          {
            "kind": "ComparisonCard",
            "id": "card_prob_a",
            "badge": "문제 A",
            "category": "전환 장벽",
            "headline": "옮기기 어렵다",
            "kpi": { "metric": "4.6개", "label": "평균 거래 은행 수", "sublabel": "자산 53~54%는 주거래 한 곳에", "cite": "[1]" },
            "bulletsTitle": "근거",
            "bullets": [
              { "text": "거래 축소·중단 이유의 **42%**가 '타행 비교'", "cite": "[1]" },
              { "text": "**10명 중 6명**이 은행 거래를 줄이거나 중단한 경험", "cite": "[1]" }
            ],
            "implication": { "title": "시사점", "body": "이익은 안 보이고 절차는 번거로워 '관성'으로 남는다.\n옮길 때의 손익과 순서를 대신 계산·계획해 줘야 한다." }
          },
          {
            "kind": "ComparisonCard",
            "id": "card_prob_b",
            "badge": "문제 B",
            "category": "정착 장벽",
            "headline": "옮겨도 머물지 않는다",
            "kpi": { "metric": "16%", "label": "새 거래은행 거래 확대 의향", "sublabel": "유지 의향도 41%에 그침", "cite": "[2]" },
            "bulletsTitle": "근거",
            "bullets": [
              { "text": "최근 1년 새 **10명 중 4명**이 신규 은행과 거래 시작", "cite": "[2]" },
              { "text": "디지털 자산관리 경험 **80% 이상**·맞춤 추천 기대", "cite": "[2]" }
            ],
            "implication": { "title": "시사점", "body": "옮긴 뒤에도 쓸 이유가 있어야 머문다.\n내 소비에 맞춘 추천과 반복 이용할 보상이 필요하다." }
          }
        ]
      },
      {
        "kind": "HeroContainer",
        "id": "solution_box",
        "eyebrow": "SERVICE PROPOSAL / 통합 솔루션",
        "headline": "타행 금융 습관을 분석해 유리한 전환을 계획하고 보상 경험으로 정착시키는 AI 매니저",
        "stepChain": [
          { "step": 1, "title": "이사 견적·이전 계획", "badge": "2p", "weight": 2.0, "bullets": ["손익 자동 견적", "자동이체·급여 분류"] },
          { "step": 2, "title": "소비 분석 매니저", "badge": "3p", "weight": 1.0, "summary": "소비 프로필 매칭" },
          { "step": 3, "title": "맞춤 상품 추천", "badge": "4p", "weight": 1.0, "summary": "순혜택 기준 추천" },
          { "step": 4, "title": "퀘스트·리워드", "badge": "5p", "weight": 1.0, "summary": "미션·보상 반복 이용" }
        ]
      }
    ],
    "connectors": [
      { "from": { "nodeId": "card_prob_a", "port": "BottomCenter" }, "to": { "nodeId": "solution_box", "port": "TopQuarterLeft" }, "style": "straightArrow" },
      { "from": { "nodeId": "card_prob_b", "port": "BottomCenter" }, "to": { "nodeId": "solution_box", "port": "TopQuarterRight" }, "style": "straightArrow" }
    ]
  }
}
```

---

## 3. 한글 오버플로우 제로 3-Pass 제약 레이아웃 알고리즘

LLM이나 수동 좌표 계산에서 가장 자주 깨지는 **한글 줄바꿈 넘침**을 막기 위해 `crates/oneoffice-diagram`은 다음 3-Pass 솔버를 수행합니다:

1. **Pass 1 — 글리프 셰이핑 및 내재 크기 측정 (Bottom-Up Intrinsic Measure)**:
   - `wordcraft-fonts`의 `harfrust` + `skrifa`를 호출해 실제 번들/시스템 한글 폰트 메트릭으로 텍스트 폭을 측정합니다.
   - 한국어 금칙처리(Kinsoku: 행두 금칙 문자 `)`, `]`, `.`, `,`, `%` 등 분리 방지 및 어절 우선 줄바꿈)를 적용하여 주어진 가용 폭 `W`에서의 정확한 줄 수와 텍스트 높이 `H_text`를 산출합니다.
2. **Pass 2 — 그리드 트랙 분배 및 형제 기준선 정렬 (Top-Down Track & Baseline Equalization)**:
   - `Grid(equalizeCardHeights: true)` 내 모든 `ComparisonCard`의 외곽 높이를 `max(H_card_i)`로 통일합니다.
   - 카드 하단의 `ImplicationBox`(`시사점`)는 `margin-top: auto` 방식으로 하단 정렬하되, 좌우 카드의 `ImplicationBox` 상단 Y좌표 역시 `min(Y_imp_top)`으로 동기화하여 컨설팅 장표 특유의 칼같은 수평 정렬선을 완성합니다.
3. **Pass 3 — 절대 좌표 하강, 직교 커넥터 라우팅 & 불변식 검증 (Lowering & BBox Audit)**:
   - 모든 노드를 `(x_pt, y_pt, w_pt, h_pt)` 절대 좌표로 확정한 후 `GluedConnector` 포트 좌표를 연결합니다.
   - 컴파일 직전 `LayoutDiagnostics`를 실행하여 `text_bbox ⊆ inner_padding_bbox`(오차 허용 `0.5 pt`) 및 비의도적 형제 중첩 면적 `0`을 수학적으로 검증합니다.

---

## 4. 타겟 포맷별 네이티브 매핑 규격

| `DiagramSpec` 프리미티브 | HWPX (KS X 6101 OWPML) | DOCX / PPTX (ECMA-376 DrawingML) | PDF / Web Preview |
| :--- | :--- | :--- | :--- |
| 루트 프레임 | `<hp:container>` (`treatAsChar="1"` 또는 지정 앵커) | `<wgp:wgp>` / `<p:grpSp>` | `<svg viewBox="...">` / `krilla` |
| 둥근 카드 / 배지 | `<hp:rect curvetype="ROUND" ...>` + `<hp:lineShape>` + `<hc:fillBrush>` | `<wps:wsp><wps:spPr><a:prstGeom prst="roundRect">` | `<rect rx="..." ry="..."/>` |
| 다중 서식 텍스트 | `<hp:drawText>` 내 `<hp:p><hp:run charPrIDRef="...">` | `<wps:txbx><w:txbxContent>` / `<p:txBody>` | `<text><tspan>` / 글리프 서브셋 |
| 연결 화살표 | `<hp:connectLine type="STRAIGHT\|STROKE_ARROW">` (`headStyle`/`tailStyle`) | `<p:cxnSp>` (`<a:ln><a:tailEnd type="triangle"/>`) | `<line>` + `<polygon>` 마커 |
| 내장 차트 노드 | `<hp:chart>` (`Chart/chart_XX.xml`) + 폴백 PNG | `<c:chart r:id="..."/>` + `embeddings/*.xlsx` | `gridcraft-chart` 벡터 프리미티브 |
