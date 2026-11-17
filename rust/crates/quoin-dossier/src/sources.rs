// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Source adapters for matrix-backed documents, RMM and SOUP (FR-194 through FR-197).

use std::sync::Arc;

use quoin_assurance::{PackRequirementsMapping, RequirementsMapping, matrix::MatrixOutput};

use crate::{Inputs, SourceFailure};

/// Where matrix-backed dossier documents read the matrix from: the same
/// `matrix.build` path `quoin matrix` runs, supplied by the caller that owns
/// the evidence store and the auditor.
pub trait MatrixSource: Send + Sync {
    /// Build the matrix for the repository `inputs` names.
    ///
    /// # Errors
    ///
    /// Returns the failed source when the matrix cannot be built (exit 3).
    fn matrix(&self, inputs: &Inputs) -> Result<MatrixOutput, SourceFailure>;
}

/// The source assembly supplied by the host. The CLI supplies the same
/// library paths it uses for `matrix` and `discharge`, including their evidence
/// joins and configured module roots.
#[derive(Clone)]
pub struct Sources {
    /// Build the evidence-backed matrix over the selected scope.
    pub matrix: Arc<dyn MatrixSource>,
    /// Build the selected bindings or pack against the evidence store.
    pub rmm: fn(&Inputs) -> Result<RmmSource, SourceFailure>,
    /// Build the shared FR-144 report without reading an SBOM.
    pub soup: fn(&Inputs) -> Result<quoin_soup::DossierSource, SourceFailure>,
    /// Read policy records through quire's typed record reader.
    pub policy: fn(&Inputs) -> Result<quoin_quire::records::Corpus, SourceFailure>,
    /// Read threat-model records and the same flow findings as quire's relation report.
    pub threat_model: fn(&Inputs) -> Result<ThreatModelSource, SourceFailure>,
    /// Read processing activities and authored model facts from quire's flow report.
    pub records_of_processing: fn(&Inputs) -> Result<RecordsOfProcessingSource, SourceFailure>,
}

/// The selected records and engine-produced flow findings for DOS-TM.
pub struct ThreatModelSource {
    /// Typed authored records from the selected scope.
    pub records: quoin_quire::records::Corpus,
    /// Generic flow findings from the linked quire-rs engine; absent when the module declares none.
    pub flow: Option<quoin_quire::model::FlowReport>,
}

/// Processing activities and the bounded flow facts used by DOS-ROP.
pub struct RecordsOfProcessingSource {
    /// Validated authored processing-activity records.
    pub records: quoin_quire::records::Corpus,
    /// Authored model facts and resolved relationships from FR-096.
    pub flow: Option<quoin_quire::model::FlowReport>,
}

/// The two discharge source forms, preserving their supplied order.
pub enum RmmSource {
    /// Separate bindings in command-line order.
    Bindings(Vec<RequirementsMapping>),
    /// A pack whose index fixes the table order.
    Pack(PackRequirementsMapping),
}
