# ONEOFFICE 마스터 아키텍처 및 엄밀 개발 계획서 (`MASTER_ARCHITECTURE_PLAN.md`)

> **문서 위상**: ONEOFFICE 전체 개발(Sprint 0 ~ Sprint 6)의 단일 진실 공급원(Single Source of Truth)  
> **세부 기술 명세서 링크**:
> - [시맨틱 도식화(`DiagramSpec`) 및 복합 개체 바이트 명세](SPEC_DIAGRAM_AND_COMPOUND_OBJECTS.md)
> - [크로스플랫폼 PDF 뷰어 & 비파괴 서명/직인 엔진 명세](SPEC_PDF_VIEWER_AND_SIGNATURE.md)
> - [통합 `oneoffice-mcp` 서버 및 AI 자가 검증 프로토콜 명세](SPEC_MCP_AND_AGENT_PROTOCOL.md)
> - [6단계 실측 QA 검증 현황판 및 애자일 캡처 체크리스트](QA_VERIFICATION_MATRIX.md)

---

## 1. 현행 코드베이스 정밀 실사 및 리스크 진단 (As-Is Deep Audit)

4개 하위 디렉토리 전수 탐색을 통해 확인한 **재사용 핵심 자산**과 **개발 착수 전 반드시 통제해야 할 구조적 리스크**는 다음과 같습니다.

### 1.1 하위 프로젝트별 실사 결과
| 디렉토리 | 기술 스택 및 규모 | 확보된 핵심 자산 | 발견된 결함 및 구조적 리스크 |
| :--- | :--- | :--- | :--- |
| **`OPEN_EXCELL/`** (`GridCraft` v0.12.1) | Pure Rust 2024 (`1.95`), 12 Crates (`L0`–`L7`), `eframe`/`wgpu` + VSIX | • **513개 엑셀 함수**(`gridcraft-functions`) & Tarjan SCC 증분 계산기(`gridcraft-calc`)<br>• **26종 툴킷 독립 차트 엔진(`gridcraft-chart` → `Vec<Prim>` / SVG / PNG)**<br>• 무손실 `.xlsx`/암호화 복호화, 22개 MCP 도구 | • 루트에 `open-excell-1.0.0.vsix`(`8.3MB`) 빌드 산출물 혼재<br>• 외부 문서(`HWPX`/`DOCX`) 내부 표에서 호출할 경량 브릿지 API 미분리 |
| **`OPEN_WORD/`** (`WordCraft` v0.11.0) | Pure Rust 2024 (`1.95`), 16 Crates (`L0`–`L6`), `vello_cpu`, `craft-pdf` | • `OPEN_EXCELL`과 **동일한 `storytold/craft-libs`(`92d392cf`) 및 `egui` 0.37 포크 공유**<br>• 다단(1~45단)/각주 조판, `harfrust` 한글 셰이핑, `BehindText` 직인, Tagged PDF, SSIM `visual-diff` | • 내장 `ChartSpec`이 8종 기본형에 제한됨(`gridcraft-chart` 26종 통합 필요)<br>• `BehindText` 직인 위 마우스 클릭 시 선택 불가(`GAP-SIG-01`), 구 경로 하드코딩 일부 잔존 |
| **`OPEN_PPTX/`** (`DeckCraft` v0.11.1) | TS VS Code Studio + `deckcraft-web_bg.wasm`(`38MB`) | • 327개 슬라이드·도형 커맨드, 180종 AutoShape, 커넥터 글루, SmartArt 15종 | • **중요**: `deckcraft_upstream/`(Rust 원본)과 `bin/deckcraft-cli`가 `.gitignore`로 빠져 있어 순수 Rust 도식 컴파일러(`oneoffice-diagram`) 자체 확보가 필수적임<br>• `src/deckcraftEditor.ts:636`의 `message.cmdId`(`undefined`) 매핑 버그 및 차트 수정 시 `.xlsx` 증발 결함(`#335`) |
| **`hanul-hwpx/`** (v0.1.0) | TS (`fflate`, `xmldom`) + `rhwp` WASM + `canvaskit` + Rust `tao`/`wry` | • `.hwpx`/`.hwp`(배포용 `ViewText` AES-128 복호화 포함)/`.hml` 지원<br>• 한컴 단축키(`F5`, `M`, `S`, `Alt+L/T`, `F11`), 38종 오프라인 한글 `.woff2` 폰트 | • **이중 엔진 분열**: UI(`rhwp` WASM)와 헤드리스/MCP(`src/core`)가 분리되어 차트·도식 생성 불가<br>• **OWPML/HWP 버그 6건**: (1) `hanul_insert_image` `<hc:img>`·`content.hpf` 누락, (2) `hanul_append_content` `<hp:p>` 표 래핑 누락, (3) `hanul_find_and_replace` raw XML 정규식 오염, (4) `hanul_fill_fields` 형제 `<hp:t>` 증발, (5) `hwpx-parser` 표 셀 텍스트 본문 중복, (6) HWP 압축 `BinData`/필드 미해제 |
| **모노레포 Git & Cargo 구조** | 루트 `.git` + 하위 4개 중첩 `.git` | • `craft-libs` 캐시 내 **`craft-pdf`(`hayro` 뷰어 + `krilla` 라이터, `MIT OR Apache-2.0`)** 확보 확인 | • **치명적 Git 함정**: 하위 4개 폴더 내부에 각각 `.git/`이 남아 있어 그대로 `git add` 시 GitHub에 빈 서브모듈(broken gitlink)로 올라감 → **Sprint 0에서 최우선 정리 필수!** |

