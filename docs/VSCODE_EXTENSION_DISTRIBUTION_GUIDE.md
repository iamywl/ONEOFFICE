# ONEOFFICE — VS Code 익스텐션 패키징 및 배포 완벽 가이드 (VSIX Distribution Guide)

> **대상 독자**: ONEOFFICE 플랫폼 개발자, 릴리즈 거버너, 시스템 관리자  
> **관련 에이전트**: [`08-crossplatform-packaging-governor`](file:///Users/ywlee/sideproejct/ONEOFFICE/.agents/agents/08-crossplatform-packaging-governor.md)  
> **대상 패키지**: [`extensions/oneoffice-vscode`](file:///Users/ywlee/sideproejct/ONEOFFICE/extensions/oneoffice-vscode)

---

## 📑 목차 (Table of Contents)

1. [개요 및 VSIX 아키텍처](#1-개요-및-vsix-아키텍처)
2. [사전 준비 요구사항](#2-사전-준비-요구사항)
3. [로컬 빌드 및 VSIX 패키징 (`vsce package`)](#3-로컬-빌드-및-vsix-패키징-vsce-package)
4. [로컬 사이드로딩 및 동작 검증 테스트](#4-로컬-사이드로딩-및-동작-검증-테스트)
5. [공식 Visual Studio Marketplace 배포 절차](#5-공식-visual-studio-marketplace-배포-절차)
   - [5.1 Azure DevOps 계정 및 개인 액세스 토큰 (PAT) 발급](#51-azure-devops-계정-및-개인-액세스-토큰-pat-발급)
   - [5.2 Marketplace Publisher 생성 및 등록](#52-marketplace-publisher-생성-및-등록)
   - [5.3 CLI 로그인 및 퍼블리싱 (`vsce publish`)](#53-cli-로그인-및-퍼블리싱-vsce-publish)
6. [오픈소스 대안 마켓플레이스 배포 (Open VSX Registry)](#6-오픈소스-대안-마켓플레이스-배포-open-vsx-registry)
7. [GitHub Actions CI/CD 자동화 배포 파이프라인](#7-github-actions-cicd-자동화-배포-파이프라인)
8. [자주 묻는 질문 및 트러블슈팅 (FAQ)](#8-자주-묻는-질문-및-트러블슈팅-faq)

---

## 1. 개요 및 VSIX 아키텍처

ONEOFFICE VS Code 확장은 `.hwpx`, `.docx`, `.xlsx`, `.pptx`, 그리고 `.pdf` 문서를 VS Code / Antigravity IDE 내부에서 직접 열람·편집·날인할 수 있는 단일 통합 VSIX 패키지입니다.

```
┌────────────────────────────────────────────────────────────────────────┐
│               ONEOFFICE All-in-One VSIX (.vsix archive)                │
├───────────────────┬────────────────────────────────────────────────────┤
│ extension.js      │ Node.js Extension Host (CustomEditor Provider)     │
│ vendor/studio/    │ 5대 오피스 캔버스 조판 런타임 & 오프라인 WASM 코어 │
│ media/            │ 플로팅 글래스 HUD, 수식 바, 서명 캔버스 웹뷰 어댑터 │
│ icon.png          │ 256x256 고해상도 브랜드 아이콘                      │
│ package.json      │ 5대 파일 매핑 및 커스텀 에디터 선언 매니페스트    │
└───────────────────┴────────────────────────────────────────────────────┘
```

- **순수 오프라인 보안 (Zero External Network)**: 외부 CDN에 의존하지 않고 모든 폰트, 스크립트, WASM이 단일 패키지 안에 내장됩니다.
- **WASM 호스트 격리**: `vscode.WebviewPanel`을 통해 엄격한 Content Security Policy(CSP) 환경에서 안전하게 구동됩니다.

---

## 2. 사전 준비 요구사항

익스텐션을 빌드하고 배포하기 위해 아래 개발 환경이 준비되어야 합니다:

1. **Node.js**: v18.0.0 이상 (권장: v20 LTS)
2. **패키지 관리자**: npm v9+
3. **VS Code CLI 도구 (`@vscode/vsce`)**:
   ```bash
   npm install -g @vscode/vsce
   # 또는 npx로 즉시 실행 가능
   ```
4. **VS Code 에디터**: v1.85.0 이상 (테스트용)

---

## 3. 로컬 빌드 및 VSIX 패키징 (`vsce package`)

### 3.1 원클릭 패키징 명령어
프로젝트 루트 또는 `extensions/oneoffice-vscode` 디렉토리에서 패키징 명령을 실행합니다:

```bash
cd extensions/oneoffice-vscode

# 빌드 및 VSIX 생성 (원클릭)
npm run package
```

내부적으로 다음 프로세스가 순차 실행됩니다:
1. `node build.mjs`: `src/extension.ts`를 `dist/extension.js`로 번들링 (esbuild)
2. `npx @vscode/vsce package --no-dependencies --follow-symlinks`: 불필요한 개발 소스를 제외하고 배포용 `.vsix` 압축 생성

### 3.2 빌드 결과물 확인
명령이 완료되면 디렉토리에 버전 번호가 포함된 VSIX 파일이 생성됩니다:
```text
extensions/oneoffice-vscode/oneoffice-vscode-0.1.0-alpha.1.vsix
```

> [!IMPORTANT]
> **Git 커밋 위생 수칙**: `.vsix` 바이너리 파일은 대용량 파일이므로 git 저장소에 커밋하지 않습니다. 루트 `.gitignore`에 의해 자동으로 차단되어 있습니다.

---

## 4. 로컬 사이드로딩 및 동작 검증 테스트

빌드된 VSIX 파일을 공식 마켓플레이스에 올리기 전에 내 컴퓨터의 VS Code에서 직접 설치하여 테스트합니다.

### 4.1 CLI를 통한 설치
```bash
# 터미널에서 로컬 VSIX 직접 설치
code --install-extension extensions/oneoffice-vscode/oneoffice-vscode-0.1.0-alpha.1.vsix

# Cursor IDE 사용자일 경우
cursor --install-extension extensions/oneoffice-vscode/oneoffice-vscode-0.1.0-alpha.1.vsix
```

### 4.2 GUI를 통한 설치
1. VS Code 실행 → 좌측 바의 **확장 탭**(`Cmd+Shift+X` / `Ctrl+Shift+X`) 선택
2. 패널 상단 우측의 **`···` (추가 작업)** 클릭
3. **`VSIX에서 설치... (Install from VSIX...)`** 메뉴 클릭
4. 빌드된 `oneoffice-vscode-0.1.0-alpha.1.vsix` 파일 선택

### 4.3 동작 검증 체크리스트
- [ ] `.hwpx` 파일 클릭 시 한컴 뷰어가 열리고 표 셀 선택(`F5`)이 동작하는가?
- [ ] `.docx` 파일 클릭 시 본문 텍스트와 타이포그래피 HUD가 정상 표시되는가?
- [ ] `.xlsx` 파일 클릭 시 스프레드시트 수식 계산이 동작하는가?
- [ ] `.pptx` 파일 클릭 시 프레젠테이션 슬라이드가 렌더링되는가?
- [ ] `.pdf` 파일 클릭 시 고해상도 뷰어가 열리고 서명 패드가 동작하는가?

---

## 5. 공식 Visual Studio Marketplace 배포 절차

전 세계 사용자가 VS Code 내부 검색(`ONEOFFICE`)을 통해 설치할 수 있도록 Microsoft 공식 마켓플레이스에 배포하는 절차입니다.

### 5.1 Azure DevOps 계정 및 개인 액세스 토큰 (PAT) 발급

마켓플레이스 CLI 배포를 위해 Azure DevOps의 개인 액세스 토큰(Personal Access Token)이 필수적입니다.

1. [Azure DevOps](https://dev.azure.com)에 Microsoft 계정으로 로그인합니다.
2. 우측 상단의 **사용자 설정(User Settings)** 아이콘 → **Personal access tokens**를 클릭합니다.
3. **`+ New Token`**을 클릭합니다.
4. 토큰 생성 폼을 다음과 같이 설정합니다:
   - **Name**: `oneoffice-vsce-token`
   - **Organization**: 반드시 **`All accessible organizations`** 선택 (특정 조직 지정 시 401 오류 발생)
   - **Expiration**: 유효 기간 설정 (예: 90일 또는 1년)
   - **Scopes**: 하단의 `Show all scopes`를 클릭한 후, **`Marketplace`** 항목을 찾아 **`Acquire`** 및 **`Manage`** 체크박스에 체크합니다.
5. **Create**를 누르고 생성된 토큰 문자열을 복사하여 안전한 곳에 보관합니다 (화면을 벗어나면 다시 볼 수 없습니다).

---

### 5.2 Marketplace Publisher 생성 및 등록

1. [Visual Studio Marketplace Management Portal](https://marketplace.visualstudio.com/manage)에 접속합니다.
2. **`Create publisher`** 버튼을 클릭합니다.
3. 퍼블리셔 정보를 입력합니다:
   - **ID**: 고유 식별자 (예: `iamywl` — `package.json`의 `"publisher"` 필드와 100% 일치해야 함)
   - **Name**: 표시 이름 (예: `ONEOFFICE Team` 또는 개인명)
4. 저장을 완료합니다.

> [!NOTE]
> `package.json`의 `"publisher"` 값이 마켓플레이스에 등록된 ID와 다를 경우 배포가 거부됩니다. 현재 프로젝트의 퍼블리셔 ID는 `iamywl`로 설정되어 있습니다.

---

### 5.3 CLI 로그인 및 퍼블리싱 (`vsce publish`)

#### 1) 퍼블리셔 계정 로그인 (최초 1회)
```bash
cd extensions/oneoffice-vscode
npx @vscode/vsce login iamywl
# 프롬프트가 나타나면 발급받은 Personal Access Token (PAT)을 입력합니다.
```

#### 2) 익스텐션 퍼블리싱
```bash
# 기본 배포
npx @vscode/vsce publish --no-dependencies --follow-symlinks

# 또는 버전 자동 증가와 함께 배포:
npx @vscode/vsce publish patch --no-dependencies --follow-symlinks   # 0.1.0 -> 0.1.1
npx @vscode/vsce publish minor --no-dependencies --follow-symlinks   # 0.1.0 -> 0.2.0
npx @vscode/vsce publish major --no-dependencies --follow-symlinks   # 0.1.0 -> 1.0.0
```

배포가 성공하면 수 분 내에 [Visual Studio Marketplace](https://marketplace.visualstudio.com)에 등록되며, VS Code 확장 탭 검색에 노출됩니다.

---

## 6. 오픈소스 대안 마켓플레이스 배포 (Open VSX Registry)

VS Code 포크 에디터(예: **VSCodium, Gitpod, Eclipse Theia, Cursor**)는 마이크로소프트 전용 마켓플레이스를 사용할 수 없으며 오픈 마켓플레이스인 **Open VSX Registry**를 사용합니다.

### 6.1 배포 단계
1. [open-vsx.org](https://open-vsx.org)에 GitHub 또는 Eclipse 계정으로 로그인합니다.
2. 프로필 설정에서 **Access Tokens**를 생성합니다.
3. 퍼블리셔 네임스페이스(`iamywl`)를 등록합니다.
4. CLI 도구 `ovsx`를 통해 이미 빌드된 VSIX 파일을 배포합니다:
   ```bash
   # ovsx 도구로 VSIX 배포
   npx ovsx publish extensions/oneoffice-vscode/oneoffice-vscode-0.1.0-alpha.1.vsix -p <OPEN_VSX_TOKEN>
   ```

---

## 7. GitHub Actions CI/CD 자동화 배포 파이프라인

Git 태그(`v*.*.*`)를 푸시했을 때 자동으로 VSIX 빌드, 테스트, GitHub Release 업로드, 마켓플레이스 배포를 수행하는 워크플로우 구성 예시입니다.

```yaml
# .github/workflows/release-vscode-extension.yml
name: Release VS Code Extension

on:
  push:
    tags:
      - 'v*'

jobs:
  build-and-publish:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout Code
        uses: actions/checkout@v4
        with:
          submodules: recursive

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Build and Package Extension
        run: |
          cd extensions/oneoffice-vscode
          npm run package

      - name: Publish to VS Code Marketplace
        run: |
          cd extensions/oneoffice-vscode
          npx @vscode/vsce publish --packagePath *.vsix -p ${{ secrets.VSCE_PAT }}

      - name: Publish to Open VSX Registry
        run: |
          cd extensions/oneoffice-vscode
          npx ovsx publish *.vsix -p ${{ secrets.OVSX_PAT }}

      - name: Upload VSIX to GitHub Releases
        uses: softprops/action-gh-release@v1
        with:
          files: extensions/oneoffice-vscode/*.vsix
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

---

## 8. 자주 묻는 질문 및 트러블슈팅 (FAQ)

### Q1. `vsce publish` 실행 시 `Error: 401 Unauthorized` 오류가 발생합니다.
> **원인**: PAT 토큰 발급 시 조직 범위를 특정 단체로 한정했거나, `Marketplace (Manage)` 권한이 누락된 경우입니다.  
> **해결**: Azure DevOps에서 새 PAT를 발급받을 때 Organization을 반드시 **`All accessible organizations`**로 선택하고 `Marketplace: Acquire & Manage`를 부여하세요.

### Q2. `vsce package` 실행 시 `Error: EISDIR: illegal operation on a directory` 오류가 발생합니다.
> **원인**: `vendor/` 또는 `media/` 디렉토리가 심볼릭 링크로 연결되어 있어 vsce가 디렉토리를 파일로 읽으려 할 때 발생합니다.  
> **해결**: 반드시 `--follow-symlinks` 옵션을 함께 전달해야 합니다. `npm run package` 스크립트에 이미 포함되어 있습니다.

### Q3. 패키징된 VSIX의 용량을 줄이고 싶습니다.
> **해결**: `.vscodeignore` 파일에 개발용 소스(`src/**`), 테스트 파일(`test*.mjs`), 소스맵(`**/*.map`)이 포함되어 있는지 확인하세요. 현재 ONEOFFICE의 번들 크기는 약 28MB로 최적화되어 있습니다.
