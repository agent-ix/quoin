// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Repository traversal and file classification (quoin#377).
//!
//! A faithful port of `walk`, `shellFiles`, `wiringFiles`, `portable` and
//! `extname` from `src/validators/gates.ts`. The traversal order is part of the
//! contract: `wiredBy` is the *first* wiring file that references a script in
//! full-path sort order, so a different sort is a different payload.
//!
//! Classification ([`is_shell_file`], [`is_wiring_file`]) takes a
//! repository-relative path rather than an absolute one, so it is the same
//! function whether the paths came off a disk walk or out of a `quoin-core`
//! request (quoin#412). It is deliberately on THIS side of the boundary: Node's
//! `extname` semantics, the `/^makefile(?:\..+)?$/` shape and the
//! case-sensitive `/\.ya?ml$/` are the subtle half of the port, and a caller
//! that pre-filters its walk can only ever send a superset — it cannot decide
//! a verdict.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ValidatorError;
use crate::source::RepoSource;

/// Directory names never descended into, at any depth.
///
/// Verbatim from `EXCLUDED` in `gates.ts`. `spec` and `vendor` are here because
/// a specification or a vendored tree contains shell that is documentation, not
/// a gate this repository owns.
pub(crate) const EXCLUDED: [&str; 6] = [".git", "dist", "node_modules", "spec", "target", "vendor"];

/// A repository on disk, read lazily.
///
/// The [`RepoSource`] the library's own entry point runs on, so the golden
/// corpus exercises the analysis the boundary also runs. Bodies are cached
/// after their first read; the TypeScript re-reads the same wiring body once
/// per candidate script, and caching only ever removes a re-read of a file
/// already opened, so the set of files opened is unchanged.
pub struct DiskRepo<'a> {
    root: &'a Path,
    cache: BTreeMap<String, String>,
}

impl<'a> DiskRepo<'a> {
    /// Read the repository rooted at `root`.
    #[must_use]
    pub fn new(root: &'a Path) -> Self {
        Self {
            root,
            cache: BTreeMap::new(),
        }
    }
}

impl RepoSource for DiskRepo<'_> {
    fn paths(&mut self) -> Result<Vec<String>, ValidatorError> {
        scan(self.root)
    }

    fn text(&mut self, path: &str) -> Result<String, ValidatorError> {
        if let Some(cached) = self.cache.get(path) {
            return Ok(cached.clone());
        }
        let text = read_text(&self.root.join(path))?;
        self.cache.insert(path.to_owned(), text.clone());
        Ok(text)
    }
}

