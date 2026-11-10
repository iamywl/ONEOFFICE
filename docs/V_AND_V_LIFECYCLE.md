# ONEOFFICE — V&V 라이프사이클 프레임워크 (Verification & Validation)

> **관리 책임**: `09-adversarial-qa-auditor` (독립 감사) & `01-chief-planner` (추적성 거버넌스)  
> **표준 근거**: IEEE Std 1012-2016 (System, Software, and Hardware V&V), ISO/IEC/IEEE 29119, ISO 32000-1 (PDF), ECMA-376 (OOXML), KS X 6101 (OWPML)

---

## 1. 개요 및 V&V 철학

ONEOFFICE는 복합 문서(한글 HWPX, 워드 DOCX, 엑셀 XLSX, 파워포인트 PPTX, PDF)와 3대 런타임(VS Code 확장, 크로스플랫폼 데스크톱, AI Agent MCP)이 유기적으로 결합된 복합 시스템입니다.

시스템의 결함을 원천 차단하고 사용자의 실무 요구를 완벽히 충족하기 위해 **Verification(검증)**과 **Validation(확인)**을 명확히 분리하고 상호 대응시키는 **V-Model V&V 라이프사이클**을 강제합니다:

- **Verification (검증 — "Are we building the product right?")**:
  - 소프트웨어가 정의된 기술 규격(ISO/ECMA/KS), 아키텍처 스키마(`DiagramSpec`, `LiveTableSidecar`), 메모리 안전성 규칙(`unsafe_code = "forbid"`), 수학적 오라클 및 바이트 무손실 보존률을 정확히 만족하는지 증명합니다.
- **Validation (확인 — "Are we building the right product?")**:
  - 소프트웨어가 실제 사용자의 업무 시나리오(DOCX 내 표-차트 실시간 연동, PPT 구조도의 워드 인플레이스 작업, PDF 흰 배경 도장 자동 투명화 날인, 한글 Tofu 없는 가독성)를 온전히 달성하는지 실측 증명합니다.

---

## 2. ONEOFFICE 4-Level V&V 수명주기 매핑 (V-Model)

```mermaid
flowchart TD
    subgraph Design_Verification["개발 및 설계 검증 (Verification)"]
        REQ["Level 1: 사용자 업무 요구사항\n(제안서·계약서·공문서 단절 해소)"]
        ARCH["Level 2: 복합 개체 & 아키텍처 규격\n(KS X 6101, ECMA-376, ISO 32000-1)"]
        COMP["Level 3: 시맨틱 DSL & 수식/차트 설계\n(DiagramSpec, 513 함수, 26 차트)"]
        CODE["Level 4: 순수 Rust 클린룸 구현\n(unsafe forbid, zero unwrap/panic)"]
    end

    subgraph User_Validation["실측 확인 및 인수 (Validation)"]
        VAL_E2E["Level 1 확인: 실전 사용자 시나리오 Validation\n(DOCX 표-차트 연동, PPT 도식 인라인, PDF 날인)"]
        VAL_INT["Level 2 확인: 5대 포맷 상호운용성 & 라운드트립\n(100% 무손실 바이트 역직렬화 & DrawingML)"]
        VAL_COMP["Level 3 확인: 기하 불변식 & 오라클 일치성\n(overflow == 0, 513 수식 부동소수점 오차 0)"]
        VAL_UNIT["Level 4 확인: 정적 안전성 & 3대 OS 무결성\n(WASM 타겟 체크, Zero-Tofu, WCAG AAA)"]
    end

    REQ <==== "요구사항 추적성 (Traceability)" ====> VAL_E2E
    ARCH <==== "규격 적합성 (Compliance)" ====> VAL_INT
    COMP <==== "불변식 증명 (Invariants)" ====> VAL_COMP
    CODE <==== "정적 분석 및 컴파일" ====> VAL_UNIT

    REQ --> ARCH --> COMP --> CODE
    CODE --> VAL_UNIT --> VAL_COMP --> VAL_INT --> VAL_E2E
```

