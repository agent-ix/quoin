// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The spec's dependency lists and document ids make claims that nothing else
//! checks; this reads them against the repository.
//!
//! * PLAT-1045: no `FR`, `NFR` or `StR` cites a `*.test.ts` file the repository no
//!   longer has.
//! * PLAT-118: every id cited in a `## Dependencies` section resolves to a
//!   spec document, and a live document does not cite a withdrawn one.
//!
//! # What a green run does and does not mean
//!
//! Green means no dependency dangles or names a withdrawn requirement, no id
//! names two documents, and no requirement cites a deleted TypeScript test. It
//! does NOT mean a stated dependency relationship is true: both defects
//! PLAT-118 was filed for cited live requirements correctly and described them
//! wrongly.
//!
//! This census carries no `Trace:` tag: it is an engineering check, not an
//! obligation a requirement states.
//!
//! Provenance: PLAT-1045, PLAT-118

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
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

// ---- PLAT-118 ------------------------------------------------------------

/// Spec documents by id: `true` when the document is withdrawn.
fn spec_documents(root: &Path) -> BTreeMap<String, Vec<(PathBuf, bool)>> {
    let mut paths = Vec::new();
    files_named(&root.join("spec"), ".md", &mut paths);
    let mut documents: BTreeMap<String, Vec<(PathBuf, bool)>> = BTreeMap::new();
    for path in paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let text = std::fs::read_to_string(&path).unwrap();
        // A document not named for its id (`evals.md`) declares it in frontmatter.
        let Some(id) =
            leading_id(name).or_else(|| frontmatter_id(&text).filter(|id| id.starts_with("TM-")))
        else {
            continue;
        };
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
    if !matches!(prefix, "StR" | "US" | "FR" | "NFR" | "IT" | "ADR") {
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

/// Problems in one document's frontmatter `relationships:` targets. Only
/// `ix://agent-ix/quoin/<ID>` targets are read; another repository's ids are
/// not this tree's to resolve.
fn relationship_problems(
    text: &str,
    documents: &BTreeMap<String, Vec<(PathBuf, bool)>>,
) -> Vec<(usize, String)> {
    let mut problems = Vec::new();
    let mut lines = text.lines().enumerate();
    if lines.next().map(|(_, l)| l) != Some("---") {
        return problems;
    }
    for (index, line) in lines {
        if line == "---" {
            break;
        }
        let Some(target) = line
            .trim()
            .strip_prefix("- target:")
            .map(|t| t.trim().trim_matches(['"', '\'']))
            .and_then(|t| t.strip_prefix("ix://agent-ix/quoin/"))
        else {
            continue;
        };
        let target = target.split(['#', '?']).next().unwrap_or(target);
        match documents.get(target) {
            None => problems.push((
                index + 1,
                format!("{target} does not resolve to a document"),
            )),
            Some(found) if found.iter().all(|(_, w)| *w) => {
                problems.push((index + 1, format!("{target} is withdrawn")));
            }
            Some(_) => {}
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
            let found = dependency_problems(&text, &documents)
                .into_iter()
                .chain(relationship_problems(&text, &documents));
            for (line, problem) in found {
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

// ---- PLAT-1273 (spec/ documents only; plan/ ids are a separate census) -----------------------------------------------------------

/// Ids that name more than one spec document, with the files that share them.
fn shared_document_ids(
    documents: &BTreeMap<String, Vec<(PathBuf, bool)>>,
) -> BTreeMap<String, Vec<PathBuf>> {
    documents
        .iter()
        .filter(|(_, found)| found.len() > 1)
        .map(|(id, found)| (id.clone(), found.iter().map(|(p, _)| p.clone()).collect()))
        .collect()
}

#[test]
fn tc_1045_003_no_id_names_two_spec_documents() {
    let documents = spec_documents(&repo_root());
    assert!(
        documents.len() > 100,
        "anti-vacuity: the spec tree must be populated"
    );
    let shared = shared_document_ids(&documents);
    assert!(
        shared.is_empty(),
        "these ids name more than one spec document (allocate the next free id to the newer one \
         and update every reference): {shared:?}"
    );
}

/// Plan and task documents under `plan/` by frontmatter id.
fn plan_documents(root: &Path) -> BTreeMap<String, Vec<(PathBuf, bool)>> {
    let mut paths = Vec::new();
    files_named(&root.join("plan"), ".md", &mut paths);
    let mut documents: BTreeMap<String, Vec<(PathBuf, bool)>> = BTreeMap::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).unwrap();
        if let Some(id) = frontmatter_id(&text) {
            documents.entry(id).or_default().push((path, false));
        }
    }
    documents
}

#[test]
fn tc_1045_011_no_id_names_two_plan_or_task_documents() {
    let documents = plan_documents(&repo_root());
    assert!(
        documents.len() > 50
            && documents.contains_key("PLAN-001")
            && documents.contains_key("TASK-001"),
        "anti-vacuity: the plan tree must be populated, saw {} ids",
        documents.len()
    );
    let shared = shared_document_ids(&documents);
    assert!(
        shared.is_empty(),
        "these ids name more than one plan or task document (allocate the next free id to the \
         newer one and update every reference): {shared:?}"
    );
}

#[test]
fn tc_1045_012_a_plan_or_task_file_carries_its_documents_id() {
    let mut paths = Vec::new();
    files_named(&repo_root().join("plan"), ".md", &mut paths);
    let (mut plans, mut tasks) = (0, 0);
    let mut mismatched = Vec::new();
    for path in &paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let text = std::fs::read_to_string(path).unwrap();
        let Some(id) = frontmatter_id(&text) else {
            continue;
        };
        // A task file is named for its id; a plan's directory is named for the plan's id.
        let named_for = if id.starts_with("TASK-") {
            tasks += 1;
            name.to_owned()
        } else if id.starts_with("PLAN-") {
            plans += 1;
            path.parent()
                .and_then(|d| d.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_owned()
        } else {
            continue;
        };
        if !named_for.starts_with(&format!("{id}-")) {
            mismatched.push(format!("{}: id {id}", path.display()));
        }
    }
    assert!(
        plans >= 10 && tasks >= 50,
        "anti-vacuity: saw {plans} plans and {tasks} tasks"
    );
    assert!(
        mismatched.is_empty(),
        "these plan or task files declare an id their name does not carry: {mismatched:?}"
    );
}

#[test]
fn tc_1045_004_probe_a_planted_shared_id_is_caught() {
    let mut documents = BTreeMap::new();
    documents.insert(
        "FR-046".to_owned(),
        vec![
            (PathBuf::from("FR-046-a.md"), false),
            (PathBuf::from("FR-046-b.md"), false),
        ],
    );
    documents.insert("FR-047".to_owned(), vec![(PathBuf::from("c.md"), false)]);
    let shared = shared_document_ids(&documents);
    assert_eq!(shared.len(), 1);
    assert_eq!(shared["FR-046"].len(), 2);
}

/// The frontmatter `id:` of a document, if it has one.
fn frontmatter_id(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return None;
    }
    lines
        .take_while(|l| *l != "---")
        .find_map(|l| l.strip_prefix("id:"))
        .map(|v| v.trim().trim_matches('"').to_owned())
}

#[test]
fn tc_1045_005_a_documents_frontmatter_id_is_the_id_its_file_name_carries() {
    let root = repo_root();
    let mut drifted = Vec::new();
    for (id, found) in spec_documents(&root) {
        for (path, _) in found {
            let text = std::fs::read_to_string(&path).unwrap();
            if let Some(declared) = frontmatter_id(&text)
                && declared != id
            {
                drifted.push(format!(
                    "{}: file name says {id}, frontmatter says {declared}",
                    path.strip_prefix(&root).unwrap().display()
                ));
            }
        }
    }
    assert!(drifted.is_empty(), "id drift:\n{}", drifted.join("\n"));
}

#[test]
fn tc_1045_006_probe_frontmatter_id_is_read() {
    assert_eq!(
        frontmatter_id("---\nid: FR-9\ntitle: x\n---\n"),
        Some("FR-9".to_owned())
    );
    assert_eq!(frontmatter_id("# no frontmatter\n"), None);
}

#[test]
fn tc_118_004_probe_planted_frontmatter_relationships_fail() {
    let mut documents = BTreeMap::new();
    documents.insert("FR-001".to_owned(), vec![(PathBuf::from("a"), false)]);
    documents.insert("FR-002".to_owned(), vec![(PathBuf::from("b"), true)]);
    let text = "---\nid: NFR-1\nrelationships:\n  - target: \"ix://agent-ix/quoin/FR-001\"\n    type: \"constrains\"\n  - target: \"ix://agent-ix/quoin/FR-002\"\n    type: \"constrains\"\n  - target: \"ix://agent-ix/quoin/FR-999\"\n    type: \"constrains\"\n  - target: \"ix://agent-ix/quire-rs/FR-777\"\n    type: \"constrains\"\n---\n\n- target: \"ix://agent-ix/quoin/FR-999\"\n";
    assert_eq!(
        relationship_problems(text, &documents),
        vec![
            (6, "FR-002 is withdrawn".to_owned()),
            (8, "FR-999 does not resolve to a document".to_owned()),
        ]
    );
}

// ---- PLAT-1275 -----------------------------------------------------------

/// Backticked or bare `*.test.ts` names anywhere in a document: prose, a
/// `Verification` cell or a frontmatter `ref:`.
fn test_ts_mentions(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for word in line.split(|c: char| !(c.is_ascii_alphanumeric() || "-_./".contains(c))) {
            let word = word.trim_end_matches('.');
            if word.ends_with(".test.ts") {
                found.push((index + 1, word.to_owned()));
            }
        }
    }
    found
}

/// Documents whose narrative, verification cells and frontmatter evidence are
/// held to the native suite: everything under the `NFR`, `StR` and `FR` directories. A mention
/// of a deleted TypeScript suite there reads as live evidence. The review
/// records stay outside the set because they narrate the retirement
/// of those suites by name.
fn native_only_documents(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for dir in ["spec/non-functional", "spec/stakeholder", "spec/functional"] {
        files_named(&root.join(dir), ".md", &mut paths);
    }
    paths
}

#[test]
fn tc_1045_007_nfr_str_and_every_fr_cite_no_deleted_typescript_test() {
    let root = repo_root();
    let paths = native_only_documents(&root);
    assert!(
        paths.len() > 20
            && paths
                .iter()
                .any(|p| p.to_string_lossy().contains("FR-066-")),
        "anti-vacuity: expected the NFR and StR documents and every FR including FR-066, saw {}",
        paths.len()
    );
    let mut stale = Vec::new();
    for path in &paths {
        let text = std::fs::read_to_string(path).unwrap();
        for (line, file) in test_ts_mentions(&text) {
            stale.push(format!(
                "{}:{line}: {file}",
                path.strip_prefix(&root).unwrap().display()
            ));
        }
    }
    assert!(
        stale.is_empty(),
        "these documents cite a TypeScript test the repository no longer has (cite the native \
         test, or say none is cited):\n{}",
        stale.join("\n")
    );
}

#[test]
fn tc_1045_008_probe_planted_test_ts_mentions_are_caught() {
    let planted = "---\nevidence:\n  - ref: tests/a-b.test.ts\n---\nSee (`cli.test.ts`, `write.test.ts`).\n\
                   | AC | x | Test (flows.test.ts) |\nA native `tc_1.rs` test. Ends a sentence: old.test.ts.\n";
    assert_eq!(
        test_ts_mentions(planted),
        vec![
            (3, "tests/a-b.test.ts".to_owned()),
            (5, "cli.test.ts".to_owned()),
            (5, "write.test.ts".to_owned()),
            (6, "flows.test.ts".to_owned()),
            (7, "old.test.ts".to_owned()),
        ]
    );
}
