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

OSV-Scanner report bytes and offline database archives are not checked into this
repository. The ignored `tc_osv_scanner_authentic_ecosystem_report_is_parsed_and_joined`
test reads `QUOIN_TEST_OSV_REPORT` in place and requires an authentic, unedited report
from OSV-Scanner 2.7.1 over one CycloneDX SBOM containing npm, PyPI, Go and Debian OS
packages, a scoped npm package, and same-name npm/PyPI packages. It checks exact
purl/advisory identity, including the Debian distro-qualified purl, scoped npm purl,
and the two distinct `redis` package identities, through the adapter and join. It is
an adapter/join behavior check; it does not establish compliance or discharge
obligations.

The local replay input adds `pkg:npm/%40hapi/hoek@8.3.2`, `pkg:npm/redis@3.0.0`,
and `pkg:pypi/redis@4.4.1` to the four-ecosystem SBOM. The authentic report includes
`GHSA-22h7-7wwg-qmgg` for the scoped npm package, `GHSA-35q2-47q7-3pc3` for npm redis,
and `GHSA-24wv-mv5m-xv4h` for PyPI redis. Only the report bytes are passed to this
test; no report or database archive is checked in.

With OSV-Scanner 2.7.1 installed and its user-local database already available, produce
an external JSON report from a CycloneDX SBOM such as `probe.cdx.json`:

```sh
OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY=/absolute/path/to/local-osv-db \
  osv-scanner scan source --offline --all-packages \
  -S /absolute/path/to/probe.cdx.json --format json \
  --output-file /absolute/path/to/report.json
```

From the repository root, replay that file through the Rust adapter and SOUP join:

```sh
cd rust
QUOIN_TEST_OSV_REPORT=/absolute/path/to/report.json \
  env -u CARGO_TARGET_DIR /home/peter/dev/worktrees/locked-build.sh "$PWD/target" \
  cargo test -p quoin-cli --bin quoin -- --ignored \
  tc_osv_scanner_authentic_ecosystem_report_is_parsed_and_joined
```

The path must be absolute. This test reads the supplied report without rewriting it;
the scanner binary and database remain external to the repository.
