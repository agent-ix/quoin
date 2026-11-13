---
id: FR-069
title: "Campaign-native result adapters"
type: FR
verification_method: test
evidence: []
relationships:
  - target: "ix://agent-ix/quoin/StR-005"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/FR-034"
    type: "extends"
---
# FR-069: Campaign-native result adapters

## Description

Quoin SHALL transcribe the structured result formats the contract campaign
repositories actually emit, adding an adapter only where a real producer emits
a real format that no existing adapter represents.

An adapter SHALL validate and transcribe. It SHALL execute no producer and
SHALL decide no question of assurance sufficiency.

## Inputs

- Contract conformance JSONL under the declared protocol
  `quire.contract.conformance-jsonl/v1`, one replayed corpus fixture per line.
- A `quire.differential-report/v2` document, read by the `differential-report` adapter of
  [FR-151](./FR-151-differential-report-v2-adapter.md), which replaces the earlier
  `<domain>.differential-summary/v1` reader (no producer wrote that schema).

## Outputs

- `RunRecord` entries, one per replayed fixture (contract-conformance) or one per report (differential-report, FR-151).
- An `unrepresented` list naming each producer result that no run-entry outcome
  carries, with the producer's own state verbatim and the reason.
- A published inventory recording, for every campaign-native format, the real
  producer, the real sample, the governed target record, and why an existing
  adapter does or does not represent it.

## Behavior

- An unknown protocol or schema version SHALL be refused rather than read by
  the nearest reader that recognises the fields it knows.
- An unknown case or fixture status SHALL be refused rather than mapped to the
  nearest outcome.
- An empty result set SHALL be refused: a conformance run that emitted no line
  did not report that every fixture matched. A differential report always holds
  exactly one comparison, so this refusal applies to the contract-conformance
  adapter; the differential adapter's refusals are FR-151's.
- A conformance entry's identity SHALL include its corpus and operation, so one
  replayed fixture cannot overwrite another.
- A supplied conformance `trace_ids` SHALL be a non-empty array of distinct
  strings containing non-whitespace text. The adapter SHALL preserve identifiers
  and their order exactly as `RunEntry.traceIds`, without trimming, sorting,
  deduplicating, or inferring bindings. A legacy row omitting `trace_ids` SHALL
  remain readable and SHALL carry no inferred trace ids.
- Malformed supplied trace metadata SHALL fail with an `AdapterError` naming
  the line and `trace_ids`, before any run or binding is recorded. Valid trace
  ids SHALL pass through the existing Test Case-to-criterion binding path;
  unmatched ids remain reported, and failing entries discharge no obligation.
- A producer state that no run-entry outcome carries SHALL NOT be transcribed
  as a different state. The adapter SHALL name it, and the command SHALL print
  it.
- Arbitrary stdout or stderr SHALL NOT be scraped for a verdict.

## Error Conditions

Malformed JSON, a wrong protocol or schema version, a missing corpus, fixture,
operation, fixture id or status, an unknown status, and an empty result set each
fail with an `AdapterError` naming the offending line (a differential report is
refused as FR-151 states). No partial run is
recorded on any of these paths.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-069-CON-1 | An adapter SHALL execute no producer, spawn no process, and perform no network request. | Architecture | Inspection |
| FR-069-CON-2 | An adapter SHALL introduce no new record family and SHALL change no stored schema. | Compatibility | Test |
| FR-069-CON-3 | An adapter SHALL NOT recover a verdict from unstructured console output. | Responsibility | Inspection |
| FR-069-CON-4 | Every added adapter SHALL name a real producer and a real sample. | Integrity | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-069-AC-1 | The `contract-conformance` adapter transcribes a real conformance run into one entry per replayed fixture, keyed by corpus, operation, and fixture. | Test |
| FR-069-AC-2 | The `differential-report` adapter transcribes a real `quire.differential-report/v2` report as FR-151 states. | Test |
| FR-069-AC-3 | A differential disagreement (FR-151) is not transcribed as a skip, an error, a fail or a pass; it is named with its own state `disagreement` and a reason. | Test |
| FR-069-AC-4 | An unknown protocol, an unknown schema version, an unknown status, a missing required field, malformed JSON, and an empty result set are each refused by the contract-conformance adapter with a message naming the line (the differential adapter: FR-151-AC-6 to AC-11). | Property |
| FR-069-AC-5 | Both adapters are selectable by name and by declared tool, and appear in the adapter listing. | Test |
| FR-069-AC-6 | `quoin evidence record` prints every unrepresented result in both human and JSON output (CON-2). | Test |
| FR-069-AC-7 | The inventory names, for every scope item, a real producer and a verdict, and every added adapter names its sample (CON-4). | Test |
| FR-069-AC-8 | Static boundaries prove neither adapter spawns a process, performs network work, or scrapes console text for a verdict (CON-1, CON-3). | Inspection |
| FR-069-AC-9 | A real conformance row retains every supplied trace id and its order in `RunEntry.traceIds`; legacy rows omitting the field remain readable without trace ids. | Test |
| FR-069-AC-10 | A supplied non-array, empty array, non-string id, empty or whitespace-only id, or duplicate id fails naming the line and `trace_ids`; a valid preceding row causes no partial evidence write. | Test |
| FR-069-AC-11 | `quoin evidence record --adapter contract-conformance` retains trace ids and binds the real producer's Test Case ids through Quire-derived criterion targets; failing entries bind nothing and unmatched ids remain visible. | Integration |

## Dependencies

- **Upstream**: [FR-034](./FR-034-finding-shaped-evidence.md) and the existing adapter
  registry.
- **Downstream**: `agent-ix/engineering-assurance#10` cites this inventory when
  deciding what each migrating repository keeps, replaces, or deletes.
