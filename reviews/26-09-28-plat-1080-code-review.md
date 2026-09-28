---
id: SR-174
title: "Code review — PLAT-1080 evidence-backed test matrix (matrix.build, quoin matrix)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@1cb9159127bb7588f7960794905ff208cbc9389a; PR 650 diff vs merge base e8679e9: Makefile, rust/Cargo.lock, rust/Cargo.toml, rust/deny.toml, rust/crates/quoin-assurance/{Cargo.toml,src/lib.rs,src/matrix.rs,tests/tc_1080_matrix.rs}, rust/crates/quoin-cli/{Cargo.toml,src/evidence.rs,src/evidence/audit.rs,src/flow.rs,src/help.rs,src/main.rs,src/matrix.rs,tests/fixtures/retained-command-help.json,tests/tc_1080_matrix_command.rs}, rust/crates/quoin-core/{src/dispatch.rs,src/ops/matrix.rs,src/ops/mod.rs,src/ops/quire/mod.rs,src/ops/quire/tests.rs,src/ops/quire/wire.rs,tests/tc_1080_matrix_boundary.rs}, rust/crates/quoin-quire-types/src/lib.rs, rust/crates/quoin-quire/{Cargo.toml,src/lib.rs,tests/fixtures/coverage-volume/PROVENANCE.md,tests/manifest_pin.rs}"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-115"
    type: "reviews"
---

# SR-174: Code review — PLAT-1080 evidence-backed test matrix

## Summary

Ticket: PLAT-1080. PR agent-ix/quoin#650 at `1cb9159`. This review covers `code-review` with the
`rust-review` lane folded in. The repo's `.claude/skills/rust-style/SKILL.md` was applied as the
higher authority.

`quoin_assurance::matrix::build` is a pure join. It checks for a contradictory audit first, then
indexes findings, unevaluated checks and bindings by obligation, then regroups by `requirement_of`.
The status mapping reads the first finding of a detail list sorted by bucket. That is correct
because the sort is by `(bucket, full FindingDetail Ord)`. The `quoin-core` shell matches
`assurance.build_case`. It has a 16 MiB byte ceiling with `CORE_REFUSED`, `deny_unknown_fields` on
`MatrixInput`, `CORE_REFUSED` with `context.reason = contradictory-audit`, and no capabilities.
`quoin matrix` refuses an empty HEAD before `assemble` runs. It reuses `evidence::audit::assemble`
and forwards an absent `coverage_matrix` as absent.

Measured:

- **Gate.** `make test` exited 0 at `1cb9159` (fmt, clippy `-D warnings`, `cargo deny` with
  advisories, bans, licenses and sources all ok, and 211 test binaries ok).
- **Mutation.** 14 mutants were run and all 14 were killed:
  - status map swapped;
  - suspect and stale buckets swapped;
  - `vacuous-evidence` dropped from the suspect kinds;
  - contradiction check disabled;
  - contradiction check ignoring unevaluated;
  - contradiction check ignoring findings;
  - `.min()` replaced by first match;
  - detail sort without buckets;
  - reversed tie-break;
  - bindings unsorted;
  - unevaluated unsorted;
  - `healthy` ignored;
  - empty-store override off;
  - CLI HEAD refusal off, killed by `tc_1080_204`.
- **`quoin evidence audit` byte-identity.** Main `e8679e9` and the branch were compared on a real
  fixture store: a passing run at HEAD with a mock inspection, a run behind HEAD, and a failing
  latest run.
  - stdout, stderr and exit code were identical for `""`, `--json`, `--strict`, `--ratchet` and
    `--ratchet --json`.
  - They were also identical for malformed-baseline, bad-policy, bad-mutation-floor and no-git cases.
  - Only one case differs, a double fault. See FND-002.
- **PLAT-1086 rung.** The failing latest run renders `stale` in `quoin matrix`, with the auditor's
  high-severity "bound to a failing run" summary in the detail.
- **quire-rs repin.**
  - `agent-ix-semantic-schema` comes from filament-core-data, which is PUBLIC. quire-rs's own
    manifest pins it by `tag = "semantic-schema-v0.1.0"`. The tag resolves to `dc6e85b`, which is
    the commit `Cargo.lock` records, and the `--locked` builds hold it.
  - `allow-git` is URL-scoped, which is the narrowest scope cargo-deny offers. The addition is
    justified and minimal.
  - A clean `cargo update -p quire-rs` from main reproduces the branch `Cargo.lock` byte for byte.
    The lockfile holds no hand edits.
- **`flow::FLOWS`** now holds only `review` and `to-plan`. The dispatch match is narrowed, and no
  dead matrix-flow code remains.
- **Repin cleanup.** The removal of the literal-SHA assertion in `tc_379_026` and the PROVENANCE
  de-duplication are consistent with the pins rule.

## Verdict

**CONDITIONAL**. There are two low findings and neither blocks. The code is correct, and the tests
that back it are strong under mutation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The repin moves errno, rustix, tempfile, quinn-udp, winapi-util and equivalent onto windows-sys 0.52.0 (cargo's own resolution, reproduced from main). deny.toml's windows-sys@0.52.0 skip reason and the path comment above it still say 0.52.0 is "reachable only via ring" and 0.61.2 "via errno/rustix". Both statements are now false, so the stated removal condition misleads whoever next retires the ring edge. | rust/deny.toml:286-306, rust/Cargo.lock |
| FND-002 | low | The `assemble` extraction moved the `--ratchet` baseline read ahead of coverage, the independence policy and the store read. With a broken baseline.json AND a broken bindings.json, main reports `evidence.audit_inputs` CORE_REFUSED (bindings) and the branch reports `evidence.read_baseline` CORE_REFUSED (baseline). The exit code is the same (1). Every single-fault and success case is byte-identical. | rust/crates/quoin-cli/src/evidence/audit.rs:128-136 |

## Dispositions

Round 1, reviewed at `5d5cbd9c1a22224415e36f7d934d7356129be4d8`. `make test` exited 0 at this head: fmt, clippy, and deny with advisories, bans, licenses and sources all ok, plus 211 test binaries ok.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | dabb8e7: the deny.toml path comment, skip reason and REMOVAL CONDITION now list errno, rustix, tempfile, winapi-util and rustls-platform-verifier as reaching windows-sys 0.52.0 since the quire-rs 0.48.0 re-resolve. They also give a `cargo tree -i` retirement test for 0.52.0 that is separate from reqwest 0.12. |
| FND-002 | accepted-no-change | The leader accepted the baseline read reorder as intended. The only observable difference is which of two simultaneous input errors (a broken baseline.json and a broken bindings.json under --ratchet) is reported. The exit code is unchanged, and every single-fault and success case is byte-identical to main. |

New in this round, and clean: `tc_1080_010` (FR-115-CON-1, TC-1979) runs engineering-assurance `source_audit` over `src/matrix.rs`. Five mutants were injected into `build` and all five were killed: `std::fs::read`, `std::process::Command`, `std::net::TcpStream::connect`, a `use std::fs::File` import, and `std::env::var`. The new dev-dependency is `engineering-assurance = { workspace = true, features = ["source-audit"] }`. It uses the workspace's exact `=0.5.0` rev pin, and its git source is already in `allow-git`. This is the same declaration quoin-core and quoin-config use. The Cargo.lock delta is one dependency edge, and the gate's `cargo deny` passes.
