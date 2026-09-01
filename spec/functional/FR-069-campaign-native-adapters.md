---
id: FR-069
title: "Campaign-native result adapters"
type: FR
verification_method: test
evidence:
  - kind: test_case
    ref: tests/campaign-adapters.test.ts
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
- A domain differential summary under a declared
  `<domain>.differential-summary/v1` schema version.

## Outputs

- `RunRecord` entries, one per replayed fixture or compared case.
- An `unrepresented` list naming each producer result that no run-entry outcome
  carries, with the producer's own state verbatim and the reason.
- A published inventory recording, for every campaign-native format, the real
  producer, the pinned sample, the governed target record, and why an existing
  adapter does or does not represent it.

## Behavior

- An unknown protocol or schema version SHALL be refused rather than read by
  the nearest reader that recognises the fields it knows.
- An unknown case or fixture status SHALL be refused rather than mapped to the
  nearest outcome.
- An empty result set SHALL be refused: a conformance run that emitted no line
  did not report that every fixture matched, and a differential that compared
  no case did not report agreement.
- A conformance entry's identity SHALL include its corpus and operation, so one
  replayed fixture cannot overwrite another.
- A producer state that no run-entry outcome carries SHALL NOT be transcribed
  as a different state. The adapter SHALL name it, and the command SHALL print
  it.
- Arbitrary stdout or stderr SHALL NOT be scraped for a verdict.

## Error Conditions

Malformed JSON, a wrong protocol or schema version, a missing corpus, fixture,
operation, case id or status, an unknown status, and an empty result set each
fail with an `AdapterError` naming the offending line or case. No partial run is
recorded on any of these paths.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-069-CON-1 | An adapter SHALL execute no producer, spawn no process, and perform no network request. | Architecture | Inspection |
| FR-069-CON-2 | An adapter SHALL introduce no new record family and SHALL change no stored schema. | Compatibility | Test |
| FR-069-CON-3 | An adapter SHALL NOT recover a verdict from unstructured console output. | Responsibility | Inspection |
| FR-069-CON-4 | Every added adapter SHALL name a real producer and a pinned real sample. | Integrity | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-069-AC-1 | The `contract-conformance` adapter transcribes a real conformance run into one entry per replayed fixture, keyed by corpus, operation, and fixture. | Test (TC-1328) |
| FR-069-AC-2 | The `differential-report` adapter transcribes a real differential report into one entry per compared case. | Test (TC-1329) |
| FR-069-AC-3 | An unsupported case is not transcribed as a skip, an error, or a pass; it is named with its own state and reason. | Test (TC-1330) |
| FR-069-AC-4 | An unknown protocol, an unknown schema version, an unknown status, a missing required field, malformed JSON, and an empty result set are each refused with a message naming the line or case. | Property (TC-1331) |
| FR-069-AC-5 | Both adapters are selectable by name and by declared tool, and appear in the adapter listing. | Test (TC-1332) |
| FR-069-AC-6 | `quoin evidence record` prints every unrepresented result in both human and JSON output (CON-2). | Test (TC-1333) |
| FR-069-AC-7 | The inventory names, for every scope item, a real producer and a verdict, and every added adapter names its pinned sample (CON-4). | Test (TC-1334) |
| FR-069-AC-8 | Static boundaries prove neither adapter spawns a process, performs network work, or scrapes console text for a verdict (CON-1, CON-3). | Inspection (TC-1335) |

## Dependencies

- **Upstream**: [FR-034](./FR-034-finding-shaped-evidence.md) and the existing adapter
  registry.
- **Downstream**: `agent-ix/engineering-assurance#10` cites this inventory when
  deciding what each migrating repository keeps, replaces, or deletes.