---

## 3. 계층별 V&V 세부 활동 명세

### Level 1. 사용자 업무 시나리오 (User Requirements & System Validation)
- **Verification 활동**:
  - `docs/MASTER_ARCHITECTURE_PLAN.md` 및 PRD 요구사항 매트릭스 정합성 감사.
  - 사용자 스토리별 기능 갭(GAP-ID) 누락 검사.
- **Validation 활동 (실무 5대 골든 시나리오 실측)**:
  1. **[VAL-SCN-01] DOCX 표-차트 인플레이스 연동 시나리오**:
     - 워드 문서 내 표의 수치를 수정했을 때 연동된 26종 벡터 차트가 즉시 자동 갱신되는지 확인.
  2. **[VAL-SCN-02] PPT 구조도 DOCX 인플레이스 작업 시나리오**:
     - PPT급 비교 카드, KPI 콜아웃, 화살표 구조도가 A4 용지 폭(`425.2pt`)에 자동 맞춤 조판되고 텍스트 박스/화살표 넘침이 없는지 확인.
  3. **[VAL-SCN-03] PDF 크로마키 전자직인 날인 시나리오**:
     - 흰 배경 도장 사진을 불러와 자동 투명화(Chroma-Key) 날인 후, 원본 텍스트/하이퍼링크가 비파괴 보존되는지 확인.
  4. **[VAL-SCN-04] 공문서 HWPX 엑셀 수식표 시나리오**:
     - HWPX 문서 내부 표에서 엑셀 수식(`=SUM(D2:D3)`)이 자동 계산되고 한컴오피스에서 정상 열람되는지 확인.
  5. **[VAL-SCN-05] AI 에이전트 폐루프 자율 작성 시나리오**:
     - `oneoffice-mcp`가 제안서 도식을 생성하고 `visual.verify_page`로 자가 검증을 완주하는지 확인.

### Level 2. 아키텍처 및 복합 규격 (Architecture & Subsystem Integration V&V)
- **Verification 활동**:
  - ISO 32000-1 (PDF 구조 스트림 무결성), ECMA-376 (OOXML DrawingML/SpreadsheetML), KS X 6101 (OWPML) 매직 바이트 및 XML 파트 검증.
- **Validation 활동**:
  - 5대 포맷(`PDF`, `HWPX`, `XLSX`, `DOCX`, `PPTX`) 더미 문서의 바이트 역직렬화 무손실 라운드트립(`wordcraft_docx::read`, `HwpxPackage::from_bytes`, `PdfDocumentSession::load`) 100% 보존 확인.
  - 외부 오피스(MS Word, 한컴오피스)에서 편집 후 재열람 시 충돌 감지(`rendered_cells_sha256`) 및 수식 자동 동기화 확인.

### Level 3. 컴포넌트 및 알고리즘 (Component Verification & Invariant Proof)
- **Verification 활동**:
  - `gridcraft-functions` 513개 엑셀 수식 레지스트리 전수 적재 검증.
  - `UnifiedChartEngine` 26종 벡터 차트 및 6종 추세선의 SVG, PNG, DrawingML(`<c:chartSpace>`) 생성 검증.
  - HarfRust 한글 글리프 폭·높이 3-Pass 실측 레이아웃 솔버 수렴성 검증.
- **Validation 활동**:
  - **기하 불변식 (Geometric Invariants)**: 10종 제안서 도식 템플릿 레이아웃 시 `overflow_count == 0`, `collision_count == 0` 필수 성립 확인.
  - **수식 오라클 일치성**: 엑셀 원본 계산 결과와 부동소수점 오차 `1e-9` 이내 일치 확인.

### Level 4. 코드 안전성 및 단위 검증 (Code Unit Verification & Static Analysis)
- **Verification 활동**:
  - `#![deny(unsafe_code)]` 및 `#![deny(clippy::unwrap_used, clippy::panic)]` 강제.
  - `cargo check --target wasm32-unknown-unknown` 오프라인 웹뷰 컴파일 검증.
  - `hanul-hwpx` TypeScript 타입 체크 (`tsc --noEmit`).
