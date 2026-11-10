import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { unzipSync } from 'fflate';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const sampleCandidate = path.join(__dirname, '../../hanul-hwpx/templates/report.hwpx');

assert.ok(fs.existsSync(sampleCandidate), 'Sample report.hwpx must exist');
const bytes = fs.readFileSync(sampleCandidate);
assert.ok(bytes.length > 0, 'Bytes must not be empty');

const zip = unzipSync(new Uint8Array(bytes));
const keys = Object.keys(zip);

const hasHeader = keys.some(k => /header\.xml$/i.test(k));
const hasSection = keys.some(k => /section\d+\.xml$/i.test(k));
const hasHpf = keys.some(k => /content\.hpf$/i.test(k));

assert.ok(hasHeader, 'HWPX must have header.xml');
assert.ok(hasSection, 'HWPX must have section*.xml');
assert.ok(hasHpf, 'HWPX must have content.hpf');

console.log(`✅ [HWPX Real Verification] Successfully verified ${sampleCandidate} (${bytes.length} bytes, ${keys.length} ZIP entries)`);
