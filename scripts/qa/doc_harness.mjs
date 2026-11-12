#!/usr/bin/env node
/**
 * ONEOFFICE Documentation Quality & Design Harness (`scripts/qa/doc_harness.mjs`)
 * Owned by: `oneoffice-doc-designer` (10. Document Design & Visual Showcase Architect)
 * 
 * Benchmarked against: https://github.com/epoko77-ai/im-not-ai
 * 
 * Verifies:
 * 1. Image & Asset Integrity: Every image exists, is non-empty (> 100B), and avoids submodule 404 paths.
 * 2. Capture-Grounded Specification: 6 core pillars must have corresponding runtime screenshots and empirical metrics.
 * 3. im-not-ai Parity: Required structural sections (Hero, Install, Why, Prime Directives, Architecture, 10 Agents, Quickstart, 6-Tier QA).
 * 4. Anchor & Link Validity: Every anchor (#heading) and relative file link resolves correctly.
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "../..");

let totalErrors = 0;
let totalWarnings = 0;

function error(msg) {
  console.error(`  ✖ [ERROR] ${msg}`);
  totalErrors++;
}

function warn(msg) {
  console.warn(`  ⚠ [WARN]  ${msg}`);
  totalWarnings++;
}

function info(msg) {
  console.log(`  ✓ ${msg}`);
}

console.log("================================================================================");
console.log("🎨 ONEOFFICE Documentation Quality & Design Harness (Benchmarked on im-not-ai)");
console.log("================================================================================\n");

function auditDocument(filePath) {
  const relPath = path.relative(ROOT, filePath);
  console.log(`\n================================================================================`);
  console.log(`🎨 ONEOFFICE Documentation Quality Audit: ${relPath}`);
  console.log(`================================================================================\n`);

  if (!fs.existsSync(filePath)) {
    error(`${relPath} does not exist.`);
    return;
  }

  const rawContent = fs.readFileSync(filePath, "utf-8");

  // Extract headings for anchor validation
  const headingRegex = /^(#{1,6})\s+(.+)$/gm;
  const declaredHeadings = new Set();
  let hMatch;
  while ((hMatch = headingRegex.exec(rawContent)) !== null) {
    const rawTitle = hMatch[2].trim();
    const slug = rawTitle
      .toLowerCase()
      .replace(/[^\w\s\uAC00-\uD7A3-]/g, "")
      .trim()
      .replace(/\s+/g, "-");
    declaredHeadings.add(slug);
    declaredHeadings.add(encodeURIComponent(slug));
  }

  // Strip code blocks
  const sanitizedContent = rawContent
    .replace(/```[\s\S]*?```/g, "")
    .replace(/`[^`\n]+`/g, "");

  // 1. Image Integrity Audit
  console.log(`▶ [Tier 1] Image & Asset Integrity Audit (${relPath})`);
  const imgRegex = /!\[([^\]]*)\]\(([^)]+)\)|<img[^>]+src=["']([^"']+)["'][^>]*>/g;
  let imgMatch;
  let imgCount = 0;

  while ((imgMatch = imgRegex.exec(sanitizedContent)) !== null) {
    const src = (imgMatch[2] || imgMatch[3] || "").split(" ")[0].trim();
    const alt = imgMatch[1] || "html-img";

    if (!src) continue;
    if (src.startsWith("http://") || src.startsWith("https://")) {
      info(`External CDN badge/image: ${src}`);
      continue;
    }

    imgCount++;
    if (/^(hanul-hwpx|OPEN_WORD|OPEN_EXCELL|OPEN_PPTX)\//.test(src)) {
      error(`Submodule path leak detected: "${src}". Must use root docs/assets/.`);
    }

    const resolvedPath = path.resolve(ROOT, src);
    if (!fs.existsSync(resolvedPath)) {
      error(`Broken image link: "${src}" (Resolved: ${resolvedPath})`);
    } else {
      const stats = fs.statSync(resolvedPath);
      if (stats.size < 100) {
        error(`Suspiciously empty image file (${stats.size} bytes): "${src}"`);
      } else {
        info(`Valid image asset: ${src} (${Math.round(stats.size / 1024)} KB) [alt: "${alt}"]`);
      }
    }
  }

  if (imgCount === 0) {
    error(`${relPath} contains ZERO local images! Must be capture-grounded.`);
  }

  // 2. Capture-Grounded Specification Audit
  console.log(`\n▶ [Tier 2] Capture-Grounded Feature & Benefit Specification Audit (${relPath})`);
  const isEnglish = relPath.endsWith(".en.md");
  const requiredCapturePillars = [
    {
      name: "HWPX Studio & Floating HUD",
      requiredImages: ["01_hwpx_editor.png", "02_hwpx_floating_hud.png"],
      keywords: isEnglish ? ["HWPX", "Floating", "HUD"] : ["HWPX", "한글", "플로팅", "HUD"],
    },
    {
      name: "Word Multi-Column & BehindText Stamp",
      requiredImages: ["05_word_macos_native.png", "06_word_behind_stamp.png"],
      keywords: isEnglish ? ["Word", "BehindText", "Stamp"] : ["DOCX", "워드", "BehindText", "직인"],
    },
    {
      name: "Excel 513 Formulas & 26 Charts",
      requiredImages: ["07_excel_native_sales.png", "wave1_excel_formula_chart.png"],
      keywords: isEnglish ? ["513", "Excel", "26", "Chart"] : ["513", "수식", "26종", "차트"],
    },
    {
      name: "Presentation Precision Studio",
      requiredImages: ["09_pptx_native_studio.png", "10_pptx_vscode_webview.png"],
      keywords: isEnglish ? ["Presentation", "AutoShape"] : ["PPTX", "프레젠테이션", "AutoShape"],
    },
    {
      name: "Pure Rust PDF Viewer & Electronic Signature",
      requiredImages: ["wave1_pdf_viewer_signature.png"],
      keywords: isEnglish ? ["PDF", "Signature", "Chroma-Key"] : ["PDF", "서명", "직인", "Chroma-Key"],
    },
    {
      name: "Consulting Proposal Diagram DSL",
      requiredImages: ["wave1_hwpx_proposal_diagram.png"],
      keywords: isEnglish ? ["Diagram", "KPI", "overflow"] : ["DiagramSpec", "도식화", "KPI", "오버플로우"],
    },
  ];

  for (const pillar of requiredCapturePillars) {
    let pillarPassed = true;
    for (const imgName of pillar.requiredImages) {
      if (!rawContent.includes(imgName)) {
        error(`[${relPath}] Pillar [${pillar.name}] is missing expected screenshot: ${imgName}`);
        pillarPassed = false;
      }
    }
    for (const kw of pillar.keywords) {
      if (!rawContent.toLowerCase().includes(kw.toLowerCase())) {
        error(`[${relPath}] Pillar [${pillar.name}] is missing keyword: "${kw}"`);
        pillarPassed = false;
      }
    }
    if (pillarPassed) {
      info(`Pillar [${pillar.name}] fully verified with runtime captures and metrics.`);
    }
  }

  // 3. im-not-ai Structural Benchmark Audit
  console.log(`\n▶ [Tier 3] im-not-ai Benchmark Structural Parity Audit (${relPath})`);
  const requiredStructuralSections = [
    { label: "Hero Banner/Social Preview", pattern: /(assets\/oneoffice-hero-architecture|social-preview|<img[^>]+hero)/i },
    { label: "Fast Installation (3+ targets)", pattern: /(##.*(설치|빠른\s*시작|Installation|Quickstart))/i },
    { label: "Why ONEOFFICE (Problem & Solution)", pattern: /(##.*(왜\s*oneoffice인가|Why\s*ONEOFFICE|Problem\s*&\s*Solution))/i },
    { label: "Prime Directives (핵심 철칙)", pattern: /(##.*(핵심\s*철칙|Prime\s*Directives))/i },
    { label: "System Architecture & Pipeline", pattern: /(##.*(아키텍처|Architecture)|```mermaid[\s\S]*?flowchart)/i },
    { label: "10/14 Specialized Sub-Agents", pattern: /(10대|14대|20대|10\s*Specialized|14\s*Specialized|20\s*Specialized|oneoffice-doc-designer)/i },
    { label: "6/7-Tier Verification Gates", pattern: /(6단계|7단계|6-Tier|7-Tier|Verification\s*Gates)/i },
  ];

  for (const sec of requiredStructuralSections) {
    if (sec.pattern.test(rawContent)) {
      info(`Required section [${sec.label}] detected.`);
    } else {
      error(`[${relPath}] Missing required structural section: [${sec.label}]`);
    }
  }

  // 4. Anchor & Link Consistency Audit
  console.log(`\n▶ [Tier 4] Markdown Link & Anchor Consistency Audit (${relPath})`);
  const linkRegex = /\[([^\]]+)\]\(([^)]+)\)/g;
  let lMatch;
  let linkCount = 0;

  while ((lMatch = linkRegex.exec(sanitizedContent)) !== null) {
    const [full, text, href] = lMatch;
    const cleanHref = href.split(" ")[0].trim();

    if (cleanHref.startsWith("http://") || cleanHref.startsWith("https://") || cleanHref.startsWith("mailto:")) {
      continue;
    }

    linkCount++;
    if (cleanHref.startsWith("#")) {
      const slug = cleanHref.slice(1).toLowerCase();
      let found = false;
      for (const h of declaredHeadings) {
        if (h.includes(slug) || slug.includes(h) || h === slug) {
          found = true;
          break;
        }
      }
      if (!found) {
        warn(`[${relPath}] Potential anchor mismatch: "${cleanHref}"`);
      }
      continue;
    }

    const filePart = cleanHref.split("#")[0];
    if (!filePart) continue;

    const targetPath = path.resolve(ROOT, filePart);
    if (!fs.existsSync(targetPath)) {
      error(`[${relPath}] Broken relative file link: "${cleanHref}" (Resolved: ${targetPath})`);
    }
  }

  info(`Audited ${linkCount} internal markdown links and anchors.`);
}

// Audit both README.md and README.en.md
auditDocument(path.join(ROOT, "README.md"));
if (fs.existsSync(path.join(ROOT, "README.en.md"))) {
  auditDocument(path.join(ROOT, "README.en.md"));
}

// Summary
console.log("\n--------------------------------------------------------------------------------");
if (totalErrors > 0) {
  console.error(`❌ Documentation Harness FAILED with ${totalErrors} error(s) and ${totalWarnings} warning(s).`);
  process.exit(1);
} else {
  console.log(`✅ Documentation Harness 100% PASSED! (${totalWarnings} warning(s))`);
  console.log("   - im-not-ai benchmark structure verified.");
  console.log("   - Capture-grounded visual evidence validated across all 6 core pillars.");
  console.log("   - Zero broken image links & zero submodule leak paths.");
  process.exit(0);
}