- **Validation 활동**:
  - **Zero-Tofu 보증**: 시스템 한글 폰트(`Pretendard`, `Apple SD Gothic Neo`, `Malgun Gothic`) 의무 바인딩으로 UI 화면의 CJK `ㅁ` 결함 0건 실측 확인.
  - **WCAG 테마 가독성**: 용지 백색 보존 및 글래스모피즘 모달 명도 대비 `≥ 7.0:1`(AAA) 실측 확인.

---

## 4. V&V 요구사항 추적 매트릭스 (V&V Traceability Matrix - RTM)

| 요구사항 ID | 기능 명세 | 검증 활동 (Verification) | 확인 활동 (Validation) | 통과 기준 (Pass Criteria) | 상태 |
| :---: | :--- | :--- | :--- | :--- | :---: |
| **REQ-01** | HWPX OWPML 호환성 | OWPML XML 파서 단위 테스트 | 실전 공문서 37종 무손실 파싱 | XML 스키마 적합 & 렌더링 결함 0건 | `PASS` |
| **REQ-02** | 513개 엑셀 수식 | 함수 레지스트리 적재 검사 | 제안 예산안 수식 자동 연산 확인 | FIPS SHA-256 오라클 100% 일치 | `PASS` |
| **REQ-03** | 26종 벡터 차트 | SVG/PNG/OOXML 포맷 생성 검사 | HWPX/DOCX 내 차트 렌더링 확인 | 26종 전 차트 한글 폰트 렌더링 | `PASS` |
| **REQ-04** | 제안서 도식화 DSL | DiagramSpec JSON 스키마 검증 | A4 본문 폭 자동 조판 확인 | `overflow_count == 0` | `PASS` |
| **REQ-05** | 순수 Rust PDF 뷰어 | ISO 32000-1 스트림 검사 | 고해상도 PDF 열람 및 검색 확인 | 텍스트 선명도 & 키워드 매칭 100% | `PASS` |
| **REQ-06** | 크로마키 전자직인 | 투명 알파 채널 매팅 연산 검사 | 실제 도장 사진 흰 배경 제거 날인 | 원본 텍스트/링크 비파괴 보존 | `PASS` |
| **REQ-07** | DOCX 복합 개체 | OpenXML DrawingML 패키징 검사 | 워드 내 도식·차트·직인 복원 확인 | `word/media/` 3종 이미지 무손실 | `PASS` |
| **REQ-08** | Zero CJK Tofu | 폰트 패밀리 폴백 체인 검사 | 네이티브 데스크톱 한글 렌더링 확인 | UI 상 `ㅁ` 글리프 0건 | `PASS` |
| **REQ-09** | AI Agent MCP | JSON-RPC 2.0 프로토콜 검증 | 폐루프 자율 작성 및 자가 검증 | 5종 도구 응답 및 시각 검증 완료 | `PASS` |
| **REQ-10** | 문서 품질 하네스 | 마크다운 링크/이미지 자동 감사 | 캡처 기반 6대 기둥 1:1 매칭 확인 | `doc_harness.mjs` 오류 0건 | `PASS` |

---

## 5. V&V 실행 거버넌스 및 자가 승인 금지 원칙

1. **독립 감사관 통제**: V&V 파이프라인(`scripts/qa/v_and_v_runner.mjs`)은 구현 에이전트가 아닌 `09-adversarial-qa-auditor`에 의해서만 실행 및 서명됩니다.
2. **단 하나의 실패도 불허 (Zero-Tolerance)**: 10개 요구사항 중 단 1개라도 Verification 또는 Validation에 실패하면 릴리스가 즉시 차단되며, 원인 분석 리포트가 해당 전문 에이전트로 반송됩니다.
3. **증적 자동 생성**: 모든 V&V 실행 결과는 타임스탬프와 함께 `docs/V_AND_V_REPORT.md`에 영구 기록됩니다.
