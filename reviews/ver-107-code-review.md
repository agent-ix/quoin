---
id: SR-20261113155334
title: "Code review (Rust lane) — VER-107 FR-116 qualifier and quoin-quire bridge (quoin PR 877)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-evidence/src/qualify/mod.rs, rust/crates/quoin-evidence/src/qualify/premises.rs, rust/crates/quoin-evidence/src/qualify/runs.rs, rust/crates/quoin-evidence/src/lib.rs, rust/crates/quoin-quire/src/evidence.rs, rust/crates/quoin-quire/src/coverage.rs, rust/crates/quoin-quire/src/lib.rs, rust/crates/quoin-quire/Cargo.toml, rust/Cargo.lock, rust/crates/quoin-quire/tests/analysis_counting.rs, rust/crates/quoin-quire/tests/fixtures/analysis/**"
review_set: subset
---

## Summary

Ticket: VER-107. PR head `db0ed2ae`, diff `git diff origin/main...HEAD` (one commit, 12 files).
Rust lane (rust-style) folded in. Checked against FR-116, FR-153, FR-184, FR-152, FR-093 and
TC-002 steps 3 to 6 as merged on main. Focused checks run on this head:
`cargo test -p quoin-quire --test analysis_counting` (13 passed), `cargo fmt --check` on both
crates (clean), `cargo clippy -p quoin-evidence -p quoin-quire --all-targets --all-features
-D warnings` (clean).

The disposition mapping is right and tested. refuted maps to failed, inconclusive and unavailable
to errored, disagreement to absent (the same result as a never-recorded obligation), a superseded
pass to stale with the later outcome kept, and a non-Analysis method to qualification-rejected on
the method and act pairing premise. The FR-030 run order (timestamp, then file name, whatever the
supply order) is implemented and tested. A claim is emitted only from `Qualification::Qualified`,
and trace reuses the same backing projection, so it returns record claims and never mints a source
binder. A multi-obligation `--discharges` qualifies none. With no FR-093 assessment or no CI
checkout SHA, the result is `unevaluated` and names the right premise. The PR adds no pins,
digests, ledgers, compatibility readers, vendored files or workflow edits. The new production code
has no `unwrap`, `expect`, indexing or `unsafe`. The `coverage::walk` refactor keeps the
exhaustive `CoverageError` mapping.

## Verdict

