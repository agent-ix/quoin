---
id: SR-005
title: "Code review — ADR-0011 Phase 2 Waves C and D (FR-033, FR-034, FR-035)"
type: SpecReview
analysis: code-review
scope: "src/evidence/adapters/, src/evidence/types.ts, src/evidence/store.ts, src/auditor/audit.ts, src/auditor/combinatorial.ts, src/advisor/advise.ts, src/commands/evidence/"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-034"
    type: "reviews"
---

## Summary

Reviewed the Waves C and D changes as one diff — FR-033 (evidence adapters), FR-034 (finding-shaped
evidence) and FR-035 (t-way coverage). **FR-034 is inert end to end**: a `FindingRecord` is written
and nothing else in the system can see it. Five findings, four of them high. Four share one root cause; the fifth was found by writing the test for the first.

This review exists because it was skipped. Waves A and B produced `SR-045`–`SR-048` in `quire-rs`;
Waves C and D produced nothing here, and an unstructured pass replaced the gate. That pass caught the
_write_ side of this same defect (PR #121) and never checked the read side.

## Verdict

**FAIL** — four `high` findings. The feature ships, is tested, and cannot do its job.

## Findings

| ID      | Severity | Summary                                                                                                          | Refs                                |
| ------- | -------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FND-001 | high     | The scan path writes a record and binds no obligation, so a scan discharges nothing                              | src/commands/evidence/record.ts:104 |
| FND-002 | high     | `quoin evidence audit` never passes `scans`, so no `FindingRecord` reaches the auditor                           | src/commands/evidence/audit.ts:83   |
| FND-003 | high     | `listRecordedSuites` reads only `runs/`, so a scan-only suite is never enumerated                                | src/evidence/store.ts:265           |
| FND-004 | medium   | `gc` walks only `RUNS_DIR`, so scan records are never collected and the store grows unbounded                    | src/evidence/store.ts:404           |
| FND-005 | high     | The SARIF adapter omits `rulesEvaluated` when it is 0, conflating "reported zero rules" with "reported no count" | src/evidence/adapters/sarif.ts:74   |

## Detail

### One root cause, four symptoms

FR-034 added a second record type and wired it into `audit()`'s _pure function_. Everything the tests
exercise is the function. **Nothing that reads the store was taught the new directory exists.**

| Path      | Run-shaped                         | Finding-shaped                          |
| --------- | ---------------------------------- | --------------------------------------- |
| write     | `recordRun` → writes **and binds** | `writeScan` → writes, **binds nothing** |
| enumerate | `listRecordedSuites` → `runs/`     | —                                       |
| audit     | `latestRuns()` → `audit({runs})`   | —                                       |
| collect   | `gc` → `RUNS_DIR`                  | —                                       |

A scan is therefore recorded, bound to no obligation, invisible to the auditor, and never collected.
`vacuous-evidence` for a rule-less scan — the check FR-034 exists to make possible — cannot fire in
any real run.

### Why the tests did not catch it

Every FR-034 criterion is stated over `audit(...)` with a hand-built `scans:` array, or over
`parseSarif`. `TC-177`–`TC-179` do reach the command, but only the **write** half: they assert a
file appears in `scans/`, which it does.

**No criterion asserts that anything ever reads it.** A capability is reachable when a user's
invocation reaches it, not when a test constructs the input the capability expects — and that
distinction is exactly what `agent-ix/quoin#115` was written to force.

### The pattern is now three-for-three

Wave C's PR #121 fixed this same class on the write path. The P1 review found it three times. It has
now appeared once per wave in this program, and every occurrence has been _a Test Matrix reading ✅
over something no invocation can reach_.

The mechanical tell is available and cheap: **for each new capability, grep `src/commands/` for the
symbol.** If no command names it, the capability is unreachable regardless of how many tests pass.

### FND-005 — found by writing the test, not by reading the code

The vacuity fix could not be tested until a scan could _be_ vacuous, and it could not: the SARIF
adapter initialised its counter to `0` and then omitted the field when it was still `0`. A scan
declaring `"rules": []` — a tool explicitly stating it evaluated none — was indistinguishable from a
tool that reported no count at all.

That erases the exact distinction FR-034 turns on. `scanIsVacuous` returns `null` for an absent count
and stays silent, so **the vacuity check was silent on the one input it exists to catch**.

Worth recording how it surfaced: in the adapter, the omission reads as ordinary tidiness. It became
visible only when a test asserted the behaviour end to end and the binding it forbade happened anyway.

## Coverage

`make build` ✅ · `make test` ✅ 384 passed (379 at review time, +5 for the fixes) · `make lint` ✅ — none of which can see this class of
defect, which is the point of the finding.

The Python-shaped sections of the review skill (Test Standards, Mock Compliance, both Completeness
sections) were **not** run: this change is TypeScript, and transliterating them produces false
findings that bury real ones. The `rust-review` sub-skill the dispatch table names is also absent
from the installed skill — recorded so the gap is visible rather than silently skipped.
