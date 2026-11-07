// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The traceability matrix and the spec's dependency lists make claims that
//! nothing else checks; this reads them against the repository.
//!
//! * PLAT-1045: a matrix row marked `✅`/`🚧` must not cite a `*.test.ts` file
//!   that is not in the repository. A `✅` against a deleted test reads as
//!   discharged coverage and keeps the criterion from being picked up as work.
//! * PLAT-119: a `TC-` id names one test case. Two rows sharing an id make
//!   `Traces To` ambiguous for every resolver that joins on it.
//! * PLAT-118: every id cited in a `## Dependencies` section resolves to a
//!   spec document, and a live document does not cite a withdrawn one.
//!
//! # What a green run does and does not mean
//!
//! Green means no matrix row cites a missing test file, no TC id is shared,
//! and no dependency dangles or names a withdrawn requirement. It does NOT
//! mean the matrix is complete, that a cited test asserts its criterion, or
//! that a stated dependency relationship is true. Those stay review
//! obligations: both defects PLAT-118 was filed for cited live requirements
//! correctly and described them wrongly.
//!
//! This census carries no `Trace:` tag: it is an engineering check, not an
//! obligation a requirement states.
//!
//! Provenance: PLAT-1045, PLAT-119, PLAT-118

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root")
}

fn files_named(root: &Path, extension: &str, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root).expect("readable directory") {
        let path = entry.expect("a directory entry").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_dir = std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir());
        if is_dir {
            if !matches!(name, "target" | "node_modules" | ".git" | "dist" | ".venv") {
                files_named(&path, extension, into);
            }
        } else if name.ends_with(extension) {
            into.push(path);
        }
    }
}

// ---- PLAT-1045 -----------------------------------------------------------

/// Backticked `*.test.ts` names cited on a matrix row.
fn cited_test_files(row: &str) -> Vec<String> {
    row.split('`')
        .skip(1)
        .step_by(2)
        .filter(|span| span.ends_with(".test.ts"))
        .map(str::to_owned)
        .collect()
}

/// A row claims coverage when any cell begins with `✅` or `🚧`.
fn claims_coverage(row: &str) -> bool {
    row.starts_with('|')
        && row
            .split('|')
            .any(|cell| cell.trim_start().starts_with('✅') || cell.trim_start().starts_with('🚧'))
}

/// `(line, file)` for every covered row citing a test file not in `present`.
fn missing_cited_tests(matrix: &str, present: &BTreeSet<String>) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (index, row) in matrix.lines().enumerate() {
        if !claims_coverage(row) {
            continue;
        }
        for file in cited_test_files(row) {
            // A bare name is matched on its file name; a path must match as a suffix.
            let exists = present
                .iter()
                .any(|have| have == &file || have.ends_with(&format!("/{file}")));
            if !exists {
                found.push((index + 1, file));
            }
        }
    }
    found
}

