---
type: log
title: "PLAN-008 — Update Log"
description: "Chronological log of changes to the PLAN-008 bundle."
---

# PLAN-008 — Update Log

## History

- **2026-09-04** — Plan created from the validated issue #307 specification (StR-008, US-021, FR-076..083, NFR-018..020, TC-1400..1470, SR-128..135) with seven tasks: template core and conformance contract, schema source and emit pipeline, manifest semantic block, skeletons and rendered suite, public baseline and governance tree, render gate and drift check, review and pull request. The plan generalizes the two completed hand migrations (spec-objects-business#4 `567e5c4`, spec-artifacts-iso#34 `6686f11`) and encodes neither module's vocabulary.
- **2026-09-04** — TASK-045..TASK-050 done. `templates/semantic-module/` renders all three variants; `tests/semantic-module-template.test.ts` (59 rows on `make test`) checks each rendered tree against `templates/semantic-module/conformance.yaml`; `make template-gate` renders, installs the pinned toolchain, emits, re-checks byte-for-byte and runs the rendered suite — 31 rows pass per variant with zero skipped. `quire validate` over each rendered `spec/` tree exits clean with zero grammar warnings. One emitter trap was found and closed by instantiating rather than reading: `seal-object-schemas: true` seals the open marker models used as `@contains` predicates, so every identity check silently matched nothing; sealing is now per-model via `...Record<never>`. Twelve matrix rows remain `🚧` — implemented refusals and absent-tool diagnostics that no run yet provokes — filed as `agent-ix/quoin#346`. The copied emit driver is filed as `agent-ix/quoin#345`.
