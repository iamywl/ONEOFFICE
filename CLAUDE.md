# ONEOFFICE — Instructions for Claude Code & AI Agents

> ONEOFFICE is a unified, clean-room, open-source office platform bringing together **HWPX/HWP (`hanul-hwpx`), Word (`OPEN_WORD`), Excel (`OPEN_EXCELL`), PowerPoint (`OPEN_PPTX`), and PDF Viewer/Signer (`oneoffice-pdf`)** into a single Native Desktop app (macOS/Win/Linux), an all-in-one VS Code extension (`.vsix`), and an AI Agent server (`oneoffice-mcp`).

---

## 🚀 "개발진행해" (Start / Continue Autonomous Development)

사용자가 **"개발진행해"**, **"개발 시작해"**, 또는 **"계속 개발해"**라고 입력하면, 묻지 말고 즉시 다음 자율 실행 루프를 시작하세요:

1. **상태 진단 (Status Check)**:
   - [`docs/MASTER_ARCHITECTURE_PLAN.md`](docs/MASTER_ARCHITECTURE_PLAN.md)와 [`docs/QA_VERIFICATION_MATRIX.md`](docs/QA_VERIFICATION_MATRIX.md)를 읽어 현재 미완료(`🟡 Sprint Target` 또는 `🔴 Bug / Blocked`)인 최우선 Task를 확인합니다.
2. **역할 빙의 (Role Assumption)**:
   - 선택된 Task의 담당 서브에이전트([`.agents/agents/`](.agents/agents/))의 권한과 독점 쓰기 영역(Write Scope)을 엄격히 준수합니다. (타 영역 파일 무단 수정 금지)
3. **클린룸 & Never Crash 코딩**:
   - Rust: `unsafe_code = "forbid"`, `unwrap()`/`expect()`/`panic!()`/`todo!()` 절대 금지. `Result<T, E>`로 처리.
   - 외부 자산: MS/한컴/Adobe 독점 에셋 및 GPL/AGPL(MuPDF, Poppler 등) 코드 유입 절대 금지.
4. **셀프 힐링 빌드 & 테스트 (Self-Healing Loop)**:
   - `cargo test` / `npm test` / `cargo check --target wasm32-unknown-unknown` 실행.
   - 컴파일 또는 테스트 실패 시 사용자에게 묻지 않고 오류 로그를 분석하여 **최대 3회까지 자율 수정(Self-Fix)**합니다.
5. **독립 실측 QA & 매트릭스 갱신 (`oneoffice-qa-auditor`)**:
   - [`.agents/skills/ground-truth-qa-gate/SKILL.md`](.agents/skills/ground-truth-qa-gate/SKILL.md) 6단계 게이트 검증.
   - 통과 시 [`docs/QA_VERIFICATION_MATRIX.md`](docs/QA_VERIFICATION_MATRIX.md)에 증거 로그/스크린샷 경로를 기록하고 상태를 `✅ Verified`로 승격합니다.
6. **다음 태스크 자율 연쇄 (Keep Going)**:
   - 한 태스크가 완료되면 멈추지 않고 다음 미완료 태스크로 넘어가 자율 루프를 이어갑니다. (비상 정지 사안인 비밀키/라이선스 변경 시에만 사용자 확인)

자세한 자율 운용 지침은 [`docs/AUTONOMOUS_OPERATING_PROCEDURE.md`](docs/AUTONOMOUS_OPERATING_PROCEDURE.md)를 참조하세요.

---

## 🔍 개발 중 새로운 기능/미문서화 스펙을 발견했을 때 (Feature Discovery)

개발 중 실무 공문서/제안서 파싱이나 레이아웃 처리 과정에서 **사전 명세에 없던 새로운 기능, 미지원 OWPML/OOXML 태그, 엣지케이스 속성**을 발견한 경우:
1. 임의로 무시하거나 프로세스를 크래시내지 말고 **패스스루(Passthrough)**로 데이터를 원본 보존합니다.
2. [`docs/GAPS.md`](docs/GAPS.md)에 신규 `GAP-ID`를 발행하고 기능 설명과 재현 픽스처를 등록합니다.
3. [`.agents/agents/01-chief-planner.md`](.agents/agents/01-chief-planner.md) 지침에 따라 `docs/MASTER_ARCHITECTURE_PLAN.md`에 흡수 일정을 반영합니다.
상세 프로토콜: [`docs/FEATURE_DISCOVERY_PROTOCOL.md`](docs/FEATURE_DISCOVERY_PROTOCOL.md)

---

## 📦 Git Submodule & 2단계 커밋 규약 (2-Step Commit Flow)

4대 하위 엔진(`hanul-hwpx`, `OPEN_WORD`, `OPEN_EXCELL`, `OPEN_PPTX`)은 Git Submodule로 관리됩니다.

### 커밋 메시지 규격 (Conventional Commits + Task ID)
모든 커밋은 반드시 다음 형식을 준수해야 합니다:
```text
<type>(<scope>): [<TASK-ID>] <간결한 한 줄 요약 (50자 이내)>

[상세 본문]
- 변경 이유 및 실측 검증 결과 (Pass 여부)

Refs docs/MASTER_ARCHITECTURE_PLAN.md#TASK-ID
```
- **Type**: `feat`, `fix`, `docs`, `qa`, `test`, `refactor`, `chore`
- **Scope**: `hwpx`, `word`, `excel`, `pptx`, `pdf`, `diagram`, `mcp`, `ui`, `packaging`, `deps`, `root`

### 2단계 커밋 파이프라인
서브모듈 코드를 수정한 경우 상위/하위 저장소 간 이력 분리를 방지하기 위해 반드시 아래 2단계를 순서대로 실행합니다:
1. **1단계 (서브모듈 내부)**:
   ```bash
   cd hanul-hwpx  # 또는 OPEN_EXCELL 등
   git add <수정파일>
   git commit -m "fix(owpml): [GAP-HWPX-01] fix image schema"
   git push origin main
   ```
2. **2단계 (상위 ONEOFFICE 루트)**:
   ```bash
   cd ..
   git add hanul-hwpx docs/QA_VERIFICATION_MATRIX.md
   git commit -m "chore(hwpx): [GAP-HWPX-01] bump submodule pointer with verified QA"
   git push origin main
   ```

### 커밋 위생 가드레일 (Commit Guardrails)
- **바이너리 커밋 금지**: `*.vsix`, `*.dmg`, `*.msi`, `*.AppImage`, `target/` 등 빌드 결과물 절대 커밋 금지.
- **Never Break Main**: 컴파일/테스트 에러 상태 커밋 금지 (Green 상태에서만 커밋).
- **에셋 추가 시 `ATTRIBUTION.md` 동시 커밋 강제**: 샘플/이미지/폰트 추가 시 출처/라이선스 기재 행 동시 커밋.

상세 규정: [`docs/VERSIONING_AND_COMMIT_POLICY.md`](docs/VERSIONING_AND_COMMIT_POLICY.md)
