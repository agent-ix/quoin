---
id: SR-118
title: "Base review of the semantic module contract"
type: SpecReview
analysis: base
scope: "US-020, FR-070..FR-075, NFR-017, spec/matrix.md TC-1336..TC-1382"
review_set: all
---

# Base review of the semantic module contract

## Summary

Issue #293 adds one user story, six functional requirements, one non-functional
requirement, and 47 test cases for the `semantic` manifest block, the typed
Properties table and `sysml` fence mapping, Invariants/Operations mapping,
`data_schema` by path and digest, legacy forms at `warning`, and package
exports/locks. All 47 acceptance criteria and constraints map to at least one
test case; Quire reports zero errors and zero grammar findings on the new
artifacts. The bundle is ready for the seven analyses.

## Checklist Results

| Area | Result | Evidence |
|---|---|---|
| ID format and uniqueness | Pass | US-020, FR-070..075, NFR-017, TC-1336..1382 continue the quoin sequences |
| User story quality | Pass | US-020 has the story shape, four examples, options, constraints, dependencies, priority, traceability |
| Functional requirement quality | Pass | Rationale, EARS-shaped behavior, constraints with validation, measurable ACs, dependencies on every FR |
| Coverage (Rule 1) | Pass | 47/47 → TC-1336..1382 (scripted check, 2026-09-03) |
| Option permutation (Rule 2) | Pass | table vs fence × content-equal; inline vs reference `data_schema` × `semantic` block present; legacy severity × sweep report |
| Constraint boundary (Rule 3) | Pass | multiplicity `5..2`, unknown keyword, unknown key, version `2.0.0`, path escape, URL identity |
| Error path (Rule 4) | Pass | every rejection has a TC with a locus expectation |
| State transition (Rule 5) | Pass | inline → reference `data_schema`; `warning` → `error` promotion gated on the sweep report |
| Edge case (Rule 6) | Pass | both forms present; clause inline and external; duplicate package across modules |
| Cross-referencing | Pass | FR → US-020 `implements`; NFR-017 `constrains` US-020 and FR-070; relative body links validate |

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-163 | low | US-020 carries illustrative examples rather than an AC table, per the installed US archetype; EX-1..4 map to TC-1344, TC-1345, TC-1360, TC-1367. | US-020 |
| FND-164 | low | quoin's fixed-column FR coverage table is described as generated from tracking tags; the #293 rows are hand-added as 🚧 and will be regenerated when the tagged tests land. | spec/matrix.md |
| FND-165 | low | The SysML fence subset (`attribute`, `ref item`, `item`, `:>`) is stated in FR-071 by example; the golden fixtures (FR-071-CON-2) are the normative surface quire-rs#388 consumes. | FR-071 |

## Gate Result

| Gate | Result | Evidence |
|---|---|---|
| IDs, structure, and EARS grammar | Pass | Quire: zero errors, zero grammar warnings on the new artifacts |
| Requirement clarity and atomicity | Pass | One `shall` per statement in FR-070..075 and NFR-017 |
| Complete traceability | Pass | 47/47; TC-1336..1382 |
| Failure, transition, boundary, and option coverage | Pass | see checklist |
| Non-disruptive scope | Pass | NFR-017; no required key, no corpus write, advisory by default |
| Analyses | Pass after remediation | SR-119..125 in this directory; dispositions below |

## Dispositions of analysis findings

Applied to the specification on 2026-09-03 before planning (FR-070..075 rewritten; matrix TC-1336..1386):

| Theme | Findings | Change |
|---|---|---|
| SysML fence implementer and subset | FND-143, FND-100, FND-133, FND-165 | Quire recognises the fence at line level; subset reduced to `attribute` and `ref item` (both with table equivalents); `item`, `:>`, `part def` are errors; US-020 and FR-071-CON-1 say so |
| Clause ids are `Identifier`s | FND-082, FND-098 | FR-072 requires `Identifier`, heading-derived, examples renamed (`notArchived`); heading failure AC |
| Unresolved `Type` representation | FND-083 | Placeholder identity `ix://<org>/<repo>/unresolved/<token>` plus advisory; field survives to the IR reader |
| Closed `semantic` key set | FND-151, FND-096, FND-132, FND-152, FND-094 | FR-070 enumerates all ten keys (`compatibility_posture`, `legacy_forms`, `sweep_report`, `imports`, …), required subset, target-registry and package-form rejections (AC-7) |
| Manifest schema owner and two loaders | FND-166, FND-107, FND-131, FND-112, FND-145 | filament-core-service#21 filed and cited; Quoin vendors the schema with provenance (CON-2); install-time rejections allocated to Quoin, artifact-time to Quire; FR-009 upstream |
| Pipe-split AC cell | FND-121 | FR-074-AC-1 pipes escaped |
| FR-075 rests on absent surfaces | FND-135, FND-097, FND-106, FND-169, FND-147 | FR-075 derives a complete `package-manifest.json` (every required field's derivation stated), pins digests in the existing ts-plugin-kit `registry.json`, uses `quoin module install`; `package` is the IR `packageIdentity` form `<org>/<repo>`; #287 may relocate pins later |
| Sweep report identity | FND-085, FND-146, FND-170, FND-108, FND-137 | FR-074 defines `semantic.sweep_report`, the report schema, and `quoin semantic sweep`; guard checks package and version |
| `data_schema` failure modes | FND-084, FND-091, FND-113, FND-136 | Missing/unreadable/non-JSON/`$id`-less, `$ref` version/unshipped/cycle, symlink escape, ambiguous mixed keys; digest over raw bytes; `$id` scheme fixed; vendored semantic-core bundle with provenance (AC-6) |
| Type resolution and normalization | FND-086, FND-102, FND-087, FND-103, FND-089, FND-138 | Case-sensitive id-then-title-then-import order, duplicate-title error, backticks stripped, `enumeration` `## Values` → `EnumValue[]`, normalization rule stated, imports via `semantic.imports` with cycle rejection |
| Cell grammars | FND-101, FND-109, FND-088 | Full `Constraints` item grammar (valueless, `enumValues: a\|b`, `pattern: /…/`, `format: ns:name`); flags only on collections; reader rules surfaced with row locus (AC-8) |
| Clause span, languages, scope | FND-099, FND-134, FND-153, FND-110, FND-092 | `sourceSpan` is a semantic-core `SourceLocus`; advisory `semantic.clause-language-unchecked` for `sysml`/`fretish`/namespaced; clause-id scope across sections; duplicate operation names; external form `Clause: <path>#<id>` |
| Verification alignment | FND-123, FND-124, FND-127, FND-122 | CON validations name their oracle; TC-1365 Integration; TC-1345 Unit against golden fixtures; NFR-017 methods name test kinds |
| Scope and NFR edges | FND-144, FND-114, FND-148, FND-149, FND-111 | spec.md In Scope bullet; NFR-017 constrains FR-073/074/075; targets rejection; Quoin-held FR-006 copies |
| Duplicate package ordering | FND-090, FND-115 | Sorted module-root order (FR-009); later root rejected |

Not changed: FND-125 (suite registry gains Property/Static suites at implementation), FND-129
(fuzz target recommended, tracked at planning), FND-139 (quire-only verification is inherent
to the split and is listed per TC in SR-123), FND-160, FND-163..165, and the remaining lows.
