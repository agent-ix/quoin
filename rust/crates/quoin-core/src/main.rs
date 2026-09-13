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
use std::path::{Path, PathBuf};

use quoin_change_assurance::EvidenceStore;
use quoin_change_assurance::intake::disk::DiskEvidenceStore;
use quoin_core::capabilities::{
    Capabilities, ChangeAssuranceHost, EvidenceHost, ModuleHost, SemanticHost,
};
use quoin_core::dispatch::{dispatch, parse_operation, read_request};
use quoin_core::error::CoreError;
use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_evidence::{DiskEvidence, EvidenceSource};
use quoin_modules::{
    ContractGate, GixResolver, InstallOutcome, InstallPaths, InstalledModule, IxHome,
    MarketplaceManifest, ModuleInstaller, ModuleName, ModulesError, ReconcileMode, ReconcileReport,
    Source,
};
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
    let semantic = HostSemantic::new();
    let change_assurance = HostChangeAssurance;
    let evidence = HostEvidence;
    let capabilities = Capabilities::with_hosts(&modules, &semantic, &change_assurance, &evidence);
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
