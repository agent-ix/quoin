---
id: FR-054
title: "Publish canonical semantic audit artifacts"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-014"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-053"
    type: "depends_on"
---

# FR-054: Publish canonical semantic audit artifacts

> **⛔ Withdrawn — 2026-09-29.** This requirement is withdrawn and is not
> implemented, with the rest of the semantic type-fit audit (US-014, FR-051..FR-055,
> NFR-015, NFR-016) under [quoin#654](https://github.com/agent-ix/quoin/pull/654).
> The audit ran once under [quoin#288](https://github.com/agent-ix/quoin/issues/288);
> its generator and test were removed by the native cutover
> ([quoin#524](https://github.com/agent-ix/quoin/pull/524)), and its retained output
> is deleted by quoin#654. Nothing in quoin produces or reads the audit any more.
>
> The text below is retained unchanged. It describes what was required, not what
> is required.

## Description

When inventory and scoring complete, the audit SHALL publish machine-readable canonical data and a generated
human review that expose the same denominators, evidence, findings, and decisions.

## Rationale

The audit is both an architecture review for people and input to later planning and tooling. One
canonical data set prevents the prose report, tickets, and future automation from diverging.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-054-AC-1 | `analysis/semantic-module-type-fit/` contains a versioned snapshot, corpus inventory, type-fit matrix, conflict ledger, missing-type ledger, repository-impact assessment, and summary manifest as canonical JSON. | Test (TC-1177) |
| FR-054-AC-2 | Every ledger row has a stable id, severity, status, confidence, affected qualified types/modules/repositories, evidence references, rationale, and recommended next decision or ticket boundary. | Test (TC-1178) |
| FR-054-AC-3 | Repository impact records every default module and each affected Quoin, Quire, Filament, compiler, generated-package, database, API, CLI, and UI boundary as `none`, `candidate`, `required`, or `unknown`, with effort, risk, dependency, wave, confidence, and rationale. | Test (TC-1179) |
| FR-054-AC-4 | A validated `SpecReview` and generated `report.md` summarize the canonical JSON, link every finding and denominator, and contain no independently maintained score or finding. | Test (TC-1180) |
| FR-054-AC-5 | The summary manifest records artifact schema versions and SHA-256 digests and fails validation when an artifact is missing, unreferenced, stale, or disagrees with a declared count. | Test (TC-1181) |
| FR-054-AC-6 | Re-running against identical inputs yields byte-identical canonical JSON and report content except for a separately identified run timestamp field excluded from content identity. | Test (TC-1182) |

## Constraints

- Canonical JSON is the source for the Markdown report; the report is a projection for humans and LLMs.
- The artifacts contain no credentials, absolute user paths, or mutable local-only identifiers.

## Dependencies

- **Upstream**: [FR-053](./FR-053-score-semantic-type-fit.md)
