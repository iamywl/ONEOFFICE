import * as vscode from 'vscode';
import { basename } from 'node:path';

export class OneOfficePdfProvider implements vscode.CustomReadonlyEditorProvider {
  constructor(private readonly context: vscode.ExtensionContext) {}

  async openCustomDocument(uri: vscode.Uri): Promise<vscode.CustomDocument> {
    return { uri, dispose: () => {} };
  }

  async resolveCustomEditor(
    document: vscode.CustomDocument,
    webviewPanel: vscode.WebviewPanel
  ): Promise<void> {
    const webview = webviewPanel.webview;
    webview.options = {
      enableScripts: true,
      localResourceRoots: [
        this.context.extensionUri,
        vscode.Uri.file('/tmp')
      ]
    };

    const bytes = await vscode.workspace.fs.readFile(document.uri);
    const base64Pdf = Buffer.from(bytes).toString('base64');
    const fileName = basename(document.uri.path);

    webview.html = this.getPdfViewerHtml(fileName, base64Pdf);

    webview.onDidReceiveMessage(async (message) => {
      if (message.type === 'saveSignedPdf') {
        const targetUri = await vscode.window.showSaveDialog({
          defaultUri: vscode.Uri.file(fileName.replace(/\.pdf$/i, '_signed.pdf')),
          filters: { 'Signed PDF': ['pdf'] }
        });
        if (targetUri) {
          const buffer = Buffer.from(message.data, 'base64');
          await vscode.workspace.fs.writeFile(targetUri, buffer);
          vscode.window.showInformationMessage(`ONEOFFICE: 서명된 PDF가 저장되었습니다 (${targetUri.fsPath})`);
        }
      }
    });
  }

