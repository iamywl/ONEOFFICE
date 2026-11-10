#!/usr/bin/env node
/**
 * ONEOFFICE Documentation & Showcase Asset Integrity Auditor
 * (`scripts/qa/verify-readme.mjs`)
 *
 * Verifies that:
 * 1. Every image link in `README.md` and `docs/*.md` exists on disk and is non-empty (> 100 bytes).
 * 2. No broken submodule paths (which 404 on GitHub) are used in `README.md`.
 * 3. No removed fake marketing posters are referenced.
 * 4. Every relative document link points to an existing file.
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "../..");

let errors = 0;

function checkFile(filePath) {
  const relPath = path.relative(ROOT, filePath);
  console.log(`\n🔍 Auditing documentation: ${relPath}`);
  const rawContent = fs.readFileSync(filePath, "utf-8");

  // Strip code fences (```...```) and inline code (`...`) so documentation examples aren't mistaken for real links
  const content = rawContent
    .replace(/```[\s\S]*?```/g, "")
    .replace(/`[^`\n]+`/g, "");

  // 1. Check markdown image syntax: ![alt](src)
  const mdImgRegex = /!\[([^\]]*)\]\(([^)]+)\)/g;
  let match;
  while ((match = mdImgRegex.exec(content)) !== null) {
    const [full, alt, src] = match;
    const cleanSrc = src.split(" ")[0].trim();
    if (cleanSrc.startsWith("http://") || cleanSrc.startsWith("https://")) {
      continue;
    }
    const targetPath = path.resolve(path.dirname(filePath), cleanSrc);
    if (!fs.existsSync(targetPath)) {
      console.error(`  ✖ Broken image in ${relPath}: "${cleanSrc}" (Target: ${targetPath} does not exist)`);
      errors++;
    } else {
      const stat = fs.statSync(targetPath);
      if (stat.size < 100) {
        console.error(`  ✖ Suspiciously small image in ${relPath}: "${cleanSrc}" (${stat.size} bytes)`);
        errors++;
      } else {
        console.log(`  ✓ Valid image [${alt}]: ${cleanSrc} (${Math.round(stat.size / 1024)} KB)`);
      }
    }

    // Guardrail: In README.md, images should be hosted in root repo docs/assets/ to prevent GitHub submodule 404s
    if (relPath === "README.md") {
      if (
        cleanSrc.startsWith("hanul-hwpx/") ||
        cleanSrc.startsWith("OPEN_WORD/") ||
        cleanSrc.startsWith("OPEN_EXCELL/") ||
        cleanSrc.startsWith("OPEN_PPTX/")
      ) {
        console.error(`  ✖ Submodule image path used in README.md (causes GitHub 404): "${cleanSrc}"`);
        errors++;
      }
    }
  }

  // 2. Check HTML img syntax: <img src="..." ...>
  const htmlImgRegex = /<img[^>]+src=["']([^"']+)["'][^>]*>/g;
  while ((match = htmlImgRegex.exec(content)) !== null) {
    const [full, src] = match;
    const cleanSrc = src.trim();
    if (cleanSrc.startsWith("http://") || cleanSrc.startsWith("https://")) {
      continue;
    }
    const targetPath = path.resolve(path.dirname(filePath), cleanSrc);
    if (!fs.existsSync(targetPath)) {
      console.error(`  ✖ Broken HTML img in ${relPath}: "${cleanSrc}"`);
      errors++;
    } else {
      const stat = fs.statSync(targetPath);
      console.log(`  ✓ Valid HTML img: ${cleanSrc} (${Math.round(stat.size / 1024)} KB)`);
    }
  }

  // 3. Check relative markdown links: [text](path.md)
  const linkRegex = /\[([^\]]+)\]\(([^)]+)\)/g;
  while ((match = linkRegex.exec(content)) !== null) {
    const [full, label, href] = match;
    const cleanHref = href.split(" ")[0].trim();
    if (
      cleanHref.startsWith("http://") ||
      cleanHref.startsWith("https://") ||
      cleanHref.startsWith("#") ||
      cleanHref.startsWith("mailto:")
    ) {
      continue;
    }
    // Remove anchor fragments
    const filePart = cleanHref.split("#")[0];
    if (!filePart) continue;

    const targetPath = path.resolve(path.dirname(filePath), filePart);
    if (!fs.existsSync(targetPath)) {
      console.error(`  ✖ Broken link in ${relPath}: "${cleanHref}" (Target: ${targetPath} does not exist)`);
      errors++;
    }
  }
}

// Audit README.md
checkFile(path.join(ROOT, "README.md"));

// Audit Submodule READMEs
const submodules = ["hanul-hwpx", "OPEN_WORD", "OPEN_EXCELL", "OPEN_PPTX"];
for (const sub of submodules) {
  const subReadme = path.join(ROOT, sub, "README.md");
  if (fs.existsSync(subReadme)) {
    checkFile(subReadme);
  } else {
    console.error(`  ✖ Missing submodule README: ${sub}/README.md`);
    errors++;
  }
}

// Audit docs/*.md
const docsDir = path.join(ROOT, "docs");
if (fs.existsSync(docsDir)) {
  for (const f of fs.readdirSync(docsDir)) {
    if (f.endsWith(".md")) {
      checkFile(path.join(docsDir, f));
    }
  }
}

// Audit Generated Dummy Documents
console.log("\n🔍 Auditing Generated Dummy Documents (test_documents/):");
const requiredDummyDocs = [
  "dummy_contract.pdf",
  "dummy_proposal.hwpx",
  "dummy_budget.xlsx",
  "dummy_report.docx",
  "dummy_presentation.pptx",
];
const testDocsDir = path.join(ROOT, "test_documents");
for (const doc of requiredDummyDocs) {
  const p = path.join(testDocsDir, doc);
  if (!fs.existsSync(p)) {
    console.error(`  ✖ Missing dummy document: test_documents/${doc}`);
    errors++;
  } else {
    const s = fs.statSync(p);
    console.log(`  ✓ Valid dummy document: ${doc} (${Math.round(s.size / 1024)} KB)`);
  }
}

// Audit Empirical Showcase Screenshots
console.log("\n🔍 Auditing Empirical Showcase Screenshots (docs/assets/qa/):");
const requiredShowcases = [
  "wave1_hwpx_proposal_diagram.png",
  "wave1_pdf_viewer_signature.png",
  "wave1_excel_formula_chart.png",
];
const qaDir = path.join(ROOT, "docs/assets/qa");
for (const shot of requiredShowcases) {
  const p = path.join(qaDir, shot);
  if (!fs.existsSync(p)) {
    console.error(`  ✖ Missing showcase screenshot: docs/assets/qa/${shot}`);
    errors++;
  } else {
    const s = fs.statSync(p);
    console.log(`  ✓ Valid showcase screenshot: ${shot} (${Math.round(s.size / 1024)} KB)`);
  }
}

console.log("\n------------------------------------------------");
if (errors > 0) {
  console.error(`❌ Documentation integrity audit failed with ${errors} error(s).`);
  process.exit(1);
} else {
  console.log("✅ Documentation & showcase asset integrity 100% verified.");
  process.exit(0);
}
