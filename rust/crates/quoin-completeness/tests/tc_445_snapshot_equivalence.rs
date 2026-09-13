// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

// The crate denies `unwrap`, `panic!` and indexing because a request path must
// not take a process down. A test *is* the failure path: a tree that will not
// materialise is a failing test by design. Scoped to this file, never to the
// library.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The disk reader and the snapshot reader answer the same question (quoin#445).
//!
//! `read_bundle_frontmatter(&Path)` walks a bundle itself; `frontmatter_from_sources`
//! parses documents somebody else walked — the shell in `src/core/snapshot.ts`.
//! Which documents that shell chooses to put on the wire is therefore directly
//! verdict-changing, and the failure mode is asymmetric: re-classifying on this
//! side protects only against a pre-filter that is too COARSE, never against one
//! that is too NARROW. A dropped document simply is not there to be re-admitted.
//!
//! agent-ix/quoin#448 is the incident. That PR defended the same boundary with
//! tests that enumerated the pre-filter's own rules; adding one directory name
//! to the walker's exclusion set left 14/14 of them green while the shipped
//! command's verdict went from one finding to zero, exit 0 both ways. A test
//! that restates a filter's rules can only ever agree with the filter.
//!
//! So nothing here enumerates a rule. ONE real tree is materialised and run
//! through BOTH paths, and the results are asserted identical. The snapshot fed
//! to the pure path is built by [`independent_snapshot`] below — a plain
//! recursive read written in this file — and deliberately NOT by calling the
//! crate's own walker, which would make the test agree with itself.
//!
//! Independence is a per-AXIS property, not a property of the file. This
//! differential was blind to SYMLINKS for exactly that reason: both arms
//! classified with `file_type().is_dir()`-else-file, so both followed a link
//! and both agreed, while the TypeScript shell on the other side of the same
//! boundary asked `Dirent.isFile()` — false for every symlink — and dropped it.
//! [`descend`] now classifies from `symlink_metadata`/`metadata` instead, which
//! is a genuinely different question, and [`assert_shapes`] fails if the
//! fixture ever stops materialising a shape it claims to cover — a tree that
//! quietly shrinks would otherwise pass this file over less than it names.

use std::fs;
use std::path::Path;

use quoin_completeness::{DocumentSource, frontmatter_from_sources, read_bundle_frontmatter};
use tempfile::TempDir;

/// A document with parseable frontmatter, so it lands in `documents`.
fn document(title: &str) -> String {
    format!("---\ntitle: {title}\nquality_attribute: security\n---\n\n# {title}\n")
}

/// The bundle root, inside the fixture's temp dir.
///
/// Nested rather than being the temp dir itself so that `shared/` can sit
/// BESIDE it: a symlink whose target lives inside the bundle would be found by
/// the walk anyway, and could not show a walk that loses the link.
const BUNDLE: &str = "bundle";

/// The bundle root of a fixture built by [`bundle`].
fn root_of(temp: &TempDir) -> std::path::PathBuf {
    temp.path().join(BUNDLE)
}