/// Walk `root` once and return every file it holds, repository-relative and
/// `/`-separated, in full-path sort order.
///
/// The TypeScript walks twice, once per classification; one walk over a static
/// tree yields the same list and halves the syscalls.
///
/// # Errors
///
/// [`ValidatorError::RepoRootUnreadable`] when `root` is not a listable
/// directory, [`ValidatorError::DirectoryUnreadable`] when a directory inside
/// it cannot be listed.
pub(crate) fn scan(root: &Path) -> Result<Vec<String>, ValidatorError> {
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    // Sorted as ABSOLUTE byte paths, then made relative, rather than sorted as
    // relative strings: every entry shares the root prefix, so the order is the
    // same, and this keeps the comparison on the bytes the walk produced rather
    // than on a lossy string conversion.
    files.sort_by(|a, b| {
        a.as_os_str()
            .as_encoded_bytes()
            .cmp(b.as_os_str().as_encoded_bytes())
    });
    Ok(files
        .iter()
        .map(|path| relative_to(root, path))
        .collect::<Vec<String>>())
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

/// Is any segment of this repository-relative path an [`EXCLUDED`] directory?
///
/// The walk never descends into one, so [`scan`] can never produce such a path.
/// It is applied again to whatever a caller sends, because the exclusion is a
/// rule of the analysis and not an optimisation of the walk: a snapshot that
/// happens to include `vendor/gate.sh` must reach the same verdict as a disk
/// walk that skipped it.
#[must_use]
pub(crate) fn is_excluded(relative: &str) -> bool {
    let mut segments = relative.split('/');
    // The last segment is the file name, which is never a directory here.
    segments.next_back();
    segments.any(|segment| EXCLUDED.contains(&segment))
}

/// `extname(path) === ".sh"`, on a repository-relative path.
#[must_use]
pub(crate) fn is_shell_file(relative: &str) -> bool {
    node_extname(file_name_of(relative)) == ".sh"
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
#[must_use]
pub(crate) fn is_wiring_file(relative: &str) -> bool {
    let lowered = file_name_of(relative).to_lowercase();

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
    relative.starts_with(".github/workflows/")
        && (relative.ends_with(".yml") || relative.ends_with(".yaml"))
}

fn file_name_of(relative: &str) -> &str {
    relative.rsplit('/').next().unwrap_or(relative)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{is_excluded, is_shell_file, is_wiring_file, node_extname, relative_to};
    use std::path::Path;

    /// Node's `extname` treats a leading dot as part of the name.
    #[test]
    fn tc_377_007_extname_matches_node() {
        assert_eq!(node_extname("gate.sh"), ".sh");
        assert_eq!(node_extname(".sh"), "");
        assert_eq!(node_extname("a.b.sh"), ".sh");
        assert_eq!(node_extname("Makefile"), "");
        assert_eq!(node_extname("gate.SH"), ".SH");
        assert!(is_shell_file("gate.sh"));
        assert!(!is_shell_file(".sh"));
        assert!(!is_shell_file("gate.bash"));
    }

    /// Relative paths are repo-rooted and `/`-separated.
    #[test]
    fn tc_377_008_relative_paths_are_portable() {
        assert_eq!(
            relative_to(Path::new("/repo"), Path::new("/repo/scripts/gate.sh")),
            "scripts/gate.sh"
        );
    }

    /// An excluded directory anywhere in the path removes the file, and the
    /// file name itself is never read as a directory.
    #[test]
    fn tc_412_005_excluded_directories_are_excluded_at_any_depth() {
        assert!(is_excluded("vendor/gate.sh"));
        assert!(is_excluded("a/node_modules/b/gate.sh"));
        assert!(is_excluded("target/debug/Makefile"));
        assert!(!is_excluded("scripts/gate.sh"));
        assert!(!is_excluded("Makefile"));
        // A FILE called `vendor` is not an excluded directory.
        assert!(!is_excluded("vendor"));
        // The prefix must be a whole segment.
        assert!(!is_excluded("vendored/gate.sh"));
    }

    /// Each of the five wiring shapes, and the near-misses next to them.
    #[test]
    fn tc_377_009_wiring_shapes() {
        assert!(is_wiring_file("Makefile"));
        assert!(is_wiring_file("makefile"));
        assert!(is_wiring_file("Makefile.ci"));
        assert!(is_wiring_file("justfile"));
        assert!(is_wiring_file("package.json"));
        assert!(is_wiring_file("Taskfile.yml"));
        assert!(is_wiring_file("Taskfile.yaml"));
        assert!(is_wiring_file(".github/workflows/ci.yml"));
        assert!(is_wiring_file(".github/workflows/nested/ci.yaml"));

        assert!(!is_wiring_file("Makefile."));
        assert!(!is_wiring_file("Taskfile.toml"));
        assert!(!is_wiring_file("ci/workflows/ci.yml"));
        assert!(!is_wiring_file(".github/actions/ci.yml"));
        assert!(!is_wiring_file(".github/workflows/README.md"));
        assert!(!is_wiring_file("package-lock.json"));
        // `.github/workflows/` must be at the ROOT of the relative path, the
        // same as the oracle's anchored `/^\.github\/workflows\//`.
        assert!(!is_wiring_file("vendor/.github/workflows/ci.yml"));
    }
}
