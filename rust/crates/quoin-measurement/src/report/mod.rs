// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The per-repository measurement report: what is measured, what the latest
//! evidence says, what needs attention, and how two collections compare.
//!
//! Ports `src/measurement/report.ts`.
//!
//! | retained TypeScript | here |
//! | --- | --- |
//! | `report.ts:24-103` | [`build`] |
//! | `report.ts:105-200` | [`render`] |
//! | `report.ts:202-204` | [`render_json`] |
//! | `report.ts:206-221` | [`series`] |
//! | `report.ts:223-330` | [`comparison`] |
//! | — (PLAT-958, no retained counterpart) | [`verdict`], [`vanished`] |
//!
//! # Why the renderers return a `Result`
//!
//! A dimension value is a stored [`quoin_store::JsonValue`] and the retained
//! renderer interpolates it with `${value}`, which is `String(value)`. This
//! crate has exactly one `String(x)` — [`crate::common::scalar::js_string`] —
//! and it speaks [`serde_json::Value`], so reaching it means crossing
//! [`crate::json_bridge`], the crate's one crossing, which is fallible for a
//! value no canonical writer can spell. [`crate::compare`] returns a `Result`
//! for the same reason and in the same place.
//!
//! # Where the bytes come from
//!
//! [`render_json`] builds a `serde` view and hands it to
//! [`quoin_store::canonical_json_bytes`]. There is no second JSON writer here:
//! `canonicalJson` (`src/store/canonical.ts:19`) is `quoin-store`'s, and
//! `tc_468_boundary` asserts over this crate's sources that it stayed that way.

pub mod build;
pub mod comparison;
pub mod render;
pub mod render_json;
pub mod series;
pub mod vanished;
pub mod verdict;
pub(crate) mod verdict_render;
mod verdict_wire;
pub mod wire;

pub use build::{
    CollectionSummary, CurrentRow, MeasurementReport, build_measurement_report,
    build_measurement_report_from,
};
pub use comparison::{
    MeasurementCollectionReference, MeasurementComparisonReport, MeasurementComparisonStatus,
    comparison_for, render_measurement_comparison, render_measurement_comparison_json,
};
pub use render::render_measurement_report;
pub use render_json::render_measurement_report_json;
pub use series::{SeriesPoint, render_series_json, series_for};
pub use vanished::{VanishedSlice, vanished_slices};
pub use verdict::{
    BestPrior, InconclusiveReason, RatchetOutcome, StageVerdict, TargetOutcome, stage_verdict,
};
