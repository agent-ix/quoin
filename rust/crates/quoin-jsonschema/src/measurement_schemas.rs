// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The committed measurement schema documents, and parsing against them.
//!
//! # Two documents, two provenances
//!
//! | schema | retained source (deleted in quoin#479) | how it got here |
//! |---|---|---|
//! | [`MeasurementSchema::OperationalEvidenceV1`] | `src/measurement/schemas/operational-evidence-v1.schema.json` | copied byte-for-byte |
//! | [`MeasurementSchema::InterventionExperimentV1`] | `src/measurement/intervention-schema.ts` | captured, then two recorded deltas |
//!
//! The retained TypeScript no longer exists: quoin#479 deleted `src/measurement/`
//! after this port took over its routes. Both documents are therefore measured
//! against **committed captures** under `tests/goldens/`, each with a
//! `.provenance.json` naming the file, binding, serializer, producer and
//! revision it came from. FR-101-AC-5 forbids a live non-Rust runtime oracle
//! after cutover; a committed byte sequence is not one.
//!
//! The operational schema was already a JSON document, so it is carried over
//! verbatim and `tests/tc_470_measurement_schemas.rs` fails if the committed
//! document and the capture ever disagree.
//!
//! The intervention schema was not a document at all: `intervention-schema.ts`
//! was a *program* that built an object out of shared fragments. It was run
//! once, serialized with the repository's own `canonicalJson`, and the capture
//! is committed beside the retained document as
//! `tests/goldens/intervention-experiment-v1.captured.json` with its producing
//! revision. The retained document then differs from that capture by **exactly
//! two deltas**, both rulings recorded in `DIVERGENCE.md`, and the test
//! enumerates the difference set rather than asserting that one exists.
//!
//! # Nothing here reads the filesystem
//!
//! Both documents arrive through `include_str!`. A schema read with `std::fs`
//! at run time is a schema that can differ from the one the tests measured.

use std::path::Path;

use serde_json::Value;

use crate::error::JsonSchemaError;
use crate::validator::{FormatCheck, SchemaError, SchemaValidator};

/// The committed schema documents this crate ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum MeasurementSchema {
    /// FR-100 intervention-experiment records.
    InterventionExperimentV1,
    /// FR-059 operational-evidence records.
    OperationalEvidenceV1,
}

impl MeasurementSchema {
    /// Every committed schema, for the censuses that must not measure nothing.
    pub const ALL: &'static [Self] = &[Self::InterventionExperimentV1, Self::OperationalEvidenceV1];

    /// The schema's `$id`, which is also how it names itself in a refusal.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::InterventionExperimentV1 => {
                "https://agent-ix.github.io/quoin/schemas/intervention-experiment-v1.schema.json"
            }
            Self::OperationalEvidenceV1 => {
                "https://agent-ix.github.io/quoin/schemas/operational-evidence-v1.schema.json"
            }
        }
    }

    /// The committed document's path, relative to the repository root.
    #[must_use]
    pub const fn vendored_path(self) -> &'static str {
        match self {
            Self::InterventionExperimentV1 => {
                "rust/crates/quoin-jsonschema/schemas/intervention-experiment-v1.schema.json"
            }
            Self::OperationalEvidenceV1 => {
                "rust/crates/quoin-jsonschema/schemas/operational-evidence-v1.schema.json"
            }
        }
    }

    /// The committed document's bytes.
    #[must_use]
    pub const fn source(self) -> &'static str {
        match self {
            Self::InterventionExperimentV1 => {
                include_str!("../schemas/intervention-experiment-v1.schema.json")
            }
            Self::OperationalEvidenceV1 => {
                include_str!("../schemas/operational-evidence-v1.schema.json")
            }
        }
    }

    /// The committed document, parsed.
    ///
    /// # Errors
    ///
    /// [`JsonSchemaError::VendoredSchemaInvalid`] when the committed bytes are
    /// not JSON — a defect in this repository, never ordinary data.
    pub fn document(self) -> Result<Value, JsonSchemaError> {
        serde_json::from_str(self.source()).map_err(|source| {
            JsonSchemaError::VendoredSchemaInvalid {
                path: Path::new(self.vendored_path()).to_path_buf(),
                detail: source.to_string(),
            }
        })
    }

    /// Compile this schema, with `date_time` registered as `format: "date-time"`.
    ///
    /// Both documents carry `format: "date-time"`, and the retained ajv
    /// instances both registered a check for it, so both are asserted rather
    /// than annotated. The check is a parameter because the one RFC 3339
    /// grammar in this domain lives in `quoin-measurement`, and a crate the
    /// measurement crate depends on cannot depend back on it. Passing a second
    /// grammar in here would be the defect this arrangement exists to prevent:
    /// pass `quoin_measurement::date_time::Rfc3339DateTime::parse(..).is_ok()`.
    ///
    /// # Errors
    ///
    /// [`JsonSchemaError::VendoredSchemaInvalid`] when the committed document
    /// does not compile.
    pub fn compile(
        self,
        date_time: fn(&str) -> bool,
    ) -> Result<MeasurementValidator, JsonSchemaError> {
        let document = self.document()?;
        let validator = SchemaValidator::compile_with_formats(
            Path::new(self.vendored_path()),
            &document,
            &[],
            &[FormatCheck {
                name: "date-time",
                check: date_time,
            }],
        )?;
        Ok(MeasurementValidator {
            schema: self,
            validator,
        })
    }
}

impl std::fmt::Display for MeasurementSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// A compiled validator over one committed schema.
#[derive(Debug)]
pub struct MeasurementValidator {
    schema: MeasurementSchema,
    validator: SchemaValidator,
}

impl MeasurementValidator {
    /// Which schema this validator carries.
    #[must_use]
    pub const fn schema(&self) -> MeasurementSchema {
        self.schema
    }

    /// Parse `document` against the schema.
    ///
    /// Parse, don't validate: what comes back on success is a
    /// [`ValidDocument`], so a caller holding one cannot have skipped the
    /// check and no caller re-validates what it was handed.
    ///
    /// # Errors
    ///
    /// [`JsonSchemaError::DocumentRefused`], carrying every failure in
    /// `(instance location, keyword)` order.
    pub fn parse(&self, document: Value) -> Result<ValidDocument, JsonSchemaError> {
        let errors = self.validator.errors(&document);
        if errors.is_empty() {
            Ok(ValidDocument {
                schema: self.schema,
                value: document,
            })
        } else {
            Err(JsonSchemaError::DocumentRefused {
                schema: self.schema,
                errors,
            })
        }
    }

    /// The verdict alone, for parity measurement against ajv.
    ///
    /// The verdict is the contractual half; error text is not.
    #[must_use]
    pub fn is_valid(&self, document: &Value) -> bool {
        self.validator.is_valid(document)
    }

    /// Every failure, for diagnostic-shape measurement.
    #[must_use]
    pub fn errors(&self, document: &Value) -> Vec<SchemaError> {
        self.validator.errors(document)
    }
}

/// A document that satisfied the schema named inside it.
///
/// Modelled on `quoin_store::RawFileSha256Digest::parse_stored`: holding the
/// value *is* the proof, so it is constructed here and nowhere else.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidDocument {
    schema: MeasurementSchema,
    value: Value,
}

impl ValidDocument {
    /// The schema that accepted it.
    #[must_use]
    pub const fn schema(&self) -> MeasurementSchema {
        self.schema
    }

    /// The document.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// The document, by value.
    #[must_use]
    pub fn into_value(self) -> Value {
        self.value
    }
}
