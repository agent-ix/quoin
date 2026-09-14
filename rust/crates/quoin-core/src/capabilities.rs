// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What `main.rs` grants an operation, and the only way an operation gets it
//! (quoin#446).
//!
//! # The rule this module exists to keep
//!
//! `tests/tc_library_containment.rs` audits every file under `src/` except
//! `main.rs` with `engineering_assurance::source_audit` in the
//! `ReusableLibrary` role, and a `std::fs`, `std::env`, `std::process` or
//! `std::net` path anywhere in the library half is a `ForbiddenCapability`
//! finding. That is not a formality to route around: it is what makes the
//! boundary's decisions unit-testable with no disk, which is the property
//! `src/core/exec.ts` and `quoin-difftest` both rest on.
//!
//! Stage 7 is the first stage where that rule bites, because config and modules
//! are inherently I/O-heavy — two config files, `.git/config`, a JSON registry,
//! a git remote. It is resolved with **one rule in two shapes**, and nothing
//! about the test was weakened, skipped or allow-listed to reach it:
//!
//! 1. **Small, bounded state travels in the request.** The config layer
//!    documents, `.git/config`'s text and the environment map are bytes the
//!    TypeScript caller already has open; `config.resolve_org` decides over
//!    them and touches nothing. `quoin_config::resolve_org_from_documents` is
//!    the filesystem-free twin of `resolve_org`, and the two are pinned against
//!    each other over a real temporary tree by `tc_446_026`, so the pure path
//!    cannot drift away from the one the CLI takes.
//!
//! 2. **A tree too large to serialise, or a network, is a CAPABILITY OBJECT
//!    that `main.rs` constructs and hands in.** A module install fetches a git
//!    remote, materialises a subtree and rewrites a registry; those bytes
//!    cannot ride on stdin. So `ops::modules` never constructs an installer —
//!    it is given a [`ModuleHost`], whose only production implementation lives
//!    in `main.rs` beside the other host wiring. Every unit test in `ops/`
//!    substitutes an in-memory one, which is how the refusal, rollback and
//!    exit-mapping paths are exercised with no disk and no network at all.
//!
//! The invariant is therefore stronger than "the audit passes": **the library
//! half acquires nothing.** It is handed what it may use, and a reader can see
//! the whole grant in one struct.
//!
//! # One pattern, not two
//!
//! Stage 2 (quoin#412) reached the same two shapes for `validators.run`: the
//! repository content rides in the request, and `quoin-validators` grew a
//! `RepoSource` trait with a disk implementation and a pure in-memory one so
//! that one analysis runs over both. Shape 1 above is that, with
//! `resolve_org_from_documents` as the pure source and `tc_446_026` as the
//! equivalence pin. Shape 2 is the same trait seam applied where content-in is
//! not available at all: a module install resolves a remote over the **network**
//! and rewrites a subtree, and neither can ride on stdin, so its production
//! implementation is constructed outside the audited half rather than read into
//! a request first. `ContractGate` — the semantic decision an install is
//! subject to — is built from `quoin-semantic` in the same place, which is why
//! that implementation lives in `main.rs` and not in `quoin-modules`.
//!
//! The trap quoin#412 hit is worth naming because it applies to any pure twin:
//! its first in-memory source skipped the directory exclusions the disk walk
//! applied, so the pure path could analyse a file the real one would never have
//! seen. The defence here is that `tc_446_026` does not hand-write the pure
//! path's inputs — it builds a real tree, runs both paths over it, and reads
//! the documents off that same tree.

use std::path::{Path, PathBuf};

use quoin_catalog::ModuleDocument;
use quoin_change_assurance::EvidenceStore;
use quoin_evidence::EvidenceSource;
use quoin_graph_analysis::GraphInputReader;
use quoin_modules::{InstallOutcome, InstalledModule, MarketplaceManifest, ModuleName, Source};
use quoin_modules::{ModulesError, ReconcileMode, ReconcileReport};
use quoin_semantic::{CorpusRoot, SemanticError, SemanticReadResult, SweepIdentity, SweepReport};

