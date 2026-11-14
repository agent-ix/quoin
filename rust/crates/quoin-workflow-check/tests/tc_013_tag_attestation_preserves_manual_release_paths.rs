// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Quoin's release workflows preserve manual publish gates beside tag attestation.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use serde_yaml_ng::Value;
use std::fs;
use std::path::PathBuf;

fn workflow(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.github/workflows")
        .join(name);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read workflow {}: {error}", path.display()));
    serde_yaml_ng::from_str(&source)
        .unwrap_or_else(|error| panic!("parse workflow {}: {error}", path.display()))
}

fn only_event<'a>(workflow: &'a Value, name: &str) -> &'a Value {
    let events = workflow["on"]
        .as_mapping()
        .expect("workflow has event mapping");
    assert_eq!(events.len(), 1, "workflow must keep one trigger");
    &events[&Value::String(name.to_owned())]
}

#[trace("FR-112-AC-5")]
#[test]
fn tc_013_tag_attestation_preserves_manual_release_paths() {
    let release = workflow("release.yml");
    let manual = only_event(&release, "workflow_dispatch");
    let publish = &manual["inputs"]["publish"];
    assert_eq!(publish["default"].as_bool(), Some(false));

    let publish_step = release["jobs"]["publish-npm"]["steps"]
        .as_sequence()
        .expect("npm workflow has steps")
        .iter()
        .find(|step| step["name"].as_str() == Some("Package and publish the npm distribution"))
        .expect("npm publish step exists");
    assert_eq!(
        publish_step["with"]["publish"].as_str(),
        Some("${{ inputs.publish }}")
    );

    let smoke_step = release["jobs"]["publish-npm"]["steps"]
        .as_sequence()
        .expect("npm workflow has steps")
        .iter()
        .find(|step| step["name"].as_str() == Some("Smoke the published npm packages"))
        .expect("post-publish smoke step exists");
    assert_eq!(smoke_step["if"].as_str(), Some("inputs.publish"));

    let native_release = workflow("native-release.yml");
    only_event(&native_release, "workflow_dispatch");

    let tag_attestation = workflow("tag-attestation.yml");
    let push = only_event(&tag_attestation, "push");
    assert_eq!(push["tags"][0].as_str(), Some("v*"));
    let attest = &tag_attestation["jobs"]["attest"];
    assert!(
        attest["uses"]
            .as_str()
            .expect("tag attestation calls a reusable workflow")
            .starts_with(
                "agent-ix/engineering-assurance/.github/workflows/release-attestation.yml@"
            )
    );
}
