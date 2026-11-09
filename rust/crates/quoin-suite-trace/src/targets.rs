// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Which cargo target a source file belongs to, and so what nextest calls its
//! tests.
//!
//! The quire engine names a test by file and the qualified name inside that
//! file (`tests::tc_1`). nextest names it by binary id and the full module path
//! (`pkg::suite` and `a::tests::tc_1`). Joining the two needs the target a file
//! compiles into and the module path the file contributes, and both come from
//! `cargo metadata` rather than from a guess at directory names, because a
//! package directory need not match its package name.

use std::path::{Component, Path, PathBuf};

use crate::error::Error;

/// How a target's modules map onto its files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `lib` or `bin`: the root file's directory holds the module tree.
    Crate,
    /// An integration test: `tests/x.rs` owns the modules under `tests/x/`.
    Test,
}

/// One compiled target that holds tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    /// The id nextest reports as the test case's `classname`.
    binary_id: String,
    /// Root source file, repository-relative and `/`-separated.
    root: String,
    kind: Kind,
}

impl Target {
    /// The nextest binary id.
    pub(crate) fn binary_id(&self) -> &str {
        &self.binary_id
    }

    /// Whether this is a package's library target.
    pub(crate) fn is_library(&self) -> bool {
        self.kind == Kind::Crate && !self.binary_id.contains("::bin/")
    }

    /// The module path `file` contributes inside this target, or `None` when
    /// the file is not one of this target's modules.
    ///
    /// A file such as `tests/common/mod.rs`, shared by `mod common;` from
    /// several integration tests, belongs to none of them by path and is
    /// reported unmatched rather than attributed to a guess.
    pub(crate) fn module_path(&self, file: &str) -> Option<Vec<String>> {
        if file == self.root {
            return Some(Vec::new());
        }
        let root = Path::new(&self.root);
        let base = match self.kind {
            Kind::Crate => root.parent()?.to_path_buf(),
            Kind::Test => {
                let parent = root.parent()?;
                if root.file_name()? == "main.rs" {
                    parent.to_path_buf()
                } else {
                    parent.join(root.file_stem()?)
                }
            }
        };
        let relative = Path::new(file).strip_prefix(&base).ok()?;
        let mut segments: Vec<String> = Vec::new();
        for component in relative.with_extension("").components() {
            if let Component::Normal(part) = component {
                segments.push(part.to_str()?.to_owned());
            }
        }
        if segments.last().is_some_and(|last| last == "mod") {
            segments.pop();
        }
        Some(segments)
    }
}

/// Targets that hold tests, read from `cargo metadata --no-deps` output.
///
/// `repo` is the repository root the engine's paths are relative to.
///
/// # Errors
/// [`Error::Metadata`] when the text is not the metadata shape.
pub(crate) fn from_metadata(json: &str, repo: &Path) -> Result<Vec<Target>, Error> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|error| Error::Metadata(error.to_string()))?;
    let packages = value
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| Error::Metadata("no `packages` array".to_owned()))?;
    let mut targets = Vec::new();
    for package in packages {
        let package_name = text(package, "name")?;
        for target in package
            .get("targets")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let name = text(target, "name")?;
            let src_path = PathBuf::from(text(target, "src_path")?);
            let kinds: Vec<&str> = target
                .get("kind")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .collect();
            let (binary_id, kind) = if kinds.iter().any(|k| matches!(*k, "lib" | "proc-macro")) {
                (package_name.to_owned(), Kind::Crate)
            } else if kinds.contains(&"test") {
                (format!("{package_name}::{name}"), Kind::Test)
            } else if kinds.contains(&"bin") {
                (format!("{package_name}::bin/{name}"), Kind::Crate)
            } else {
                continue;
            };
            let Some(root) = relative_to(repo, &src_path) else {
                continue;
            };
            targets.push(Target {
                binary_id,
                root,
                kind,
            });
        }
    }
    Ok(targets)
}

fn text<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, Error> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Error::Metadata(format!("missing string `{key}`")))
}

/// `path` under `repo`, `/`-separated, or `None` outside it.
fn relative_to(repo: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(repo).ok()?;
    let parts: Option<Vec<&str>> = relative
        .components()
        .map(|component| component.as_os_str().to_str())
        .collect();
    Some(parts?.join("/"))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn target(binary_id: &str, root: &str, kind: Kind) -> Target {
        Target {
            binary_id: binary_id.to_owned(),
            root: root.to_owned(),
            kind,
        }
    }

    #[trace("FR-033-AC-15")]
    #[test]
    fn a_library_file_contributes_its_path_as_the_module_path() {
        let lib = target("pkg", "r/crates/p/src/lib.rs", Kind::Crate);
        assert_eq!(lib.module_path("r/crates/p/src/lib.rs"), Some(vec![]));
        assert_eq!(
            lib.module_path("r/crates/p/src/ops/evidence.rs"),
            Some(vec!["ops".to_owned(), "evidence".to_owned()])
        );
        assert_eq!(
            lib.module_path("r/crates/p/src/ops/evidence/mod.rs"),
            Some(vec!["ops".to_owned(), "evidence".to_owned()])
        );
        assert_eq!(lib.module_path("r/crates/p/tests/x.rs"), None);
    }

    #[trace("FR-033-AC-15")]
    #[test]
    fn an_integration_test_owns_only_its_own_file_and_directory() {
        let suite = target("pkg::suite", "r/crates/p/tests/suite.rs", Kind::Test);
        assert_eq!(suite.module_path("r/crates/p/tests/suite.rs"), Some(vec![]));
        assert_eq!(
            suite.module_path("r/crates/p/tests/suite/helper.rs"),
            Some(vec!["helper".to_owned()])
        );
        assert_eq!(suite.module_path("r/crates/p/tests/common/mod.rs"), None);
        assert_eq!(suite.module_path("r/crates/p/tests/other.rs"), None);
    }

    #[trace("FR-033-AC-15")]
    #[test]
    fn metadata_yields_the_binary_ids_nextest_reports() {
        let json = r#"{"packages":[{"name":"pkg","targets":[
            {"name":"pkg","kind":["lib"],"src_path":"/repo/rust/crates/p/src/lib.rs"},
            {"name":"suite","kind":["test"],"src_path":"/repo/rust/crates/p/tests/suite.rs"},
            {"name":"tool","kind":["bin"],"src_path":"/repo/rust/crates/p/src/main.rs"},
            {"name":"b","kind":["bench"],"src_path":"/repo/rust/crates/p/benches/b.rs"}]}]}"#;
        let targets = from_metadata(json, Path::new("/repo")).unwrap();
        let ids: Vec<&str> = targets.iter().map(Target::binary_id).collect();
        assert_eq!(ids, ["pkg", "pkg::suite", "pkg::bin/tool"]);
        assert!(from_metadata("{}", Path::new("/repo")).is_err());
    }
}