/// Materialise the tree both paths are asked about.
///
/// Every shape here is one a walker could lose without any rule-enumerating
/// test noticing: depth (three levels of nesting), a document at every level, a
/// document whose frontmatter does not parse, a file that is not markdown, a
/// file whose extension is markdown in the WRONG case, a filename holding a
/// space, a filename holding a non-ASCII character, a directory holding nothing
/// at all, a directory NAMED `*.md`, and — on unix — the three symlink shapes:
/// one resolving to a markdown file outside the bundle, one resolving to a
/// directory, and one resolving to nothing.
fn bundle() -> TempDir {
    let temp = TempDir::new().unwrap();
    let root = &temp.path().join(BUNDLE);
    fs::create_dir_all(root).unwrap();

    let write = |relative: &str, contents: &str| {
        let target = root.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&target, contents).unwrap();
    };

    write("index.md", &document("Index"));
    write("level1/a.md", &document("A"));
    write("level1/level2/b.md", &document("B"));
    write("level1/level2/level3/c.md", &document("C"));

    // Unparseable frontmatter: an unterminated flow sequence. Both paths must
    // report it as `unreadable` rather than drop it, because the document that
    // does not parse may be the one carrying the exclusion.
    write(
        "level1/level2/level3/broken.md",
        "---\nquality_attribute: [security\n---\n\nbody\n",
    );

    // NOT markdown, and its bytes are valid frontmatter on purpose: a reader
    // that took it would show up as an extra document rather than as nothing.
    write("level1/notes.txt", &document("Notes"));

    // Markdown extension in the wrong case. Both sides match `.md`
    // case-sensitively; if either ever stopped, this file would appear on one
    // side only.
    write("level1/UPPER.MD", &document("Upper"));

    write("level1/with space.md", &document("With Space"));
    write("level1/caf\u{e9}.md", &document("Caf\u{e9}"));

    fs::create_dir_all(root.join("level1/empty")).unwrap();

    // A DIRECTORY whose name ends in `.md`. The seam's one real divergence
    // lived here: the shell's walk filtered on the extension alone, opened it,
    // and recorded `EISDIR` as an unreadable document the bundle does not
    // contain, while this crate's disk reader matches on the file type first
    // and reports nothing. Both sides now agree that it is not a document, and
    // this is the shape that says so.
    fs::create_dir_all(root.join("level1/notes.md")).unwrap();
    fs::write(root.join("level1/notes.md/inner.txt"), "not a document\n").unwrap();

    materialise_symlinks(temp.path());
    assert_shapes(root);
    temp
}

/// The three symlink shapes, beside the bundle rather than inside it.
///
/// `spec/FR-042.md` symlinked into a shared spec directory is the ordinary
/// monorepo layout, and it is the axis this differential was blind to: both
/// arms used to classify with `file_type().is_dir()`-else-file, so both
/// followed the link and agreed, while the shell across the boundary asked
/// `Dirent.isFile()` — false for every symlink — and dropped the document
/// entirely. A document that does not cross is not merely missing from the
/// request, it is missing from the VERDICT: the value it owned reads `unowned`
/// and a finding appears that this disk reader never produces.
#[cfg(unix)]
fn materialise_symlinks(temp: &Path) {
    let shared = temp.join("shared");
    fs::create_dir_all(shared.join("nested")).unwrap();
    fs::write(shared.join("FR-042.md"), document("Shared FR-042")).unwrap();
    // Inside a directory that is only ever reached THROUGH a symlink, so a walk
    // that descended one would show up as this extra document.
    fs::write(
        shared.join("nested/unreachable.md"),
        document("Unreachable"),
    )
    .unwrap();

    let root = temp.join(BUNDLE);
    // Resolves to a FILE: a document.
    std::os::unix::fs::symlink(shared.join("FR-042.md"), root.join("level1/shared.md")).unwrap();
    // Resolves to a DIRECTORY, and named `*.md`: not a document, not descended.
    std::os::unix::fs::symlink(shared.join("nested"), root.join("level1/linked-dir.md")).unwrap();
    // Resolves to NOTHING: not a document. There are no bytes to read, and
    // reporting it `unreadable` would invent an entry the bundle does not hold.
    std::os::unix::fs::symlink(shared.join("gone.md"), root.join("level1/dangling.md")).unwrap();
}

#[cfg(not(unix))]
fn materialise_symlinks(_temp: &Path) {}

