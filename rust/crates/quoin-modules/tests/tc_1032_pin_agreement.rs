// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Compile the engineering-assurance pin-agreement promise (PLAT-1032).
//!
//! quoin pins `engineering-assurance` twice: `rust/Cargo.toml`'s crate
//! dependency, which decides measurement verdicts, and
//! `default-modules.yaml`'s `engineering-assurance` module entry, which
//! carries the schemas that validate `AssuranceProfile` and `MeasurementPlan`
//! frontmatter. PR #618 moved only the crate to `=0.4.0` and left the module
//! at `0.2.0+4e6522f`, which made this repo's own `spec/assurance/AP-201` and
//! `AP-202` unvalidatable against the module quoin itself ships — v0.4.0
//! renamed `AssuranceProfile`'s `profile_version` to `schema_version` under
//! `additionalProperties: false`. No gate caught it: CI never validates
//! assurance frontmatter, and `default-modules.yaml` is embedded via
//! `include_str!`, so a green `cargo build` proves nothing about its
//! contents. It was found only by smoke-testing the release binary.
//!
//! Both files are read with `include_str!`, not a runtime file read, so the
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

const DEFAULT_MODULES: &str = include_str!("../../../../default-modules.yaml");
const CARGO_MANIFEST: &str = include_str!("../../../Cargo.toml");

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
fn tc_1032_the_module_pin_and_the_crate_pin_are_the_same_commit() {
    let manifest =
        MarketplaceManifest::from_yaml(DEFAULT_MODULES).expect("default-modules.yaml parses");
    let entry = manifest
        .entries
        .iter()
        .find(|e| e.name.as_str() == "engineering-assurance")
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
        .find(|line| line.starts_with("engineering-assurance"))
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
}
