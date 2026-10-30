// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Portable, typed claims about explicit producer input byte origins.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use engineering_assurance::campaign::{
    MeasurementProcedure, ProcedureInputOrigin, ProcedureInputOriginKind,
};

use super::run::{InputSource, RunMemberBindings, SelectedInput};

/// Versioned Quoin origin inventory carried by a retained collection.
pub const INPUT_ORIGINS_SCHEMA: &str = "quoin.campaign-input-origins/v1";

/// Whether the authored EA procedure fixed the origin of every explicit role.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginDeclaration {
    /// Legacy procedure made no enforceable origin declaration.
    Unclaimed,
    /// Every explicit role was matched to an authored EA origin declaration.
    Enforced,
}

/// Complete ordered inventory of explicit producer inputs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputOriginInventory {
    /// Versioned schema discriminator.
    pub schema: String,
    /// Whether an authored origin policy was enforced.
    pub declaration: OriginDeclaration,
    /// One role/path/source claim per explicit EA request input.
    pub inputs: Vec<InputOriginClaim>,
}

/// One explicit EA binding paired with a claimed source of its bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputOriginClaim {
    /// Producer request role, path and mode.
    pub binding: InputOriginBinding,
    /// Claimed origin of the exact bytes.
    pub source: OriginSource,
}

/// Portable projection of one EA producer input binding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputOriginBinding {
    /// Procedure input role.
    pub role: String,
    /// Portable relative path staged into EA's invocation root.
    pub path: String,
    /// Whether EA stages the bytes with executable mode.
    pub executable: bool,
}

/// Closed generic origin vocabulary; no host path is retained.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OriginSource {
    /// Caller selected exact retained bytes without source/dependency provenance.
    SelectedBytes,
    /// File in one exact verified Git source tree.
    SourceFile {
        /// Campaign source repository key.
        repository: String,
        /// Tracked regular path in that Git tree.
        path: String,
    },
    /// Sealed output artifact of an earlier declared dependency attempt.
    Dependency {
        /// Dependency member name.
        member: String,
        /// Positive attempt index.
        index: i64,
        /// EA output artifact role, including an output-tree leaf when applicable.
        artifact_role: String,
    },
}

/// A runtime selection did not match the actual EA request inventory.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum InputOriginError {
    /// An explicit role or path is duplicated.
    #[error("duplicate explicit producer input role or path")]
    Duplicate,
    /// Authored origin kind or source identity differs from machine selection.
    #[error("authored procedure input origin differs from selected source")]
    DeclarationMismatch,
}

/// Derive EA's runtime origin claim from the actual selected input sources.
///
/// An absent authored declaration is legacy and must receive no runtime claim.
#[must_use]
pub fn selected_declarations(
    inputs: &[SelectedInput],
    procedure: &MeasurementProcedure,
) -> Option<Vec<ProcedureInputOrigin>> {
    procedure.input_origins.as_ref()?;
    Some(
        inputs
            .iter()
            .map(|input| {
                let (
                    kind,
                    source_repository,
                    source_path,
                    dependency_member,
                    dependency_artifact_role,
                ) = match &input.source {
                    InputSource::File(_) => (
                        ProcedureInputOriginKind::SelectedBytes,
                        None,
                        None,
                        None,
                        None,
                    ),
                    InputSource::SourceFile { repository, path } => (
                        ProcedureInputOriginKind::SourceFile,
                        Some(repository.clone()),
                        Some(path.clone()),
                        None,
                        None,
                    ),
                    InputSource::Dependency {
                        member,
                        artifact_role,
                        ..
                    } => (
                        ProcedureInputOriginKind::Dependency,
                        None,
                        None,
                        Some(member.clone()),
                        Some(artifact_role.clone()),
                    ),
                };
                ProcedureInputOrigin {
                    role: input.role.clone(),
                    kind,
                    source_repository,
                    source_path,
                    dependency_member,
                    dependency_artifact_role,
                }
            })
            .collect(),
    )
}

/// Record the machine-selected origins of the explicit producer inputs.
///
/// # Errors
/// Refuses a duplicate role or path, or a claim the authored procedure contradicts.
pub fn from_runtime(
    runtime: &RunMemberBindings,
    procedure: &MeasurementProcedure,
) -> Result<InputOriginInventory, InputOriginError> {
    let mut roles = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut inputs = Vec::with_capacity(runtime.inputs.len());
    for selected in &runtime.inputs {
        if !roles.insert(selected.role.as_str()) || !paths.insert(selected.path.as_str()) {
            return Err(InputOriginError::Duplicate);
        }
        let source = match &selected.source {
            InputSource::File(_) => OriginSource::SelectedBytes,
            InputSource::SourceFile { repository, path } => OriginSource::SourceFile {
                repository: repository.clone(),
                path: path.clone(),
            },
            InputSource::Dependency {
                member,
                index,
                artifact_role,
            } => OriginSource::Dependency {
                member: member.clone(),
                index: *index,
                artifact_role: artifact_role.clone(),
            },
        };
        inputs.push(InputOriginClaim {
            binding: InputOriginBinding {
                role: selected.role.clone(),
                path: selected.path.clone(),
                executable: selected.executable,
            },
            source,
        });
    }
    inputs.sort_by(|a, b| a.binding.role.cmp(&b.binding.role));
    let declaration = declaration_for(&inputs, procedure)?;
    Ok(InputOriginInventory {
        schema: INPUT_ORIGINS_SCHEMA.to_owned(),
        declaration,
        inputs,
    })
}

/// Compare exact authored role origins with runtime claims before execution credit.
///
/// # Errors
/// Refuses any declared role whose source kind or identity differs.
pub fn declaration_for(
    inputs: &[InputOriginClaim],
    procedure: &MeasurementProcedure,
) -> Result<OriginDeclaration, InputOriginError> {
    let Some(authored) = &procedure.input_origins else {
        return Ok(OriginDeclaration::Unclaimed);
    };
    let declared_roles: BTreeSet<_> = authored.iter().map(|origin| origin.role.as_str()).collect();
    let actual_roles: BTreeSet<_> = inputs
        .iter()
        .map(|input| input.binding.role.as_str())
        .collect();
    let procedure_roles: BTreeSet<_> = procedure
        .inputs
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|input| input.role.as_str())
        .collect();
    if declared_roles.len() != authored.len()
        || actual_roles.len() != inputs.len()
        || declared_roles != procedure_roles
        || !actual_roles.is_subset(&declared_roles)
    {
        return Err(InputOriginError::DeclarationMismatch);
    }
    for input in inputs {
        let Some(declaration) = authored
            .iter()
            .find(|declared| declared.role == input.binding.role)
        else {
            return Err(InputOriginError::DeclarationMismatch);
        };
        let matches = match (&declaration.kind, &input.source) {
            (ProcedureInputOriginKind::SelectedBytes, OriginSource::SelectedBytes) => true,
            (
                ProcedureInputOriginKind::SourceFile,
                OriginSource::SourceFile { repository, path },
            ) => {
                declaration.source_repository.as_deref() == Some(repository)
                    && declaration.source_path.as_deref() == Some(path)
            }
            (
                ProcedureInputOriginKind::Dependency,
                OriginSource::Dependency {
                    member,
                    artifact_role,
                    ..
                },
            ) => {
                declaration.dependency_member.as_deref() == Some(member)
                    && declaration.dependency_artifact_role.as_deref() == Some(artifact_role)
            }
            _ => false,
        };
        if !matches {
            return Err(InputOriginError::DeclarationMismatch);
        }
    }
    Ok(OriginDeclaration::Enforced)
}