use crate::error::CoreError;

/// Installing, listing and removing spec modules in one `~/.ix` home.
///
/// The capability `ops::modules` is granted rather than acquires. Implemented
/// for real in `main.rs` over `quoin_modules::ModuleInstaller`, a
/// `gix`-backed `GitResolver` and the `quoin-semantic` gate; implemented in
/// memory by every unit test in `ops::modules`.
///
/// `home` is `None` for "the home this process resolves", which only the
/// production implementation can answer — a test passes an explicit one.
pub trait ModuleHost {
    /// Every installed module, in registry order.
    ///
    /// # Errors
    /// Whatever the registry read returns.
    fn list(&self, home: Option<&Path>) -> Result<Vec<InstalledModule>, ModulesError>;

    /// Install one ad-hoc source, rolling back to the previous version on a
    /// semantic-contract rejection.
    ///
    /// # Errors
    /// Any resolution, materialisation, registry or semantic failure.
    fn install(&self, home: Option<&Path>, source: &Source)
    -> Result<InstallOutcome, ModulesError>;

    /// Remove an installed module and its registry record.
    ///
    /// # Errors
    /// [`ModulesError::ModuleNotInstalled`] and any registry-write failure.
    fn remove(&self, home: Option<&Path>, name: &ModuleName) -> Result<(), ModulesError>;

    /// Reconcile the default module set into the home.
    ///
    /// # Errors
    /// The first entry's failure, which stops the reconcile.
    fn ensure_defaults(
        &self,
        home: Option<&Path>,
        manifest: &MarketplaceManifest,
        mode: ReconcileMode,
    ) -> Result<ReconcileReport, ModulesError>;

    /// Re-read every installed module's `semantic` block and refuse on the
    /// first violation.
    ///
    /// Separate from [`ModuleHost::ensure_defaults`] because it judges what is
    /// **already on disk** rather than what a reconcile is about to do, and a
    /// `lazy` reconcile deliberately skips a settled entry without re-reading
    /// it. `src/modules.ts` ran `reconcile(...)` and then
    /// `validateInstalledSemantics(home)` for exactly that reason: a module
    /// that was valid when installed can stop being valid — its manifest edited
    /// by hand, or the vendored contract moved under it — and the reconcile
    /// path is where an author finds out (FR-070-AC-1, NFR-017-AC-1).
    ///
    /// # Errors
    /// [`ModulesError::SemanticContractViolation`] carrying
    /// `QM020_SEMANTIC_CONTRACT_VIOLATION` and
    /// [`quoin_modules::RollbackOutcome::NotAttempted`], since nothing was installed here and
    /// the offending copy is left where it is.
    fn validate_installed(&self, home: Option<&Path>) -> Result<(), ModulesError>;
}

/// Locating module roots and reading their catalog inputs (quoin#373, Stage 8).
///
/// The catalog projection itself is pure and lives in `quoin-catalog`; this
/// trait is deliberately only the filesystem grant that turns candidate paths
/// into [`ModuleDocument`] values. `None` means the host's default candidate
/// roots (`QUOIN_MODULE_PATHS` followed by the installed modules directory),
/// while `Some` is the explicit list a caller supplied for a reproducible
/// catalog view.
pub trait CatalogHost {
    /// Locate candidates, read their manifests, and list skeleton filenames.
    ///
    /// # Errors
    /// The first filesystem failure that prevents an otherwise located module
    /// from being represented.
    fn read_modules(&self, roots: Option<&[PathBuf]>) -> std::io::Result<Vec<ModuleDocument>>;
}

