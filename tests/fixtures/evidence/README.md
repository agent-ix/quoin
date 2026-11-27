# Evidence adapter fixtures

The three `contract-conformance-*.jsonl` fixtures are unedited rows from the output of
`filament-contract-conformance run --corpus corpus/contract-v0.1 --schemas schemas`
(`cargo run --features conformance-cli`, as `make corpus` does) in
`agent-ix/filament-contract-ir` at `ce1970fb`, selected by `fixture_id` and `operation`.
`contract-conformance-real.jsonl` holds five rows (`package-constructs`,
`package-invalid-namespace`, `expression-boolean-literal`, `expression-invalid-wire`,
`coverage-shallow`); `contract-conformance-traces-real.jsonl` is the `package-constructs`
row with its seven ordered `trace_ids`; `contract-conformance-criteria-real.jsonl` is the
`package-invalid-namespace` namespace-diagnostic row.
The source is a reviewed candidate, not a claim of accepted producer bindings.
Negative/mismatch and custom-order variants are constructed explicitly in tests.
`rust/crates/quoin-evidence/tests/tc_contract_conformance_fixtures.rs` parses all three.

The criteria row validates coverage-token-to-criterion ownership before execution;
the sample names FR-011-AC-3 and FR-018-AC-1, not operation-wide Test Case aliases.
The requirement documents it names (`FR-011-AC-3`, `FR-018-AC-1`) are owned by
`agent-ix/filament-contract-ir`; quoin keeps no copy of them. TC-1587 is unbacked until
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

`differential-report-v2/` holds `filament.differential-report/v2` documents for the
`differential-report` adapter (FR-151, TC-001). The accepted reports except one are unedited,
byte-exact captures written by `filament-analyze run-package` at
`agent-ix/filament-analyze` `e17fbf0f6c593d32ce2555420d615c5dd862e8ef`. The capture used
`engineering-assurance producer-execute` 0.8.2, the version pinned by this Quoin main, and
deterministic local shell stubs for the solver executables because Z3 and cvc5 are not
installed: `agreement-satisfied`, `agreement-refuted`, `disagreement`, `inconclusive-unknown`,
`inconclusive-timeout` and `unavailable-missing-executable`. They are producer reports driven by
scripted solver responses, not claims of real solver output. `unavailable-unsupported-platform`
is the one documented derived case: engine 0's `outcome` is set to `unsupported-platform` and
its `status` to `unsupported`, then the document is re-emitted as canonical JSON. A Linux runner
reports a containment failure as `cleanup-failed`, so it cannot produce this outcome.
`filament-analyze`'s own validator accepts it. The refusal fixtures are derived in
`rust/crates/quoin-evidence/tests/tc_differential_report_v2.rs`, one change each.

## OSV-Scanner external report replay

`osv-scanner-2.7.1-authentic.json` is the raw, unedited JSON output of the official
OSV-Scanner 2.7.1 Linux amd64 release asset (SHA-256
`fb7dab19a8bd9bbe5c3189f09d4b7ee1820c9553a23bac202648a56efd2f6ee8`, version reports
commit `02f96a40691c2d155fdda799af436e111231e365`). It was captured on 2026-11-27 with
`osv-scanner scan source --sbom probe.cdx.json --format json --all-packages
--output report.json`. The small npm `package-lock.json` used to establish the npm
coordinates was generated with npm 10.9.2 from the registry; its SHA-256 is
`abe46209f2214b29c8b8edd0bd4c401e5d0696a5cb45beeb93c583424c62b76d`. The composite
CycloneDX probe also contains the requested PyPI, Go and Debian package coordinates,
including scoped npm and same-name npm/PyPI packages; the checked-in probe bytes have
SHA-256 `4ab9f7a64535df10aa866de1fd6d10cc59d391a52bc3205251e859953ffa647b`. The
checked-in report bytes have SHA-256
`e27bf74fccabcfc972735a3e7850bd10398321fb65bc9c6de5dc1c7bb934ace8`.

`tc_osv_scanner_authentic_ecosystem_report_is_parsed_and_joined` replays those bytes
without rewriting them. It checks exact purl/advisory identity, including the Debian
distro-qualified purl, scoped npm purl, and the two distinct `redis` identities,
through the adapter and SOUP join. It is an adapter/join behavior check; it does not
establish compliance or discharge obligations. No offline database archive is checked
in.
