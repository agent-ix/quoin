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
| Advisory scans                    | `cargo audit`, `cargo deny`                                                                                                     | **Covered** by `cargo-audit`                                                                      |
| Measurements                      | `scripts/measure_rlib_size.sh` in `quire-contract-runtime`                                                                      | **Covered** by `quoin measurement record`; not an adapter concern                                 |
| Contract conformance JSONL        | `quire-contract-conformance run --manifest` in `quire-contract-ir`                                                              | **Adapter added** — `contract-conformance`                                                        |
| Domain differential reports       | `corpus/r2u2-v4.2/differential-report.json` in `tl-mltl`                                                                        | **Adapter added** — `differential-report`                                                         |
| Corpus reports                    | `corpus/*/manifest.json` in `quire-contract-ir` and `tl-mltl`                                                                   | **Out of scope** — a corpus manifest is a verification _definition_, owned by Quire, not a result |
| Kani outputs                      | none of the eight runs Kani today                                                                                               | **No adapter** — nothing to pin                                                                   |
| Solver analysis reports           | `quire-analyze` is a stub whose library surface is `hello()`                                                                    | **No adapter** — nothing to pin                                                                   |
| Counterexamples                   | carried inside the differential report's own cases                                                                              | **Covered** by `differential-report`                                                              |
| Domain-specific proof reports     | none emitted today                                                                                                              | **No adapter** — nothing to pin                                                                   |

Three scope items get no adapter for the same reason: **no producer emits
them yet.** Writing a reader for a format nobody produces means inventing the
format, and the first real sample would then be shaped to fit the reader rather
than the other way round. When one of those repositories emits a real sample,
that is the ticket to reopen.

## The two adapters added

### `contract-conformance`

- **Producer** — `quire-contract-conformance run --manifest corpus/contract-v0.1/manifest.json`, `agent-ix/quire-contract-ir`.
- **Pinned sample** — `tests/fixtures/evidence/contract-conformance-real.jsonl`: seven rows of a real 99-row run, each line unedited, spanning all four operations (`package`, `expression`, `coverage`, `migration`) and both valid and invalid fixtures. The full run is `sha256:b003309d25f6002c73c0c660725dce0eb2139dafd1198c794d0e561aca147485`.
- **Governed target record** — `RunRecord` entries, one per replayed fixture.
- **Why nothing existing represents it** — it is JSONL, one object per line. `entries` requires a single object with an `entries` array; `junit` requires XML. The finding-shaped adapters would write it into `findings/`, which loses the clean-versus-unrun distinction FR-034 exists to make.
- **Identity** — `<corpus>::<operation>::<fixture>`. The same fixture id is replayed under several operations, so collapsing them would let one result overwrite another.
- **Trace metadata** — optional row `trace_ids` is retained as `RunEntry.traceIds`, preserving order and values. Supplied arrays must be non-empty and contain distinct nonblank strings. Omission remains compatible with the original producer. `tests/fixtures/evidence/contract-conformance-traces-real.jsonl` adds a byte-exact row from candidate `9b9102c3806e9cda0ed70312f4f6c23a211f6fbf`; its capture command and full-run digest are recorded in the fixture README. The adapter preserves identifiers; the producer owns their verification meaning, and the existing record path performs binding.

### `differential-report`

- **Producer** — `agent-ix/tl-mltl`, comparing its engine against R2U2 4.2-release / C2PO 4.1.0.
- **Pinned sample** — `tests/fixtures/evidence/differential-report-real.json`, the committed report at `tl-mltl` `fe1c620`, unedited. It carries eight agreements and one `unsupported`.
- **Governed target record** — `RunRecord` entries, one per compared case.
- **Why nothing existing represents it** — `junit` has no state for "the external reference cannot express this case"; `cargo-mutants` is mutation-shaped; `agent-eval` is measurement-shaped; the finding-shaped adapters would record a disagreement as a rule violation, which is a different claim from "two implementations disagree" and one the report does not make.
- **Schema family** — `<domain>.differential-summary/v1`, so another domain can use the shape without claiming `tl-mltl`'s identity. A `v2` is refused rather than read by a `v1` reader that happens to recognise the fields.

## The state that does not fit, and what happens to it

`RunEntry.outcome` is `pass | fail | skip | error`. A differential report's
`unsupported` is none of those: nothing chose to omit the case, and nothing
failed.

Rather than round it to the nearest word, the adapter **does not transcribe it
and names it**. `AdapterResult.unrepresented` carries the producer's own state
verbatim with the reason no outcome holds it, and `quoin evidence record`
prints each one:

```
  not transcribed — closed-profile-not-mapped-v1 reported unsupported: the external
  reference does not support this case; that is neither a skip nor an error,
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
that emitted no line did not report that every fixture matched, and a
differential that compared no case did not report agreement.