/// Reading the vendored semantic contract, and the trees it judges
/// (quoin#452, Stage 8).
///
/// Granted rather than acquired, for the same reason [`ModuleHost`] is. The
/// contract is a tree of 35 vendored JSON schemas that ships inside the npm
/// package; a module's `data_schema` references resolve against files inside
/// that module; a corpus sweep walks whole repositories. None of that can ride
/// on stdin, and only the CALLER knows where its own package was installed —
/// the path arrives as `QUOIN_SEMANTIC_ROOT`. So `main.rs` holds the root and
/// the compiled validators, and `ops::semantic` decides what to ask and what to
/// report.
///
/// Implemented for real in `main.rs` over `quoin_semantic::SemanticValidators`;
/// implemented in memory by every unit test in `ops::semantic`.
pub trait SemanticHost {
    /// Read and judge one module root's `manifest.yaml` semantic block.
    ///
    /// A module whose block violates the contract is NOT an error: it comes
    /// back as diagnostics inside the result, which is what
    /// `readSemanticBlock` did and what `loadCatalog` reports.
    ///
    /// # Errors
    /// [`SemanticError::ManifestUnreadable`], [`SemanticError::ManifestNotYaml`],
    /// [`SemanticError::ManifestNotAMapping`], [`SemanticError::ContractRootUnset`]
    /// and the `VendoredSchema*` conditions.
    fn read_module(&self, module_root: &Path) -> Result<SemanticReadResult, SemanticError>;

    /// Walk corpus roots and classify every Markdown artifact's Properties form.
    ///
    /// # Errors
    /// [`SemanticError::CorpusUnreadable`] when a root or one of its files
    /// cannot be read.
    fn sweep(
        &self,
        roots: &[CorpusRoot],
        identity: &SweepIdentity,
        generated_at: &str,
    ) -> Result<SweepReport, SemanticError>;
}

/// Where change-assurance evidence is retained (quoin#457, Stage 9).
///
/// Granted rather than acquired, for the same reason [`ModuleHost`] and
/// [`SemanticHost`] are, and with the smallest surface of the three: ONE
/// method, returning the store rooted at the repository the caller named.
/// Everything `ops::change_assurance` decides — that a request is well formed,
/// that a document parses strictly, that a record satisfies FR-063, what a
/// candidate's evidence comes to — is decided with no disk at all, and its unit
/// tests prove it by running the whole domain against
/// `quoin_change_assurance::intake::memory::MemoryEvidenceStore`.
///
/// The seam is the crate's own [`EvidenceStore`] rather than a second trait
/// restating it: `quoin-change-assurance` already states "where evidence is
/// kept" once, addressed only by digest, and a parallel declaration here would
/// be a shape nobody checks against it.
///
/// Implemented for real in `main.rs` over
/// `quoin_change_assurance::intake::disk::DiskEvidenceStore`.
pub trait ChangeAssuranceHost {
    /// The evidence store rooted at `repo`.
    ///
    /// A factory rather than one store held as state: a process answers one
    /// operation and exits, and the repository root is a REQUEST field, so
    /// there is no store to build until an operation says which one.
    fn store<'a>(&'a self, repo: &Path) -> Box<dyn EvidenceStore + 'a>;
}

/// Reading and writing one repository's evidence store (quoin#458, Stage 9).
///
/// Granted rather than acquired, for the same reason [`ModuleHost`] and
/// [`SemanticHost`] are, and for the shape-2 reason in this module's header: an
/// evidence store is a directory tree of run, scan, inspection, trust and
/// assurance records that `gc` walks and that every reader lists. Those bytes
/// cannot ride on stdin.
///
/// The seam is deliberately a **scope**, not a handle. `quoin-evidence`'s store
/// functions take `&mut S where S: EvidenceSource + ?Sized`, so the host opens
/// the store, hands `ops::evidence` a `&mut dyn EvidenceSource` for the length
/// of one action, and closes it again. `ops::evidence` therefore never holds a
/// disk object, never names a path, and every one of its tests substitutes a
/// [`quoin_evidence::MemoryEvidence`] with no temporary directory at all.
///
/// Implemented for real in `main.rs` over [`quoin_evidence::DiskEvidence`].
pub trait EvidenceHost {
    /// Run one action against the store belonging to `repo`.
    ///
    /// The action returns the operation's payload, already serialised, because
    /// the borrow of the store must end before this returns and a payload that
    /// borrowed from it could not outlive the call.
    ///
    /// # Errors
    /// Whatever the action returns.
    fn with_store(
        &self,
        repo: &Path,
        action: &mut dyn FnMut(&mut dyn EvidenceSource) -> Result<serde_json::Value, CoreError>,
    ) -> Result<serde_json::Value, CoreError>;

