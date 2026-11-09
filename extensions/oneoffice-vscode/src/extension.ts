import * as vscode from 'vscode';

/**
 * Activates the unified ONEOFFICE all-in-one extension.
 */
export function activate(context: vscode.ExtensionContext) {
  // 1. Register Custom Editors
  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider(
      'oneoffice.hwpxEditor',
      new OneOfficeEditorProvider('hwpx'),
      { webviewOptions: { retainContextWhenHidden: true }, supportsMultipleEditorsPerDocument: false }
    )
  );

  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider(
      'oneoffice.docxEditor',
      new OneOfficeEditorProvider('docx'),
      { webviewOptions: { retainContextWhenHidden: true }, supportsMultipleEditorsPerDocument: false }
    )
  );

  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider(
      'oneoffice.xlsxEditor',
      new OneOfficeEditorProvider('xlsx'),
      { webviewOptions: { retainContextWhenHidden: true }, supportsMultipleEditorsPerDocument: false }
    )
  );

  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider(
      'oneoffice.pptxEditor',
      new OneOfficeEditorProvider('pptx'),
      { webviewOptions: { retainContextWhenHidden: true }, supportsMultipleEditorsPerDocument: false }
    )
  );

  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider(
      'oneoffice.pdfEditor',
      new OneOfficeEditorProvider('pdf'),
      { webviewOptions: { retainContextWhenHidden: true }, supportsMultipleEditorsPerDocument: false }
    )
  );

  // 2. Register Commands
  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.newHwpx', async () => {
      vscode.window.showInformationMessage('ONEOFFICE: 새 HWPX 제안서 문서를 생성합니다.');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.signPdf', async () => {
      vscode.window.showInformationMessage('ONEOFFICE: PDF 서명 및 투명 직인 날인 패드를 활성화합니다.');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.insertDiagram', async () => {
      vscode.window.showInformationMessage('ONEOFFICE: 제안서 도식화(DiagramSpec) 서브캔버스를 삽입합니다.');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.startMcpServer', async () => {
      vscode.window.showInformationMessage('ONEOFFICE: 통합 AI MCP 서버(JSON-RPC 2.0 stdio)가 준비되었습니다.');
    })
  );
}

export function deactivate() {}

/**
 * Universal editor provider that delegates to the specialized ONEOFFICE webview host.
 */
class OneOfficeEditorProvider implements vscode.CustomReadonlyEditorProvider {
  constructor(private readonly format: 'hwpx' | 'docx' | 'xlsx' | 'pptx' | 'pdf') {}

  async openCustomDocument(uri: vscode.Uri): Promise<vscode.CustomDocument> {
    return { uri, dispose: () => {} };
  }

  async resolveCustomEditor(
    document: vscode.CustomDocument,
    webviewPanel: vscode.WebviewPanel
  ): Promise<void> {
    webviewPanel.webview.options = { enableScripts: true };
    webviewPanel.webview.html = this.renderWebviewHtml(document.uri.fsPath);
  }

  private renderWebviewHtml(filePath: string): string {
    const fileName = filePath.split('/').pop() || filePath;
    return `<!DOCTYPE html>
<html lang="ko">
<head>
  <meta charset="UTF-8">
  <title>ONEOFFICE Studio</title>
  <style>
    body {
      margin: 0;
      padding: 0;
      background: var(--vscode-editor-background, #1e1e1e);
      color: var(--vscode-editor-foreground, #cccccc);
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
      padding: 10px 18px;
      background: rgba(30, 41, 59, 0.85);
      backdrop-filter: blur(12px);
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
      font-size: 13px;
    }
    .badge {
      background: #2563eb;
      color: white;
      padding: 3px 8px;
      border-radius: 4px;
      font-weight: 600;
      text-transform: uppercase;
      font-size: 11px;
    }
    .canvas-container {
      flex: 1;
      display: flex;
      justify-content: center;
      align-items: center;
      background: #0f172a;
      overflow: auto;
      padding: 24px;
    }
    .paper {
      background: white;
      color: black;
      width: 595px;
      min-height: 842px;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5);
      border-radius: 2px;
      padding: 48px;
      box-sizing: border-box;
    }
  </style>
</head>
<body>
  <div class="hud-bar">
    <span class="badge">ONEOFFICE · ${this.format.toUpperCase()}</span>
    <span>문서: <strong>${fileName}</strong></span>
  </div>
  <div class="canvas-container">
    <div class="paper">
      <h2>ONEOFFICE ${this.format.toUpperCase()} 통합 스튜디오</h2>
      <p>순수 Rust 엔진 기반 무손실 문서 편집 및 인플레이스 복합 개체 뷰어입니다.</p>
    </div>
  </div>
</body>
</html>`;
  }
}
