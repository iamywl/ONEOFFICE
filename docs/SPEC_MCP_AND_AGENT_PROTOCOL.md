# 통합 MCP 서버 및 AI 에이전트 프로토콜 명세서 (`oneoffice-mcp`)

> **연관 에이전트**: `07-mcp-architect`, `03-diagram-engine`, `04-hwpx-bridge`, `05-pdf-engineer`, `09-qa-auditor`  
> **지원 표준**: Model Context Protocol `2025-06-18` / `2025-03-26` (Stdio & Streamable HTTP) + `2024-11-05` 하위 호환

---

## 1. 아키텍처 개요: 듀얼 백엔드 (Headless & Live GUI Co-Editing)

`oneoffice-mcp`는 단일 바이너리(`oneoffice-mcp` / `oneoffice-cli mcp`)로 두 가지 실행 모드를 지원합니다:

1. **Headless Sandbox 모드 (기본값)**:
   - GUI 창 없이 서버·터미널·CI·클라우드 에이전트 환경에서 인프로세스 Rust 엔진(`oneoffice-bridge`, `oneoffice-diagram`, `oneoffice-hwpx-core`, `oneoffice-pdf`, `wordcraft-engine`)을 직접 구동합니다.
   - `--workspace-root <DIR>` 지정 시 심볼릭 링크 탈출(`canonicalize` 검증) 및 루트 외부 파일 접근을 원천 차단합니다.
2. **Live Studio Connected 모드 (`--connect 127.0.0.1:<PORT>`)**:
   - 사용자가 열어둔 `OneOffice` 네이티브 앱 또는 VS Code 확장의 로컬 루프백 제어 소켓(256-bit OS 난수 토큰 `0600` 파일 인증, 상수 시간 비교)에 연결되어, **AI 에이전트가 수정하는 표·차트·도식·PDF 서명이 사용자 화면에 실시간으로 반영**되고 단일 Undo 스택으로 관리됩니다.

---

## 2. 표준 도구 명세 (Unified Tool Schemas)

### 2.1 문서 열람·생성·변환 (`oneoffice.doc.*`)
| 도구명 | 주요 입력 파라미터 | 반환 데이터 요약 |
| :--- | :--- | :--- |
| `oneoffice.doc.inspect` | `{ path, includeTables?, includeDiagrams?, includeFields? }` | 포맷(`hwpx\|hwp\|docx\|xlsx\|pptx\|pdf`), 페이지/섹션 수, 제목 목차, 표/차트/도식 ID 목록, 누름틀(`fields`) 목록 |
| `oneoffice.doc.create` | `{ outputPath, format, title?, pageSetup? }` | 생성된 빈 문서/템플릿 메타데이터 및 세션 핸들 |
| `oneoffice.doc.convert` | `{ inputPath, outputPath, options?: { pdfTagged?: boolean } }` | `.hwpx` / `.hwp` / `.docx` / `.pptx` / `.xlsx` / `.md` → `.pdf` / `.hwpx` / `.docx` / `.md` 변환 결과 |
| `oneoffice.doc.fill_fields` | `{ path, outputPath?, fields: Record<string, string> }` | `.hwpx`/`.docx` 누름틀 및 양식 셀 치환 건수 (형제 텍스트 훼손 제로 보장) |

### 2.2 제안서 시맨틱 도식화 (`oneoffice.diagram.*`)
| 도구명 | 주요 입력 파라미터 | 동작 및 반환값 |
| :--- | :--- | :--- |
| `oneoffice.diagram.list_templates` | `{ category?: "proposal" \| "process" \| "architecture" \| "roadmap" }` | 15종 표준 도식 템플릿 이름, 권장 용도, 필수 슬롯 스키마 반환 |
| `oneoffice.diagram.compose` | `{ path, outputPath?, target: { section?, afterBlock?, slide? }, spec: DiagramSpec }` | 한글 `harfrust` 3-Pass 레이아웃 계산 후 네이티브 벡터 도식 삽입 + **`layoutReport` (`overflowCount`, `collisionCount`, `finalSizePt`)** 반환 |
| `oneoffice.diagram.patch` | `{ path, diagramId, operations: DiagramPatchOp[] }` | 특정 카드 문구·KPI 수치·단계 노드만 부분 수정 후 자동 높이 균등화 및 화살표 재라우팅 |

