// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
#![forbid(unsafe_code)]
//! Measurement plans, collections, comparisons, assurance profiles and
//! raw-evidence accounting — quoin FR-100, ported from `src/measurement/`.
//!
//! # What is here
//!
//! Wave 1 of Stage 6 (quoin#468): the eight leaf modules of the measurement
//! domain that depend on nothing else in it, plus the raw-evidence accounting
//! lifted out of `intervention.ts`. Wave 2 (quoin#469): the intervention and
//! operational record declarations and the two pure report renderers over
//! them.
//!
//! | retained TypeScript | here |
//! | --- | --- |
//! | `types.ts` | [`types`] |
//! | `compare.ts` | [`compare`] |
//! | `validate.ts` | [`validate`] |
//! | `date-time.ts` | [`date_time`] |
//! | `plans.ts` | [`plans`] |
//! | `profiles.ts` | [`profiles`] |
//! | `store.ts`, `atomic-file.ts` | [`store`] |
//! | `intervention.ts:115-216` | [`raw_evidence`] |
//! | `intervention-types.ts` | [`intervention::record`] |
//! | `intervention-report.ts` | [`intervention::report`] |
//! | `operational-types.ts` | [`operational::record`] |
//! | `operational-report.ts` | [`operational::report`] |
//!
//! [`common`] holds what those four files declare more than once — the
//! subject, the producer, the scalar unions — declared once here.
//!
//! The TypeScript is retained, not deleted: this is a port wave and the
//! cutover is quoin#479. Intake and the producers are quoin#471 and
//! quoin#472.
//!
//! # What is deliberately absent
//!
//! - **No canonicalizer, no JSON parser, no digest function, no atomic write.**
//!   All four are [`quoin_store`]'s, and `tc_468_boundary` asserts over this
//!   crate's own sources that no second one appeared (FR-100-CON-4).
//! - **No second YAML reader.** Frontmatter is [`quoin_yaml`]'s.
//! - **No seventh instant validator.** [`date_time`] is the one RFC 3339
//!   grammar in this crate, and it is the permissive one, per the owner ruling
//!   in the Stage 6 plan §5.
//! - **No filesystem outside [`source`] and [`store`].** Everything else takes
//!   a [`source::MeasurementSource`].

pub mod common;
pub mod compare;
pub mod date_time;
pub mod discovery;
pub mod error;
pub mod intervention;
pub mod operational;
pub mod plans;
pub mod profiles;
pub mod raw_evidence;
pub mod source;
pub mod store;
pub mod types;
pub mod validate;

pub use compare::compare_measurement_collections;
pub use date_time::Rfc3339DateTime;
pub use error::{MeasurementError, MeasurementErrorCode};
pub use plans::{PlanLoadOptions, load_measurement_plans};
pub use profiles::load_active_assurance_profiles;
pub use raw_evidence::{
    RawEvidenceClaim, RawEvidencePath, RawEvidenceReference, assert_governing_definition,
    raw_evidence_for, verify_raw_evidence_references,
};
pub use source::{DiskMeasurement, MeasurementSource, MemoryMeasurement, RawEvidenceFile};
pub use store::{
    MeasurementCollectionReadResult, measurement_path, measurements_root,
    read_measurement_collection_results, read_measurement_collections,
    write_measurement_collection,
};
pub use types::collection::{MEASUREMENT_SCHEMA_VERSION, MeasurementCollection};
pub use types::comparison::MeasurementComparison;
pub use types::observation::MeasurementObservation;
pub use types::plan::MeasurementPlan;
pub use types::profile::AssuranceProfileSummary;
pub use validate::{measurement_collection, stored_measurement_collection};
