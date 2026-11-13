---
id: FR-151
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

# FR-151: Differential report v2 adapter

## Description

`quire-analyze` runs one analysis request on two SMT solvers (Z3 and cvc5) and publishes one
`quire.differential-report/v2` document (`ix://agent-ix/quire-analyze/FR-005`, `FR-009`). That document reaches
quoin as the input of the `differential-report` run adapter of the FR-033 registry. The adapter reads the
document directly: there is no converter, and there is no reader for any other schema.

The report contract is owned by quire-analyze and is **referenced, not copied**: the schema is
`schemas/differential-report-v2.schema.json` of `agent-ix/quire-analyze` (its `$id` is
`https://agent-ix.github.io/quire-analyze/schemas/differential-report-v2.schema.json`), and the validation entry
point it names is `validate_report_document`. The adapter adopts the owner's validation as the single definition of
a well-formed report: it calls `quire_analyze::validate_report_document` and bounds the input with
`quire_analyze::MAX_REPORT_BYTES` (64 MiB), and adds only the mapping of an accepted report to a run entry or an
`unrepresented` result. The validator decides acceptance. It accepts canonical compact, key-sorted JSON only, and
it re-derives what the owner re-derives (`queryDigest` from `requestDigest` and the query bytes, per
`quire-analyze` FR-005-AC-6, and the engines' retained stdout, stderr and model bytes), so a non-canonical or
internally inconsistent document is refused. The adapter performs no digest derivation, hex decoding or output
parsing of its own.

### How quoin reaches the validator

quoin SHALL depend on `agent-ix/quire-analyze` as a Cargo git dependency pinned by `rev` in `rust/Cargo.toml`,
as it depends on `quire-rs` and `engineering-assurance`. quoin SHALL NOT copy, re-embed or re-implement the schema,
its enums or the validator. A change to the schema or the validator reaches quoin by moving that `rev`, and not by
an edit here.

Upstream enablement dependency: the `quire-analyze` library SHALL build on every quoin release target, and quoin
releases `x86_64-pc-windows-msvc`. The library's runner imports `std::os::unix` unguarded today; that is being
fixed in `quire-analyze` under VER-73. The change that implements this requirement (VER-80) checks that the
dependency builds for the Windows target before it lands. The dependency also brings in `jsonschema =0.17.1`
alongside quoin's own `=0.56.0`; both versions coexist in the lockfile.

### This adapter replaces the `<domain>.differential-summary/v1` adapter

The `differential-report` adapter that FR-069 specified read `<domain>.differential-summary/v1`, a case-by-case
summary of one domain engine against an external reference. That schema has no producer: the only emitter of the
string is a report committed once into `agent-ix/tl-mltl` (`corpus/r2u2-v4.2/differential-report.json`), and no source
in `tl-mltl`, `quire-analyze` or `quire-rs` writes it. `tl-mltl`'s live differential code writes other schemas,
`tl-mltl.differential/v1` and `v2`, `tl-mltl.r2u2-past-differential/v1`, `tl-mltl.r2u2-future-differential/v1` and
`tl-mltl.campaign-run/v1`, which that adapter never read and this adapter does not read either. A reader whose input nothing produces is a
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
| `inconclusive` | `null` | one entry, outcome `error` |
| `unavailable` | `null` | one entry, outcome `error` |
| `disagreement` | `null` | no entry; one `unrepresented` result with state `disagreement` |

`inconclusive` and `unavailable` are `error` and not `fail` or `skip`: in quoin `Outcome::Error` means the producer
could not run or could not conclude, and an engine that timed out, answered `unknown` or could not be started is
that. The run entry carries no field that tells the two apart (`RunEntry` has outcome, score, metric, trace ids,
config and survivors only), so the distinction stays in the report, which the producer retains.

`disagreement` is not an `error`: both engines ran and concluded opposite results, which signals a soundness defect
in a solver or in the encoding, and `Outcome::Error` would record it as a producer that did not run. It is not a
`fail` either, because it asserts no refutation of the property. No run-entry outcome carries it, so the adapter
names it instead of rounding it (FR-069). The `unrepresented` result has the symbol `<analysisKind>::<requestDigest>`,
the state `disagreement`, and a reason that lists, for each of the two engines, the engine name, its `status` and its
`outcome`, each verbatim from the report. The adapter maps no engine outcome onto a run-entry outcome of its own;
`engines[].outcome` and `engines[].status` are read only to word that reason.

What `quoin evidence record` writes for a `disagreement` report: `record_run` writes the run unconditionally, so one
`RunRecord` with the suite, tool and timestamp of the command and **zero entries** is written. It discharges and
binds nothing. The `unrepresented` result is an output of the command (printed in human and JSON output,
FR-069-AC-6) and is not stored. FR-069's refusal of an empty result set does not fire: it guards an input that
reported no result at all, and a v2 report always reports exactly one comparison, which here is the `unrepresented`
result.

### Entry identity and what the entry binds

The entry symbol is `<analysisKind>::<requestDigest>`, both taken from the report. `requestDigest` is the
analysis request's own identity as the producer assigned it; the adapter computes no digest and verifies none.
The same request analysed again yields the same symbol, so a later run supersedes an earlier one in the store
and a different request cannot overwrite it. The report names no obligation or criterion, so the entry carries
no `traceIds` and binds nothing by itself.

## Behavior

- The adapter SHALL be selected by `--adapter differential-report` and by `--tool differential-report` or
  `--tool quire-analyze`.
- The adapter SHALL accept a document only when `quire_analyze::validate_report_document` accepts it.
- The adapter SHALL produce exactly one run entry or one `unrepresented` result per accepted report, as the
  table of dispositions states.
- The adapter SHALL take the entry symbol from `analysisKind` and `requestDigest` of the report.
- The adapter SHALL leave `evidenceKind` and `traceIds` unset.
- The adapter SHALL give identical input bytes identical output, with no dependence on time, environment, input
  path or locale.
- If the input is larger than `quire_analyze::MAX_REPORT_BYTES`, then the adapter SHALL refuse it without parsing it.
- If the input is not a single JSON object, is not UTF-8, is empty, is truncated, or is not in the canonical compact,
  key-sorted form the owner's validator requires, then the adapter SHALL refuse it.
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
- If the document is refused, then `quoin evidence record` SHALL write no run.
- If the document is refused, then `quoin evidence record` SHALL exit non-zero with an error naming the reason.
- The adapter SHALL NOT perform any digest derivation, hex decoding or solver-output parsing beyond what
  `validate_report_document` performs.
- The adapter SHALL NOT re-run a solver or replay a model.
- The adapter SHALL NOT adjudicate a disagreement.
- The adapter SHALL NOT map a disposition or an engine outcome other than as the table states.
- The adapter SHALL NOT run a solver or `quire-analyze`, read a file other than its input, or open the network (FR-069-CON-1).
- The adapter SHALL NOT read the retired `<domain>.differential-summary/v1` schema.
- The adapter SHALL NOT convert a v1 document, a v2 document or any other document into either schema.
- quoin SHALL NOT copy, re-embed or re-implement the v2 schema, its enums or the validator.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-151-CON-1 | quoin SHALL contain no copy of the v2 schema, its enums or its text, and no copy of a report except a byte-exact capture used as a test input (or a document derived from one by the changes AC-14 states). | Integrity | Inspection |
| FR-151-CON-2 | The adapter SHALL introduce no record family and change no stored schema (FR-069-CON-2). | Compatibility | Test |
| FR-151-CON-3 | quoin SHALL reach the validator and `MAX_REPORT_BYTES` only through a Cargo git dependency on `agent-ix/quire-analyze` pinned by `rev`. | Architecture | Inspection |
| FR-151-CON-4 | The `quire-analyze` dependency SHALL build on every quoin release target, including `x86_64-pc-windows-msvc`. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-151-AC-1 | A report produced by `quire-analyze run-package` with `disposition` `agreement` and `agreedStatus` `satisfied` is recorded as one entry with outcome `pass` and symbol `<analysisKind>::<requestDigest>`. | Test |
| FR-151-AC-2 | A report with `disposition` `agreement` and `agreedStatus` `refuted` is recorded as one entry with outcome `fail`. | Test |
| FR-151-AC-3 | A report with `disposition` `disagreement` (one engine `satisfied`, the other `refuted`) is recorded as no entry and one `unrepresented` result with symbol `<analysisKind>::<requestDigest>` and state `disagreement`, whose reason names both engines with their `status` and `outcome` verbatim; the run written has zero entries; `quoin evidence record` prints the result in human and JSON output. It is not recorded as `pass`, `fail`, `skip` or `error`. | Test |
| FR-151-AC-4 | A report with `disposition` `inconclusive` (for example an engine `timeout` or `unknown`) is recorded as one entry with outcome `error`. | Test |
| FR-151-AC-5 | A report with `disposition` `unavailable` (an engine `missing-executable`, and separately an engine `unsupported-platform`) is recorded as one entry with outcome `error` and symbol `<analysisKind>::<requestDigest>`. The `unsupported-platform` input is a captured or hand-constructed v2 document that the owner's validator accepts, not a runner capture: a Linux runner reports a containment failure as `cleanup-failed`, which is `inconclusive`. | Test |
| FR-151-AC-6 | A document whose `schema` is `quire.differential-report/v1`, `tl-mltl.differential-summary/v1`, `quire.differential-report/v3`, absent, or not a string is refused naming the value, and no run is written. | Test |
| FR-151-AC-7 | A v2 document with a missing required member, an undeclared extra member (at the top level and inside an engine), a wrongly typed member, and each of an unknown `disposition`, engine `status` and engine `outcome` is refused naming the member, and no run is written. | Test |
| FR-151-AC-8 | Empty input, a truncated report, non-UTF-8 bytes, a JSON array, a JSON scalar and non-JSON text are each refused, and no run is written. | Test |
| FR-151-AC-9 | A report with `agreement` and a `null` `agreedStatus`, a report with `disagreement` and a non-null `agreedStatus`, a report with `agreement` whose engine `status` differs from `agreedStatus`, and a report naming the same engine twice are each refused. | Test |
| FR-151-AC-10 | An input of `quire_analyze::MAX_REPORT_BYTES` + 1 bytes is refused with an error naming the size limit before any JSON parsing; an input of exactly `MAX_REPORT_BYTES` bytes is not refused for size (it may be refused for another reason). | Test |
| FR-151-AC-11 | Two runs over the same bytes, from different paths, working directories and environments, return identical results. A copy of a valid report whose members are reordered, or that is pretty-printed, is refused, with the same error on every run. | Test |
| FR-151-AC-12 | `--adapter differential-report`, `--tool differential-report` and `--tool quire-analyze` each select the adapter, `--tool tl-mltl`, `--tool r2u2` and `--tool c2po` do not, and the adapter appears in the adapter listing naming `quire.differential-report/v2` and not `differential-summary`. | Test |
| FR-151-AC-13 | The adapter source opens no file other than its input, spawns no process and opens no network connection, and contains no `differential-summary` reader and no schema copy; `rust/Cargo.toml` names `agent-ix/quire-analyze` as a git dependency with a `rev`. | Inspection |
| FR-151-AC-14 | The adapter's accepted fixtures are reports captured unedited from `quire-analyze run-package` (one per disposition that the runner can produce; the `unsupported-platform` fixture is AC-5's). Each refusal fixture is derived from one capture by one stated change, stays canonical unless the change is the non-canonical form itself, and the refusal tests are table-driven with one row per refusal class: schema `quire.differential-report/v1`, `tl-mltl.differential-summary/v1`, `quire.differential-report/v3`, schema absent, schema not a string, a required member removed, an undeclared top-level member, an undeclared engine member, a wrongly typed member, an unknown `disposition`, an unknown engine `status`, an unknown engine `outcome`, empty input, a truncated report, non-UTF-8 bytes, a JSON array, a JSON scalar, non-JSON text, `agreement` with a `null` `agreedStatus`, `disagreement` with a non-null `agreedStatus`, `agreement` with an engine `status` differing from `agreedStatus`, the same engine twice, members reordered, a pretty-printed copy, and an input of `MAX_REPORT_BYTES` + 1 bytes (synthetic padding, the only fixture that is neither a capture nor a one-change derivation). | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md), [FR-069](./FR-069-campaign-native-adapters.md);
  `ix://agent-ix/quire-analyze/FR-005` and `FR-009` own the report.
- **Upstream enablement**: the `quire-analyze` library builds on every quoin release target, including
  `x86_64-pc-windows-msvc` (VER-73 removes the unguarded `std::os::unix` import from its runner); VER-80 checks the
  Windows target builds before the dependency lands.
- **Downstream**: retained-evidence counting over these entries is specified separately and is not part of this
  requirement.