---

## 2. 통합 시스템 아키텍처 설계 (Unified Technical Design)

### 2.1 Cargo 워크스페이스 및 크레이트 계층 통합 전략
`OPEN_WORD`와 `OPEN_EXCELL`은 동일한 Rust `1.95` / `edition = "2024"`와 동일한 `[patch.crates-io]`(`storytold/egui` `6d518fc7`, `storytold/winit` `a517b384`, `craft-libs` `92d392cf`)를 사용합니다.  
Cargo 규칙상 `[patch.crates-io]`는 **최상위 루트 워크스페이스(`ONEOFFICE/Cargo.toml`)에만 단 한 번 선언**되어야 하므로 다음과 같이 통합합니다:

```text
ONEOFFICE/
├── Cargo.toml                         # 단일 루트 Workspace (공통 [workspace.dependencies] & [patch.crates-io])
├── crates/                            # 신규 통합 순수 Rust 크레이트 (100% wasm32 + macOS/Win/Linux 호환)
│   ├── oneoffice-bridge/              # L3: gridcraft-calc(513 함수) + gridcraft-chart(26 차트) + wordcraft-fonts(harfrust) 통합 API
│   ├── oneoffice-diagram/             # L3: DiagramSpec v1 파서, 한글 3-Pass 제약 레이아웃 솔버, OWPML/DrawingML/SVG 컴파일러
│   ├── oneoffice-hwpx-core/           # L3: KS X 6101 무손실 ZIP/XML AST, <hp:tbl>+xlsx 사이드카, 외부편집 SHA-256 동기화기
│   ├── oneoffice-pdf/                 # L3: craft-pdf(hayro 뷰어/검색 + krilla/증분 업데이트 서명·직인·배경투명화 엔진)
│   ├── oneoffice-ui/                  # L6: 공통 egui 글래스 위젯 (인라인 수식바, 차트 드로어, PDF 뷰어 & 서명 패드)
│   └── oneoffice-mcp/                 # L5: 5대 포맷 통합 JSON-RPC 2.0 MCP 서버 (Headless + Live TCP GUI 브릿지)
├── apps/
│   └── oneoffice-desktop/             # L7: macOS · Windows · Linux 통합 네이티브 데스크톱 앱
└── extensions/
    └── oneoffice-vscode/              # 올인원 VS Code / Antigravity VSIX (5대 포맷 오프라인 WASM 커스텀 에디터)
```

### 2.2 3대 실행 표면(VS Code · 3-OS Native · MCP) 통합 구동 방식