Not mergeable at this head. FND-001 is a spec claim the code violates: FR-116 exact correlation
and FR-116-AC-7. The host observations are one request-wide value applied to every retained run,
so one CI checkout SHA and one CI invocation qualify records from separate acts. The PR's own
FR-153-AC-6 test counts two claims from two separately recorded reports under one invocation.
Nothing ships qualified today, because no FR-093 assessment exists, but the qualifier is the
owner contract that a future assessment will unlock. FND-002 and FND-003 are real correlation and
audit gaps on the same path. The rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Performed revision, execution association and actor are one `HostObservations` per request, applied by `host_premises` to every candidate run whatever its record. Nothing correlates them to the record/entry they observe. FR-116 requires correlation with the same selected record and entry: "a current HEAD paired with an unrelated historical record cannot make it current" (FR-116-AC-7), and "a passing unrelated CI job cannot make an older record current". FR-184 defines the act as one `run-package` invocation. Reproduced by the PR's own test: two reports recorded into SUITE-A and SUITE-B are both qualified (`counted == 2`) under a single CI run/job/invocation and one checkout SHA. An old retained record is qualified by today's SHA in the same way. Fix: carry the host observations per retained run (for example on `RetainedRun`, keyed by its locator), and make a run with no correlated observation `unevaluated`. | rust/crates/quoin-evidence/src/qualify/mod.rs:157-185; rust/crates/quoin-evidence/src/qualify/premises.rs:31-41; rust/crates/quoin-quire/tests/analysis_counting.rs:591-594 |
| FND-002 | medium | The bounded-reliance premise checks only the assessment's status and `permitted_decisions`. It never compares `TrustAssessment.producer` (or the assessed context) with the report's `producer.name`/`producer.version`, which the tool premise reads. An assessment for any producer that permits `qualify-analysis-act` therefore admits a report from another producer or version. FR-116 requires the FR-093 decision "for the exact intended producer use", correlated with the same record. `AcceptedWithLimitations` is also accepted with its limitations unread (FR-116: "a limitation that excludes a required act or requested target cannot be treated as an accepted exception"). | rust/crates/quoin-evidence/src/qualify/premises.rs:43-97 |
| FND-003 | medium | The bridge never invokes the FR-032 audit. FR-116 Inputs say "the bridge invokes the shared audit itself"; FR-153 credit step 2 is "a binding [that] survives the FR-032 audit of the latest run"; FR-153-CON-3 keeps the audit's failed-run rule as FR-032 defines it. `runs::dispositions` re-implements latest-run selection and binding presence instead, so audit findings (failed-run staleness, vacuity, orphaned binding) never reach the qualifier. Either way that is a second checker that can drift from `quoin_auditor::audit::audit`, which is available to quoin-quire. | rust/crates/quoin-evidence/src/qualify/runs.rs:54-163; rust/crates/quoin-quire/src/evidence.rs:127-215 |
| FND-004 | low | The actor-authority premise admits any non-empty string. The positive decision keeps only `actor-authority:<name>`, with no host authority-admission reference and no authorized scope for the obligation/method. FR-116 Inputs and FR-116-AC-18 require the admission basis to be exposed, and FR-116-AC-6 requires an out-of-scope authority to be qualification-rejected. | rust/crates/quoin-evidence/src/qualify/premises.rs:99-114; rust/crates/quoin-evidence/src/qualify/mod.rs:157-170 |
| FND-005 | low | Refusals are sorted by the derived `Ord`, whose first field is `cause`, so the result order is cause then premise then locator. FR-116 orders results by target, then record/entry locus, then cause category. | rust/crates/quoin-evidence/src/qualify/mod.rs:200-211, 283-291 |
| FND-006 | low | `selected.entries().nth(ordinal)` is an O(n) walk, and it is repeated per requested id in `compute` and per claim/result inside `with_backing` (and again on every `trace` call), after `entries_stating` has already walked the inventory. Collect the entries once, or have `entries_stating` return the entries. | rust/crates/quoin-quire/src/evidence.rs:114-121, 153-160, 224-229, 287 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-153 disposition table (refuted, inconclusive, unavailable, disagreement) | examined | Correct and tested |
| FR-153 supersession: later fail/error gives stale plus outcome; later lacking gives absent; order by timestamp then file name | examined | Correct and tested in both supply orders |
| FR-153-AC-6 two-suite all-or-nothing | examined | Correct; see FND-001 for the shared host observation |
| FR-153 method pairing (non-Analysis gives qualification-rejected) | examined | Correct |
| FR-184 default unevaluated (no FR-093 assessment / no CI SHA) | examined | Correct, names the premise |
| FR-184 act scope (multi-obligation `--discharges` qualifies none) | examined | Correct |
| FR-184 producer-execute engine fields | examined | Correct field set; no version pin |
| Claim only on positive decision; trace never a source symbol | examined | Correct: claims only from `Qualified`, `binders` stays empty |
| FR-184-CON-1 / FR-116-CON-2 (no pins, digests, ledgers, compat reader, new store field) | examined | Clean |
| FR-184-CON-2 / FR-116-CON-1 (pure reader) | examined | Clean: no clock, file or solver use in the qualifier |
| Panics, unsafe, error mapping, `coverage::walk` refactor | examined | Clean |
| Vendoring, workflow edits | examined | None; fixtures are the repo's own `tests/fixtures` |
| FR-116 exact correlation | examined | FND-001, FND-002 |
| FR-116 audit invocation | examined | FND-003 |
