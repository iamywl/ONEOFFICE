---
name: parallel-subagent-sprint
description: >-
  Orchestrate fast, conflict-free parallel agile sprints across ONEOFFICE using the 9 specialized
  subagents, strict write-scope isolation, Workspace share worktrees, and mandatory adversarial QA gates.
  Use this skill whenever executing multi-component implementation tasks in ONEOFFICE.
---

# Parallel Subagent Sprint Workflow (9-Agent Squad)

Use this skill to execute ONEOFFICE agile sprints rapidly without merge conflicts, context bloat, or unverified claims.

## 1. Wave 0 — Contract Lock (`oneoffice-planner`)
Before spawning write-capable developer subagents concurrently, confirm shared Rust/JSON schemas (`DiagramSpec`, `CalcTablePayload`, `ChartUpsertSpec`, `PdfSignatureSpec`, MCP tool schemas) in `docs/MASTER_ARCHITECTURE_PLAN.md`.

## 2. Disjoint Write-Scope Map (Zero File Collisions)
Never assign two concurrent subagents overlapping write paths:
- **`01` `oneoffice-planner`**: `docs/MASTER_ARCHITECTURE_PLAN.md` (Read-Only on all code)
- **`02` `oneoffice-rust-core`**: `crates/oneoffice-bridge/**`, `OPEN_EXCELL/crates/**`, `OPEN_WORD/crates/**`
- **`03` `oneoffice-diagram-engine`**: `crates/oneoffice-diagram/**`
- **`04` `oneoffice-hwpx-bridge`**: `crates/oneoffice-hwpx-core/**`, `hanul-hwpx/src/core/**`, `hanul-hwpx/src/storage.ts`
- **`05` `oneoffice-pdf-engineer`**: `crates/oneoffice-pdf/**`
- **`06` `oneoffice-studio-ux`**: `crates/oneoffice-ui/**`, `hanul-hwpx/media/**`, `hanul-hwpx/src/html.ts`, `OPEN_PPTX/src/webview/**`
- **`07` `oneoffice-mcp-architect`**: `crates/oneoffice-mcp/**`, `hanul-hwpx/src/mcp/**`, `hanul-hwpx/src/cli.ts`
- **`08` `oneoffice-release-governor`**: `extensions/oneoffice-vscode/**`, `apps/oneoffice-desktop/**`, `packaging/**`, `.github/workflows/**`
- **`09` `oneoffice-qa-auditor`**: `tests/**`, `scripts/qa/**`, `docs/QA_VERIFICATION_MATRIX.md`

When two tasks touch the same sub-repository, pass `Workspace: "share"` to `invoke_subagent` so each subagent works in an isolated worktree before merging.

## 3. 4-Wave Parallel Execution Cadence
1. **Wave 1 — Core Engines (4x Parallel)**: Run `02` (Calc/Chart/Font), `03` (Proposal Diagram DSL), `04` (HWPX Compound Core), and `05` (PDF Viewer & Signer) simultaneously.
2. **Wave 2 — Surface & AI Protocol (2x Parallel)**: Run `06` (In-Place Studio UX + PDF Sign Pad) and `07` (Unified `oneoffice-mcp`) simultaneously.
3. **Wave 3 — Packaging & Targets**: Run `08` to bundle the unified VS Code `.vsix` and cross-platform native desktop targets (`macOS` / `Windows` / `Linux`).
4. **Wave 4 — Independent Adversarial QA Gate**: Run `09` (`oneoffice-qa-auditor`) with `ground-truth-qa-gate`. Update `docs/QA_VERIFICATION_MATRIX.md` and `README.md` showcase captures only after 100% pass.

## 4. Git Submodule 2-Step Commit Pipeline
Whenever subagents touch files within sub-repositories (`hanul-hwpx`, `OPEN_WORD`, `OPEN_EXCELL`, `OPEN_PPTX`), follow the mandatory 2-step commit procedure:
1. **Submodule Commit**: Commit changes inside the submodule directory and push to its remote (`git commit -m "<type>(<scope>): [<TASK-ID>] <desc>" && git push origin main`).
2. **Root Pointer Commit**: Return to repository root, stage the updated submodule pointer and `docs/QA_VERIFICATION_MATRIX.md`, and commit at the root level (`git commit -m "chore(<scope>): [<TASK-ID>] bump submodule pointer with verified QA"`).

## 5. Autonomous Loop & Self-Healing Protocol
When executing sprints under the `"개발진행해"` instruction:
1. **Self-Healing**: If builds fail during testing, diagnose the exact compiler/lint errors and attempt up to 3 automated self-fixes before requesting user intervention.
2. **Keep Going**: After `oneoffice-qa-auditor` verifies a task and passes the 6-tier gate, immediately proceed to the next incomplete task in `docs/MASTER_ARCHITECTURE_PLAN.md` without halting.
3. **Commit Hygiene**: Never commit broken builds, never commit binary build artifacts (`*.vsix`, `*.dmg`, `*.msi`, `target/`), and always commit attribution updates in `ATTRIBUTION.md` alongside any new assets.
