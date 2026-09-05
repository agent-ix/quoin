---
id: PLAN-009
title: "Advisory corpus measurement"
type: Plan
status: active
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: references
  - target: "ix://agent-ix/quoin/FR-084"
    type: references
  - target: "ix://agent-ix/quoin/FR-085"
    type: references
  - target: "ix://agent-ix/quoin/FR-086"
    type: references
  - target: "ix://agent-ix/quoin/FR-087"
    type: references
  - target: "ix://agent-ix/quoin/FR-088"
    type: references
  - target: "ix://agent-ix/quoin/FR-089"
    type: references
  - target: "ix://agent-ix/quoin/FR-090"
    type: references
  - target: "ix://agent-ix/quoin/FR-091"
    type: references
  - target: "ix://agent-ix/quoin/FR-092"
    type: references
  - target: "ix://agent-ix/quoin/NFR-021"
    type: references
  - target: "ix://agent-ix/quoin/NFR-022"
    type: references
  - target: "ix://agent-ix/quoin/NFR-023"
    type: references
---

# PLAN-009: Advisory corpus measurement

Issue: `agent-ix/quoin#291`, under EPIC `agent-ix/quoin#286`. Reviews: SR-140..SR-148
under `reviews/26-09-04-spec-review-*-291.md`. Upstream: the semantic-module contract
(`agent-ix/quoin#293`, merged `3e842ce`) and the nine module schema completions of this
wave. Downstream: the promotion gate `agent-ix/quoin#290`, which has no other evidence
to consume.

**Advisory and report only.** No corpus repository is edited by this plan, no schema is
weakened to lower a count, and a high failure rate pauses promotion rather than
justifying a looser rule.

## Requirements Summary

### User Story

- [ ] **US-022:** The campaign owner gets a deterministic, advisory measurement of the whole governed corpus against every completed module schema, each failure carrying an owner and a disposition.

### Functional Requirements

- [ ] **FR-084:** Enumerate and pin the governed corpus under a declared exclusion vocabulary, with the worktree, nested-repository and symlink rules, and record cleanliness and stability per repository.
- [ ] **FR-085:** Resolve the declared required module set from each module repository's object store, record contract-surface digests, check declared `data_schema` digests, and publish the toolchain record.
- [ ] **FR-086:** Give every enumerated document exactly one state from `measured`, `out-of-model`, `unreadable` and `contested`, resolving types case-sensitively on a module-qualified key.
- [ ] **FR-087:** Obtain structural conformance by running the Quire engine over each measured document against the resolved module set, recording `pass`, `fail` or `could-not-run` and never implementing a second extractor.
- [ ] **FR-088:** Publish the Properties form census over the FR-084 population, and record the field-level semantic dimension `could-not-run` while no released toolchain exposes it.
- [ ] **FR-089:** Partition every finding into the eight classes with an owner and a disposition, on a line-independent identity, reporting unmatched ledger entries.
- [ ] **FR-090:** Publish every rate with its unit, population and method, partitioned by module, type and repository, with the divergence list and the figure index.
- [ ] **FR-091:** Record each known tool defect as a cited exclusion and report the population share it covers.
- [ ] **FR-092:** Exit zero whatever is measured, write only under the declared output directory, and leave every corpus and module repository byte-identical.

### Non-Functional Requirements

- [ ] **NFR-021:** Repeated runs over the committed fixture corpus produce digest-identical artifacts, including with networking disabled and in a shuffled enumeration order.
- [ ] **NFR-022:** A full run over up to 30,000 documents in 300 repositories completes inside 15 minutes on the recorded reference machine, opening every corpus file read-only.
- [ ] **NFR-023:** Every figure the report prints names the artifact and field it came from and equals the value recomputed from it.

## Dependency Graph

```text
TASK-060 -> TASK-061 -> TASK-062 -> TASK-063 -> TASK-066 -> TASK-067 -> TASK-069
        \-> TASK-064 ------------------------/        /
        \-> TASK-065 ------------------------/       /
                     TASK-068 ----------------------/
```

- TASK-060 is enablement: nothing can be measured before the population exists and the read-only envelope holds.
- TASK-061 gives every later task the contract and the toolchain identity it reports against.
- TASK-063 (engine-driven measurement) needs the states TASK-062 assigns; TASK-064 (form census) and TASK-065 (tool-defect ledger) need only the population and the toolchain record, so both run parallel to it.
- TASK-066 consumes every finding stream, so it waits on TASK-063, TASK-064 and TASK-065.
- TASK-067 publishes; TASK-068 makes the publication reproducible and bounded and joins before it.
- TASK-069 runs the real measurement, publishes the report, files what it found, and closes the cycle.

