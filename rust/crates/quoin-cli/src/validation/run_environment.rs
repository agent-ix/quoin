// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Capture suite checkout and temporary-directory facts for FR-217 reports.

use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};

const REQUIRED_CHECKOUTS: [(&str, &str, &str); 2] = [
    (
        "agent-ix/spec-objects-safety",
        "SPEC_OBJECTS_SAFETY_DIR",
        "spec-objects-safety",
    ),
    (
        "agent-ix/spec-objects-security",
        "SPEC_OBJECTS_SECURITY_DIR",
        "spec-objects-security",
    ),
];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct RunEnvironment {
    pub(crate) checkouts: Vec<RepositoryCheckout>,
    pub(crate) temporary_directory: TemporaryDirectory,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct RepositoryCheckout {
    pub(crate) expected_repository: String,
    pub(crate) repository: Option<String>,
    pub(crate) revision: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) available: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct TemporaryDirectory {
    pub(crate) path: Option<String>,
    pub(crate) writable: bool,
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
pub(crate) fn capture_run_environment(working_directory: &Path) -> RunEnvironment {
    let paths = REQUIRED_CHECKOUTS.map(|(identity, variable, name)| {
        (identity, checkout_path(working_directory, variable, name))
    });
    let temporary_directory = std::env::temp_dir();
    capture_from_paths(paths, &temporary_directory)
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn checkout_path(working_directory: &Path, variable: &str, name: &str) -> Option<PathBuf> {
    if let Some(configured) = std::env::var_os(variable).filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(configured));
    }

    let repository_root = quoin_root(working_directory)?;
    repository_root
        .ancestors()
        .flat_map(|ancestor| [ancestor.join(".ci-deps").join(name), ancestor.join(name)])
        .find(|candidate| candidate.is_dir())
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn quoin_root(working_directory: &Path) -> Option<PathBuf> {
    working_directory
        .ancestors()
        .find(|candidate| {
            candidate.join("rust/Cargo.toml").is_file() && candidate.join(".git").exists()
        })
        .map(Path::to_path_buf)
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn capture_from_paths(
    paths: [(&str, Option<PathBuf>); 2],
    temporary_directory: &Path,
) -> RunEnvironment {
    RunEnvironment {
        checkouts: paths
            .into_iter()
            .map(|(expected, path)| observe_checkout(expected, path.as_deref()))
            .collect(),
        temporary_directory: observe_temporary_directory(temporary_directory),
    }
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn observe_checkout(expected_repository: &str, candidate: Option<&Path>) -> RepositoryCheckout {
    let Some(candidate) = candidate else {
        return unavailable_checkout(expected_repository, None, None, None);
    };
    let Some(root) = git_output(candidate, &["rev-parse", "--show-toplevel"])
        .and_then(|output| output_path(&output))
        .and_then(|path| path.canonicalize().ok())
    else {
        return unavailable_checkout(expected_repository, None, None, None);
    };
    let path = root.to_str().map(str::to_owned);
    let repository = git_output(&root, &["remote", "get-url", "origin"])
        .and_then(|output| output_string(&output))
        .and_then(|remote| canonical_repository_identity(&remote));
    let revision = git_output(&root, &["rev-parse", "HEAD"])
        .and_then(|output| output_string(&output))
        .filter(|revision| {
            matches!(revision.len(), 40 | 64)
                && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    let available =
        repository.as_deref() == Some(expected_repository) && revision.is_some() && path.is_some();
    RepositoryCheckout {
        expected_repository: expected_repository.to_owned(),
        repository,
        revision,
        path,
        available,
    }
}

#[implements("FR-217-AC-14")]
fn unavailable_checkout(
    expected_repository: &str,
    repository: Option<String>,
    revision: Option<String>,
    path: Option<String>,
) -> RepositoryCheckout {
    RepositoryCheckout {
        expected_repository: expected_repository.to_owned(),
        repository,
        revision,
        path,
        available: false,
    }
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn observe_temporary_directory(path: &Path) -> TemporaryDirectory {
    let resolved = path.canonicalize().ok();
    let writable = resolved.as_deref().is_some_and(|directory| {
        tempfile::Builder::new()
            .prefix(".quoin-selftest-write-probe-")
            .tempfile_in(directory)
            .is_ok()
    });
    TemporaryDirectory {
        path: resolved.and_then(|path| path.to_str().map(str::to_owned)),
        writable,
    }
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn git_output(repository: &Path, arguments: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn output_string(output: &[u8]) -> Option<String> {
    String::from_utf8(output.to_vec())
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[implements("FR-217-AC-13")]
#[implements("FR-217-AC-14")]
fn output_path(output: &[u8]) -> Option<PathBuf> {
    output_string(output).map(PathBuf::from)
}

#[implements("FR-217-AC-13")]
fn canonical_repository_identity(remote: &str) -> Option<String> {
    let remote = remote.trim();
    let path = [
        "git@github.com:",
        "ssh://git@github.com/",
        "https://github.com/",
        "http://github.com/",
        "git://github.com/",
    ]
    .into_iter()
    .find_map(|prefix| remote.strip_prefix(prefix))?;
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut segments = path.split('/');
    let owner = segments.next()?;
    let repository = segments.next()?;
    if owner.is_empty() || repository.is_empty() || segments.next().is_some() {
        return None;
    }
    Some(format!("{owner}/{repository}"))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "the test module reports fixture setup and assertion failures directly"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use std::fs;

    fn checkout(path: &Path, repository: &str) -> PathBuf {
        fs::create_dir_all(path).expect("checkout directory is created");
        let init = Command::new("git")
            .args(["init", "--quiet"])
            .arg(path)
            .status()
            .expect("git init starts");
        assert!(init.success());
        fs::write(path.join("README.md"), "fixture\n").expect("checkout file is written");
        for (key, value) in [
            ("user.name", "Fixture"),
            ("user.email", "fixture@example.test"),
        ] {
            let configured = Command::new("git")
                .args(["-C"])
                .arg(path)
                .args(["config", key, value])
                .status()
                .expect("git config starts");
            assert!(configured.success());
        }
        let added = Command::new("git")
            .args(["-C"])
            .arg(path)
            .args(["add", "README.md"])
            .status()
            .expect("git add starts");
        assert!(added.success());
        let committed = Command::new("git")
            .args(["-C"])
            .arg(path)
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.test",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ])
            .status()
            .expect("git commit starts");
        assert!(committed.success());
        let remote = format!("https://github.com/{repository}.git");
        let configured = Command::new("git")
            .args(["-C"])
            .arg(path)
            .args(["remote", "add", "origin", &remote])
            .status()
            .expect("git remote starts");
        assert!(configured.success());
        path.to_path_buf()
    }

    #[trace("FR-217-AC-13")]
    #[test]
    fn records_both_canonical_checkouts_and_writable_temp_directory() {
        let fixture = tempfile::tempdir().expect("fixture root");
        let safety = checkout(
            &fixture.path().join("safety"),
            "agent-ix/spec-objects-safety",
        );
        let security = checkout(
            &fixture.path().join("security"),
            "agent-ix/spec-objects-security",
        );
        let environment = capture_from_paths(
            [
                ("agent-ix/spec-objects-safety", Some(safety)),
                ("agent-ix/spec-objects-security", Some(security)),
            ],
            fixture.path(),
        );

        assert!(
            environment
                .checkouts
                .iter()
                .all(|checkout| checkout.available)
        );
        for checkout in &environment.checkouts {
            assert_eq!(
                checkout.repository.as_deref(),
                Some(checkout.expected_repository.as_str())
            );
            assert_eq!(checkout.revision.as_ref().map(String::len), Some(40));
            assert!(
                checkout
                    .path
                    .as_ref()
                    .is_some_and(|path| Path::new(path).is_absolute())
            );
        }
        assert_eq!(
            environment.temporary_directory.path.as_deref(),
            fixture
                .path()
                .canonicalize()
                .ok()
                .and_then(|path| path.to_str().map(str::to_owned))
                .as_deref()
        );
        assert!(environment.temporary_directory.writable);
    }

    #[trace("FR-217-AC-14")]
    #[test]
    fn records_missing_checkout_and_unwritable_temp_directory_separately() {
        let fixture = tempfile::tempdir().expect("fixture root");
        let security = checkout(
            &fixture.path().join("security"),
            "agent-ix/spec-objects-security",
        );
        let not_a_directory = fixture.path().join("not-a-directory");
        fs::write(&not_a_directory, "file\n").expect("temp path fixture is written");
        let environment = capture_from_paths(
            [
                ("agent-ix/spec-objects-safety", None),
                ("agent-ix/spec-objects-security", Some(security)),
            ],
            &not_a_directory,
        );

        assert_eq!(
            environment.checkouts[0].expected_repository,
            "agent-ix/spec-objects-safety"
        );
        assert!(!environment.checkouts[0].available);
        assert!(environment.checkouts[0].revision.is_none());
        assert!(environment.checkouts[1].available);
        assert!(!environment.temporary_directory.writable);
        assert!(environment.temporary_directory.path.is_some());
    }
}