/// Every shape [`bundle`]'s doc comment claims, asserted to be on disk.
///
/// Without this the differential passes over whatever tree it happens to get: a
/// fixture that silently stopped materialising the symlinks — a `create_dir_all`
/// that moved, a target path that drifted — would leave both arms agreeing
/// about a smaller tree and the axis uncovered again, which is precisely how
/// agent-ix/quoin#448 stayed green.
fn assert_shapes(root: &Path) {
    let file = |relative: &str| {
        let meta = fs::symlink_metadata(root.join(relative))
            .unwrap_or_else(|cause| panic!("fixture shape '{relative}' is missing: {cause}"));
        assert!(
            meta.file_type().is_file(),
            "fixture shape '{relative}' is no longer a plain file"
        );
    };
    let directory = |relative: &str| {
        let meta = fs::symlink_metadata(root.join(relative))
            .unwrap_or_else(|cause| panic!("fixture shape '{relative}' is missing: {cause}"));
        assert!(
            meta.file_type().is_dir(),
            "fixture shape '{relative}' is no longer a directory"
        );
    };

    file("index.md");
    file("level1/level2/level3/c.md");
    file("level1/level2/level3/broken.md");
    file("level1/notes.txt");
    file("level1/UPPER.MD");
    file("level1/with space.md");
    file("level1/caf\u{e9}.md");
    directory("level1/empty");
    directory("level1/notes.md");

    #[cfg(unix)]
    {
        let link = |relative: &str, resolves_to_file: bool, resolves_to_dir: bool| {
            let meta = fs::symlink_metadata(root.join(relative))
                .unwrap_or_else(|cause| panic!("fixture shape '{relative}' is missing: {cause}"));
            assert!(
                meta.file_type().is_symlink(),
                "fixture shape '{relative}' is no longer a symlink"
            );
            let resolved = fs::metadata(root.join(relative));
            assert_eq!(
                resolved.as_ref().is_ok_and(std::fs::Metadata::is_file),
                resolves_to_file,
                "fixture shape '{relative}' no longer resolves to a file as claimed"
            );
            assert_eq!(
                resolved.as_ref().is_ok_and(std::fs::Metadata::is_dir),
                resolves_to_dir,
                "fixture shape '{relative}' no longer resolves to a directory as claimed"
            );
        };
        link("level1/shared.md", true, false);
        link("level1/linked-dir.md", false, true);
        link("level1/dangling.md", false, false);
    }
}

/// Every `*.md` under `root`, read from the filesystem, sorted by relative path.
///
/// Written here rather than reached for from the crate: the point of the
/// comparison is that two INDEPENDENT walks of one tree see the same documents.
fn independent_snapshot(root: &Path) -> Vec<DocumentSource> {
    let mut found: Vec<(String, String)> = Vec::new();
    descend(root, Path::new(""), &mut found);
    found.sort();
    found
        .into_iter()
        .map(|(path, raw)| DocumentSource { path, raw })
        .collect()
}

fn descend(root: &Path, prefix: &Path, out: &mut Vec<(String, String)>) {
    let entries = fs::read_dir(root.join(prefix)).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let relative = prefix.join(entry.file_name());
        let absolute = root.join(&relative);

        // Classified from the FILESYSTEM, with two calls that ask two different
        // questions — not from `DirEntry::file_type()`, which is what the crate
        // under test uses. Independence has to hold per AXIS: while both arms
        // classified with `is_dir()`-else-file they agreed about symlinks no
        // matter what either did, and the divergence lived across the boundary
        // where nothing here could see it.
        //
        // `symlink_metadata` never follows, so only a REAL directory is
        // descended and a symlinked one cannot loop the walk out of the tree.
        if fs::symlink_metadata(&absolute).unwrap().is_dir() {
            descend(root, &relative, out);
            continue;
        }
        let path = relative.to_string_lossy().replace('\\', "/");
        // Case-sensitive on purpose, and written out rather than borrowed from
        // the crate: `UPPER.MD` is markdown to neither side, and an
        // `eq_ignore_ascii_case` here would quietly hide a reader that started
        // taking it.
        #[allow(
            clippy::case_sensitive_file_extension_comparisons,
            reason = "the wrong-case extension is a fixture shape under test"
        )]
        let is_markdown = path.ends_with(".md");
        if !is_markdown {
            continue;
        }
        // `metadata` always follows, so a link is a document exactly when it
        // RESOLVES to a file: a shared spec document is one, a directory named
        // `*.md` is not, and a broken link resolves to nothing.
        if !fs::metadata(&absolute).is_ok_and(|meta| meta.is_file()) {
            continue;
        }
        out.push((path, fs::read_to_string(&absolute).unwrap()));
    }
}

