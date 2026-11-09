---
name: proposal-diagram-authoring
description: >-
  Design, generate, and edit consulting-grade Korean proposal infographics (도식화), Excel-formula tables,
  and linked vector charts inside HWPX, DOCX, and PPTX documents using ONEOFFICE DiagramSpec and MCP tools.
  Use when the user asks to create or visually structure reports, RFPs, business plans, or one-pagers.
---

# Proposal Diagram & Visual Report Authoring Guide

When transforming plain prose or data into persuasive Korean proposals (제안서·사업계획서·기획보고서), avoid walls of text. Decompose each key section into **Lead Message + Structured Visual Diagram (`DiagramSpec`) + Data Evidence (Calc Table / Chart)**.

## 1. Canonical Consulting One-Pager Anatomy (Reference Pattern)
Model executive pages on the proven 4-zone vertical hierarchy:
1. **Header & Governing Message**:
   - Top metadata bar (`Project / Team` left, `Section Category` right).
   - Accent Eyebrow (`00 / WHY NOW`) + Bold Action Title (`기획 배경 및 필요성`) + 2-line Governing Takeaway summarizing *why change is needed and what solves it*.
2. **Evidence & Problem Comparison Zone (`Grid` of `ComparisonCard`s)**:
   - Pair 2–3 equal-height rounded cards (`문제 A · 전환 장벽` vs `문제 B · 정착 장벽`).
   - Inside each card stack:
     - **Pill Badge + Category** -> **Short Punchy Headline** (`옮기기 어렵다`)
     - **KPI Hero Callout** (`4.6개`, `16%` in 28–32pt primary accent + 2-line context + `[ref]` superscript)
     - **Evidence Bullets** (3 concise bullets with **bold numbers/keywords** and right-aligned `[ref]` tags)
     - **Tinted Takeaway Box (`시사점`)** pinned to card bottom so horizontal baselines align across columns.
3. **Directional Synthesis Connectors (`GluedConnector`)**:
   - Route clean vertical/elbow arrows (`↓`) from problem cards directly into the solution container below.
4. **Solution Architecture / Process Zone (`HeroContainer` + `ProcessStepChain`)**:
   - High-contrast container (e.g., `#0F2942` deep navy with white typography) stating the core value proposition, followed by numbered step cards (`①` -> `②` -> `③` -> `④`) with page cross-reference badges (`2p`..`5p`) and chevron (`>`) connectors.
5. **Citation & Running Footer**:
   - Compact 2-column numbered source list (`[1] 기관명, 보고서명 (연도)`) + page counter (`01 / 05`).

## 2. Choosing the Right Visual Pattern
- **2–3 Parallel Pain Points -> Unified Solution**: Use `problem-comparison-to-solution`.
- **Sequential Workflow / Service Journey**: Use `step-process-pipeline` (`①..⑤` with inputs/outputs).
- **Before vs After Value Leap**: Use `as-is-to-be-bridge` (left gray pain card -> center catalyst -> right accent value card).
- **System / AI Platform Architecture**: Use `layered-architecture-stack` (Infrastructure -> Core Engine -> MCP/Agent -> UX).
- **Quantitative Trend / Market Share / Budget**: Pair `oneoffice.table.upsert` (Excel formulas `SUM`, `CAGR`, `% share`) with `oneoffice.chart.upsert` (`Combo`, `ColumnStacked`, `Waterfall`, `Doughnut`).
- **Execution Timeline (추진 일정)**: Use `gantt-milestone-roadmap`.

## 3. Mandatory Self-Check Protocol
After inserting or updating any diagram, table, or chart in `.hwpx` / `.docx` / `.pptx`:
1. Run `oneoffice.visual.verify_page` (or render preview PNG/SVG).
2. Inspect the rendered image to verify zero Korean line-wrap orphans (e.g., trailing `다.` alone on a new line when tightening copy or box width solves it), zero overlapping boxes, and clear visual hierarchy.
