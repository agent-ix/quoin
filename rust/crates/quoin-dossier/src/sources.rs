// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Source adapters for the trace matrix and RMM (FR-194, FR-195).

use quoin_assurance::{PackRequirementsMapping, RequirementsMapping, matrix::MatrixOutput};

use crate::{Inputs, SourceFailure};

/// The source assembly supplied by the host. The CLI supplies the same
/// library paths it uses for `matrix` and `discharge`, including their evidence
/// joins and configured module roots.
#[derive(Clone, Copy)]
pub struct Sources {
    /// Build the evidence-backed matrix over the selected scope.
    pub matrix: fn(&Inputs) -> Result<MatrixOutput, SourceFailure>,
    /// Build the selected bindings or pack against the evidence store.
    pub rmm: fn(&Inputs) -> Result<RmmSource, SourceFailure>,
}

/// The two discharge source forms, preserving their supplied order.
pub enum RmmSource {
    /// Separate bindings in command-line order.
    Bindings(Vec<RequirementsMapping>),
    /// A pack whose index fixes the table order.
    Pack(PackRequirementsMapping),
}
