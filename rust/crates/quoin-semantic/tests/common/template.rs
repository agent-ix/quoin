// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rendering the real semantic-module cookiecutter into a temporary directory,
//! shared by the template render suite (PLAT-264) and the refusal suite
//! (PLAT-194).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

pub const TEMPLATE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../templates/semantic-module"
);

/// An import a `mixed` variant needs; the form is `<org>/<repo>@<requirement>`.
pub const IMPORT: &str = "agent-ix/spec-artifacts-iso@0.1.0";

/// One rendered repository, living only as long as this value: dropping it
/// removes the directory, including when a test panics.
pub struct Render {
    pub _dir: TempDir,
    pub root: PathBuf,
    pub package: String,
}

impl Render {
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative.replace("{package}", &self.package))
    }

    pub fn text(&self, relative: &str) -> String {
        let path = self.path(relative);
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    pub fn manifest(&self) -> Value {
        quoin_yaml::from_str(&self.text("{package}/manifest.yaml")).unwrap()
    }
}

/// `cookiecutter` 1.x silently replaces an off-list choice with the first one,
/// so FR-076-AC-6 cannot hold there; the floor is 2.x (FR-076).
fn assert_renderer_floor() {
    static CHECKED: std::sync::Once = std::sync::Once::new();
    CHECKED.call_once(|| {
        let out = Command::new("cookiecutter").arg("--version").output().expect(
            "`cookiecutter` is not installed; install it with `pipx install cookiecutter` \
             (NFR-020 declares it a toolchain floor, so this is a failure, not a skip)",
        );
        let text = String::from_utf8_lossy(&out.stdout);
        let major: u32 = text
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.split('.').next())
            .and_then(|m| m.parse().ok())
            .unwrap_or_else(|| panic!("unreadable `cookiecutter --version`: {text}"));
        assert!(
            major >= 2,
            "cookiecutter {major}.x is below the 2.x floor (FR-076); upgrade with `pipx upgrade cookiecutter`"
        );
    });
}

pub fn cookiecutter(output: &Path, args: &[&str]) -> std::process::Output {
    assert_renderer_floor();
    Command::new("cookiecutter")
        .args(["--no-input", "--output-dir"])
        .arg(output)
        .arg(TEMPLATE)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .expect(
            "`cookiecutter` is not installed; install it with `pipx install cookiecutter` \
             (NFR-020 declares it a toolchain floor, so this is a failure, not a skip)",
        )
}

/// Render `kind` with extra `key=value` inputs; panics if rendering fails.
pub fn render(kind: &str, imports: &str, extra: &[&str]) -> Render {
    let dir = TempDir::new().unwrap();
    let mut args = vec![format!("module_kind={kind}")];
    if !imports.is_empty() {
        args.push(format!("imported_modules={imports}"));
    }
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = cookiecutter(dir.path(), &refs);
    assert!(
        out.status.success(),
        "variant {kind}: rendering failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let root = dir.path().join("spec-objects-example");
    Render {
        _dir: dir,
        root,
        package: "spec_objects_example".to_owned(),
    }
}
