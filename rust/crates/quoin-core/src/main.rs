// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin-core <domain>.<op>` — the I/O shell.
//!
//! Everything decidable lives in the library. This file does five things and
//! must keep doing only five: read argv, read stdin, **construct the host
//! capabilities**, call [`quoin_core::dispatch::dispatch`], write the two
//! streams and exit.
//!
//! Even "read stdin" is a library decision here: the ceiling on an untrusted
//! stream is [`quoin_core::protocol::MAX_REQUEST_BYTES`] and the bounded read
//! is [`quoin_core::dispatch::read_request`], so the refusal is unit-testable
//! against a `&[u8]` without a process. This file supplies the handle and
//! nothing else.
//!
//! The discipline that matters here is that **stdout is written only when the
//! outcome carries a payload**. A caller reading a half-written object off a
//! failed run is the defect this ordering prevents: the payload is serialised
//! in full, in memory, before a byte reaches stdout.
//!
//! # Why the capability construction is HERE
//!
//! `tests/tc_library_containment.rs` audits every source under `src/` except
//! this one and refuses a `std::fs`, `std::env`, `std::process` or `std::net`
//! path in any of them. A module install needs all four. So the installer, the
//! `gix` resolver, the semantic gate and the `~/.ix` home are built in this
//! file and handed to `dispatch` as a `Capabilities` grant; `ops::modules`
//! decides what to do and never acquires the means to do it. See
//! `quoin_core::capabilities`.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use quoin_core::capabilities::{Capabilities, ModuleHost};
use quoin_core::dispatch::{dispatch, parse_operation, read_request};
use quoin_core::error::CoreError;
use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_modules::{
    GixResolver, InstallOutcome, InstallPaths, InstalledModule, IxHome, MarketplaceManifest,
    ModuleInstaller, ModuleName, ModuleRegistry, ModulesError, ReconcileMode, ReconcileReport,
    RollbackOutcome, SemanticGate, SemanticPin, SemanticVerdict, Source,
};

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(response) => emit(&response),
        Err(error) => emit(&Response {
            payload: serde_json::Value::Null,
            diagnostics: vec![Diagnostic::from(&error)],
            outcome: error.outcome(),
        }),
    }
}

fn run(args: &[String]) -> Result<Response, CoreError> {
    let op = parse_operation(args)?;
    // Bounded, and bounded HERE: `read_request` stops the stream one byte past
    // `protocol::MAX_REQUEST_BYTES` rather than reading whatever arrives and
    // measuring it afterwards. See quoin#448's review — an uncapped
    // `read_to_string` in this function made every operation's own size limit
    // a remark about an allocation that had already happened.
    let request = read_request(std::io::stdin())?;

    // The grant is built once, here, and nothing downstream can widen it.
    let host = HostModules::new();
    let capabilities = Capabilities::with_modules(&host);
    dispatch(op, &request, &capabilities)
}

/// The production [`ModuleHost`]: a `gix` resolver, the semantic gate and the
/// `~/.ix` home, all resolved from the process environment.
///
/// `ops::modules` is handed this and never constructs one. The in-memory host
/// its unit tests use implements the same trait, which is how the refusal,
/// mapping and validation paths are exercised with no disk and no network.
struct HostModules {
    /// `$IX_HOME`, or `<home>/.ix`, resolved once.
    default_home: IxHome,
    /// `$QUOIN_SEMANTIC_ROOT`: the vendored schema tree inside the npm package.
    ///
    /// Supplied by the caller because only the caller knows where its own
    /// package was installed. Absent means the gate cannot judge, and an
    /// install is REFUSED rather than accepted unjudged — see [`ContractGate`].
    semantic_root: Option<PathBuf>,
}

impl HostModules {
    fn new() -> Self {
        Self {
            default_home: IxHome::resolve(
                std::env::var("IX_HOME").ok().as_deref(),
                std::env::home_dir().as_deref(),
            ),
            semantic_root: std::env::var_os("QUOIN_SEMANTIC_ROOT").map(PathBuf::from),
        }
    }

