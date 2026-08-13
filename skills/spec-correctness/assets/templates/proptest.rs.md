# Rust / proptest templates

File: `tests/props_fr_NNN.rs`.
Every emitted test runs — there is no inert form.

Paths are **flat**: cargo auto-discovers integration tests only at `tests/*.rs`. A file
under `tests/props/` compiles as a helper module and never runs. See step 4.

## Anatomy

```rust
// tests/props_fr_012.rs
use proptest::prelude::*;
use quoin::catalog::{detect_duplicates, Entry};

fn entry() -> impl Strategy<Value = Entry> {
    // grounded in step 2 from `## Inputs` + the signature at src/catalog.rs:88
    ("[a-z]{1,12}", "[a-z]{1,12}").prop_map(|(kind, name)| Entry::new(kind, name))
}

proptest! {
    /// Trace: FR-012-AC-1 — declaring modules are listed in sorted order.
    /// spec-correctness: row=FR-012-AC-1 property=ordering extraction=extractable origin=regex review=none
    #[test]
    fn fr_012_ac_1_duplicate_modules_sorted(entries in prop::collection::vec(entry(), 0..32)) {
        for dup in detect_duplicates(&entries).duplicates {
            let mut sorted = dup.modules.clone();
            sorted.sort();
            prop_assert_eq!(dup.modules, sorted);
        }
    }
}
```

Both tag carriers are present: `Trace:` on its own doc-comment line, and `fr_012_ac_1_` in
the function name.

## Family bodies

Same `proptest! { #[test] fn … }` wrapper; only the assertion changes.

```rust
// round-trip
prop_assert_eq!(parse(&render(&x))?, x);

// round-trip, lossy (cite the FR line stating the loss)
prop_assert_eq!(normalize(&parse(&render(&x))?), normalize(&x));

// idempotence — seed with f's own outputs
fn seeded() -> impl Strategy<Value = Config> {
    prop_oneof![raw_config(), raw_config().prop_map(|c| normalize(&c))]
}
prop_assert_eq!(normalize(&normalize(&x)), normalize(&x));

// invariant
prop_assert!(resolve(&x).len() <= MAX_MODULES);
```

## error-case — negative domain

```rust
proptest! {
    /// Trace: FR-005-AC-1 — an unknown command raises an error that names the usage.
    /// spec-correctness: row=FR-005-AC-1 property=error-case extraction=extractable origin=regex review=none
    #[test]
    fn fr_005_ac_1_unknown_command_errors(
        cmd in "[a-z]{1,12}".prop_filter("must be unknown", |c| !KNOWN.contains(&c.as_str()))
    ) {
        let err = run(&[cmd.as_str()]).unwrap_err();
        prop_assert!(matches!(err, Error::UnknownCommand(_)));   // class, from the error enum
        prop_assert!(err.to_string().contains("Usage:"));        // payload, from ## Outputs
    }
}
```

## lifecycle — model vs SUT

```rust
proptest! {
    /// Trace: FR-040-AC-2 — a session accepts exactly the ops its state allows.
    /// spec-correctness: row=FR-040-AC-2 property=lifecycle extraction=extractable origin=regex review=none
    #[test]
    fn fr_040_ac_2_session_lifecycle(ops in prop::collection::vec(any_op(), 1..24)) {
        let (mut model, mut sut) = (Model::default(), Session::new());
        for op in ops {
            let legal = model.allows(&op);
            prop_assert_eq!(sut.apply(&op).is_ok(), legal);
            if legal { model.apply(&op); }
            prop_assert!(sut.invariant_holds());
        }
        prop_assert_eq!(sut.state(), model.state());
    }
}
```

## concurrency — loom

Not a `proptest!` block; `loom` enumerates interleavings itself.

```rust
/// Trace: FR-050-AC-1 — concurrent writers never lose an entry.
/// spec-correctness: row=FR-050-AC-1 property=concurrency extraction=extractable origin=regex review=none
#[test]
fn fr_050_ac_1_concurrent_writes_linearizable() {
    loom::model(|| {
        let store = loom::sync::Arc::new(Store::new());
        let handles: Vec<_> = (0..2)
            .map(|i| { let s = store.clone(); loom::thread::spawn(move || s.put(i, i)) })
            .collect();
        for h in handles { h.join().unwrap(); }
        assert_eq!(store.len(), 2);
    });
}
```

Keep the loom model tiny — two threads, two ops. Interleavings explode.

## A `candidate` record

Same file, same path, and it **runs** — `extraction: candidate` means the metamorphic label
was not corroborated structurally, which is something for a reviewer to read, not a reason
to disable a test:

```rust
proptest! {
    /// Trace: FR-018-AC-3 — a plugin source maps to exactly one resolved root.
    /// spec-correctness: row=FR-018-AC-3 property=invariant extraction=candidate origin=regex-candidate
    #[test]
    fn fr_018_ac_3_source_maps_to_one_root(src in plugin_source()) { … }
}
```

What marks it for review is a `medium` finding in the review artifact (step 6), not an
`#[ignore]` in the tree.

## Notes

- **Import by the `[lib] name`, not the package name.** They differ often —
  `package.name = "quire-rs"` but `lib.name = "quire_rs"`, so an integration test writes
  `use quire_rs::…`. Read `[lib] name` in `Cargo.toml`; fall back to the package name with
  `-` replaced by `_` only when no `[lib]` block declares one.
- `prop_assert*` inside `proptest!`, plain `assert*` inside `loom::model`.
- Bound every `prop::collection::vec` — `0..32` unless the FR states a bound.
- Prefer building a negative domain directly over `prop_filter` when the filter rejects
  most values; proptest gives up after too many rejections.
- Do not set `ProptestConfig { cases, .. }` above the default without recording why as a
  finding.
