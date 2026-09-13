// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The in-memory host: a repository stated by the caller.
//!
//! Split out of `source.rs` when quoin#471's and quoin#472's waves met on one
//! tree; see that module's header for the contract this implements.

use std::collections::BTreeMap;

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::raw_evidence::RawEvidencePath;

use super::{MeasurementSource, RawEvidenceFile, has_extension};

/// A repository stated in memory.
///
/// Holds no files, so [`MeasurementSource::raw_evidence_file`] refuses; see the
/// module header for why that is a refusal rather than a second hasher.
#[derive(Clone, Debug, Default)]
pub struct MemoryMeasurement {
    documents: BTreeMap<String, String>,
    collections: BTreeMap<String, Vec<u8>>,
    retained: BTreeMap<String, String>,
    interventions: BTreeMap<String, Vec<u8>>,
}

impl MemoryMeasurement {
    /// An empty repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an assurance document at a repository-relative path.
    #[must_use]
    pub fn with_document(mut self, path: impl Into<String>, text: impl Into<String>) -> Self {
        self.documents.insert(path.into(), text.into());
        self
    }

    /// Add a stored collection under a bare `<id>.json` file name.
    #[must_use]
    pub fn with_collection(mut self, name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.collections.insert(name.into(), bytes.into());
        self
    }

    /// Add the text of one retained evidence file, at a store-relative path.
    ///
    /// The file is still not *digestible* from here — see
    /// [`MeasurementSource::raw_evidence_file`] — so this states content for a
    /// reader, not evidence for an accounting.
    #[must_use]
    pub fn with_retained_evidence(
        mut self,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        self.retained.insert(path.into(), text.into());
        self
    }

    /// Add a retained intervention record under its `p-`/`b-` file name.
    #[must_use]
    pub fn with_intervention(mut self, name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.interventions.insert(name.into(), bytes.into());
        self
    }
}

impl MeasurementSource for MemoryMeasurement {
    fn assurance_documents(&self) -> Result<Vec<String>, MeasurementError> {
        // A `BTreeMap` is already in the order the disk walk sorts into.
        Ok(self
            .documents
            .keys()
            .filter(|path| {
                has_extension(path, "md")
                    && (path.starts_with("spec/assurance/") || path.starts_with("assurance/"))
            })
            .cloned()
            .collect())
    }

    fn document_text(&self, path: &str) -> Result<String, MeasurementError> {
        self.documents.get(path).cloned().ok_or_else(|| {
            MeasurementError::new(MeasurementErrorCode::Io, format!("{path}: absent"))
        })
    }

    fn collection_names(&self) -> Result<Vec<String>, MeasurementError> {
        Ok(self
            .collections
            .keys()
            .filter(|name| has_extension(name, "json"))
            .cloned()
            .collect())
    }

    fn collection_bytes(&self, name: &str) -> Result<Vec<u8>, MeasurementError> {
        self.collections.get(name).cloned().ok_or_else(|| {
            MeasurementError::new(MeasurementErrorCode::Io, format!("{name}: absent"))
        })
    }

    fn intervention_names(&self) -> Result<Vec<String>, MeasurementError> {
        Ok(self
            .interventions
            .keys()
            .filter(|name| has_extension(name, "json"))
            .cloned()
            .collect())
    }

    fn intervention_bytes(&self, name: &str) -> Result<Vec<u8>, MeasurementError> {
        self.interventions.get(name).cloned().ok_or_else(|| {
            MeasurementError::new(MeasurementErrorCode::Io, format!("{name}: absent"))
        })
    }

    fn raw_evidence_text(&self, path: &RawEvidencePath) -> Result<String, MeasurementError> {
        Err(MeasurementError::new(
            MeasurementErrorCode::RawEvidenceUnavailable,
            format!("this source holds no files, so `{path}` cannot be read"),
        ))
    }

    fn raw_evidence_file(
        &self,
        path: &RawEvidencePath,
    ) -> Result<RawEvidenceFile, MeasurementError> {
        Err(MeasurementError::new(
            MeasurementErrorCode::RawEvidenceUnavailable,
            format!("this source holds no files, so `{path}` cannot be digested"),
        ))
    }

    fn retained_evidence_text(&self, path: &RawEvidencePath) -> Result<String, MeasurementError> {
        self.retained.get(path.as_str()).cloned().ok_or_else(|| {
            MeasurementError::new(
                MeasurementErrorCode::RawEvidenceUnavailable,
                format!("this source holds no file at `{path}`"),
            )
        })
    }
}
