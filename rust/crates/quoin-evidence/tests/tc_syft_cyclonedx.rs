// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The SBOM adapter (FR-041) read against a real Syft `CycloneDX` document,
//! captured from Syft 1.57.1 over a four-package scratch `Cargo.lock` and
//! checked in unedited (`tests/fixtures/evidence/syft-cyclonedx-real.json`).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::adapters::parse_sbom;

const SYFT: &str = include_str!("../../../../tests/fixtures/evidence/syft-cyclonedx-real.json");

/// Syft's own output yields one passing entry per component: three identified
/// by purl, and the root package, which Syft emits without a purl, by name.
#[trace("FR-041-AC-1", "FR-041-AC-5")]
#[test]
fn tc_1430_a_real_syft_cyclonedx_document_yields_one_entry_per_component() {
    let result = parse_sbom(SYFT).unwrap();
    assert_eq!(result.entries.len(), 4, "{:?}", result.entries);
    let symbols: Vec<&str> = result
        .entries
        .iter()
        .map(|entry| entry.symbol.as_str())
        .collect();
    assert!(
        symbols.contains(&"pkg:cargo/gizmo-core@1.2.3"),
        "{symbols:?}"
    );
    assert!(symbols.contains(&"fictional-widget@0.1.0"), "{symbols:?}");
}