### 2.3 엑셀 수식 표 & 26종 연동 차트 (`oneoffice.table.*`, `oneoffice.chart.*`)
| 도구명 | 주요 입력 파라미터 | 동작 및 반환값 |
| :--- | :--- | :--- |
| `oneoffice.table.upsert` | `{ path, tableId?, cells: (string\|number)[][], merges?, colWidthsPt?, numberFormats?, stylePreset? }` | `=`로 시작하는 셀을 `gridcraft-calc`(513개 함수)로 계산하여 `<hp:tbl>`/`<w:tbl>` 표시값과 `.xlsx` 수식 사이드카를 동시 저장하고 계산 결과 배열 반환 |
| `oneoffice.chart.upsert` | `{ path, chartId?, chartKind, title?, sourceTableId? \| series, trendline?, sizePt? }` | 26종 `ChartKind` 중 지정 차트를 생성하여 벡터 XML + `.xlsx` 데이터 + 폴백 이미지를 패키지에 등록 |

### 2.4 PDF 뷰어 인스펙션 & 전자서명 날인 (`oneoffice.pdf.*`)
| 도구명 | 주요 입력 파라미터 | 동작 및 반환값 |
| :--- | :--- | :--- |
| `oneoffice.pdf.inspect` | `{ path, pageRange? }` | 페이지별 크기, 회전, 목차, 추출 텍스트 요약 반환 |
| `oneoffice.pdf.find_anchors` | `{ path, query: "(인)" \| "서명" \| string }` | 일치하는 모든 텍스트의 `{ page, text, bboxPt: [x, y, w, h] }` 좌표 배열 반환 |
| `oneoffice.pdf.sign` | `{ path, outputPath, placement: { page, rectPt } \| { anchorText, offsetPt? }, stamp: { imagePath, removeWhiteBg?: boolean } \| { svgPath } \| { textStamp } }` | 비파괴 증분 업데이트로 서명/직인을 합성 저장하고 날인된 페이지 프리뷰 PNG 경로 및 검증 상태 반환 |

### 2.5 엑셀 513 수식 단독 평가 (`oneoffice.calc.eval`)
| 도구명 | 주요 입력 파라미터 | 동작 및 반환값 |
| :--- | :--- | :--- |
| `oneoffice.calc.eval` | `{ formula: string }` | GridCraft 513개 엑셀 함수 엔진을 통해 임의의 수식(`=SUM(100, 250, 400)`, `=NPV(...)`, `=XLOOKUP(...)`)을 단독 평가하고 결과 문자열 및 엔진 지원 함수 수 반환 |

### 2.6 폐루프 비주얼 자가 검증 (`oneoffice.visual.verify_page`)
AI 에이전트가 문서를 사용자에게 전달하기 전 **반드시 호출해야 하는 품질 게이트 도구**입니다:
- **입력**: `{ path, pageOrSlide: number, dpi?: 144 | 216, checkContrast?: boolean }`
- **출력**:
  ```json
  {
    "renderedPngPath": "/tmp/oneoffice_verify_p1.png",
    "status": "PASS",
    "diagnostics": {
      "textOverflows": [],
      "boxCollisions": [],
      "formulaErrors": [],
      "contrastViolations": [],
      "schemaWarnings": []
    }
  }
  ```
- 만약 `status == "WARN" | "FAIL"`이면 에이전트는 `textOverflows`에 지목된 `nodeId`의 문구를 축약하거나 카드 폭/폰트 크기를 조정(`diagram.patch`)한 뒤 `PASS`가 나올 때까지 자율 교정합니다.
