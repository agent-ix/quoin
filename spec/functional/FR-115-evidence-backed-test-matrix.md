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
  - target: "ix://agent-ix/quoin/FR-040"
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
| `stale` | The auditor reports `stale-evidence`: either a binding names a suite with **no recorded run** at all, or its bound run is **behind HEAD**. |
| `suspect` | The auditor distrusts the binding: `suspect-link` (the statement changed since binding), `mocked-confirmation` (a stand-in for the verified behavior), `insufficient-independence` (a separation-axis violation), or `vacuous-evidence` (every bound symbol was skipped or absent from its run). All four are the auditor's own "claims to exist and does not hold" class. |
| `undischarged` | No binding exists for this criterion while the store holds bindings for others, or the auditor's remaining finding kinds apply (`undischarged`, `unknown-method`, `method-conformance`, `insufficient-multiplicity`, `insufficient-mutation-score`, `unmeasured-mutation-score`, `combinatorial-gap`), or the criterion has an `unevaluated` entry, or it appears in none of `healthy`/`findings`/`unevaluated`. |
| `no run evidence` | The evidence store's binding graph is **empty** — no criterion in this run has ever been discharged. |

That is all 12 `FindingKind` values this FR reads
(`quoin-finding-types::FindingKind::ALL`): `suspect-link`, `mocked-confirmation`,
`insufficient-independence`, and `vacuous-evidence` map to `suspect`;
`stale-evidence` maps to `stale`; `undischarged`, `unknown-method`,
`method-conformance`, `insufficient-multiplicity`,
`insufficient-mutation-score`, `unmeasured-mutation-score`, and
`combinatorial-gap` map to `undischarged`. Folding `vacuous-evidence` into
`suspect` (rather than `stale`, where an earlier draft of this FR left it) is
deliberate: `behind_head` does not stop the auditor's ladder, so a run behind
HEAD in which every symbol was skipped would otherwise carry both
`stale-evidence` and `vacuous-evidence` and render only the medium-severity
`stale` — hiding the high-severity vacuity behind it. Folding vacuity into
`suspect` (which outranks `stale`, below) means that case now renders
`suspect`.

`no run evidence` is a store-wide precondition, not a per-criterion finding:
it is reported for every criterion when, and only when, the binding graph is
empty, **overriding** what would otherwise be a uniform `undischarged` from
the auditor. This is deliberate (CR-001, below): it distinguishes "the
evidence pipeline has never been fed" from "this one criterion, in an
otherwise populated store, has no test." Until [Quoin#413](https://github.com/agent-ix/quoin/issues/413)
feeds the evidence store from real CI runs, every criterion in every repository
reads `no run evidence`. **That is correct**, not a defect to work around.

### `bound`'s "passing" promise depends on an upstream auditor gap (CR-002)

