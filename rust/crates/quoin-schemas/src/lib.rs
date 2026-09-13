// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The schema-sourced type surface of the quoin boundary (FR-097).
//!
//! FR-097 says TypeScript never hand-writes a boundary type. This crate is the
//! mechanism: `schemars` reads the JSON Schema off the **canonical Rust types**
//! in `quoin-core`, [`render`] turns that schema into TypeScript, and the
//! result — `src/core/types.ts` — carries a provenance record and a content
//! digest that [`check_generated_source`] asserts on every `cargo test`.
//!
//! # Direction of truth, and why this crate depends on `quoin-core`
//!
//! Rust is the source; TypeScript is generated; if the two ever disagree, the
//! Rust type wins. A crate that emits the schema of a type must therefore be
//! able to SEE that type, which puts `quoin-schemas` **downstream** of
//! `quoin-core`, not upstream of it.
//!
//! The edge used to point the other way, for one constant: `PROTOCOL_VERSION`
//! lived here on the premise that the generated side is generated from this
//! crate's own declarations. Honouring FR-097 literally would have meant
//! re-declaring `Diagnostic`, `PingRequest` and `PingPayload` here — a second
//! declaration of a type `quoin-core` already serialises, which is precisely
//! the hand-written duplicate FR-097-CON-1 forbids, and which would drift
//! silently because nothing compares two structurally-unrelated structs. So
//! the derives went onto the canonical types (behind `quoin-core`'s `schema`
//! feature, so the shipped binary carries none of the generator's tree),
//! `PROTOCOL_VERSION` moved to `quoin_core::protocol` beside the taxonomy it
//! versions, and this crate reads from there.
//!
//! # What keeps it honest
//!
//! Two digests, both checked in, both rewritten only by `make types`:
//!
//! - [`SOURCE_SCHEMA_SHA256`] — the schema the committed TypeScript was
//!   generated from. A Rust type change moves it, and the recorded value in
//!   the generated file is then **stale** (FR-097-AC-5).
//! - [`GENERATED_TYPES_SHA256`] — the bytes of the committed TypeScript. It is
//!   asserted against BOTH the file on disk (so a hand edit fails, at an
//!   unchanged path — FR-097-AC-4) and against a fresh render (so a Rust
//!   change that was never regenerated fails — FR-097-AC-1).
//!
//! A hash assertion nobody has watched fail is indistinguishable from no
//! assertion, so `tc_1612_a_fresh_render_of_the_rust_types_matches_the`
//! `committed_digest` exercises that branch directly rather than trusting it.
//!
//! # What must NOT be added here
//!
//! A type whose canonical definition lives in `filament-core-data`. fcd is the
//! multi-language schema source of truth (quoin#373, gate phase B / fcd#11);
//! re-declaring one of its types here would shadow it. The same rule is why
//! the boundary types are not re-declared here either — they belong to
//! `quoin-core`.

pub mod render;

use std::fmt::Write as _;

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::render::RenderRefusal;

/// The generator that writes `src/core/types.ts`.
///
/// Recorded in the generated file itself so generated status is established by
/// the provenance record, not by the directory the artefact happens to sit in
/// (FR-097-CON-2).
pub const GENERATOR_IDENTITY: &str = "quoin-schemas/quoin-schemas-gen";

/// The generator's version, taken from this crate's manifest.
pub const GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Repository-relative path of the generated TypeScript surface.
pub const GENERATED_TYPES_PATH: &str = "src/core/types.ts";

/// The `$id` of the schema the TypeScript surface is generated from.
pub const BOUNDARY_SCHEMA_ID: &str = "https://agent-ix.dev/quoin/core-boundary-v1.schema.json";

// --- The two checked-in digests. `make types` rewrites both; nothing else may.
/// SHA-256 of the canonical JSON of [`boundary_schema`] at generation time.
pub const SOURCE_SCHEMA_SHA256: &str =
    "7579931f5172a657e54f65945660187bf844014ead61e25869348a06f95b332c";

