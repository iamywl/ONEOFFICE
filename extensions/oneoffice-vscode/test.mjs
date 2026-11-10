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

// 2. Verify dist/extension.js
const distPath = path.join(__dirname, 'dist/extension.js');
assert.ok(fs.existsSync(distPath), 'dist/extension.js must exist');
const code = fs.readFileSync(distPath, 'utf8');
assert.ok(code.includes('activate'), 'activate function must be exported');
assert.ok(code.includes('oneoffice.hwpxEditor'), 'hwpx editor must be registered');
assert.ok(code.includes('oneoffice.pdfEditor'), 'pdf editor must be registered');

console.log('All extension tests passed: package.json manifests and dist/extension.js verified.');
