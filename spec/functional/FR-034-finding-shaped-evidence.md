---
id: FR-034
title: "Finding-shaped evidence: a clean scan and an unrun scan must not look alike"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "requires"
---
# FR-034: Finding-shaped evidence

## Description

`RunEntry` is `{symbol, outcome, score?, traceIds?}`. Five of the eight formats quoin means to read
emit **findings, not run outcomes**: semgrep, SARIF, cargo-audit/deny and ZAP have no symbol and no
pass/fail.

Forced into `RunEntry`, **a clean semgrep run and a semgrep run that never executed are
indistinguishable** — both have zero failing entries. An evidence store whose whole purpose is
telling *verified* from *looks verified* cannot carry that ambiguity. It would be the
green-matrix-over-dead-links defect reproduced inside the store built to prevent it.

quoin SHALL carry a second record type, `FindingRecord`, alongside `RunRecord`.

### The distinction lives on the record, not in the findings

A `FindingRecord` is written **only when an adapter read a real report envelope**, so the record's
existence is the proof the scan executed:

| State | Store | Meaning |
|---|---|---|
| Scan ran, found nothing | record present, `findings: []` | **evidence** |
| Scan never ran | no record | **not evidence** |

Real tool output already works this way, which is why the design follows it rather than inventing
one: `cargo audit --json` emits `vulnerabilities.found: false` beside the advisory `database` and
`lockfile` it consulted, and a SARIF `run` with an empty `results` array is still a run. A SARIF log
whose `runs` array is **empty** is rejected — that file proves nothing executed, and recording it
would manufacture the exact evidence this record type exists to distinguish.

### Vacuity means something different here

For a run, vacuity is *every bound symbol skipped*. For a scan there are no symbols, so that
definition is not merely wrong — it is inapplicable, and leaving `vacuous-evidence` silent on this
record type would let the store's most valuable check go quiet exactly where the new ambiguity lives.

**A finding-shaped scan is vacuous when it evaluated no rules.** It reports zero findings too, and
from the findings list alone the two are identical — but one looked and found nothing, and the other
never looked. `rulesEvaluated` carries the difference.

When the tool does not say how many rules it evaluated, the question **cannot be asked** and the
check SHALL stay silent rather than guess — the same posture method conformance takes when no
evidence kind is declared (`agent-ix/quoin#105`).

### Freshness reuses the fix, it does not rebuild it

Scan ordering is by `timestamp`, with the commit as tiebreak — identical to `readRuns`. A filename
is a commit prefix and a commit prefix is uniformly random hex, so lexical order is not time order.
`agent-ix/quoin#104` fixed that once for runs; reintroducing it here would rebuild the same defect
beside its fix.

### SARIF first, decided from real output

The ticket asked for this to be decided *"by reading real output from each tool, not from the spec of
the format"*.

**SARIF is the primary adapter** — semgrep (`--sarif`), CodeQL, ESLint and ZAP (via its converter)
all converge on it, so one robust reader subsumes several bespoke ones.

**`cargo-audit` keeps a bespoke adapter for a measured reason: it emits no SARIF at all**, so the
SARIF reader cannot subsume it, and its native JSON is what a consumer's CI produces. Its warning
kinds (`unsound`, `unmaintained`, `yanked`) are kept as their own severity rather than flattened —
those are distinctions the tool drew, and quoin normalizes no scanner's severities.

