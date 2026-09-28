---
id: FR-115
title: "Evidence-backed test matrix (`matrix.build`)"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-021"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-050"
    type: "requires"
    cardinality: "1:1"
---

# FR-115: Evidence-backed test matrix (`matrix.build`)

## Description

`quoin-core` SHALL expose `matrix.build`, an operation beside
`assurance.build_case`, that joins the quire-rs static coverage matrix
([FR-050](ix://agent-ix/quire-rs/FR-050)-AC-47..51's `coverage_matrix`) with
the `quoin-evidence` binding graph ([FR-030](./FR-030-evidence-store.md)) and
the `quoin-auditor` `AuditReport` ([FR-032](./FR-032-evidence-auditor.md)) to
answer the question the static matrix cannot: was a criterion discharged by a
**passing run**, or only tagged?

`coverage_matrix` already computes, per criterion, one of `tagged` /
`untagged` / `tagged-by-ignored-test` / `method-without-symbol` — whether a
test *exists* for the obligation. It is silent on whether that test *ran and
passed*, whether the passing run is *current*, or whether the binding backing
it is *suspect*. Those three questions are exactly what
[FR-030](./FR-030-evidence-store.md)'s binding graph and
[FR-032](./FR-032-evidence-auditor.md)'s auditor already answer per
obligation — `matrix.build` is a join, not a new check.

### Two independent axes, one row

A criterion's row carries **both** its static status (unchanged, sourced
verbatim from `coverage_matrix`) and a new **evidence status**, computed from
the binding graph and the audit report alone. The two axes answer different
questions and neither is derived from the other: a criterion can be `tagged`
and `undischarged` (a test exists, tagged, but no passing run is recorded —
the tag alone proves nothing), or `untagged` and, trivially, carry no
evidence status worth reporting beyond `no run evidence`.

### The evidence-status vocabulary is closed and total

Every criterion in the population gets exactly one of:

| Status | Meaning |
|---|---|
| `bound` | A passing run backs this criterion, and the auditor found nothing wrong with it. |
| `stale` | A binding exists, but its evidence is recorded as out of date (`stale-evidence`, [FR-032](./FR-032-evidence-auditor.md)). |
| `suspect` | A binding exists, but the auditor distrusts it (`suspect-link`, `mocked-confirmation`, or `insufficient-independence`). |
| `undischarged` | The evidence store holds bindings for *other* criteria, but this one has none, or the auditor's remaining findings (`unknown-method`, `method-conformance`, `insufficient-multiplicity`, `insufficient-mutation-score`, `unmeasured-mutation-score`, `combinatorial-gap`) or an `unevaluated` entry apply, or the criterion appears in none of `healthy`/`findings`/`unevaluated`. |
| `no run evidence` | The evidence store's binding graph is **empty** — no criterion in this run has ever been discharged. |

`no run evidence` is a store-wide precondition, not a per-criterion finding:
it is reported for every criterion when, and only when, `bindings` is empty,
**overriding** what would otherwise be a uniform `undischarged` from the
auditor. This is deliberate (CR-001, below): it distinguishes "the evidence
pipeline has never been fed" from "this one criterion, in an otherwise
populated store, has no test." Until [Quoin#413](https://github.com/agent-ix/quoin/issues/413)
feeds the evidence store from real CI runs, every criterion in every repository
reads `no run evidence`. **That is correct**, not a defect to work around.

### Mapping precedence

When `bindings` is non-empty, each criterion's obligation id is looked up in
the supplied `AuditReport` (`Finding.obligation` / `healthy` /
`UnevaluatedCheck.obligation` are all the same id space as the criterion's own
`id` — quire-rs's `Obligation.id: String`, unchanged by this FR):

1. **`bound`** — the id appears in `AuditReport.healthy`.
2. **`suspect`** — else, the id has a `Finding` whose `kind` is
   `suspect-link`, `mocked-confirmation`, or `insufficient-independence`.
3. **`stale`** — else, the id has a `Finding` whose `kind` is `stale-evidence`.
4. **`undischarged`** — else (any other `Finding` kind, an `unevaluated` entry,
   or absence from all three collections).

This order is not invented for this FR: it mirrors `quoin-auditor`'s own
ladder (`suspect_link` and `mocked-confirmation` are asked before
`unrecorded_evidence`/`behind_head`), which already prevents an obligation
from carrying both a suspect-class and a stale-class finding in the common
case. The only two ladder rungs that do not stop evaluation
(`unknown-method`, `behind_head`) are the sole way one obligation can carry
two `Finding` kinds; this ordering resolves that case deterministically
without inventing a second severity ranking (`quoin-finding-types::Severity`
is an open string, not a rank, and is not reused here).

### Grouping diverges deliberately from quire's own grouping

`coverage_matrix.requirements[]` groups criteria by their **owning document**
(a file path — FR-050-AC-48). `matrix.build` flattens every criterion across
every document into one list and re-groups by
**`quoin_assurance::requirement_of(criterion.id)`** — the same requirement-id
prefix grouping [FR-040](./FR-040-assurance-case-view.md)'s `build_case`
already uses. This is the same principle in both places: a document holding
more than one requirement's obligations must not collapse them into one
section keyed by the document that happens to hold them.

### The op is pure, like `assurance.build_case`

`matrix.build` runs nothing and reads nothing from disk or network. Its three
inputs — the coverage matrix, the bindings, the audit report — are supplied
whole on stdin by the caller, which already had to assemble them (run `quire
coverage`, read the store, run the auditor) to produce this call's arguments.
Determining "at which commit relative to HEAD" a binding counts as fresh is
**not** re-decided here: it was already decided by whichever `head_commit` the
caller passed into the auditor before calling `matrix.build` — `bound`
requires membership in `AuditReport.healthy`, and `healthy` membership already
required the auditor's own `behind_head` check to pass. `matrix.build` adds no
second commit comparison.

## Inputs

- `coverage`: the quire-rs `coverage_matrix` payload (`requirements[].document`,
  `criteria[].id`/`method`/`binders`/`status`) as published by FR-050-AC-47..51.
- `bindings`: the `quoin-evidence` binding graph (`Vec<Binding>`, [FR-030](./FR-030-evidence-store.md)).
- `audit`: the `quoin-auditor` `AuditReport` (`findings`, `healthy`,
  `unevaluated`, [FR-032](./FR-032-evidence-auditor.md)) computed over the
  same `bindings` and the same obligations.

## Outputs

- A `MatrixOutput`: `requirements[]`, each carrying the requirement id and its
  `criteria[]` in `coverage_matrix`'s own relative order; each criterion
  carries its `id`, `method`, verbatim `static_status` and `binders`, a
  computed `evidence_status`, and an `evidence_detail` (present for `stale` /
  `suspect` / `undischarged`, absent — never null — for `bound` / `no run
  evidence`) naming the backing `Finding`'s `kind`, `suite` (when a binding
  exists), `commit`, and `summary` verbatim.
- `reason`: present, and `requirements` empty, when the supplied `coverage`
  carries no criteria at all (mirrors `AssuranceCase.reason`,
  [FR-040](./FR-040-assurance-case-view.md)).

## Behavior

- `quoin-core` SHALL register `matrix.build` exactly as it registers
  `assurance.build_case`: an `OPERATIONS` entry, an exhaustive `dispatch()`
  match arm calling `crate::ops::matrix::build(request)`, no `Capabilities`
  (the op is pure).
- `matrix.build` SHALL enforce a byte ceiling on the whole request
  (`MAX_MATRIX_BUILD_BYTES`, refused via `CoreErrorCode::Refused` naming `op`,
  `limit_bytes`, `observed_bytes`), and SHALL reject an unrecognized field via
  `#[serde(deny_unknown_fields)]` on every request type.
- `matrix.build` SHALL emit canonical JSON (`protocol::canonical_json`): two
  calls over byte-identical input SHALL emit byte-identical output.
- `quoin matrix` SHALL render `MatrixOutput` as markdown (default) or JSON
  (`--json`) to stdout, and SHALL write nothing under `spec/`.
- `quoin matrix` SHALL no longer launch `ix-flow` ([FR-021](./FR-021-launch-ix-flow-runs.md)).
  `quoin-cli`'s `flow::FLOWS` table SHALL drop its `("matrix", "matrix")` entry,
  and the CLI's flow-dispatch match SHALL route only `"review"` and
  `"to-plan"` through `flow::run` — `matrix` SHALL instead assemble the three
  inputs (running `quire coverage`, reading the evidence store, running the
  auditor) and call `matrix.build`.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-115-CON-1 | `matrix.build` SHALL perform no file, network, or subprocess I/O; every input is supplied on stdin. | Architecture | Test |
| FR-115-CON-2 | `matrix.build` SHALL NOT re-derive or override any `AuditReport` finding; it maps existing `Finding.kind` values to an evidence status and invents no new evidence-of-rot check ([FR-032](./FR-032-evidence-auditor.md) owns that). | Architecture | Inspection |
| FR-115-CON-3 | `quoin matrix` SHALL write nothing under `spec/`. | Architecture | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-115-AC-1 | `matrix.build` is registered in `quoin-core`'s `OPERATIONS` list and dispatched by an exhaustive `match` arm identically in shape to `assurance.build_case`'s (no capabilities), covered by the existing `the_operations_const_is_every_operation_dispatch_routes` drift test. | Test |
| FR-115-AC-2 | The request is a `#[serde(deny_unknown_fields)]` struct carrying `coverage`, `bindings`, and `audit`; a request missing one or carrying an unrecognized field is refused with `CoreErrorCode::BadRequest` naming `op`; a request over `MAX_MATRIX_BUILD_BYTES` is refused with `CoreErrorCode::Refused` naming `op`, `limit_bytes`, `observed_bytes`. | Test |
| FR-115-AC-3 | When `bindings` is empty, every criterion's `evidence_status` is `no run evidence` and `evidence_detail` is absent, regardless of what `audit` contains. | Test |
| FR-115-AC-4 | When `bindings` is non-empty: a criterion whose id is in `audit.healthy` is `bound`; else a criterion with a `suspect-link`, `mocked-confirmation`, or `insufficient-independence` finding is `suspect`; else a criterion with a `stale-evidence` finding is `stale`; else (any other finding kind, an `unevaluated` entry, or absence from `healthy`/`findings`/`unevaluated`) is `undischarged`. | Test |
| FR-115-AC-5 | A fixture evidence store holding one passing binding, one binding whose run is behind the audited HEAD (`stale-evidence`/`behind_head`), and one criterion with zero bindings (while the store holds the other two) yields three distinct `evidence_status` values (`bound`, `stale`, `undischarged`) in one `matrix.build` call. | Test |
| FR-115-AC-6 | Each criterion's `static_status` and `binders` are copied verbatim from the supplied `coverage_matrix` criterion, unmodified by evidence computation; a criterion's `evidence_status` never influences its `static_status` or vice versa. | Test |
| FR-115-AC-7 | `requirements[]` groups criteria by `quoin_assurance::requirement_of(criterion.id)`, not by `coverage_matrix`'s own document grouping; a fixture document holding obligations from two requirement-id prefixes yields two `requirements[]` entries, not one. Requirements are ordered by requirement id (ASCII byte order); criteria within a requirement retain `coverage_matrix`'s own relative order. | Test |
| FR-115-AC-8 | Two `matrix.build` calls over byte-identical canonical JSON input emit byte-identical canonical JSON output, covered by a `quoin-core` protocol test (`tc_NNN_matrix_boundary.rs`) built on the same fixture-corpus pattern as `tc_447_assurance_boundary.rs`, asserting exit class and diagnostic code only. | Test |
| FR-115-AC-9 | `quoin matrix` renders `MatrixOutput` as a markdown table (`Requirement`/`Criterion`/`Method`/`Static Status`/`Evidence Status`/`Detail`, grouped under a `##` heading per requirement in requirement-id order) by default, and as the canonical `MatrixOutput` JSON under `--json`; an empty `requirements[]` renders the `reason` string, not an empty table. | Test |
| FR-115-AC-10 | Running `quoin matrix` against a fixture repository leaves the fixture's `spec/` directory byte-for-byte unchanged (directory listing and every file's bytes diffed before/after). | Test |
| FR-115-AC-11 | `quoin-cli`'s `flow::FLOWS` no longer maps `"matrix"`; the CLI's flow-dispatch match routes only `"review"` and `"to-plan"` through `flow::run`; invoking `quoin matrix` never spawns `ix-flow`. | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) (the binding graph),
  [FR-032](./FR-032-evidence-auditor.md) (the audit report),
  [FR-040](./FR-040-assurance-case-view.md) (`requirement_of`, the grouping
  and `reason`-on-empty conventions this FR reuses), quire-rs
  [FR-050](ix://agent-ix/quire-rs/FR-050)-AC-47..51 (`coverage_matrix`).
- **Downstream**: [FR-021](./FR-021-launch-ix-flow-runs.md), narrowed by this
  FR to `review` and `to-plan` only (see its own CR note).

> **CR-001 (2026-09-27, PLAT-1080):** `coverage_matrix` (quire-rs
> [FR-050](ix://agent-ix/quire-rs/FR-050)-AC-47..51) is **not yet vendored**.
> quoin currently pins quire-rs `=0.46.0` (`rev 523e47f`), whose
> `CoverageReport` carries `obligations: Vec<Obligation>` and
> `criteria: Vec<CriteriaCounts>` (per-document property-shape counts) but no
> `coverage_matrix` field and no per-criterion `binders`/`status` — that
> repin is [PLAT-1077](https://github.com/agent-ix/quoin/issues/1077)'s own
> ticket, in progress at authoring time. This FR is written against the
> target contract FR-050-AC-47..51 states; `matrix.build`'s implementation is
> blocked on the repin landing, not on anything this FR leaves undecided. No
> interim shape is authorized here — implementing against `obligations` alone
> and calling it `coverage_matrix` would be exactly the kind of compatibility
> layer this program exists to avoid building twice.
