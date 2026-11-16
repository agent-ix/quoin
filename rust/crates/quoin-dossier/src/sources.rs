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
}

/// The two discharge source forms, preserving their supplied order.
pub enum RmmSource {
    /// Separate bindings in command-line order.
    Bindings(Vec<RequirementsMapping>),
    /// A pack whose index fixes the table order.
    Pack(PackRequirementsMapping),
}
