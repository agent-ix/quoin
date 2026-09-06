# Evidence adapter fixtures

`contract-conformance-traces-real.jsonl` is one byte-exact row, selected by
`fixture_id: package-invalid-namespace`, from the 99-row output of
`cargo run --locked --offline --bin quire-contract-conformance -- run --manifest corpus/contract-v0.1/manifest.json`
at `agent-ix/quire-contract-ir` revision
`9b9102c3806e9cda0ed70312f4f6c23a211f6fbf`, captured 2026-09-06 using an isolated
Cargo target directory. The full output SHA-256 is
`584248f25ef48a3e1fb782603739fe7dfa7c27082703fdfafe7e1df414488c6e`.
The row carries `trace_ids`; the older `contract-conformance-real.jsonl`
remains the compatibility fixture for producers omitting that optional field.
The source is a reviewed candidate, not a claim of accepted producer bindings.
Negative/mismatch and custom-order variants are constructed explicitly in tests.

`cargo-audit-real.json` is **real output**, captured with
`cargo audit --json` in `agent-ix/quire-rs` on 2026-08-18. It is not
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
