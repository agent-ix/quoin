// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every zod declaration in the retained module has a counterpart here, and
//! the two are counted rather than believed.
//!
//! # Why a census
//!
//! A port of 763 lines of schema can lose a type silently: the crate still
//! compiles, the parity corpus still passes if no captured document exercised
//! the lost member, and the loss surfaces as a production refusal months later.
//! Counting both sides makes a dropped type a failing test.
//!
//! The census is over **constructs**, not over names, because names diverge on
//! purpose: `bareDigest` is [`quoin_measurement_graph::scalars::BareDigest`]
//! but `z.enum([...])` has no name at all in the retained file. Each row below
//! pairs one zod construct with the single Rust spelling this port uses for it,
//! and the pairing is one-to-one by construction:
//!
//! | zod | here |
//! | --- | --- |
//! | `z.object({…}).strict()` | `#[serde(deny_unknown_fields)]` |
//! | `z.enum([…])`, `z.discriminatedUnion(…)` | `#[serde(rename_all = "snake_case")]` |
//! | `z.literal(…)` | `string_literal!` / `integer_literal!` in `wire.rs` |
//! | `.optional()` | `#[serde(default, deserialize_with = "…absent_or")]` |
//! | `.nullable()` | a `NullableText` member |
//!
//! # The ticket's number does not hold
//!
//! quoin#475 states "107 zod declarations". No measurement of
//! `src/measurement/graph-adapters.ts` produces 107: there are
//! [`ZOD_CONSTRUCTORS`] constructor calls, [`ZOD_OBJECTS`] object schemas and
//! 24 named `*Schema` constants. The census asserts what is there rather than
//! what the ticket says is there, and the discrepancy is reported on the
//! ticket instead of being rounded into a passing test.
//!
//! Trace: FR-066-AC-1, FR-101-AC-5
//! Provenance: quoin#475

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The retained module this crate ports, relative to the crate root.
const RETAINED: &str = "../../../src/measurement/graph-adapters.ts";

/// Every `z.<constructor>(` call in the retained module.
///
/// Not paired with anything: it is the total the other rows are drawn from, and
/// it is the number the ticket's "107" was measured against.
const ZOD_CONSTRUCTORS: usize = 146;

/// `z.object(` calls, every one of which carries `.strict()`.
const ZOD_OBJECTS: usize = 29;

/// `z.enum(` calls plus the one `z.discriminatedUnion(`.
const ZOD_CLOSED_SETS: usize = 12;

/// `z.literal(` calls: 11 string literals and 2 integer literals.
const ZOD_LITERALS: usize = 13;

/// The `z.literal(` calls that are the arms of the one `z.discriminatedUnion`.
///
/// They have no counterpart in `wire.rs`, and should not: serde consumes the
/// discriminant before the arm's struct sees the object, so `"corpus"`,
/// `"verifies"` and `"implements"` are the *variant tags* of
/// `assurance::relations::Relation`. Declaring them as members too would be a
/// member no document may carry.
const DISCRIMINANT_LITERALS: usize = 3;

/// `.optional()` modifiers: a member that may be absent, and may not be null.
const ZOD_OPTIONALS: usize = 7;

/// `.nullable()` modifiers: a member that must be present, and may be null.
const ZOD_NULLABLES: usize = 1;

/// The count below which this census is not reading the retained module at all.
const RETAINED_FLOOR: usize = 700;

/// The retained TypeScript, with every run of whitespace removed.
///
/// The retained file is prettier-formatted, so `z\n  .object(` and `z.object(`
/// are the same declaration written two ways and a line-oriented count would
/// miss one of them. Removing whitespace makes the count a fact about the
/// declarations rather than about the formatter.
fn retained_source() -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join(RETAINED);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
    assert!(
        text.lines().count() >= RETAINED_FLOOR,
        "anti-vacuity floor: {} is {} lines, under {RETAINED_FLOOR} — the census is not reading \
         the module it claims to",
        path.display(),
        text.lines().count()
    );
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Every `.rs` file under `src/`, concatenated.
fn crate_source() -> String {
    fn walk(directory: &Path, into: &mut String) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(directory)
            .expect("src/ is readable")
            .map(|entry| entry.expect("a directory entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                into.push_str(&std::fs::read_to_string(&path).expect("a source file"));
                into.push('\n');
            }
        }
    }
    let mut found = String::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    assert!(
        found.lines().count() >= RETAINED_FLOOR,
        "anti-vacuity floor: the crate reads as {} lines — the census is not reading the crate",
        found.lines().count()
    );
    found
}

