---
id: SR-136
title: "Gap analysis of PLAN-008, the semantic-module cookiecutter"
type: SpecReview
analysis: gap-analysis
scope: "PLAN-008; StR-008; US-021; FR-076..FR-083; NFR-018..NFR-020; TC-1400..TC-1470"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/PLAN-008"
    type: "reviews"
  - target: "ix://agent-ix/quoin/TM-001"
    type: "references"
---

## Summary

PLAN-008's seven tasks are `done` bar TASK-051, which is this review and the
pull request. `quire coverage` reconciles the matrix against the source with
**zero status lies**: every row marked `✅` binds to a test carrying its
tracking tag, and the twelve rows reported unbacked are exactly the twelve
marked `🚧`, each naming the refusal or absent-tool path that is implemented and
not yet driven. `make test` is 1050 green in 84 files; `make template-gate`
renders all three variants, installs the pinned toolchain, emits, re-checks
byte-for-byte, lints with ruff and black, and runs the rendered suite — 32 rows
per variant, zero skipped.

Two defects were found by instantiating rather than reading, and both are fixed
here: `seal-object-schemas: true` sealed the open marker models used as
`@contains` predicates, so every identity check matched nothing; and
`process.exit()` inside the gate's try block skipped its cleanup and made the
per-variant tally unreachable. A third — a regex character class dropping a
whole file from `quire coverage`'s scanner — is fixed in this repository and
filed against the engine as `agent-ix/quire-rs#404`.

## Verdict

CONDITIONAL. No task is incomplete, no row claims a backing it does not have,
and no code path lacks an owning requirement. Twelve rows remain uncovered by
observation, declared as such on the row and filed as `agent-ix/quoin#346`.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                                                                                                       | Refs                                                                                     |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| FND-001 | medium   | Twelve implemented refusal and absent-tool paths are not driven by any run; each row is `🚧` naming the path, filed as agent-ix/quoin#346.                                                                                                                                                                    | TC-1414..TC-1416; TC-1428; TC-1429; TC-1431; TC-1448..TC-1450; TC-1457; TC-1460; TC-1465 |
| FND-002 | medium   | The rendered emit driver is a per-repository copy rather than a versioned dependency, which is the fleet-drift shape StR-008 exists to end, one level up. FR-076-CON-3 keeps every copy byte-identical so the drift is detectable; extraction is filed as agent-ix/quoin#345.                                 | FR-076-CON-3                                                                             |
| FND-003 | medium   | `quire coverage` silently dropped every tracking tag in a source file because one regex character class contained an unmatched brace, reporting 59 written tests as missing. Fixed here by rewriting the class; filed against the engine as agent-ix/quire-rs#404.                                            | TC-1400..TC-1470                                                                         |
| FND-004 | medium   | The template gate exercised the pinned Node toolchain but ran pytest against whatever the ambient interpreter had. It now names every Python module the rendered suite imports, with the command that installs it, and fails rather than proceeding.                                                          | NFR-020-AC-1                                                                             |
| FND-005 | medium   | The negative fixtures asserted only that their declared `expect` labels were distinct; nothing ran them through the engine. A fixture the engine accepted would have passed. TC-035 now extracts each and asserts the engine or its emitted schema refuses it.                                                | FR-003-AC-15                                                                             |
| FND-006 | medium   | The rendered repository's own `make gate` would have failed on `ruff check` at first run, on bytes the template shipped. The rendered suite is now formatted, and the template gate runs ruff and black against every variant.                                                                                | NFR-001-AC-1                                                                             |
| FND-007 | low      | `process.exit()` inside the gate's try block skipped the directory cleanup and made the per-variant tally unreachable, so a failing run leaked its tree and could report at most one failure. Now throws; verified by forcing a failure — three variants attempted, tally correct, zero leftover directories. | FR-083-AC-2                                                                              |
| FND-008 | low      | Only `json-schema` of the declared target registry has an emitter. The rendered README, Test Matrix and `semantic.targets` agree that the rest are declared and not emitted; the emitters belong to agent-ix/filament-core-data#11.                                                                           | FR-076-AC-10                                                                             |
| FND-009 | low      | The rendered `build_tools.py` uses `subprocess.run(shell=True)` for its git helpers, inconsistent with every other subprocess call in the change. No untrusted input reaches it; carried over from the organization's Python cookiecutter rather than introduced here.                                        | FR-081-AC-2                                                                              |
| FND-010 | low      | Cookiecutter interpolates context values into the hooks' Python source before parsing, so a value containing a quote executes before validation runs. This is a property of cookiecutter hooks, not of this template; the inputs come from the maintainer running the generator.                              | FR-076-AC-5                                                                              |

## Coverage

- **Plan**: 7 tasks, 6 `done`, 1 `in_progress` (TASK-051, this review and the pull request). No task is blocked.
- **Matrix**: 74 rows in the TC-1400..TC-1470 range plus TC-032..TC-035 in the rendered matrix. 62 `✅`, 12 `🚧` with reasons. `quire coverage` reports 0 status lies and 12 unbacked rows — the same 12.
- **Suites**: `make test` 1050 passed / 84 files / 0 skipped. `make template-gate` 3 variants × 32 rows passed / 0 skipped, each after a real `npm install`, a real `tsp compile`, a real ruff and black run, and the real Quire engine.
- **Reverse gap**: every file the change adds is named by a requirement — the template by FR-076..FR-082, the gate by FR-083, the conformance contract by FR-083-CON-2, the toolchain floors by NFR-020. No unowned behaviour was found.
- **Semantic review**: not run. This gap analysis is mechanical plus the findings above; the intent↔test↔code pass was covered instead by the composite `/spec-review` (SR-128..SR-135) and the `/code-review` whose high findings are FND-004 through FND-007.
