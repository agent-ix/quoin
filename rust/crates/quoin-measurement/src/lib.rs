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
//! them. Wave 4 (quoin#471): intervention record-id encoding, schema and
//! semantic validation, intake, and the agent-eval producer. Wave 5
//! (quoin#472): operational intake, the store-wide write lock, the discharge
//! question and the GitHub-release producer.
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
//! | `operational.ts` | [`operational`]'s nine other modules |
//! | `github-release-operational.ts` | [`operational::github_release`] |
//! | `intervention.ts` | [`intervention::ids`], [`intervention::validate`], [`intervention::semantics`], [`intervention::intake`] |
//! | `agent-eval-intervention.ts` | [`intervention::agent_eval`] |
//!
//! [`common`] holds what those four files declare more than once — the
//! subject, the producer, the scalar unions — declared once here.
//!
//! The TypeScript is retained, not deleted: this is a port wave and the
//! cutover is quoin#479. The operational producer is quoin#472 and
//! intervention intake is quoin#471.
//!
//! [`json_bridge`] is the one crossing between [`quoin_store::JsonValue`] —
//! the store's value, with ECMAScript number semantics and the canonical
//! writers — and [`serde_json::Value`], which is what the schema validator and
//! the record types speak. It decides nothing: one direction goes through the
//! store's own writer and the other is a structural walk. Both waves needed
//! it; there is one.
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
pub mod json_bridge;
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
pub use intervention::ids::{
    InterventionRecordBasename, InterventionRecordId, MAX_RECORD_ID_BYTES, RecordIdNamespace,
};
pub use intervention::intake::{
    InterventionIntakeError, InterventionRefusalCode, read_intervention_records,
    write_intervention_record,
};
pub use intervention::validate::validate_intervention_record;
pub use plans::{PlanLoadOptions, load_measurement_plans};
pub use profiles::load_active_assurance_profiles;
pub use raw_evidence::{
    RawEvidencePath, RawEvidenceReference, assert_governing_definition, raw_evidence_for,
    verify_raw_evidence_references,
};
pub use source::{DiskMeasurement, MeasurementSource, MemoryMeasurement, RawEvidenceFile};
pub use store::{
    MeasurementCollectionReadResult, intervention_path, interventions_root, measurement_path,
    measurements_root, read_measurement_collection_results, read_measurement_collections,
    write_measurement_collection,
};
pub use types::collection::{MEASUREMENT_SCHEMA_VERSION, MeasurementCollection};
pub use types::comparison::MeasurementComparison;
pub use types::observation::MeasurementObservation;
pub use types::plan::MeasurementPlan;
pub use types::profile::AssuranceProfileSummary;
pub use validate::{measurement_collection, stored_measurement_collection};