/// How many `#[serde(…)]` attributes in the crate carry `needle`.
///
/// Attribute lines only, so a mention of a spelling in a doc comment is not
/// counted as a declaration of it — the difference between describing the rule
/// and applying it.
fn serde_attributes_with(source: &str, needle: &str) -> usize {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("#[serde(") && line.contains(needle))
        .count()
}

/// The retained zod constructs are the ones this census claims.
#[test]
fn tc_475_040_the_retained_zod_census_is_the_one_declared() {
    let retained = retained_source();
    let count = |needle: &str| retained.matches(needle).count();

    assert_eq!(count("z.object("), ZOD_OBJECTS, "z.object( calls");
    assert_eq!(
        count(".strict()"),
        ZOD_OBJECTS,
        "every z.object( in the retained module is .strict(); one that is not would admit \
         unknown members, and this port would refuse them"
    );
    assert_eq!(
        count("z.enum(") + count("z.discriminatedUnion("),
        ZOD_CLOSED_SETS,
        "closed sets"
    );
    assert_eq!(count("z.literal("), ZOD_LITERALS, "z.literal( calls");
    assert_eq!(count(".optional()"), ZOD_OPTIONALS, ".optional() modifiers");
    assert_eq!(count(".nullable()"), ZOD_NULLABLES, ".nullable() modifiers");

    let constructors = [
        "z.object(",
        "z.string(",
        "z.array(",
        "z.number(",
        "z.literal(",
        "z.enum(",
        "z.record(",
        "z.unknown(",
        "z.discriminatedUnion(",
    ]
    .into_iter()
    .map(count)
    .sum::<usize>();
    assert_eq!(
        constructors, ZOD_CONSTRUCTORS,
        "the retained module's constructor total changed; the port is against a different file \
         than the one this census was measured from"
    );
}

/// Every retained construct has exactly as many counterparts here.
#[test]
fn tc_475_041_every_retained_construct_has_a_counterpart_here() {
    let source = crate_source();

    assert_eq!(
        serde_attributes_with(&source, "deny_unknown_fields"),
        ZOD_OBJECTS,
        "a `z.object(…).strict()` without a `#[serde(deny_unknown_fields)]` counterpart is a \
         type this port dropped, or one it invented"
    );
    assert_eq!(
        serde_attributes_with(&source, r#"rename_all = "snake_case""#),
        ZOD_CLOSED_SETS,
        "closed sets: 11 `z.enum(` plus the one `z.discriminatedUnion(`, each a Rust enum whose \
         wire spellings are the retained lowercase ones"
    );
    assert_eq!(
        source
            .lines()
            .map(str::trim)
            .filter(
                |line| line.starts_with("string_literal!") || line.starts_with("integer_literal!")
            )
            .count(),
        ZOD_LITERALS - DISCRIMINANT_LITERALS,
        "each `z.literal(…)` that is a member is a type with one inhabitant in wire.rs; the \
         other three are the discriminated union's variant tags"
    );
    assert_eq!(
        serde_attributes_with(&source, "absent_or"),
        ZOD_OPTIONALS,
        "each `.optional()` is `#[serde(default, deserialize_with = \"…absent_or\")]`, which \
         admits absence and refuses null exactly as zod does"
    );
    assert_eq!(
        source.matches(": NullableText").count(),
        ZOD_NULLABLES,
        "each `.nullable()` is a NullableText member, which admits null and refuses absence"
    );
}

/// The two sides are counted from two different trees.
///
/// Without this the census could be satisfied by reading one file twice.
#[test]
fn tc_475_042_the_census_reads_two_distinct_trees() {
    let retained = retained_source();
    let ported = crate_source();
    assert!(
        retained.contains("z.object("),
        "the retained side is TypeScript"
    );
    assert!(
        !ported.contains("z.object("),
        "the ported side carries no zod; if it does, the census is reading the retained file"
    );
    assert!(
        ported.contains("deny_unknown_fields"),
        "the ported side is Rust"
    );
}
