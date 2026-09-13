// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

//! Quoin's spec-module install registry, git-backed sources and default-set
//! reconciliation (quoin#381, Stage 7 of the EPIC #373 Rust burn-down).
//!
//! The Rust successor to `src/plugins.ts` and `src/modules.ts`, and to the
//! `@agent-ix/ts-plugin-kit` surface they use. `ts-plugin-kit` shells out to the
//! `git` binary; this crate uses `gix` in-process.
//!
//! # Layout
//!
//! | Module | Owns |
//! |---|---|
//! | [`source`] | Typed source descriptors and CLI argument parsing |
//! | [`manifest`] | The committed default module set |
//! | [`registry`] | `~/.ix/filament/registry.json`, wire-compatible with ts-plugin-kit |
//! | [`git`] | `gix`-backed fetch and tree extraction, with bounds |
//! | [`install`] | Installing one entry, with rollback |
//! | [`reconcile`] | Lazy and sync reconciliation of the default set |
//! | [`semantic`] | The semantic-contract seam Stage 3 fills |
//!
//! # Blocking, by design
//!
//! See [`git`] — the fetch is blocking with an explicit wall-clock budget
//! rather than async with cancellation, because `quoin-core` is a one-shot
//! subprocess with nothing to overlap.

pub mod error;
pub mod git;
pub mod ids;
pub mod install;
pub mod manifest;
pub mod module_name;
pub mod paths;
pub mod reconcile;
pub mod registry;
pub mod semantic;
pub mod source;

pub use error::{ModulesError, ModulesErrorCode, RollbackOutcome};
pub use git::{GitLimits, GitResolver, GixResolver, ResolvedGitSource, Revision};
pub use ids::{CommitSha, ModuleName};
pub use install::{InstallOutcome, ModuleInstaller};
pub use manifest::{MarketplaceEntry, MarketplaceManifest};
pub use module_name::read_module_name;
pub use paths::{InstallPaths, IxHome};
pub use reconcile::{ReconcileMode, ReconcileReport};
pub use registry::{InstalledModule, ModuleRegistry};
pub use semantic::{PermissiveGate, SemanticGate, SemanticPin, SemanticVerdict};
pub use source::{Source, parse_source_arg, to_git_url};
