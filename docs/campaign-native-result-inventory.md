# Campaign-native result inventory

What the eight contract campaign repositories actually emit, and which Quoin
adapter reads it. Answers `agent-ix/quoin#323`.

The rule this inventory applies: **an adapter is added only when a real
producer emits a real format that no existing adapter can represent, and the
reason it cannot is stated.** Arbitrary stdout or stderr scraping is out of
scope, here and permanently — a verdict recovered from console text is a
verdict the producer never made.

## Repositories surveyed

`quire-contract-ir`, `quire-contract-runtime`, `quire-contract-codegen`,
`quire-analyze`, `tl-syntax`, `tl-parse`, `tl-mltl`, `tl-rewrite`, each read at
its `origin/main`.

## Verdicts

| Scope item                        | Real producer                                                                                                                   | Verdict                                                                                           |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| JUnit-compatible test reports     | every repository's `cargo test`, via a JUnit emitter                                                                            | **Covered** by `junit`                                                                            |
| SARIF                             | not currently emitted by any of the eight                                                                                       | **Covered** by `sarif` when one does                                                              |
| cargo-mutants                     | mutation runs in the Rust repositories                                                                                          | **Covered** by `cargo-mutants`                                                                    |
| Audit-script / normalized entries | `scripts/check_unsafe_comments.sh`, `check_panic_surface.sh`, `check_linked_footprint.sh` in `quire-contract-runtime` and peers | **Covered** by `audit-script`                                                                     |
| Advisory scans                    | `cargo audit`, `cargo deny`                                                                                                     | **Covered** by `cargo-audit` and `cargo-deny`                                                                   |
| Measurements                      | `scripts/measure_rlib_size.sh` in `quire-contract-runtime`                                                                      | **Covered** by `quoin measurement record`; not an adapter concern                                 |
| Contract conformance JSONL        | `quire-contract-conformance run --manifest` in `quire-contract-ir`                                                              | **Adapter added** — `contract-conformance`                                                        |
| Differential reports              | `quire-analyze run-package` (`quire.differential-report/v2`)                                                                   | **Adapter added** — `differential-report`                                                         |
| tl-mltl live differential outputs | `tl-mltl.r2u2-past-differential/v1`, `tl-mltl.r2u2-future-differential/v1` (`src/r2u2_live.rs`), `tl-mltl.campaign-run/v1` (`src/campaign.rs`) in `tl-mltl` | **Not covered** — `differential-report` does not read them; a reader needs its own ticket and a captured sample |
| Corpus reports                    | `corpus/*/manifest.json` in `quire-contract-ir` and `tl-mltl`                                                                   | **Out of scope** — a corpus manifest is a verification _definition_, owned by Quire, not a result |
| Kani outputs                      | none of the eight runs Kani today                                                                                               | **No adapter** — nothing to pin                                                                   |
| Solver analysis reports           | `quire-analyze run-package`                                                                                                     | **Covered** by `differential-report` (FR-150)                                                     |
| Counterexamples                   | retained inside the v2 report as engine model bytes                                                                              | **Not transcribed** — the adapter decodes no raw solver output (FR-150)                           |
| Domain-specific proof reports     | none emitted today                                                                                                              | **No adapter** — nothing to pin                                                                   |

Two scope items get no adapter for the same reason: **no producer emits
them yet.** Writing a reader for a format nobody produces means inventing the
format, and the first real sample would then be shaped to fit the reader rather
than the other way round. When one of those repositories emits a real sample,
that is the ticket to reopen.

## The two adapters added

### `contract-conformance`

- **Producer** — `quire-contract-conformance run --manifest corpus/contract-v0.1/manifest.json`, `agent-ix/quire-contract-ir`.
- **Sample** — `tests/fixtures/evidence/contract-conformance-real.jsonl`: seven rows of a real 99-row run, each line unedited, spanning all four operations (`package`, `expression`, `coverage`, `migration`) and both valid and invalid fixtures.
- **Governed target record** — `RunRecord` entries, one per replayed fixture.
- **Why nothing existing represents it** — it is JSONL, one object per line. `entries` requires a single object with an `entries` array; `junit` requires XML. The finding-shaped adapters would write it into `findings/`, which loses the clean-versus-unrun distinction FR-034 exists to make.
- **Identity** — `<corpus>::<operation>::<fixture>`. The same fixture id is replayed under several operations, so collapsing them would let one result overwrite another.
- **Trace metadata** — optional row `trace_ids` is retained as `RunEntry.traceIds`, preserving order and values. Supplied arrays must be non-empty and contain distinct nonblank strings. Omission remains compatible with the original producer. `tests/fixtures/evidence/contract-conformance-traces-real.jsonl` adds a byte-exact row from the upstream runner. The adapter preserves identifiers; the producer owns their verification meaning, and the existing record path performs binding.

### `differential-report`

- **Producer** — `quire-analyze run-package`, `agent-ix/quire-analyze`, which publishes one `quire.differential-report/v2` per analysis request (two engines, Z3 and cvc5). Specified in FR-150; it replaces the earlier `<domain>.differential-summary/v1` reader, whose schema no code wrote.
- **Sample** — byte-exact reports captured from `quire-analyze run-package`, one per disposition the runner can produce, kept as test fixtures under `tests/fixtures/evidence/` by the implementing change (VER-80); the `unsupported-platform` fixture is captured on a non-Linux host or hand-constructed and validated by the owner's validator. quoin keeps no copy of the schema.
- **Governed target record** — one `RunRecord` entry per report (`agreement` is `pass` or `fail`; `inconclusive` and `unavailable` are `error`); a `disagreement` is named, not transcribed, and writes a run with no entries.
- **Why nothing existing represents it** — `junit` has no state for "the engines disagreed" or "an engine could not run"; the finding-shaped adapters would record a disagreement as a rule violation, which is a different claim.

## The state that does not fit, and what happens to it

`RunEntry.outcome` is `pass | fail | skip | error`. A differential report's
`disagreement` (two engines that both ran and reached opposite conclusions) is
none of those: it asserts no refutation, and it is not a producer that failed
to run.

Rather than round it to the nearest word, the adapter **does not transcribe it
and names it**. `AdapterResult.unrepresented` carries the producer's own state
verbatim with the reason no outcome holds it, and `quoin evidence record`
prints each one:

```
  not transcribed — <analysisKind>::<requestDigest> reported disagreement: <engine> <status>/<outcome>,
  <engine> <status>/<outcome>; that is neither a pass nor a fail nor an error,
  and no run-entry outcome carries it
```

That is an adapter-layer field. It adds no record family, changes no stored
schema, and duplicates nothing Engineering Assurance owns. Extending the stored
outcome vocabulary is a larger decision than an adapter ticket, and it should be
made against a count of how many real producers need it — not against this one.

## What every adapter here does not do

Neither adapter runs a producer, reads a repository, or decides whether the
evidence was sufficient. Both fail closed on an unknown protocol or schema
version, on an unknown status, and on an empty result set: a conformance run
that emitted no line did not report that every fixture matched. A differential
report always holds one comparison, and is refused unless `quire-analyze`'s own
validator accepts it.
