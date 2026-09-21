// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Checking the installed `SpecReview.analysis` schema before emitting a
//! document (PLAT-837, PLAT-891).
//!
//! `spec-review`'s own rule: "a selected analysis absent from the installed
//! `SpecReview.analysis` schema is an unavailable dependency, not permission
//! to emit an invalid review document." PLAT-891 tracks that the fix
//! (`spec-artifacts-process` `a4fb5f3`) has not reached every installed
//! module copy yet. This module checks the ACTUAL installed file at run
//! time -- never a point-in-time claim about it -- per this ticket's own
//! instruction not to assert a dated claim.
//!
//! The module path resolved here (`<modules_dir>/spec-artifacts-process/
//! schemas/spec-review-frontmatter.schema.json`) reuses
//! [`quoin_modules::IxHome`] rather than re-deriving `~/.ix/filament/modules`
//! a second time -- that directory name is one fact `quoin-modules` already
//! owns (see its own doc: "the same layout `src/catalog.ts` and quire-rs
//! both read").

use std::path::PathBuf;

use quoin_modules::IxHome;
use serde::Deserialize;

use crate::error::{JevError, JevErrorCode, Result};

/// The value this lens's `SpecReview` documents declare in their frontmatter.
pub const ANALYSIS_VALUE: &str = "criterion-strength";

/// Where the schema lives beneath a resolved `IxHome`'s modules directory.
fn schema_path(home: &IxHome) -> PathBuf {
    home.modules_dir()
        .join("spec-artifacts-process")
        .join("schemas")
        .join("spec-review-frontmatter.schema.json")
}

/// Only the slice of the schema this check reads.
#[derive(Debug, Deserialize)]
struct SpecReviewSchema {
    properties: Properties,
}

#[derive(Debug, Deserialize)]
struct Properties {
    analysis: AnalysisProperty,
}

#[derive(Debug, Deserialize)]
struct AnalysisProperty {
    #[serde(default)]
    #[serde(rename = "enum")]
    values: Vec<String>,
}

/// Confirms `criterion-strength` is in the installed `SpecReview.analysis`
/// enum, resolving the schema path from `home`.
///
/// # Errors
/// [`JevErrorCode::SchemaUnreadable`] when the file is missing or does not
/// parse as the expected shape; [`JevErrorCode::SchemaUnavailable`] when it
/// parses fine and simply does not list `criterion-strength` yet (PLAT-891's
/// exact situation) -- the caller must stop and report this rather than emit
/// `spec/reviews/criterion-strength.md` anyway.
pub fn check(home: &IxHome) -> Result<()> {
    let path = schema_path(home);
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        JevError::new(
            JevErrorCode::SchemaUnreadable,
            format!("could not read {}: {error}", path.display()),
        )
    })?;
    let schema: SpecReviewSchema = serde_json::from_str(&raw).map_err(|error| {
        JevError::new(
            JevErrorCode::SchemaUnreadable,
            format!(
                "{} did not parse as the expected schema shape: {error}",
                path.display()
            ),
        )
    })?;
    if schema
        .properties
        .analysis
        .values
        .iter()
        .any(|value| value == ANALYSIS_VALUE)
    {
        return Ok(());
    }
    Err(JevError::new(
        JevErrorCode::SchemaUnavailable,
        format!(
            "{} does not list \"{ANALYSIS_VALUE}\" in SpecReview.analysis yet (PLAT-891); \
             reinstall the spec-artifacts-process module before emitting \
             spec/reviews/criterion-strength.md",
            path.display()
        ),
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use std::fs;

    use quoin_modules::IxHome;
    use tempfile::TempDir;

    use super::{ANALYSIS_VALUE, check, schema_path};
    use crate::error::JevErrorCode;

    fn home_with_schema(analysis_values: &[&str]) -> (TempDir, IxHome) {
        let dir = TempDir::new().expect("tempdir");
        let home = IxHome::new(dir.path());
        let schema_dir = home
            .modules_dir()
            .join("spec-artifacts-process")
            .join("schemas");
        fs::create_dir_all(&schema_dir).expect("create schema dir");
        let schema = serde_json::json!({
            "properties": {
                "analysis": {"type": "string", "enum": analysis_values}
            }
        });
        fs::write(schema_path(&home), schema.to_string()).expect("write schema");
        (dir, home)
    }

    /// Provenance: PLAT-837, PLAT-891. The exact situation this ticket
    /// documents: the schema exists and parses, but does not list
    /// `criterion-strength` yet. This must refuse loudly, never emit.
    #[test]
    fn a_schema_missing_criterion_strength_is_reported_as_unavailable() {
        let (_dir, home) = home_with_schema(&["base", "architecture-evaluation"]);
        let error = check(&home).expect_err("criterion-strength is not listed");
        assert_eq!(error.code, JevErrorCode::SchemaUnavailable);
    }

    /// Provenance: PLAT-837. Once the value is present, the check passes.
    #[test]
    fn a_schema_carrying_criterion_strength_passes() {
        let (_dir, home) = home_with_schema(&["base", ANALYSIS_VALUE]);
        check(&home).expect("criterion-strength is listed");
    }

    /// Provenance: PLAT-837. A missing file is a distinct, named failure
    /// from "the value is absent" -- it means "no installed copy was found
    /// at all", not "reinstall to pick up a newer one".
    #[test]
    fn a_missing_schema_file_is_reported_as_unreadable() {
        let dir = TempDir::new().expect("tempdir");
        let home = IxHome::new(dir.path());
        let error = check(&home).expect_err("nothing was installed here");
        assert_eq!(error.code, JevErrorCode::SchemaUnreadable);
    }
}
