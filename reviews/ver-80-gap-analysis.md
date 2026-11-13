---
id: SR-20261113055215
title: "Gap analysis — VER-80 differential-report v2 adapter against FR-151, NFR-032, TC-001 (quoin PR 850)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-151-differential-report-v2-adapter.md, spec/non-functional/NFR-032-differential-report-intake-is-bounded-and-deterministic.md, spec/test/TC-001-differential-report-v2-adapter.md, spec/functional/FR-069-campaign-native-adapters.md, docs/campaign-native-result-inventory.md, rust/crates/quoin-evidence/tests/tc_differential_report_v2.rs, rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-151", type: references}
  - {target: "ix://agent-ix/quoin/NFR-032", type: references}
  - {target: "ix://agent-ix/quoin/TC-001", type: references}
---

## Summary

Ticket: VER-80. PR head `e6c29c90`. Plan completion: not assessed. Each acceptance criterion was
checked against the tagged tests and against the built `quoin evidence record` binary. The tests
cover AC-1, AC-2, AC-4, AC-5, AC-6, AC-9, AC-10 (adapter level), AC-11 and AC-12. AC-13 and
CON-1 to CON-3 hold on inspection. CON-4 holds for `cargo check` only (SR-20261113055214 FND-004).
The schema refusals of AC-6 name the value, as required.

## Verdict

Gaps remain in AC-3 (human output), AC-7 (naming the member), AC-8/AC-14 (non-UTF-8) and in one
FR-151 statement about entry identity. FND-001 must be fixed in quire-analyze; the spec wording
there is right.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-7 requires the refusal to name the member, but for wrong-type and out-of-set values the owner's validator names only the value. Reproduced: `"x" is not of type "integer"`, `"maybe" is not one of [...]`, `"bogus" is not one of [...]`, which does not say whether the engine `status` or `outcome` was wrong. The table rows for these classes assert the value or `""`, so they cannot fail on the AC-7 property. The "wrongly typed member" row (`logic = 5`) actually trips a const check (`"QF_UF" was expected`), not a type check. Fix upstream, not in the spec: in quire-analyze `validate_report_document`, use `format!("report does not satisfy its schema at {}: {error}", error.instance_path)` (jsonschema 0.17.1 `ValidationError::instance_path`). Then move quoin's `rev` and assert `/logic`, `/differential/disposition`, `/engines/0/status` and `/engines/0/outcome` in those rows. | rust/crates/quoin-evidence/tests/tc_differential_report_v2.rs:248-276 |
| FND-002 | medium | Non-UTF-8 input (AC-8, and a named row in AC-14 and TC-001 step 2) has no test. The table has 23 rows plus a separate size test, against 25 classes. The behaviour is correct, reproduced through the command: `results are not UTF-8`, exit 1, no run written. It is unrepresentable at the adapter (`&str`), so the test belongs at the CLI: a `\xff\xfe` file, asserting a non-zero exit and that no `spec/evidence/runs` exists. | rust/crates/quoin-cli/src/evidence.rs:119-120 |
| FND-003 | medium | `tc_001_a_disagreement_writes_a_run_with_no_entries_and_prints_the_named_result` is tagged FR-069-AC-6 and FR-151-AC-3. It asserts the dispatch payload only, not printed output, and the human output does not print the result (SR-20261113055214 FND-002). The binding is half-correct: it covers JSON, not human. | rust/crates/quoin-core/tests/tc_ver80_differential_report_record.rs:96-114 |
| FND-004 | medium | FR-151 "Entry identity" says a later run of the same request supersedes the earlier one "and a different request cannot overwrite it". The store's current view is the newest run per suite (`records::latest`), and every report is its own one-entry run. So recording request B into the same suite does supersede request A's run. Either state that each request is recorded into its own suite, or reword the claim to the suite granularity the store has. | spec/functional/FR-151-differential-report-v2-adapter.md (Entry identity) |
| FND-005 | low | FR-151 says the dependency brings `jsonschema =0.17.1` and "both versions coexist in the lockfile". There are three versions (0.17.1, 0.55.1 via quire-verification-contracts, 0.56.0), and coexistence contradicts rust/deny.toml `multiple-versions = "deny"` without a ruling. The spec pre-concedes what the repository's gate refuses. | spec/functional/FR-151-differential-report-v2-adapter.md (How quoin reaches the validator) |
| FND-006 | low | The inventory doc's sample printed line uses the reason format `<engine> <status>/<outcome>`, but the adapter emits `z3 status satisfied outcome sat`, and no human output prints it (FND-003). | docs/campaign-native-result-inventory.md (The state that does not fit) |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-151-AC-1, AC-2 | examined | Covered at adapter and dispatch level |
| FR-151-AC-3 | examined | JSON covered; human output missing (FND-003) |
| FR-151-AC-4, AC-5 | examined | Covered; unsupported-platform fixture is the permitted hand-constructed document |
| FR-151-AC-6 | examined | Covered; value named |
| FR-151-AC-7 | examined | Member not named for type/enum (FND-001) |
| FR-151-AC-8 | examined | Non-UTF-8 untested (FND-002) |
| FR-151-AC-9 | examined | Covered |
| FR-151-AC-10 | examined | Covered at adapter; command bound is lower (SR-20261113055214 FND-003) |
| FR-151-AC-11 | examined | Covered (weakly, SR-20261113055214 FND-006) |
| FR-151-AC-12 | examined | Covered |
| FR-151-AC-13, CON-1..CON-3 | examined | Hold on inspection |
| FR-151-AC-14 | examined | Captures are byte-exact; table misses non-UTF-8 (FND-002) |
| FR-151-CON-4 | examined | cargo check only |
| NFR-032 | examined | Adapter bound and determinism hold |
| TC-001 | examined | Steps run at dispatch level, not the CLI; step 3 at command level hits the 48 MiB bound |
| FR-069-AC-2, AC-3, AC-5, AC-6 | examined | AC-6 human half missing |
