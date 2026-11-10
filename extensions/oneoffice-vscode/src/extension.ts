import * as vscode from 'vscode';
import { OneOfficeHwpxProvider } from './hwpxProvider';
import { OneOfficeDocxProvider } from './docxProvider';
import { OneOfficeXlsxProvider } from './xlsxProvider';
import { OneOfficePptxProvider } from './pptxProvider';
import { OneOfficePdfProvider } from './pdfProvider';

/**
 * Activates the unified ONEOFFICE all-in-one extension.
 */
export function activate(context: vscode.ExtensionContext) {
  const log = vscode.window.createOutputChannel('ONEOFFICE Studio');
  context.subscriptions.push(log);
  log.appendLine('ONEOFFICE Studio is initializing...');

  // 1. HWPX/HWP Editor Provider (Full-featured WYSIWYG Korean Document Editor)
  const hwpxProvider = new OneOfficeHwpxProvider(context, log);
  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider('oneoffice.hwpxEditor', hwpxProvider, {
      supportsMultipleEditorsPerDocument: false,
      webviewOptions: { retainContextWhenHidden: true }
    })
  );

  // 2. DOCX Editor Provider
  const docxProvider = new OneOfficeDocxProvider(context);
  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider('oneoffice.docxEditor', docxProvider, {
      supportsMultipleEditorsPerDocument: false,
      webviewOptions: { retainContextWhenHidden: true }
    })
  );

  // 3. XLSX Spreadsheet Provider
  const xlsxProvider = new OneOfficeXlsxProvider(context);
  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider('oneoffice.xlsxEditor', xlsxProvider, {
      supportsMultipleEditorsPerDocument: false,
      webviewOptions: { retainContextWhenHidden: true }
    })
  );

  // 4. PPTX Presentation Provider
  const pptxProvider = new OneOfficePptxProvider(context);
  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider('oneoffice.pptxEditor', pptxProvider, {
      supportsMultipleEditorsPerDocument: false,
      webviewOptions: { retainContextWhenHidden: true }
    })
  );

  // 5. PDF Viewer & Electronic Signature Provider
  const pdfProvider = new OneOfficePdfProvider(context);
  context.subscriptions.push(
    vscode.window.registerCustomEditorProvider('oneoffice.pdfEditor', pdfProvider, {
      supportsMultipleEditorsPerDocument: false,
      webviewOptions: { retainContextWhenHidden: true }
    })
  );

  // 6. Registered Global Commands
  let hwpxSequence = 0;
  const openEditor = async (uri: vscode.Uri, viewType: string) => {
    await vscode.commands.executeCommand('vscode.openWith', uri, viewType);
  };

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.newHwpx', async () => {
      await openEditor(vscode.Uri.parse(`untitled:새 문서-${++hwpxSequence}.hwpx`), 'oneoffice.hwpxEditor');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.newDocx', async () => {
      await openEditor(vscode.Uri.parse(`untitled:새 문서-${++hwpxSequence}.docx`), 'oneoffice.docxEditor');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.newXlsx', async () => {
      await openEditor(vscode.Uri.parse(`untitled:새 통합문서-${++hwpxSequence}.xlsx`), 'oneoffice.xlsxEditor');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.newPptx', async () => {
      await openEditor(vscode.Uri.parse(`untitled:새 프레젠테이션-${++hwpxSequence}.pptx`), 'oneoffice.pptxEditor');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.signPdf', async () => {
      const files = await vscode.window.showOpenDialog({
        canSelectMany: false,
        filters: { 'PDF 문서': ['pdf'] }
      });
      if (files?.[0]) {
        await openEditor(files[0], 'oneoffice.pdfEditor');
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.insertDiagram', async () => {
      vscode.window.showInformationMessage('ONEOFFICE: 제안서 도식화(DiagramSpec) 서브캔버스를 삽입합니다.');
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('oneoffice.startMcpServer', async () => {
      vscode.window.showInformationMessage('ONEOFFICE: AI MCP 서버 (stdio)가 대기 중입니다.');
    })
  );

  log.appendLine('ONEOFFICE Studio initialized successfully with 5-format full engine.');
}

export function deactivate() {}
