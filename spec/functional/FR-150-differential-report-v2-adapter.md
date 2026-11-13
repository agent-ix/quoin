---
id: FR-150
title: "Differential report v2 adapter"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-069"
    type: "extends"
  - target: "ix://agent-ix/quire-analyze/FR-005"
    type: "references"
  - target: "ix://agent-ix/quire-analyze/FR-009"
    type: "references"
---

# FR-150: Differential report v2 adapter

## Description

`quire-analyze` runs one analysis request on two SMT solvers (Z3 and cvc5) and publishes one
`quire.differential-report/v2` document (`ix://agent-ix/quire-analyze/FR-005`, `FR-009`). That document reaches
quoin as the input of the `differential-report` run adapter of the FR-033 registry. The adapter reads the
document directly: there is no converter, and there is no reader for any other schema.

The report contract is owned by quire-analyze and is **referenced, not copied**: the schema is
`schemas/differential-report-v2.schema.json` of `agent-ix/quire-analyze` (its `$id` is
`https://agent-ix.github.io/quire-analyze/schemas/differential-report-v2.schema.json`), and the validation entry
point it names is `validate_report_document`. quoin keeps no copy of the schema, its enums or its text. The
adapter adopts the owner's validation as the single definition of a well-formed report, so a change to the
schema reaches quoin by dependency and not by an edit here.

### This adapter replaces the `<domain>.differential-summary/v1` adapter

The `differential-report` adapter that FR-069 specified read `<domain>.differential-summary/v1`, a case-by-case
summary of one domain engine against an external reference. That schema has no producer: the only emitter of the
string is a report committed once into `agent-ix/tl-mltl` (`corpus/r2u2-v4.2/differential-report.json`), no source
in `tl-mltl`, `quire-analyze` or `quire-rs` writes it, and `tl-mltl`'s own differential code writes
`tl-mltl.differential/v1` and `v2`, which that adapter never read. A reader whose input nothing produces is a
compatibility layer for a sample. The adapter name `differential-report` is therefore reassigned to the v2 reader,
and the v1 reader, its `unsupported` handling and its registry claims on the tools `tl-mltl`, `r2u2` and `c2po`
are removed. A `differential-summary/v1` document is refused like any other wrong schema (below).

### One report is one comparison, not a list of cases

A v2 report holds one analysis request judged by two engines. It has no case list. The adapter therefore
transcribes **one run entry per report**. The entry says what the two engines concluded together; it does not say
whether the analysed requirements are right, and it chooses no acceptance policy (FR-033).

### What a disposition becomes

The report's `differential.disposition` is the closed set `agreement`, `disagreement`, `unavailable` and
`inconclusive`. The producer has already folded the analysis kind into the status (`agreedStatus` is `satisfied`
when the property the kind asks about holds, `refuted` when it does not), so the adapter reads the status as
given and applies no per-kind rule.

| Disposition | `agreedStatus` | Result of the adapter |
| --- | --- | --- |
| `agreement` | `satisfied` | one entry, outcome `pass` |
| `agreement` | `refuted` | one entry, outcome `fail` |
| `disagreement` | `null` | one entry, outcome `error` |
| `inconclusive` | `null` | one entry, outcome `error` |
| `unavailable` | `null` | no entry; one `unrepresented` result with state `unavailable` |

`disagreement` and `inconclusive` are `error` and not `fail`: the engines reached no conclusion, so the run
asserts no refutation of the property. `unavailable` carries no run-entry outcome for the reason the retired
`unsupported` state did not: an engine that could not be started is neither a skip nor an error nor a result,
and the adapter names it instead of rounding it (FR-069). The `unrepresented` reason lists, for each of the
two engines, the engine name, its `status` and its `outcome`, each verbatim from the report, so a missing
executable is distinguishable from an unsupported platform. The adapter maps no engine outcome onto a run-entry
outcome of its own; `engines[].outcome` and `engines[].status` are read only to word that reason and to check
agreement (below).

### Entry identity and what the entry binds

The entry symbol is `<analysisKind>::<requestDigest>`, both taken from the report. `requestDigest` is the
analysis request's own identity as the producer assigned it; the adapter computes no digest and verifies none.
The same request analysed again yields the same symbol, so a later run supersedes an earlier one in the store
and a different request cannot overwrite it. The report names no obligation or criterion, so the entry carries
no `traceIds` and binds nothing by itself.

## Behavior

- The adapter SHALL be selected by `--adapter differential-report` and by `--tool differential-report` or
  `--tool quire-analyze`.
- The adapter SHALL accept a document only when `quire-analyze`'s own validation of `quire.differential-report/v2`
  accepts it.
- The adapter SHALL produce exactly one run entry or one `unrepresented` result per accepted report, as the
  table of dispositions states.
- The adapter SHALL take the entry symbol from `analysisKind` and `requestDigest` of the report.
- The adapter SHALL leave `evidenceKind` and `traceIds` unset.
- The adapter SHALL give identical input bytes identical output, with no dependence on time, environment, input
  path, locale or the order of members in the JSON text.
- If the input is larger than 16 MiB (16,777,216 bytes), then the adapter SHALL refuse it without parsing it.
- If the input is not a single JSON object, is not UTF-8, is empty or is truncated, then the adapter SHALL
  refuse it.
