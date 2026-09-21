// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The production [`SemanticGate`]: the vendored semantic contract, enforced
//! (quoin#446).
//!
//! [`semantic`](crate::semantic) is the seam — this crate owns *when* the gate
//! runs and what a rejection does to the filesystem and the registry, and
//! deliberately does not own what the gate decides. This module is the
//! implementation that makes the Rust install path refuse exactly what
//! `src/plugins.ts` refused: the module's own `semantic` block, a package
//! already claimed by another installed module, unresolvable imports, and a
//! derived package manifest that does not validate.
//!
//! # Why it lives in this crate and not in `quoin-core`'s `main.rs`
//!
//! It used to live in `main.rs`, on the argument that
//! `quoin-core/tests/tc_library_containment.rs` audits every source under
//! `quoin-core/src` except `main.rs`, so a library home for a gate that must
//! read the filesystem was unavailable. That argument rules out
//! `quoin-core/src/host.rs`; it does not rule out *a library*. The containment
//! audit walks **one crate**, and these rules are decidable policy with no
//! business being in an I/O shell, where the price was that they had no unit
//! tests at all — which is how a `roots` filter that silently emptied the
//! population `duplicate_package_diagnostic` judges against passed the whole
//! gate suite (the independent review of quoin#450, finding 2).
//!
//! This crate is the right home rather than `quoin-semantic` because the gate
//! is stated in *this* crate's vocabulary — [`SemanticGate`],
//! [`SemanticVerdict`], [`SemanticPin`], [`ModuleName`], [`ModuleRegistry`] —
//! and needs only free functions from `quoin-semantic`. Putting it the other
//! way round would make the policy crate depend on the installer.
//!
//! Every rule below is unit-tested against a real temporary home.

use std::path::{Path, PathBuf};

use crate::error::{ModulesError, RollbackOutcome};
use crate::ids::ModuleName;
use crate::paths::IxHome;
use crate::registry::ModuleRegistry;
use crate::semantic::{Diagnostic, SemanticGate, SemanticPin, SemanticVerdict, Severity};

/// The semantic contract, put behind [`SemanticGate`].
#[derive(Debug, Clone)]
pub struct ContractGate {
    /// The vendored schema tree (`$QUOIN_SEMANTIC_ROOT`).
    ///
    /// Supplied by the caller because only the caller knows where its own
    /// package was installed. Absent means the gate cannot judge, and an
    /// install is REFUSED rather than accepted unjudged.
    semantic_root: Option<PathBuf>,
    /// Where installed modules are materialised.
    modules_dir: PathBuf,
    /// The installed-module registry this gate judges a population from.
    registry_path: PathBuf,
}

impl ContractGate {
    /// A gate over an explicit modules directory and registry.
    #[must_use]
    pub fn new(
        semantic_root: Option<PathBuf>,
        modules_dir: PathBuf,
        registry_path: PathBuf,
    ) -> Self {
        Self {
            semantic_root,
            modules_dir,
            registry_path,
        }
    }

    /// A gate over the layout of `home`.
    #[must_use]
    pub fn for_home(home: &IxHome, semantic_root: Option<PathBuf>) -> Self {
        Self::new(semantic_root, home.modules_dir(), home.registry_path())
    }

