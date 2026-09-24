// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Compile the engineering-assurance pin-agreement promise (PLAT-1032).
//!
//! quoin pins `engineering-assurance` in three committed places that must
//! hold one revision: `rust/Cargo.toml`'s crate dependency, which decides
//! measurement verdicts; `default-modules.yaml`'s `engineering-assurance`
//! module entry, which carries the schemas that validate `AssuranceProfile`
//! and `MeasurementPlan` frontmatter; and `quoin-cli`'s retained-catalog
//! fixture registry, which reconciliation compares against that module entry. PR #618 moved only the crate to `=0.4.0` and left the module
//! at `0.2.0+4e6522f`, which made this repo's own `spec/assurance/AP-201` and
//! `AP-202` unvalidatable against the module quoin itself ships — v0.4.0
//! renamed `AssuranceProfile`'s `profile_version` to `schema_version` under
//! `additionalProperties: false`. No gate caught it: CI never validates
//! assurance frontmatter, and `default-modules.yaml` is embedded via
//! `include_str!`, so a green `cargo build` proves nothing about its
//! contents. It was found only by smoke-testing the release binary.
//!
//! Bumping the module and leaving the fixture behind then broke `quoin-cli`'s
//! `tc_1650` with "fixture deliberately resolves no modules" — a failure whose
//! text points at catalog discovery and says nothing about a pin two crates
//! away. All three are asserted here so the next bump is told which files to
//! move, by name, instead of being sent to debug the symptom.
//!
//! Every file is read with `include_str!`, not a runtime file read, so the
//! assertion is a build input and stays honest regardless of the working
//! directory the test happens to run from.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_modules::manifest::MarketplaceManifest;
use quoin_modules::source::Source;

/// The module and crate this test holds to one revision.
const NAME: &str = "engineering-assurance";

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");
const CARGO_MANIFEST: &str = include_str!("../../../Cargo.toml");

/// `quoin-cli`'s frozen retained-catalog fixture, whose contract is that
/// `catalog list` resolves no modules from it. Reconciliation compares this
/// registry's `ref` against the embedded `default-modules.yaml`, so a pin bump
/// that leaves this behind makes the fixture reconcile a module into existence
/// and `tc_1650` fails with "fixture deliberately resolves no modules" — which
/// reads as a `quoin-cli` catalog-discovery defect and not as what it is, an
/// unbumped pin two crates away. That is exactly how PLAT-1032 lost time, so
/// the third copy is asserted here beside the other two rather than left to be
/// rediscovered from a confusing failure.
const RETAINED_CATALOG_REGISTRY: &str =
    include_str!("../../quoin-cli/tests/fixtures/retained-catalog/ix-home/filament/registry.json");

/// The value of `key = "…"` inside one TOML inline-table line.
///
/// Duplicated from `quoin-quire`'s `build_support/manifest_pin.rs` rather than
/// shared or reached across the crate boundary with a relative `include!`:
/// it is a small, private build-support helper, not a published dependency
/// between crates, and the workspace has no `toml` (or serde YAML/TOML)
/// crate available to either crate's dev-dependencies to parse this
/// properly (checked: neither appears anywhere in `rust/Cargo.lock`).
///
/// The key must begin at a TOML token boundary — start of line, whitespace,
/// `{` or `,` — and be followed by `=`. A naive `line.find(key)` would match
/// the same letters inside the dependency's own name or its git URL; neither
/// `engineering-assurance` (the crate name and the URL's last path segment)
/// contains `version` or `rev` as a substring today, but the boundary check
/// keeps that true by construction rather than by accident.
fn field(line: &str, key: &str) -> Option<String> {
    line.match_indices(key)
        .filter(|(index, _)| {
            line.get(..*index)
                .and_then(|before| before.chars().next_back())
                .is_none_or(|char| char.is_whitespace() || char == '{' || char == ',')
        })
        .find_map(|(index, _)| {
            let after_key = line.get(index.saturating_add(key.len())..)?;
            let after_equals = after_key.trim_start().strip_prefix('=')?;
            let rest = after_equals.trim_start().strip_prefix('"')?;
            let closing = rest.find('"')?;
            rest.get(..closing).map(str::to_string)
        })
}

/// Trace: FR-016-AC-2
/// Provenance: PLAT-1032
#[test]
fn tc_1032_every_engineering_assurance_pin_is_the_same_commit() {
    let manifest =
        MarketplaceManifest::from_yaml(DEFAULT_MODULES).expect("default-modules.yaml parses");
    let entry = manifest
        .entries
        .iter()
        .find(|e| e.name.as_str() == NAME)
        .expect("default-modules.yaml declares an engineering-assurance entry");
    let module_rev = match &entry.source {
        Source::GitSubdir { r#ref, .. } => r#ref
            .as_deref()
            .expect("the engineering-assurance module entry pins a ref"),
        other => panic!("engineering-assurance module entry is not git-subdir: {other:?}"),
    };

    let crate_line = CARGO_MANIFEST
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(NAME))
        .expect("rust/Cargo.toml declares the engineering-assurance dependency");
    let crate_rev =
        field(crate_line, "rev").expect("the engineering-assurance dependency line declares a rev");

    assert_eq!(
        module_rev, crate_rev,
        "default-modules.yaml's engineering-assurance module pins ref {module_rev:?} but \
         rust/Cargo.toml's engineering-assurance crate dependency pins rev {crate_rev:?}. \
         The crate decides measurement verdicts and the module carries the schemas that \
         validate AssuranceProfile and MeasurementPlan frontmatter against those verdicts — \
         they are two halves of one engineering-assurance version and must be bumped \
         together (PLAT-1032)."
    );

    // The declared `version` label is informational (ref/rev governs what is
    // actually installed and built), but it is meant to be legible at a
    // glance, so it is worth catching the two labels themselves drifting
    // apart even when the (checked above) revisions still agree.
    let module_version = entry
        .version
        .as_deref()
        .expect("the engineering-assurance module entry declares a version");
    let crate_version = field(crate_line, "version")
        .expect("the engineering-assurance dependency line declares a version")
        .trim_start_matches('=')
        .to_string();

    assert_eq!(
        module_version, crate_version,
        "default-modules.yaml's engineering-assurance module declares version {module_version:?} \
         but rust/Cargo.toml's engineering-assurance crate dependency declares version \
         {crate_version:?} (after normalising the exact-pin `=`). The crate pin and the module \
         pin are two halves of one engineering-assurance version and must be bumped together \
         (PLAT-1032)."
    );

    let registry: serde_json::Value = serde_json::from_str(RETAINED_CATALOG_REGISTRY)
        .expect("the retained-catalog fixture registry is JSON");
    let fixture_rev = registry
        .get("plugins")
        .and_then(serde_json::Value::as_array)
        .expect("the retained-catalog registry lists plugins")
        .iter()
        .find(|plugin| plugin.get("name").and_then(serde_json::Value::as_str) == Some(NAME))
        .and_then(|plugin| plugin.get("ref"))
        .and_then(serde_json::Value::as_str)
        .expect("the retained-catalog registry pins engineering-assurance at a ref");

    assert_eq!(
        module_rev, fixture_rev,
        "default-modules.yaml's engineering-assurance module pins ref {module_rev:?} but \
         quoin-cli's retained-catalog fixture registry pins ref {fixture_rev:?}. The fixture's \
         contract is that `catalog list` resolves nothing from it, and reconciliation compares \
         that registry's ref against the embedded default-modules.yaml, so leaving the fixture \
         behind makes tc_1650 fail as though catalog discovery were broken (PLAT-1032)."
    );
}
