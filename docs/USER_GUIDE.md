# ONEOFFICE — 사용자 매뉴얼 및 설치/사용 가이드 (User Guide)

> **All-in-One Office Platform**: HWP/HWPX(한글), DOCX(Word), XLSX(Excel), PPTX(PowerPoint), PDF 뷰어 & 전자서명/직인 날인을 완벽 통합한 차세대 올인원 오피스 플랫폼

---

## 📑 목차 (Table of Contents)

1. [시스템 요구사항](#1-시스템-요구사항)
2. [설치 가이드 (Installation)](#2-설치-가이드-installation)
   - [2.1 VS Code / Antigravity IDE 올인원 확장 설치](#21-vs-code--antigravity-ide-올인원-확장-설치)
   - [2.2 네이티브 데스크톱 앱 (macOS · Windows · Linux) 설치 및 실행](#22-네이티브-데스크톱-앱-macos--windows--linux-설치-및-실행)
   - [2.3 AI MCP 서버 (Model Context Protocol) 연동](#23-ai-mcp-서버-model-context-protocol-연동)
3. [상세 기능 및 사용 방법 (How to Use)](#3-상세-기능-및-사용-방법-how-to-use)
   - [3.1 한컴 HWPX / HWP 문서 열람 및 인플레이스 편집](#31-한컴-hwpx--hwp-문서-열람-및-인플레이스-편집)
   - [3.2 Word (DOCX) 문서 조판 및 리본 서식/문단 제어 HUD](#32-word-docx-문서-조판-및-리본-서식문단-제어-hud)
   - [3.3 513개 엑셀 수식 표 & 26종 벡터 차트 삽입/편집](#33-513개-엑셀-수식-표--26종-벡터-차트-삽입편집)
   - [3.4 컨설팅급 제안서 인포그래픽 도식화 (DiagramSpec)](#34-컨설팅급-제안서-인포그래픽-도식화-diagramspec)
   - [3.5 순수 Rust 고속 PDF 뷰어 & 전자서명 / 직인 크로마키 날인](#35-순수-rust-고속-pdf-뷰어--전자서명--직인-크로마키-날인)
   - [3.6 통합 찾기/바꾸기 HUD 및 라이트/다크 테마](#36-통합-찾기바꾸기-hud-및-라이트다크-테마)
4. [단축키 레퍼런스 (Keyboard Shortcuts)](#4-단축키-레퍼런스-keyboard-shortcuts)
5. [문제 해결 및 FAQ (Troubleshooting)](#5-문제-해결-및-faq-troubleshooting)

---

## 1. 시스템 요구사항

| 항목 | 최소 사양 | 권장 사양 |
| :--- | :--- | :--- |
| **운영체제 (OS)** | macOS 12 (Monterey) 이상<br>Windows 10 (1809) 이상 (x64/ARM64)<br>Linux Ubuntu 20.04+, Fedora 36+, Arch Linux | macOS 14 (Sonoma) 이상 (Apple Silicon M1/M2/M3)<br>Windows 11 (x64/ARM64)<br>Wayland/X11 지원 모던 Linux 배포판 |
| **IDE 환경** | VS Code v1.85.0 이상 / Antigravity IDE / Cursor | 최신 VS Code 또는 Antigravity IDE |
| **빌드 도구 (직접 컴파일 시)** | Rust 1.80+ (2024 Edition), Node.js v18.0+, npm v9+ | Rust 최신 stable, Node.js v20 LTS, `@vscode/vsce` |
| **외부 종속성** | **제로 (Zero)**: MS Office, 한컴오피스, Adobe Acrobat 미설치 PC 완벽 지원 | 외부 종속성 불필요 (순수 Rust 엔진 내장) |

---

## 2. 설치 가이드 (Installation)

ONEOFFICE는 사용 환경에 따라 세 가지 방식으로 설치하여 사용할 수 있습니다.

### 2.1 VS Code / Antigravity IDE 올인원 확장 설치

VS Code나 Antigravity IDE에서 별도 프로그램 설치 없이 `.hwpx`, `.docx`, `.xlsx`, `.pptx`, `.pdf` 파일을 원클릭으로 열람·편집할 수 있습니다.

#### 방법 A: 마켓플레이스 설치 (출시 후)
1. VS Code 좌측 액티비티 바의 **확장 탭**(`Ctrl+Shift+X` / `Cmd+Shift+X`)을 클릭합니다.
2. 검색창에 `ONEOFFICE`를 입력합니다.
3. **설치(Install)** 버튼을 누르면 즉시 모든 오피스 문서 및 PDF 뷰어 기능이 활성화됩니다.

#### 방법 B: 로컬 VSIX 패키지 파일로 직접 설치
배포용 `.vsix` 파일(`oneoffice-vscode-0.1.0-alpha.1.vsix`)을 가지고 있는 경우:

* **터미널(CLI) 명령으로 설치**:
  ```bash
  # VS Code에 설치
  code --install-extension extensions/oneoffice-vscode/oneoffice-vscode-0.1.0-alpha.1.vsix

  # Cursor IDE에 설치
  cursor --install-extension extensions/oneoffice-vscode/oneoffice-vscode-0.1.0-alpha.1.vsix
  ```

* **VS Code GUI로 설치**:
  1. VS Code 확장 탭(`Cmd+Shift+X`)을 엽니다.
  2. 확장 패널 상단 우측의 **`···` (추가 작업)** 메뉴를 클릭합니다.
  3. **`VSIX에서 설치... (Install from VSIX...)`**를 선택합니다.
  4. 다운로드된 `.vsix` 파일을 선택하면 즉시 설치가 완료됩니다.

---

### 2.2 네이티브 데스크톱 앱 (macOS · Windows · Linux) 설치 및 실행

외부 에디터 없이 독립형 데스크톱 오피스 스위트로 사용하고자 할 때 사용합니다.

#### 1) 소스코드에서 즉시 실행 (개발 및 사용자 공통)
```bash
# 최신 소스코드 클론
git clone https://github.com/iamywl/ONEOFFICE.git
cd ONEOFFICE

# 네이티브 데스크톱 앱 실행
cargo run --release -p oneoffice-desktop
```

#### 2) 플랫폼별 릴리즈 바이너리 패키징
[`packaging/build-desktop.sh`](file:///Users/ywlee/sideproejct/ONEOFFICE/packaging/build-desktop.sh) 스크립트를 사용하여 독립 실행 파일을 생성합니다:

* **macOS (Universal Binary: Apple Silicon + Intel)**:
  ```bash
  ./packaging/build-desktop.sh macos-universal
  # 출력: dist/desktop/ONEOFFICE-macos-universal
  ```
* **Windows (x64 Release)**:
  ```powershell
  cargo build --release -p oneoffice-desktop --target x86_64-pc-windows-msvc
  # 출력: target/release/oneoffice-desktop.exe
  ```
* **Linux (x86_64 Release)**:
  ```bash
  ./packaging/build-desktop.sh linux-release
  # 출력: dist/desktop/ONEOFFICE-linux-x86_64
  ```

---

### 2.3 AI MCP 서버 (Model Context Protocol) 연동

Claude Desktop, Antigravity IDE, Cursor 등과 연동하여 AI 에이전트가 문서를 직접 생성·편집·검증하도록 설정합니다.

#### 1) MCP 바이너리 빌드
```bash
cargo build --release -p oneoffice-mcp
```

#### 2) `claude_desktop_config.json` 또는 IDE MCP 설정 등록
```json
{
  "mcpServers": {
    "oneoffice": {
      "command": "/절대경로/ONEOFFICE/target/release/oneoffice-mcp",
      "args": []
    }
  }
}
```

제공되는 핵심 AI 도구:
- `diagram.compose`: 보고서 도식화(비교 카드, 로드맵) 자동 레이아웃
- `table.upsert`: 513개 엑셀 수식 계산 표 삽입
- `chart.upsert`: 26종 벡터 차트 생성 및 데이터 바인딩
- `pdf.sign`: 텍스트 앵커(`(인)`) 자동 감지 및 투명 직인 날인
- `visual.verify_page`: 페이지 렌더링 BBox 및 시각적 무결성 자가 검증

---

## 3. 상세 기능 및 사용 방법 (How to Use)

### 3.1 한컴 HWPX / HWP 문서 열람 및 인플레이스 편집

- **파일 열기**: 탐색기에서 `.hwpx`, `.hwp`, `.hml` 파일을 더블클릭하면 즉시 열람됩니다.
- **표 조작**: 표 셀 내부를 클릭 후 한컴 표준 단축키를 그대로 사용합니다:
  - `F5`: 표 셀 블록 지정 (한 번 더 누르면 다중 선택, `F7`로 행/열 크기 조절)
  - `M` (Merge): 선택된 셀 합치기
  - `S` (Split): 선택된 셀 나누기 (가로/세로 분할)
- **플로팅 글래스 HUD**: 표를 선택하면 94% 불투명도의 세련된 플로팅 조작 바가 나타나 테두리, 배경색, 여백을 마우스로 직관적으로 조절할 수 있습니다.

---

### 3.2 Word (DOCX) 문서 조판 및 리본 서식/문단 제어 HUD

문서 작성 시 직관적인 텍스트 서식 변경과 문단 재배치를 지원합니다.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [서식]  [Pretendard ▼]  [본문 ▼]  [-] 14pt [+]  [B] [I] [U] [S]  [글자색 ▼] [형광펜 ▼] [ < C > ] │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

1. **상단 서식 리본바 활용**:
   - **글꼴 선택**: `Pretendard`, `Apple SD Gothic Neo`, `Malgun Gothic` 등 원하는 폰트를 실시간으로 전환합니다.
   - **문단 프리셋**: `제목 1 (24pt)`, `제목 2 (18pt)`, `소제목 (14pt)`, `본문 (11pt)` 중 원하는 서식을 즉시 적용합니다.
   - **타이포그래피 스타일**: **굵게(B)**, *기울임(I)*, <u>밑줄(U)</u>, ~~취소선(S)~~ 토글 버튼으로 스타일을 부여합니다.
   - **컬러 및 하이라이트**: 텍스트 색상(검정, 네이비, 빨강, 초록, 보라)과 형광펜(노랑, 연두, 하늘)을 적용합니다.
   - **정렬**: 좌측 정렬(`<`), 가운데 정렬(`C`), 우측 정렬(`>`)을 지원합니다.

2. **문단별 퀵 컨트롤 HUD**:
   - 본문의 특정 문단을 클릭하면 해당 문단 상단에 미니 컨트롤 HUD가 나타납니다:
     - `[▲ 위로]`, `[▼ 아래로]`: 문단 순서를 드래그 없이 클릭 한 번으로 즉시 교체합니다.
     - `[+ 복제]`: 현재 문단의 스타일과 내용을 그대로 복사하여 아래에 새 문단을 생성합니다.
     - `[ B ]`, `[ I ]`, `[ U ]`, `[ S ]`: 현재 문단 전체의 서식을 즉시 토글합니다.
     - `[-]`, `[+]`: 글자 크기를 1pt 단위로 즉시 미세 조절합니다.
     - `[삭제 X]`: 해당 문단을 문서에서 즉시 제거합니다.
   - 문서 제목(`Title`) 및 부제(`Subtitle`)는 캔버스 상단에서 직접 클릭하여 즉시 텍스트를 수정할 수 있습니다.

---

### 3.3 513개 엑셀 수식 표 & 26종 벡터 차트 삽입/편집

외부 엑셀 프로그램을 켜서 작업 후 스크린샷을 찍을 필요가 없습니다.

1. **엑셀 수식 표 삽입**:
   - 상단 툴바의 **`[+ 엑셀 수식표 삽입]`**을 누릅니다.
   - 셀을 클릭하고 `=SUM(B2:B10)`, `=AVERAGE(C2:C10)`, `=VLOOKUP(...)` 등 엑셀 함수를 입력하면 순수 Rust `gridcraft-calc` 엔진이 실시간으로 재계산합니다.
2. **26종 고화질 벡터 차트 삽입**:
   - 상단 툴바의 **`[+ 26종 차트 삽입]`**을 누릅니다.
   - 막대형(Bar), 꺾은선형(Line), 원형(Pie), 도넛(Doughnut), 방사형(Radar) 등 원하는 차트 종류를 선택합니다.
   - 표 데이터가 변경되면 차트가 실시간 벡터로 다시 그려집니다.

---

### 3.4 컨설팅급 제안서 인포그래픽 도식화 (DiagramSpec)

기획서, 사업계획서, 제안서에서 가장 중요한 비주얼 요소를 코드 및 GUI로 생성합니다.

1. **도식화 삽입**:
   - 상단 툴바의 **`[+ 제안서 도식화 삽입]`**을 클릭합니다.
   - **3단 비교 카드**, **대형 숫자 KPI 콜아웃**, **단계별 프로세스 로드맵** 중 원하는 템플릿을 선택합니다.
2. **한글 오버플로우 제로 자동 레이아웃**:
   - 텍스트 길이에 맞춰 박스의 크기가 자동으로 늘어나며, `harfrust` 한글 글리프 실측 조판 엔진이 텍스트가 박스 바깥으로 튀어나가지 않도록 자동으로 여백을 재계산합니다.

---

### 3.5 순수 Rust 고속 PDF 뷰어 & 전자서명 / 직인 크로마키 날인

종이 문서를 출력하여 도장을 찍고 다시 스캔하는 번거로운 과정을 완전히 디지털화합니다.

1. **PDF 열람 및 검색**:
   - `.pdf` 파일을 열면 고해상도 벡터 렌더링으로 즉시 페이지가 로드됩니다.
   - `Ctrl+F` / `Cmd+F`를 눌러 본문 텍스트를 실시간으로 검색할 수 있습니다.
2. **서명 패드 및 도장 날인**:
   - 상단의 **`[서명 패드 열기]`**를 클릭합니다.
   - **자필 서명**: 마우스나 터치펜으로 부드러운 베지어 곡선 서명을 그립니다.
   - **직인(도장) 이미지 추가**: 스마트폰으로 촬영한 도장 사진(PNG/JPG)을 업로드합니다.
3. **흰 배경 자동 투명화 (Chroma-Key Alpha Matting)**:
   - 일반 도장 사진을 올리면 알고리즘이 배경의 흰 종이 색상을 자동으로 감지하여 **완전 투명**하게 만듭니다.
   - 계약서나 공문서의 `(인)` 또는 `서명란` 위치로 도장을 드래그하여 배치합니다.
4. **무손실 PDF 저장 (`Export PDF`)**:
   - 상단의 **`[저장]`** 또는 **`[PDF 내보내기]`**를 누르면, 원본 PDF의 텍스트 레이어, 하이퍼링크, 북마크를 100% 손실 없이 보존한 상태로 날인된 최종 PDF가 생성됩니다.

---

### 3.6 통합 찾기/바꾸기 HUD 및 라이트/다크 테마

- **통합 검색 (`Ctrl+F` / `Cmd+F`)**:
  - HWPX, DOCX, XLSX, PDF 전 포맷 공통으로 우측 상단에 플로팅 검색 HUD가 열립니다.
  - 일치하는 단어의 개수 실시간 표시 및 `[다음]`, `[이전]` 이동, `[바꾸기]`, `[모두 바꾸기]`를 지원합니다.
- **테마 전환**:
  - 우측 상단의 테마 토글 버튼을 통해 **라이트 모드(Sun)**와 **다크 모드(Moon)**를 언제든 전환할 수 있습니다.
  - 종이 문서 본문(Paper)은 표준 가독성을 위해 흰색 배경을 유지하며, 주변 툴바 및 HUD는 눈의 피로를 덜어주는 글래스모피즘 테마가 적용됩니다.

---

## 4. 단축키 레퍼런스 (Keyboard Shortcuts)

| 단축키 (macOS) | 단축키 (Windows/Linux) | 기능 설명 |
| :--- | :--- | :--- |
| `Cmd + S` | `Ctrl + S` | 문서 저장 (무손실 원본 포맷 갱신) |
| `Cmd + F` | `Ctrl + F` | 통합 찾기 / 바꾸기 플로팅 HUD 토글 |
| `Cmd + Z` | `Ctrl + Z` | 실행 취소 (Undo) |
| `Cmd + Shift + Z` | `Ctrl + Y` | 다시 실행 (Redo) |
| `F5` | `F5` | 한글 표 셀 선택 / 다중 블록 지정 |
| `M` | `M` | 한글 표 선택된 셀 합치기 (Merge) |
| `S` | `S` | 한글 표 선택된 셀 분할 (Split) |
| `Alt + L` | `Alt + L` | 한글 글자 모양 대화상자 |
| `Alt + T` | `Alt + T` | 한글 문단 모양 대화상자 |
| `F11` | `F11` | 제안서 인포그래픽 도식화 삽입 HUD |

---

## 5. 문제 해결 및 FAQ (Troubleshooting)

### Q1. VS Code에서 파일을 열었는데 텍스트 에디터로 열립니다.
> **해결**: 파일 탐색기에서 해당 파일 우클릭 → `연결 프로그램으로 열기... (Open With...)` → `ONEOFFICE` 커스텀 에디터를 선택하고 `기본값으로 설정(Configure default editor for ...)`을 선택하세요.

### Q2. 한글 폰트가 깨지거나 네모 박스(ㅁ)로 표시됩니다.
> **해결**: ONEOFFICE는 시스템의 `Pretendard`, `Apple SD Gothic Neo`, `맑은 고딕(Malgun Gothic)`을 자동 감지합니다. 시스템에 한글 폰트가 설치되어 있는지 확인하거나 상단 리본바에서 `Pretendard`를 선택해 주세요.

### Q3. 도장 사진을 올렸는데 배경이 투명해지지 않습니다.
> **해결**: 도장 사진이 너무 어둡거나 그림자가 진 경우 크로마키 민감도를 조절할 수 있습니다. 서명 패드에서 `알파 매팅(투명화)` 체크박스가 켜져 있는지 확인하세요.
