---
name: ground-truth-qa-gate
description: >-
  Run the 6-tier empirical verification gate for ONEOFFICE across HWPX, DOCX, XLSX, PPTX, and PDF
  (including PDF signature round-trip, VSIX Extension Host E2E, macOS/Windows/Linux checks, and Light/Dark
  WCAG contrast captures). Use before marking any feature complete or updating showcase screenshots.
---

# Ground-Truth QA Gate Runbook

Never mark an ONEOFFICE task complete based on compilation alone. Execute all applicable tiers below and commit real screenshot proofs registered in `ATTRIBUTION.md` and `docs/QA_VERIFICATION_MATRIX.md`.

## Tier 1: Cross-Platform Static Safety & Crash-Freedom
- **Rust (`OPEN_WORD`, `OPEN_EXCELL`, root `crates/`, `apps/`)**:
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets -- -D warnings` (zero `unwrap_used`, `expect_used`, `panic`, `todo`)
  - `cargo test --workspace` + `cargo check --target wasm32-unknown-unknown`
- **TypeScript (`hanul-hwpx`, `OPEN_PPTX`, `extensions/oneoffice-vscode`)**:
  - `npm --prefix hanul-hwpx run check && npm --prefix hanul-hwpx test`
  - `npm --prefix OPEN_PPTX run compile`

## Tier 2: 5-Format Lossless Round-Trip & Calc/PDF Integrity
- **HWPX (KS X 6101)**: Verify `mimetype` uncompressed, `Contents/content.hpf` manifest completeness, `<hp:p><hp:run><hp:tbl>` hierarchy, valid `cellAddr` grid, and lossless re-open of formulas, charts, and `DiagramSpec`.
- **XLSX / DOCX / PPTX**: Run `gridcraft-cli check <file> --json` (`< 1e-9` calc oracle tolerance) and verify embedded chart `.xlsx` preservation.
- **PDF Signature & Stamp (`oneoffice-pdf`)**: Apply freehand Bezier signature + white-bg-removed stamp (`removeWhiteBg: true`) -> save PDF -> reload in `craft-pdf` -> assert 100% original text extraction, intact links/outlines, and clean alpha compositing (zero black/white bounding box artifacts).

## Tier 3: Proposal Diagram Geometry (`Zero Overflow / Zero Collision`)
- Validate Korean proposal infographics (`DiagramSpec`): `overflow_count == 0`, `collision_count == 0`, and connector port attachment within `<= 1.5 pt`.

## Tier 4: VS Code VSIX & Native Desktop Runtime Gate
- Test `oneoffice-vscode` via `@vscode/test-electron` across `.hwpx`, `.docx`, `.xlsx`, `.pptx`, and `.pdf` custom editors.
- Test Native Desktop (`OneOffice.app` on macOS + cross-platform Linux/Windows CI targets) via `--control` loopback RPC and `ui.screenshot`.

## Tier 5 & 6: Dual-Theme WCAG Contrast, Hangul IME & MCP Autopilot
- Capture 2x Retina screenshots in **both `light` and `dark` themes**, inspect rendered PNGs via `read_file` to verify white paper canvas, `>= 94%` modal opacity, `>= 4.5:1` contrast (`>= 7.0:1` modals), Hangul IME composition integrity, zero selection traps, and full MCP E2E workflow (`compose diagram` -> `verify_page` -> `export pdf` -> `pdf.sign`).

## Tier 7: Verification Promotion & Commit Standard
- Only when all applicable Tiers are 100% satisfied:
  - Update `docs/QA_VERIFICATION_MATRIX.md` status from `🟡 Sprint Target` / `🔴 Bug` to `✅ Verified`.
  - Commit using: `qa(matrix): [<TASK-ID>] promote to verified with screenshot proof` (or `test(oracle): [<TASK-ID>] add adversarial regression fixture`).
  - Never commit verification claims without corresponding log artifacts and screenshots committed to disk.
