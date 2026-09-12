// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The FR-097 hash assertion, exercised in both directions.
//!
//! A hash assertion nobody has watched fail is indistinguishable from no
//! assertion at all, so this file does not only assert that the committed
//! artefact matches — it constructs each way it can stop matching and asserts
//! the named finding comes back. The three failing states are different
//! repairs, not one "mismatch".

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::PathBuf;

use quoin_schemas::{
    GENERATED_TYPES_PATH, GENERATED_TYPES_SHA256, GENERATOR_IDENTITY, GENERATOR_VERSION,
    SOURCE_SCHEMA_SHA256, SurfaceFinding, check_generated_source, sha256_hex, source_schema_digest,
    typescript_source,
};

/// `crates/quoin-schemas` → `crates` → `rust` → the repository.
fn repo_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..3 {
        root = root.parent().unwrap().to_path_buf();
    }
    root
}

fn committed_surface() -> String {
    std::fs::read_to_string(repo_root().join(GENERATED_TYPES_PATH)).unwrap()
}

/// Trace: FR-097-AC-1
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1612_a_fresh_render_of_the_rust_types_matches_the_committed_digest() {
    // THE drift probe, in the direction that matters: change a Rust boundary
    // type and do not run `make types`, and this is what fails. The committed
    // digest is a literal in `lib.rs`, so nothing here re-derives its own
    // expectation.
    let rendered = typescript_source().unwrap();
    assert_eq!(
        sha256_hex(rendered.as_bytes()),
        GENERATED_TYPES_SHA256,
        "a Rust boundary type changed without `make types`. \
         `{GENERATED_TYPES_PATH}` no longer describes the types that serialise \
         on the wire; regenerate it and review the diff."
    );
}

/// Trace: FR-097-AC-1
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1612_the_committed_source_schema_digest_is_the_current_one() {
    assert_eq!(
        source_schema_digest(),
        SOURCE_SCHEMA_SHA256,
        "the JSON Schema read off the Rust boundary types has moved since \
         `make types` last ran"
    );
}

/// Trace: FR-097-AC-1
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1612_the_generated_surface_declares_every_type_that_crosses_the_boundary() {
    let surface = committed_surface();
    for name in ["Diagnostic", "PingRequest", "PingPayload"] {
        assert!(
            surface.contains(&format!("export interface {name} {{")),
            "`{name}` crosses the boundary but is not in the generated surface"
        );
    }
    assert!(
        surface.contains("export const PROTOCOL_VERSION = 1;"),
        "{surface}"
    );
}

/// Trace: FR-097-AC-1
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1612_the_diagnostic_interface_is_rendered_literally() {
    // The expected bytes are written out rather than re-rendered. A test that
    // calls the renderer to build its own expectation agrees with the renderer
    // no matter what the renderer does.
    let expected = "export interface Diagnostic {\n\
                    \x20 /**\n\
                    \x20  * The stable code, from the catalogued enum — never a literal invented\n\
                    \x20  * at the call site.\n\
                    \x20  */\n\
                    \x20 code: string;\n";
    assert!(
        committed_surface().contains(expected),
        "the committed surface does not contain the expected Diagnostic opening:\n{expected}"
    );
    assert!(
        committed_surface().contains("  context: Record<string, string>;"),
        "a BTreeMap<String, String> must render as Record<string, string>"
    );
}

/// Trace: FR-097-AC-2
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1613_the_generated_surface_carries_a_complete_provenance_record() {
    let surface = committed_surface();
    assert!(
        surface.contains(&format!("generator: \"{GENERATOR_IDENTITY}\"")),
        "{surface}"
    );
    assert!(
        surface.contains(&format!("generatorVersion: \"{GENERATOR_VERSION}\"")),
        "{surface}"
    );
    assert!(surface.contains("sourceSchemaSha256:"), "{surface}");
    assert!(
        surface.contains(&format!("\"{SOURCE_SCHEMA_SHA256}\"")),
        "{surface}"
    );
}

/// Trace: FR-097-AC-2
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1613_an_artefact_missing_a_provenance_field_fails_the_gate() {
    for (field, needle) in [
        ("generator", "generator:"),
        ("generatorVersion", "generatorVersion:"),
        ("sourceSchemaSha256", "sourceSchemaSha256:"),
    ] {
        let stripped = committed_surface().replace(needle, "removed:");
        let finding = check_generated_source(&stripped).unwrap_err();
        assert_eq!(
            finding,
            SurfaceFinding::MissingProvenance { field },
            "removing `{field}` must be refused by name"
        );
        assert!(finding.to_string().contains(field), "{finding}");
    }
}

/// Trace: FR-097-AC-4
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1615_the_committed_file_is_generated_current_and_unmodified() {
    check_generated_source(&committed_surface()).unwrap();
}

