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
- **Disposition: retire in favour of EA.**

## Known blockers, filed against EA

Each is an EA ticket, never a local copy.

| #   | EA issue                           |
| --- | ---------------------------------- |
| 1   | agent-ix/engineering-assurance#99  |
| 2   | agent-ix/engineering-assurance#100 |
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
   the only feature set that builds.

5. **`source_audit`'s `RequirementTests` role assumes `ix-trace-rs`
   `#[trace(...)]` attributes.** quoin's convention, stated in
   `.claude/skills/rust-style/SKILL.md`, is a `/// Trace:` doc line with
   comma-separated criteria. Measured over quoin's three integration-test files:
   **13 findings, 13 false positives, 0 true positives** — one
   `TraceImportMissing` per file and one `TestTraceMissing` per test, on tests
   that every one of them carries a `/// Trace:` line. The half of
   `source_audit` quoin most needs is unreachable.
