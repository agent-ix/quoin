---
id: FR-153
title: "An analysis result counts toward an obligation only as a qualified passing record"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-152"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-151"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-116"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-093"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-078"
    type: "requires"
---

# FR-153: An analysis result counts toward an obligation only as a qualified passing record

## Description

An analysis result bound to an obligation by FR-152 is a retained record. This requirement states when that record
counts as coverage and trace for the obligation, per disposition of the report (FR-151). The counting rules
themselves are quire-rs [FR-078](ix://agent-ix/quire-rs/FR-078) and the owner qualification
contract [Quoin FR-116](./FR-116-owner-qualified-quire-evidence.md); they are cited here and not restated. This
requirement adds only what is specific to an analysis entry.

### What counts

Coverage credit for an obligation backed by an analysis result is the FR-078 `qualified-record` status and nothing
else. A binding in the store, a passing entry and a healthy `quoin evidence audit` result each fall short of it, as
FR-116 states for a healthy audit. Credit therefore needs, in this order:

1. an entry whose obligation the recorder stated with `--discharges` (FR-152, quoin);
2. a passing entry whose binding survives the FR-032 audit of the latest run (quoin);
3. a positive FR-116 decision for the exact selected target and its declared method, from the planned qualifier and
   `quoin-quire` bridge (quoin; planned, not built);
4. FR-078 reconciliation of that decision as a `record_binders` claim (quire-rs; built).

### What each disposition counts as

| Report | Entry (FR-151) | Binding (FR-152) | FR-078 category | Counted |
| --- | --- | --- | --- | --- |
| `agreement`, `satisfied` | `pass` | created when the recorder stated the obligation | qualified claim, or the `unevaluated` / `qualification-rejected` cause of the premise that fails | Only as a qualified claim |
| `agreement`, `refuted` | `fail` | none | `failed` | No |
| `inconclusive` | `error` | none | `errored` | No |
| `unavailable` | `error` | none | `errored` | No |
| `disagreement` | none; `unrepresented` result, run with zero entries | none | `absent` | No |

`inconclusive` and `unavailable` are the same category because the entry does not tell them apart (FR-151); the
retained report does. `disagreement` is `absent` because the zero-entry run holds no result for the obligation; the
`unrepresented` result is printed and not stored (FR-151), so the stored evidence cannot say that two engines
disagreed.

### Qualification of an analysis act

The entry's `evidenceKind` is unset (FR-151), and a stated `evidenceKind` establishes nothing as Quoin FR-116 states,
so neither supplies a qualification premise. Qualification of an analysis entry needs a producer definition that
Quoin FR-116 requires the owner to have approved for this act. The owner has approved it
([FR-184](./FR-184-analysis-act-producer-definition.md)). An analysis-backed obligation is `unevaluated`, and none is
counted, while no FR-093 assessment exists (the bounded-reliance premise) or when the run-package step has no CI
checkout SHA (the performed-revision premise). The credit is consistency evidence only.

### What is decided here and what is not

Decided here, from the text of FR-152, FR-116 and FR-078:

- The binding source is the `--discharges` option of FR-152. A declared target or a plan naming the analysis request
  is not a binding source in this version; adding one would be a change to FR-152.
- Quoin does not judge whether a `satisfied` report analyses the statement of the obligation it is stated for. The
  recorder's assertion is evaluated only by the actor-authority premise of Quoin FR-116.
- A `disagreement` is not distinguishable from an absent result after recording, because the `unrepresented` result is
  printed and not stored (FR-151). A store change under FR-030 would be a separate requirement.

Ruled by the owner, not yet specified in quire-rs:

- Waivers. The FR-116 owner ruled (relayed by the planner 2026-11-13), option A: a waiver is a named person's
  decision with an approval reference, signed through the signed-records feature, reported as its own waiver outcome
  and never as a pass. Strict mode still fails by default; a policy setting must opt in before approved waivers pass.
  It reuses the person-owned disposition pattern of the advisory join (PLAT-1516) and the anomaly release rationale
  (PLAT-1584). The quire-rs FR-078 side still needs its own spec change from the IR and quire-rs owners; until it
  lands nothing is waivable (FR-184 item 9).

## Behavior

- The bridge SHALL ask the FR-116 qualifier about an analysis entry with the obligation's declared method and the
  producer definition the owner has approved for this act.
- The bridge SHALL count an analysis-backed obligation as covered only by a positive FR-116 decision, reconciled by
  FR-078 as a `record_binders` claim and status `qualified-record`.
- The bridge SHALL correlate a requested target to analysis entries by the `traceIds` stored on the run entries of
  any outcome, so a failing or erroring entry is reported as `failed` or `errored` with its record locators and not
  as an unbound obligation.
- If the bound entry's outcome is `fail`, then the result SHALL be `failed`.
- If the bound entry's outcome is `error`, then the result SHALL be `errored`.
- If the obligation's declared method is not Analysis, then the result SHALL be `qualification-rejected`, naming the
  method and act pairing premise of FR-116 (FR-152).
- If the latest run of a suite holds zero entries, or no entry for the bound symbol, then the result SHALL be
  `absent`, with no invented record locator and no producer outcome.
- The bridge SHALL order the runs of a suite as the store does (FR-030): by the run's `--timestamp`, ties broken by
  the record's file name, whatever order the records are supplied in; the later run is the one that decides.
- If a later run of the same suite fails or errors on the bound symbol of an earlier pass, then the result SHALL be
  `stale`, with the later `failed` or `errored` outcome and its locators visible as FR-078 and FR-116 require, and the
  bridge SHALL NOT count the earlier pass.
- If a later run of the same suite lacks the bound symbol of an earlier pass, then the result SHALL be `absent`.
- If an obligation has more than one bound symbol, then a failing or erroring bound symbol SHALL leave the
  obligation uncounted (FR-032).
- If no FR-093 assessment exists, or the run-package step has no CI checkout SHA, then the result SHALL be
  `unevaluated`, naming the bounded-reliance or the performed-revision premise (FR-184).
- The bridge SHALL NOT count an analysis entry from a binding alone, from `AuditReport.healthy`, or from the
  `unrepresented` result.
- The bridge SHALL NOT judge whether the analysis request is the right analysis for the obligation.
- Trace SHALL expose a counted analysis result as a record claim with its record, run and entry locators, separate
  from source-symbol claims and citations (FR-078); the symbol `<analysisKind>::<requestDigest>` SHALL appear only as
  the retained entry's own identity and SHALL NOT be reported as a source symbol.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-153-CON-1 | The bridge SHALL add no analysis-specific field, status or report member to quire-rs; it uses the FR-078 claim, cause categories and statuses unchanged. | Compatibility | Inspection |
| FR-153-CON-2 | Counting SHALL be a read of the store and supplied observations: no solver is run, no report is re-read from outside the store, and nothing is written (FR-078-AC-20). | Architecture | Test |
| FR-153-CON-3 | Binding, per-suite supersession and the audit's failed-run rule SHALL stay as FR-030 and FR-032 define them; a report recorded into a suite supersedes the earlier report of that suite whatever its `requestDigest`. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-153-AC-1 | Given a bound `pass` entry, an approved producer definition and every other FR-116 premise admitted, the obligation is `qualified-record` in `record_binders` with the entry's record, run and entry locators, and leaves `unbacked_rows`. | Test |
| FR-153-AC-2 | The same bound `pass` entry with no FR-093 assessment, or with no CI checkout SHA, yields `unevaluated` naming the bounded-reliance or the performed-revision premise, the obligation stays in `unbacked_rows`, and the strict decision rejects. | Test |
| FR-153-AC-3 | An `agreement`/`refuted` entry recorded with `--discharges FR-001-AC-1` yields `failed`, an `inconclusive` entry and an `unavailable` entry each yield `errored`; each carries its record locators, adds no `record_binders` claim and rejects strict mode. | Test |
| FR-153-AC-4 | A `disagreement` report recorded with `--discharges FR-001-AC-1` yields `absent` for that obligation, with no record locator invented and no claim, the same result as an obligation never recorded; the printed `unrepresented` result is the only place the disagreement is visible. | Test |
| FR-153-AC-5 | A `pass` run followed by a later `fail` or `error` on the same symbol leaves the obligation uncounted with the result `stale` and the later `failed` or `errored` outcome visible; a later run of the suite lacking the symbol gives `absent`; supplying the records in the reverse order, and giving the two runs equal timestamps, gives the same result. | Test |
| FR-153-AC-6 | An obligation bound through two suites, one with a passing symbol and one with a failing symbol, is uncounted and reports `failed` with the failing suite's locators; no `record_binders` claim and no `qualified-record` status is produced for the passing suite. | Test |
| FR-153-AC-7 | A second report for a different `requestDigest` recorded into the same suite as a first replaces the first as the suite's latest run, and only the second can count; recorded into a different suite, both are bound and both must pass. | Test |
| FR-153-AC-8 | A bound `pass` entry with a healthy `quoin evidence audit` and no FR-116 decision is not counted: no `qualified-record` status and no `record_binders` claim. | Test |
| FR-153-AC-9 | Forward trace on the counted obligation returns the record claim with its locators and provenance; inverse trace by the record or run locator returns the same claim; a source-symbol query for `<analysisKind>::<requestDigest>` returns no claim, and the id appearing in prose stays a citation. | Test |
| FR-153-AC-10 | A `pass` report recorded without `--discharges` produces no claim for any obligation, even where its symbol or digest text equals an obligation id. | Test |
| FR-153-AC-11 | A passing report stated with `--discharges` for an obligation whose declared method is Test yields `qualification-rejected` naming the method and act pairing premise, with an approved producer definition and every other premise admitted, and is not counted. | Test |

## Dependencies

- **Upstream**: [FR-152](./FR-152-analysis-run-entry-link-to-obligation.md) (the stated obligation and binding),
  [FR-151](./FR-151-differential-report-v2-adapter.md), [FR-032](./FR-032-evidence-auditor.md),
  [FR-116](./FR-116-owner-qualified-quire-evidence.md) (qualifier, planned) and
  [FR-093](./FR-093-evidence-producer-trust.md).
- **Cross-repository**: quire-rs [FR-078](ix://agent-ix/quire-rs/FR-078), whose engine side is built
  and unchanged here.
