// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The engine-pin parser `build.rs` enforces the `rev` with (quoin#379).
//!
//! `build.rs` is not compiled as a test target, so its parser is included
//! here from `build_support/manifest_pin.rs` — the same bytes the build
//! script includes — and asserted in the ordinary suite.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

include!("../build_support/manifest_pin.rs");

/// Trace: FR-097
#[test]
fn tc_379_026_the_pin_parses_out_of_this_crates_own_manifest() {
    let manifest = include_str!("../Cargo.toml");
    let line = manifest
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("quire-rs"))
        .expect("the manifest declares the engine pin");

    // The shape is asserted, not the literal pin: the manifest line IS the
    // pin, and a second copy of its value here only has to be re-typed on
    // every engine bump without guarding anything the parse does not.
    let version = field(line, "version").expect("the pin declares a version");
    assert!(version.starts_with('='), "an exact pin, got {version:?}");
    let rev = field(line, "rev").expect("the pin declares a rev");
    assert_eq!(rev.len(), 40, "a full commit id, got {rev:?}");
    assert!(rev.bytes().all(|b| b.is_ascii_hexdigit()), "hex, got {rev:?}");
}

/// Trace: FR-097
///
/// The regression finding 3 of the PR #398 review names: a URL carrying the
/// word `version` (or `rev`) BEFORE the real key. A `split(key)` parser cuts
/// the line at the first occurrence and reads the URL's own text as the pin,
/// silently. The revision pin is the thing this function exists to enforce, so
/// a silent mis-parse of it is the worst available failure.
#[test]
fn tc_379_027_a_url_containing_the_key_does_not_shadow_the_real_pin() {
    let line = r#"quire-rs = { git = "https://github.com/agent-ix/version-of-rev-engine", version = "=0.46.0", rev = "85dfe9d5a937c52af6456f2e6aa3a6bc4c82db9f" }"#;

    assert_eq!(
        field(line, "version").as_deref(),
        Some("=0.46.0"),
        "the `version` inside the URL must not shadow the declared pin"
    );
    assert_eq!(
        field(line, "rev").as_deref(),
        Some("85dfe9d5a937c52af6456f2e6aa3a6bc4c82db9f"),
        "the `rev` inside the URL must not shadow the declared pin"
    );
}

/// Trace: FR-097
#[test]
fn tc_379_028_a_key_that_is_only_a_substring_is_not_a_match() {
    // `subversion` ends in `version`, and `irrev` ends in `rev`; neither is
    // the key, and neither may be read as one.
    let line = r#"quire-rs = { subversion = "9", irrev = "no", version = "=1.2.3", rev = "0123456789abcdef0123456789abcdef01234567" }"#;

    assert_eq!(field(line, "version").as_deref(), Some("=1.2.3"));
    assert_eq!(
        field(line, "rev").as_deref(),
        Some("0123456789abcdef0123456789abcdef01234567")
    );

    // A line with no such key at all yields nothing rather than the next
    // quoted string it happens to find.
    assert_eq!(field(r#"quire-rs = { path = "../engine" }"#, "rev"), None);
}
