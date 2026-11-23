---
id: SR-20261123163420
title: "Base review of PLAT-1806 evidence record exit status and pytest symbol mapping"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-030-evidence-store.md; spec/functional/FR-033-evidence-format-adapters.md (PR #1025, ticket PLAT-1806)"
review_set: subset
---

# Base review of PLAT-1806 evidence record exit status and pytest symbol mapping

## Summary

Ticket: PLAT-1806. PR agent-ix/quoin#1025 adds the FR-030 exit-status SHALL with FR-030-AC-32/33/34, and to FR-033 it adds the pytest symbol-mapping section with FR-033-AC-16/17. Each AC was checked against `rust/crates/quoin-evidence/src/record.rs`, `adapters/junit.rs`, `types/run.rs`, `quoin-core/src/ops/evidence/records.rs`, `quoin-cli/src/evidence.rs`, `quoin-cli/src/main.rs` and the quire-rs Python extractor (`src/symbols/python.rs`, `qualify`). `quire validate` on both documents passes (2/2 grammar-clean). Two high findings block merge. FR-030-AC-33 turns every legitimate recording whose adapter never carries trace ids (cargo-mutants, and sbom without `--discharges`) into an error. The FR-033 description procedure also doubles the FR-120 emitter's symbol, which contradicts FR-033-AC-16.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-030-AC-33 applies to every run adapter, so a cargo-mutants run (whose entries name production functions and bind nothing by design, FR-033) and an sbom run without `--discharges` (FR-041-AC-7 "binds nothing") exit non-zero; scope it to test-result adapters | spec/functional/FR-030-evidence-store.md:264; spec/functional/FR-033-evidence-format-adapters.md:75-80; rust/crates/quoin-evidence/src/adapters/registry.rs:97-150 |
| FND-002 | high | FR-033 pytest procedure step 2 exempts only a case with no `file` whose classname equals name, but FR-120 emitters write `file` too, so `classname=name="TestDose.test_a" file="tests/test_bolus.py"` becomes `TestDose.test_a.TestDose.test_a`; contradicts FR-033-AC-16 and FR-120-AC-2 | spec/functional/FR-033-evidence-format-adapters.md:85-97 |
| FND-003 | medium | Exit value is "non-zero" in the FR-030 SHALL and AC-32/33; state exit 1 (the payload-carrying Partial status) so the test can assert it and stdout is still guaranteed | spec/functional/FR-030-evidence-store.md:128-132,263-264 |
| FND-004 | medium | The spec never says how the adapter knows a case is a pytest case; the settled rule (`file` attribute ending `.py`) and the out-of-scope case (no `file`, classname != name) are not written down | spec/functional/FR-033-evidence-format-adapters.md:83-97,137 |
| FND-005 | medium | FR-030-AC-32 "stdout still carries the unmatched list" is untrue of the human render today (it prints only `recorded <suite> → <path>` and unrepresented lines); say whether the AC means the `--json` payload or requires the human render to list unmatched ids | spec/functional/FR-030-evidence-store.md:262; rust/crates/quoin-cli/src/evidence.rs:187-214 |
| FND-006 | low | FR-030-AC-34 "trace ids that every obligation states" reads as ids common to all obligations; the FR-030 SHALL says "failed or was skipped" while AC-34 adds "errored" | spec/functional/FR-030-evidence-store.md:128-132,264 |
| FND-007 | low | AC-32/33 put a prose line on stderr; the repo boundary rule says stderr carries a JSON diagnostics array. The CLI's `warning` prose path (main.rs:614-621) makes it implementable, but the spec should state the line is that warning and not a diagnostic | spec/functional/FR-030-evidence-store.md:262-263; rust/crates/quoin-cli/src/main.rs:613-665 |
| FND-008 | low | Procedure step 3 relies on `RunEntry.path` (FR-120-AC-3) and the store join (FR-030-AC-18); neither exists on main (`RunEntry` has `deny_unknown_fields` and no `path`), and no new AC tests step 3, so name the dependency | spec/functional/FR-033-evidence-format-adapters.md:96-97; rust/crates/quoin-evidence/src/types/run.rs:103-133 |

## Verdict

Not mergeable as written. FND-001 and FND-002 are spec contradictions that a coder following the text would implement wrongly. FND-003 to FND-005 are fixes to the text that the lead's settlements already decide. What holds: AC-32's "including one carried only by a failing or skipped entry" matches record.rs, where unmatched ids are collected before the pass-only filter at lines 155-170. The publication order (the run is written first and the bindings before any exit) is already the code's order. Byte-ordered unmatched ids are already produced by the BTreeSet and the later sort/dedup. AC-34 is consistent with FR-030-AC-4. FR-033-AC-16's two worked examples agree with the quire-rs Python `qualify` (bare at module level, `Class.method` under a class, no module prefix). FR-033-AC-17 agrees with FR-120-AC-5.

## Examined

FR-030 SHALL (exit status), FR-030-AC-32, FR-030-AC-33, FR-030-AC-34, FR-033 pytest procedure section, FR-033-AC-16, FR-033-AC-17 (examined). FR-030-AC-4, FR-030-AC-7, FR-030-AC-15, FR-030-AC-18, FR-030-AC-30, FR-033-AC-12, FR-041-AC-7, FR-041-AC-8, FR-120 (context only).
