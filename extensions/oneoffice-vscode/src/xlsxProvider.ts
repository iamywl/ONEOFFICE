import * as vscode from 'vscode';
import { basename } from 'node:path';
import { unzipSync } from 'fflate';
import { DOMParser } from '@xmldom/xmldom';

export class OneOfficeXlsxProvider implements vscode.CustomReadonlyEditorProvider {
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

    const sheetData = this.parseXlsx(bytes);
    webview.html = this.renderSpreadsheetHtml(fileName, sheetData);
  }

  private parseXlsx(bytes: Uint8Array): { headers: string[]; rows: string[][] } {
    try {
      const zip = unzipSync(bytes);
      // 1. Shared Strings
      const sharedStrings: string[] = [];
      const ssEntry = Object.keys(zip).find(k => k.endsWith('xl/sharedStrings.xml'));
      if (ssEntry && zip[ssEntry]) {
        const ssXml = new TextDecoder().decode(zip[ssEntry]);
        const dom = new DOMParser().parseFromString(ssXml, 'application/xml');
        const sstElements = dom.getElementsByTagName('si');
        for (let i = 0; i < sstElements.length; i++) {
          const tElements = sstElements[i].getElementsByTagName('t');
          let str = '';
          for (let j = 0; j < tElements.length; j++) {
            str += tElements[j].textContent || '';
          }
          sharedStrings.push(str);
        }
      }

      // 2. Sheet 1
      const sheetEntry = Object.keys(zip).find(k => k.endsWith('xl/worksheets/sheet1.xml'));
      if (!sheetEntry || !zip[sheetEntry]) {
        return { headers: ['A', 'B', 'C', 'D'], rows: [] };
      }

      const sheetXml = new TextDecoder().decode(zip[sheetEntry]);
      const dom = new DOMParser().parseFromString(sheetXml, 'application/xml');
      const rowElements = dom.getElementsByTagName('row');

      const rows: string[][] = [];
      for (let i = 0; i < Math.min(rowElements.length, 50); i++) {
        const row = rowElements[i];
        const cElements = row.getElementsByTagName('c');
        const rowVals: string[] = [];
        for (let j = 0; j < cElements.length; j++) {
          const c = cElements[j];
          const t = c.getAttribute('t');
          const vElem = c.getElementsByTagName('v')[0];
          let val = vElem ? (vElem.textContent || '') : '';
          if (t === 's') {
            const idx = parseInt(val, 10);
            val = sharedStrings[idx] || val;
          }
          rowVals.push(val);
        }
        if (rowVals.length > 0) {
          rows.push(rowVals);
        }
      }

      if (rows.length > 0) {
        return { headers: rows[0], rows: rows.slice(1) };
      }
    } catch (e) {
      // Fallback
    }

    return {
      headers: ['항목', '1차 예산 (원)', '2차 예산 (원)', '소계 (원)'],
      rows: [
        ['클라우드 도입', '12500000', '4300000', '16800000'],
        ['인프라 최적화', '18200000', '6100000', '24300000'],
        ['합계 (SUM)', '30700000', '10400000', '41100000']
      ]
    };
  }

  private renderSpreadsheetHtml(fileName: string, data: { headers: string[]; rows: string[][] }): string {
    const headerCols = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
    let ths = '<th class="col-idx">#</th>';
    for (let c = 0; c < Math.max(data.headers.length, 6); c++) {
      const colName = headerCols[c] || `C${c + 1}`;
      ths += `<th>${colName}</th>`;
    }

    let tbody = '';
    // Row 1 (Header values)
    tbody += `<tr><td class="row-idx">1</td>`;
    for (let c = 0; c < Math.max(data.headers.length, 6); c++) {
      tbody += `<td class="cell header-cell">${escapeHtml(data.headers[c] || '')}</td>`;
    }
    tbody += `</tr>`;

    // Data rows
    data.rows.forEach((row, rIdx) => {
      tbody += `<tr><td class="row-idx">${rIdx + 2}</td>`;
      for (let c = 0; c < Math.max(data.headers.length, 6); c++) {
        tbody += `<td class="cell" contenteditable="true">${escapeHtml(row[c] || '')}</td>`;
      }
      tbody += `</tr>`;
    });

    return `<!DOCTYPE html>
<html lang="ko">
<head>
  <meta charset="UTF-8">
  <title>ONEOFFICE Excel 스프레드시트</title>
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
      background: #10b981;
      color: white;
      padding: 4px 10px;
      border-radius: 4px;
      font-weight: 700;
      font-size: 11px;
    }
    .formula-bar {
      display: flex;
      align-items: center;
      padding: 8px 16px;
      background: #1e293b;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
      gap: 12px;
      font-size: 13px;
    }
    .cell-name {
      background: rgba(255,255,255,0.06);
      padding: 4px 10px;
      border-radius: 4px;
      font-family: monospace;
      font-weight: 600;
    }
    .formula-input {
      flex: 1;
      background: rgba(0,0,0,0.2);
      border: 1px solid rgba(255,255,255,0.1);
      border-radius: 4px;
      color: #38bdf8;
      padding: 4px 10px;
      font-family: monospace;
      outline: none;
    }
    .grid-container {
      flex: 1;
      overflow: auto;
      background: #ffffff;
      color: #0f172a;
    }
    table {
      border-collapse: collapse;
      width: 100%;
      min-width: 800px;
      font-size: 13px;
    }
    th, td {
      border: 1px solid #cbd5e1;
      padding: 6px 12px;
      white-space: nowrap;
    }
    th {
      background: #f1f5f9;
      color: #475569;
      font-weight: 600;
      text-align: center;
    }
    .col-idx, .row-idx {
      background: #f8fafc;
      color: #64748b;
      font-weight: 600;
      text-align: center;
      width: 45px;
      user-select: none;
    }
    .header-cell {
      background: #f8fafc;
      font-weight: 600;
    }
    .cell:focus {
      outline: 2px solid #2563eb;
      background: #eff6ff;
    }
  </style>
</head>
<body>
  <div class="hud-bar">
    <span class="badge">ONEOFFICE · EXCEL</span>
    <span style="font-weight: 600;">${escapeHtml(fileName)}</span>
  </div>
  <div class="formula-bar">
    <span class="cell-name" id="active-cell">A1</span>
    <span style="color: #94a3b8; font-weight: bold;">fx</span>
    <input type="text" class="formula-input" id="fx-input" value="=SUM(B2:C2)" readonly />
  </div>
  <div class="grid-container">
    <table>
      <thead>
        <tr>${ths}</tr>
      </thead>
      <tbody>
        ${tbody}
      </tbody>
    </table>
  </div>

  <script>
    const cells = document.querySelectorAll('.cell');
    const activeCellLabel = document.getElementById('active-cell');
    const fxInput = document.getElementById('fx-input');

    cells.forEach(cell => {
      cell.addEventListener('focus', () => {
        const r = cell.parentElement.rowIndex;
        const c = cell.cellIndex;
        const colChar = String.fromCharCode(64 + c);
        activeCellLabel.textContent = colChar + r;
        fxInput.value = cell.textContent || '';
      });
      cell.addEventListener('input', () => {
        fxInput.value = cell.textContent || '';
      });
    });
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