- If the `schema` member is absent, is not a string or is anything other than `quire.differential-report/v2`,
  then the adapter SHALL refuse the document naming the value found, so a `quire.differential-report/v1` document, a
  `<domain>.differential-summary/v1` document and any later version are each refused and never read by this reader.
- If a required member is absent, a member the schema does not declare is present, or a member has the wrong type or
  an out-of-set value (including a `disposition`, an engine `status` or an engine `outcome` outside the closed sets
  of the schema), then the adapter SHALL refuse the document naming the member.
- If `disposition` is `agreement` and `agreedStatus` is `null`, or `disposition` is not `agreement` and
  `agreedStatus` is not `null`, then the adapter SHALL refuse the document, because a report that contradicts itself
  states no conclusion.
- If `disposition` is `agreement` and either engine's `status` differs from `agreedStatus`, or the two engines
  are the same engine, then the adapter SHALL refuse the document.
- If the document is refused, then `quoin evidence record` SHALL write no run and exit non-zero with an error
  naming the reason.
- The adapter SHALL NOT recompute `queryDigest`, `requestDigest`, `bindingSetDigest` or any clause digest, because re-derivation against retained material is the owner's validation (`quire-analyze` FR-005-AC-6) and not an intake step.
- The adapter SHALL NOT re-run a solver or replay a model.
- The adapter SHALL NOT decode `stdoutHex`, `stderrHex`, `modelHex` or `queryHex`.
- The adapter SHALL NOT adjudicate a disagreement.
- The adapter SHALL NOT map a disposition or an engine outcome other than as the table states.
- The adapter SHALL NOT run a solver or `quire-analyze`, read a file other than its input, or open the network (FR-069-CON-1).
- The adapter SHALL NOT read the retired `<domain>.differential-summary/v1` schema.
- The adapter SHALL NOT convert a v1 document, a v2 document or any other document into either schema.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-150-CON-1 | quoin SHALL contain no copy of the v2 schema, its enums or its text, and no copy of a report except a byte-exact capture used as a test input. | Integrity | Inspection |
| FR-150-CON-2 | The adapter SHALL introduce no record family and change no stored schema (FR-069-CON-2). | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-150-AC-1 | A report produced by `quire-analyze run-package` with `disposition` `agreement` and `agreedStatus` `satisfied` is recorded as one entry with outcome `pass` and symbol `<analysisKind>::<requestDigest>`. | Test |
| FR-150-AC-2 | A report with `disposition` `agreement` and `agreedStatus` `refuted` is recorded as one entry with outcome `fail`. | Test |
| FR-150-AC-3 | A report with `disposition` `disagreement` (one engine `satisfied`, the other `refuted`) is recorded as one entry with outcome `error`, and is not recorded as `pass`, `fail` or `skip`. | Test |
| FR-150-AC-4 | A report with `disposition` `inconclusive` (for example an engine `timeout` or `unknown`) is recorded as one entry with outcome `error`. | Test |
| FR-150-AC-5 | A report with `disposition` `unavailable` (an engine `missing-executable`, and separately an engine `unsupported-platform`) is recorded as no entry and one `unrepresented` result with state `unavailable`, whose reason names both engines with their `status` and `outcome` verbatim; `quoin evidence record` prints it in human and JSON output. | Test |
| FR-150-AC-6 | A document whose `schema` is `quire.differential-report/v1`, `tl-mltl.differential-summary/v1`, `quire.differential-report/v3`, absent, or not a string is refused naming the value, and no run is written. | Test |
| FR-150-AC-7 | A v2 document with a missing required member, an undeclared extra member (at the top level and inside an engine), a wrongly typed member, and each of an unknown `disposition`, engine `status` and engine `outcome` is refused naming the member, and no run is written. | Test |
| FR-150-AC-8 | Empty input, a truncated report, non-UTF-8 bytes, a JSON array, a JSON scalar and non-JSON text are each refused, and no run is written. | Test |
| FR-150-AC-9 | A report with `agreement` and a `null` `agreedStatus`, a report with `disagreement` and a non-null `agreedStatus`, a report with `agreement` whose engine `status` differs from `agreedStatus`, and a report naming the same engine twice are each refused. | Test |
| FR-150-AC-10 | An input of 16,777,217 bytes is refused with an error naming the size limit before any JSON parsing; an otherwise valid input of exactly 16,777,216 bytes is not refused for size. | Test |
| FR-150-AC-11 | Two runs over the same bytes, from different paths, working directories and environments, return identical results; reordering the members of the JSON object changes nothing. | Test |
| FR-150-AC-12 | `--adapter differential-report`, `--tool differential-report` and `--tool quire-analyze` each select the adapter, `--tool tl-mltl`, `--tool r2u2` and `--tool c2po` do not, and the adapter appears in the adapter listing naming `quire.differential-report/v2` and not `differential-summary`. | Test |
| FR-150-AC-13 | The adapter source opens no file other than its input, spawns no process and opens no network connection, and contains no `differential-summary` reader and no schema copy. | Inspection |
| FR-150-AC-14 | The adapter's fixtures are reports captured unedited from `quire-analyze run-package` (one per disposition that the runner can produce), and a refusal fixture derived from one of them by a single stated change. | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md), [FR-069](./FR-069-campaign-native-adapters.md);
  `ix://agent-ix/quire-analyze/FR-005` and `FR-009` own the report.
- **Downstream**: retained-evidence counting over these entries is specified separately and is not part of this
  requirement.
