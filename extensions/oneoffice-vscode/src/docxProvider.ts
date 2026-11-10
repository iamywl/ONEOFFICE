import * as vscode from 'vscode';
import { basename } from 'node:path';
import { unzipSync } from 'fflate';
import { DOMParser } from '@xmldom/xmldom';

export class OneOfficeDocxProvider implements vscode.CustomReadonlyEditorProvider {
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

    let docHtml = '';
    try {
      const zip = unzipSync(bytes);
      const documentXmlEntry = Object.keys(zip).find(k => k.endsWith('word/document.xml'));
      if (documentXmlEntry && zip[documentXmlEntry]) {
        const xmlStr = new TextDecoder().decode(zip[documentXmlEntry]);
        const dom = new DOMParser().parseFromString(xmlStr, 'application/xml');
        const paragraphs = dom.getElementsByTagName('w:p');

        for (let i = 0; i < paragraphs.length; i++) {
          const p = paragraphs[i];
          const texts = p.getElementsByTagName('w:t');
          let pText = '';
          for (let j = 0; j < texts.length; j++) {
            pText += texts[j].textContent || '';
          }
          if (pText.trim().length > 0) {
            // Heading or body
            const isHeading = pText.length < 50 && (pText.startsWith('제') || pText.startsWith('1.') || pText.startsWith('2.'));
            if (isHeading) {
              docHtml += `<h3 style="margin-top: 18px; color: #1e293b;">${escapeHtml(pText)}</h3>`;
            } else {
              docHtml += `<p style="margin: 8px 0; line-height: 1.7; color: #334155;">${escapeHtml(pText)}</p>`;
            }
          }
        }
      }
    } catch (e) {
      docHtml = `<div style="color: red;">문서 파싱 중 오류: ${String(e)}</div>`;
    }

    if (!docHtml) {
      docHtml = `<p style="color: #64748b; font-style: italic;">내용이 비어 있거나 텍스트를 파싱할 수 없는 문서입니다.</p>`;
    }

    webview.html = `<!DOCTYPE html>
<html lang="ko">
<head>
  <meta charset="UTF-8">
  <title>ONEOFFICE DOCX 뷰어</title>
  <style>
    body {
      margin: 0;
      padding: 0;
      background: #0f172a;
      color: #f8fafc;
      font-family: -apple-system, BlinkMacSystemFont, 'Apple SD Gothic Neo', 'Pretendard', sans-serif;
      display: flex;
      flex-direction: column;
      height: 100vh;
    }
    .hud-bar {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 10px 20px;
      background: rgba(30, 41, 59, 0.85);
      backdrop-filter: blur(12px);
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }
    .badge {
      background: #2563eb;
      color: white;
      padding: 4px 10px;
      border-radius: 4px;
      font-weight: 700;
      font-size: 11px;
    }
    .viewport {
      flex: 1;
      overflow: auto;
      display: flex;
      justify-content: center;
      padding: 32px 16px;
    }
    .paper {
      background: white;
      width: 100%;
      max-width: 794px; /* A4 */
      min-height: 1123px;
      box-shadow: 0 16px 36px rgba(0,0,0,0.5);
      border-radius: 4px;
      padding: 72px 64px;
      box-sizing: border-box;
    }
  </style>
</head>
<body>
  <div class="hud-bar">
    <span class="badge">ONEOFFICE · DOCX</span>
    <span style="font-weight: 600;">${escapeHtml(fileName)}</span>
  </div>
  <div class="viewport">
    <div class="paper">
      ${docHtml}
    </div>
  </div>
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