1. **VS Code 올인원 익스텐션 (`extensions/oneoffice-vscode`)**:
   - 모든 에디터 탭이 Webview 환경이므로 **100% WebAssembly(`wasm32-unknown-unknown`)** 로 통일 구동됩니다:
     - `.hwpx`/`.hwp`: `rhwp` Studio WASM + `oneoffice-bridge/diagram/hwpx-core` WASM 결합(인플레이스 수식바·차트·PPT 도식 서브캔버스 제공)
     - `.docx`/`.doc`: `wordcraft-web` WASM + `oneoffice-bridge/diagram`
     - `.xlsx`/`.csv`: `gridcraft-web` WASM
     - `.pptx`: `deckcraft-web` WASM + `oneoffice-diagram/bridge`
     - `.pdf`: `oneoffice-pdf` WASM 뷰어 + 자필/도장 날인 오버레이
2. **크로스플랫폼 네이티브 데스크톱 (`apps/oneoffice-desktop` — macOS · Windows · Linux)**:
   - **2단계 실용 통합 전략**:
     - **Stage A (Sprint 2~6 즉시 상용 품질 달성)**: `DOCX`·`XLSX`·`PDF 뷰어/서명`·`도식 캔버스`는 초고속 순수 Rust GPU(`eframe` + `wgpu`: Metal / DX12 / Vulkan)로 단일 프로세스 구동하고, `HWPX/HWP` 탭은 임베디드 오프라인 스튜디오 뷰포트(`wry` / 로컬 루프백 브릿지)와 공유 세션으로 끊김 없이 연결하여 3대 OS 모두에서 한컴 100% 호환 편집을 즉시 보장합니다.
     - **Stage B (장기 순수 Rust 수렴)**: `oneoffice-hwpx-core`의 OWPML AST를 `wordcraft-layout` + `vello_cpu` 조판 파이프라인에 직접 매핑하여 WebView 의존성마저 제거한 100% 단일 GPU 네이티브 렌더링으로 수렴합니다.
3. **통합 MCP 서버 (`crates/oneoffice-mcp`)**:
   - 단일 실행 파일로 헤드리스 배치 처리와 실행 중인 데스크톱/VS Code 실시간 공동 편집(`--connect <port>`)을 모두 지원합니다.

---

## 3. 엄밀한 단계별 개발 계획 (Sprint 0 ~ Sprint 6 WBS)

> **실행 규율**: 사용자 지시로 개발이 개시되면 각 스프린트는 **Wave 병렬 서브에이전트 실행 → 통합 빌드 → `09-qa-auditor`의 6단계 실측 게이트 통과 → 캡처 아티팩트 커밋** 순서로만 다음 스프린트로 진입합니다.

---

## 3. 고속 병렬 애자일 개발 계획 (Concurrent Multi-Wave Sprint Plan)

> **병렬 실행 원칙 (Zero-Collision Concurrency)**:  
> ONEOFFICE는 순차적 개발 지연을 완전히 제거하기 위해 **쓰기 경로(Write Scope)가 완전히 격리된 트랙별로 서브에이전트들을 동시(Parallel)에 가동**합니다. 각 트랙은 독립 브랜치/작업트리(`Workspace: "share"`)에서 작업하며, 로컬 컴파일과 단위 테스트가 통과하는 즉시 통합됩니다.

```mermaid
gantt
    title ONEOFFICE 고속 병렬 애자일 스프린트 타임라인
    dateFormat  X
    axisFormat  Day %d
    
    section Wave 0 (Setup)
    Submodule 등록 및 통합 루트 Cargo 워크스페이스 (08, 02) :milestone, m0, 0, 0d
    
    section Wave 1 (4-Track Core 병렬)
    Track A: 엑셀수식·26종차트 브릿지 (02-rust-core)        :active, tA, 0, 4d
    Track B: 제안서 도식화 DSL & 3-Pass 솔버 (03-diagram)   :active, tB, 0, 4d
    Track C: HWPX OWPML 버그 6건 & hwpx-core (04-hwpx)     :active, tC, 0, 4d
    Track D: PDF 뷰어 & 직인 배경투명화 날인 (05-pdf)      :active, tD, 0, 4d
    
    section Wave 2 (2-Track Surface 병렬)
    Track E: 인플레이스 수식 HUD & 도식 서브캔버스 (06-studio) :tE, 4, 7d
    Track F: 통합 oneoffice-mcp & 자율 검증 (07-mcp)        :tF, 4, 7d
    
    section Wave 3 (Packaging)
    Track G: 올인원 VSIX & 3-OS 데스크톱 패키징 (08-release)  :tG, 7, 9d
    
    section Continuous QA Gate
    09-adversarial-qa 6단계 실측 검증 & 스크린샷 캡처         :crit, qGate, 1, 9d
```

