// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Production host-capability construction for the native Quoin runtime.
//!
//! This module is deliberately the sole I/O-bearing portion of quoin-core's
//! library surface. The domain engine remains capability-only; the binary and
//! the native CLI both call dispatch with one explicit runtime setting set.

use std::path::{Component, Path, PathBuf};

use quoin_auditor::catalog::load_method_catalog_from;
use quoin_auditor::{
    DiskModuleCatalogSource, ModuleRoot as AuditorModuleRoot, load_method_catalog,
};
use quoin_catalog::ModuleDocument;
use quoin_change_assurance::{EvidenceStore, intake::disk::DiskEvidenceStore};
use quoin_evidence::{DiskEvidence, EvidenceSource};
use quoin_graph_analysis::OsGraphInputReader;
use quoin_modules::{
    ContractGate, GixResolver, InstallOutcome, InstallPaths, InstalledModule, IxHome,
    MarketplaceManifest, ModuleInstaller, ModuleName, ModulesError, ReconcileMode, ReconcileReport,
    Source,
};
use quoin_quire::{ModuleRoot, ModuleSelection, ScopeRoot};
use quoin_semantic::{
    CorpusRoot, SemanticError, SemanticReadResult, SweepIdentity, SweepReport,
    manifest::SemanticValidators,
};

use crate::capabilities::{
    Capabilities, CatalogHost, ChangeAssuranceHost, EvidenceHost, ModuleHost, QuireHost,
    SemanticHost,
};
use crate::dispatch::dispatch as dispatch_request;
use crate::error::CoreError;
use crate::protocol::Response;

/// Explicit host settings for one in-process command invocation.
#[derive(Clone, Debug, Default)]
pub struct RuntimeSettings {
    /// Overrides the IX home discovered from the environment.
    pub ix_home: Option<PathBuf>,
    /// Overrides the vendored semantic-contract root discovered from the environment.
    pub semantic_root: Option<PathBuf>,
}

/// Dispatch one parsed protocol request with production filesystem capabilities.
///
/// The settings object is data, not a process-global mutation, so an embedded
/// caller cannot leak one command's config-root into another.
///
/// # Errors
///
/// Returns the operation's typed refusal or internal error without altering
/// its protocol taxonomy.
pub fn dispatch(
    operation: &str,
    request: &serde_json::Value,
    settings: &RuntimeSettings,
) -> Result<Response, CoreError> {
    let modules = HostModules::new(settings);
    let catalog = HostCatalog::new(settings);
    let semantic = HostSemantic::new(settings);
    let change_assurance = HostChangeAssurance;
    let evidence = HostEvidence;
    let graph = OsGraphInputReader;
    let quire = HostQuire;
    let capabilities = Capabilities::with_hosts(
        &modules,
        &catalog,
        &semantic,
        &change_assurance,
        &evidence,
        &graph,
        &quire,
    );
    dispatch_request(operation, request, &capabilities)
}

/// The production [`CatalogHost`]: explicit or default candidate discovery and
/// the bounded filesystem reads the pure `quoin-catalog` projection needs.
///
/// Discovery reproduces `src/module-roots.ts`: `$QUOIN_MODULE_PATHS` leads the
/// candidate sequence, installed `$IX_HOME/filament/modules/*` follows, a
/// candidate may be a module root or a directory containing roots, and missing
/// candidates are ignored. The projection itself does not see `std::fs`.
struct HostCatalog {
    /// `$IX_HOME`, or `<home>/.ix`, resolved once for default discovery.
    default_home: IxHome,
    /// Explicit module candidates supplied by the process environment.
    env_roots: Vec<PathBuf>,
    /// Original module search path for the method-catalog source. The source
    /// owns this compatibility grammar; the artifact catalog owns its own.
    module_paths: Option<String>,
}

