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
  - target: "ix://agent-ix/quire-rs/spec/functional/FR-078"
    type: "requires"
---

# FR-153: An analysis result counts toward an obligation only as a qualified passing record

## Description

An analysis result linked to an obligation by FR-152 is a retained record. This requirement states when that record
counts as coverage and trace for the obligation, per disposition of the report (FR-151). The counting rules
themselves are quire-rs [FR-078](ix://agent-ix/quire-rs/spec/functional/FR-078) and the owner qualification
contract [Quoin FR-116](./FR-116-owner-qualified-quire-evidence.md); they are cited here and not restated. This
requirement adds only what is specific to an analysis entry.

### What counts

Coverage credit for an obligation backed by an analysis result is the FR-078 `qualified-record` status and nothing
else. A binding in the store, a passing entry and a healthy `quoin evidence audit` result each fall short of it, as
FR-116 states for a healthy audit. Credit therefore needs, in this order:

1. a linked entry (FR-152, quoin);
2. a passing entry whose binding survives the FR-032 audit of the latest run (quoin);
3. a positive FR-116 decision for the exact selected target and its declared method, from the planned qualifier and
   `quoin-quire` bridge (quoin; planned, not built);
4. FR-078 reconciliation of that decision as a `record_binders` claim (quire-rs; built).

### What each disposition counts as

| Report | Entry (FR-151) | Binding (FR-152) | FR-078 category | Counted |
| --- | --- | --- | --- | --- |
| `agreement`, `satisfied` | `pass` | created when linked | qualified claim, or the `unevaluated` / `qualification-rejected` cause of the premise that fails | Only as a qualified claim |
| `agreement`, `refuted` | `fail` | none | `failed` | No |
| `inconclusive` | `error` | none | `errored` | No |
| `unavailable` | `error` | none | `errored` | No |
| `disagreement` | none; `unrepresented` result, run with zero entries | none | `absent` | No |

`inconclusive` and `unavailable` are the same category because the entry does not tell them apart (FR-151); the
retained report does. `disagreement` is `absent` because the zero-entry run holds no result for the obligation; the
`unrepresented` result is printed and not stored (FR-151), so the stored evidence cannot say that two engines
disagreed.

### Qualification of an analysis act

The bridge SHALL ask the FR-116 qualifier about an analysis entry with the obligation's declared method and a
producer definition that Quoin FR-116 requires the owner to have approved for this act. The entry's `evidenceKind`
is unset (FR-151) and no run field establishes the act, so neither supplies a qualification premise. No producer
definition for this act is approved today; until one is, every analysis-backed obligation is `unevaluated`, naming
the producer-definition premise, and none is counted.

### Not decided here

These need an owner and are not specified. This requirement states what holds whichever way each is decided.

- Whether the `--link` shape of FR-152 is the binding source, or the obligation's own declared target should name the
  analysis request, or a plan should (FR-152 specifies the first because it changes no schema and no obligation
  record).
- The content of the producer definition: the required acts, the tool and configuration premises the report's
  `producer` and `engines` members can supply, and the CI or manual association (Quoin FR-116 owner approval).
- Whether a `disagreement` must stay distinguishable from an absent result after recording. That needs the
  `unrepresented` result stored (a store change under FR-030) or a FR-116 result category.
- Whether a report that is `satisfied` for a request counts for an obligation whose statement the request does not
  actually analyse. Quoin does not judge that; only the actor-authority premise bears on it.
- How an owner waiver of an analysis result is represented (FR-078 holds this open for all methods).

## Behavior

- The bridge SHALL count an analysis-backed obligation as covered only by a positive FR-116 decision, reconciled by
  FR-078 as a `record_binders` claim and status `qualified-record`.
- The bridge SHALL correlate a requested target to analysis entries by the `traceIds` stored on the run entries of
  any outcome, so a failing or erroring entry is reported as `failed` or `errored` with its record locators and not
  as an unbound obligation.
- If the linked entry's outcome is `fail`, then the result SHALL be `failed`; if it is `error`, then `errored`.
- If the latest run of a suite holds zero entries, or no entry for the bound symbol, then the result SHALL be
  `absent`, with no invented record locator and no producer outcome.
- If a later run of the same suite fails, errors, or lacks the bound symbol of an earlier pass, then
  the bridge SHALL take the later run as deciding, whatever order the records are supplied in.
- If a later run of the same suite fails, errors, or lacks the bound symbol of an earlier pass, then
  the bridge SHALL NOT count the earlier pass.
- If an obligation has more than one bound symbol, then a failing or erroring bound symbol SHALL leave the
  obligation uncounted (FR-032).
- If no approved producer definition exists for the act, then the result SHALL be `unevaluated`, naming the
  producer-definition premise.
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
| FR-153-AC-1 | Given a linked, bound `pass` entry, an approved producer definition and every other FR-116 premise admitted, the obligation is `qualified-record` in `record_binders` with the entry's record, run and entry locators, and leaves `unbacked_rows`. | Test |
| FR-153-AC-2 | The same bound `pass` entry with no approved producer definition yields `unevaluated` naming the producer-definition premise, the obligation stays in `unbacked_rows`, and the strict decision rejects. | Test |
| FR-153-AC-3 | A linked `agreement`/`refuted` entry yields `failed`, a linked `inconclusive` entry and a linked `unavailable` entry each yield `errored`; each carries its record locators, adds no `record_binders` claim and rejects strict mode. | Test |
| FR-153-AC-4 | A `disagreement` report recorded with a link yields `absent` for the linked obligation, with no record locator invented, and no claim. | Test |
| FR-153-AC-5 | A `pass` run followed by a `fail` (same symbol), an `error` (same symbol), or a run of the same suite lacking the symbol leaves the obligation uncounted, and supplying the records in the reverse order gives the same result. | Test |
| FR-153-AC-6 | An obligation linked to two suites, one with a passing symbol and one with a failing symbol, is uncounted and reports the failing one; the passing one stays visible. | Test |
| FR-153-AC-7 | A second report for a different `requestDigest` recorded into the same suite as a first replaces the first as the suite's latest run, and only the second can count; recorded into a different suite, both are bound and both must pass. | Test |
| FR-153-AC-8 | A bound `pass` entry with a healthy `quoin evidence audit` and no FR-116 decision is not counted: no `qualified-record` status and no `record_binders` claim. | Test |
| FR-153-AC-9 | Forward trace on the counted obligation returns the record claim with its locators and provenance; inverse trace by the record or run locator returns the same claim; a source-symbol query for `<analysisKind>::<requestDigest>` returns no claim, and the id appearing in prose stays a citation. | Test |
| FR-153-AC-10 | An obligation counted from an analysis result is not counted when the same store is read with the entry's link removed from its stored `traceIds`; no claim is produced from the symbol alone. | Test |
| FR-153-AC-11 | The change that implements this requirement edits no quire-rs type, status or report member, and the only analysis-specific code in the bridge is the correlation by stored `traceIds` and the outcome mapping of the table. | Inspection |

## Dependencies

- **Upstream**: [FR-152](./FR-152-analysis-run-entry-link-to-obligation.md) (the link and binding),
  [FR-151](./FR-151-differential-report-v2-adapter.md), [FR-032](./FR-032-evidence-auditor.md),
  [FR-116](./FR-116-owner-qualified-quire-evidence.md) (qualifier, planned) and
  [FR-093](./FR-093-evidence-producer-trust.md).
- **Cross-repository**: quire-rs [FR-078](ix://agent-ix/quire-rs/spec/functional/FR-078), whose engine side is built
  and unchanged here.
