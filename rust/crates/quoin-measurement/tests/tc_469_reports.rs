// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The two record renderers render an empty record set as nothing at all.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_measurement::intervention::report::render_intervention_report;
use quoin_measurement::operational::report::render_operational_report;

/// Trace: FR-100-AC-4
/// Provenance: quoin#469
#[test]
fn tc_469_an_empty_report_renders_as_nothing_at_all() {
    assert_eq!(render_intervention_report(&[]), "");
    assert_eq!(render_operational_report(&[]), "");
}
