<!--
SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Agent-IX
-->

# Goldens

`rust/goldens/` is the one home for corpora that Rust tests read as data.

## Layout

```text
rust/goldens/<crate>/<name>.json     the corpus
rust/goldens/<crate>/PROVENANCE.md   where it came from
```

`<crate>` is the directory name of a crate under `rust/crates/`. A crate's tests
reach its corpus through `CARGO_MANIFEST_DIR/../../goldens/<crate>/`. No crate
keeps a `golden` or `goldens` directory of its own.

Hand-written test inputs that only one test file reads stay in the crate's
`tests/fixtures/`. A file belongs here when it is a corpus: a set of cases that
several tests, or a test in another crate, read.

## Provenance

Every `PROVENANCE.md` has three sections, in this order:

- `## Origin` states what the corpus holds and how it was produced
  (captured from the retired TypeScript, or hand-authored), and whether it
  stores recorded outputs or inputs only.
- `## Captured at` names the commit and ticket that introduced it.
- `## Divergences` links the crate's `DIVERGENCE.md` where Rust and the
  retired TypeScript differ, or states `None`.

The TypeScript oracle these corpora were captured against has been retired, so
there is no source hash left to compare. The corpus is the record.

`quoin-core/tests/tc_127_goldens_root.rs` enforces this layout.