---

### 🔹 Wave 0: Git Submodule 원격 연동 정규화 & 공통 환경 셋업 (Day 0)
> **목표**: 4개 전문 엔진 원격 저장소를 상위 저장소의 Git Submodule로 공식 등록·고정하고 단일 루트 빌드 환경을 구축한다.

| Task ID | 담당 에이전트 | 세부 구현 과업 | 산출물 / 대상 경로 | 완료 기준 (Exit Gate) |
| :--- | :---: | :--- | :--- | :--- |
| **S0.1** | `08-release-governor` | **Git Submodule 공식 등록**: `.gitmodules` 파일 작성 및 `hanul-hwpx`, `OPEN_WORD`, `OPEN_EXCELL`, `OPEN_PPTX` 4개 하위 저장소를 각각 `https://github.com/iamywl/<repo>.git` 공식 서브모듈로 바인딩하여 커밋 포인터 고정 | `.gitmodules`, 루트 Git 트리 | `git submodule status`에서 4개 서브모듈의 독립 HEAD 포인터가 정상 등록되고 클론 재현성 검증 |
| **S0.2** | `02-rust-core` | 루트 `Cargo.toml` 통합 워크스페이스 구성 (`[patch.crates-io]` 단일화, `unsafe_code = "forbid"` 및 `clippy` 금지 린트 전역 설정) | `/Cargo.toml`, `crates/*` | `cargo check --workspace` 및 `wasm32-unknown-unknown` 체크 0 에러 통과 |
| **S0.3** | `09-qa-auditor` | 통합 검증 러너 스켈레톤(`scripts/qa/run-all-gates.mjs`) 구축 (Rust + TS 테스트 일괄 실행 및 리포트 출력) | `scripts/qa/run-all-gates.mjs` | 기존 4개 엔진 기본 단위 테스트 일괄 통과 확인 |

---

### 🔹 Wave 1: 4-Track Core 엔진 동시 병렬 스프린트 (Concurrent Core Sprint)
> **4개 서브에이전트가 파일 충돌 없이 100% 동시에 가동됩니다.**

#### 🟢 Track A: 수식·차트·폰트 통합 브릿지 (`02-rust-core-bridge`)
- **과업**: `crates/oneoffice-bridge` 구현: `CalcTableEngine`(513개 함수 평가 + `.xlsx` 직렬화), `UnifiedChartEngine`(26종 차트 → SVG/PNG/OOXML `c:chartSpace`), `KoreanTextMeasurer`(`harfrust` 줄바꿈 폭/높이 실측) 및 WASM 바인딩.
- **연계**: `OPEN_WORD` 8종 차트를 26종으로 확장 및 `BehindText` 직인 클릭 개선(`GAP-SIG-01`).
- **게이트**: 513개 함수 및 26종 차트 단위 테스트 100% Pass, WASM 컴파일 통과.

#### 🟢 Track B: 제안서 시맨틱 도식화 엔진 (`03-diagram-layout-architect`)
- **과업**: `crates/oneoffice-diagram` 구현: `DiagramSpec` v1 파서, `harfrust` 한글 금칙처리 기반 **3-Pass 제약 레이아웃 솔버** (`ComparisonCard`, `KpiCallout`, `EvidenceList`, `HeroContainer`, `ProcessStepChain`, `GluedConnector`), 실무 제안서 10종 템플릿 및 OWPML/DrawingML/SVG 3중 컴파일러.
- **게이트**: 레퍼런스 제안서 원페이저 벤치마크에서 `overflow_count == 0`, `collision_count == 0` 달성.

