# Contributing to ONEOFFICE (기여 가이드)

ONEOFFICE에 관심을 가져주셔서 감사합니다!  
우리는 **한글(`HWPX`/`HWP`) · 워드(`DOCX`) · 엑셀(`XLSX`) · 파워포인트(`PPTX`) · PDF(뷰어 및 서명/직인)** 를 하나의 네이티브 앱(`macOS` / `Windows` / `Linux`), 하나의 VS Code 익스텐션(`.vsix`), 그리고 하나의 AI 에이전트 프로토콜(`oneoffice-mcp`)로 통합하는 오픈소스 프로젝트입니다.

작은 버그 제보, 제안서 도식 템플릿(`DiagramSpec`) 1개 추가, 번역 개선, 엑셀 함수/차트 보강, PDF 서명 UX 개선 모두 환영합니다!

---

## 1. 5분 만에 개발 환경 준비하기 (Prerequisites & Submodules)

ONEOFFICE는 개별 전문 엔진의 독립적인 원격 저장소(`hanul-hwpx`, `OPEN_WORD`, `OPEN_EXCELL`, `OPEN_PPTX`)를 **Git Submodule**로 관리합니다.

### 저장소 클론 및 서브모듈 초기화
```bash
# 1. 서브모듈까지 재귀적으로 한 번에 클론
git clone --recurse-submodules https://github.com/iamywl/ONEOFFICE.git
cd ONEOFFICE

# 이미 일반 클론을 받은 경우 서브모듈 동기화
git submodule update --init --recursive
```

### 서브모듈 기여 워크플로우 (Submodule Workflow)
1. **특정 엔진만 수정할 때**: 해당 폴더(`hanul-hwpx/`, `OPEN_WORD/` 등)로 이동하여 브랜치를 생성하고 커밋한 뒤, 해당 엔진의 독립 원격 저장소로 PR을 보내거나 직접 푸시합니다.
2. **상위 ONEOFFICE에서 통합할 때**:
   ```bash
   git submodule update --remote <submodule_name> # 최신 커밋 반영
   git add <submodule_name>
   git commit -m "chore: bump <submodule_name> pointer"
   ```

모듈별로 독립 빌드가 가능하므로, 기여하려는 영역의 도구만 있어도 바로 시작할 수 있습니다:

| 도구 | 권장 버전 | 필요한 영역 | 설치 확인 |
| :--- | :--- | :--- | :--- |
| **Rust Toolchain** | `1.95+` (`edition = "2024"`) | `OPEN_WORD`, `OPEN_EXCELL`, `crates/*`, `apps/*` | `rustc --version` |
| **WASM Target** | `wasm32-unknown-unknown` | 웹뷰 / VS Code 확장 WASM 빌드 | `rustup target add wasm32-unknown-unknown` |
| **Node.js & npm** | `Node 20+` | `hanul-hwpx`, `OPEN_PPTX`, `extensions/oneoffice-vscode` | `node -v && npm -v` |

---

## 2. 영역별 빠른 빌드 & 테스트 명령어 (Quick Start by Module)

전체 모노레포를 한 번에 빌드할 필요 없이, 수정한 디렉토리에서 즉시 검증할 수 있습니다:

### A. 한글 HWPX / HWP 코어 & 스튜디오 (`hanul-hwpx/`)
```bash
cd hanul-hwpx
npm install
npm run check          # TypeScript 타입 검증
npm run build          # 코어, CLI, MCP, VS Code 확장 번들링
npm test               # 파서, 스토리지, 인터랙션, MCP 단위 테스트
npm run smoke          # 오프라인 헤드리스 E2E 및 스크린샷 검증
```

### B. 엑셀 수식(513개) & 26종 차트 엔진 (`OPEN_EXCELL/`)
```bash
cd OPEN_EXCELL
cargo test --workspace # 수식 평가기, 차트 프리미티브, XLSX 라운드트립 테스트
cargo xtask ci         # fmt + clippy + test + assets + layers + wasm 통합 게이트
```

### C. 워드 조판 & 벡터 PDF 출력 엔진 (`OPEN_WORD/`)
```bash
cd OPEN_WORD
cargo test --workspace # 다단 레이아웃, 직인 레이어링, DOCX/PDF 테스트
cargo xtask ci         # 전체 품질 게이트 실행
```

### D. 프레젠테이션 & 도형 스튜디오 (`OPEN_PPTX/`)
```bash
cd OPEN_PPTX
npm install
npm run compile        # VS Code 확장 및 웹뷰 컴파일
npm run parity:check   # 기능/레이아웃 패리티 드리프트 점검
```

---

