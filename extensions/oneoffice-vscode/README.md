# ONEOFFICE — All-in-One Office Platform for VS Code

[![License](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/iamywl/ONEOFFICE/blob/main/LICENSE-MIT)
[![VS Code](https://img.shields.io/badge/VS%20Code-v1.85%2B-2563EB.svg)](https://marketplace.visualstudio.com)
[![Rust Engine](https://img.shields.io/badge/Rust-unsafe__code%20%3D%20%22forbid%22-10B981.svg)](https://github.com/iamywl/ONEOFFICE)

**ONEOFFICE**는 HWP/HWPX(한글), DOCX(Word), XLSX(Excel), PPTX(PowerPoint), 그리고 **PDF 뷰어 및 전자서명/직인 날인**을 VS Code 및 Antigravity IDE 내부에서 외부 앱 설치 없이 즉시 열람·편집할 수 있는 차세대 올인원 오피스 확장팩입니다.

---

## 🌟 주요 기능 (Key Features)

1. **한컴 HWPX / HWP 무손실 뷰어 & 에디터**
   - `.hwpx`, `.hwp`(배포용 복호화 지원), `.hml` 완벽 지원
   - 표준 한컴 단축키(`F5`, `M`, `S`, `Alt+L`, `Alt+T`, `F11`) 완벽 지원
   - 플로팅 글래스모피즘 HUD 및 한글 조합형 IME 완벽 지원

2. **Word (DOCX) 조판 및 인플레이스 서식 HUD**
   - Pretendard 등 시스템 폰트 실시간 반영, 글자 크기, 볼드/이탤릭/밑줄/취소선
   - 텍스트 색상 및 형광펜 하이라이트, 좌/중/우 정렬
   - 문단별 퀵 컨트롤 HUD (`[▲]`, `[▼]` 순서 이동, `[+]` 복제, `[X]` 삭제)

3. **Excel (XLSX) 513개 수식 표 & 26종 벡터 차트**
   - 513개 엑셀 수식(`SUM`, `AVERAGE`, `VLOOKUP`, `XLOOKUP`, `IF` 등) 실시간 연산
   - 26종 고화질 벡터 차트(막대, 꺾은선, 원형, 도넛, 방사형 등) 인플레이스 생성 및 바인딩

4. **컨설팅급 제안서 인포그래픽 도식화 (DiagramSpec)**
   - 비교 카드, 대형 KPI 콜아웃, 프로세스 로드맵 자동 레이아웃
   - 한글 HarfRust 실측 글리프 레이아웃 기반 텍스트 오버플로우 방지

5. **순수 Rust 고속 PDF 뷰어 & 무손실 전자서명/직인 날인**
   - 고해상도 PDF 열람, 텍스트 검색, 목차 탐색
   - 마우스/펜 자필 베지어 서명 및 스마트폰 도장 사진 첨부
   - **흰 배경 자동 투명화 (Chroma-Key Matting)**: 도장 뒷배경을 자동으로 날려 본문 텍스트를 가리지 않고 날인

6. **통합 AI Agent MCP 서버 (`oneoffice-mcp`) 내장**
   - `diagram.compose`, `table.upsert`, `chart.upsert`, `pdf.sign`, `visual.verify_page` 등 표준 MCP 도구 제공

---

## 🚀 빠른 시작 (Quick Start)

### 1. 확장 설치
VS Code 마켓플레이스에서 `ONEOFFICE`를 검색하여 설치하거나, 로컬 `.vsix` 파일로 설치합니다:
```bash
code --install-extension oneoffice-vscode-0.1.0-alpha.1.vsix
```

### 2. 파일 열기
탐색기(Explorer)에서 `.hwpx`, `.docx`, `.xlsx`, `.pptx`, `.pdf` 파일을 클릭하면 전용 ONEOFFICE 커스텀 에디터로 자동 실행됩니다.

### 3. 새 문서 만들기 커맨드
명령 팔레트(`Ctrl+Shift+P` / `Cmd+Shift+P`):
- `ONEOFFICE: 새 HWPX 제안서 문서 만들기`
- `ONEOFFICE: 새 DOCX 보고서 만들기`
- `ONEOFFICE: 새 XLSX 수식 스프레드시트 만들기`
- `ONEOFFICE: 새 PPTX 프레젠테이션 만들기`
- `ONEOFFICE: PDF 전자서명 및 직인 날인하기`

---

## 📄 라이선스 (License)

이 프로젝트는 [MIT License](LICENSE)에 따라 배포됩니다.
자세한 정보 및 소스 코드는 [ONEOFFICE GitHub 저장소](https://github.com/iamywl/ONEOFFICE)를 참고해 주세요.
