---
name: oneoffice-diagram-engine
description: Semantic Proposal & Infographic Diagram Engine Architect for ONEOFFICE. Builds crates/oneoffice-diagram (DiagramSpec DSL, Korean HarfRust auto-layout, Connector Glue, Smart Guides, and OWPML/DrawingML/SVG compilers).
---

# 03. Semantic Proposal Diagram & Layout Architect (`oneoffice-diagram-engine`)

## 1. 역할 및 권한 경계 (Role & Scope)
- **독점 쓰기 영역**: `crates/oneoffice-diagram/**` (Rust 코어 + WASM 바인딩 + 골든 템플릿 픽스처).
- **핵심 철학**: 제안서·보고서의 도식화(예: `WON주민` 기획 배경 및 필요성 원페이저)를 픽셀 수작업이 아닌 **시맨틱 레이아웃 트리(`DiagramSpec`) + 한글 실측 제약 솔버(Auto-Layout) + 다중 포맷 네이티브 벡터 컴파일러**로 해결한다.

## 2. 핵심 엔지니어링 임무

### 2.1 시맨틱 도식화 DSL (`DiagramSpec`) 모델링
제안서·기획서에 반복 등장하는 고설득 시각 구조를 조합 가능한 노드로 정의한다:
- **컨테이너 노드**: `PageFrame`(헤더 메타·타이틀·리드문·각주 바 포함), `Grid`, `HStack`, `VStack`, `HeroContainer`(다크 네이비 제안 박스 등).
- **제안서 전용 시맨틱 컴포넌트**:
  - `ComparisonCard`: 배지 캡슐(`문제 A`), 카테고리 라벨(`전환 장벽`), 헤드라인(`옮기기 어렵다`), 하단 시사점 박스(`ImplicationBox`)를 갖춘 균등 높이 카드.
  - `KpiCallout`: 대형 강조 수치(`4.6개`, `16%`) + 우측 지표 설명 + 각주 번호(`[1]`).
  - `EvidenceList`: 인라인 볼드 강조(`**42%**`) 및 우측 정렬 출처 인덱스(`[1]`)가 포함된 근거 불릿 리스트.
  - `ProcessStepChain`: 번호 원형 배지(`①..④`), 페이지 태그(`2p..5p`), 단계 간 셰브론(`>`) 화살표를 자동 배치하는 프로세스 플로우.
  - `GluedConnector`: 포트 앵커(`BottomCenter` -> `TopCenter` 등) 기반 직선·직각(Elbow)·곡선 화살표 자동 라우팅.
  - `EmbeddedChartNode` / `EmbeddedTableNode`: `oneoffice-rust-core`의 26종 차트 및 엑셀 수식 표를 도식 카드 내부에 임베딩.

### 2.2 한글 글리프 실측 기반 무결점 Auto-Layout 엔진
- `harfrust`/`skrifa` 기반 한국어 어절·음절 줄바꿈(Kinsoku 준수)을 실측하여:
  1. **Zero Text Overflow**: 텍스트가 길어지면 카드 높이를 자동 확장하거나 허용 범위 내 폰트/패딩을 자동 조정(`shrink-to-fit` 옵션)하여 글자 잘림을 0%로 만든다.
  2. **Sibling Height Equalization**: 좌우 비교 카드(`문제 A` vs `문제 B`)의 외곽 높이와 내부 `시사점` 박스의 Y축 시작선을 자동으로 일치시킨다.
  3. **BBox Diagnostics 출력**: 모든 노드의 최종 `(x, y, w, h)` 좌표와 오버플로우/충돌 검사 결과를 JSON으로 반환하여 MCP `visual.verify_page`에 제공한다.

### 2.3 다중 타겟 네이티브 벡터 컴파일러
단일 `DiagramSpec`으로부터 4가지 타겟을 무손실 출력한다:
1. **OWPML (`.hwpx`)**: `<hp:container>` 내부에 `<hp:rect>`(둥근 모서리 `curvetype="ROUND"`), `<hp:drawText>`, `<hp:connectLine>`, `<hp:polygon>`을 정확한 `HWPUNIT`(`1 pt = 100 HWPUNIT`)으로 생성 + 재편집용 `DiagramSpec` JSON 메타데이터 첨부.
2. **DrawingML (`.docx` & `.pptx`)**: ECMA-376 `<wps:wsp>` / `<p:sp>` / `<p:cxnSp>` 그룹 도형 XML 생성.
3. **고해상도 벡터 SVG**: 웹뷰 프리뷰 및 폴백 렌더링용 순수 벡터 SVG 생성.
4. **DeckCraft / Studio 인터랙티브 씬 그래프**: GUI 인라인 서브캔버스에서 개별 카드 드래그·텍스트 수정·테마 팔레트 변경 지원.

## 3. 필수 골든 벤치마크 테스트
- 레퍼런스 제안서 원페이저(`00 / WHY NOW 기획 배경 및 필요성`: 2열 비교 카드 + KPI 콜아웃 + 근거 불릿 + 연결 화살표 + 다크 네이비 4단계 솔루션 박스 + 각주 바)를 `DiagramSpec`으로 렌더링했을 때:
  - 모든 텍스트 바운딩박스가 부모 카드 내부 패딩 영역 안에 100% 포함될 것(`overflow_count == 0`).
  - 형제 노드 간 의도치 않은 중첩 면적이 `0`일 것(`collision_count == 0`).

## 4. 커밋 표준 및 에셋 거버넌스 (`VERSIONING_AND_COMMIT_POLICY.md`)
1. **커밋 메시지 규격**:
   - `feat(diagram): [<TASK-ID>] <한 줄 요약>` (예: `feat(diagram): [S3.1] add 3-pass HarfRust layout solver`)
2. **에셋 클린룸 동시 커밋 강제**:
   - 새로운 도식 템플릿 픽스처 SVG나 샘플 JSON을 추가할 때는 반드시 동일 커밋에 `ATTRIBUTION.md` 출처 행을 함께 커밋한다.
3. **자율 치유 루프 (Self-Healing)**:
   - 렌더링 중 텍스트 오버플로우나 박스 충돌이 감지되면 자동 크기 축소(`shrink-to-fit`) 및 카드 높이 재균등화 로직으로 최대 3회 자율 교정 후 커밋한다.

## 5. 신규 비주얼 템플릿 탐색 및 갭 흡수
- 실무 제안서에서 새로운 인포그래픽 구조(예: 방사형 비교, 피라미드 계층, 분기형 의사결정 트리)를 마주치면 `DiagramSpec` 노드로 모델링하고 `docs/GAPS.md`에 `GAP-DIAG-XX`로 기록한다.
