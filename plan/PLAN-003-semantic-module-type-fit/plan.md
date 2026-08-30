---
id: PLAN-003
title: "Default-module semantic type-fit audit"
type: Plan
relationships:
  - target: "ix://agent-ix/quoin/US-014"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-051"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-052"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-053"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-054"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-055"
    type: "references"
  - target: "ix://agent-ix/quoin/NFR-015"
    type: "references"
  - target: "ix://agent-ix/quoin/NFR-016"
    type: "references"
---

# PLAN-003: Default-module semantic type-fit audit

## Objective

Implement issue #288 as a reproducible, read-only census and semantic type-fit review of the entire
pinned default-module corpus. Publish canonical JSON and generated Markdown/SpecReview projections,
then stop at any recommendation that would activate a breaking contract, compiler, schema, migration,
publication, enforcement, retirement, database, API, UI, or conflicting feature-work change.

## Requirements in scope

- [x] **US-014:** Maintainers can reason from complete corpus evidence rather than samples; the read-only audit promotion is approved and every recommendation remains separately gated.
- [x] **FR-051:** Snapshot exact source, installed-content, tool, and external-evidence identities.
- [x] **FR-052:** Inventory every module, declaration, contract surface, Markdown path, and parse state.
- [x] **FR-053:** Assess every declaration on all semantic-fit axes with evidence and confidence.
- [x] **FR-054:** Publish canonical artifacts and generated human projections.
- [x] **FR-055:** Reconcile findings with architecture, Quire, core-data, and downstream boundaries.
- [x] **NFR-015:** Close all denominators and reproduce equal-input canonical bytes.
- [x] **NFR-016:** Remain read-only; TC-1194 records the human promotion decision without activating downstream work.

## Scope boundaries

### In scope

- A pure audit library under `scripts/semantic-module-type-fit/`, a thin runner, fixture-first tests,
  and retained results under `analysis/semantic-module-type-fit/`.
- The exact `.prettierignore` entry that keeps generated, content-addressed audit bytes out of source formatting.
- Exact module-source and tool identity, complete declaration and Markdown enumeration, explicit
  parse failures, semantic heuristics with confidence/evidence, canonical serialization, ledgers,
  repository impact, generated report/SpecReview, and staleness validation.
- Specifications, Test Matrix, plan, code review, and gap analysis artifacts.

### Out of scope

- Changes under production `src/`, module manifests, schemas, skeletons, registries, generated
  language packages, Quire parsing, persistence, APIs, CLIs, UIs, migrations, or consumers.
- Selecting or implementing a schema compiler, wire format, code generator, package release,
  compatibility policy, migration, enforcement, or retirement.
- Merging the stacked audit ahead of #289's named maintainer approval.

## Inputs and accepted premises

| Input                  | Required identity and treatment                                                                                                         |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `default-modules.yaml` | Branch-local bytes, digest, ordered declarations, requested refs and resolved SHAs.                                                     |
| Default module sources | Canonical repository/subdirectory and exact inspected content/commit; disagreement is retained.                                         |
| Quire                  | CLI and engine identity plus pinned `quire-rs#385` corpus revision; parsing failures remain records.                                    |
| Semantic architecture  | PR #311 merged as `4a82644ad3cf75770cc53ef3812e3b13e80b516d`; its decisions guide classification while downstream changes remain gated. |
| Core-data census       | Merged `filament-core-data#10` commit and its contract inventory, linked rather than copied.                                            |

## Deliverable flow

```text
default-modules.yaml + resolved module sources + Quire/core-data evidence
                              │
                              v
                    immutable audit snapshot
                              │
                              v
 module declarations + contract surfaces + every Markdown path/parse state
                              │
                              v
 per-qualified-type axes + dispositions + conflicts + missing concepts + impact
                              │
                              v
 canonical JSON manifest ──> generated report.md + validated SpecReview
                              │
                              v
                fresh-census and non-disruption gates
```