/// Trace: FR-037, FR-096
/// Provenance: quoin#445, agent-ix/quoin#448
#[test]
fn tc_445_320_the_disk_reader_and_the_snapshot_reader_agree_on_one_real_tree() {
    let temp = bundle();
    let root = &root_of(&temp);

    let from_disk = read_bundle_frontmatter(root);
    let from_snapshot = frontmatter_from_sources(&independent_snapshot(root), &[]);

    // One equality, covering both leaking axes at once: which FILENAMES cross
    // and which DIRECTORIES are descended into. Neither is restated as a rule.
    assert_eq!(from_disk, from_snapshot);
}

/// Trace: FR-037, FR-096
/// Provenance: quoin#445, agent-ix/quoin#448
#[test]
fn tc_445_321_the_tree_under_test_is_actually_populated() {
    // The floor under the equivalence above. Two paths that both swept NOTHING
    // are also equal, so without this a walker that lost the whole tree would
    // pass the differential.
    let temp = bundle();
    let root = &root_of(&temp);

    let snapshot = independent_snapshot(root);
    assert!(
        snapshot.len() >= 7,
        "the fixture tree yielded {} markdown documents, expected at least 7",
        snapshot.len()
    );

    let read = read_bundle_frontmatter(root);
    assert!(
        read.documents.len() >= 6,
        "the disk reader yielded {} parseable documents, expected at least 6",
        read.documents.len()
    );
    assert!(
        !read.unreadable.is_empty(),
        "the fixture carries a document whose frontmatter does not parse; \
         a reader reporting none has stopped reporting them"
    );

    // And the deepest level is genuinely reached, so "at least 6" cannot be
    // met by six documents from the top two levels.
    assert!(
        read.documents
            .iter()
            .any(|document| document.path == "level1/level2/level3/c.md"),
        "the depth-3 document is missing from {:?}",
        read.documents
            .iter()
            .map(|document| &document.path)
            .collect::<Vec<_>>()
    );
}

/// A symlink is a document exactly when it RESOLVES to a file.
///
/// The equivalence above would catch a disagreement between the two arms, but
/// not a decision both arms got wrong in the same direction — and the decision
/// is verdict-changing on its own: `spec/FR-042.md` symlinked into a shared
/// spec directory is the ordinary monorepo layout, so dropping it silently
/// removes that document's claims and reports the value it owned `unowned`.
/// This states the chosen behaviour outright, in all three directions.
///
/// Trace: FR-037, FR-096
/// Provenance: quoin#445, agent-ix/quoin#448
#[cfg(unix)]
#[test]
fn tc_445_322_a_symlink_is_a_document_only_when_it_resolves_to_a_file() {
    let temp = bundle();
    let root = &root_of(&temp);

    let read = read_bundle_frontmatter(root);
    let paths: Vec<&str> = read
        .documents
        .iter()
        .map(|document| document.path.as_str())
        .chain(read.unreadable.iter().map(|entry| entry.path.as_str()))
        .collect();

    assert!(
        paths.contains(&"level1/shared.md"),
        "a symlink resolving to a markdown file is a document; {paths:?}"
    );
    assert!(
        !paths.contains(&"level1/linked-dir.md"),
        "a symlink resolving to a directory is not a document, and must not be \
         reported unreadable either; {paths:?}"
    );
    assert!(
        !paths.contains(&"level1/dangling.md"),
        "a broken symlink has no bytes, so it is neither a document nor an \
         unreadable one; {paths:?}"
    );
    assert!(
        !paths.iter().any(|path| path.contains("unreachable.md")),
        "a symlinked directory must not be descended; {paths:?}"
    );
}