    /// The absolute directory the store for `repo` is rooted at.
    ///
    /// Load-bearing, not a convenience. `quoin_evidence::store::gc` returns
    /// STORE-RELATIVE paths — correct for a library that names no host
    /// capability, and the opposite of what the retained `gc()` returned. The
    /// caller prints those paths for a human to act on, so the join has to
    /// happen somewhere, and it happens here: the only place that knows where
    /// the store actually is.
    fn store_root(&self, repo: &Path) -> PathBuf;
}

/// Running the quire engine over a repository (quoin#502, Stage 7).
///
/// Granted rather than acquired, for the shape-2 reason in this module's
/// header. A coverage run walks a whole repository's `spec/` tree, its trace
/// tags and every installed module's manifest; a classification run reads each
/// Markdown document it is pointed at. Those bytes cannot ride on stdin, and
/// `ScopeRoot::open` canonicalizes a path — a filesystem call — so even naming
/// the scope is something only the granting half may do.
///
/// The seam takes **plain paths and returns the engine's own outcome**. The
/// projection from a `quire_rs` report to the wire shape is a decision, so it
/// stays in `ops::quire` where it is unit-tested against an in-memory host;
/// the host does the one thing the library half may not, which is touch the
/// disk.
///
/// Implemented for real in `main.rs` over `quoin_quire::coverage::compute` and
/// `quoin_quire::properties::classify`.
pub trait QuireHost {
    /// The coverage report for one scope, under a closed or ambient module set.
    ///
    /// `modules` empty means ambient discovery — the resolution order
    /// `quire coverage` takes with no `--module`, made explicit by
    /// [`quoin_quire::ModuleSelection`].
    ///
    /// # Errors
    /// Whatever the engine returns: a scope that is not a directory, a missing
    /// `spec/` root, a module set that does not load, or no declared
    /// traceability model.
    fn coverage(
        &self,
        scope: &Path,
        modules: &[PathBuf],
    ) -> Result<quoin_quire::coverage::Outcome, quoin_quire::Error>;

    /// Classify every document `documents` names, relative to the scope.
    ///
    /// # Errors
    /// The same resolution failures as [`QuireHost::coverage`]. A document that
    /// resolves to no archetype is **not** one of them: it comes back inside
    /// the outcome, which is the distinction `quire properties`' exit 1 loses.
    fn properties(
        &self,
        scope: &Path,
        modules: &[PathBuf],
        documents: &[String],
    ) -> Result<quoin_quire::properties::Outcome, quoin_quire::Error>;
}

/// Everything `main.rs` grants one dispatch.
///
/// A struct rather than a growing argument list so that adding a capability is
/// a visible, reviewable change to one declaration — and so a reader can see
/// the whole grant in one place rather than inferring it from call sites.
pub struct Capabilities<'a> {
    /// The module-installation capability, absent when nothing granted one.
    ///
    /// `Option` rather than a no-op default: an operation that needs a host and
    /// silently got a stub would report success having done nothing, which is
    /// the failure mode `PermissiveGate`'s doc comment in `quoin-modules` warns
    /// about for the same reason.
    pub modules: Option<&'a dyn ModuleHost>,
    /// The catalog-input capability, absent when nothing granted one.
    pub catalog: Option<&'a dyn CatalogHost>,
    /// The semantic-contract capability, absent when nothing granted one.
    pub semantic: Option<&'a dyn SemanticHost>,
    /// The change-assurance evidence store, absent when nothing granted one.
    pub change_assurance: Option<&'a dyn ChangeAssuranceHost>,
    /// The evidence-store capability, absent when nothing granted one.
    pub evidence: Option<&'a dyn EvidenceHost>,
    /// The graph-analysis input reader, absent when nothing granted one.
    ///
    /// The seam is `quoin-graph-analysis`' own [`GraphInputReader`] rather than
    /// a second trait restating it, for the reason [`ChangeAssuranceHost`]
    /// gives about [`EvidenceStore`]: that crate already says "where the four
    /// declared inputs come from" once, with one method, and a parallel
    /// declaration here would be a shape nobody checks against it.
    pub graph: Option<&'a dyn GraphInputReader>,
    /// The quire engine, absent when nothing granted one.
    pub quire: Option<&'a dyn QuireHost>,
}

