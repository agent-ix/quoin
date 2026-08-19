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
run, re-affirm a binding, and collect unreferenced records.

Every quoin verification surface recomputes its view from scratch and emits a
report; nothing persisted what was checked. That is the "green matrix over dead
links" failure mode, already observed at ecosystem scale: **1,014 trace tags
binding to nothing while matrices read as covered**
([quire-rs#72](ix://agent-ix/quire-rs/FR-050)).

### Governing principle: store only what cannot be recomputed

Run outcomes, hash-at-binding, affirmations and the ratchet baseline are facts
about the past — nothing can re-derive them tomorrow. **Obligations are not.**
They are re-derived live from quire ([FR-053](ix://agent-ix/quire-rs/FR-053))
on every read, so they cannot drift out of agreement with the spec. A stored
copy would be one more thing to keep in sync, and the thing it would most likely
disagree with is the requirement itself.

### Authored → markdown; machine-transcribed → JSON

`suites.md` and `inspections.md` are validated corpus documents
(`spec-artifacts-process` FR-006). `bindings.json`, `baseline.json` and
`runs/**` are machine-written, typeless and corpus-invisible; their shapes are
quoin's.

No SQLite anywhere. quoin writes files; parsing, loading and indexing for an
application is `filament-ide-rs` territory, and a derived index inside the store
would be a second source of truth for facts the files already hold.

### The store lives under `spec/`

quire-rs CR-045 bounds the document walk to `<scope>/spec`, so the authored half
is only a validated corpus document if it lives there. **[RAN]** the alternative:
a typed, well-formed registry at the repository root minted nothing and was
reported nowhere, while the identical file under `spec/` was walked, validated
and minted its ids. agent-ix/quoin#79's original layout rested on the premise
that quire "validates them wherever they live", which measurement contradicts.

### The suite is the atomic unit

One file per (suite, commit); aggregation is a view. A partial run must never be
able to masquerade as a full one — the `make ci` / `make ci-python` split
already caused exactly that rot (quire-rs TC-715), where a green "CI" meant one
of two suites. Re-runs at the same commit are last-write-wins: flake history is
analytics, not the record of record.

### Auto-bind, explicit affirmation

First discharge binds without asking. Requiring a signature on something the
evidence already proves puts a gate in front of the common case and teaches
people to click through it. What stays explicit is **re-affirmation** after a
statement changes — that is the judgement call, and the one worth a name against
it.

Re-running a test SHALL NOT clear suspicion. If it did, the suspect state would
clear itself on the next CI run and the detector would never fire.

## Inputs

- Normalized run entries keyed on FR-051 stable symbol identities
- A validated `quire coverage --json` payload, for the obligations of the day

## Outputs

- `spec/evidence/runs/<SUITE-N>/<commit12>.json` — one run of one suite
- `spec/evidence/bindings.json` — obligation → hash-at-binding → evidence
- `spec/evidence/baseline.json` — the accepted violation set

## Behavior

- Every write SHALL be canonical: key-sorted, stable ordering, two-space JSON
  with a trailing newline, so a PR diff of the store **is** the per-PR delta.
- The store SHALL bind an obligation only on a **passing** symbol. A failing or
  skipped result is evidence the suite ran, which the run record already holds;
  binding on it would make the store agree with a red build.
- The store SHALL write a run record whatever its outcome. A suite that stopped
  passing is exactly what a freshness check needs to see.
- The `record` verb SHALL report a trace id matching no derived obligation
  rather than dropping it. That is the quire-rs#72 class arriving from the other
  direction: a test claiming to verify something the spec does not state.
- `gc` SHALL retain the **newest** run per suite — by `timestamp`, never by
  filename — plus anything a binding references. Dir-per-suite keeps the policy local, so an expensive suite's
  retention or `.gitignore` decision touches one directory.
- An absent store SHALL read as empty rather than as an error.
- Every verb SHALL check the quire version premise ([FR-029](./FR-029-consume-the-quire-json-contract.md))
  before reading a payload.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-030-CON-1 | quoin SHALL NOT execute a consumer's suite. It transcribes; the consumer's CI runs (ADR-0011 invariant 1). | Architecture | Inspection |
| FR-030-CON-2 | The store SHALL NOT persist an obligation's statement, document or method — only its id and hash. Anything re-derivable is re-derived, so the store cannot disagree with the spec. | Architecture | Test |
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
| FR-030-AC-1 | The store root is `spec/evidence/`, and a run path is `runs/<SUITE-N>/<commit12>.json`. | Test (TC-119) |
| FR-030-AC-2 | Writes are canonical and byte-identical across repeated serialization of the same record. | Test (TC-120) |
| FR-030-AC-3 | Two suites at one commit produce two files, and a re-run at the same commit is last-write-wins with one file retained. | Test (TC-121) |
| FR-030-AC-4 | A passing symbol binds its obligation with the hash as it stands now; a failing or skipped symbol binds nothing while its run is still recorded. | Test (TC-122) |
| FR-030-AC-5 | A reworded statement makes an existing binding suspect, and re-recording the same passing run does not overwrite the hash. | Test (TC-123) |
| FR-030-AC-6 | Affirmation moves the hash forward and records who and at which commit; affirming an unknown obligation reports rather than inventing a binding. | Test (TC-124) |
| FR-030-AC-7 | A trace id no obligation states is reported as unmatched rather than silently dropped. | Test (TC-125) |
| FR-030-AC-8 | `gc` deletes only runs that are neither the latest for their suite nor referenced by a binding, and `--dry-run` deletes nothing. | Test (TC-126) |
| FR-030-AC-9 | An absent store reads as an empty binding graph, an empty run list and an empty collection. | Test (TC-127) |
| FR-030-AC-10 | No obligation statement, document or method appears anywhere in the written store — only the id and hash (CON-2). | Test (TC-128) |
| FR-030-AC-11 | A binding is keyed on `(obligation, suite)`: a second suite discharging the same obligation **appends** a binding rather than replacing the first, re-discharging the same suite merges into its own binding, and affirmation clears every suite's suspicion unless narrowed to one. The graph is cross-suite, so the file must be able to hold two. | Test (TC-129) |
| FR-030-AC-12 | The **latest** run of a suite is the newest by `timestamp`, never the lexicographically last filename: a run filename is a commit prefix, which carries no time. `gc` retains that run, and the auditor reads it. Two runs sharing a timestamp order by commit, so a tie resolves the same way on every machine. | Test (TC-130) |
| FR-030-AC-13 | A store file that exists and is not readable JSON raises a diagnostic naming the file and the cause, never a bare `SyntaxError`. One unreadable **run** file is skipped and reported rather than fatal; the binding graph and the baseline are not, because reading an empty graph would report every obligation as undischarged. | Test (TC-131) |
| FR-030-AC-14 | Store ordering is locale-independent: written bytes are pinned by test, so a runtime's collation data cannot change the diff of a checked-in file. | Test (TC-132) |
| FR-030-AC-15 | A run's trace id binds through an obligation's declared test cases as well as its own id, so a tool keyed on the Test Matrix discharges the criteria that name it. A direct obligation id wins over the indirect route, and an id no obligation states by either route is still reported unmatched. | Test (TC-245) |

## Dependencies

- **Upstream**: [FR-029](./FR-029-consume-the-quire-json-contract.md) (the validated payload the obligations arrive on), quire-rs [FR-053](ix://agent-ix/quire-rs/FR-053) (the obligation records and their hashes), `spec-artifacts-process` FR-006 (the authored registries) and FR-007-AC-8 (the declared obligation sources, without which the machinery is inert)
- **Downstream**: the suspect-link / freshness / vacuous-evidence auditor (agent-ix/quoin#80), which reads this store and runs nothing; the evidence adapters (agent-ix/quoin#91), which normalize tool output into the run entries this verb accepts
