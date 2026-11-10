import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// 1. Verify package.json manifests
const pkgPath = path.join(__dirname, 'package.json');
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
assert.strictEqual(pkg.name, 'oneoffice-vscode');
assert.strictEqual(pkg.version, '0.1.0-alpha.1');
assert.strictEqual(pkg.contributes.customEditors.length, 5);
assert.strictEqual(pkg.contributes.commands.length, 7);

// 2. Verify dist/extension.js bundle
const distPath = path.join(__dirname, 'dist/extension.js');
assert.ok(fs.existsSync(distPath), 'dist/extension.js must exist');
const code = fs.readFileSync(distPath, 'utf8');
assert.ok(code.includes('activate'), 'activate function must be exported');
assert.ok(code.includes('oneoffice.hwpxEditor'), 'hwpx editor must be registered');
assert.ok(code.includes('oneoffice.docxEditor'), 'docx editor must be registered');
assert.ok(code.includes('oneoffice.xlsxEditor'), 'xlsx editor must be registered');
assert.ok(code.includes('oneoffice.pptxEditor'), 'pptx editor must be registered');
assert.ok(code.includes('oneoffice.pdfEditor'), 'pdf editor must be registered');
assert.ok(code.includes('OneOfficeHwpxProvider'), 'real HWPX provider must be compiled in');

// 3. Verify real assets linkage (vendor studio and media host)
assert.ok(fs.existsSync(path.join(__dirname, 'vendor/studio/index.html')), 'vendor studio index.html must exist');
assert.ok(fs.existsSync(path.join(__dirname, 'media/host.js')), 'media host.js must exist');
assert.ok(fs.existsSync(path.join(__dirname, 'media/host.css')), 'media host.css must exist');

console.log('✅ [ONEOFFICE] All 5-format VS Code extension tests passed successfully!');