impl HostCatalog {
    fn new(settings: &RuntimeSettings) -> Self {
        let default_home = settings.ix_home.clone().map_or_else(
            || {
                IxHome::resolve(
                    std::env::var("IX_HOME").ok().as_deref(),
                    std::env::home_dir().as_deref(),
                )
            },
            IxHome::new,
        );
        let module_paths = std::env::var("QUOIN_MODULE_PATHS")
            .ok()
            .filter(|value| !value.is_empty());
        let env_roots = std::env::var_os("QUOIN_MODULE_PATHS")
            .map(|value| {
                std::env::split_paths(&value)
                    .filter(|path| !path.as_os_str().is_empty())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            default_home,
            env_roots,
            module_paths,
        }
    }

    fn default_candidates(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut candidates = self.env_roots.clone();
        let installed = self.default_home.as_path().join("filament").join("modules");
        if installed.exists() {
            for entry in std::fs::read_dir(installed)? {
                candidates.push(entry?.path());
            }
        }
        Ok(candidates)
    }

    fn locate(candidate: &Path) -> std::io::Result<Option<PathBuf>> {
        let candidate = lexical_absolute(candidate)?;
        if !candidate.exists() {
            return Ok(None);
        }
        if candidate.join("manifest.yaml").exists() {
            return Ok(Some(candidate));
        }
        if !candidate.is_dir() {
            return Ok(None);
        }
        for child in std::fs::read_dir(candidate)? {
            let child = child?.path();
            if child.join("manifest.yaml").exists() {
                return Ok(Some(child));
            }
        }
        Ok(None)
    }
}

/// Make a candidate absolute while retaining its lexical identity for the
/// catalog's visible paths. In particular, do not canonicalize symlinks: the
/// TypeScript compatibility surface exposed the supplied spelling.
fn lexical_absolute(candidate: &Path) -> std::io::Result<PathBuf> {
    let candidate = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        std::env::current_dir()?.join(candidate)
    };
    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

impl CatalogHost for HostCatalog {
    fn read_modules(&self, roots: Option<&[PathBuf]>) -> std::io::Result<Vec<ModuleDocument>> {
        let candidates =
            roots.map_or_else(|| self.default_candidates(), |roots| Ok(roots.to_vec()))?;
        let mut documents = Vec::new();
        for candidate in candidates {
            let Some(root) = Self::locate(&candidate)? else {
                continue;
            };
            let manifest = std::fs::read_to_string(root.join("manifest.yaml"))?;
            let skeletons = root.join("skeletons");
            let skeleton_names = std::fs::read_dir(skeletons).map_or_else(
                |_| Vec::new(),
                |entries| {
                    entries
                        .filter_map(Result::ok)
                        .filter_map(|entry| entry.file_name().into_string().ok())
                        .collect()
                },
            );
            documents.push(ModuleDocument {
                root: root.to_string_lossy().into_owned(),
                manifest,
                skeleton_names,
            });
        }
        Ok(documents)
    }

