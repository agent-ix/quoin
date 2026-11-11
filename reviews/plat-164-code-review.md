---
id: SR-20261103118171
title: "Code review of quoin PR 802 (truthful build profiles in MeasurementCollection)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@589a073; rust/crates/quoin-measurement, rust/crates/quoin-measurement-graph, spec/functional/FR-044, spec/functional/FR-108, spec/matrix.md"
review_set: subset
---

# Code review of quoin PR 802

## Summary

Ticket: PLAT-164. Scoped to `git diff origin/main...pr-802` at 589a073 (21 files, two
commits). Rust lane (`rust-review`, with rust-style) folded in. Tools: quoin 0.30.0.

Checks run:

- `cargo test -p quoin-measurement -p quoin-measurement-graph` (default features): 34
  test binaries, all pass. `cargo fmt --check` is clean for both crates.
- `cargo clippy --workspace --all-targets -- -D warnings` fails to compile `quoin-measurement`
  (see FND-001), because `quoin-cli` turns on the `campaign` feature.
- `cargo check --all-targets -p quoin-jev` fails (see FND-002).
- Every consumer of `BuildProfile`, `buildProfile` and `ReleaseProfile` was grepped across
  `rust/`. Stored `release`, `debug` and absent profiles still decode, because the
  `validate/stack.rs` parser only gained `undeclared`. The fixtures that state `release` pass,
  and `tc_479_004` still reads a historical record that has no profile. The graph crate's
  attestation used to refuse `debug`. It now accepts it, as AC-15 intends.
- Callers of the comparison: `portfolio/build.rs` is the only production caller and passes
  its active plans. `quoin report` does not compute comparisons.
- No ceremony was added: no digest, pin or identity binding. The diff only prose-edits the
  existing AC-13 "nothing is hashed" rule.
- The engineering-assurance MeasurementPlan schema (origin/main
  `engineering_assurance/schemas/measurement-plan-frontmatter.schema.json`) is
  `additionalProperties: false` and has no `cross_profile_comparison`. This was reproduced
  with `quire validate` on a copy of MP-201 with the key added: "Additional properties are not
  allowed ('cross_profile_comparison' was unexpected)". See FND-003.

## Verdict

**FAIL (two high, one medium, two low).** The design is sound and matches FR-044-AC-14/15:
the profile is kept as stated, `undeclared` replaces the false `release`, and the reason codes
block or stratify correctly. But the PR breaks the workspace build. The `campaign` module
(which builds the `quoin` binary through quoin-cli) imports a path that does not exist, and a
quoin-jev test constructs `MeasurementPlan` without the new field. The PR's "clippy clean"
claim covers only the default features of one crate. The plan opt-in also cannot be used in
any validated repository until engineering-assurance adds the key to its schema.

The PR body is stale: it says the graph adapter stays release-only and that cross-profile
refusal is not done, but the second commit does both. Transition note, not a defect: campaign
collections already stored say `release`, and the next campaign run says `undeclared`. That
first portfolio pair will therefore be `incomparable` with `profile_changed`, which is what
AC-15 prescribes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `use crate::types::BuildProfile;` does not resolve, because `types` does not re-export it; it lives at `crate::types::collection::BuildProfile`. The `campaign` feature fails with E0432. quoin-cli enables that feature, so the `quoin` binary and `cargo clippy --workspace` do not build | rust/crates/quoin-measurement/src/campaign/run/collection.rs:14 |
| FND-002 | high | `MeasurementPlan` gained the public field `cross_profile_comparison`, but the struct literal in quoin-jev's test was not updated, giving E0063 "missing field `cross_profile_comparison`". The quoin-jev test target does not compile | rust/crates/quoin-jev/tests/tc_1016_constant_predictor.rs:141 |
| FND-003 | medium | engineering-assurance's MeasurementPlan frontmatter schema is `additionalProperties: false` and has no `cross_profile_comparison`. Any plan that opts in fails `quire validate`, so the AC-15 permission path cannot be used. EA needs a matching boolean property, landed together with this PR | rust/crates/quoin-measurement/src/plans/design.rs:247 |
| FND-004 | low | The plan-less public `compare_measurement_collections` is kept as a wrapper that passes `&[]`. It now always reports a cross-profile pair as blocking, even when the governing plan permits it. Only tests call it. Collapse it into one function that takes the plans | rust/crates/quoin-measurement/src/compare.rs:55 |
| FND-005 | low | The graph crate declares its own `BuildProfile` copy of quoin-measurement's enum, and its test hardcodes the three spellings. AC-15 requires that the attestation "accepts every profile intake accepts", but a profile added to intake would not fail this test. Drive the test from `quoin_measurement::types::collection::BuildProfile::ALL`, or decode through that type | rust/crates/quoin-measurement-graph/src/wire.rs:151 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-044-AC-14, path: spec/functional/FR-044-plan-governed-measurements.md, role: examined, excerpt: "A new collection states verificationStack.buildProfile as one of debug, release or undeclared, and intake records the stated profile exactly"}
- {id: FR-044-AC-15, path: spec/functional/FR-044-plan-governed-measurements.md, role: examined, excerpt: "Comparing two collections whose verificationStack.buildProfile differs ... is incomparable with a blocking profile_changed reason ... unless the governing plan states cross_profile_comparison: true"}
- {id: FR-108-AC-22, path: spec/functional/FR-108-measurement-verdict-checker.md, role: examined, excerpt: "Intake admits any stated verificationStack.buildProfile (FR-044-AC-14)"}
- {id: build-profile-consumers, path: rust/, role: examined, excerpt: "every BuildProfile/buildProfile/ReleaseProfile consumer grepped; stored release/debug/absent decode"}
- {id: ceremony, path: diff, role: examined, excerpt: "no digest, pin or identity binding added"}
```