    /// The home a request named, or the one this process resolved.
    fn home(&self, home: Option<&Path>) -> IxHome {
        home.map_or_else(|| self.default_home.clone(), IxHome::new)
    }

    /// Run `work` against an installer for `home`.
    ///
    /// The installer borrows its resolver and gate, so both must outlive it; a
    /// closure is how they get a scope without either becoming a global.
    fn with_installer<T>(
        &self,
        home: Option<&Path>,
        work: impl FnOnce(&ModuleInstaller<'_>) -> Result<T, ModulesError>,
    ) -> Result<T, ModulesError> {
        let home = self.home(home);
        let paths = InstallPaths::for_home(&home);
        let resolver = GixResolver::new(paths.cache_root.clone());
        let gate = ContractGate {
            semantic_root: self.semantic_root.clone(),
            modules_dir: home.modules_dir(),
            registry_path: home.registry_path(),
        };
        work(&ModuleInstaller::new(paths, &resolver, &gate))
    }
}

impl ModuleHost for HostModules {
    fn list(&self, home: Option<&Path>) -> Result<Vec<InstalledModule>, ModulesError> {
        // The closure is NOT redundant, and clippy's suggested method path does
        // not compile: `with_installer` binds its installer to a lifetime local
        // to itself, so `work` is a higher-ranked bound, and a method path is
        // only ever instantiated at one specific lifetime ("implementation of
        // FnOnce is not general enough"). The allow is one line, for one lint,
        // on one call, because the lint is wrong here — not a widening.
        #[allow(
            clippy::redundant_closure_for_method_calls,
            reason = "the suggested method path fails the higher-ranked bound"
        )]
        self.with_installer(home, |installer| installer.list())
    }

    fn install(
        &self,
        home: Option<&Path>,
        source: &Source,
    ) -> Result<InstallOutcome, ModulesError> {
        self.with_installer(home, |installer| installer.install_ad_hoc(source))
    }

    fn remove(&self, home: Option<&Path>, name: &ModuleName) -> Result<(), ModulesError> {
        self.with_installer(home, |installer| installer.remove(name))
    }

    fn ensure_defaults(
        &self,
        home: Option<&Path>,
        manifest: &MarketplaceManifest,
        mode: ReconcileMode,
    ) -> Result<ReconcileReport, ModulesError> {
        self.with_installer(home, |installer| {
            quoin_modules::reconcile::reconcile(manifest, installer, mode)
        })
    }

    fn validate_installed(&self, home: Option<&Path>) -> Result<(), ModulesError> {
        // `validateInstalledSemantics` in `src/plugins.ts`, rule for rule: the
        // module's OWN diagnostics only. The duplicate-package and
        // import-resolution checks are install-time rules about a module
        // joining a population, and re-running them here would fail a home that
        // the install path had already accepted.
        let home = self.home(home);
        // Unjudged is not the same as clean, so an absent or unreadable
        // contract refuses rather than passes — the same choice `ContractGate`
        // makes on the install path, for the same reason.
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
        let modules_dir = home.modules_dir();
        // An unreadable registry is NOT an empty registry. `unwrap_or_default`
        // here would have re-validated a population of zero and reported the
        // home clean, which is the "check over an empty population" this file's
        // own doc above refuses to make.
        let registry = ModuleRegistry::read(&home.registry_path()).map_err(|error| {
            unavailable(format!(
                "the installed-module registry at {} could not be read, so installed \
                 modules cannot be re-validated: {error}",
                home.registry_path().display()
            ))
        })?;
        for installed in registry.plugins {
            let root = modules_dir.join(installed.name.as_str());
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
}

/// The semantic contract, put behind `quoin-modules`' gate seam.
///
/// `quoin-modules` owns *when* the gate runs and what a rejection does to the
/// filesystem and the registry; it deliberately does not own what the gate
/// decides, and its own `PermissiveGate` accepts everything. This is the
/// implementation that makes the Rust install path refuse exactly what
/// `src/plugins.ts` refused: the module's own `semantic` block, a package
/// already claimed by another installed module, unresolvable imports, and a
/// derived package manifest that does not validate.
struct ContractGate {
    semantic_root: Option<PathBuf>,
    modules_dir: PathBuf,
    registry_path: PathBuf,
}

impl ContractGate {
    /// Every OTHER installed module that declares a semantic block, in sorted
    /// root order — the population `duplicate_package_diagnostic` and
    /// `resolve_imports` judge against.
    ///
    /// `installedSemanticModules(home).filter(m => m.name !== installed.name)`
    /// in `src/plugins.ts`, including the sort: the diagnostics name whichever
    /// module is found first, so the order is user-visible.
    ///
    /// # Errors
    /// A description of what could not be read, when the population cannot be
    /// established. This is deliberately not an empty population: the
    /// duplicate-package and import-resolution rules are judgements *against*
    /// this list, so an unreadable registry or an unreadable sibling silently
    /// turns both rules off and passes a module they would have refused.
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
        let mut roots: Vec<PathBuf> = registry
            .plugins
            .iter()
            .filter(|installed| &installed.name != exclude)
            .map(|installed| self.modules_dir.join(installed.name.as_str()))
            .filter(|root| root.join("manifest.yaml").is_file())
            .collect();
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
/// pin is what `quoin-modules` reads as "do not install this".
fn refuse(rule: &str, message: String) -> SemanticVerdict {
    use quoin_modules::semantic::{Diagnostic as GateDiagnostic, Severity as GateSeverity};
    SemanticVerdict {
        diagnostics: vec![GateDiagnostic {
            severity: GateSeverity::Error,
            rule: rule.to_owned(),
            message,
        }],
        pin: None,
    }
}

/// `quoin_semantic`'s diagnostics in the gate's own vocabulary.
fn as_gate_diagnostics(
    diagnostics: &[quoin_semantic::SemanticDiagnostic],
) -> Vec<quoin_modules::semantic::Diagnostic> {
    use quoin_modules::semantic::{Diagnostic as GateDiagnostic, Severity as GateSeverity};
    diagnostics
        .iter()
        .map(|d| GateDiagnostic {
            severity: match d.severity {
                quoin_semantic::Severity::Error => GateSeverity::Error,
                quoin_semantic::Severity::Warning => GateSeverity::Warning,
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

        // No readable manifest is not this gate's rule to enforce:
        // `quoin-modules` already refuses a module root without one, and
        // `src/plugins.ts` left modules with no `semantic` block untouched.
        let Ok(result) = quoin_semantic::read_module_semantic(root, &validators) else {
            return SemanticVerdict::default();
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

/// Write the streams and choose the status.
///
/// A serialisation or write failure downgrades the run to `Internal` (4) and
/// writes NO payload, rather than reporting success over a truncated stream.
fn emit(response: &Response) -> std::process::ExitCode {
    let payload = if response.outcome.carries_payload() {
        match canonical_json(&response.payload) {
            Ok(text) => Some(text),
            Err(error) => return emit_internal(&error),
        }
    } else {
        None
    };
    let diagnostics = match canonical_json(&response.diagnostics) {
        Ok(text) => text,
        Err(error) => return emit_internal(&error),
    };

    if let Some(payload) = payload {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        if writeln!(stdout, "{payload}").is_err() || stdout.flush().is_err() {
            return std::process::ExitCode::from(quoin_core::protocol::Outcome::Internal.code());
        }
    }
    if !response.diagnostics.is_empty() {
        let _ = writeln!(std::io::stderr(), "{diagnostics}");
    }
    std::process::ExitCode::from(response.outcome.code())
}

fn emit_internal(error: &CoreError) -> std::process::ExitCode {
    let _ = writeln!(std::io::stderr(), "[{}] {}", error.code, error.message);
    std::process::ExitCode::from(quoin_core::protocol::Outcome::Internal.code())
}
