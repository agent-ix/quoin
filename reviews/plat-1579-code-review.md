---
id: SR-20261113203414
title: "Code review (Rust lane) — PLAT-1579 per-class evidence floors (quoin PR 880)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/Cargo.lock, rust/Cargo.toml, rust/crates/quoin-quire/Cargo.toml, rust/crates/quoin-quire-types/{Cargo.toml,src/lib.rs}, rust/crates/quoin-auditor/src/{audit/class.rs,audit/input.rs,audit/ladder.rs,audit/mod.rs,audit/scores.rs,error.rs}, rust/crates/quoin-evidence/src/independence.rs, rust/crates/quoin-finding-types/src/finding.rs, rust/crates/quoin-core/src/ops/{auditor/taxonomy.rs,quire/mod.rs,quire/wire.rs,quire/tests.rs}, rust/crates/quoin-assurance/src/{lib.rs,matrix.rs}, rust/crates/quoin-cli/src/evidence/audit.rs, tests tc_160_class_floors.rs, tc_1080_matrix.rs, tc_160_class_policy_command.rs"
review_set: subset
---

## Summary

Ticket: PLAT-1579. PR head `46d853e4`, base `63acd34f`, diff `git diff origin/main...HEAD` (24
files). The Rust lane (quoin `.claude/skills/rust-style`, then the ecosystem `rust-style`/`rust-review`)
is folded in. Checks run on this head in a detached scratch worktree:
`cargo test --locked -p quoin-auditor -p quoin-assurance -p quoin-cli --test tc_160_class_floors
--test tc_1080_matrix --test tc_160_class_policy_command` (34 passed, 0 failed);
`cargo clippy --locked` on the 8 touched crates with `--all-targets --all-features -- -D warnings`
(clean); `cargo fmt --all --check` (clean). A targeted `cargo mutants` run over `independence_failed`,
`floor_for`, `shortfall`, `evidence_findings` and `mutation_floor` is recorded under the gap-analysis
review (SR-20261113203415).

**quire-rs 0.59.0 -> 0.60.0.** The bump is needed: `carried` on `coverage_matrix` entries and the
top-level `carried_vocabulary` do not exist at `51f81dd` and do exist at `5a45ec9` (quire-rs
PLAT-1578/PLAT-1526, released as 0.60.0). The manifest change is correct: the `=0.60.0` version
and the rev agree, and `quire-fixtures` moves to the same rev. The engine moves by about 15 PRs.
Among them, PLAT-1597 (criticality carried onto obligations) is opt-in through a module's
`criticality_field`, so FR-160's criticality axis is unchanged by default. The whole-workspace gate
has not been run on this bump. It is due once, on the final head, before merge.

**The lockfile moved more than the bump.** See FND-001: I reproduced the minimal resolution.

The class code does what FR-160 says. The class is read only from `coverage_matrix[].carried`
under the policy's `class_field`. Quire drops null values, and a non-string value is kept as its
JSON text. Nothing derives a class from criticality, from `item_class` or from evidence, and
nothing defaults one. An obligation with no stated class gets no floor and is listed in
`classReport.unassigned`. The join is by criterion id under each document's entry. quire puts every
obligation of a document in that entry's `criteria`, so this is the requirement join FR-160 states.
The policy is refused whole, before anything is audited, with every cause joined into one
`QAU-CLASS-POLICY-REFUSED` (mapped to `Refused`). `floor_for` takes the higher ratio and names its
source through `FloorSource`, and a tie goes to criticality. Profile and class independence are
assessed separately on their own dimensions through the shared `assess_lines`, and each failure is
its own finding with `source`. Without `--class-policy`, `ClassContext::prepare` returns `None` and
the report is byte-for-byte what it was before, apart from the profile finding, which gains
`source: profile` only when a policy is in force.

No compatibility layer, no shim, no vendored file, no pin or digest ceremony, and no standard text
is named or quoted (AC-13 by inspection: class values and the `class` field name appear only in
tests). Rust lane: no `unwrap`/`expect`/indexing outside tests, `#![allow]`s carry reasons, the new
error code is in `ALL` and in the explicit core mapping test, `requirement_of` was moved (not
copied) to `quoin-quire-types` and re-exported, and no module is near the 700-line census.

