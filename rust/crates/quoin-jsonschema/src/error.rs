// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one error envelope this crate reports, with stable codes.
//!
//! Two things can go wrong here and they are different failures with different
//! audiences:
//!
//! - A **vendored schema does not compile.** That is a defect in this
//!   repository — every schema the crate compiles is a committed artifact —
//!   and it names the file so the reader knows which one.
//! - A **document was refused.** That is ordinary data, and it carries the
//!   ajv-shaped diagnostics the caller reports.
//!
//! Codes are stable strings, not discriminants, because they cross a process
//! boundary in the callers' envelopes. [`JsonSchemaErrorCode::from_code`] and
//! [`JsonSchemaErrorCode::as_str`] round-trip, and `tests/tc_470_errors.rs`
//! asserts it over [`JsonSchemaErrorCode::ALL`] so a variant added without a
//! code fails the build rather than the wire.

use std::path::PathBuf;

use crate::measurement_schemas::VendoredSchema;
use crate::validator::SchemaError;

/// The stable code an error reports itself under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum JsonSchemaErrorCode {
    /// A committed schema document did not compile.
    VendoredSchemaInvalid,
    /// A document did not satisfy the schema it was parsed against.
    DocumentRefused,
}

impl JsonSchemaErrorCode {
    /// Every code, for the round-trip census.
    pub const ALL: &'static [Self] = &[Self::VendoredSchemaInvalid, Self::DocumentRefused];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VendoredSchemaInvalid => "jsonschema.vendored-schema-invalid",
            Self::DocumentRefused => "jsonschema.document-refused",
        }
    }

    /// The code with this wire spelling, if there is one.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|c| c.as_str() == code)
    }
}

impl std::fmt::Display for JsonSchemaErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Everything this crate refuses.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum JsonSchemaError {
    /// A committed schema document did not compile.
    #[error("vendored schema {} did not compile: {detail}", path.display())]
    VendoredSchemaInvalid {
        /// The schema document that did not compile.
        path: PathBuf,
        /// What the compiler said. Not contractual.
        detail: String,
    },
    /// A document did not satisfy the schema it was parsed against.
    #[error("{schema} refused the document: {} failure(s)", errors.len())]
    DocumentRefused {
        /// The schema that refused.
        schema: VendoredSchema,
        /// Every failure, in `(instance location, keyword)` order.
        errors: Vec<SchemaError>,
    },
}

impl JsonSchemaError {
    /// The stable code this error reports itself under.
    #[must_use]
    pub const fn code(&self) -> JsonSchemaErrorCode {
        match self {
            Self::VendoredSchemaInvalid { .. } => JsonSchemaErrorCode::VendoredSchemaInvalid,
            Self::DocumentRefused { .. } => JsonSchemaErrorCode::DocumentRefused,
        }
    }

    /// The failures, when this is a refusal; empty otherwise.
    #[must_use]
    pub fn errors(&self) -> &[SchemaError] {
        match self {
            Self::VendoredSchemaInvalid { .. } => &[],
            Self::DocumentRefused { errors, .. } => errors,
        }
    }
}
