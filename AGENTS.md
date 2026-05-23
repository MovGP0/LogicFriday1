# Agent Instructions

- do not stage or commit any changes without user approval
- check the file IssueTracking.md for information on issue tracking with bd (beads)

## Understand Anything Knowledge Graph

- The repository has an Understand Anything graph in `.understand-anything/knowledge-graph.json`.
- Supporting graph state lives in `.understand-anything/meta.json` and `.understand-anything/fingerprints.json`.
- Validation/enrichment reports are stored in `.understand-anything/review-enrichment.json` and `.understand-anything/review-decompiled-reference.json`.
- To view the graph, run the `understand-anything:understand-dashboard` skill from the repository root. The dashboard must be opened with the tokenized URL printed by Vite.
- To refresh the graph, run the `understand-anything:understand` skill from the repository root and keep `.understand-anything/.understandignore` reviewed before scanning.

## Decompiled Reference Graph Notes

- `Decompiled/FunctionsTracking.csv` is the authoritative source for decompiled `FUN_*` responsibilities, evidence, and confidence.
- Decompiled function file nodes under `Decompiled/logicfriday_decompiled_functions/*.c` should use the matching CSV row for their summary and source metadata when one exists.
- Function-level graph nodes should use IDs of the form `function:<file-path>:<function-name>`.
- Direct references between tracked `FUN_*` routines, including callback/function-pointer references such as `DialogBoxParamA(..., FUN_0040b055, ...)`, should be represented as `calls` edges.
- File-level decompiled references should also receive `calls` edges so the `Decompiled Reference` layer is meaningful when viewed at file-node level.
- All graph edges should include `direction: "forward"` to satisfy graph validation.