impl<'a> Capabilities<'a> {
    /// A grant of nothing.
    ///
    /// What the purely decidable operations — `core.ping`, every `assurance.*`,
    /// `config.resolve_org` — run with, and what their tests pass.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            modules: None,
            catalog: None,
            semantic: None,
            change_assurance: None,
            evidence: None,
            graph: None,
            quire: None,
        }
    }

    // Each single grant is `none()` with one field set, rather than a struct
    // literal listing every capability. Six literals restating all of them is
    // how a seventh capability becomes seven edits in one file, and how one of
    // those edits silently grants a host a test did not mean to grant.
    // `const` is given up to say it once; nothing constructs these at compile
    // time.

    /// A grant of the module host only.
    #[must_use]
    pub fn with_modules(host: &'a dyn ModuleHost) -> Self {
        Self {
            modules: Some(host),
            ..Self::none()
        }
    }

    /// A grant of the catalog host only.
    #[must_use]
    pub fn with_catalog(host: &'a dyn CatalogHost) -> Self {
        Self {
            catalog: Some(host),
            ..Self::none()
        }
    }

    /// A grant of the semantic host only.
    #[must_use]
    pub fn with_semantic(host: &'a dyn SemanticHost) -> Self {
        Self {
            semantic: Some(host),
            ..Self::none()
        }
    }

    /// A grant of the change-assurance store only.
    #[must_use]
    pub fn with_change_assurance(host: &'a dyn ChangeAssuranceHost) -> Self {
        Self {
            change_assurance: Some(host),
            ..Self::none()
        }
    }

    /// A grant of the evidence host only.
    #[must_use]
    pub fn with_evidence(host: &'a dyn EvidenceHost) -> Self {
        Self {
            evidence: Some(host),
            ..Self::none()
        }
    }

    /// A grant of the graph input reader only.
    #[must_use]
    pub fn with_graph(reader: &'a dyn GraphInputReader) -> Self {
        Self {
            graph: Some(reader),
            ..Self::none()
        }
    }

    /// A grant of the quire engine only.
    #[must_use]
    pub fn with_quire(host: &'a dyn QuireHost) -> Self {
        Self {
            quire: Some(host),
            ..Self::none()
        }
    }

    /// The full grant `main.rs` hands one dispatch.
    ///
    /// The one place that names every capability, deliberately: adding one
    /// must fail to compile here until `main.rs` decides what to pass.
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "the explicit argument list is the audited capability census; a bag would hide a grant"
    )]
    pub const fn with_hosts(
        modules: &'a dyn ModuleHost,
        catalog: &'a dyn CatalogHost,
        semantic: &'a dyn SemanticHost,
        change_assurance: &'a dyn ChangeAssuranceHost,
        evidence: &'a dyn EvidenceHost,
        graph: &'a dyn GraphInputReader,
        quire: &'a dyn QuireHost,
    ) -> Self {
        Self {
            modules: Some(modules),
            catalog: Some(catalog),
            semantic: Some(semantic),
            change_assurance: Some(change_assurance),
            evidence: Some(evidence),
            graph: Some(graph),
            quire: Some(quire),
        }
    }
}

impl Default for Capabilities<'_> {
    fn default() -> Self {
        Self::none()
    }
}