## Execution Tracks

| Track | Tasks | Runs |
| --- | --- | --- |
| A — population and contract | TASK-060, TASK-061, TASK-062, TASK-063 | serial |
| B — semantic census | TASK-064 | parallel with A after TASK-061 |
| C — tool defects | TASK-065 | parallel with A after TASK-061 |
| D — partition and publication | TASK-066, TASK-067 | after A, B and C |
| E — evidence and close | TASK-068, TASK-069 | TASK-068 parallel from TASK-060; TASK-069 last |

## Quality Gates

| Gate | Condition | Blocks |
| --- | --- | --- |
| G1 — population stated | The enumerated document count, the exclusion vocabulary and every repository pin are recorded, and the per-repository counts sum to the total. | TASK-061 onward |
| G2 — contract pinned | Every required module resolves to a commit, every contract surface carries a digest, and the toolchain record names the engine and CLI revisions. | TASK-062 onward |
| G3 — no second extractor | No source file of the measurement parses a module mapping, builds a semantic record, or resolves a type, multiplicity or constraint keyword. | TASK-066 |
| G4 — nothing hidden | Every document holds exactly one state, every finding holds exactly one class, `could-not-run` never enters a rate, and the `unknown` and `undispositioned` counts are headline figures. | TASK-067 |
| G5 — reproducible | Two runs over the fixture corpus are digest-identical, an offline run agrees, and a shuffled enumeration order agrees. | TASK-069 |
| G6 — repo green | `make lint` passes, `npx vitest run` passes apart from the four failures already failing on `main`, `quire validate --scope . "spec/**/*.md"` is structurally clean, and every corpus repository is untouched. | the pull request |

## Test Plan

| Test group | File | Test cases |
| --- | --- | --- |
| Enumeration and pinning | `tests/corpus-measurement/enumeration.test.ts` | TC-1500..TC-1505, TC-1566, TC-1567 |
| Module resolution and toolchain | `tests/corpus-measurement/modules.test.ts` | TC-1506..TC-1511, TC-1568..TC-1571 |
| Document states | `tests/corpus-measurement/states.test.ts` | TC-1512..TC-1517, TC-1573, TC-1574 |
| Engine-driven measurement | `tests/corpus-measurement/engine.test.ts` | TC-1518..TC-1524, TC-1572 |
| Form census and the L3 gap | `tests/corpus-measurement/representation.test.ts` | TC-1525..TC-1530 |
| Partition | `tests/corpus-measurement/partition.test.ts` | TC-1531..TC-1537, TC-1575..TC-1577 |
| Rates, report and figure index | `tests/corpus-measurement/publication.test.ts` | TC-1538..TC-1544, TC-1563..TC-1565, TC-1578..TC-1580 |
| Tool-defect ledger | `tests/corpus-measurement/tool-defects.test.ts` | TC-1545..TC-1550, TC-1581, TC-1582 |
| Advisory and read-only envelope | `tests/corpus-measurement/envelope.test.ts` | TC-1551..TC-1556, TC-1583 |
| Reproducibility and budget | `tests/corpus-measurement/reproducibility.test.ts` | TC-1557..TC-1562, TC-1584 |

Five criteria are discharged by inspection of the measurement's own source rather
than by a test, because each asserts the absence of code: FR-087-AC-9,
FR-087-CON-2, FR-088-AC-7, FR-088-CON-2 and FR-089-CON-3. A test can witness
behaviour that exists; it cannot witness that a second implementation was never
written. Gate G3 is where that inspection is recorded.

## Out of Scope

- Editing, normalizing or migrating any corpus repository. The findings feed a later
  normalization campaign, named per deferred finding, that this plan does not start.
- Promoting any module constraint from advisory to enforcing. That is
  `agent-ix/quoin#290`, and this plan produces the evidence it consumes rather than
  taking its decision.
- Field-level semantic conformance, until a released toolchain exposes the
  `agent-ix/quire-rs#388` extraction. Until then it is reported `could-not-run` with a
  citation, which is the honest state and not a gap in this plan.
- Publishing any package. Nothing here is released.