## TDD and evidence strategy

1. Add red fixture tests with exact TC-1156..TC-1193 and AC trace tags; retain the red evidence.
2. Implement pure identity, inventory, scoring, reconciliation, and serialization functions in that order.
3. Run the pure functions against fixture modules, including duplicate names, placeholder schemas,
   malformed/untyped Markdown, absent instances, provenance drift, and output tampering.
4. Run the read-only audit against all 10 pinned default modules and retain canonical artifacts.
5. Generate report and SpecReview from the canonical object, validate all artifacts, and reconcile counts/digests.
6. Run a fresh census, full repository gates, `/code-review`, and `/gap-analysis`; fix all findings.
7. Open a stacked PR and stop for #289 approval and any major-interference recommendation.

## Test Matrix allocation

| Cases            | Evidence                                                  | Owner task         |
| ---------------- | --------------------------------------------------------- | ------------------ |
| TC-1156..TC-1161 | Snapshot and provenance                                   | TASK-012, TASK-013 |
| TC-1162..TC-1168 | Corpus and denominator inventory                          | TASK-012, TASK-014 |
| TC-1169..TC-1176 | Semantic scoring and missing/conflict ledgers             | TASK-012, TASK-015 |
| TC-1177..TC-1182 | Canonical output and generated projections                | TASK-012, TASK-016 |
| TC-1183..TC-1187 | Architecture/core-data/Quire reconciliation and freshness | TASK-016, TASK-017 |
| TC-1188..TC-1193 | Completeness, determinism, and non-mutation               | TASK-012..TASK-017 |
| TC-1194          | Major-interference human gate                             | TASK-019           |

## Execution order and dependencies

```text
TASK-012 red contract suite
       │
       v
TASK-013 snapshot/provenance
       │
       v
TASK-014 complete inventory
       │
       v
TASK-015 scoring and ledgers
       │
       v
TASK-016 canonical artifacts and projections
       │
       v
TASK-017 fresh census + reproducibility + non-mutation
       │
       v
TASK-018 traceability + code review + gap analysis
       │
       v
TASK-019 stacked PR + major-interference/architecture gate
```

## Safety and promotion gates

| Gate            | Pass condition                                                                                                    | Failure action                                                   |
| --------------- | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Specification   | Composite review has no unresolved medium/high finding and Quire validates all artifacts.                         | Return to `/specify`; do not code.                               |
| Provenance      | Requested, resolved, installed, and reviewed identities agree or conflicts are retained and verdict is non-clean. | Do not normalize or omit the disagreement.                       |
| Completeness    | All source denominators reconcile exactly.                                                                        | Keep the audit incomplete; do not publish a clean review.        |
| Scope           | Diff is limited to spec, plan, audit scripts/tests, analysis, and reviews.                                        | Remove or split behavior/module/consumer changes.                |
| Reproducibility | Equal identified inputs produce equal canonical content and all digests/counts validate.                          | Fix nondeterminism or stale projection before review.            |
| Freshness       | Immediate pre-signoff census matches the retained snapshot.                                                       | Regenerate and re-review the audit.                              |
| Review          | Code review and gap analysis contain no unresolved medium/high finding.                                           | Fix and rerun affected evidence.                                 |
| Promotion       | #289 architecture is approved and no recommendation crosses a major-interference boundary without its own gate.   | Leave PR stacked/open; do not merge or activate downstream work. |

## Definition of done

- TC-1156..TC-1193 pass with exact tags and every automated matrix row is covered.
- Every default-module entry, declaration, contract surface, and Markdown path is reconciled.
- Every qualified type has every axis, one disposition, confidence, and linked evidence.
- Canonical JSON, generated report, and validated SpecReview agree by count and digest.
- Fresh-census, deterministic rerun, read-only filesystem, full repository, code-review, and gap gates pass.
- TC-1194 and the #289 approval disposition are retained in SR-058; every downstream major-interference boundary remains explicit and unsatisfied.
