// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The one filesystem seam (`load.ts`).
//!
//! Everything else in this crate is a pure function over values. Here, and
//! only here, four files are read: the assurance export, the accepted
//! premises, the audit envelope, and the retained bindings store. They are
//! read through [`GraphInputReader`], which has exactly one method, so a test
//! supplies a tree in memory and the production reader is the four lines at
//! the bottom of this file.
//!
//! No subprocess, no network, no environment variable and no working
//! directory: the repository root arrives in [`GraphLoadOptions`] as a value.
//!
//! # Absent is not unreadable
//!
//! The bindings store has three states and they are not the same answer: an
//! absent store is a repository that has recorded no evidence, an unreadable
//! one is a repository whose evidence cannot be trusted. The retained loader
//! separates them with an `existsSync` probe before the read
//! (`load.ts:218`); this one separates them by the read's own
//! [`std::io::ErrorKind::NotFound`], which is the same distinction taken one
//! syscall earlier and without the window between the two calls.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::{GraphError, GraphInput, Result};
use crate::input::{
    check_accepted_premises, check_audit_identity, parse_accepted_premises, parse_assurance_export,
    parse_audit_envelope,
};
use crate::json as reader;
use crate::model::binding::{BindingInput, parse_bindings_file};
use crate::model::report::GraphAnalysisInput;

/// Where the four inputs are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphLoadOptions {
    /// The repository root. The bindings store is found under it.
    pub repo: PathBuf,
    /// The assurance export.
    pub export_path: PathBuf,
    /// The accepted premises.
    pub premises_path: PathBuf,
    /// The audit envelope.
    pub audit_path: PathBuf,
}

impl GraphLoadOptions {
    /// The retained bindings store for this repository.
    ///
    /// `storeRoot(repo)/bindings.json` (`src/core/evidence.ts:124`). Both
    /// halves are asked of the crates that own them rather than re-spelled.
    #[must_use]
    pub fn bindings_path(&self) -> PathBuf {
        quoin_store::store::store_root(&self.repo).join(quoin_evidence::paths::bindings_path())
    }
}

/// Reads one file's text.
///
/// One method, because there is one thing this crate does to a filesystem.
/// The absent/unreadable distinction is [`std::io::ErrorKind::NotFound`] on
/// the error this returns, so an implementation does not have to model
/// existence separately and cannot disagree with itself about it.
pub trait GraphInputReader {
    /// The file's contents as UTF-8 text.
    ///
    /// # Errors
    ///
    /// Whatever the underlying store reports. [`ErrorKind::NotFound`] means
    /// the file is not there; anything else means it could not be read.
    fn read(&self, path: &Path) -> std::io::Result<String>;
}

/// The production reader: the real filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct OsGraphInputReader;

impl GraphInputReader for OsGraphInputReader {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }
}

/// Read the three declared inputs and the retained bindings store
/// (`load.ts:189`).
///
/// The three declared inputs are required: a failure to read or parse any of
/// them is a refusal. The bindings store is not — its absence is an answer the
/// reports carry as a gap.
///
/// # Errors
///
/// [`GraphError::InputUnreadable`] naming which input could not be read, and
/// [`GraphError::InputInvalid`] naming which one did not satisfy its contract.
pub fn load_graph_analysis_input(
    reader: &dyn GraphInputReader,
    options: &GraphLoadOptions,
) -> Result<GraphAnalysisInput> {
    let assurance =
        parse_assurance_export(&required(reader, GraphInput::Export, &options.export_path)?)?;

    let premises = parse_accepted_premises(&required(
        reader,
        GraphInput::Premises,
        &options.premises_path,
    )?)?;
    check_accepted_premises(&assurance, &premises)?;

    let audit = parse_audit_envelope(&required(reader, GraphInput::Audit, &options.audit_path)?)?;
    check_audit_identity(&audit, &assurance)?;

    Ok(GraphAnalysisInput {
        bindings: read_bindings(reader, &options.bindings_path()),
        assurance,
        premises,
        audit,
    })
}

/// One of the three required inputs (`load.ts:298`).
fn required(reader: &dyn GraphInputReader, input: GraphInput, path: &Path) -> Result<String> {
    reader
        .read(path)
        .map_err(|error| GraphError::InputUnreadable {
            input,
            path: path.display().to_string(),
            reason: error.to_string(),
        })
}

/// The retained bindings store, or why there is none (`load.ts:216`).
fn read_bindings(reader: &dyn GraphInputReader, path: &Path) -> BindingInput {
    let text = match reader.read(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return BindingInput::Absent {
                reason: format!("{} is absent", path.display()),
            };
        }
        Err(error) => {
            return BindingInput::Unreadable {
                reason: error.to_string(),
            };
        }
    };
    // The two refusals are not one refusal: `JSON.parse` throwing is reported
    // as the parser's own sentence (`load.ts:246`), and a document that parses
    // but does not satisfy the schema is reported as the longer one
    // (`load.ts:239`). Collapsing them would tell a reader their file is valid
    // JSON when it is not.
    let document = match reader::document(&text) {
        Ok(document) => document,
        Err(reason) => return BindingInput::Unreadable { reason },
    };
    match parse_bindings_file(&document) {
        Ok(bindings) => BindingInput::Available(bindings),
        Err(reason) => BindingInput::Unreadable {
            reason: format!(
                "{} is valid JSON but does not match the retained bindings schema: {reason}",
                path.display()
            ),
        },
    }
}