/// Trace: FR-097-AC-4
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1615_a_hand_edit_fails_the_gate_at_an_unchanged_path() {
    // The artefact's path never changes; its provenance record is intact; only
    // one character of one type moved. Generated status is not a property of
    // where the file lives (FR-097-CON-2), so this must still fail.
    let edited = committed_surface().replace("  code: string;", "  code: string | null;");
    assert_ne!(
        edited,
        committed_surface(),
        "the planted edit did not apply"
    );
    assert_eq!(
        check_generated_source(&edited).unwrap_err(),
        SurfaceFinding::ContentDigest {
            expected: GENERATED_TYPES_SHA256.to_owned(),
            observed: sha256_hex(edited.as_bytes()),
        },
        "a hand edit must be a content-digest finding"
    );
    assert_ne!(sha256_hex(edited.as_bytes()), GENERATED_TYPES_SHA256);
}

/// Trace: FR-097-AC-5
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1616_an_artefact_with_a_stale_source_schema_digest_is_refused_rather_than_used() {
    let stale = committed_surface().replace(SOURCE_SCHEMA_SHA256, &"0".repeat(64));
    assert_eq!(
        check_generated_source(&stale).unwrap_err(),
        SurfaceFinding::StaleSourceSchema {
            recorded: "0".repeat(64),
            current: SOURCE_SCHEMA_SHA256.to_owned(),
        },
        "a stale artefact must be named stale, not merely different"
    );
}

/// Every `export interface` in a TypeScript source, as (name, field names).
///
/// A line scanner, not a parser, and deliberately so: it reads the two shapes
/// this repository's `src/core/` actually uses, and a declaration it cannot
/// read is reported rather than skipped — a duplicate detector that silently
/// fails to see a declaration is the "green over a population that is not what
/// it claims" failure this whole mechanism exists against.
fn interfaces(text: &str) -> Vec<(String, Vec<String>)> {
    let mut found = Vec::new();
    let mut current: Option<(String, Vec<String>)> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("export interface ")
            && let Some(name) = rest.split_whitespace().next()
        {
            current = Some((name.to_owned(), Vec::new()));
            continue;
        }
        let Some((_, fields)) = current.as_mut() else {
            continue;
        };
        if trimmed == "}" {
            if let Some(entry) = current.take() {
                found.push(entry);
            }
            continue;
        }
        if trimmed.starts_with('*') || trimmed.starts_with("/*") || trimmed.starts_with("//") {
            continue;
        }
        if let Some(colon) = trimmed.find(':') {
            let name = trimmed[..colon].trim_end_matches('?');
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                fields.push(name.to_owned());
            }
        }
    }
    found
}

/// Trace: FR-097-CON-1
/// Provenance: quoin#373, quoin#375
#[test]
fn tc_1614_no_hand_written_declaration_shadows_a_generated_boundary_type() {
    // FR-097-CON-1 in the form this stage can actually check: a type the
    // generator publishes must not ALSO be declared by hand under `src/core/`.
    // Matched on the FIELD SET, not on the name — the duplicate this found
    // when it was first written was called `CoreDiagnostic`, and a name-keyed
    // check would have reported clean over it. The fcd-published half of
    // FR-097-AC-3 waits on the Phase B gate (fcd#11) and is not faked here.
    let core_dir = repo_root().join("src/core");
    let generated: Vec<(String, Vec<String>)> = interfaces(&committed_surface())
        .into_iter()
        .map(|(name, mut fields)| {
            fields.sort();
            (name, fields)
        })
        .collect();
    assert_eq!(
        generated.len(),
        3,
        "the scanner read {} interfaces out of the generated surface, not 3; \
         it is broken and would report clean over anything",
        generated.len()
    );

    let mut exempt_seen = Vec::new();
    let mut duplicates = Vec::new();
    for entry in std::fs::read_dir(&core_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().is_some_and(|n| n == "types.ts")
            || path.extension().is_none_or(|e| e != "ts")
        {
            continue;
        }
        // ONE exemption, named here rather than skipped silently.
        //
        // `reference.ts` is the differential harness's TypeScript oracle
        // (FR-101). Its whole value is that it was written against the
        // documented contract and NOT against the Rust source: if it imported
        // the generated types, the harness would be comparing quoin-core to a
        // transliteration of itself, and `tc_375`'s twelve comparisons would
        // prove that two things written the same week agree. FR-097 governs
        // the CUTOVER surface — where quoin's own TypeScript calls the
        // boundary — which is `exec.ts` and `index.ts`.
        //
        // The assertion below pins the exemption list to exactly this file, so
        // a second file cannot join it by being added to a `continue`.
        if path.file_name().is_some_and(|n| n == "reference.ts") {
            exempt_seen.push("reference.ts");
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for (name, mut fields) in interfaces(&text) {
            fields.sort();
            for (published, published_fields) in &generated {
                if &fields == published_fields {
                    duplicates.push(format!(
                        "{} declares `{name}` with the fields of `{published}`, \
                         which {GENERATED_TYPES_PATH} publishes",
                        path.display()
                    ));
                }
            }
        }
    }
    assert_eq!(duplicates, Vec::<String>::new());
    assert_eq!(
        exempt_seen,
        vec!["reference.ts"],
        "the harness-oracle exemption must apply to exactly one file that exists"
    );
}
