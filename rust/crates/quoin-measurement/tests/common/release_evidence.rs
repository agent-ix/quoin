// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Synthetic GitHub release evidence, built in the test.
//!
//! The GitHub-release producer reads a workflow export, a workflow-run export
//! and a workflow-jobs export. These builders write small ones into a
//! temporary repository, with the head revision generated per call, so no
//! export captured from a real run is committed and no revision is frozen in
//! the tree.

#![allow(
    dead_code,
    reason = "each test binary includes this module and uses a different subset"
)]

use std::path::Path;

use serde_json::{Value, json};

/// The prefix the produced record identities carry.
pub(crate) const PREFIX: &str = "synthetic-release";

/// The pair file the producer names from the two record identities.
pub(crate) const PAIR: &str = "synthetic-release-capability+synthetic-release-exercise.json";

/// The definition file, under `spec/evidence/github-actions/`.
pub(crate) const DEFINITION: &str = "synthetic-release-definition.json";

/// The workflow export.
pub(crate) const WORKFLOW: &str = "synthetic-release-workflow.yml";

/// The workflow-run export.
pub(crate) const RUN: &str = "synthetic-release-run.json";

/// The workflow-jobs export.
pub(crate) const JOBS: &str = "synthetic-release-jobs.json";

/// The run id the synthetic exports carry.
pub(crate) const RUN_ID: u64 = 4_200_000_001;

/// The producer definition, as a document.
pub(crate) fn definition_document() -> Value {
    json!({
        "record_prefix": PREFIX,
        "workflow_path": ".github/workflows/release.yml",
        "release_job": "Publish",
        "accepted_event": "workflow_dispatch",
        "control_id": "quoin-npm-release",
        "subject": { "id": "agent-ix/quoin" },
        "producer": {
            "tool_identity": "quoin-github-release-operational-adapter",
            "tool_version": "0.9.0",
            "configuration_ref": format!("github-actions/{DEFINITION}"),
            "environment": {
                "github_repository": "agent-ix/quoin",
                "github_workflow_run_id": RUN_ID,
                "release_version": "0.0.1"
            },
            "definition_version": "github-actions.release-operational-v2"
        },
        "scope": {
            "service": "agent-ix/quoin",
            "environment": "npm-production",
            "population": "synthetic package release"
        },
        "supported_transition": "tagged-to-published",
        "authorized_roles": ["repository maintainer with Actions dispatch access"],
        "coverage": "manual npm publication of the exact tagged @agent-ix/quoin version",
        "limitations": ["the producer covers GitHub Actions release workflows only"],
        "owner": "engineering assurance",
        "gaps": ["npm registry receipt is not part of this first producer's retained input contract"],
        "actions": ["retain a later adverse release run to exercise the non-success path"],
        "clock_deadline_seconds": 600,
        "workflow_evidence_path": format!("github-actions/{WORKFLOW}"),
        "run_evidence_path": format!("github-actions/{RUN}"),
        "jobs_evidence_path": format!("github-actions/{JOBS}")
    })
}

/// A workflow with four jobs, one of them the release job.
pub(crate) fn workflow_text() -> String {
    "name: Release\n\
on:\n  workflow_dispatch:\n\
jobs:\n  test:\n    name: Test\n    runs-on: ubuntu-24.04\n    steps: []\n  \
publish:\n    name: Publish\n    needs: test\n    runs-on: ubuntu-24.04\n    steps: []\n"
        .to_owned()
}

/// A head revision generated for this call.
///
/// Both exports carry it so the producer's within-run comparison has something
/// to compare; nothing about it is fixed.
pub(crate) fn generated_revision() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |elapsed| elapsed.as_nanos());
    format!("{nanos:040x}")
}

/// A completed, successful workflow-run export.
pub(crate) fn run_export(revision: &str) -> Value {
    json!({
        "id": RUN_ID,
        "name": "Release",
        "head_sha": revision,
        "path": ".github/workflows/release.yml",
        "event": "workflow_dispatch",
        "status": "completed",
        "conclusion": "success",
        "html_url": format!("https://github.example.test/agent-ix/quoin/actions/runs/{RUN_ID}"),
        "created_at": "2026-08-29T23:08:29Z",
        "updated_at": "2026-08-29T23:11:38Z",
        "run_attempt": 1,
        "actor": { "login": "release-actor" }
    })
}

/// A workflow-jobs export: one Test job and the Publish job.
pub(crate) fn jobs_export(revision: &str) -> Value {
    let job = |name: &str, started: &str, completed: &str| {
        json!({
            "run_id": RUN_ID,
            "run_attempt": 1,
            "head_sha": revision,
            "status": "completed",
            "conclusion": "success",
            "started_at": started,
            "completed_at": completed,
            "name": name
        })
    };
    json!({
        "total_count": 2,
        "jobs": [
            job("Test", "2026-08-29T23:08:33Z", "2026-08-29T23:10:57Z"),
            job("Publish", "2026-08-29T23:11:00Z", "2026-08-29T23:11:37Z")
        ]
    })
}

/// The measurement plan the governance check reads.
fn governance_text() -> String {
    [
        "---",
        "id: MP-9001",
        "title: Synthetic release operational evidence",
        "type: MeasurementPlan",
        "status: active",
        "owner: engineering assurance",
        "stage: gate",
        "metric: release_operational_evidence",
        "definition_version: github-actions.release-operational-v2",
        "ground_truth_kind: mechanical",
        "protected_apparatus:",
        "  - spec/evidence/github-actions/synthetic-release-workflow.yml",
        "negative_controls:",
        "  - kind: apparatus-edit",
        "    description: an edit to the protected apparatus without a new definition version rejects",
        "relationships: []",
        "---",
        "",
        "# Synthetic release operational evidence",
        "",
        "## Decision Use",
        "",
        "Decide whether a release capability and a successful exercise exist.",
        "",
    ]
    .join("\n")
}

/// Write the governance document, the definition and the three exports into
/// `root`, for a producer run.
pub(crate) fn write_repository(root: &Path) {
    let assurance = root.join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("a writable assurance directory");
    std::fs::write(
        assurance.join("MP-9001-synthetic-release-operational-evidence.md"),
        governance_text(),
    )
    .expect("a writable governance document");

    let actions = root.join("spec").join("evidence").join("github-actions");
    std::fs::create_dir_all(&actions).expect("a writable exports directory");
    let revision = generated_revision();
    let write_json = |name: &str, value: &Value| {
        std::fs::write(
            actions.join(name),
            serde_json::to_string_pretty(value).expect("the export serialises"),
        )
        .expect("a writable export");
    };
    write_json(DEFINITION, &definition_document());
    write_json(RUN, &run_export(&revision));
    write_json(JOBS, &jobs_export(&revision));
    std::fs::write(actions.join(WORKFLOW), workflow_text()).expect("a writable workflow export");
}
