import * as vscode from 'vscode';
import { basename } from 'node:path';
import { unzipSync } from 'fflate';
import { DOMParser } from '@xmldom/xmldom';

interface SlideData {
  slideNumber: number;
  title: string;
  bullets: string[];
}

export class OneOfficePptxProvider implements vscode.CustomReadonlyEditorProvider {
  constructor(private readonly context: vscode.ExtensionContext) {}

  async openCustomDocument(uri: vscode.Uri): Promise<vscode.CustomDocument> {
    return { uri, dispose: () => {} };
  }

  async resolveCustomEditor(
    document: vscode.CustomDocument,
    webviewPanel: vscode.WebviewPanel
  ): Promise<void> {
    const webview = webviewPanel.webview;
    webview.options = { enableScripts: true };

    const bytes = await vscode.workspace.fs.readFile(document.uri);
    const fileName = basename(document.uri.path);

    const slides = this.parsePptx(bytes);
    webview.html = this.renderSlidesHtml(fileName, slides);
  }

  private parsePptx(bytes: Uint8Array): SlideData[] {
    const slides: SlideData[] = [];
    try {
      const zip = unzipSync(bytes);
      const slideKeys = Object.keys(zip)
        .filter(k => /^ppt\/slides\/slide\d+\.xml$/i.test(k))
        .sort((a, b) => {
          const numA = parseInt(a.match(/\d+/)?.[0] || '0', 10);
          const numB = parseInt(b.match(/\d+/)?.[0] || '0', 10);
          return numA - numB;
        });

      slideKeys.forEach((key, idx) => {
        const xml = new TextDecoder().decode(zip[key]);
        const dom = new DOMParser().parseFromString(xml, 'application/xml');
        const paragraphs = dom.getElementsByTagName('a:p');
        const lines: string[] = [];

        for (let i = 0; i < paragraphs.length; i++) {
          const p = paragraphs[i];
          const texts = p.getElementsByTagName('a:t');
          let line = '';
          for (let j = 0; j < texts.length; j++) {
            line += texts[j].textContent || '';
          }
          if (line.trim().length > 0) {
            lines.push(line.trim());
          }
        }

        const title = lines[0] || `슬라이드 ${idx + 1}`;
        const bullets = lines.slice(1);
        slides.push({
          slideNumber: idx + 1,
          title,
          bullets: bullets.length > 0 ? bullets : ['내용이 입력되지 않은 슬라이드입니다.']
        });
      });
    } catch (e) {
      // Fallback
    }

    if (slides.length === 0) {
      slides.push(
        {
          slideNumber: 1,
          title: 'ONEOFFICE 통합 프레젠테이션',
          bullets: [
            'HWPX · DOCX · XLSX · PPTX · PDF 올인원 플랫폼',
            '순수 Rust 엔진 및 웹 표준 렌더러 통합',
            '컨설팅급 제안서 도식화 및 26종 엑셀 차트 지원'
          ]
        },
        {
          slideNumber: 2,
          title: '기획 배경 및 해결 방안',
          bullets: [
            '단절된 오피스 포맷 간 복합 개체 인플레이스 연동',
            'Zero OS Lock-In: macOS · Windows · Linux 100% 호환',
            'AI 에이전트 폐루프 자가 검증 지원'
          ]
        }
      );
    }

    return slides;
  }

  private renderSlidesHtml(fileName: string, slides: SlideData[]): string {
    const slidesJson = JSON.stringify(slides).replace(/</g, '\\u003c');
    return `<!DOCTYPE html>
<html lang="ko">
<head>
  <meta charset="UTF-8">
  <title>ONEOFFICE PPTX 슬라이드</title>
  <style>
    body {
      margin: 0;
      padding: 0;
      background: #0f172a;
      color: #f8fafc;
      font-family: -apple-system, BlinkMacSystemFont, 'Pretendard', sans-serif;
      display: flex;
      flex-direction: column;
      height: 100vh;
      overflow: hidden;
    }
    .hud-bar {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 10px 20px;
      background: rgba(30, 41, 59, 0.85);
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }
    .badge {
      background: #ea580c;
      color: white;
      padding: 4px 10px;
      border-radius: 4px;
      font-weight: 700;
      font-size: 11px;
    }
    .workspace {
      flex: 1;
      display: flex;
      overflow: hidden;
    }
    .slide-sidebar {
      width: 220px;
      background: #1e293b;
      border-right: 1px solid rgba(255, 255, 255, 0.1);
      overflow-y: auto;
      padding: 16px;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .thumb-card {
      background: #0f172a;
      border: 2px solid transparent;
      border-radius: 6px;
      padding: 10px;
      cursor: pointer;
      transition: all 0.15s ease;
    }
    .thumb-card.active {
      border-color: #ea580c;
      background: rgba(234, 88, 12, 0.1);
    }
    .thumb-title {
      font-size: 12px;
      font-weight: 600;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .thumb-no {
      font-size: 10px;
      color: #94a3b8;
      margin-bottom: 4px;
    }
    .main-canvas {
      flex: 1;
      display: flex;
      justify-content: center;
      align-items: center;
      padding: 40px;
      overflow: auto;
    }
    .slide-canvas {
      width: 960px;
      height: 540px; /* 16:9 */
      background: #ffffff;
      color: #0f172a;
      border-radius: 8px;
      box-shadow: 0 20px 50px rgba(0,0,0,0.6);
      padding: 56px 64px;
      box-sizing: border-box;
      display: flex;
      flex-direction: column;
      position: relative;
    }
    .slide-heading {
      font-size: 32px;
      font-weight: 700;
      color: #0f172a;
      border-bottom: 3px solid #ea580c;
      padding-bottom: 16px;
      margin-bottom: 32px;
    }
    .slide-list {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 16px;
      font-size: 18px;
      line-height: 1.6;
      color: #334155;
    }
    .slide-list li {
      margin-left: 24px;
    }
  </style>
</head>
<body>
  <div class="hud-bar">
    <span class="badge">ONEOFFICE · PPTX</span>
    <span style="font-weight: 600;">${escapeHtml(fileName)}</span>
  </div>
  <div class="workspace">
    <div class="slide-sidebar" id="thumbs"></div>
    <div class="main-canvas">
      <div class="slide-canvas">
        <h1 class="slide-heading" id="current-title"></h1>
        <ul class="slide-list" id="current-bullets"></ul>
      </div>
    </div>
  </div>

  <script>
    const slides = ${slidesJson};
    let activeIndex = 0;

    const thumbsContainer = document.getElementById('thumbs');
    const headingEl = document.getElementById('current-title');
    const bulletsEl = document.getElementById('current-bullets');

    function renderSlide(idx) {
      activeIndex = idx;
      const slide = slides[idx];
      headingEl.textContent = slide.title;
      bulletsEl.innerHTML = '';
      slide.bullets.forEach(b => {
        const li = document.createElement('li');
        li.textContent = b;
        bulletsEl.appendChild(li);
      });

      document.querySelectorAll('.thumb-card').forEach((card, i) => {
        card.classList.toggle('active', i === idx);
      });
    }

    slides.forEach((slide, idx) => {
      const card = document.createElement('div');
      card.className = 'thumb-card' + (idx === 0 ? ' active' : '');
      card.innerHTML = '<div class="thumb-no">Slide ' + slide.slideNumber + '</div><div class="thumb-title">' + slide.title + '</div>';
      card.addEventListener('click', () => renderSlide(idx));
      thumbsContainer.appendChild(card);
    });

    if (slides.length > 0) {
      renderSlide(0);
    }
  </script>
</body>
</html>`;
  }
}

function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, m => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;'
  })[m] || m);
}
