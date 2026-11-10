#!/usr/bin/env node
/**
 * ONEOFFICE 6-Tier Empirical QA Gate Runner (`scripts/qa/run-all-gates.mjs`)
 *
 * Executes:
 *   Tier 1: Rust static safety (`unsafe_code = "forbid"`, clippy zero-panic) & WASM target check
 *   Tier 2: 5-format lossless round-trip & calc/PDF signature integrity tests
 *   Tier 3: Proposal diagram geometry invariant tests (`overflow_count == 0`, `collision_count == 0`)
 *   Tier 4: VS Code extension & Native desktop verification checks
 *   Tier 5: Dual-theme screenshot & artifact presence audit
 *   Tier 6: Unified `oneoffice-mcp` end-to-end autopilot test suite
 */

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "../..");

const env = {
  ...process.env,
  PATH: `${process.env.HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${process.env.PATH ?? ""}`,
};

function runStep(label, cmd, args, cwd = ROOT) {
  console.log(`\n▶ [${label}] ${cmd} ${args.join(" ")}`);
  const res = spawnSync(cmd, args, { cwd, env, stdio: "inherit" });
  if (res.status !== 0) {
    console.error(`✖ FAILED: ${label} (exit ${res.status})`);
    process.exit(res.status ?? 1);
  }
  console.log(`✓ PASSED: ${label}`);
}

const mode = process.argv[2] ?? "all";

if (mode === "all" || mode === "rust") {
  runStep(
    "Tier 1A: Rust Workspace Clippy (-D warnings, zero unwrap/panic)",
    "cargo",
    ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]
  );
  runStep("Tier 1B: Rust WASM32 Target Check", "cargo", [
    "check",
    "-p",
    "oneoffice-bridge",
    "-p",
    "oneoffice-diagram",
    "-p",
    "oneoffice-hwpx-core",
    "-p",
    "oneoffice-pdf",
    "--target",
    "wasm32-unknown-unknown",
  ]);
  runStep("Tier 2/3/6: Rust Workspace Unit & Integration Tests", "cargo", [
    "test",
    "--workspace",
  ]);
  runStep("Tier 4B: Korean Font Zero-Tofu & Desktop Tests", "cargo", [
    "test",
    "-p",
    "oneoffice-ui",
    "-p",
    "oneoffice-desktop",
  ]);
  runStep("Tier 2C: 5-Format Dummy Document Round-Trip Pipeline", "node", [
    "scripts/qa/dummy-docs-pipeline.mjs",
  ]);
  runStep("Tier 5B: Documentation & Showcase Asset Integrity", "node", [
    "scripts/qa/verify-readme.mjs",
  ]);
}

if (mode === "all" || mode === "hwpx") {
  if (fs.existsSync(path.join(ROOT, "hanul-hwpx/node_modules"))) {
    runStep("Tier 1C: hanul-hwpx TypeScript Check", "npm", [
      "--prefix",
      "hanul-hwpx",
      "run",
      "check",
    ]);
    runStep("Tier 2B: hanul-hwpx Unit & Round-Trip Tests", "npm", [
      "--prefix",
      "hanul-hwpx",
      "test",
    ]);
  }
}

if (mode === "all" || mode === "vscode") {
  const extPkg = path.join(ROOT, "extensions/oneoffice-vscode/package.json");
  if (fs.existsSync(extPkg)) {
    runStep("Tier 4A: oneoffice-vscode Compile & Test", "npm", [
      "--prefix",
      "extensions/oneoffice-vscode",
      "test",
    ]);
  }
}

console.log("\n✅ All requested ONEOFFICE QA Gate tiers passed.");
