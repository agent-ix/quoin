// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The accepted module premises: the caller's statement of what it agreed to
//! analyse under.
//!
//! # Why this type is declared here and not taken from the engine
//!
//! `quire_rs::AcceptedAssurancePremises` is the nearest thing the workspace
//! owns, and it is not this: it carries `format_version` and `modules` and no
//! `format`, and it does not serialize. The retained contract
//! (`input.ts:28-34`) is a **strict** three-member document whose `format` is
//! the literal `"quire-assurance"`, and that member is compared against the
//! export's own (`input.ts:135-146`) — so a type without it would report a
//! premises file naming a different format as matching.
//!
//! The module and schema premises below are this document's for the same
//! reason: the engine's are not `min(1)` on name or version and not
//! `^[0-9a-f]{64}$` on the digest, and those constraints are the contract.
//!
//! # The export's own identity is never this type
//!
//! `validateAcceptedAssurancePremises` builds a premises-shaped object out of
//! the export and compares it — which means that object must be allowed to be
//! *wrong*. So it is a [`serde_json::Value`] from [`export_premises`], not an
//! [`AcceptedPremises`] built by some unchecked back door, and the comparison
//! is `sameJson` (`input.ts:257`) called by its real name: equality of the two
//! canonical texts.

use serde::Serialize;
use serde_json::{Value, json};

use crate::ids::{Archetype, ModuleName, ModuleVersion, SchemaDigest};
use crate::json as reader;

/// The only format this contract accepts (`input.ts:30`).
pub const ASSURANCE_FORMAT: &str = "quire-assurance";

/// The only format version this contract accepts (`input.ts:31`).
pub const ASSURANCE_FORMAT_VERSION: u32 = 1;

/// One active archetype's semantic schema premise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaPremise {
    /// The archetype the schema is for.
    pub archetype: Archetype,
    /// Its semantic schema digest.
    pub schema_digest: SchemaDigest,
}

/// One loaded module and its active archetypes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModulePremise {
    /// The module's name.
    pub name: ModuleName,
    /// The module's version.
    pub version: ModuleVersion,
    /// Its active archetypes' schema premises.
    pub schemas: Vec<SchemaPremise>,
}

/// What the caller accepted, as the report repeats it back.
///
/// `format` and `format_version` are carried rather than implied, because the
/// report prints them and the retained type does the same. Both are proven by
/// [`AcceptedPremises::parse`], which is the only constructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcceptedPremises {
    /// The export format. Always [`ASSURANCE_FORMAT`].
    pub format: String,
    /// The export format version. Always [`ASSURANCE_FORMAT_VERSION`].
    pub format_version: u32,
    /// The accepted modules, canonically ordered.
    pub modules: Vec<ModulePremise>,
}

impl AcceptedPremises {
    /// Read the strict premises contract from one JSON document.
    ///
    /// The result is canonically ordered. There is no second, un-normalised
    /// spelling of this type in circulation, because
    /// `parseAcceptedAssurancePremises` canonicalizes before returning
    /// (`input.ts:107`) and so does this.
    ///
    /// # Errors
    ///
    /// One line naming the member that failed and why.
    pub fn parse(value: &Value) -> std::result::Result<Self, String> {
        let root = reader::object(value, "<root>")?;
        reader::strict(root, "", &["format", "format_version", "modules"])?;
        reader::literal_text(
            reader::member(root, "", "format")?,
            "format",
            ASSURANCE_FORMAT,
        )?;
        reader::literal_number(
            reader::member(root, "", "format_version")?,
            "format_version",
            u64::from(ASSURANCE_FORMAT_VERSION),
        )?;
        let modules = reader::array(reader::member(root, "", "modules")?, "modules")?;
        let mut parsed = Vec::with_capacity(modules.len());
        for (index, module) in modules.iter().enumerate() {
            parsed.push(parse_module(
                module,
                &reader::at("modules.", &index.to_string()),
            )?);
        }
        parsed.iter_mut().for_each(sort_schemas);
        parsed
            .sort_by(|left, right| (&left.name, &left.version).cmp(&(&right.name, &right.version)));
        Ok(Self {
            format: ASSURANCE_FORMAT.to_owned(),
            format_version: ASSURANCE_FORMAT_VERSION,
            modules: parsed,
        })
    }

