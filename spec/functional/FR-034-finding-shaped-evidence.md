---
id: FR-034
title: "Finding-shaped evidence: a clean scan and an unrun scan must not look alike"
type: FR
relationships:
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
