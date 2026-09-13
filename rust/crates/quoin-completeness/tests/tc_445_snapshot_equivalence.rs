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

use std::fs;
use std::path::Path;

use quoin_completeness::{DocumentSource, frontmatter_from_sources, read_bundle_frontmatter};
use tempfile::TempDir;

/// A document with parseable frontmatter, so it lands in `documents`.
fn document(title: &str) -> String {
    format!("---\ntitle: {title}\nquality_attribute: security\n---\n\n# {title}\n")
}

/// Materialise the tree both paths are asked about.
///
/// Every shape here is one a walker could lose without any rule-enumerating
/// test noticing: depth (three levels of nesting), a document at every level, a
/// document whose frontmatter does not parse, a file that is not markdown, a
/// file whose extension is markdown in the WRONG case, a filename holding a
/// space, a filename holding a non-ASCII character, and a directory holding
/// nothing at all.
fn bundle() -> TempDir {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

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

    temp
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
        if entry.file_type().unwrap().is_dir() {
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
        if is_markdown {
            out.push((path, fs::read_to_string(root.join(&relative)).unwrap()));
        }
    }
}

/// Trace: FR-037, FR-096
/// Provenance: quoin#445, agent-ix/quoin#448
#[test]
fn tc_445_320_the_disk_reader_and_the_snapshot_reader_agree_on_one_real_tree() {
    let temp = bundle();
    let root = temp.path();

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
    let root = temp.path();

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
