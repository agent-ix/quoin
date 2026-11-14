---
id: FR-030
title: "The evidence store as the artifact of record"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quire-rs/FR-053"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-029"
    type: "requires"
---

# FR-030: The evidence store as the artifact of record

## Description

`quoin` SHALL persist what was verified, against which statement, by which run —
in a file store under `spec/evidence/` — and SHALL expose verbs to transcribe a
run and collect unreferenced records.

Every quoin verification surface recomputes its view from scratch and emits a
report; nothing persisted what was checked. That is the "green matrix over dead
links" failure mode, already observed at ecosystem scale: **1,014 trace tags
binding to nothing while matrices read as covered**
([quire-rs#72](ix://agent-ix/quire-rs/FR-050)).

### Governing principle: store only what cannot be recomputed

Run outcomes are facts
about the past — nothing can re-derive them tomorrow. **Obligations are not.**
They are re-derived live from quire ([FR-053](ix://agent-ix/quire-rs/FR-053))
on every read, so they cannot drift out of agreement with the spec. A stored
copy would be one more thing to keep in sync, and the thing it would most likely
disagree with is the requirement itself.

### Authored → markdown; machine-transcribed → JSON

`suites.md` and `inspections.md` are validated corpus documents
(`spec-artifacts-process` FR-006). `bindings.json`, `runs/**` and the
differential-report projection family are machine-written, typeless and
corpus-invisible; their shapes are quoin's. A minimal allowlisted projection of
a differential report is retained as a separate companion file rather than as
a `RunRecord` field; its path is derived from the paired run's suite and
timestamp.

No SQLite anywhere. quoin writes files; parsing, loading and indexing for an
application belongs to the consuming application, and a derived index inside the store
would be a second source of truth for facts the files already hold.

### The store lives under `spec/`

quire-rs CR-045 bounds the document walk to `<scope>/spec`, so the authored half
is only a validated corpus document if it lives there. **[RAN]** the alternative:
a typed, well-formed registry at the repository root minted nothing and was
reported nowhere, while the identical file under `spec/` was walked, validated
and minted its ids. agent-ix/quoin#79's original layout rested on the premise
that quire "validates them wherever they live", which measurement contradicts.

### The suite is the atomic unit

One file per (suite, timestamp); aggregation is a view. A differential report
and its run record form one logical retained pair at the same suite and
timestamp. A partial run must never be able to masquerade as a full one — the
`make ci` / `make ci-python` split already caused exactly that rot (quire-rs
TC-715), where a green "CI" meant one of two suites. Re-runs at the same
timestamp are last-write-wins for ordinary runs; a retained differential-report
pair is immutable at its key. Flake history is analytics, not the record of
record.

### Auto-bind

First discharge binds without asking. Requiring a signature on something the
evidence already proves puts a gate in front of the common case and teaches
people to click through it.

## Inputs

- Normalized run entries keyed on FR-051 stable symbol identities; for the
  differential-report adapter, the accepted source report bytes that produced
  those entries
- A validated `quire coverage --json` payload, for the obligations of the day

## Outputs

- `spec/evidence/runs/<SUITE-N>/<timestamp>.json` — one run of one suite
- `spec/evidence/analysis-reports/<SUITE-N>/<timestamp>.json` — the canonical
  `quoin.analysis-report-projection/v1` allowlisted projection paired with a
  differential-report run at the same suite and timestamp
- `spec/evidence/bindings.json` — obligation → evidence

## Behavior

- Every write SHALL be canonical: key-sorted, stable ordering, two-space JSON
  with a trailing newline, so a PR diff of the store **is** the per-PR delta.
- The store SHALL bind an obligation only on a **passing** symbol. A failing or
  skipped result is evidence the suite ran, which the run record already holds;
  binding on it would make the store agree with a red build.
- The store SHALL join a run entry to a declared symbol by the entry's `path` and `qualified_name` (FR-051 identity). For an entry with no `path` whose qualified name matches symbols in more than one file, the store SHALL report the join ambiguous and join the entry to none of them; the trace ids on the entry still bind the criteria they name, because the mark is what binds.
- The store SHALL write a run record whatever its outcome. A suite that stopped
  passing is exactly what a freshness check needs to see.
- When the differential-report adapter accepts a report, then `quoin evidence
  record` SHALL retain its allowlisted `quoin.analysis-report-projection/v1`
  at
  `analysis-reports/<SUITE-N>/<timestamp>.json` and write its run at
  `runs/<SUITE-N>/<timestamp>.json`.
- The store SHALL pair the two files only by this exact canonical key; neither
  file may identify a different suite or timestamp.
- A retained differential-report projection/run pair SHALL be immutable while
  retained. Repeating a record operation for the same key and byte-identical
  projection and run is idempotent. If either member already exists and the operation would
  change either member, the operation SHALL refuse without changing either
  file. A reader SHALL refuse positive qualification if exactly one member
  exists, the projection is invalid, or it does not match the stored run's
  suite, timestamp, disposition, entry cardinality and mapped outcome
  (including the zero-entry requirement for disagreement). A report-backed
  run contains exactly one entry for agreement, inconclusive or unavailable;
  the projection has no entry identity and cannot select among run entries.
  Existing run records without
  a companion projection remain readable by legacy audit views but cannot qualify
  as analysis results.
- `gc` SHALL delete a differential-report companion only with its exact paired
  run; it SHALL NOT leave an unpaired report or delete a report while retaining
  its run.
- The `record` verb SHALL report a trace id matching no derived obligation
  rather than dropping it. That is the quire-rs#72 class arriving from the other
  direction: a test claiming to verify something the spec does not state.
- `gc` SHALL retain the **newest** run per suite — by `timestamp`, never by
  filename. Dir-per-suite keeps the policy local, so an expensive suite's
  retention or `.gitignore` decision touches one directory.
- An absent store SHALL read as empty rather than as an error.
- Every newly recorded run or scan SHALL carry an immutable tool version. A
  bare name or mutable label is refused before evidence intake, and a
  finding-format payload naming a different tool is refused rather than
  silently replacing the caller's attested identity.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-030-CON-1 | quoin SHALL NOT execute a consumer's suite. It transcribes; the consumer's CI runs (ADR-0011 invariant 1). | Architecture | Inspection |
| FR-030-CON-2 | The store SHALL NOT persist an obligation's statement, document or method — only its id. Anything re-derivable is re-derived, so the store cannot disagree with the spec. | Architecture | Test |
| FR-030-CON-3 | The store SHALL NOT contain a database or a derived index. Files only. | Architecture | Inspection |

### A tool reports the id it knows

A unit test carries the criterion's own id, because the tag is written in the
test. An **agent-eval report — and any tool keyed on the Test Matrix — carries
the test case id**: `TC-EV-057`, not `FR-038-AC-8`.

Both are stated by the same criteria row (`Eval (TC-EV-057)`), and quire-rs
FR-053-AC-11 carries that join on the obligation. The store resolves it rather
than re-parsing the criteria table, which is the duplication FR-050 exists to
prevent.

Before this, `quoin evidence record --adapter agent-eval` reported `bound: 0`
and `unmatched trace ids … TC-EV-057` while the join sat in the FR's own table
(agent-ix/quoin#144). 71 matrix rows across `spec/evals.md`, FR-028 and FR-038
were unbacked for want of it.

**A direct obligation id wins.** If a criterion's cell named a sibling
criterion's id, binding through the indirect route would report a discharge
nobody stated directly, so the indirect route is not registered for an id that
is itself an obligation.

**A test case discharging several criteria binds all of them.** The row says
each is verified by that test case; binding only the first would leave the rest
undischarged with the evidence sitting right there.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-030-AC-1 | The store root is `spec/evidence/`, and a run path is `runs/<SUITE-N>/<timestamp>.json`, the timestamp reduced to ASCII letters, digits and `-`. No run record carries a commit. A differential-report projection, when retained, is at `analysis-reports/<SUITE-N>/<timestamp>.json` under the same key; it contains no run-entry symbol or request digest, and the run's entry cardinality and outcome must match its disposition. Neither file adds a commit, derived content digest, CI association, signer principal or report field to `RunRecord`. | Test |
| FR-030-AC-2 | Writes are canonical and byte-identical across repeated serialization of the same record. | Test |
| FR-030-AC-3 | Two suites at one timestamp produce separate records. A re-run at the same timestamp is last-write-wins for ordinary runs. For a differential-report projection/run pair, identical replay is idempotent and any changed projection or run at the occupied key is refused with both files unchanged. | Test |
| FR-030-AC-4 | A passing symbol binds its obligation; a failing or skipped symbol binds nothing while its run is still recorded. | Test |
| FR-030-AC-5 | Withdrawn (CR note, 2026-10-29): required a reworded statement to make a binding suspect by comparing a hash stamped at binding. The store keeps no statement hash, so nothing remains to compare. | Withdrawn |
| FR-030-AC-6 | Withdrawn (CR note, 2026-10-29): required `evidence affirm` to move the stamped hash forward and record who affirmed. With no stamped hash there is nothing to re-affirm, and the verb is removed. | Withdrawn |
| FR-030-AC-7 | A trace id no obligation states is reported as unmatched rather than silently dropped. | Test |
| FR-030-AC-8 | `gc` deletes only runs that are not the latest for their suite, and `--dry-run` deletes nothing. The command declares no `--module` — gc never invokes quire and never loads a catalog — and its help says why. | Test |
| FR-030-AC-9 | An absent store reads as an empty binding graph, an empty run list and an empty collection. | Test |
| FR-030-AC-10 | No obligation statement, document or method appears anywhere in the written store — only the id (CON-2). | Test |
| FR-030-AC-11 | A binding is keyed on `(obligation, suite)`: a second suite discharging the same obligation **appends** a binding rather than replacing the first, and re-discharging the same suite merges into its own binding. `bindings.json` contains both suite bindings for the obligation. | Test |
| FR-030-AC-12 | The **latest** run of a suite is the newest by `timestamp`, never the lexicographically last filename. `gc` retains that run, and the auditor reads it. Withdrawn: ordering of two runs that share a timestamp by their commit; a shared timestamp names one file and the last write wins. | Test |
| FR-030-AC-13 | A store file that exists and is not readable JSON raises a diagnostic naming the file and the cause, never a bare `SyntaxError`. One unreadable **run** file is skipped and reported rather than fatal; the binding graph is not, because reading an empty graph would report every obligation as undischarged. | Test |
| FR-030-AC-14 | Store ordering is locale-independent: written bytes are fixed by test, so a runtime's collation data cannot change the diff of a checked-in file. | Test |
| FR-030-AC-15 | A run's trace id binds through an obligation's declared test cases as well as its own id, so a tool keyed on the Test Matrix discharges the criteria that name it. A direct obligation id wins over the indirect route, and an id no obligation states by either route is still reported unmatched. | Test |
| FR-030-AC-16 | A completed source-level mock inspection is recorded at `mock-inspections/<SUITE-N>/<timestamp>.json`, including an empty result. Readers select the newest record of each suite: an empty record means the suite was inspected and no stand-in was observed; no record means the check was not evaluated. Withdrawn: selection by exact full commit. | Test |
| FR-030-AC-17 | `evidence record` refuses a bare or mutable `--tool` identity and accepts an exact semantic version. Withdrawn: acceptance of a full source revision (a git commit id); tool identity is the exact semantic version, with no sha. First-party Vitest and mock-inspection records include the version they actually ran. | Test |
| FR-030-AC-18 | A run entry with no `path`, whose qualified name matches symbols in more than one file, is reported ambiguous and joined to none of them, while its trace ids still bind. An entry with a `path` joins only the symbol at that path. | Test |
| FR-030-AC-19 | Recording a differential report validates the source report before retaining only its allowlisted projection beside the matching run key; query bytes, model, solver/proof output and undeclared fields are absent. A one-sided write, changed replay, wrong projection or invalid source report never yields a qualified pair, and `gc` removes both members together. | Test |

> **CR note (PLAT-1228, 2026-10-30):** Run, scan, mock-inspection and binding
> records no longer carry a commit. Filenames derive from the record's
> timestamp, a binding no longer records the run that first discharged it, `gc`
> keeps only the newest record per suite, and the exact-commit selection of mock
> inspections and the tie-break by commit are withdrawn inside FR-030-AC-12 and
> FR-030-AC-16. The ids are not reused.
>
> **CR note (2026-10-29, PLAT-1213):** The binding hash mechanism is removed.
> A binding no longer stamps the obligation's statement hash, the store keeps no
> affirmation history, and `quoin evidence affirm` and the `suspect` list of
> `quoin evidence record` are gone. FR-030-AC-5 and FR-030-AC-6 are withdrawn,
> with TC-123 and TC-124; the other ACs lose their hash and affirmation clauses.
> The ids are not reused. quire-rs no longer emits `statement_hash` on the
> `assurance-v1` export, and quoin reads none.
>
> **CR note (2026-09-30, agent-ix/quoin#658):** `spec/evidence/baseline.json`
> is no longer a store output, and FR-030-AC-13 no longer names it. The ratchet
> that read and wrote it is removed (FR-032 CR note).

## Dependencies

- **Upstream**: [FR-029](./FR-029-consume-the-quire-json-contract.md) (the validated payload the obligations arrive on), quire-rs [FR-053](ix://agent-ix/quire-rs/FR-053) (the obligation records), `spec-artifacts-process` FR-006 (the authored registries) and FR-007-AC-8 (the declared obligation sources, without which the machinery is inert)
- **Downstream**: the freshness / vacuous-evidence auditor (agent-ix/quoin#80), which reads this store and runs nothing; the evidence adapters (agent-ix/quoin#91), which normalize tool output into the run entries this verb accepts
