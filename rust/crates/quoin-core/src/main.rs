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
//!
//! **Construction is not policy, and the difference is the whole point.** What
//! the semantic gate DECIDES is `quoin_modules::ContractGate`, in a domain
//! crate, with unit tests against a real temporary home. It lived here once, on
//! the argument that no library could hold a gate that must read the
//! filesystem; the containment audit walks one crate, so that argument was only
//! ever true of `quoin-core`'s own library. The price of it being here was that
//! ~230 lines of decidable policy sat in the one file no audit walks and no
//! unit test reaches, which is how a filter that silently emptied the
//! population the duplicate-package rule judges against passed every gate
//! (quoin#450 review, findings 2 and 3).

use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use quoin_catalog::ModuleDocument;
use quoin_change_assurance::EvidenceStore;
use quoin_change_assurance::intake::disk::DiskEvidenceStore;
use quoin_core::capabilities::{
    Capabilities, CatalogHost, ChangeAssuranceHost, EvidenceHost, ModuleHost, QuireHost,
    SemanticHost,
};
use quoin_core::dispatch::{dispatch, parse_operation, read_request};
use quoin_core::error::CoreError;
use quoin_core::protocol::{Diagnostic, Response, canonical_json};
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
    let modules = HostModules::new();
    let catalog = HostCatalog::new();
    let semantic = HostSemantic::new();
    let change_assurance = HostChangeAssurance;
    let evidence = HostEvidence;
    // The production graph reader is `quoin-graph-analysis`' own four-line
    // `std::fs` implementation, named here rather than re-implemented: the
    // crate states what it reads, and this file states only that production
    // reads it from the real filesystem.
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
    dispatch(op, &request, &capabilities)
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
}

impl HostCatalog {
    fn new() -> Self {
        let default_home = IxHome::resolve(
            std::env::var("IX_HOME").ok().as_deref(),
            std::env::home_dir().as_deref(),
        );
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
        let gate = self.gate(&home);
        work(&ModuleInstaller::new(paths, &resolver, &gate))
    }

    /// The production semantic gate for `home`.
    ///
    /// The rules it enforces are `quoin_modules::ContractGate`'s and are unit
    /// tested there. This file only says WHERE the vendored contract is, which
    /// is the one part of it that is host state.
    fn gate(&self, home: &IxHome) -> ContractGate {
        ContractGate::for_home(home, self.semantic_root.clone())
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
        self.gate(&self.home(home)).validate_installed()
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
    /// package was installed. Absent means nothing can be judged, and the
    /// answer is [`SemanticError::ContractRootUnset`] rather than a clean read
    /// of an unjudged module.
    semantic_root: Option<PathBuf>,
    /// The compiled validators, built on first use.
    validators: std::cell::OnceCell<SemanticValidators>,
}

impl HostSemantic {
    fn new() -> Self {
        Self {
            semantic_root: std::env::var_os("QUOIN_SEMANTIC_ROOT").map(PathBuf::from),
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
        let root = self
            .semantic_root
            .as_deref()
            .ok_or(SemanticError::ContractRootUnset)?;
        let loaded = SemanticValidators::load(root)?;
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
