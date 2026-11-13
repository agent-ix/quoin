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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | The fix's FR-093 limitation rule is a free-text substring test: a limitation refuses the use when it `contains` the obligation id or the text `analysis-act`. Neither spec states this rule. It refuses wrongly in two ways. `FR-001-AC-1` matches a limitation about `FR-001-AC-10`. A limitation that confines reliance *to* the act, for example "valid for analysis-act/1 on Linux only", refuses the very use it permits. It also accepts wrongly: a limitation that excludes the act or target in other words ("not for consistency checks of FR-001") is accepted, and only recorded in provenance. Free-text limitations cannot be evaluated by matching strings. Either treat a decision with limitations as `unevaluated` on the bounded-reliance premise until limitations are structured, or get an owner ruling. | rust/crates/quoin-evidence/src/qualify/premises.rs:138-148 |
| FND-008 | low | `Error::AuditFailed { reason: String }` stringifies the `AuditorError` with `to_string()`. This crate's own convention, in `coverage_error`, is to map an engine error onto a typed variant rather than stringify it, so a new variant fails the build. Carry the source (`#[source]`) or map `AuditorErrorCode` explicitly. | rust/crates/quoin-quire/src/error.rs:434-439; rust/crates/quoin-quire/src/evidence.rs:165-167 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Observations are now per record: `RetainedRun.observation: RunObservation` (performed revision, execution, actor), and `HostObservations` keeps only reliance and selected HEAD. Tested by `an_observation_of_one_record_never_qualifies_another`. |
| FND-002 | fixed | `reliance` runs `assess_trust` and compares `accepted_context.name`/`version` with the report's producer, which is qualification-rejected on mismatch. Limitations are now read; see new FND-007 for the rule used. |
| FND-003 | still-open | The bridge now runs `quoin_auditor::audit` over `latest_per_suite`, but turns only `report.findings` into refusals. It ignores `report.unevaluated` and `report.healthy`. With the default `AuditInput` the bridge builds (`injections` and `mock_inspection_suites` both `None`), the ladder records a mocked-confirmation check under `unevaluated` for every run-backed suite, so the obligation is NOT healthy (audit/mod.rs:166-176; ladder.rs:189-226). The bridge still qualifies it, which is what `an_admitted_passing_report_is_a_qualified_record_for_its_own_target_only` counts. The PR's own AC-8 test had to supply `injections` and `mock_inspection_suites` to make the audit healthy. FR-116 says the qualifier SHALL retain the audit's failure/unevaluated details "without granting positive qualification". Fix: require `healthy.contains(id)`, map `unevaluated` entries to `unevaluated` refusals, and have the host supply the mock-inspection inputs, or get an owner ruling that the check does not apply to analysis suites. |
| FND-004 | fixed | `ActorAdmission { actor, admission_ref, authorized }`. A missing ref is unevaluated, out of scope is qualification-rejected, and provenance is `actor-authority:<actor>:<ref>`. Tested. |
| FND-005 | fixed | Refusals sort by `(locator, cause, premise, detail)`. |
| FND-006 | fixed | Entries are collected once into `Vec<CriterionEntry>` and indexed with `get`. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | `the_reliance_decision_must_be_for_the_reports_producer_and_its_limits_read` is tagged `FR-116-AC-6`, but it now asserts the opposite of that criterion. A decision whose limitation reads "does not cover FR-001-AC-1" yields `counted == 1` for FR-001-AC-1. FR-116-AC-6 says a "limitation excluding the required act grants no positive verification decision". A test that passes only while the AC is violated is a wrong trace and locks in the violation. The test name ("its limits read") also no longer describes the behaviour. | rust/crates/quoin-quire/tests/analysis_counting.rs:566-600 |

## Dispositions (round 2)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-003 | fixed | `audit_outcome` carries `healthy` plus every finding and every `unevaluated` check for the obligation. `qualify` refuses on each detail, and on `!healthy` with no details. The host now supplies `injections` and `mock_inspection_suites` on `Request`. Tested by `without_the_hosts_mock_inspection_the_audit_cannot_call_the_obligation_healthy` (keeps `mocked-confirmation`). |
| FND-007 | still-open | The substring rule is gone, so the wrong refusals are fixed. But limitations are now never read, so a limitation that excludes the target or act is accepted outright. The test asserts that "does not cover FR-001-AC-1" counts FR-001-AC-1. FR-116 (line 152) and FR-116-AC-6 forbid this. The stated reason ("the owner's real decision carries free-text limitations, so unevaluated-forever was rejected") arrived as relayed text. No FR-093, FR-116 or FR-184 amendment and no recorded owner ruling on the branch or main says free-text limitations are provenance-only. Close it one of three ways: record that owner ruling and amend FR-116/FR-093 (then accepted-no-change); add structured limitation scope; or keep a decision with limitations `unevaluated`. |
| FND-008 | fixed | `AuditFailed { code: AuditorErrorCode, reason }` maps `error.code` and `error.message` instead of `to_string()`. |

## Dispositions (round 3)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-007 | fixed | `reliance` fails closed. A decision with any limitation is `unevaluated` on the bounded-reliance premise, and the limitation text is kept in the detail. This matches FR-116 line 152 and FR-116-AC-6 without guessing at free text. |
| FND-009 | fixed | The test was split. `the_reliance_decision_must_be_for_the_reports_producer` is tagged FR-116-AC-15 and covers the producer mismatch and the no-limitation positive case. `a_reliance_decision_with_any_limitation_grants_no_positive_decision` is tagged FR-116-AC-6 and shows an excluding limitation and a harmless-looking one are each uncounted and unevaluated, with the text kept. |