    /// `canonicalizeAcceptedAssurancePremises` (`input.ts:178`).
    ///
    /// [`AcceptedPremises::parse`] already returns a canonical value, so this
    /// is a no-op on anything it produced. It is not dead: the fields are
    /// public because a caller assembling a [`crate::GraphAnalysisInput`] in
    /// memory writes them, and `base()` canonicalizes what it was handed
    /// (`analysis.ts:431`) rather than trusting it.
    #[must_use]
    pub fn canonicalized(&self) -> Self {
        let mut modules = self.modules.clone();
        modules.iter_mut().for_each(sort_schemas);
        modules
            .sort_by(|left, right| (&left.name, &left.version).cmp(&(&right.name, &right.version)));
        Self {
            format: self.format.clone(),
            format_version: self.format_version,
            modules,
        }
    }
}

/// `canonicalizeAcceptedAssurancePremises` applied to the identity an export
/// states about itself (`input.ts:135`, `input.ts:159`).
///
/// A [`Value`] and not an [`AcceptedPremises`]: this is the side of the
/// comparison that is allowed to be wrong.
#[must_use]
pub fn export_premises(export: &quoin_quire::model::AssuranceExport) -> Value {
    let mut modules: Vec<&quoin_quire::model::AssuranceModulePremise> =
        export.modules.iter().collect();
    modules.sort_by(|left, right| {
        compare(&left.name, &right.name).then_with(|| compare(&left.version, &right.version))
    });
    json!({
        "format": export.format,
        "format_version": export.format_version,
        "modules": modules
            .into_iter()
            .map(|module| {
                let mut schemas: Vec<&quoin_quire::model::AssuranceSchemaPremise> =
                    module.schemas.iter().collect();
                schemas.sort_by(|left, right| {
                    compare(&left.archetype, &right.archetype)
                        .then_with(|| compare(&left.schema_digest, &right.schema_digest))
                });
                json!({
                    "name": module.name,
                    "version": module.version,
                    "schemas": schemas
                        .into_iter()
                        .map(|schema| json!({
                            "archetype": schema.archetype,
                            "schema_digest": schema.schema_digest,
                        }))
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>(),
    })
}

/// The UTF-16 comparison every ordering in the retained source performs, for
/// the two sides that are still plain engine `String`s.
fn compare(left: &str, right: &str) -> std::cmp::Ordering {
    quoin_store::json::order::cmp_utf16(left, right)
}

fn sort_schemas(module: &mut ModulePremise) {
    module.schemas.sort_by(|left, right| {
        (&left.archetype, &left.schema_digest).cmp(&(&right.archetype, &right.schema_digest))
    });
}

fn parse_module(value: &Value, at: &str) -> std::result::Result<ModulePremise, String> {
    let object = reader::object(value, at)?;
    reader::strict(object, at, &["name", "version", "schemas"])?;
    let name = ModuleName::parse(reader::text(reader::member(object, at, "name")?, at)?)
        .map_err(|reason| format!("{at}name: {reason}"))?;
    let version = ModuleVersion::parse(reader::text(reader::member(object, at, "version")?, at)?)
        .map_err(|reason| format!("{at}version: {reason}"))?;
    let schemas = reader::array(reader::member(object, at, "schemas")?, at)?;
    let mut parsed = Vec::with_capacity(schemas.len());
    for (index, schema) in schemas.iter().enumerate() {
        parsed.push(parse_schema(
            schema,
            &reader::at(&format!("{at}schemas."), &index.to_string()),
        )?);
    }
    Ok(ModulePremise {
        name,
        version,
        schemas: parsed,
    })
}

fn parse_schema(value: &Value, at: &str) -> std::result::Result<SchemaPremise, String> {
    let object = reader::object(value, at)?;
    reader::strict(object, at, &["archetype", "schema_digest"])?;
    Ok(SchemaPremise {
        archetype: Archetype::parse(reader::text(reader::member(object, at, "archetype")?, at)?)
            .map_err(|reason| format!("{at}archetype: {reason}"))?,
        schema_digest: SchemaDigest::parse(reader::text(
            reader::member(object, at, "schema_digest")?,
            at,
        )?)
        .map_err(|reason| format!("{at}schema_digest: {reason}"))?,
    })
}