#[test]
fn tc_1045_001_no_covered_matrix_row_cites_a_missing_test_file() {
    let root = repo_root();
    let matrix = std::fs::read_to_string(root.join("spec/matrix.md")).expect("spec/matrix.md");
    let mut ts = Vec::new();
    files_named(&root, ".test.ts", &mut ts);
    let present: BTreeSet<String> = ts
        .iter()
        .map(|p| {
            p.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();

    assert!(
        matrix.lines().filter(|l| l.starts_with("| TC-")).count() > 100,
        "anti-vacuity: the matrix must still carry its test-case rows"
    );
    let missing = missing_cited_tests(&matrix, &present);
    assert!(
        missing.is_empty(),
        "spec/matrix.md marks these rows covered although the cited test file does not exist \
         (mark them `❌ Unbacked` or cite a real test): {missing:?}"
    );
}

#[test]
fn tc_1045_002_probe_a_planted_missing_citation_is_caught() {
    let present = BTreeSet::new();
    let planted = "| TC-1 | title | Unit | P0 | FR-1-AC-1 | ✅ `ghost.test.ts` |\n\
                   | TC-2 | title | Unit | P0 | FR-1-AC-2 | ❌ Unbacked — `ghost.test.ts` |";
    assert_eq!(
        missing_cited_tests(planted, &present),
        vec![(1, "ghost.test.ts".to_owned())],
        "only the covered row is a violation; an Unbacked row may name the file it lost"
    );
}

// ---- PLAT-119 ------------------------------------------------------------

fn duplicated_tc_ids(matrix: &str) -> BTreeMap<String, Vec<usize>> {
    let mut seen: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, row) in matrix.lines().enumerate() {
        if let Some(rest) = row.strip_prefix("| TC-") {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if !digits.is_empty() && rest[digits.len()..].starts_with(' ') {
                seen.entry(format!("TC-{digits}"))
                    .or_default()
                    .push(index + 1);
            }
        }
    }
    seen.retain(|_, lines| lines.len() > 1);
    seen
}

#[test]
fn tc_119_001_no_tc_id_names_two_rows() {
    let matrix = std::fs::read_to_string(repo_root().join("spec/matrix.md")).unwrap();
    let duplicated = duplicated_tc_ids(&matrix);
    assert!(
        duplicated.is_empty(),
        "these TC ids appear on more than one matrix row: {duplicated:?}"
    );
}

#[test]
fn tc_119_002_probe_a_planted_duplicate_is_caught() {
    let planted = "| TC-7 | a | Unit | P0 | FR-1-AC-1 | ✅ |\n| TC-70 | b | Unit | P0 | FR-1-AC-2 | ✅ |\n| TC-7 | c | Unit | P0 | FR-1-AC-3 | ✅ |";
    let duplicated = duplicated_tc_ids(planted);
    assert_eq!(duplicated.len(), 1);
    assert_eq!(duplicated["TC-7"], vec![1, 3]);
}

// ---- PLAT-118 ------------------------------------------------------------

/// Spec documents by id: `true` when the document is withdrawn.
fn spec_documents(root: &Path) -> BTreeMap<String, Vec<(PathBuf, bool)>> {
    let mut paths = Vec::new();
    files_named(&root.join("spec"), ".md", &mut paths);
    let mut documents: BTreeMap<String, Vec<(PathBuf, bool)>> = BTreeMap::new();
    for path in paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let Some(id) = leading_id(name) else { continue };
        let text = std::fs::read_to_string(&path).unwrap();
        documents
            .entry(id)
            .or_default()
            .push((path, is_withdrawn(&text)));
    }
    documents
}

/// `FR-046` from `FR-046-clause-discharge.md`.
fn leading_id(name: &str) -> Option<String> {
    let (prefix, rest) = name.split_once('-')?;
    if !matches!(prefix, "StR" | "US" | "FR" | "NFR" | "IT") {
        return None;
    }
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (!digits.is_empty() && rest[digits.len()..].starts_with('-'))
        .then(|| format!("{prefix}-{digits}"))
}

/// Withdrawn means the `⛔ Withdrawn` banner sits directly under the
/// document's own `# <ID>:` heading, not that the word appears somewhere.
fn is_withdrawn(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let body = if lines.first() == Some(&"---") {
        lines
            .iter()
            .skip(1)
            .position(|l| *l == "---")
            .map_or(0, |end| end + 2)
    } else {
        0
    };
    lines[body..]
        .iter()
        .position(|l| l.starts_with("# "))
        .map(|at| at + body)
        .is_some_and(|heading| {
            lines[heading + 1..]
                .iter()
                .take(3)
                .any(|l| l.contains("⛔ Withdrawn"))
        })
}

/// Spec ids cited on one line, minus those that name another repository's
/// requirement (`quire-rs FR-053`, `ix://agent-ix/<other>/FR-053`).
fn local_citations(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let boundary = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
        let prefix = ["StR-", "US-", "NFR-", "FR-", "IT-"]
            .into_iter()
            .find(|p| line[at..].starts_with(p));
        if let (true, Some(prefix)) = (boundary, prefix) {
            let digits: String = line[at + prefix.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if !digits.is_empty() {
                let before = &line[..at];
                let after = &line[at + prefix.len() + digits.len()..];
                let foreign = (after.starts_with("](ix://agent-ix/")
                    && !after.starts_with("](ix://agent-ix/quoin/"))
                    || before
                        .trim_end_matches(['[', ' ', '`'])
                        .ends_with("quire-rs")
                    || before.ends_with("spec-artifacts-process` ")
                    || before.rfind("ix://agent-ix/").is_some_and(|i| {
                        !before[i..].starts_with("ix://agent-ix/quoin/")
                            && !before[i..].contains(')')
                    });
                if !foreign {
                    found.push(format!("{prefix}{digits}"));
                }
                at += prefix.len() + digits.len();
                continue;
            }
        }
        at += line[at..].chars().next().map_or(1, char::len_utf8);
    }
    found
}