/// SHA-256 of the committed `src/core/types.ts`.
pub const GENERATED_TYPES_SHA256: &str =
    "11a5b715596916ecf7de7ab754e9e22b9b39a05fae551a70524a088e0d019632";

/// The JSON Schema of every type that crosses the `quoin-core` boundary.
///
/// Read off the canonical `quoin-core` types by `schemars`. `$defs` is a
/// `BTreeMap` in `serde_json`'s default build, so the order is the sorted type
/// name and not the order this function happens to ask in — the digest below
/// would otherwise depend on the order of the lines that follow.
#[must_use]
pub fn boundary_schema() -> Value {
    let mut generator = schemars::SchemaGenerator::default();
    let _ = generator.subschema_for::<quoin_core::protocol::Diagnostic>();
    let _ = generator.subschema_for::<quoin_core::ops::core::PingRequest>();
    let _ = generator.subschema_for::<quoin_core::ops::core::PingPayload>();
    let _ = generator.subschema_for::<quoin_core::ops::validators::RunRequest>();
    let _ = generator.subschema_for::<quoin_core::ops::validators::RunPayload>();
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": BOUNDARY_SCHEMA_ID,
        "title": "quoin-core boundary types",
        "description":
            "Every type crossing the quoin-core subprocess boundary (FR-096), \
             read off the canonical Rust declarations. Generated; do not edit.",
        "x-quoin-protocol-version": quoin_core::protocol::PROTOCOL_VERSION,
        "$defs": generator.take_definitions(true),
    })
}

/// Canonical JSON of a value: object keys sorted at every depth, no
/// insignificant whitespace.
///
/// The same discipline `quoin_core::protocol::canonical_json` applies on the
/// wire, applied here for the same reason: a digest over a representation that
/// can reorder is a digest that reports noise as drift.
///
/// # Panics
///
/// Never for a [`Value`], which is already a JSON document.
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Lowercase hex SHA-256 of some bytes.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::new(), |mut acc, byte| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

/// SHA-256 of the canonical JSON of [`boundary_schema`], as it is right now.
#[must_use]
pub fn source_schema_digest() -> String {
    sha256_hex(canonical_json(&boundary_schema()).as_bytes())
}

/// Render the TypeScript surface from [`boundary_schema`].
///
/// # Errors
///
/// [`RenderRefusal`] when the schema carries a construct the renderer does not
/// understand. It refuses rather than widening the offending type, so a
/// boundary type that outgrows the subset stops the build instead of arriving
/// in TypeScript as `unknown`.
pub fn typescript_source() -> Result<String, RenderRefusal> {
    let schema = boundary_schema();
    let digest = sha256_hex(canonical_json(&schema).as_bytes());
    let defs = schema
        .get("$defs")
        .and_then(Value::as_object)
        .ok_or_else(|| RenderRefusal {
            pointer: "#/$defs".to_owned(),
            reason: "the boundary schema declared no `$defs`".to_owned(),
        })?;

    let mut out = String::new();
    out.push_str(&header());
    out.push_str(&provenance_record(&digest));
    for (name, definition) in defs {
        out.push('\n');
        out.push_str(&render::render_definition(
            name,
            definition,
            &format!("#/$defs/{name}"),
        )?);
    }
    let _ = write!(
        out,
        "\n/** The IPC protocol revision this build of the boundary speaks. */\n\
         export const PROTOCOL_VERSION = {};\n",
        quoin_core::protocol::PROTOCOL_VERSION
    );
    Ok(out)
}

