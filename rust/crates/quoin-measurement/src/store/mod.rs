// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The measurement store: where collections live, how they are published, how
//! they are read back.
//!
//! Ports `src/measurement/store.ts` and `src/measurement/atomic-file.ts`, which
//! carry **two copies of the same write-once dance** on `origin/main` —
//! `store.ts:35-53` and `atomic-file.ts:12-48` both do exists-check, compare,
//! `wx` temporary, publish, unlink. Neither is ported as written. Both unify on
//! [`quoin_store::store::write_content_addressed`], which is the same
//! contract plus `fsync` (Stage 6 plan §7, the #394 durability decision) and is
//! already the write path every other Rust crate in this workspace uses.
//!
//! That is a **strengthening divergence**, declared in `DIVERGENCE.md`: the
//! retained code can lose a published collection to a crash between `link` and
//! the directory entry reaching disk, and the Rust path cannot.

mod apparatus;
pub mod paths;
pub mod publish;
pub mod read;

pub use apparatus::MAX_PROTECTED_FILES;
pub use paths::{intervention_path, interventions_root, measurement_path, measurements_root};
pub use publish::write_measurement_collection;
pub use read::{
    MeasurementCollectionReadResult, collection_order, read_measurement_collection_results,
    read_measurement_collections,
};
