---
id: SR-162
title: "SOL re-review — campaign origin proof and collection replay"
type: SpecReview
analysis: gap-analysis
scope: "Draft PR #627 through 3a3c754; FR-114; PLAT-1061 origin proof; EA f11fa94"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-114"
    type: "reviews"
---

# SR-162: SOL re-review — campaign origin proof and collection replay

## Summary

The collection-context and rehashed-substitution checks address the high finding in SR-160/SR-161. The EA dependency is pinned to f11fa9406d541be19af5c48f61a6e9752f8daaa7 in both manifest and lock. The new origin replay binds declared source files and dependency artifacts to the retained request. The cross-repository source mix-up found at 2e2561e was corrected in 631b7e7 by passing the plan source inventory separately to collection-context replay. TC-1947 at 3a3c754 now exercises authored source-file and dependency origins through full execution and retained replay.

## Verdict

**CONDITIONAL** — both SOL findings are closed by source inspection and passing Linux fixtures. FR-114's broader matrix remains partial; full CLI package auth and full TL Campaign execution are pending.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Closed in 631b7e7: collection replay now derives protected apparatus from the plan repository, matching publication; revised Linux fixture passed | rust/crates/quoin-measurement/src/campaign/verify/attempt.rs:131 |
| FND-002 | low | Closed in 3a3c754: TC-1947 runs authored source-file and dependency inputs through EA, retained replay, and a rehashed role substitution | rust/crates/quoin-measurement/tests/tc_1947_campaign_input_origins.rs:366 |

## Finding detail

### FND-001

At 2e2561e, `check_attempt` passed `source`, selected by `procedure.source_repository`, to `check_collection_context`, while `publish_collection` called `protected_artifacts(plan, plan_source)`. For TL `V10.compile.bounded`, the producer repository is `r2u2`, but MP-117 protects `campaign/procedures/v10-compile-bounded.json` and `src/bin/tl_campaign_check.rs` in `tl-mltl`. Commit 631b7e7 now looks up `own_source.repository` in the verified source inventory, passes that plan-source inventory to `check_attempt`, and uses it only for collection-context protected artifacts; producer input checks still use the producer source. This matches publication's source choice. The Linux fixture now creates distinct plan and producer Git repositories, and keeps the checker on the plan repository.

Focused source-object repro: `git cat-file -e af555f5:<protected path>` succeeds for both MP-117 protected paths in `tl-mltl`; `git cat-file -e 336a245:<protected path>` fails for both in `r2u2`. The revised call now supplies the former source inventory to `expected_artifacts`. The two-repository fixture is gated by `#[cfg(target_os = "linux")]`; the campaign owner ran it in a cached `rust:1.98` Docker container using Rust 1.98.1, `--network none`, and read-only source, registry and Git mounts. Both TC-1942 tests passed, including direct producer/checker execution and independent replay.

### FND-002

At 2e2561e the three origin tests called `check_input_origins` with synthetic request and prior-result records, while the Linux direct-process fixture selected zero explicit inputs. TC-1947 now executes `prepare` and dependent `consume` members through `run_campaign`, with authored `inputOrigins` for a tracked source file and the prior member's output artifact. It independently replays an accepted run, checks that the retained declaration is `enforced` and both origin kinds are present, changes the dependency artifact role in the collection, rehashes the collection and run attempt, and requires `verify_retained_campaign` to reject. The matrix has a tagged TC-1947 row. The campaign owner reports its Linux Docker run passed (1 test). This closes the requested complete-path coverage gap; no separate rehashed source-file substitution is present in TC-1947, though the focused origin unit test covers that refusal.

## Coverage

Rust 1.98.1 with `TMPDIR=/private/tmp`: `cargo fmt --all -- --check` passed; `cargo clippy --offline -p quoin-measurement --all-targets --features campaign -- -D warnings` passed at 3a3c754; three focused origin tests and the one macOS feature-gated non-TL fixture passed. TC-1947 is Linux-gated and yields zero tests on macOS. The campaign owner's offline Linux Docker runs passed TC-1942 (2 tests) and TC-1947 (1 test). The full crate test command in this macOS sandbox reached a permission failure in `tc_975_006_an_unreadable_protected_file_is_refused`; the campaign owner separately reports the full Quoin measurement suite passed with normal filesystem permissions. Full CLI package auth and TL `cave` Campaign execution remain pending.

The source-file origin check deliberately compares path and byte digest; `executable` describes the chosen staging mode and is bound to the EA request. EA's authored `inputOrigins` schema does not declare Git mode, so a mode difference is not a source-origin contradiction under this contract. Dependency-artifact replay checks role, digest, retained bytes and `byteLength`; the common raw-artifact replay also checks the complete unique artifact population, including output-tree leaf roles. Optional formal semantic gap review was not invoked; the findings above arose from the requested code review.
