// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The in-memory module host every test in this domain runs against.
//!
//! Shared by [`super::tests`] and [`super::taxonomy`] so that neither grows a
//! second, divergent double. The point of the capability object is that the
//! refusal, mapping and argument-validation paths are all exercisable with no
//! disk and no network; this is what makes that true, and what keeps
//! `tests/tc_library_containment.rs` satisfied by construction rather than by
//! exemption.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::cell::RefCell;
use std::path::Path;

use quoin_modules::{
    InstallOutcome, InstalledModule, MarketplaceManifest, ModuleName, ModulesError,
    ModulesErrorCode, ReconcileMode, ReconcileReport, RollbackOutcome, Source,
    error::SourceFieldRule,
};

use crate::capabilities::ModuleHost;

/// A module host with no disk and no network.
///
/// Every test below runs against this, which is the point of the capability
/// object: the refusal, mapping and argument-validation paths are all
/// exercised without a filesystem, so `tests/tc_library_containment.rs`
/// stays satisfied by construction rather than by exemption.
#[derive(Default)]
pub(super) struct FakeHost {
    pub(super) modules: Vec<InstalledModule>,
    pub(super) fail_with: Option<ModulesErrorCode>,
    /// An installed module whose manifest no longer satisfies the contract.
    ///
    /// Set by `tests::reconciling_revalidates_what_is_installed_and_refuses_a_tampered_module`:
    /// the whole point of that path is a module that was accepted when
    /// installed and is not acceptable now, which no install-time failure
    /// (`fail_with`) can stand in for, because it fails the reconcile itself
    /// rather than the re-validation after it.
    pub(super) tampered: Option<&'static str>,
    pub(super) calls: RefCell<Vec<String>>,
}

impl FakeHost {
    pub(super) fn with_module(name: &str) -> Self {
        Self {
            modules: vec![module(name)],
            ..Self::default()
        }
    }

    pub(super) fn failing(code: ModulesErrorCode) -> Self {
        Self {
            fail_with: Some(code),
            ..Self::default()
        }
    }

    fn fail(&self) -> Option<ModulesError> {
        self.fail_with.map(error_with_code)
    }

    fn record(&self, call: impl Into<String>) {
        self.calls.borrow_mut().push(call.into());
    }
}

impl ModuleHost for FakeHost {
    fn list(&self, home: Option<&Path>) -> Result<Vec<InstalledModule>, ModulesError> {
        self.record(format!("list({home:?})"));
        self.fail().map_or_else(|| Ok(self.modules.clone()), Err)
    }

    fn install(
        &self,
        home: Option<&Path>,
        source: &Source,
    ) -> Result<InstallOutcome, ModulesError> {
        self.record(format!("install({home:?},{})", source.type_name()));
        self.fail().map_or_else(
            || {
                Ok(InstallOutcome {
                    module: module("installed"),
                    replaced_previous: true,
                })
            },
            Err,
        )
    }

    fn remove(&self, home: Option<&Path>, name: &ModuleName) -> Result<(), ModulesError> {
        self.record(format!("remove({home:?},{})", name.as_str()));
        self.fail().map_or(Ok(()), Err)
    }

    fn ensure_defaults(
        &self,
        home: Option<&Path>,
        manifest: &MarketplaceManifest,
        mode: ReconcileMode,
    ) -> Result<ReconcileReport, ModulesError> {
        self.record(format!(
            "ensure_defaults({home:?},{},{mode:?})",
            manifest.entries.len()
        ));
        self.fail().map_or_else(
            || {
                Ok(ReconcileReport {
                    installed: vec![ModuleName::new("alpha").unwrap()],
                    unchanged: vec![ModuleName::new("beta").unwrap()],
                    ..ReconcileReport::default()
                })
            },
            Err,
        )
    }

    fn validate_installed(&self, home: Option<&Path>) -> Result<(), ModulesError> {
        self.record(format!("validate_installed({home:?})"));
        if let Some(name) = self.tampered {
            return Err(ModulesError::SemanticContractViolation {
                name: ModuleName::new(name).unwrap(),
                report: "semantic.unknown-target: target \"go\" is not admitted".to_owned(),
                rollback: RollbackOutcome::NotAttempted,
            });
        }
        Ok(())
    }
}

pub(super) fn module(name: &str) -> InstalledModule {
    InstalledModule {
        name: ModuleName::new(name).unwrap(),
        source: Source::Path {
            path: "/m".to_owned(),
        },
        r#ref: None,
        sha: None,
        resolved_path: "/cache/m".to_owned(),
        target_path: "/home/filament/modules/m".to_owned(),
        installed_at: "2026-01-01T00:00:00Z".to_owned(),
        semantic: None,
    }
}

/// A real `ModulesError` for the handful of codes the fake host needs to
/// raise.
///
/// Deliberately NOT a total function over the catalogue: the exhaustive
/// mapping is asserted against [`core_code`] over
/// `ModulesErrorCode::all()`, which needs no constructible variant, so a
/// catch-all here cannot quietly make a mapping assertion vacuous.
pub(super) fn error_with_code(code: ModulesErrorCode) -> ModulesError {
    match code {
        ModulesErrorCode::ModuleNotInstalled => ModulesError::ModuleNotInstalled {
            name: ModuleName::new("gone").unwrap(),
        },
        ModulesErrorCode::SemanticContractViolation => ModulesError::SemanticContractViolation {
            name: ModuleName::new("bad").unwrap(),
            report: "error: import not in contract".to_owned(),
            rollback: RollbackOutcome::Restored,
        },
        ModulesErrorCode::RegistryUnwritable => ModulesError::RegistryUnwritable {
            path: std::path::PathBuf::from("/h/filament/registry.json"),
            source: std::io::Error::other("disk"),
        },
        ModulesErrorCode::InvalidSource => ModulesError::InvalidSourceField {
            field: "url",
            rule: SourceFieldRule::NonEmpty,
        },
        other => panic!("no test constructor for {other}; add one"),
    }
}

pub(super) fn manifest_yaml() -> String {
    "schemaVersion: 1\nentries:\n  - name: alpha\n    source:\n      type: path\n      path: /a\n"
        .to_owned()
}
