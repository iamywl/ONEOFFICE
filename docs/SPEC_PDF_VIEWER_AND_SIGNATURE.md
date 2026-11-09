# 크로스플랫폼 PDF 뷰어 및 비파괴 서명/직인 엔진 기술 명세서 (`oneoffice-pdf`)

> **연관 에이전트**: `05-pdf-viewer-signature-engineer`, `06-studio-ux-craftsman`, `07-mcp-agent-architect`, `09-adversarial-qa-auditor`  
> **핵심 목표**: 외부 유료 라이브러리나 `GPL`/`AGPL`(`MuPDF`, `Poppler`) 오염 없이 **100% 순수 Rust(`MIT OR Apache-2.0`)** 로 `macOS`, `Windows`, `Linux`, 그리고 `VS Code Webview(WASM)`에서 동작하는 고해상도 PDF 뷰어 및 전자서명·직인 날인 엔진을 구축합니다.

---

## 1. 기반 크레이트 및 라이선스 검증

워크스페이스 내 `storytold/craft-libs`(`rev = "92d392cff132eaf53e13afff77f38b5c11b73aa0"`)에 이미 검증·내장된 **`craft-pdf`** 스택을 그대로 활용합니다:
- **`hayro-syntax` (`0.7.2`)**: PDF XRef 테이블(레거시 테이블 및 PDF 1.5+ XRef 스트림), 간접 객체(`IndirectObject`), 페이지 트리, 북마크(`Outlines`), 주석(`Annots`) 파싱.
- **`hayro-interpret` (`0.7.0`, `embed-fonts`, `embed-cmaps`)**: CJK(한국어 UniKS-UCS2/UTF16 CMap 포함) 폰트 디코딩, 콘텐츠 스트림 해석, 텍스트 스팬(`TextSpan`) 및 변환 행렬(`CTM`) 추출.
- **`hayro` (`0.7.1`)**: 멀티스레드(네이티브) 및 싱글스레드(WASM) 고해상도 RGBA 페이지/타일 래스터라이저.
- **`krilla` (`0.8.2`) + 증분 PDF 라이터**: 벡터 그래픽, 서브셋 폰트, 소프트 알파 마스크(`SMask`) 이미지 XObject 생성.
- **안전성**: 전 계층 `unsafe_code = "forbid"`, OS 네이티브 C++ 의존성 **0개**.

---

## 2. 고성능 PDF 뷰어 아키텍처 (`PdfViewerCore`)

1. **다단계 해상도 타일 & 프리페칭 캐시 (`PageTileCache`)**:
   - 메모리 상한(`256 MiB` 기본값) 내에서 현재 뷰포트 페이지(`1.5x`~`3.0x` HiDPI)와 인접 `±2` 페이지를 백그라운드 렌더링하고, 좌측 사이드바용 저해상도(`0.2x`) 썸네일 스트립을 별도 유지합니다.
2. **텍스트 검색 & 선택 좌표계 (`PageTextIndex`)**:
   - 페이지별 글리프 런을 PDF 사용자 공간 좌표(`pt`, 좌하단 원점)에서 뷰어 화면 좌표(좌상단 원점, `/Rotate` `0/90/180/270` 및 `/CropBox` 보정 완료)로 정규화하여:
     - 드래그 텍스트 선택 및 클립보드 복사(`Cmd/Ctrl+C`)
     - 전체 페이지 키워드 정규식/부분문자열 고속 검색 및 하이라이트
     - **서명 앵커 텍스트(`"(인)"`, `"(서명)"`, `"대표자"`) 바운딩박스 자동 탐지**에 공용으로 사용합니다.
3. **안전한 하이퍼링크 및 목차 네비게이션**:
   - 내부 페이지 점프(`/Dest`) 및 `craft_pdf::safe_url`을 통과한 안전한 외부 `https://` / `mailto:` 링크만 열기를 허용합니다(`javascript:` 및 로컬 파일 실행 원천 차단).

---

## 3. 3대 서명·직인 생성 파이프라인