    fn load_method_catalog(&self, roots: Option<&[PathBuf]>) -> quoin_auditor::MethodCatalog {
        let source =
            DiskModuleCatalogSource::new(self.default_home.as_path(), self.module_paths.clone());
        roots.map_or_else(
            || load_method_catalog(&source),
            |roots| {
                let candidates = roots
                    .iter()
                    .map(|root| AuditorModuleRoot::candidate(root.to_string_lossy()))
                    .collect::<Vec<_>>();
                load_method_catalog_from(&source, &candidates)
            },
        )
    }
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
    fn new(settings: &RuntimeSettings) -> Self {
        Self {
            default_home: settings.ix_home.clone().map_or_else(
                || {
                    IxHome::resolve(
                        std::env::var("IX_HOME").ok().as_deref(),
                        std::env::home_dir().as_deref(),
                    )
                },
                IxHome::new,
            ),
            semantic_root: settings
                .semantic_root
                .clone()
                .or_else(|| std::env::var_os("QUOIN_SEMANTIC_ROOT").map(PathBuf::from)),
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
        let gate = self.gate(&home)?;
        work(&ModuleInstaller::new(paths, &resolver, &gate))
    }

    /// The production semantic gate for `home`.
    ///
    /// The rules it enforces are `quoin_modules::ContractGate`'s and are unit
    /// tested there. This file only says WHERE the vendored contract is, which
    /// is the one part of it that is host state.
    fn gate(&self, home: &IxHome) -> Result<ContractGate, ModulesError> {
        let semantic_root = self.semantic_root.clone().map_or_else(
            || {
                let root = home.as_path().join("cache/quoin-semantic/v1");
                quoin_semantic::materialize_embedded_contract(&root).map_err(|error| {
                    ModulesError::SemanticContractUnavailable {
                        detail: format!(
                            "the embedded semantic contract could not be prepared: {error}"
                        ),
                    }
                })?;
                Ok(root)
            },
            Ok,
        )?;
        Ok(ContractGate::for_home(home, Some(semantic_root)))
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
        self.gate(&self.home(home))?.validate_installed()
    }
}

/// The production [`SemanticHost`]: the vendored contract tree, resolved from
/// the process environment (quoin#452).
///
/// `ops::semantic` is handed this and never constructs one. The in-memory host
/// its unit tests use implements the same trait, which is how the refusal,
/// mapping and payload paths are exercised with no disk at all.
///
/// The validators are compiled **per invocation and at most once**: compiling
/// the two vendored schemas is the expensive half, `read_blocks` asks about
/// every installed module in one call, and a process answers one operation and
/// exits. `OnceCell` rather than eager construction so that
/// `semantic.migration_example` — which needs no contract at all — does not pay
/// for one, and so a missing `QUOIN_SEMANTIC_ROOT` is reported by the
/// operation that needed it rather than at start-up.
struct HostSemantic {
    /// `$QUOIN_SEMANTIC_ROOT`: the vendored schema tree inside the npm package.
    ///
    /// Supplied by the caller because only the caller knows where its own
    /// package was installed. Absent falls back to the embedded contract:
    /// [`Self::validators`] materialises `quoin-semantic`'s embedded contract
    /// (designed in quoin#527) under `default_home`'s cache directory (wired
    /// into this boundary in quoin#539) — a release executable owns its own
    /// contract and must not depend on this variable being set.
    semantic_root: Option<PathBuf>,
    /// `$IX_HOME`, used for the self-contained native contract cache.
    default_home: IxHome,
    /// The compiled validators, built on first use.
    validators: std::cell::OnceCell<SemanticValidators>,
}

impl HostSemantic {
    fn new(settings: &RuntimeSettings) -> Self {
        Self {
            default_home: settings.ix_home.clone().map_or_else(
                || {
                    IxHome::resolve(
                        std::env::var("IX_HOME").ok().as_deref(),
                        std::env::home_dir().as_deref(),
                    )
                },
                IxHome::new,
            ),
            semantic_root: settings
                .semantic_root
                .clone()
                .or_else(|| std::env::var_os("QUOIN_SEMANTIC_ROOT").map(PathBuf::from)),
            validators: std::cell::OnceCell::new(),
        }
    }

    /// The compiled validators, or the refusal of having no contract.
    ///
    /// `get_or_init` cannot carry a failure, so the fallible load is written
    /// out: a failed compile is NOT cached as a success, and a second call
    /// re-reports the same condition rather than reporting a poisoned cell.
    fn validators(&self) -> Result<&SemanticValidators, SemanticError> {
        if let Some(ready) = self.validators.get() {
            return Ok(ready);
        }
        let root = self.semantic_root.clone().map_or_else(
            || {
                let root = self.default_home.as_path().join("cache/quoin-semantic/v1");
                quoin_semantic::materialize_embedded_contract(&root)?;
                Ok(root)
            },
            Ok,
        )?;
        let loaded = SemanticValidators::load(&root)?;
        Ok(self.validators.get_or_init(|| loaded))
    }
}

impl SemanticHost for HostSemantic {
    fn read_module(&self, module_root: &Path) -> Result<SemanticReadResult, SemanticError> {
        quoin_semantic::read_module_semantic(module_root, self.validators()?)
    }

    fn sweep(
        &self,
        roots: &[CorpusRoot],
        identity: &SweepIdentity,
        generated_at: &str,
    ) -> Result<SweepReport, SemanticError> {
        // No validator is needed to CLASSIFY: the sweep reads Markdown as text
        // and never validates it against a schema. Asking for one here would
        // make `quoin semantic sweep` require a vendored contract it does not
        // consult.
        quoin_semantic::sweep_corpus(roots, identity, generated_at)
    }
}

/// The production [`ChangeAssuranceHost`]: the evidence store on disk
/// (quoin#457).
///
/// It holds no state at all. The repository root is a REQUEST field — `--repo`
/// on every `quoin change-assurance` command — so there is nothing to resolve
/// from the environment and nothing to cache between operations; a process
/// answers one operation and exits. The whole host is the one line that says
/// which implementation of `EvidenceStore` production uses.
struct HostChangeAssurance;

impl ChangeAssuranceHost for HostChangeAssurance {
    fn store<'a>(&'a self, repo: &Path) -> Box<dyn EvidenceStore + 'a> {
        Box::new(DiskEvidenceStore::new(repo))
    }
}