## Verdict

The production logic is correct. Merge after FND-001: the lockfile should carry only the quire-rs
move. FND-002 and FND-003 are optional.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `rust/Cargo.lock` moves dependencies the bump does not need. `agent-ix-semantic-schema` goes from filament-core-data `ce72bc5` to `327a49c` (semantic-schema 0.6.0: manifest `where`/`record_validation` keys, PLAT-1482). `windows-sys` is downgraded from 0.61.2 to 0.52.0 for six crates (errno, quinn-udp, rustix, rustls-platform-verifier, tempfile, winapi-util) and `hashbrown` from 0.17.1 to 0.16.1 under gix-imara-diff. Reproduced: with main's lock and this PR's manifests, `cargo metadata` resolves only the three quire-rs packages (`quire-rs`, `quire-fixtures`, `quire-code-extraction`). `cargo check -p quoin-quire` builds against the old semantic-schema rev, and the CLI test uses its own fixture module, so nothing in this PR needs the other moves. Downgrades and a schema-crate move riding in unreviewed is churn. Fix: `git checkout origin/main -- rust/Cargo.lock`, then let cargo re-resolve (any `cargo metadata`/build) and commit the 3-package diff. | rust/Cargo.lock:22,595,1259,2937,3880,3938,4465,5222 |
| FND-002 | low | Avoidable clones on the audit path. `audit` clones the whole obligation vector into `classified` even when no class policy is given, and only uses it for `classReport`. `ClassContext::independence` clones every bound `Binding` per obligation into `owned`, and `assess_lines` then `to_vec`s it again. Fix: build `classified` only when `indexed.class.is_some()`, or collect ids as the loop runs. Pass `&[&Binding]` through, or let `assess_lines` take an iterator of `&Binding`. | rust/crates/quoin-auditor/src/audit/mod.rs:173; rust/crates/quoin-auditor/src/audit/class.rs:301 |
| FND-003 | low | `rust/crates/quoin-quire-types/Cargo.toml` leaves a double blank line where `[dev-dependencies]` was removed. This is cosmetic. | rust/crates/quoin-quire-types/Cargo.toml:30 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| quire-rs pin `=0.60.0` / rev `5a45ec9` (quoin-quire, workspace quire-fixtures) | examined | Needed for `carried`/`carried_vocabulary`; correct |
| rust/Cargo.lock | examined | FND-001 |
| `ClassContext::prepare` (validation, refusal, class join) | examined | Correct; no derived or default class |
| `ClassContext::evidence_findings` (class-unconfigured, class-method-missing, run+scan kind join) | examined | Correct |
| `ClassContext::independence`, `independence_finding`, `shortfall` | examined | Correct; FND-002 on clones |
| `ladder::independence_failed` | examined | Profile then class, each its own finding, both may fail |
| `scores::floor_for` / `Floor` / `FloorSource` / `mutation_finding` | examined | Higher ratio wins, source named, tie to criticality |
| `audit()` classReport/classIndependence assembly | examined | Present only under a policy; FND-002 |
| `quoin_evidence::independence::assess_lines` refactor | examined | Behaviour-preserving extraction |
| `AuditorErrorCode::ClassPolicyRefused`, core `core_code` mapping | examined | In `ALL`, mapped to Refused, tested |
| `FindingKind::CLASS_METHOD_MISSING` / `CLASS_UNCONFIGURED` | examined | Added to `ALL`; matrix buckets them as "other" |
| quoin-core `coverage` carried/carried_vocabulary passthrough | examined | Verbatim, omitted when empty |
| quoin-assurance matrix class column / unclassified count / markdown | examined | Switch is `classReport`'s presence |
| quoin-cli `--class-policy`, `add_class_inputs`, JSON/text rendering | examined | YAML via no-alias reader; refusal left to the auditor |
| `requirement_of` move to quoin-quire-types | examined | Moved, not copied; re-exported |


## Disposition Round 1 Evidence

