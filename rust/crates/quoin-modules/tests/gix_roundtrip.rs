// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! End-to-end `gix` resolution against a repository built in the test
//! (quoin#381, FR-019).
//!
//! Hermetic: the "remote" is a bare repository this test writes object by
//! object and then fetches over the local transport. No network, no `git`
//! binary, and no `#[ignore]` lane — the fetch, revision resolution and tree
//! extraction paths all run for real.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::Path;

use quoin_modules::ModulesErrorCode;
use quoin_modules::git::{GitLimits, GitResolver, GixResolver, Revision};

/// Build a bare repository holding one commit with this layout:
///
/// ```text
/// manifest.yaml
/// spec_objects_fixture/manifest.yaml
/// spec_objects_fixture/artifacts/marker.txt
/// ```
///
/// Returns the repository path and the commit id, and tags the commit `v1.0.0`.
fn build_fixture_repo(dir: &Path) -> (String, String) {
    let repo = gix::init_bare(dir).expect("init bare fixture repo");

    let root_manifest = repo
        .write_blob(b"name: fixture-root\n")
        .expect("write blob");
    let sub_manifest = repo
        .write_blob(b"name: spec-objects-fixture\n")
        .expect("write blob");
    let marker = repo.write_blob(b"hello\n").expect("write blob");

    let blob = gix::objs::tree::EntryKind::Blob.into();
    let tree_kind = gix::objs::tree::EntryKind::Tree.into();

    let artifacts = gix::objs::Tree {
        entries: vec![gix::objs::tree::Entry {
            mode: blob,
            filename: "marker.txt".into(),
            oid: marker.into(),
        }],
    };
    let artifacts_id = repo.write_object(&artifacts).expect("write tree");

    // Git requires tree entries sorted by name.
    let subdir = gix::objs::Tree {
        entries: vec![
            gix::objs::tree::Entry {
                mode: tree_kind,
                filename: "artifacts".into(),
                oid: artifacts_id.into(),
            },
            gix::objs::tree::Entry {
                mode: blob,
                filename: "manifest.yaml".into(),
                oid: sub_manifest.into(),
            },
        ],
    };
    let subdir_id = repo.write_object(&subdir).expect("write tree");

    let root = gix::objs::Tree {
        entries: vec![
            gix::objs::tree::Entry {
                mode: blob,
                filename: "manifest.yaml".into(),
                oid: root_manifest.into(),
            },
            gix::objs::tree::Entry {
                mode: tree_kind,
                filename: "spec_objects_fixture".into(),
                oid: subdir_id.into(),
            },
        ],
    };
    let root_id = repo.write_object(&root).expect("write tree");

    let who = gix::actor::Signature {
        name: "Quoin Test".into(),
        email: "test@agent-ix.invalid".into(),
        time: gix::date::Time::new(1_700_000_000, 0),
    };
    let commit = repo
        .commit_as(
            who.to_ref(&mut gix::date::parse::TimeBuf::default()),
            who.to_ref(&mut gix::date::parse::TimeBuf::default()),
            "refs/heads/main",
            "fixture",
            root_id,
            gix::commit::NO_PARENT_IDS,
        )
        .expect("write commit");
    repo.reference(
        "refs/tags/v1.0.0",
        commit,
        gix::refs::transaction::PreviousValue::Any,
        "tag the fixture",
    )
    .expect("write tag");

    (
        dir.to_string_lossy().into_owned(),
        commit.to_hex().to_string(),
    )
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_290_gix_resolves_a_tag_and_extracts_the_whole_tree() {
    let root = tempfile::tempdir().expect("temp dir");
    let (url, commit) = build_fixture_repo(&root.path().join("remote.git"));
    let resolver = GixResolver::new(root.path().join("cache"));

    let fetched = resolver
        .resolve(&url, Revision::Ref("v1.0.0"), None)
        .expect("tag resolves");
    assert_eq!(fetched.sha.as_str(), commit);
    assert_eq!(fetched.requested_ref.as_deref(), Some("v1.0.0"));
    assert_eq!(
        fs::read_to_string(fetched.dir.join("manifest.yaml")).expect("root manifest"),
        "name: fixture-root\n"
    );
    assert_eq!(
        fs::read_to_string(
            fetched
                .dir
                .join("spec_objects_fixture")
                .join("artifacts")
                .join("marker.txt")
        )
        .expect("nested blob"),
        "hello\n"
    );
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_291_gix_extracts_only_the_requested_subdirectory() {
    let root = tempfile::tempdir().expect("temp dir");
    let (url, _) = build_fixture_repo(&root.path().join("remote.git"));
    let resolver = GixResolver::new(root.path().join("cache"));

    let fetched = resolver
        .resolve(&url, Revision::Ref("v1.0.0"), Some("spec_objects_fixture"))
        .expect("subdir resolves");
    assert_eq!(
        fs::read_to_string(fetched.dir.join("manifest.yaml")).expect("subdir manifest"),
        "name: spec-objects-fixture\n",
        "the subdir's own manifest must be at the content root"
    );
    assert!(
        !fetched.dir.join("spec_objects_fixture").exists(),
        "only the subtree is extracted, not the path leading to it"
    );
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_292_an_exact_sha_pin_resolves_and_then_needs_no_remote() {
    let root = tempfile::tempdir().expect("temp dir");
    let remote = root.path().join("remote.git");
    let (url, commit) = build_fixture_repo(&remote);
    let resolver = GixResolver::new(root.path().join("cache"));

    let first = resolver
        .resolve(&url, Revision::Sha(&commit), None)
        .expect("sha resolves");
    assert_eq!(first.sha.as_str(), commit);
    assert_eq!(first.requested_ref, None);

    // Delete the remote entirely. A settled exact pin must still resolve from
    // the extracted content — this is the property that makes `ensure_defaults`
    // free to call before every catalog read.
    fs::remove_dir_all(&remote).expect("remove the remote");
    let second = resolver
        .resolve(&url, Revision::Sha(&commit), None)
        .expect("a settled pin needs no remote");
    assert_eq!(second.dir, first.dir);
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_293_an_unknown_revision_is_reported_not_followed_to_head() {
    let root = tempfile::tempdir().expect("temp dir");
    let (url, _) = build_fixture_repo(&root.path().join("remote.git"));
    let resolver = GixResolver::new(root.path().join("cache"));

    let err = resolver
        .resolve(&url, Revision::Ref("v9.9.9"), None)
        .expect_err("no such tag");
    assert_eq!(err.code(), ModulesErrorCode::GitRevisionNotFound);
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_294_an_absent_subdirectory_is_refused() {
    let root = tempfile::tempdir().expect("temp dir");
    let (url, _) = build_fixture_repo(&root.path().join("remote.git"));
    let resolver = GixResolver::new(root.path().join("cache"));

    let err = resolver
        .resolve(&url, Revision::Ref("v1.0.0"), Some("not_here"))
        .expect_err("no such subdirectory");
    assert_eq!(err.code(), ModulesErrorCode::GitObjectStore);
}

/// Trace: NFR-004
#[test]
fn tc_381_295_extraction_stops_at_the_declared_byte_bound() {
    let root = tempfile::tempdir().expect("temp dir");
    let (url, _) = build_fixture_repo(&root.path().join("remote.git"));
    let resolver = GixResolver::new(root.path().join("cache")).with_limits(GitLimits {
        max_extracted_bytes: 4,
        ..GitLimits::default()
    });

    let err = resolver
        .resolve(&url, Revision::Ref("v1.0.0"), None)
        .expect_err("the fixture is larger than four bytes");
    assert_eq!(err.code(), ModulesErrorCode::ResourceBoundExceeded);
    assert!(
        err.to_string().contains("extracted byte count"),
        "the message must name which bound, got: {err}"
    );
}

/// Trace: NFR-004
#[test]
fn tc_381_296_extraction_stops_at_the_declared_file_bound() {
    let root = tempfile::tempdir().expect("temp dir");
    let (url, _) = build_fixture_repo(&root.path().join("remote.git"));
    let resolver = GixResolver::new(root.path().join("cache")).with_limits(GitLimits {
        max_extracted_files: 1,
        ..GitLimits::default()
    });

    let err = resolver
        .resolve(&url, Revision::Ref("v1.0.0"), None)
        .expect_err("the fixture holds three blobs");
    assert_eq!(err.code(), ModulesErrorCode::ResourceBoundExceeded);
    assert!(err.to_string().contains("file count"), "got: {err}");
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_297_head_resolves_the_remotes_default_branch() {
    let root = tempfile::tempdir().expect("temp dir");
    let (url, commit) = build_fixture_repo(&root.path().join("remote.git"));
    let resolver = GixResolver::new(root.path().join("cache"));

    let fetched = resolver
        .resolve(&url, Revision::Head, None)
        .expect("HEAD resolves to the remote's default branch");
    assert_eq!(fetched.sha.as_str(), commit);
    assert_eq!(fetched.requested_ref, None);
    assert_eq!(
        fs::read_to_string(fetched.dir.join("manifest.yaml")).expect("root manifest"),
        "name: fixture-root\n"
    );
}