`AuditReport.healthy` is not, today, a guarantee that the bound run passed.
`quoin-auditor`'s ladder checks `Outcome::Skip` (vacuity) but never
`Outcome::Fail`/`Outcome::Error`: a binding minted by an earlier passing run
survives even when the *latest* run of that suite, at HEAD, reports the bound
symbol failing. That obligation completes the ladder with no finding and
lands in `healthy` — a red build renders `bound`. This is a real,
pre-existing gap in [FR-032](./FR-032-evidence-auditor.md), now tracked as
**[PLAT-1086](https://github.com/agent-ix/quoin/issues/1086)**, and it
**blocks this FR's implementation**: `matrix.build` keeps the word "passing"
in `bound`'s definition above, and CON-2 still forbids `matrix.build` from
adding the check itself — the fix belongs to the auditor's own ladder, not to
this join.

### Mapping precedence

When the binding graph is non-empty, each criterion's obligation id is looked
up in the supplied `AuditReport` (`Finding.obligation` / `healthy` /
`UnevaluatedCheck.obligation` are all the same id space as the criterion's own
`id` — quire-rs's `Obligation.id: String`, unchanged by this FR):

1. **`bound`** — the id appears in `AuditReport.healthy`.
2. **`suspect`** — else, the id has a `Finding` whose `kind` is one of
   `suspect-link`, `mocked-confirmation`, `insufficient-independence`,
   `vacuous-evidence`.
3. **`stale`** — else, the id has a `Finding` whose `kind` is `stale-evidence`.
4. **`undischarged`** — else (any other `Finding` kind, an `unevaluated`
   entry, or absence from all three collections).

This order mirrors `quoin-auditor`'s own ladder (`suspect_link` and
`mocked-confirmation` are asked before `unrecorded_evidence`/`behind_head`),
which already prevents an obligation from carrying both a suspect-class and a
stale-class finding in the common case. The only ladder rung that does not
stop evaluation once a suspect- or undischarged-class finding has already
fired is `behind_head`; folding `vacuous-evidence` into `suspect` (above)
closes the one case that rung could otherwise hide.

**A contradictory `AuditReport` is refused, not resolved by precedence.**
`healthy` is constructed to be disjoint from `findings` and `unevaluated`
(`quoin-auditor`'s own `clean` computation), so a genuine audit report never
names one id in both. An id appearing in `AuditReport.healthy` **and** in any
`Finding` or `UnevaluatedCheck` for that id is a contradictory input — a
hand-edited or merged report, not one the auditor produced — and
`matrix.build` SHALL refuse the whole request rather than let precedence pick
a winner silently.

### Assembling the auditor's inputs

`quoin matrix` SHALL assemble the `AuditReport` it passes to `matrix.build`
through **the same code path `quoin evidence audit` already uses** — not a
second, parallel assembly. `head_commit` is resolved as the repository's own
git HEAD, exactly as `quoin evidence audit` resolves it, and the
mock-inspection, catalog, and independence inputs are the identical values
`quoin evidence audit` would supply for the same working tree. When HEAD
cannot be resolved (no git repository, or the resolution itself fails),
`quoin matrix` SHALL refuse rather than call the auditor with
`head_commit: None` — an auditor invoked that way silently skips the
`behind_head` check ([FR-032](./FR-032-evidence-auditor.md)'s `ladder.rs`
filters that check on `Some(head)`), which would render a binding from months
ago as fresh rather than `stale`.

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
`matrix.build` adds no second commit comparison of its own: whether a binding
counts as fresh at HEAD was already decided by the auditor, before
`matrix.build` ever sees the result (see "Assembling the auditor's inputs",
above, and CR-002 for the one respect in which that upstream decision is
still incomplete).

## Inputs

- `coverage`: the quire-rs `coverage_matrix` payload (`requirements[].document`,
  `criteria[].id`/`method`/`binders`/`status`) as published by
  FR-050-AC-47..51, or **absent** when the module declares no `obligations:`
  source and the field is omitted entirely (FR-050-AC-51).
- `bindings`: the evidence store's binding graph **exactly as persisted** —
  `quoin-evidence`'s `BindingsFile` shape (`{schema_version, bindings:
  Vec<Binding>}`, [FR-030](./FR-030-evidence-store.md)), not a bare array.
- `audit`: the `quoin-auditor` `AuditReport` (`findings`, `healthy`,
  `unevaluated`, [FR-032](./FR-032-evidence-auditor.md)) computed over the
  same `bindings` and the same obligations.

## Outputs

- A `MatrixOutput`: `requirements[]`, each carrying the requirement id and its
  `criteria[]` in `coverage_matrix`'s own relative order; each criterion
  carries its `id`, `method` (omitted, never `null`, when the upstream
  obligation record omits it — mirroring FR-053's own
  `skip_serializing_if` convention), verbatim `static_status` and `binders`,
  a computed `evidence_status`, and a **total** `evidence_detail`.
- `evidence_detail` is present on **every** criterion, whatever its
  `evidence_status`, and is never a "pick one" summary. It carries three
  lists, each empty (not absent) when nothing applies:
  - `findings`: every `Finding` this id carries in `audit.findings`, each
    reduced to `{kind, summary, path, line, symbol}` (`Finding`'s own
    optional fields, taken verbatim), ordered by the same bucket order the
    status mapping uses (suspect-class kinds, alphabetically among
    themselves, then `stale-evidence`, then the remaining undischarged-class
    kinds alphabetically), then by `kind` name within a tie.
  - `bindings`: every `Binding` for this id, reduced to `{suite, commit}`
    where `commit` is the binding's own `commit` field (the commit of the run
    that **first** discharged it, per [FR-030](./FR-030-evidence-store.md) —
    `matrix.build` reports this verbatim and does not claim it is the commit
    that made the evidence stale), ordered by `(suite, commit)`.
  - `unevaluated`: the `check` name of every `UnevaluatedCheck` entry for
    this id, ordered lexically.
  - An id appearing in none of `audit.healthy`/`findings`/`unevaluated` — for
    example a criterion the caller's `bindings` never mentions — carries all
    three lists empty; this is the well-defined "reported nowhere" case, not
    an error.
- `reason`: present, and `requirements` empty, when the supplied `coverage`
  is absent or carries no criteria at all (mirrors `AssuranceCase.reason`,
  [FR-040](./FR-040-assurance-case-view.md)).

## Behavior

- `quoin-core` SHALL register `matrix.build` exactly as it registers
  `assurance.build_case`: an `OPERATIONS` entry, an exhaustive `dispatch()`
  match arm calling `crate::ops::matrix::build(request)`, no `Capabilities`
  (the op is pure).
- `matrix.build` SHALL enforce a byte ceiling on the whole request
  (`MAX_MATRIX_BUILD_BYTES`, refused via `CoreErrorCode::Refused` naming `op`,
  `limit_bytes`, `observed_bytes`), and SHALL reject an unrecognized field via
  `#[serde(deny_unknown_fields)]` on every request type. `coverage` is
  optional; `bindings` and `audit` are required.
- `matrix.build` SHALL refuse the whole request, with a named error code
  (`CoreErrorCode::ContradictoryAudit`, naming the offending obligation id in
  `context`) rather than compute a status, when the supplied `audit` names
  one id in both `healthy` and a `Finding`/`UnevaluatedCheck`.
- `matrix.build` SHALL emit canonical JSON (`protocol::canonical_json`): two
  calls over byte-identical input SHALL emit byte-identical output, including
  every field of the payload, not only its exit class.
- `quoin matrix` SHALL render `MatrixOutput` as markdown (default) or JSON
  (`--json`) to stdout, and SHALL write nothing under `spec/`.
- `quoin matrix` SHALL assemble its `AuditReport` input through the same
  assembly path `quoin evidence audit` uses (`head_commit`, mock-inspection,
  catalog, and independence inputs identical for the same working tree).
- `quoin matrix` SHALL refuse rather than proceed when `head_commit` cannot be
  resolved.
- `quoin matrix` SHALL no longer launch `ix-flow` ([FR-021](./FR-021-launch-ix-flow-runs.md)).
  `quoin-cli`'s `flow::FLOWS` table SHALL drop its `("matrix", "matrix")` entry,
  and the CLI's flow-dispatch match SHALL route only `"review"` and
  `"to-plan"` through `flow::run` — `matrix` SHALL instead assemble the three
  inputs (running `quire coverage`, reading the evidence store, running the
  auditor) and call `matrix.build`.
- `quoin matrix --help` SHALL describe the computed evidence-backed view,
  replacing `help.rs`'s retained "Build or update a requirements test
  matrix" summary, which described the retired agent workflow.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-115-CON-1 | `matrix.build` SHALL perform no file, network, or subprocess I/O; every input is supplied on stdin. | Architecture | Test |
| FR-115-CON-2 | `matrix.build` SHALL NOT re-derive or override any `AuditReport` finding; it maps existing `Finding.kind` values to an evidence status and invents no new evidence-of-rot check ([FR-032](./FR-032-evidence-auditor.md) owns that, including the [PLAT-1086](https://github.com/agent-ix/quoin/issues/1086) gap CR-002 names). | Architecture | Inspection |
| FR-115-CON-3 | `quoin matrix` SHALL write nothing under `spec/`. | Architecture | Test |
| FR-115-CON-4 | `matrix.build` SHALL refuse a contradictory `AuditReport` (an id in both `healthy` and `findings`/`unevaluated`) rather than resolve it by precedence. | Architecture | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-115-AC-1 | `matrix.build` is registered in `quoin-core`'s `OPERATIONS` list and dispatched by an exhaustive `match` arm identically in shape to `assurance.build_case`'s (no capabilities), covered by the existing `the_operations_const_is_every_operation_dispatch_routes` drift test. | Test |
| FR-115-AC-2 | The request is a `#[serde(deny_unknown_fields)]` struct carrying an optional `coverage`, and required `bindings` (the persisted `BindingsFile` shape) and `audit`; a request missing `bindings` or `audit`, or carrying an unrecognized field, is refused with `CoreErrorCode::BadRequest` naming `op`; a request over `MAX_MATRIX_BUILD_BYTES` is refused with `CoreErrorCode::Refused` naming `op`, `limit_bytes`, `observed_bytes`. | Test |
| FR-115-AC-3 | When `bindings.bindings` is empty, every criterion's `evidence_status` is `no run evidence`, regardless of what `audit` contains; `evidence_detail` is still computed per AC-9 and is not suppressed by this override. | Test |
| FR-115-AC-4 | When `bindings.bindings` is non-empty: a criterion whose id is in `audit.healthy` is `bound`; else a criterion with a `suspect-link`, `mocked-confirmation`, `insufficient-independence`, or `vacuous-evidence` finding is `suspect`; else a criterion with a `stale-evidence` finding is `stale`; else (any other finding kind, an `unevaluated` entry, or absence from `healthy`/`findings`/`unevaluated`) is `undischarged`. | Test |
| FR-115-AC-5 | An `audit` naming one obligation id in both `healthy` and a `Finding` or `UnevaluatedCheck` refuses the whole `matrix.build` call with `CoreErrorCode::ContradictoryAudit` naming that id in `context`; no partial `MatrixOutput` is emitted. | Test |
| FR-115-AC-6 | A fixture evidence store holding one passing binding **with a mock inspection recorded at HEAD** (so its obligation reaches `healthy` rather than an `unevaluated` entry), one binding whose run is behind the audited HEAD (`stale-evidence`/`behind_head`), and one criterion with zero bindings (while the store holds the other two) yields three distinct `evidence_status` values (`bound`, `stale`, `undischarged`) in one `matrix.build` call. | Test |
| FR-115-AC-7 | Each criterion's `static_status` and `binders` are copied verbatim from the supplied `coverage_matrix` criterion, unmodified by evidence computation; a criterion's `evidence_status` never influences its `static_status` or vice versa. A criterion whose upstream obligation record omits `method` renders with `method` omitted, never `null`. | Test |
| FR-115-AC-8 | `requirements[]` groups criteria by `quoin_assurance::requirement_of(criterion.id)`, not by `coverage_matrix`'s own document grouping; a fixture document holding obligations from two requirement-id prefixes yields two `requirements[]` entries, not one. Requirements are ordered by requirement id (ASCII byte order); criteria within a requirement retain `coverage_matrix`'s own relative order. | Test |
| FR-115-AC-9 | `evidence_detail` is total: for every criterion it carries `findings` (every `Finding` for that id, ordered suspect-class-then-`stale-evidence`-then-remaining-undischarged-class kinds, alphabetically by kind name within each group), `bindings` (every `Binding` for that id as `{suite, commit}`, ordered by `(suite, commit)`), and `unevaluated` (every matching `UnevaluatedCheck.check` name, ordered lexically) — each an empty list, never an absent key, when nothing applies. A fixture exercising all three non-empty, and a criterion appearing in none of `healthy`/`findings`/`unevaluated`/`bindings` (all three lists empty), are both covered by one test. | Test |
| FR-115-AC-10 | Two `matrix.build` calls over byte-identical canonical JSON input emit byte-identical canonical JSON **output** (the full payload, compared byte for byte — not only exit class or diagnostic code), covered by a `quoin-core` protocol test (`tc_NNN_matrix_boundary.rs`) built on the same fixture-corpus pattern as `tc_447_assurance_boundary.rs`. | Test |
| FR-115-AC-11 | `quoin matrix` renders `MatrixOutput` as a markdown table (`Requirement`/`Criterion`/`Method`/`Static Status`/`Evidence Status`/`Detail`, grouped under a `##` heading per requirement in requirement-id order) by default, and as the canonical `MatrixOutput` JSON under `--json`; a `coverage` that is absent or carries no criteria renders the `reason` string (never an empty table) and `quoin matrix` exits 0 unless invoked with `--strict`, which exits non-zero on that empty population — the same convention `quire coverage --strict` already uses for a zero-population report (quire-rs FR-050-AC-14). | Test |
| FR-115-AC-12 | Running `quoin matrix` against a fixture repository leaves the fixture's `spec/` directory byte-for-byte unchanged (directory listing and every file's bytes diffed before/after). | Test |
| FR-115-AC-13 | `quoin-cli`'s `flow::FLOWS` no longer maps `"matrix"`; the CLI's flow-dispatch match routes only `"review"` and `"to-plan"` through `flow::run`; invoking `quoin matrix` never spawns `ix-flow`. | Test |
| FR-115-AC-14 | `quoin matrix` resolves `head_commit` and the mock-inspection/catalog/independence inputs through the identical assembly path `quoin evidence audit` uses for the same working tree; when HEAD cannot be resolved, `quoin matrix` refuses rather than invoking the auditor with `head_commit: None`. | Test |
| FR-115-AC-15 | `quoin matrix --help` (and the retained CLI help surface, `help.rs`) describes the deterministic evidence-backed render, not the retired "Build or update a requirements test matrix" agent-workflow summary. | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) (the binding graph),
  [FR-032](./FR-032-evidence-auditor.md) (the audit report, and
  [PLAT-1086](https://github.com/agent-ix/quoin/issues/1086)'s failed/errored
  latest-run gap, which blocks this FR's implementation per CR-002),
  [FR-040](./FR-040-assurance-case-view.md) (`requirement_of`, the grouping
  and `reason`-on-empty conventions this FR reuses), quire-rs
  [FR-050](ix://agent-ix/quire-rs/FR-050)-AC-47..51 (`coverage_matrix`, and
  its own repin into quoin — see CR-001).
- **Downstream**: [FR-021](./FR-021-launch-ix-flow-runs.md), narrowed by this
  FR to `review` and `to-plan` only (see its own CR note).

> **CR-001 (2026-09-27, PLAT-1080):** quoin pins quire-rs by git revision
> (`=0.46.0`, `rev 523e47f`) — a dependency pin, not a vendored copy — whose
> `CoverageReport` carries `obligations: Vec<Obligation>` and
> `criteria: Vec<CriteriaCounts>` (per-document property-shape counts) but no
> `coverage_matrix` field and no per-criterion `binders`/`status`.
> `coverage_matrix` is [quire-rs#494](https://github.com/agent-ix/quire-rs/issues/494)'s
> `CoverageMatrix` implementation (quire-rs's own `PLAT-1077`, tracked in that
> repository, not this one) — a distinct piece of engine work this FR depends
> on but does not own. **The quoin-side repin** that brings a quire-rs
> revision carrying `coverage_matrix` into this repository's `Cargo.toml` is
> part of **this ticket's own implementation** (PLAT-1080), not a separate
> blocking ticket: once quire-rs#494 lands upstream, repinning
> `quoin-quire`'s `quire-rs` dependency is ordinary implementation work for
> whoever builds `matrix.build`. This FR is written against the target
> contract FR-050-AC-47..51 states; no interim shape is authorized —
> implementing against `obligations` alone and calling it `coverage_matrix`
> would be exactly the kind of compatibility layer this program exists to
> avoid building twice.
>
> **CR-002 (2026-09-27, PLAT-1080):** `bound`'s "passing run" promise
> currently outruns what `AuditReport.healthy` guarantees — see "`bound`'s
> 'passing' promise depends on an upstream auditor gap" above. The gap is
> `quoin-auditor`'s, tracked as
> [PLAT-1086](https://github.com/agent-ix/quoin/issues/1086), and blocks this
> FR's implementation without changing anything this FR itself specifies:
> `matrix.build` still reads `healthy` verbatim (CON-2), and once PLAT-1086
> closes, `bound` becomes true of every criterion it is reported for with no
> change to this FR's text.