Replacement reviewer: `gpt-6-astra`; run `9118bc4b-9e8a-4c1f-9f5b-4d09820fcc63`; Codex session
`01a1d6e9-d820-7821-83d5-1b05bd42a929`. Sole reviewer for this disposition.
Ticket: PLAT-1579; PR: agent-ix/quoin#880. Reviewed pushed head
`fb4c9ab7ab65bdbcc5bcf0b97150b99d600d0a1b` and fixes `f2f627dd` and `fb4c9ab7`.
Final diff: `git diff origin/main...HEAD`, merge base `63acd34f8a571f4f51541433df86757360670b30`;
verified origin/main `2975c419cfefdf38745386e3b72c3bb63c3c0613` against the remote.
All 27 changed files and the three committed review artifacts were inspected.

Current outcome: **3 fixed; no open findings in this artifact.** No new findings. Original findings and prior review text above
are retained verbatim; the following rows are this round's current dispositions.

Verification: source, test-assertion, spec and pushed-commit inspection; `git diff --check
origin/main...HEAD` passed. No Cargo build/test, mutation run, computed matrix rerun or aggregate
gate ran in this pass. Earlier test/mutation claims above remain historical, not results for
this head. The aggregate gate belongs to the code lead. Tools: quoin `0.30.1`, quire `0.44.0`
(engine `0.59.1`), spec-artifacts-process `v0.31.0`. Validation uses this artifact's explicit
scratch path with `quire validate --scope .`.

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) applies to the changed deterministic audit
and matrix report views. Inspected population preservation, source provenance, class/profile
separation, and optional-output compatibility in the changed auditor, assurance and CLI paths
and the core boundary. The remaining source-provenance loss is dispositioned once in
SR-20261113203415 FND-007. AP-202 concerns live Jev lens evaluation and does not apply.
No new measurement, impact, exception, structural-gap or producer-reliance decision is supplied
for this disposition; no green measurement/gate or human acceptance is inferred. This is the
requested disposition of existing artifacts, not a new enforced spec-review run.

## Dispositions

### Round 1

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | f2f627dd restores the unrelated semantic-schema revision and transitive Windows/hashbrown resolutions. The final origin/main...HEAD lock diff changes only quire-code-extraction, quire-fixtures and quire-rs; both manifests agree on 5a45ec9 / quire-rs 0.60.0. |
| FND-002 | fixed | f2f627dd removes the unconditional obligation-vector clone and the per-obligation Binding deep clones. fb4c9ab7 factors lazy class-report assembly into this helper. class.rs:302 passes borrowed bindings to assess_lines; independence.rs:179-183 accepts &[&Binding] and sorts only references. |
| FND-003 | fixed | f2f627dd removes the extra blank line; exactly one separates schemars from [features]. |

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  reason: f2f627dd restores the unrelated semantic-schema revision and transitive Windows/hashbrown resolutions.
    The final origin/main...HEAD lock diff changes only quire-code-extraction, quire-fixtures and quire-rs; both
    manifests agree on 5a45ec9 / quire-rs 0.60.0.
  after_path: rust/Cargo.lock
  after_lines: 20-22
  after_excerpt: |-
    name = "agent-ix-semantic-schema"
    version = "0.3.0"
    source = "git+https://github.com/agent-ix/filament-core-data?branch=main#ce72bc519945ca4676f1b8f1e19f44f44303513d"
- fnd: FND-002
  outcome: fixed
  reason: f2f627dd removes the unconditional obligation-vector clone and the per-obligation Binding deep clones.
    fb4c9ab7 factors lazy class-report assembly into this helper. class.rs:302 passes borrowed bindings to assess_lines;
    independence.rs:179-183 accepts &[&Binding] and sorts only references.
  after_path: rust/crates/quoin-auditor/src/audit/mod.rs
  after_lines: 294-303
  after_excerpt: |
    fn class_report(
        indexed: &Indexed<'_>,
        obligations: &[&quoin_quire_types::Obligation],
    ) -> Option<serde_json::Value> {
        indexed
            .class
            .as_ref()
            .map(|class| class.report(obligations))
    }
- fnd: FND-003
  outcome: fixed
  reason: f2f627dd removes the extra blank line; exactly one separates schemars from [features].
  after_path: rust/crates/quoin-quire-types/Cargo.toml
  after_lines: 29-31
  after_excerpt: |-
    schemars = { workspace = true, optional = true }

    [features]
```
