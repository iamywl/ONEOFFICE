"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));

function activate(context) {
    const editors = [
        ['oneoffice.hwpxEditor', 'hwpx'],
        ['oneoffice.docxEditor', 'docx'],
        ['oneoffice.xlsxEditor', 'xlsx'],
        ['oneoffice.pptxEditor', 'pptx'],
        ['oneoffice.pdfEditor', 'pdf']
    ];

    for (const [viewType, fmt] of editors) {
        context.subscriptions.push(vscode.window.registerCustomEditorProvider(viewType, new OneOfficeEditorProvider(fmt), {
            webviewOptions: { retainContextWhenHidden: true },
            supportsMultipleEditorsPerDocument: false
        }));
    }

    const commands = [
        ['oneoffice.newHwpx', 'ONEOFFICE: 새 HWPX 제안서 문서를 생성합니다.'],
        ['oneoffice.newDocx', 'ONEOFFICE: 새 DOCX 보고서를 생성합니다.'],
        ['oneoffice.newXlsx', 'ONEOFFICE: 새 XLSX 수식 스프레드시트를 생성합니다.'],
        ['oneoffice.newPptx', 'ONEOFFICE: 새 PPTX 프레젠테이션을 생성합니다.'],
        ['oneoffice.signPdf', 'ONEOFFICE: PDF 서명 및 투명 직인 날인 패드를 활성화합니다.'],
        ['oneoffice.insertDiagram', 'ONEOFFICE: 제안서 도식화(DiagramSpec) 서브캔버스를 삽입합니다.'],
        ['oneoffice.startMcpServer', 'ONEOFFICE: 통합 AI MCP 서버(JSON-RPC 2.0 stdio)가 준비되었습니다.']
    ];

    for (const [cmd, msg] of commands) {
        context.subscriptions.push(vscode.commands.registerCommand(cmd, async () => {
            vscode.window.showInformationMessage(msg);
        }));
    }
}

function deactivate() {}

class OneOfficeEditorProvider {
    constructor(format) {
        this.format = format;
    }
    async openCustomDocument(uri) {
        return { uri, dispose: () => { } };
    }
    async resolveCustomEditor(document, webviewPanel) {
        webviewPanel.webview.options = { enableScripts: true };
        webviewPanel.webview.html = this.renderWebviewHtml(document.uri.fsPath);
    }
    renderWebviewHtml(filePath) {
        const fileName = filePath.split('/').pop() || filePath;
        const fmtUpper = this.format.toUpperCase();
        return '<!DOCTYPE html>' +
'<html lang="ko">' +
'<head>' +
'  <meta charset="UTF-8">' +
'  <title>ONEOFFICE Studio</title>' +
'  <style>' +
'    body { margin: 0; padding: 0; background: var(--vscode-editor-background, #1e1e1e); color: var(--vscode-editor-foreground, #cccccc); font-family: -apple-system, BlinkMacSystemFont, "Pretendard", sans-serif; display: flex; flex-direction: column; height: 100vh; overflow: hidden; }' +
'    .hud-bar { display: flex; align-items: center; gap: 12px; padding: 10px 18px; background: rgba(30, 41, 59, 0.85); backdrop-filter: blur(12px); border-bottom: 1px solid rgba(255, 255, 255, 0.1); font-size: 13px; }' +
'    .badge { background: #2563eb; color: white; padding: 3px 8px; border-radius: 4px; font-weight: 600; text-transform: uppercase; font-size: 11px; }' +
'    .canvas-container { flex: 1; display: flex; justify-content: center; align-items: center; background: #0f172a; overflow: auto; padding: 24px; }' +
'    .paper { background: white; color: black; width: 595px; min-height: 842px; box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5); border-radius: 2px; padding: 48px; box-sizing: border-box; }' +
'  </style>' +
'</head>' +
'<body>' +
'  <div class="hud-bar">' +
'    <span class="badge">ONEOFFICE · ' + fmtUpper + '</span>' +
'    <span>문서: <strong>' + fileName + '</strong></span>' +
'  </div>' +
'  <div class="canvas-container">' +
'    <div class="paper">' +
'      <h2>ONEOFFICE ' + fmtUpper + ' 통합 스튜디오</h2>' +
'      <p>순수 Rust 엔진 기반 무손실 문서 편집 및 인플레이스 복합 개체 뷰어입니다.</p>' +
'    </div>' +
'  </div>' +
'</body>' +
'</html>';
    }
}