#### 🟢 Track C: HWPX 복합개체 & 기존 버그 6건 박멸 (`04-hwpx-compound-engineer`)
- **과업**: `hanul-hwpx` 핵심 버그 6건 수정 (`content.hpf` 누락, `<hp:p>` 표 래핑, DOM 정규식 XML 오염, 누름틀 형제 텍스트 소실, 표 텍스트 중복, HWP 압축 해제 누락) + `crates/oneoffice-hwpx-core` 구현 (`<hp:tbl>` 수식 사이드카, `<hp:chart>` 26종 차트, `<hp:container>` 도식 무손실 직렬화 및 SHA-256 외부편집 동기화기).
- **게이트**: 신규 회귀 테스트 6종 100% 통과 + 복합 `.hwpx` 재열기 무손실 검증.

#### 🟢 Track D: 순수 Rust PDF 뷰어 & 비파괴 서명/직인 엔진 (`05-pdf-viewer-signature-engineer`)
- **과업**: `crates/oneoffice-pdf` 구현 (`craft-pdf`의 `hayro` + `krilla` 기반): 고속 페이지/타일 렌더러, 단어 바운딩박스(`TextSpan`) 추출 및 앵커(`"(인)"`) 검색기, 3차 베지어 자필 서명 피팅, **흰 배경 도장 자동 투명화(`ChromaKeyAlphaMatting`)**, `q ... Q` 상태 격리 비파괴 증분 PDF 저장기.
- **게이트**: 날인 저장 후 원본 텍스트 추출 100% 보존, 북마크 유지, 투명 알파 검은 박스 아티팩트 0건 실측.

---

### 🔹 Wave 2: 2-Track UI & AI 프로토콜 동시 병렬 스프린트 (Surface & MCP Sprint)
> **Wave 1 엔진 산출물 위에서 UI와 AI 에이전트 인터페이스를 100% 동시에 병렬 개발합니다.**

#### 🟣 Track E: 인플레이스 통합 스튜디오 UX (`06-studio-ux-craftsman`)
- **과업**:
  1. **Inline Excel Formula HUD**: HWPX/DOCX 표 진입 시 한컴 단축키(`F5`, `M`, `S`)와 공존하는 플로팅 수식바(`fx =SUM(...)`), 천단위/퍼센트 포맷, 1-클릭 차트 삽입 액션 연동.
  2. **Inline PPT Diagram Sub-Canvas & Mini Sheet**: 도식 더블클릭 시 스마트 가이드·커넥터 스냅·원클릭 테마 스왑이 동작하는 인라인 서브캔버스 (DOM 한글 IME 브릿지 적용).
  3. **PDF Studio UI**: 페이지 썸네일/목차 사이드바, 줌/검색 HUD, 자필 서명 드로잉 패드, 도장 배경 투명화 실시간 토글 프리뷰, 드래그/리사이즈/회전 조작.
- **게이트**: 한글 IME 자모 분리 0건, `Esc` 복귀 시 선택 트랩 0건, 라이트/다크 양방향 WCAG 대비 통과.

#### 🟣 Track F: 통합 `oneoffice-mcp` 서버 & AI 자율 제안서 루프 (`07-mcp-agent-architect`)
- **과업**:
  1. `crates/oneoffice-mcp` 구축: `doc.*`, `diagram.{list_templates,compose,patch}`, `table.upsert`, `chart.upsert`, `pdf.{inspect,find_anchors,sign}`, **`visual.verify_page`** (Headless 샌드박스 + `--connect` 실시간 GUI 연동).
  2. **Closed-Loop Proposal Autopilot 벤치마크**: AI 에이전트가 원시 마크다운 + CSV 입력만으로 `.hwpx` 제안서 생성 → `visual.verify_page` 무결점 확인 → `.pdf` 변환 → `"(인)"` 앵커 자동 탐색 투명 직인 날인까지 단일 호출로 완주.
- **게이트**: MCP 도구 스키마 100% 통과, 자율 제안서 생성 시 레이아웃 오버플로우 0건, 수식 오차 0.