The cargo-audit adapter is verified against output captured with `cargo audit --json` and checked in
**unedited**. A fixture written to match the reader only proves the reader parses itself.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-034-AC-1 | A SARIF run with an empty `results` array is read as a scan that executed, reporting the tool and how many rules it evaluated. | Test (TC-165) |
| FR-034-AC-2 | A SARIF log whose `runs` array is empty is **rejected** — it proves no scan executed. | Test (TC-166) |
| FR-034-AC-3 | The SARIF adapter reads rule id, level, message and first location. | Test (TC-167) |
| FR-034-AC-4 | The nested `rule.id` form is accepted; a result naming no rule at all is skipped. | Test (TC-168) |
| FR-034-AC-5 | Malformed SARIF and a log with no `runs` array are rejected. | Test (TC-169) |
| FR-034-AC-6 | The cargo-audit adapter parses **real** captured `cargo audit --json` output, reading the advisory-database size as the rule count. | Test (TC-170) |
| FR-034-AC-7 | Each cargo-audit warning kind is kept as its own severity rather than flattened. | Test (TC-171) |
| FR-034-AC-8 | Malformed input and output that is not cargo-audit's are rejected; an advisory with no id is skipped. | Test (TC-172) |
| FR-034-AC-9 | A clean scan discharges its binding: the obligation is neither `undischarged` nor `vacuous-evidence`. | Test (TC-173) |
| FR-034-AC-10 | A scan that evaluated **no rules** is reported `vacuous-evidence` at `high`. | Test (TC-174) |
| FR-034-AC-11 | When the tool reports no rule count the vacuity check stays silent. | Test (TC-175) |
| FR-034-AC-12 | Every run-shaped check pairs a binding with **its own** suite's run when a scan is also bound to the same obligation. | Test (TC-176) |
| FR-034-AC-13 | `quoin evidence record --adapter sarif --results <file>` writes a `FindingRecord` under `scans/` and writes **nothing** under `runs/`. | Test (TC-177) |
| FR-034-AC-14 | A clean scan recorded through the command keeps `findings: []` and its rule count, so zero findings is still evidence. | Test (TC-178) |
| FR-034-AC-15 | The command selects a finding adapter from `--tool` when none is named. | Test (TC-179) |
| FR-034-AC-16 | `--discharges` binds the obligations a scan was run to check. A clean scan carries no finding to bind from, so they are stated rather than inferred. | Test (TC-192) |
| FR-034-AC-17 | A scan that evaluated **no rules** binds nothing, whatever `--discharges` names. | Test (TC-193) |
| FR-034-AC-18 | A suite that recorded only scans is enumerated by `listRecordedSuites`. | Test (TC-194) |
| FR-034-AC-19 | `gc` collects superseded scans, keeping the newest by timestamp. | Test (TC-195) |
| FR-034-AC-20 | A tool reporting **zero** rules is distinguishable from a tool reporting **no** rule count. | Test (TC-196) |

### Reachability is part of the contract

Every criterion above that describes recording SHALL be stated over
`quoin evidence record`, not over the parse function.

This is not a style preference. The first cut of this FR shipped `FindingRecord`, both adapters and
`writeScan` with **no command that could reach any of them** — a capability nothing could use, and a
Test Matrix reading ✅ over it. That is precisely the defect the P1 review found three times, and the
reason `agent-ix/quoin#115` specified its acceptance shape the way it did. Caught by the Wave C
review rather than by the tests, which is itself the finding.

A finding-shaped adapter is selected **before** anything is parsed, because it writes a different
record type: letting a scan fall through to the run path would put it in `runs/` and lose the
clean-versus-unrun distinction at the point of intake, silently and permanently for that commit.

### Reachable from every side, not only the write side

A record type is not integrated when it can be written. It is integrated when everything that reads
the store knows it exists.

`SR-005` found FR-034 **inert end to end**: `writeScan` wrote a record that bound no obligation, the
audit command never passed `scans`, `listRecordedSuites` read only `runs/`, and `gc` collected only
`runs/`. The record was written and nothing else in the system could see it, so
`vacuous-evidence` — the check this FR exists to make possible — could not fire in any real run.

AC-16..20 exist because each of those paths had **no criterion at all**. The write half was stated
over the command and passed; no criterion asked whether anything ever read what it wrote.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-034-CON-1 | A `FindingRecord` SHALL be written only for a report envelope an adapter actually read. Writing one for an absent scan destroys the single distinction this record type exists to make. | Design | Inspection |
| FR-034-CON-2 | quoin SHALL NOT normalize a scanner's severity strings. Scanners disagree about what "high" means; translating would invent a comparison the tools never made. | Design | Test (TC-171) |
| FR-034-CON-3 | Judging whether a reported finding is *acceptable* stays with the consumer's gate policy. | Design | Inspection |
| FR-034-CON-4 | Scan ordering SHALL be by timestamp, never by commit prefix (`agent-ix/quoin#104`). | Design | Inspection |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md), [FR-033](./FR-033-evidence-format-adapters.md)
- **Downstream**: `agent-ix/quoin#91c` (inventories), `agent-ix/quoin#80` (`vacuous-evidence` consumers)
