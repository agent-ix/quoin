<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Engineering Assurance adoption (quoin#373 AC-9)

quoin's Rust workspace consumes `agent-ix/engineering-assurance` (EA) rather
than regrowing assurance capability locally. The governing policy is
quire-research `implementation-language-policy.md`, **"Shared tooling is
consumed, not regrown"**: a repository needing assurance capability CONSUMES
EA; where EA does not fit, the response is a **gap ticket in EA**, never a local
harness.

**The one standing exception** is EA's own migration contract, which assigns the
evidence **store** to Quoin. That edge points from EA to Quoin and is not
inverted here.

This document is the record required by that policy: the
retention test for everything quoin keeps local, and the gaps filed against EA.

## How the dependency is consumed

A **git rev pin**, which is the pattern `agent-ix/quire-cli` already proves
against `quire-rs` (`quire-cli/Cargo.toml:20`), not an invented one. EA is
`publish = false` and quoin#373 rules out crates.io, so a registry version is
not available to either repository. `rust/deny.toml` names the URL under
`allow-git`; `unknown-git = "deny"` refuses anything else.

## Retention test

Recorded for everything quoin keeps local that EA also offers. Three parts,
each answered, with reasons — including where the answer is "no".

### 1. `rust/crates/quoin-core/tests/tc_source_conventions.rs:46` — SPDX headers, vs EA `content_rights`

- **Must it be local?** **No.** It is a two-line `head.contains(...)` on each
  file; EA's `content_rights::inspect_content` classifies the same thing over
  caller-supplied paths and bytes with a closed category set.
- **Is it Rust?** Yes, and so is EA's.
- **Should it be common?** **Yes.** Every repository in the ecosystem asserts
  its own SPDX header the same way, and each has written the check again.
- **Disposition: retire in favour of EA.** Not done in this change because the
  `full`-feature blocker (below) makes the import cost disproportionate to a
  two-line check. Tracked, not conceded.

## Known blockers, filed against EA

These are why adoption is one test and not ten. Each is an EA ticket, never a
local copy.

| #   | EA issue                           |
| --- | ---------------------------------- |
| 1   | agent-ix/engineering-assurance#99  |
| 2   | agent-ix/engineering-assurance#100 |
| 4   | agent-ix/engineering-assurance#102 |
| 5   | agent-ix/engineering-assurance#103 |

1. **17 of 19 public modules sit behind a single `full` feature**, and `full`
   pulls `cap-std`, `clap`, `flate2`, `jsonschema`, `regex`, `syn`, `tar`,
   `time`, `unicode-casefold`, `yaml_serde` and `zip`. A consumer that wants
   only `evidence` — or, as here, only `source_audit` — imports a CLI's entire
   dependency tree. EA needs per-capability features.

2. **`--no-default-features` and `--no-default-features --features
producer-execution` do not compile**. `pub mod evaluation;` is
   unconditional in `src/lib.rs` while `src/evaluation.rs` imports `time`,
   `serde`, `serde_json` and `thiserror`, all `full`-only optionals. `full` is
   the only feature set that builds. This is the defect EA#78 closed; it stands
   again at `origin/main`.

4. **Exact `=` pins make EA and every consumer mutually unsatisfiable** until
   one side moves. quoin moved up, which is
   the right direction, but it is a manual step for every consumer on every EA
   bump.

5. **`source_audit`'s `RequirementTests` role assumes `ix-trace-rs`
   `#[trace(...)]` attributes.** quoin's convention, stated in
   `.claude/skills/rust-style/SKILL.md`, is a `/// Trace:` doc line with
   comma-separated criteria. Measured over quoin's three integration-test files:
   **13 findings, 13 false positives, 0 true positives** — one
   `TraceImportMissing` per file and one `TestTraceMissing` per test, on tests
   that every one of them carries a `/// Trace:` line. The half of
   `source_audit` quoin most needs is unreachable.
