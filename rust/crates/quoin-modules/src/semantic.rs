// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The semantic-contract seam (quoin#381).
//!
//! `src/plugins.ts` rejects a module whose `semantic` block or `data_schema`
//! references fall outside the contract — at install, and again on the
//! reconcile path that materializes modules without going through install.
//! That rule is implemented in `src/semantic/manifest.ts` and
//! `src/semantic/package-manifest.ts`, which **Stage 3 owns**, not this stage.
//!
//! So it is a seam, not a stub: this crate owns *when* the gate runs, what a
//! rejection does to the filesystem and the registry, and what is pinned on
//! success. It does not own what the gate decides.
//!
//! [`PermissiveGate`] is the crate's default and passes everything. It is not a
//! placeholder for missing work — it is the honest statement that this stage
//! makes no semantic judgement — and it is the one implementation a production
//! consumer must *not* silently get by accident, which is why
//! [`ModuleInstaller`](crate::install::ModuleInstaller) takes the gate as a
//! constructor argument rather than defaulting it.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ids::ModuleName;

/// The registry pin recorded under an installed module's `semantic` key.
///
/// Field names match `SemanticRegistryPin` in `src/semantic/package-manifest.ts`
/// because the registry file is shared with the retained TypeScript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticPin {
    /// The module's declared semantic package.
    pub package: String,
    /// The semantic core it binds to.
    pub semantic_core: String,
    /// Digest per exported symbol.
    pub exports: BTreeMap<String, String>,
}

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Advisory; the install proceeds.
    Warning,
    /// The install is rejected.
    Error,
}

/// One semantic-contract finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// How serious.
    pub severity: Severity,
    /// A stable rule identifier from the semantic contract.
    pub rule: String,
    /// Human-readable detail.
    pub message: String,
}

/// A gate's verdict on one module root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticVerdict {
    /// Everything found.
    pub diagnostics: Vec<Diagnostic>,
    /// The pin to record, when the module declares a semantic block.
    pub pin: Option<SemanticPin>,
}

impl SemanticVerdict {
    /// Whether any diagnostic is an error.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// The diagnostics rendered for an error message, one per line.
    #[must_use]
    pub fn render(&self) -> String {
        self.diagnostics
            .iter()
            .map(|d| {
                format!(
                    "  [{}] {}: {}",
                    match d.severity {
                        Severity::Warning => "warning",
                        Severity::Error => "error",
                    },
                    d.rule,
                    d.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Judges whether a materialized module satisfies the semantic contract.
///
/// Implemented by Stage 3's semantic crate. Taking it as a trait keeps the
/// install/rollback/pin machinery testable with a gate that refuses on demand,
/// which is the only way to exercise the rollback paths without a real
/// contract violation.
pub trait SemanticGate {
    /// Inspect the module rooted at `root`.
    ///
    /// A gate reports findings; it does not perform I/O on the registry and it
    /// does not roll anything back. Both are this crate's job.
    fn inspect(&self, name: &ModuleName, root: &Path) -> SemanticVerdict;
}

/// A gate that accepts every module.
///
/// The default for this stage: quoin's Rust port does not yet implement the
/// semantic contract, and claiming a verdict it cannot reach would be worse
/// than declining to.
#[derive(Debug, Clone, Copy, Default)]
pub struct PermissiveGate;

impl SemanticGate for PermissiveGate {
    fn inspect(&self, _name: &ModuleName, _root: &Path) -> SemanticVerdict {
        SemanticVerdict::default()
    }
}