---

### 🔹 Wave 3: 올인원 VS Code 익스텐션 & 3대 OS 네이티브 릴리즈 (`08-release-governor`)
> **단일 확장 및 데스크톱 패키지로 최종 사용자 배포 환경을 완성합니다.**

- **과업**:
  1. `extensions/oneoffice-vscode`: `.hwpx`, `.hwp`, `.docx`, `.xlsx`, `.pptx`, `.pdf` 5대 커스텀 에디터를 단일 VSIX로 번들링, 100% 오프라인 CSP 검증.
  2. `apps/oneoffice-desktop` & 3대 OS 패키징: macOS Universal `.app`/`.dmg` (`codesign` + `lsregister`), Windows `x64`/`ARM64` `.exe`/`.msi`, Linux `AppImage`/`.deb`/`Flatpak` 및 GitHub Actions 크로스플랫폼 매트릭스 CI 완성.
- **게이트**: `@vscode/test-electron` E2E 통과, 3대 OS CI 빌드 그린 통과, `/Applications/OneOffice.app` 실기 윈도우 캡처 증명 완료.

---

### 🛡️ 상시 Continuous Track: 무신뢰 6단계 실측 검증 (`09-adversarial-qa-auditor`)
> 개발 서브에이전트의 자체 보고를 인정하지 않고, 독립적인 6단계 실측 게이트(컴파일/패닉 제로 → 무손실 라운드트립 → BBox 오버플로우 제로 → 듀얼 런타임 E2E → 라이트/다크 WCAG 명도 대비 캡처 → AI MCP 자율 완주)를 수행하여 통과된 항목만 `QA_VERIFICATION_MATRIX.md`에 승격 기록합니다.

---

## 4. 핵심 엔지니어링 리스크 등록부 및 대응책 (Risk Mitigation Matrix)

| 리스크 항목 | 발생 가능 원인 | 선제적 기술 대응책 (Mitigation) |
| :--- | :--- | :--- |
| **R1. 한글 줄바꿈 오차로 인한 도식 텍스트 넘침** | OS별 기본 폰트 메트릭 차이로 인해 macOS에서는 맞던 박스가 Windows/Linux에서 넘침 | 오프라인 오픈 라이선스 기준 폰트(`Pretendard` / `Noto Sans KR`) 메트릭을 `harfrust`로 직접 실측하고 카드 패딩 안전 마진(`+4pt`) 및 `visual.verify_page` BBox 감사를 이중 적용 |
| **R2. 외부 한컴오피스 편집 시 수식/도식 불일치** | 한컴오피스는 `section0.xml`만 수정하고 `oneoffice/` 사이드카를 갱신하지 않음 | `manifest.json`의 `rendered_cells_sha256` 해시 비교로 외부 수정을 즉시 감지해 변경된 리터럴 셀을 수식 모델에 역반영 후 자동 재계산 |
| **R3. 기형 PDF 파일에서 서명 위치 틀어짐** | 스캔/은행 PDF의 페이지 스트림 끝에 미복원 변환 행렬(`cm`) 잔존 | 증분 업데이트 시 원본 `/Contents`를 반드시 `q ... Q` 그래픽 상태 스택으로 격리한 뒤 정규화된 페이지 좌표계에서 서명 XObject 렌더링 |
| **R4. WASM `<canvas>` 상의 한국어 IME 자모 분리** | 브라우저 Canvas API가 조합 중인 한글(`compositionupdate`) 이벤트를 직접 받지 못함 | 도식·슬라이드·차트 인라인 텍스트 편집 시 투명 DOM `<textarea>` / `contentEditable` IME 프록시 브릿지를 결합하여 자모 분리 원천 차단 |
| **R5. 병렬 서브에이전트 작업 간 파일 충돌** | 복수 에이전트가 동일 설정 파일이나 크레이트를 동시 수정 | `AGENTS.md`의 9대 에이전트 독점 쓰기 경로(Write-Scope) 강제 + 동일 저장소 병렬 작업 시 `Workspace: "share"`(독립 워크트리) 격리 후 머지 |
