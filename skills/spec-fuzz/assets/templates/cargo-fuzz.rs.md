# cargo-fuzz target template

One file per obligation, at `fuzz/fuzz_targets/fuzz_<obligation-slug>_<surface>.rs`, plus a
`[[bin]]` block in `fuzz/Cargo.toml`.

**The `[[bin]]` block is not optional.** cargo-fuzz discovers targets from `Cargo.toml`, not
from the directory — a `.rs` file with no block is a file nobody runs, which is the same
"green over nothing" shape this whole program exists to catch.

## Bytes in, no interpretation

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

use quire_rs::loader::manifest::parse_manifest;

/// The manifest loader does not panic on arbitrary input.
///
/// Trace: NFR-003-M-1
///
/// spec-fuzz: obligation=NFR-003-M-1 harness=cargo-fuzz entry=loader::manifest::parse_manifest origin=advised
fuzz_target!(|data: &[u8]| {
    // `from_utf8` rather than `from_utf8_lossy`: lossy conversion silently
    // repairs the invalid sequences that are the most interesting inputs a
    // byte fuzzer produces, so the parser never sees them.
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = parse_manifest(text);
    }
});
```

Note what is absent: no `unwrap()`, no assertion on the result, no `expect`. `parse_manifest`
returning `Err` on garbage **is the requirement being met**. The `let _ =` is deliberate.

## Structured input, via `arbitrary`

Where the surface takes a typed value rather than bytes, derive `Arbitrary` instead of
hand-rolling a decoder — a hand-rolled one spends most of the corpus failing its own parse
before reaching the code under test.

```rust
#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use quire_rs::query::{run_query, Query};

#[derive(Arbitrary, Debug)]
struct Input {
    document: String,
    selector: String,
}

/// The query engine does not panic on any selector against any document.
///
/// Trace: FR-010-AC-3
///
/// spec-fuzz: obligation=FR-010-AC-3 harness=cargo-fuzz entry=query::run_query origin=authored
fuzz_target!(|input: Input| {
    if let Ok(query) = Query::parse(&input.selector) {
        let _ = run_query(&query, &input.document);
    }
});
```

`arbitrary` must be a declared dependency of the **fuzz** crate. If it is not, that is the
absent-tooling case from step 0 — add nothing, report it.

## The `[[bin]]` block

```toml
[[bin]]
name = "fuzz_nfr_003_m_1_parse_manifest"
path = "fuzz_targets/fuzz_nfr_003_m_1_parse_manifest.rs"
test = false
doc = false
```

`test = false` and `doc = false` keep the target out of `cargo test` and `cargo doc`, which
is what stops a nightly-only target from breaking a stable build.
