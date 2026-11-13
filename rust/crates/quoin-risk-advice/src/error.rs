// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The crate's failure envelope.

use std::path::Path;

/// A stable, `Copy` classification of an operational failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// The scope or a file below it could not be read.
    Scope,
    /// A module could not be loaded.
    Modules,
    /// No loaded module declares a record type the report must check.
    TypeUndeclared,
    /// A record breaks the contract its type declares.
    Record,
    /// The report could not be serialized.
    Render,
}

/// An operational failure, as opposed to a finding about a record.
#[derive(Debug, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct AdviceError {
    /// What kind of failure this is.
    pub code: ErrorCode,
    /// The human-readable detail; never branched on.
    pub message: String,
}

impl ErrorCode {
    /// The code as printed.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scope => "scope_unreadable",
            Self::Modules => "modules_unloadable",
            Self::TypeUndeclared => "type_undeclared",
            Self::Record => "record_invalid",
            Self::Render => "render_failed",
        }
    }
}

impl AdviceError {
    pub(crate) fn read(error: &quoin_quire::Error) -> Self {
        use quoin_quire::ErrorCode as Engine;
        match error.code() {
            Engine::ModuleLoad | Engine::ModuleSetEmpty => Self {
                code: ErrorCode::Modules,
                message: error.to_string(),
            },
            Engine::ArchetypeUnknown => Self {
                code: ErrorCode::TypeUndeclared,
                message: format!(
                    "{error}. The record types come from the spec-objects-safety module: pass \
                     --module DIR for a module root whose version includes the residual_risk, \
                     benefit_risk and acceptability_policy types"
                ),
            },
            _ => Self {
                code: ErrorCode::Scope,
                message: error.to_string(),
            },
        }
    }

    pub(crate) fn record(path: &Path, id: &str, reason: &str) -> Self {
        Self {
            code: ErrorCode::Record,
            message: format!(
                "{} ({id}) breaks its type's contract: {reason}",
                path.display()
            ),
        }
    }

    pub(crate) fn render(error: &serde_json::Error) -> Self {
        Self {
            code: ErrorCode::Render,
            message: error.to_string(),
        }
    }
}