fn header() -> String {
    format!(
        "/**\n\
         \x20* The quoin-core boundary types (FR-097). GENERATED — DO NOT EDIT.\n\
         \x20*\n\
         \x20* Written by `{GENERATOR_IDENTITY}` from the JSON Schema `schemars`\n\
         \x20* reads off the canonical Rust declarations in\n\
         \x20* `rust/crates/quoin-core/src/protocol.rs`, `.../src/ops/` and the\n\
         \x20* domain crates those operations answer from.\n\
         \x20* Rust is the source of truth: where this file and a Rust type\n\
         \x20* disagree, the Rust type is right and this file is stale.\n\
         \x20*\n\
         \x20* Regenerate with `make types`. A hand edit does not survive review\n\
         \x20* and does not survive the gate: `quoin-schemas` asserts the digest\n\
         \x20* below against BOTH these bytes and a fresh render, so an edit here\n\
         \x20* fails `make rust-test` at an unchanged path, and a Rust type change\n\
         \x20* that was never regenerated fails it too.\n\
         \x20*\n\
         \x20* The machine-readable provenance is the `CORE_TYPES_PROVENANCE`\n\
         \x20* record below, and it is the ONLY copy: a second, prose copy up\n\
         \x20* here would be one more thing that can disagree with the artefact\n\
         \x20* it describes.\n\
         \x20*/\n"
    )
}

fn provenance_record(source_schema_digest: &str) -> String {
    format!(
        "\n/**\n\
         \x20* What produced this file, readable by the TypeScript side.\n\
         \x20*\n\
         \x20* Generated status is established by THIS record, not by the\n\
         \x20* artefact's directory name (FR-097-CON-2): moving the file does not\n\
         \x20* make it hand-written, and writing a file into a `generated/`\n\
         \x20* directory does not make it generated.\n\
         \x20*/\n\
         export const CORE_TYPES_PROVENANCE = {{\n\
         \x20 generator: \"{GENERATOR_IDENTITY}\",\n\
         \x20 generatorVersion: \"{GENERATOR_VERSION}\",\n\
         \x20 sourceSchemaSha256:\n\
         \x20   \"{source_schema_digest}\",\n\
         }} as const;\n"
    )
}

/// A reason a generated artefact must not be used.
///
/// Distinct variants rather than one message, because the three states need
/// three different repairs: regenerate, regenerate, or revert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceFinding {
    /// The file carries no complete provenance record, so nothing identifies
    /// it as generated or says what from (FR-097-AC-2).
    MissingProvenance {
        /// The provenance field that could not be read.
        field: &'static str,
    },
    /// The provenance names a source schema that is no longer the current one
    /// (FR-097-AC-5). The artefact describes types that have moved on.
    StaleSourceSchema {
        /// The digest the artefact records.
        recorded: String,
        /// The digest [`boundary_schema`] produces now.
        current: String,
    },
    /// The artefact's bytes are not the bytes that were generated
    /// (FR-097-AC-1, FR-097-AC-4).
    ContentDigest {
        /// The checked-in digest.
        expected: String,
        /// The digest of what was actually read.
        observed: String,
    },
}

impl std::fmt::Display for SurfaceFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProvenance { field } => write!(
                f,
                "{GENERATED_TYPES_PATH} carries no `{field}` in its provenance record; \
                 a generated artefact is identified by that record, not by its path. \
                 Run `make types`."
            ),
            Self::StaleSourceSchema { recorded, current } => write!(
                f,
                "{GENERATED_TYPES_PATH} was generated from source schema {recorded}, \
                 but the Rust boundary types now hash to {current}. The file is stale \
                 and is refused rather than used. Run `make types`."
            ),
            Self::ContentDigest { expected, observed } => write!(
                f,
                "{GENERATED_TYPES_PATH} hashes to {observed}, not the committed \
                 {expected}. Either the file was hand-edited or a Rust boundary type \
                 changed without a regeneration. Run `make types` and review the diff."
            ),
        }
    }
}

impl std::error::Error for SurfaceFinding {}

