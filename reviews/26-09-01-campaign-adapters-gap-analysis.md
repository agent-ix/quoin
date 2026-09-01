---
id: SR-117
title: "Gap analysis — campaign-native result adapters"
type: SpecReview
analysis: gap-analysis
scope: "FR-069; spec/matrix.md TC-1328..TC-1335; src/evidence/adapters/; docs/campaign-native-result-inventory.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/issues/323"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-069"
    type: "references"
---

# SR-117: Gap analysis — campaign-native result adapters

## Summary

Verification gate over FR-069 after #323. Every acceptance criterion has a
matrix row and a tagged test, every #323 acceptance criterion resolves to
something checked, and every scope item in the ticket has a verdict in the
published inventory.

## Verdict

**CONDITIONAL** — no FR-069 coverage gap. Two findings record the boundary of
what this inventory can claim.

## Findings

| ID      | Severity | Summary                                                                                | Refs                                        |
| ------- | -------- | -------------------------------------------------------------------------------------- | ------------------------------------------- |
| FND-001 | medium   | Three scope items are dispositioned on absence of a producer, not on inspection of one | docs/campaign-native-result-inventory.md:31 |
| FND-002 | low      | The JUnit verdict rests on a format the campaign repositories do not yet emit          | docs/campaign-native-result-inventory.md:22 |

## Finding detail

### FND-001 — Kani, solver analysis, and proof reports are dispositioned by absence

The inventory records "no adapter — nothing to pin" for Kani outputs, solver
analysis reports, and domain proof reports. That is a fact about today: no
campaign repository runs Kani, `quire-analyze`'s entire library surface is
`hello()`, and no proof report is emitted anywhere.

Failure scenario: a reader takes "no adapter needed" as a durable architectural
conclusion, when it is a dated observation. The day `quire-analyze` emits its
first solver report, this inventory is stale and nothing here will say so.

Accepted, and stated in the inventory itself: writing a reader for a format
nobody produces means inventing the format, and the first real sample would then
be shaped to fit the reader rather than the other way round. The inventory names
the reopening condition — a real sample — rather than leaving the reader to
infer it.

### FND-002 — the JUnit verdict is about a converter, not an observed artifact

"Covered by `junit`" for the campaign repositories assumes `cargo test` output
reaches JUnit XML through an emitter. No campaign repository currently commits a
JUnit artifact, so no pinned sample from those eight repositories backs that row.

Failure scenario: a migrating repository wires up a JUnit emitter whose dialect
the adapter reads differently than expected, and the inventory said it was
covered.

Accepted: `junit` is already exercised against its own pinned samples in
`tests/evidence-adapters.test.ts`, and the risk is a dialect question that
belongs to the migration of a specific repository — `agent-ix/engineering-assurance#10`
— not to this inventory.

## Coverage

FR-069, all backed by `tests/campaign-adapters.test.ts`:

| Criterion   | Test case | Backing test                                                                |
| ----------- | --------- | --------------------------------------------------------------------------- |
| FR-069-AC-1 | TC-1328   | transcribes a real conformance run, keyed by corpus, operation, and fixture |
| FR-069-AC-2 | TC-1329   | transcribes a real differential report, one entry per compared case         |
| FR-069-AC-3 | TC-1330   | names an unsupported case rather than transcribing it as another state      |
| FR-069-AC-4 | TC-1331   | refuses every malformed, unknown, and empty input by line or case           |
| FR-069-AC-5 | TC-1332   | registers both adapters by name and by declared tool                        |
| FR-069-AC-6 | TC-1333   | prints every unrepresented result in human and JSON output                  |
| FR-069-AC-7 | TC-1334   | records a producer and a verdict for every scope item                       |
| FR-069-AC-8 | TC-1335   | executes nothing and scrapes no console text for a verdict                  |

Constraint coverage: CON-1 and CON-3 by TC-1335 (static, over both adapter
sources); CON-2 by TC-1333, which asserts the persisted run is the existing
`RunRecord` shape and that the unrepresented case appears in no entry; CON-4 by
TC-1334, which requires each added adapter's pinned sample to exist on disk.

Engine reconciliation — `quire coverage --scope . --json`:

| Measure                 | FR-069 |
| ----------------------- | ------ |
| Unbacked rows           | 0      |
| Unmatched tracking tags | 0      |

#323's acceptance criteria:

| Criterion                                                                                                                            | State                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Every proposed adapter names a real producer, pinned sample, governed target record, and why an existing adapter cannot represent it | Inventory sections per adapter; TC-1334                                                                                                  |
| Adapters validate and transcribe; they do not execute or decide sufficiency                                                          | TC-1335                                                                                                                                  |
| Arbitrary stdout/stderr scraping is explicitly out of scope                                                                          | Stated in the inventory; TC-1334 asserts the sentence                                                                                    |
| Empty, malformed, partial, failed, unavailable, unsupported, and not-computed samples covered                                        | TC-1330 and TC-1331; `unsupported` is the one state with no stored outcome, handled by naming it                                         |
| Native structured output retained when available; console streams only when material                                                 | Both adapters read structured formats; the one console-stream case in the inventory is the static-scan diagnostic, which IS the artifact |
| No adapter duplicates an EA or existing Quoin record family                                                                          | TC-1333 asserts the persisted shape is unchanged; `unrepresented` is adapter-layer                                                       |

Underspecified code — none. Both adapters are reached from the registry and
from FR-069's tests; `UnrepresentedResult` is consumed by
`src/commands/evidence/record.ts` and asserted by TC-1333.

Semantic review — performed inline over FR-069's eight criteria. TC-1328 and
TC-1329 assert against the real fixtures' own contents rather than hand-written
expectations: they re-read each line and derive the expected symbol and outcome
from it, so a fixture edited to make the test pass changes both sides and the
operation-coverage assertion still fails. TC-1331 adds a property over arbitrary
non-vocabulary statuses, so the refusal is not just tested on the statuses I
happened to think of.
