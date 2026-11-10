import * as vscode from 'vscode';
import { basename } from 'node:path';
import { randomUUID } from 'node:crypto';
import { editorHtml } from './html';
import { atomicSave, digest, formatOf, validateBytes, type Format } from './storage';

export class OneOfficeHwpxDocument implements vscode.CustomDocument {
  readonly disposed = new vscode.EventEmitter<void>();
  readonly onDidDispose = this.disposed.event;
  panel?: vscode.WebviewPanel;
  version = 0;
  savedVersion = 0;
  ready = false;
  pending = new Map<string, { resolve: (value: any) => void; reject: (error: Error) => void; timer: NodeJS.Timeout }>();

  constructor(
    readonly uri: vscode.Uri,
    public bytes: Uint8Array,
    public originalHash: string | null,
    readonly restored: boolean
  ) {}

  dispose() {
    for (const request of this.pending.values()) {
      clearTimeout(request.timer);
      request.reject(Error('HWPX 문서 편집기가 닫혔습니다.'));
    }
    this.pending.clear();
    this.disposed.fire();
    this.disposed.dispose();
  }
}

export class OneOfficeHwpxProvider implements vscode.CustomEditorProvider<OneOfficeHwpxDocument> {
  readonly documents = new Map<string, OneOfficeHwpxDocument>();
  activeDocument?: OneOfficeHwpxDocument;
  private readonly emitter = new vscode.EventEmitter<vscode.CustomDocumentContentChangeEvent<OneOfficeHwpxDocument>>();
  readonly onDidChangeCustomDocument = this.emitter.event;

  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly log: vscode.OutputChannel
  ) {
    context.subscriptions.push(this.emitter);
  }

  getActiveDocument(): OneOfficeHwpxDocument | undefined {
    return this.activeDocument || Array.from(this.documents.values())[0];
  }

  async openCustomDocument(
    uri: vscode.Uri,
    openContext: vscode.CustomDocumentOpenContext
  ): Promise<OneOfficeHwpxDocument> {
    const original = uri.scheme === 'untitled' ? new Uint8Array() : await vscode.workspace.fs.readFile(uri);
    const bytes = openContext.backupId
      ? await vscode.workspace.fs.readFile(vscode.Uri.parse(openContext.backupId))
      : openContext.untitledDocumentData ?? original;

    if (bytes.length) {
      validateBytes(bytes, formatOf(uri.path));
    }

    const document = new OneOfficeHwpxDocument(
      uri,
      bytes,
      original.length ? digest(original) : null,
      !!openContext.backupId
    );
    this.documents.set(uri.toString(), document);
    document.onDidDispose(() => {
      this.documents.delete(uri.toString());
      if (this.activeDocument === document) this.activeDocument = undefined;
    });
    return document;
  }

  async resolveCustomEditor(document: OneOfficeHwpxDocument, panel: vscode.WebviewPanel): Promise<void> {
    document.panel = panel;
    if (panel.active) this.activeDocument = document;
    panel.onDidChangeViewState(e => {
      if (e.webviewPanel.active) this.activeDocument = document;
      else if (this.activeDocument === document) this.activeDocument = undefined;
    });

    panel.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this.context.extensionUri, 'vendor'),
        vscode.Uri.joinPath(this.context.extensionUri, 'media')
      ]
    };

    const receiver = panel.webview.onDidReceiveMessage(async (message: any) => {
      try {
        if (!message || typeof message.type !== 'string') return;
        if (message.type === 'response') {
          const pending = document.pending.get(message.id);
          if (!pending) return;
          clearTimeout(pending.timer);
          document.pending.delete(message.id);
          message.error ? pending.reject(Error(String(message.error))) : pending.resolve(message.result);
        } else if (message.type === 'ready') {
          document.ready = true;
          await this.request(document, 'load', {
            bytes: Array.from(document.bytes),
            name: basename(document.uri.path),
            restored: document.restored
          });
          if (document.restored) {
            document.version++;
            this.emitter.fire({ document });
          }
        } else if (message.type === 'changed' && Number.isSafeInteger(message.version)) {
          document.version = message.version;
          this.emitter.fire({ document });
        } else if (message.type === 'save' || message.type === 'saveAs') {
          panel.reveal(undefined, true);
          const prevSaved = document.savedVersion;
          const prevUri = document.uri.toString();
          await vscode.commands.executeCommand(
            message.type === 'saveAs' || document.uri.scheme === 'untitled'
              ? 'workbench.action.files.saveAs'
              : 'workbench.action.files.save'
          );
          if (document.ready) {
            if (message.type === 'save' && document.uri.scheme !== 'untitled' && document.version === document.savedVersion) {
              await this.request(document, 'saved', { name: basename(document.uri.path), version: document.version }).catch(() => {});
            } else if (document.savedVersion === prevSaved && document.uri.toString() === prevUri) {
              await this.request(document, 'saveCancelled').catch(() => {});
            }
          }
        } else if (message.type === 'new') {
          await vscode.commands.executeCommand('oneoffice.newHwpx');
        } else if (message.type === 'error') {
          this.log.appendLine(String(message.error));
          void vscode.window.showErrorMessage(`ONEOFFICE: ${message.error}`);
        }
      } catch (error) {
        this.report(error);
      }
    });

    panel.onDidDispose(() => {
      receiver.dispose();
      document.panel = undefined;
      document.ready = false;
      for (const p of document.pending.values()) {
        clearTimeout(p.timer);
        p.reject(Error('편집기가 닫혔습니다.'));
      }
      document.pending.clear();
    });

    panel.webview.html = editorHtml(
      panel.webview,
      this.context.extensionUri,
      this.context.extensionMode !== vscode.ExtensionMode.Production
    );
  }

  private report(error: unknown) {
    this.log.appendLine(String(error));
    void vscode.window.showErrorMessage(`ONEOFFICE: ${error instanceof Error ? error.message : String(error)}`);
  }

  public async request(document: OneOfficeHwpxDocument, method: string, params: unknown = {}): Promise<any> {
    if (!document.panel || !document.ready) throw Error('문서 편집기가 아직 준비되지 않았습니다.');
    const id = randomUUID();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        document.pending.delete(id);
        reject(Error(`${method}: 편집기 응답 시간이 초과되었습니다.`));
      }, 60000);
      document.pending.set(id, { resolve, reject, timer });
      void document.panel!.webview.postMessage({ type: 'request', id, method, params }).then(ok => {
        if (!ok) {
          clearTimeout(timer);
          document.pending.delete(id);
          reject(Error('편집기에 요청을 전달하지 못했습니다.'));
        }
      });
    });
  }

  private async snapshot(document: OneOfficeHwpxDocument, format: Format, token: vscode.CancellationToken): Promise<{ bytes: Uint8Array; version: number }> {
    if (token.isCancellationRequested) throw new vscode.CancellationError();
    if (document.version === document.savedVersion && document.bytes.length && format === formatOf(document.uri.path)) {
      return { bytes: document.bytes, version: document.version };
    }
    const result = await this.request(document, 'export', { format });
    if (token.isCancellationRequested) throw new vscode.CancellationError();
    if (!Array.isArray(result.bytes) || !Number.isSafeInteger(result.version) || result.bytes.some((n: unknown) => !Number.isInteger(n) || (n as number) < 0 || (n as number) > 255)) {
      throw Error('편집기가 유효한 문서 데이터를 반환하지 않았습니다.');
    }
    const bytes = new Uint8Array(result.bytes);
    validateBytes(bytes, format);
    return { bytes, version: result.version };
  }

  async saveCustomDocument(document: OneOfficeHwpxDocument, token: vscode.CancellationToken): Promise<void> {
    if (document.uri.scheme === 'untitled') throw Error('새 문서를 다른 이름으로 저장해 주세요.');
    await this.save(document, document.uri, token);
  }

  async saveCustomDocumentAs(document: OneOfficeHwpxDocument, destination: vscode.Uri, token: vscode.CancellationToken): Promise<void> {
    await this.save(document, destination, token);
  }

  private async save(document: OneOfficeHwpxDocument, destination: vscode.Uri, token: vscode.CancellationToken): Promise<void> {
    if (destination.scheme !== 'file') throw Error('로컬 파일 저장을 지원합니다.');
    const format = formatOf(destination.path);
    try {
      const snapshot = await this.snapshot(document, format, token);
      if (token.isCancellationRequested) throw new vscode.CancellationError();
      const same = destination.toString() === document.uri.toString();
      await atomicSave(destination.fsPath, snapshot.bytes, same ? document.originalHash : undefined, true);
      if (same) {
        document.bytes = snapshot.bytes;
        document.originalHash = digest(snapshot.bytes);
        document.savedVersion = snapshot.version;
      }
      await this.request(document, 'saved', { name: basename(destination.path), version: snapshot.version });
      if (document.version > snapshot.version) {
        setTimeout(() => this.emitter.fire({ document }), 0);
      }
      this.log.appendLine(`Saved ${destination.fsPath} (${snapshot.bytes.length} bytes)`);
    } catch (error) {
      if (document.ready) void this.request(document, 'saveFailed', { error: error instanceof Error ? error.message : String(error) }).catch(() => {});
      throw error;
    }
  }

  async revertCustomDocument(document: OneOfficeHwpxDocument): Promise<void> {
    const bytes = await vscode.workspace.fs.readFile(document.uri);
    validateBytes(bytes, formatOf(document.uri.path));
    await this.request(document, 'load', { bytes: Array.from(bytes), name: basename(document.uri.path) });
    document.bytes = bytes;
    document.originalHash = digest(bytes);
    document.version = 0;
    document.savedVersion = 0;
  }

  async backupCustomDocument(
    document: OneOfficeHwpxDocument,
    context: vscode.CustomDocumentBackupContext,
    token: vscode.CancellationToken
  ): Promise<vscode.CustomDocumentBackup> {
    const { bytes } = await this.snapshot(document, formatOf(document.uri.path), token);
    await vscode.workspace.fs.writeFile(context.destination, bytes);
    return {
      id: context.destination.toString(),
      delete: async () => {
        try {
          await vscode.workspace.fs.delete(context.destination);
        } catch {}
      }
    };
  }
}