/// The production [`EvidenceHost`]: the evidence store on disk, rooted at
/// `<repo>/spec/evidence` (quoin#458).
///
/// Stateless, because there is nothing to resolve from the environment: the
/// repository root arrives in the request, and the store's own location under
/// it is `quoin_store`'s to decide. `ops::evidence` is handed this and never
/// constructs one; its unit tests substitute a `MemoryEvidence`-backed host and
/// touch no disk at all.
///
/// The store is opened for the length of ONE action and dropped. A handle that
/// outlived the operation would be a second way to reach the filesystem, and
/// the seam exists precisely so there is only the one.
struct HostEvidence;

impl EvidenceHost for HostEvidence {
    fn with_store(
        &self,
        repo: &Path,
        action: &mut dyn FnMut(&mut dyn EvidenceSource) -> Result<serde_json::Value, CoreError>,
    ) -> Result<serde_json::Value, CoreError> {
        let mut store = DiskEvidence::new(repo);
        action(&mut store)
    }

    fn store_root(&self, repo: &Path) -> PathBuf {
        DiskEvidence::new(repo).root().to_path_buf()
    }
}

/// The production [`QuireHost`]: the linked engine, walking the real
/// repository (quoin#502).
///
/// Stateless. Everything a run needs — the scope, the module roots, the
/// documents — arrives in the request, so there is nothing to resolve from the
/// environment and nothing to cache between operations. What this host adds is
/// the three filesystem acts `ops::quire` may not perform: canonicalizing the
/// scope, canonicalizing each module root, and — when the caller named no
/// documents — the `spec/**/*.md` walk that `quoin advise` used to pass as a
/// glob for `quire` to expand.
struct HostQuire;

impl HostQuire {
    /// The module set a request's roots name.
    ///
    /// An empty list is **not** an empty set: it is `ScopeOrAmbient`, the
    /// resolution `quire coverage` performs with no `--module`, which is what
    /// all seven retained commands relied on. `Closed` replaces ambient
    /// discovery rather than adding to it (quire-rs#405).
    fn selection(modules: &[PathBuf]) -> Result<ModuleSelection, quoin_quire::Error> {
        if modules.is_empty() {
            return Ok(ModuleSelection::ScopeOrAmbient);
        }
        modules
            .iter()
            .map(ModuleRoot::open)
            .collect::<Result<Vec<_>, _>>()
            .map(ModuleSelection::Closed)
    }
}

impl QuireHost for HostQuire {
    fn coverage(
        &self,
        scope: &Path,
        modules: &[PathBuf],
    ) -> Result<quoin_quire::coverage::Outcome, quoin_quire::Error> {
        quoin_quire::coverage::compute(&quoin_quire::coverage::Request {
            scope: ScopeRoot::open(scope)?,
            modules: Self::selection(modules)?,
        })
    }

    fn properties(
        &self,
        scope: &Path,
        modules: &[PathBuf],
        documents: &[String],
    ) -> Result<quoin_quire::properties::Outcome, quoin_quire::Error> {
        let scope = ScopeRoot::open(scope)?;
        // The engine classifies a NAMED list; `spec/**/*.md` was a glob the
        // shell expanded for the subprocess. An empty list asks for the same
        // set that glob named, and the walk that answers it is a filesystem
        // act, which is why it is here and not in `ops::quire`.
        let named = if documents.is_empty() {
            quoin_quire::properties::documents_under_spec(&scope)?
        } else {
            documents.iter().map(PathBuf::from).collect()
        };
        quoin_quire::properties::classify(&quoin_quire::properties::Request {
            scope,
            modules: Self::selection(modules)?,
            documents: named,
            // Never overridden: the retained `propertyShapes` passed no
            // `--archetype`, and reading frontmatter `type` is what makes an
            // untyped asset an `Unresolved` rather than a hard failure.
            archetype: None,
        })
    }
}