### 3.1 자필 벡터 서명 (`FreehandBezierSignature`)
- 마우스·트랙패드·펜 입력 점열 `(x_i, y_i, t_i)`를 수집하여 Ramer-Douglas-Peucker 노이즈 제거 후 **3차 베지어 스플라인(`kurbo::BezPath`)** 으로 피팅합니다.
- 이동 속도에 반비례하는 가변 선 두께(필압 시뮬레이션) 외곽선 폴리곤을 생성하므로, **래스터 픽셀 계단 현상 없이 400% 확대 및 인쇄 시에도 칼같이 선명한 벡터 서명**으로 PDF에 기록됩니다.

### 3.2 흰 배경 도장/서명 자동 투명화 (`ChromaKeyAlphaMatting`)
실무에서 종이에 찍은 법인인감/개인도장이나 자필 서명을 스마트폰으로 촬영한 JPG/PNG는 배경이 회백색이라 문서 글씨를 가리는 문제가 있습니다. 이를 순수 Rust 이미지 필터로 즉시 투명화합니다:
1. 입력 RGBA 픽셀을 선형 휘도 $L \in [0, 1]$ 및 채도 거리 $C = \max(|R-G|, |G-B|, |R-B|)$ 로 변환합니다.
2. 지역 적응형 배경 백색점 $L_{\text{bg}}$ (모서리 샘플 중앙값, 기본 `0.88`) 기준으로 전경 잉크 강도 $I = \max(L_{\text{bg}} - L,\; \gamma \cdot C)$ 를 계산합니다(붉은 인주는 휘도가 높아도 채도 $C$가 커서 선명하게 보존됨!).
3. 소프트 램프 $\alpha = \text{smoothstep}(\tau_{\text{low}}, \tau_{\text{high}}, I)$ 와 배경색 디콘태미네이션(Color Decontamination)을 적용해 테두리 하얀 끼임(White Halo) 없는 깨끗한 투명 PNG/SMask를 생성합니다.

### 3.3 직인/서명 + 날짜 스탬프 프리셋 (`PlacedStampLayer`)
- 편집 세션 중 각 서명 개체는 `{ id, page_index, rect_pt, rotation_deg, opacity, payload: VectorPath | RgbaImage | TextStamp }` 불변 스냅샷으로 관리되어 이동, 모서리 비율 유지 리사이즈, 회전, 삭제, `Undo/Redo`가 자유롭습니다.

---

## 4. 비파괴(Non-Destructive) PDF 서명 저장 규격

서명 1개를 추가하기 위해 수십 페이지짜리 원본 PDF 전체를 이미지로 굽거나 재작성하면 **본문 검색이 깨지고 파일 용량이 폭증하며 기존 하이퍼링크·북마크가 손실**됩니다. `oneoffice-pdf`는 **표준 PDF 증분 업데이트(Incremental Update, ISO 32000-1 §7.5.6)** 방식을 기본 채택합니다:

1. **원본 바이트 100% 보존**: 원본 PDF 바이트 뒤에 새로운 객체(서명 `Form XObject` `/FmOneOfficeSig_N`, 투명 알파 `/SMask`, 갱신된 대상 `/Page` 딕셔너리)와 추가 `xref` 및 `trailer`(`Prev` 포인터 연결)만 덧붙입니다.
2. **그래픽 상태 격리 (`q ... Q` Isolation — 필수 안전장치)**:
   - 공공기관·은행에서 생성된 일부 PDF는 기존 페이지 콘텐츠 스트림 끝에 복원되지 않은 변환 행렬(`cm`)이 남아 있어, 뒤에 스트림을 그냥 이어 붙이면 서명이 엉뚱한 위치로 날아갑니다.
   - 따라서 반드시 대상 페이지의 기존 `/Contents` 스트림 앞을 `q\n`(상태 저장) 스트림으로, 뒤를 `\nQ\nq <ctm> cm /FmOneOfficeSig_N Do Q`(상태 복원 후 정규 좌표계에서 서명 XObject 렌더링) 스트림 배열로 감싸 **어떤 기형 PDF에서도 100% 정확한 화면 위치에 서명이 찍히도록 보장**합니다.
3. **선택적 평탄화(Flatten) 내보내기 모드**: 보안 제출용으로 서명 분리 수정 자체를 막고 싶을 때는 `flatten: true` 옵션으로 페이지 재직렬화 출력도 함께 지원합니다.