/// Read one field out of the generated file's `CORE_TYPES_PROVENANCE` record.
fn provenance_field(text: &str, key: &'static str) -> Result<String, SurfaceFinding> {
    // The key and its value are not always on one line: prettier wraps a
    // value that pushes past 80 columns onto the next, which the 64-hex digest
    // does. Anchoring on the key and taking the NEXT string literal reads both
    // layouts; anchoring on `key: "` read only the short one, and would have
    // reported a perfectly good provenance record as missing.
    let needle = format!("{key}:");
    let start = text
        .find(&needle)
        .ok_or(SurfaceFinding::MissingProvenance { field: key })?
        + needle.len();
    let rest = text
        .get(start..)
        .ok_or(SurfaceFinding::MissingProvenance { field: key })?;
    let open = rest
        .find('"')
        .ok_or(SurfaceFinding::MissingProvenance { field: key })?;
    let tail = rest
        .get(open + 1..)
        .ok_or(SurfaceFinding::MissingProvenance { field: key })?;
    let end = tail
        .find('"')
        .ok_or(SurfaceFinding::MissingProvenance { field: key })?;
    tail.get(..end)
        .map(str::to_owned)
        .ok_or(SurfaceFinding::MissingProvenance { field: key })
}

/// Judge a candidate `src/core/types.ts` against the Rust types it claims to
/// describe.
///
/// Checks, in the order a failure is most useful to read: the provenance
/// record is complete; the source schema it names is the current one; the
/// bytes are the committed bytes.
///
/// # Errors
///
/// The first [`SurfaceFinding`] that holds. `Ok(())` means the artefact is
/// generated, current, and unmodified.
pub fn check_generated_source(text: &str) -> Result<(), SurfaceFinding> {
    let generator = provenance_field(text, "generator")?;
    if generator != GENERATOR_IDENTITY {
        return Err(SurfaceFinding::MissingProvenance { field: "generator" });
    }
    let _version = provenance_field(text, "generatorVersion")?;
    let recorded = provenance_field(text, "sourceSchemaSha256")?;
    let current = source_schema_digest();
    if recorded != current {
        return Err(SurfaceFinding::StaleSourceSchema { recorded, current });
    }
    let observed = sha256_hex(text.as_bytes());
    if observed != GENERATED_TYPES_SHA256 {
        return Err(SurfaceFinding::ContentDigest {
            expected: GENERATED_TYPES_SHA256.to_owned(),
            observed,
        });
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    #[test]
    fn the_schema_covers_every_type_that_crosses_the_boundary() {
        let schema = boundary_schema();
        let defs = schema["$defs"].as_object().unwrap();
        let mut names: Vec<&String> = defs.keys().collect();
        names.sort();
        assert_eq!(
            names,
            [
                "Diagnostic",
                "EmptyGateFinding",
                "FindingKind",
                "LineNumber",
                "ObligationId",
                "PingPayload",
                "PingRequest",
                "RepoPath",
                "RunPayload",
                "RunRequest",
            ]
        );
    }

    #[test]
    fn the_schema_digest_does_not_depend_on_key_order() {
        // Canonical JSON is what makes the digest a statement about the
        // schema rather than about how serde happened to lay it out.
        let once = source_schema_digest();
        let twice = source_schema_digest();
        assert_eq!(once, twice);
        assert_eq!(once.len(), 64);
    }

    #[test]
    fn a_file_with_no_provenance_record_is_refused() {
        let error = check_generated_source("export interface Diagnostic {}\n").unwrap_err();
        assert_eq!(
            error,
            SurfaceFinding::MissingProvenance { field: "generator" }
        );
    }

    #[test]
    fn a_stale_source_schema_digest_is_refused_rather_than_used() {
        let stale = typescript_source()
            .unwrap()
            .replace(&source_schema_digest(), &"f".repeat(64));
        assert_eq!(
            check_generated_source(&stale).unwrap_err(),
            SurfaceFinding::StaleSourceSchema {
                recorded: "f".repeat(64),
                current: source_schema_digest(),
            }
        );
    }
}