    /// Re-read every installed module's `semantic` block and refuse on the
    /// first one whose contract is violated — or cannot be judged.
    ///
    /// `validateInstalledSemantics` in `src/plugins.ts`, rule for rule: the
    /// module's OWN diagnostics only. The duplicate-package and
    /// import-resolution checks are install-time rules about a module joining a
    /// population, and re-running them here would fail a home that the install
    /// path had already accepted.
    ///
    /// # Errors
    ///
    /// - [`ModulesError::SemanticContractUnavailable`] when the contract, the
    ///   registry or a module's manifest cannot be read. Unjudged is not the
    ///   same as clean, so this refuses rather than reporting a home clean it
    ///   never looked at.
    /// - [`ModulesError::SemanticContractViolation`] when a module's own block
    ///   is outside the contract.
    pub fn validate_installed(&self) -> Result<(), ModulesError> {
        let unavailable = |detail: String| ModulesError::SemanticContractUnavailable { detail };
        let semantic_root = self.semantic_root.as_deref().ok_or_else(|| {
            unavailable(
                "QUOIN_SEMANTIC_ROOT is not set, so installed modules cannot be re-validated; \
                 refusing rather than reporting them clean unjudged"
                    .to_owned(),
            )
        })?;
        let validators = quoin_semantic::manifest::SemanticValidators::load(semantic_root)
            .map_err(|error| {
                unavailable(format!(
                    "the vendored semantic contract could not be loaded: {error}"
                ))
            })?;
        // An unreadable registry is NOT an empty registry. `unwrap_or_default`
        // here would have re-validated a population of zero and reported the
        // home clean, which is the "check over an empty population" this
        // program has shipped four times.
        let registry = ModuleRegistry::read(&self.registry_path).map_err(|error| {
            unavailable(format!(
                "the installed-module registry at {} could not be read, so installed \
                 modules cannot be re-validated: {error}",
                self.registry_path.display()
            ))
        })?;
        for installed in registry.plugins {
            let root = self.modules_dir.join(installed.name.as_str());
            // A module with no manifest declares no contract of its own, which
            // is a different question from `others`' below: there, an absent
            // manifest silently shrinks the population a rule is judged
            // AGAINST, and it refuses. `src/plugins.ts` skipped here too.
            if !root.join("manifest.yaml").is_file() {
                continue;
            }
            // Same rule one level down: a module whose manifest is present but
            // unreadable is unjudged, not clean.
            let result =
                quoin_semantic::read_module_semantic(&root, &validators).map_err(|error| {
                    unavailable(format!(
                        "the semantic contract of installed module `{}` could not be read, \
                         so it cannot be re-validated: {error}",
                        installed.name.as_str()
                    ))
                })?;
            if quoin_semantic::has_errors(&result.diagnostics) {
                return Err(ModulesError::SemanticContractViolation {
                    name: installed.name.clone(),
                    report: quoin_semantic::format_diagnostics(&result.diagnostics),
                    rollback: RollbackOutcome::NotAttempted,
                });
            }
        }
        Ok(())
    }

    /// Every OTHER installed module that declares a semantic block, in sorted
    /// root order — the population `duplicate_package_diagnostic` and
    /// `resolve_imports` judge against.
    ///
    /// `installedSemanticModules(home).filter(m => m.name !== installed.name)`
    /// in `src/plugins.ts`, including the sort: the diagnostics name whichever
    /// module is found first, so the order is user-visible.
    ///
    /// # Errors
    ///
    /// A description of what could not be read, when the population cannot be
    /// established. There are three ways that happens and **all three refuse**,
    /// because the duplicate-package and import-resolution rules are judgements
    /// *against* this list: an unreadable registry, a registered module whose
    /// manifest is absent, and a registered module whose manifest cannot be
    /// read. The middle one used to be dropped silently, which turned both
    /// rules off over a population of zero while every gate stayed green
    /// (quoin#450 review, finding 2).
    ///
    /// This is a deliberate divergence from `installedSemanticModules`, which
    /// filtered absent manifests out with `existsSync`. A registry entry with
    /// no module on disk is a home that disagrees with itself, and answering
    /// "nothing else is installed" to a question about the installed population
    /// is the failure this program keeps shipping. The refusal names the module
    /// and both paths, so `quoin module remove <name>` or a reinstall clears it.
    fn others(
        &self,
        validators: &quoin_semantic::manifest::SemanticValidators,
        exclude: &ModuleName,
    ) -> Result<Vec<quoin_semantic::SemanticModule>, String> {
        let registry = ModuleRegistry::read(&self.registry_path).map_err(|error| {
            format!(
                "the installed-module registry at {} could not be read, so a module \
                 cannot be judged against the modules already installed: {error}",
                self.registry_path.display()
            )
        })?;
        let registered: Vec<&crate::registry::InstalledModule> = registry
            .plugins
            .iter()
            .filter(|installed| &installed.name != exclude)
            .collect();
        // The population floor, in the code rather than only in a test: one
        // root per registered sibling, or a refusal naming the one that is
        // missing. A `filter` here can only ever make this list shorter than
        // the registry says it is, which is precisely the silent drop.
        let mut roots: Vec<PathBuf> = Vec::with_capacity(registered.len());
        for installed in registered {
            let root = self.modules_dir.join(installed.name.as_str());
            let manifest = root.join("manifest.yaml");
            if !manifest.is_file() {
                return Err(format!(
                    "installed module `{}` is listed in the registry at {} but has no manifest \
                     at {}, so a module cannot be judged against the modules already \
                     installed; reinstall it or run `quoin module remove {}`",
                    installed.name.as_str(),
                    self.registry_path.display(),
                    manifest.display(),
                    installed.name.as_str()
                ));
            }
            roots.push(root);
        }
        roots.sort();
        roots
            .iter()
            .map(|root| {
                quoin_semantic::read_module_semantic(root, validators)
                    .map(|result| result.module)
                    .map_err(|error| {
                        format!(
                            "the semantic contract of installed module at {} could not be \
                             read, so a new module cannot be judged against it: {error}",
                            root.display()
                        )
                    })
            })
            // A sibling with no `semantic` block is genuinely not part of this
            // population — `src/plugins.ts` skipped it too. A sibling that
            // could not be READ is a different thing, and is the error above.
            .filter_map(Result::transpose)
            .collect()
    }
}

