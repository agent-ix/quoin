// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Check that npm repackaging consumes the exact GitHub Release assets.

#![allow(clippy::expect_used, reason = "test setup failures should be explicit")]

use ix_trace_rs::trace;
use std::fs;
use std::path::PathBuf;

fn workflow_text(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.github/workflows")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read workflow {}: {error}", path.display()))
}

#[trace("FR-112-AC-2")]
#[test]
fn npm_repack_downloads_manifest_named_assets_and_never_rebuilds_quoin() {
    let release = workflow_text("release.yml");
    let native = workflow_text("native-release.yml");

    assert!(release.contains("ref: ${{ inputs.tag }}"), "{release}");
    assert!(
        release.contains("gh release download \"$TAG\""),
        "{release}"
    );
    assert!(
        release.contains("test -f release/quoin-update-manifest.json"),
        "{release}"
    );
    assert!(
        release.contains("jq -c '.artifacts[]' release/quoin-update-manifest.json"),
        "{release}"
    );
    assert!(
        release.contains("(.artifacts | length == 4)"),
        "the release manifest must name every supported platform asset"
    );
    assert!(release.contains("file=\"release/$asset\""), "{release}");
    assert!(release.contains("test -f \"$file\""), "{release}");
    assert!(
        release
            .find("test -f release/quoin-update-manifest.json")
            .expect("missing manifest is rejected")
            < release
                .find("uses: agent-ix/nodejs-actions/publish-native-npm@main")
                .expect("shared publisher is called after validation"),
        "a missing manifest fails before publication"
    );
    assert!(
        release
            .find("test -f \"$file\"")
            .expect("missing asset is rejected")
            < release
                .find("uses: agent-ix/nodejs-actions/publish-native-npm@main")
                .expect("shared publisher is called after extraction"),
        "a missing manifest asset fails before publication"
    );
    assert!(
        release.contains("package: \"@agent-ix/quoin\""),
        "{release}"
    );
    assert!(
        !release.lines().any(|line| {
            line.trim_start().starts_with("run:")
                && line.contains("cargo build")
                && line.contains("quoin-cli")
        }),
        "npm packaging may build its checker but must not rebuild the native quoin binary"
    );
    assert!(
        native.contains("gh release create \"$TAG\" --repo \"$GITHUB_REPOSITORY\""),
        "native workflow publishes GitHub Release assets before the repackage workflow consumes them"
    );
}