## 3. 처음 기여하기 좋은 주제 (Good First Contributions)

1. **제안서 시맨틱 도식 템플릿 추가 (`crates/oneoffice-diagram/`)**:
   - 사업계획서·제안서에 자주 쓰이는 도식(예: `SWOT 2x2`, `추진일정 간트 차트`, `수행조직도`, `As-Is → To-Be 브릿지`, `시장규모 TAM-SAM-SOM`)을 `DiagramSpec` 프리셋으로 추가하기.
2. **HWPX OWPML 스키마 및 복합 개체 개선 (`hanul-hwpx/src/core/`, `crates/oneoffice-hwpx-core/`)**:
   - `<hp:tbl>`, `<hp:chart>`, `<hp:container>` 직렬화 정밀도 향상 및 실제 공공기관 공고문 픽스처 호환성 보강.
3. **PDF 뷰어 & 전자서명/직인 편의 기능 (`crates/oneoffice-pdf/`)**:
   - 도장 이미지 배경 투명화(Chroma-Key) 감도 개선, 날짜/성명 스탬프 프리셋, 텍스트 앵커(`(인)`) 탐색 정확도 개선.
4. **한국어/다국어 로컬라이제이션 & 단축키 에르고노믹스**:
   - 리본·플로팅 HUD 다국어 번역 보강 및 OS별(`macOS` `Cmd` / `Win·Linux` `Ctrl/Alt`) 단축키 매핑 개선.
5. **MCP 에이전트 프롬프트 & 레시피 추가 (`.agents/skills/`)**:
   - AI 에이전트가 제안서 초안에서 표·차트·도식을 생성하는 자동화 레시피 고도화.

---

## 4. AI 서브에이전트와 함께 빠르게 기여하기

ONEOFFICE는 `.agents/` 디렉토리에 **9개의 전문 서브에이전트 정의**와 **3개의 실행 스킬**을 기본 내장하고 있습니다 (`AGENTS.md` 참조).
- 구현 전 설계가 필요할 때: `01-chief-planner.md` 지침에 따라 인터페이스 스키마부터 확인하세요.
- 병렬 구현 시: `.agents/skills/parallel-subagent-sprint/SKILL.md`의 **쓰기 경로 격리 규칙(Write-Scope Isolation)**을 지키면 충돌 없이 개발할 수 있습니다.
- PR 제출 전 자가 감사: `.agents/skills/ground-truth-qa-gate/SKILL.md` 절차에 따라 테스트 및 캡처를 수행하세요.

---

## 5. 절대 수호 품질 및 라이선스 규칙 (Non-Negotiable Rules)

모든 PR은 아래 5가지 원칙을 반드시 통과해야 머지됩니다:

1. **Never Crash (패닉 제로)**:
   - Rust 프로덕션 코드에서 `unsafe`, `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()` 사용을 금지합니다(`Result<T, E>`로 안전하게 반환).
2. **클린룸 & 외부 독점/오염 라이선스 자산 유입 금지**:
   - MS Office, 한컴오피스, Adobe 등의 아이콘·폰트·템플릿·코드를 절대 복사하거나 번들링하지 마세요.
   - `GPL` / `LGPL` / `AGPL` 라이선스 코드 및 라이브러리(예: LibreOffice, MuPDF, Poppler) 의존성 추가를 엄격히 금지합니다 (`MIT` 또는 `Apache-2.0` 호환 크레이트만 허용).
   - 새로운 이미지, SVG, 샘플 문서, 아이콘 파일을 커밋할 때는 **반드시 같은 커밋에서 [`ATTRIBUTION.md`](ATTRIBUTION.md)에 저작자·출처·라이선스를 기록**해야 합니다.
3. **실측 스크린샷 증명 (UI / 렌더링 변경 시 필수)**:
   - UI, 도식화 레이아웃, 차트, PDF 서명 관련 변경 PR에는 **라이트 모드 및 다크 모드 실측 캡처 스크린샷**(및 WCAG 명도 대비 `≥ 4.5:1` 확인)을 첨부해 주세요.
4. **3대 OS 및 WASM 호환성 유지**:
   - 특정 OS 전용 API(예: Win32 COM/DLL)에 의존하지 않고 `macOS`, `Windows`, `Linux`, 그리고 VS Code `wasm32-unknown-unknown` 타겟 빌드가 깨지지 않아야 합니다.
5. **듀얼 라이선스 동의**:
   - 제출하시는 모든 기여물은 프로젝트 표준인 **`MIT OR Apache-2.0`** 듀얼 라이선스 하에 배포됨에 동의하는 것으로 간주됩니다.
