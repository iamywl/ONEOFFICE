import { build } from 'esbuild';
import { mkdir, access } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

async function main() {
  console.log('[ONEOFFICE] VS Code 올인원 확장 빌드 시작...');
  
  await mkdir(path.join(__dirname, 'dist'), { recursive: true });
  await access(path.join(__dirname, 'vendor/studio/index.html'));
  await access(path.join(__dirname, 'media/host.js'));

  await build({
    entryPoints: [path.join(__dirname, 'src/extension.ts')],
    bundle: true,
    platform: 'node',
    format: 'cjs',
    external: ['vscode'],
    outfile: path.join(__dirname, 'dist/extension.js'),
    sourcemap: true,
    logLevel: 'info',
  });

  console.log('✅ [ONEOFFICE] dist/extension.js 번들링 완료 (실제 5대 오피스 엔진 통합)');
}

main().catch((err) => {
  console.error('❌ [ONEOFFICE] 빌드 실패:', err);
  process.exit(1);
});
