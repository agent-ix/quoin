// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! The install-time diagnostic (FR-070).
//!
//! A diagnostic is a **reported refusal**, not a crate error: the caller
//! collects every one of them and prints them, which is why
//! `readSemanticBlock` returns a list rather than stopping at the first.
//!
//! The code is an enum rather than a `String`. The TypeScript writes each code
//! as a literal at its construction site — sixteen of them, spelled by hand —
//! and a typo there produces a diagnostic nothing downstream can match. The
//! wire spelling is unchanged: [`DiagnosticCode::as_str`] is the contract.

use std::fmt;

/// How much a diagnostic is worth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Blocks the install.
    Error,
    /// Advisory; the install proceeds.
    Warning,
}

impl Severity {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

macro_rules! diagnostic_codes {
    ($($(#[$meta:meta])* $variant:ident => $wire:literal),+ $(,)?) => {
        /// Every diagnostic this crate can emit.
        ///
        /// Wire spellings are stable and are what `quoin module install` prints
        /// and what quire's artifact-time diagnostics join on.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[non_exhaustive]
        pub enum DiagnosticCode {
            $($(#[$meta])* $variant),+
        }

        impl DiagnosticCode {
            /// The stable wire spelling.
            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }

            /// Every code, so a catalogue check can enumerate them.
            #[must_use]
            pub fn all() -> &'static [Self] {
                &[$(Self::$variant),+]
            }

            /// Parse a wire spelling back to its variant.
            #[must_use]
            pub fn from_code(code: &str) -> Option<Self> {
                Self::all().iter().copied().find(|c| c.as_str() == code)
            }
        }
    };
}

diagnostic_codes! {
    /// A key the `semantic` block does not admit.
    UnknownKey => "semantic.unknown-key",
    /// A required `semantic` key is absent.
    MissingKey => "semantic.missing-key",
    /// A `semantic.targets` entry outside the declared registry.
    UnknownTarget => "semantic.unknown-target",
    /// A `semantic` value the schema refuses, with no more specific code.
    InvalidValue => "semantic.invalid-value",
    /// `semantic.package` is not `<org>/<repo>`.
    InvalidPackage => "semantic.invalid-package",
    /// `semantic.contract_version` is not the version this quoin understands.
    UnsupportedContractVersion => "semantic.unsupported-contract-version",
    /// `semantic.exports` names a type `object_types` does not declare.
    UnknownExport => "semantic.unknown-export",
    /// `semantic.semantic_core` is not a version this quoin ships a bundle for.
    UnknownSemanticCore => "semantic.unknown-semantic-core",
    /// `legacy_forms: error` without a usable sweep report (FR-074).
    SweepReportRequired => "semantic.sweep-report-required",
    /// An export whose `data_schema` is not a `{ schema, digest }` reference.
    ExportWithoutSchema => "semantic.export-without-schema",
    /// Another installed module already provides this `semantic.package`.
    DuplicatePackage => "semantic.duplicate-package",
    /// A `semantic.imports` entry no installed module provides at that version.
    ImportUnresolved => "semantic.import-unresolved",
    /// The import graph has a cycle.
    ImportCycle => "semantic.import-cycle",
    /// `data_schema` is not an object.
    DataSchemaShape => "semantic.data-schema-shape",
    /// `data_schema` mixes the reference form with other keys.
    DataSchemaAmbiguous => "semantic.data-schema-ambiguous",
    /// An inline `data_schema` under a module that declares a `semantic` block.
    InlineDataSchema => "semantic.inline-data-schema",
    /// `data_schema.schema` is not a module-relative path.
    DataSchemaPath => "semantic.data-schema-path",
    /// `data_schema.digest` is not `sha256:<64 hex>`.
    DataSchemaDigest => "semantic.data-schema-digest",
    /// `data_schema.schema` leaves the module root.
    DataSchemaEscape => "semantic.data-schema-escape",
    /// `data_schema.schema` names a file the module does not ship.
    DataSchemaMissing => "semantic.data-schema-missing",
    /// `data_schema.schema` names a file that could not be read.
    DataSchemaUnreadable => "semantic.data-schema-unreadable",
    /// The shipped schema's bytes do not hash to the recorded digest.
    DataSchemaDigestMismatch => "semantic.data-schema-digest-mismatch",
    /// The shipped schema's bytes are not JSON.
    DataSchemaNotJson => "semantic.data-schema-not-json",
    /// The shipped schema is not a JSON Schema 2020-12 document.
    DataSchemaNotSchema => "semantic.data-schema-not-schema",
    /// The shipped schema's `$id` is not the one its path implies.
    DataSchemaId => "semantic.data-schema-id",
    /// A `$ref` chain inside the shipped bundle is cyclic.
    SchemaRefCycle => "semantic.schema-ref-cycle",
    /// A `$ref` names a file neither bundle ships.
    SchemaRefUnshipped => "semantic.schema-ref-unshipped",
    /// A `$ref` names a semantic-core version the manifest does not record.
    SchemaRefVersion => "semantic.schema-ref-version",
    /// A legacy Properties form found by the sweep (FR-074).
    LegacyPropertiesForm => "semantic.legacy-properties-form",
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl serde::Serialize for DiagnosticCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// One install-time refusal or advisory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SemanticDiagnostic {
    /// The stable code.
    pub code: DiagnosticCode,
    /// How much it is worth.
    pub severity: Severity,
    /// Manifest or file locus, e.g. `object_types[entity].data_schema.schema`.
    pub path: String,
    /// Human-readable detail. **Not contractual** — see `DIVERGENCE.md`.
    pub message: String,
}

impl SemanticDiagnostic {
    /// An error-severity diagnostic.
    #[must_use]
    pub fn error(
        code: DiagnosticCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Error,
            path: path.into(),
            message: message.into(),
        }
    }

    /// A warning-severity diagnostic.
    #[must_use]
    pub fn warning(
        code: DiagnosticCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            path: path.into(),
            message: message.into(),
        }
    }

    /// The identity a parity check compares on: code, severity and path.
    ///
    /// `message` is deliberately excluded. The acceptance criterion for the
    /// port fixes diagnostic parity on (path, keyword, instance-location) and
    /// states that error text is not contractual.
    #[must_use]
    pub fn identity(&self) -> (DiagnosticCode, Severity, &str) {
        (self.code, self.severity, self.path.as_str())
    }
}

impl fmt::Display for SemanticDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} at {}: {}",
            self.severity, self.code, self.path, self.message
        )
    }
}

#[cfg(test)]
// Indexing and `unreachable!` are a test-only convenience: an out-of-range
// index in a test is a failing test, not a downed worker.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Trace: FR-070
    #[test]
    fn tc_378_020_every_diagnostic_code_round_trips() {
        for code in DiagnosticCode::all() {
            assert_eq!(DiagnosticCode::from_code(code.as_str()), Some(*code));
        }
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_021_diagnostic_codes_are_unique_and_namespaced() {
        let mut seen = std::collections::BTreeSet::new();
        for code in DiagnosticCode::all() {
            assert!(
                code.as_str().starts_with("semantic."),
                "{code} is not namespaced"
            );
            assert!(seen.insert(code.as_str()), "duplicate code {code}");
        }
        assert_eq!(seen.len(), 29);
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_022_display_matches_the_typescript_format() {
        let diagnostic = SemanticDiagnostic::error(
            DiagnosticCode::UnknownKey,
            "semantic.nope",
            "unknown key inside semantic: nope",
        );
        assert_eq!(
            diagnostic.to_string(),
            "error: semantic.unknown-key at semantic.nope: unknown key inside semantic: nope"
        );
    }
}