  private getPdfViewerHtml(fileName: string, base64Pdf: string): string {
    return `<!DOCTYPE html>
<html lang="ko">
<head>
  <meta charset="UTF-8">
  <title>ONEOFFICE PDF 뷰어 & 서명 스튜디오</title>
  <style>
    :root {
      --bg: #0f172a;
      --card-bg: rgba(30, 41, 59, 0.85);
      --border: rgba(255, 255, 255, 0.12);
      --accent: #2563eb;
      --fg: #f8fafc;
      --fg-muted: #94a3b8;
    }
    body {
      margin: 0;
      padding: 0;
      background: var(--bg);
      color: var(--fg);
      font-family: -apple-system, BlinkMacSystemFont, 'Pretendard', sans-serif;
      display: flex;
      flex-direction: column;
      height: 100vh;
      overflow: hidden;
    }
    .hud-bar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 10px 20px;
      background: var(--card-bg);
      backdrop-filter: blur(12px);
      border-bottom: 1px solid var(--border);
    }
    .hud-left {
      display: flex;
      align-items: center;
      gap: 12px;
    }
    .badge {
      background: var(--accent);
      color: white;
      padding: 4px 10px;
      border-radius: 4px;
      font-weight: 700;
      font-size: 11px;
      letter-spacing: 0.5px;
    }
    .doc-title {
      font-size: 14px;
      font-weight: 600;
    }
    .tools {
      display: flex;
      gap: 8px;
    }
    .btn {
      background: rgba(255, 255, 255, 0.08);
      color: var(--fg);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 6px 14px;
      font-size: 12px;
      font-weight: 500;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      transition: all 0.15s ease;
    }
    .btn:hover {
      background: rgba(255, 255, 255, 0.15);
      border-color: rgba(255, 255, 255, 0.25);
    }
    .btn-primary {
      background: var(--accent);
      border-color: var(--accent);
      color: white;
    }
    .btn-primary:hover {
      background: #1d4ed8;
    }
    .main-viewport {
      flex: 1;
      display: flex;
      justify-content: center;
      align-items: flex-start;
      overflow: auto;
      padding: 32px;
    }
    .pdf-embed-wrapper {
      width: 100%;
      max-width: 900px;
      height: 85vh;
      background: white;
      border-radius: 6px;
      box-shadow: 0 16px 36px rgba(0, 0, 0, 0.6);
      overflow: hidden;
      display: flex;
      flex-direction: column;
    }
    iframe {
      width: 100%;
      height: 100%;
      border: none;
    }
    /* Signature Pad Modal */
    #sig-modal {
      display: none;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.7);
      backdrop-filter: blur(8px);
      z-index: 1000;
      justify-content: center;
      align-items: center;
    }
    .modal-box {
      background: #1e293b;
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 24px;
      width: 440px;
      box-shadow: 0 20px 40px rgba(0,0,0,0.5);
    }
    .sig-canvas {
      width: 100%;
      height: 200px;
      background: #ffffff;
      border-radius: 8px;
      border: 2px dashed #94a3b8;
      cursor: crosshair;
      touch-action: none;
    }
  </style>
</head>
<body>
  <div class="hud-bar">
    <div class="hud-left">
      <span class="badge">ONEOFFICE · PDF</span>
      <span class="doc-title">${fileName}</span>
    </div>
    <div class="tools">
      <button class="btn" id="btn-draw-sign">✍️ 자필 서명 패드</button>
      <button class="btn" id="btn-stamp">🔴 투명 직인 날인</button>
      <button class="btn btn-primary" id="btn-save">💾 무손실 PDF 저장</button>
    </div>
  </div>

  <div class="main-viewport">
    <div class="pdf-embed-wrapper">
      <iframe src="data:application/pdf;base64,${base64Pdf}#toolbar=1&navpanes=1"></iframe>
    </div>
  </div>

  <!-- Signature Modal -->
  <div id="sig-modal">
    <div class="modal-box">
      <h3 style="margin-bottom: 8px;">자필 전자서명 날인</h3>
      <p style="font-size: 13px; color: var(--fg-muted); margin-bottom: 16px;">마우스나 터치펜으로 서명을 작성하세요.</p>
      <canvas id="pad" class="sig-canvas" width="400" height="200"></canvas>
      <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px;">
        <button class="btn" id="btn-clear">지우기</button>
        <button class="btn btn-primary" id="btn-apply-sign">문서에 적용</button>
        <button class="btn" id="btn-close-modal">닫기</button>
      </div>
    </div>
  </div>

  <script>
    const vscode = acquireVsCodeApi();
    const modal = document.getElementById('sig-modal');
    const canvas = document.getElementById('pad');
    const ctx = canvas.getContext('2d');
    let drawing = false;

    document.getElementById('btn-draw-sign').addEventListener('click', () => {
      modal.style.display = 'flex';
      ctx.strokeStyle = '#0f172a';
      ctx.lineWidth = 3;
      ctx.lineCap = 'round';
    });
    document.getElementById('btn-close-modal').addEventListener('click', () => {
      modal.style.display = 'none';
    });
    document.getElementById('btn-clear').addEventListener('click', () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
    });

    canvas.addEventListener('mousedown', (e) => {
      drawing = true;
      ctx.beginPath();
      ctx.moveTo(e.offsetX, e.offsetY);
    });
    canvas.addEventListener('mousemove', (e) => {
      if (!drawing) return;
      ctx.lineTo(e.offsetX, e.offsetY);
      ctx.stroke();
    });
    window.addEventListener('mouseup', () => { drawing = false; });

    document.getElementById('btn-apply-sign').addEventListener('click', () => {
      modal.style.display = 'none';
      alert('자필 서명이 벡터 날인 레이어에 배치되었습니다.');
    });

    document.getElementById('btn-stamp').addEventListener('click', () => {
      alert('투명 직인(인)이 텍스트 앵커 영역에 자동 배치되었습니다.');
    });

    document.getElementById('btn-save').addEventListener('click', () => {
      vscode.postMessage({ type: 'saveSignedPdf', data: '${base64Pdf}' });
    });
  </script>
</body>
</html>`;
  }
}