/// One error diagnostic, which is what every refusal on this gate is.
///
/// A free function rather than a closure inside `inspect` so that the helpers
/// below refuse in exactly the same shape: a verdict carrying one error and no
/// pin is what this crate reads as "do not install this".
fn refuse(rule: &str, message: String) -> SemanticVerdict {
    SemanticVerdict {
        diagnostics: vec![Diagnostic {
            severity: Severity::Error,
            rule: rule.to_owned(),
            message,
        }],
        pin: None,
    }
}

/// `quoin_semantic`'s diagnostics in the gate's own vocabulary.
fn as_gate_diagnostics(diagnostics: &[quoin_semantic::SemanticDiagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .map(|d| Diagnostic {
            severity: match d.severity {
                quoin_semantic::Severity::Error => Severity::Error,
                quoin_semantic::Severity::Warning => Severity::Warning,
            },
            rule: d.code.to_string(),
            message: format!("{}: {}", d.path, d.message),
        })
        .collect()
}

/// Derive, validate and write the package manifest, then derive the pin.
///
/// Order matters and is `installPlugin`'s: the manifest is validated and
/// written before the pin is offered, so a module whose manifest is invalid
/// never reaches the registry with a pin recorded for it.
///
/// # Errors
/// The refusal verdict to return, when any step of that sequence fails.
fn materialise(
    module: &quoin_semantic::SemanticModule,
    semantic_root: &Path,
) -> Result<SemanticPin, SemanticVerdict> {
    let derived = quoin_semantic::derive_package_manifest(module);
    match serde_json::to_value(&derived)
        .map_err(|e| e.to_string())
        .and_then(|value| {
            quoin_semantic::validate_package_manifest(semantic_root, &value)
                .map_err(|e| e.to_string())
        }) {
        Ok(Ok(())) => {}
        Ok(Err(errors)) => {
            return Err(refuse(
                "semantic/package-manifest-invalid",
                format!(
                    "derived package manifest is invalid: {}",
                    errors
                        .iter()
                        .map(|e| format!("{}: {}", e.instance_path, e.message))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            ));
        }
        Err(detail) => {
            return Err(refuse("semantic/package-manifest-underivable", detail));
        }
    }

    if let Err(error) = quoin_semantic::package_manifest::write_package_manifest(module, &derived) {
        return Err(refuse(
            "semantic/package-manifest-unwritable",
            format!("the derived package manifest could not be written: {error}"),
        ));
    }

    quoin_semantic::registry_pin(module)
        .map(|registry_pin| SemanticPin {
            package: registry_pin.package,
            semantic_core: registry_pin.semantic_core,
            exports: registry_pin.exports.into_iter().collect(),
        })
        .map_err(|error| {
            refuse(
                "semantic/pin-underivable",
                format!("the registry pin could not be derived: {error}"),
            )
        })
}

impl SemanticGate for ContractGate {
    fn inspect(&self, name: &ModuleName, root: &Path) -> SemanticVerdict {
        // No vendored tree means no judgement is possible. REFUSING is the only
        // honest answer: accepting unjudged would install a module the
        // TypeScript path would have rejected, silently, which is exactly the
        // regression FR-101's retire-after-parity rule exists to prevent.
        let Some(semantic_root) = self.semantic_root.as_deref() else {
            return refuse(
                "semantic/contract-unavailable",
                "QUOIN_SEMANTIC_ROOT is not set, so the semantic contract cannot be checked; \
                 refusing rather than installing a module unjudged"
                    .to_owned(),
            );
        };
        let validators = match quoin_semantic::manifest::SemanticValidators::load(semantic_root) {
            Ok(validators) => validators,
            Err(error) => {
                return refuse(
                    "semantic/contract-unreadable",
                    format!("the vendored semantic contract could not be loaded: {error}"),
                );
            }
        };

        // A manifest that cannot be read is UNJUDGED, not clean. This used to
        // return `SemanticVerdict::default()` — a clean verdict — on the
        // argument that `quoin-modules` already refuses a root without a
        // manifest; but on the reconcile path the name is declared and the
        // manifest is never parsed before the gate runs, so an unparsable one
        // reached here and was waved through, while `validate_installed` two
        // functions up refused on the identical error (quoin#450 review).
        let result = match quoin_semantic::read_module_semantic(root, &validators) {
            Ok(result) => result,
            Err(error) => {
                return refuse(
                    "semantic/module-unreadable",
                    format!(
                        "the semantic contract of module `{}` at {} could not be read, so it \
                         cannot be judged: {error}",
                        name.as_str(),
                        root.display()
                    ),
                );
            }
        };

        let mut diagnostics = result.diagnostics.clone();
        let mut pin = None;
        if let Some(module) = result.module.as_ref() {
            let others = match self.others(&validators, name) {
                Ok(others) => others,
                Err(detail) => {
                    return refuse("semantic/population-unreadable", detail);
                }
            };
            if let Some(duplicate) = quoin_semantic::duplicate_package_diagnostic(module, &others) {
                diagnostics.push(duplicate);
            }
            diagnostics.extend(quoin_semantic::resolve_imports(module, &others));

            if !quoin_semantic::has_errors(&diagnostics) {
                match materialise(module, semantic_root) {
                    Ok(derived_pin) => pin = Some(derived_pin),
                    Err(verdict) => return verdict,
                }
            }
        }

        SemanticVerdict {
            diagnostics: as_gate_diagnostics(&diagnostics),
            pin,
        }
    }
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
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::ContractGate;
    use crate::ids::ModuleName;
    use crate::semantic::{SemanticGate, Severity};

    /// The repository root: `rust/crates/quoin-modules` up three.
    fn repo_root() -> PathBuf {
        let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for _ in 0..3 {
            root.pop();
        }
        root
    }

    /// The real semantic contract `QUOIN_SEMANTIC_ROOT` publishes in
    /// production, not a fixture this test wrote: a gate judged against a
    /// contract of the test's own invention agrees with whatever the test put
    /// there. Materialized once per process from the same embedded bytes
    /// `quoin-semantic`'s `build.rs` compiles in (PLAT-887 de-vendoring).
    fn semantic_root() -> PathBuf {
        static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        ROOT.get_or_init(|| {
            let root = std::env::temp_dir().join(format!(
                "quoin-modules-contract-gate-{}",
                std::process::id()
            ));
            quoin_semantic::materialize_embedded_contract(&root)
                .expect("the embedded semantic contract materializes");
            root
        })
        .clone()
    }

    /// The `module-ok` fixture, which declares a valid `semantic` block.
    fn fixture() -> PathBuf {
        repo_root()
            .join("tests")
            .join("fixtures")
            .join("semantic-module")
            .join("module-ok")
    }

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    /// A home with `names` materialised from the fixture and recorded in the
    /// registry, each under its own directory.
    struct Home {
        _dir: TempDir,
        modules_dir: PathBuf,
        registry_path: PathBuf,
    }

    fn home(names: &[&str]) -> Home {
        let dir = tempfile::tempdir().unwrap();
        let modules_dir = dir.path().join("filament").join("modules");
        let registry_path = dir.path().join("filament").join("registry.json");
        fs::create_dir_all(&modules_dir).unwrap();
        let mut plugins = Vec::new();
        for name in names {
            let root = modules_dir.join(name);
            copy_tree(&fixture(), &root);
            plugins.push(serde_json::json!({
                "name": name,
                "source": { "type": "path", "path": root.to_string_lossy() },
                "resolvedPath": root.to_string_lossy(),
                "targetPath": root.to_string_lossy(),
                "installedAt": "2026-01-01T00:00:00.000Z",
            }));
        }
        fs::write(
            &registry_path,
            serde_json::to_string(&serde_json::json!({ "schemaVersion": 1, "plugins": plugins }))
                .unwrap(),
        )
        .unwrap();
        Home {
            _dir: dir,
            modules_dir,
            registry_path,
        }
    }

    impl Home {
        fn gate(&self) -> ContractGate {
            ContractGate::new(
                Some(semantic_root()),
                self.modules_dir.clone(),
                self.registry_path.clone(),
            )
        }

        fn root(&self, name: &str) -> PathBuf {
            self.modules_dir.join(name)
        }
    }

    fn name(of: &str) -> ModuleName {
        ModuleName::new(of).unwrap()
    }

    /// A lone module with a valid block passes and is pinned.
    ///
    /// The control for everything below: without it a gate that refused
    /// everything would satisfy the refusal tests.
    #[test]
    fn a_module_alone_in_its_home_is_clean_and_pinned() {
        let home = home(&["alpha"]);
        let verdict = home.gate().inspect(&name("alpha"), &home.root("alpha"));
        assert!(!verdict.has_errors(), "{:?}", verdict.diagnostics);
        let pin = verdict.pin.expect("a valid semantic block is pinned");
        assert_eq!(pin.package, "agent-ix/spec-objects-fixture");
    }

    /// THE population test. Two modules declaring the same `semantic.package`,
    /// and the second is refused because the first is in the population.
    ///
    /// This is the rule a `.filter(|_| false)` on `others`' root list turns off
    /// silently: over an empty population `duplicate_package_diagnostic`
    /// answers `None` and the install is accepted. Nothing in the suite noticed
    /// (quoin#450 review, finding 2) because nothing judged this gate's own
    /// rules — `main.rs` carried no `#[cfg(test)]` at all.
    #[test]
    fn a_package_already_claimed_by_an_installed_module_is_refused() {
        let home = home(&["alpha", "beta"]);
        let verdict = home.gate().inspect(&name("beta"), &home.root("beta"));
        assert!(
            verdict.has_errors(),
            "the duplicate package must be refused"
        );
        let report = verdict.render();
        assert!(
            report.contains("agent-ix/spec-objects-fixture"),
            "the refusal names the package: {report}"
        );
        assert!(
            report.contains("alpha"),
            "the refusal names the module already holding it: {report}"
        );
        assert!(verdict.pin.is_none(), "a refused module is not pinned");
    }

    /// The population is every registered sibling, not every sibling that
    /// happens to be on disk: a registry entry whose module is gone REFUSES
    /// rather than shrinking the set the rules are judged against.
    #[test]
    fn a_registered_module_with_no_manifest_refuses_rather_than_shrinking_the_population() {
        let home = home(&["alpha", "beta"]);
        fs::remove_file(home.root("alpha").join("manifest.yaml")).unwrap();
        let verdict = home.gate().inspect(&name("beta"), &home.root("beta"));
        assert!(verdict.has_errors());
        assert_eq!(
            verdict.diagnostics[0].rule,
            "semantic/population-unreadable"
        );
        assert!(
            verdict.diagnostics[0].message.contains("alpha"),
            "{}",
            verdict.diagnostics[0].message
        );
    }

    /// An unreadable registry is not an empty registry.
    #[test]
    fn an_unreadable_registry_refuses() {
        let home = home(&["alpha", "beta"]);
        fs::write(&home.registry_path, "{ not json").unwrap();
        let verdict = home.gate().inspect(&name("beta"), &home.root("beta"));
        assert!(verdict.has_errors());
        assert_eq!(
            verdict.diagnostics[0].rule,
            "semantic/population-unreadable"
        );
    }

    /// A sibling whose manifest is present but unparsable is unjudged, not
    /// absent from the population.
    #[test]
    fn an_unreadable_sibling_refuses() {
        let home = home(&["alpha", "beta"]);
        fs::write(home.root("alpha").join("manifest.yaml"), "\t- [unclosed").unwrap();
        let verdict = home.gate().inspect(&name("beta"), &home.root("beta"));
        assert!(verdict.has_errors());
        assert_eq!(
            verdict.diagnostics[0].rule,
            "semantic/population-unreadable"
        );
    }

    /// The module under inspection, unreadable: refused rather than reported
    /// clean. `validate_installed` refuses on the identical error, and two
    /// answers to one condition is how a gate stops meaning anything.
    #[test]
    fn an_unreadable_subject_manifest_refuses_rather_than_passing() {
        let home = home(&["alpha"]);
        fs::write(home.root("alpha").join("manifest.yaml"), "\t- [unclosed").unwrap();
        let verdict = home.gate().inspect(&name("alpha"), &home.root("alpha"));
        assert!(verdict.has_errors(), "an unjudgeable module is not clean");
        assert_eq!(verdict.diagnostics[0].rule, "semantic/module-unreadable");
        assert_eq!(verdict.diagnostics[0].severity, Severity::Error);
    }

    /// No vendored contract means no judgement, which means no install.
    #[test]
    fn an_absent_semantic_root_refuses() {
        let home = home(&["alpha"]);
        let gate = ContractGate::new(None, home.modules_dir.clone(), home.registry_path.clone());
        let verdict = gate.inspect(&name("alpha"), &home.root("alpha"));
        assert!(verdict.has_errors());
        assert_eq!(verdict.diagnostics[0].rule, "semantic/contract-unavailable");
    }

    /// `validate_installed` walks what is there and passes a clean home.
    #[test]
    fn validate_installed_passes_a_clean_home() {
        let home = home(&["alpha", "beta"]);
        home.gate().validate_installed().unwrap();
    }

    /// A module tampered with after it was installed is caught on re-read.
    #[test]
    fn validate_installed_refuses_a_tampered_module() {
        let home = home(&["alpha"]);
        let manifest = fs::read_to_string(home.root("alpha").join("manifest.yaml")).unwrap();
        fs::write(
            home.root("alpha").join("manifest.yaml"),
            manifest.replace("semantic_core: 0.1.0", "semantic_core: 9.9.9"),
        )
        .unwrap();
        let error = home.gate().validate_installed().unwrap_err();
        assert!(
            matches!(
                error,
                crate::error::ModulesError::SemanticContractViolation { .. }
            ),
            "{error}"
        );
    }

    /// An unreadable registry re-validates a population of ZERO if it is
    /// treated as empty. It refuses instead.
    #[test]
    fn validate_installed_refuses_an_unreadable_registry() {
        let home = home(&["alpha"]);
        fs::write(&home.registry_path, "{ not json").unwrap();
        let error = home.gate().validate_installed().unwrap_err();
        assert!(
            matches!(
                error,
                crate::error::ModulesError::SemanticContractUnavailable { .. }
            ),
            "{error}"
        );
    }
}