/// Problems in one document's `## Dependencies` section.
fn dependency_problems(
    text: &str,
    documents: &BTreeMap<String, Vec<(PathBuf, bool)>>,
) -> Vec<(usize, String)> {
    let mut problems = Vec::new();
    let mut inside = false;
    for (index, line) in text.lines().enumerate() {
        if line.starts_with("## ") {
            inside = line.trim() == "## Dependencies";
            continue;
        }
        if !inside {
            continue;
        }
        // A sentence that says an id is deliberately not cited names it correctly.
        let explains_absence = line.to_ascii_lowercase().contains("not cited");
        for id in local_citations(line) {
            match documents.get(&id) {
                None => problems.push((index + 1, format!("{id} does not resolve to a document"))),
                Some(found) if found.iter().all(|(_, w)| *w) && !explains_absence => {
                    problems.push((index + 1, format!("{id} is withdrawn")));
                }
                Some(_) => {}
            }
        }
    }
    problems
}

#[test]
fn tc_118_001_dependency_citations_resolve_and_are_live() {
    let root = repo_root();
    let documents = spec_documents(&root);
    assert!(
        documents.len() > 100,
        "anti-vacuity: the spec tree must be populated"
    );
    let mut checked_sections = 0;
    let mut problems = Vec::new();
    for found in documents.values() {
        for (path, withdrawn) in found {
            let text = std::fs::read_to_string(path).unwrap();
            if text.lines().any(|l| l.trim() == "## Dependencies") {
                checked_sections += 1;
            }
            // A withdrawn document's list is history, not a live claim.
            if *withdrawn {
                continue;
            }
            for (line, problem) in dependency_problems(&text, &documents) {
                problems.push(format!(
                    "{}:{line}: {problem}",
                    path.strip_prefix(&root).unwrap().display()
                ));
            }
        }
    }
    assert!(
        checked_sections > 20,
        "anti-vacuity: expected many Dependencies sections, saw {checked_sections}"
    );
    assert!(
        problems.is_empty(),
        "dependency problems:\n{}",
        problems.join("\n")
    );
}

#[test]
fn tc_118_002_probe_planted_dangling_and_withdrawn_citations_fail() {
    let mut documents = BTreeMap::new();
    documents.insert("FR-001".to_owned(), vec![(PathBuf::from("a"), false)]);
    documents.insert("FR-002".to_owned(), vec![(PathBuf::from("b"), true)]);
    let text = "## Dependencies\n\n- Upstream: FR-001, FR-999\n- Upstream: FR-002\n\
                - FR-002 is withdrawn and deliberately not cited here.\n\
                - Upstream: FR-001 (see also withdrawn FR-002)\n\
                - quire-rs [FR-777](ix://agent-ix/quire-rs/FR-777)\n## Next\n- FR-999\n";
    let found = dependency_problems(text, &documents);
    assert_eq!(
        found,
        vec![
            (3, "FR-999 does not resolve to a document".to_owned()),
            (4, "FR-002 is withdrawn".to_owned()),
            (6, "FR-002 is withdrawn".to_owned()),
        ]
    );
}

#[test]
fn tc_118_003_withdrawal_is_the_banner_under_the_heading() {
    assert!(is_withdrawn(
        "---\nid: FR-1\n---\n\n# FR-1: x\n\n> **⛔ Withdrawn — 2026.**\n"
    ));
    assert!(!is_withdrawn(
        "# FR-1: x\n\nBody.\n\nSome row says ⛔ Withdrawn much later\n\n\n\n\nmore\n⛔ Withdrawn\n"
    ));
}
