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

use quoin_change_assurance::EvidenceStore;
use quoin_evidence::EvidenceSource;
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
    /// The semantic-contract capability, absent when nothing granted one.
    pub semantic: Option<&'a dyn SemanticHost>,
    /// The change-assurance evidence store, absent when nothing granted one.
    pub change_assurance: Option<&'a dyn ChangeAssuranceHost>,
    /// The evidence-store capability, absent when nothing granted one.
    pub evidence: Option<&'a dyn EvidenceHost>,
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
            semantic: None,
            change_assurance: None,
            evidence: None,
        }
    }

    /// A grant of the module host only.
    #[must_use]
    pub const fn with_modules(host: &'a dyn ModuleHost) -> Self {
        Self {
            modules: Some(host),
            semantic: None,
            change_assurance: None,
            evidence: None,
        }
    }

    /// A grant of the semantic host only.
    #[must_use]
    pub const fn with_semantic(host: &'a dyn SemanticHost) -> Self {
        Self {
            modules: None,
            semantic: Some(host),
            change_assurance: None,
            evidence: None,
        }
    }

    /// A grant of the change-assurance store only.
    #[must_use]
    pub const fn with_change_assurance(host: &'a dyn ChangeAssuranceHost) -> Self {
        Self {
            modules: None,
            semantic: None,
            change_assurance: Some(host),
            evidence: None,
        }
    }

    /// A grant of the evidence host only.
    #[must_use]
    pub const fn with_evidence(host: &'a dyn EvidenceHost) -> Self {
        Self {
            modules: None,
            semantic: None,
            change_assurance: None,
            evidence: Some(host),
        }
    }

    /// The full grant `main.rs` hands one dispatch.
    #[must_use]
    pub const fn with_hosts(
        modules: &'a dyn ModuleHost,
        semantic: &'a dyn SemanticHost,
        change_assurance: &'a dyn ChangeAssuranceHost,
        evidence: &'a dyn EvidenceHost,
    ) -> Self {
        Self {
            modules: Some(modules),
            semantic: Some(semantic),
            change_assurance: Some(change_assurance),
            evidence: Some(evidence),
        }
    }
}

impl Default for Capabilities<'_> {
    fn default() -> Self {
        Self::none()
    }
}
