// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Repository traversal and file classification (quoin#377).
//!
//! A faithful port of `walk`, `shellFiles`, `wiringFiles`, `portable` and
//! `extname` from `src/validators/gates.ts`. The traversal order is part of the
//! contract: `wiredBy` is the *first* wiring file that references a script in
//! full-path sort order, so a different sort is a different payload.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ValidatorError;

/// Directory names never descended into, at any depth.
///
/// Verbatim from `EXCLUDED` in `gates.ts`. `spec` and `vendor` are here because
/// a specification or a vendored tree contains shell that is documentation, not
/// a gate this repository owns.
const EXCLUDED: [&str; 6] = [".git", "dist", "node_modules", "spec", "target", "vendor"];

/// Files a shell gate can be wired from, split by how they are recognised.
pub(crate) struct RepoFiles {
    /// Every `.sh` file, in full-path sort order.
    pub(crate) shell: Vec<PathBuf>,
    /// Every build/CI file, in full-path sort order.
    pub(crate) wiring: Vec<PathBuf>,
}

/// Walk `root` once and classify every file it holds.
///
/// The TypeScript walks twice, once per classification; one walk over a static
/// tree yields the same two lists and halves the syscalls.
pub(crate) fn scan(root: &Path) -> Result<RepoFiles, ValidatorError> {
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort_by(|a, b| {
        a.as_os_str()
            .as_encoded_bytes()
            .cmp(b.as_os_str().as_encoded_bytes())
    });

    let mut shell = Vec::new();
    let mut wiring = Vec::new();
    for path in files {
        if is_shell_file(&path) {
            shell.push(path);
        } else if is_wiring_file(root, &path) {
            wiring.push(path);
        }
    }
    Ok(RepoFiles { shell, wiring })
}

/// Read a file the way Node's `readFileSync(path, "utf8")` does: invalid UTF-8
/// becomes U+FFFD rather than an error, because a gate script with one bad byte
/// still has to be inspected.
pub(crate) fn read_text(path: &Path) -> Result<String, ValidatorError> {
    let bytes = fs::read(path).map_err(|source| ValidatorError::FileUnreadable {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// The repository-relative, `/`-separated form of an absolute walk result.
pub(crate) fn relative_to(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

fn visit(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ValidatorError> {
    let entries = fs::read_dir(dir).map_err(|source| {
        let path = dir.to_path_buf();
        if dir == root {
            ValidatorError::RepoRootUnreadable { path, source }
        } else {
            ValidatorError::DirectoryUnreadable { path, source }
        }
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| ValidatorError::DirectoryUnreadable {
            path: dir.to_path_buf(),
            source,
        })?;
        let file_type =
            entry
                .file_type()
                .map_err(|source| ValidatorError::DirectoryUnreadable {
                    path: dir.to_path_buf(),
                    source,
                })?;
        let name = entry.file_name();

        // `readdirSync(..., { withFileTypes: true })` reports a symlink as
        // neither a directory nor a file, so the TypeScript walks past it. That
        // is also what keeps the walk from following a cycle out of the repo.
        if file_type.is_dir() {
            if EXCLUDED.iter().any(|excluded| name == *excluded) {
                continue;
            }
            visit(root, &dir.join(&name), out)?;
        } else if file_type.is_file() {
            out.push(dir.join(&name));
        }
    }
    Ok(())
}

/// `extname(path) === ".sh"`.
fn is_shell_file(path: &Path) -> bool {
    file_name_of(path).is_some_and(|name| node_extname(name) == ".sh")
}

/// `path.extname` semantics, which differ from [`Path::extension`] in the cases
/// that matter here: a leading dot is not an extension, so a file literally
/// named `.sh` is not a shell script.
fn node_extname(name: &str) -> &str {
    match name.rfind('.') {
        Some(0) | None => "",
        Some(index) => name.get(index..).unwrap_or(""),
    }
}

/// The four wiring shapes from `wiringFiles`, expressed without a regex because
/// each one is a literal or a prefix test and a regex here would only add a
/// fallible compile step to a total function.
#[allow(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "deliberate: the oracle's /\\.ya?ml$/ is case-sensitive, so `ci.YML` is not wiring in the TypeScript and must not become wiring here. Taking clippy's suggestion would be a verdict divergence, not a cleanup."
)]
fn is_wiring_file(root: &Path, path: &Path) -> bool {
    let Some(name) = file_name_of(path) else {
        return false;
    };
    let lowered = name.to_lowercase();

    // `/^makefile(?:\..+)?$/` — bare `Makefile`, or `Makefile.<at least one char>`.
    let is_makefile = lowered == "makefile"
        || lowered
            .strip_prefix("makefile.")
            .is_some_and(|suffix| !suffix.is_empty());
    // `/^taskfile\.(?:ya?ml)$/`
    let is_taskfile = lowered == "taskfile.yml" || lowered == "taskfile.yaml";

    if is_makefile || is_taskfile || lowered == "justfile" || lowered == "package.json" {
        return true;
    }

    // `/^\.github\/workflows\//.test(rel) && /\.ya?ml$/.test(rel)` — note the
    // extension is tested against the whole relative path, so a workflow in a
    // nested directory still counts.
    let relative = relative_to(root, path);
    relative.starts_with(".github/workflows/")
        && (relative.ends_with(".yml") || relative.ends_with(".yaml"))
}

fn file_name_of(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

#[cfg(test)]
mod tests {
    use super::{is_shell_file, node_extname, relative_to};
    use std::path::Path;

    /// Node's `extname` treats a leading dot as part of the name.
    #[test]
    fn tc_377_007_extname_matches_node() {
        assert_eq!(node_extname("gate.sh"), ".sh");
        assert_eq!(node_extname(".sh"), "");
        assert_eq!(node_extname("a.b.sh"), ".sh");
        assert_eq!(node_extname("Makefile"), "");
        assert_eq!(node_extname("gate.SH"), ".SH");
        assert!(is_shell_file(Path::new("/r/gate.sh")));
        assert!(!is_shell_file(Path::new("/r/.sh")));
        assert!(!is_shell_file(Path::new("/r/gate.bash")));
    }

    /// Relative paths are repo-rooted and `/`-separated.
    #[test]
    fn tc_377_008_relative_paths_are_portable() {
        assert_eq!(
            relative_to(Path::new("/repo"), Path::new("/repo/scripts/gate.sh")),
            "scripts/gate.sh"
        );
    }

    /// Each of the five wiring shapes, and the near-misses next to them.
    #[test]
    fn tc_377_009_wiring_shapes() {
        let root = Path::new("/repo");
        let wired = |relative: &str| super::is_wiring_file(root, &root.join(relative));
        assert!(wired("Makefile"));
        assert!(wired("makefile"));
        assert!(wired("Makefile.ci"));
        assert!(wired("justfile"));
        assert!(wired("package.json"));
        assert!(wired("Taskfile.yml"));
        assert!(wired("Taskfile.yaml"));
        assert!(wired(".github/workflows/ci.yml"));
        assert!(wired(".github/workflows/nested/ci.yaml"));

        assert!(!wired("Makefile."));
        assert!(!wired("Taskfile.toml"));
        assert!(!wired("ci/workflows/ci.yml"));
        assert!(!wired(".github/actions/ci.yml"));
        assert!(!wired(".github/workflows/README.md"));
        assert!(!wired("package-lock.json"));
    }
}
