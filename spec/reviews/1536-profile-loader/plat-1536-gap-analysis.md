---
id: SR-20261113152319
title: "Gap analysis — PLAT-1536 profile evaluate against FR-180, FR-181, FR-182, NFR-037 (quoin PR 875)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-180-source-profile-selection.md, spec/functional/FR-181-source-profile-evaluation.md, spec/functional/FR-182-local-pack-joins-a-profile.md, spec/non-functional/NFR-037-source-profile-evaluation-is-deterministic-and-states-only.md, rust/crates/quoin-cli/src/profile/, rust/crates/quoin-cli/tests/tc_1536_profile_evaluate.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-180", type: references}
  - {target: "ix://agent-ix/quoin/FR-181", type: references}
  - {target: "ix://agent-ix/quoin/FR-182", type: references}
  - {target: "ix://agent-ix/quoin/NFR-037", type: references}
---

## Summary

Ticket: PLAT-1536. PR head `a550f49a`. Plan completion: not assessed. I ran `quire trace --id`
(quire 0.44.0, engine 0.59.1) for all 34 criteria rather than `quoin matrix`, because of PLAT-1672.
33 criteria have at least one `verifies` claim from a `#[trace]` tag. **FR-181-AC-10 has none.**
For each sampled criterion (FR-180-AC-4/5/8, FR-181-AC-4/7/9, FR-182-AC-7, plus the rest) I read
the bound tests and checked that their assertions can fail for the stated behaviour.
`tc_1536_profile_evaluate` passes (21) through the built binary, as do the unit tests.

On the coder's three spec notes:

1. Finding profiles through `catalog.load` follows FR-006. Module names are first-wins, so a
   shadowed same-name module is not installed in effect. The semantic read runs as it does for every
   catalog consumer. Neither is a defect.
2. With no selection, the output is plain text and exit 0. FR-180-AC-7 literally permits this, but
   FR-181's Outputs say otherwise (FND-004, a spec ambiguity).
3. NFR-037-AC-2 is weakly tested (FND-003).

## Verdict

The implementation covers the criteria. The gaps are one untagged criterion (FND-001), one
criterion whose test misses the bypass found in code review (FND-002), and weaker evidence for
NFR-037-AC-2 and FR-181-AC-11. FND-001 and FND-002 should be fixed in this PR. FND-004 is for the
spec owner.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-181-AC-10 has no trace claim: `quire trace --id FR-181-AC-10` returns 0. Its behaviour is asserted in `a_selected_profile_is_evaluated_with_the_effective_context`: `us` is admitted by alpha (alpha-3 not_binding), beta-3 is unresolved, and the context is unrewritten. The tag is missing and only a code comment names the criterion, so the computed matrix shows it unverified. Fix: add `"FR-181-AC-10"` to that test's `#[trace]`. | rust/crates/quoin-cli/tests/tc_1536_profile_evaluate.rs:158-167 |
| FND-002 | medium | The FR-180-AC-10 tests check only a repository `profiles/only-here.yaml` selected by a bare name. They do not cover a selection whose name is absolute or contains `..`, which reads a repository file today (SR-20261113152318 FND-003). Fix: add both forms to the AC-10 test, expecting refusal. | rust/crates/quoin-cli/tests/tc_1536_profile_evaluate.rs:372-386; rust/crates/quoin-cli/src/profile/model/tests.rs:192 |
| FND-003 | low | NFR-037-AC-2 ("networking disabled gives the same output") is tested by setting `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY` to an unreachable address. A client that ignores proxy variables, or a direct socket, would pass. Reading the code, there is no network path in `profile evaluate` (catalog.load reads local roots, the engine is in-process), so this is about evidence strength. Fix: run the offline leg under `unshare -rn` (available on this host) and skip with a stated reason where user namespaces are absent. | rust/crates/quoin-cli/tests/tc_1536_profile_evaluate.rs:430-460 |
| FND-004 | low | Spec ambiguity. FR-180-AC-7 says the no-selection "result states that no profile is selected", and the PR prints a plain sentence with exit 0. FR-181's Outputs say "One `source-profile-evaluation-v1` document on stdout", and that closed shape requires a `profile` name, so it cannot express "none selected". A machine consumer that parses stdout as JSON fails on this path. The text satisfies AC-7 as written. The spec owner should state which form the no-selection output takes (text, or a v1 variant), and the code then follows it. | spec/functional/FR-180-source-profile-selection.md; spec/functional/FR-181-source-profile-evaluation.md; rust/crates/quoin-cli/src/profile/mod.rs:128-132 |
| FND-005 | low | FR-181-AC-11 asks for hashes of every module root, every selection file and `spec/`. The test hashes `filament/`, the project `.ix/` and `spec/`, but no user selection file exists in that run, so the user layer's file is not shown unmodified. Fix: write a user `source-profile.yaml` in that test and include it in the snapshot. | rust/crates/quoin-cli/tests/tc_1536_profile_evaluate.rs:430-460 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-180-AC-1..3, AC-6, AC-9, AC-11, AC-12 | examined | Tagged and asserted (unit and binary) |
| FR-180-AC-4, AC-5 | examined | Tagged; the binary tests assert non-zero exit and the names |
| FR-180-AC-7 | examined | Tagged; FND-004 |
| FR-180-AC-8 | examined | Tagged three times; seven FR-020 rules, file and rule named, selected by name through the binary |
| FR-180-AC-10 | examined | FND-002 |
| FR-181-AC-1..6, AC-8, AC-9 | examined | Tagged; AC-5/AC-8 use a fake engine; AC-9 closed shape tested for extra and missing fields |
| FR-181-AC-4 | examined | Binary test with two real modules declaring alpha/beta |
| FR-181-AC-7 | examined | The out-dir file is byte-equal to the in-process engine report, and `quoin discharge --binding` accepts it |
| FR-181-AC-10 | examined | FND-001 |
| FR-181-AC-11 | examined | FND-005 |
| FR-182-AC-1..8 | examined | Tagged; AC-7 through the binary with a synthetic restricted pack; AC-5 asserts the sentinel and no text/title |
| NFR-037-AC-1, AC-3 | examined | Byte-identical stdout and out-dir; key-name scan and closed shape |
| NFR-037-AC-2 | examined | FND-003 |

## Dispositions

Round 1 is checked against fix commits `c0079da7` and `5244020b` (head `5244020b`). I re-ran
`quire trace --id`:

- FR-181-AC-10 now has 1 claim.
- FR-180-AC-10 has 3.
- NFR-037-AC-2 has 2, including `a_run_with_no_network_namespace_gives_the_same_output`, which ran
  rather than skipped on this host.
- FR-181-AC-11 has 1.

tc_1536 passed 24/24 at this head.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
