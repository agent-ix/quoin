# Evidence adapter fixtures

`contract-conformance-traces-real.jsonl` is one byte-exact row, selected by
`fixture_id: package-invalid-namespace`, from the output of the
`quire-contract-conformance` runner over the `agent-ix/quire-contract-ir`
contract corpus.
The row carries `trace_ids`; the older `contract-conformance-real.jsonl`
remains the compatibility fixture for producers omitting that optional field.
The source is a reviewed candidate, not a claim of accepted producer bindings.
Negative/mismatch and custom-order variants are constructed explicitly in tests.

`contract-conformance-criteria-real.jsonl` is the unedited namespace-diagnostic
row from the native IR runner.
This candidate validates coverage-token-to-criterion ownership before execution;
the sample names FR-011-AC-3 and FR-018-AC-1, not operation-wide Test Case aliases.
The requirement documents it names (`FR-011-AC-3`, `FR-018-AC-1`) are owned by
`agent-ix/quire-contract-ir`; quoin keeps no copy of them. TC-1587 is unbacked until
a native test reads them from that repository.
Candidate integration is demonstrated; final producer acceptance and release
qualification remain open.

`cargo-audit-real.json` is **real output**, captured with
`cargo audit --json` in `agent-ix/quire-rs`. It is not
hand-written, and it is checked in unedited.

The ticket that asked for these adapters (agent-ix/quoin#115) was explicit:
decide the format questions *"by reading real output from each tool, not from
the spec of the format."* A fixture someone wrote to match their own reader
proves the reader parses itself.

What this file happens to contain is the case that matters most: **zero
vulnerabilities and one `unsound` warning**, alongside the `database`
(1217 advisories) and `lockfile` the scan consulted. That is a scan which ran
and found almost nothing — the state `FindingRecord` exists to tell apart from
a scan that never ran.

`cargo-mutants-real.json` is the `mutants.out/outcomes.json` of
`cargo mutants` 27.1.0 (fresh target dir, one job) over a three-function
scratch crate, with missed and caught mutants, unedited except that the absolute path of the `cargo` binary is shortened to `cargo`. Its `name` carries the
`<file>:<line>:<col>: ` locus prefix that the adapter strips.

`syft-cyclonedx-real.json` is the unedited output of Syft 1.57.1,
`SYFT_FILE_METADATA_SELECTION=none syft scan dir:. -o cyclonedx-json=...`, run with an empty
config over a scratch crate whose `Cargo.lock` lists the fictional `fictional-widget`
(the root package, which Syft emits with no purl) and three fictional registry
dependencies. It holds four components: three carry a purl, one is identified by name and version.
