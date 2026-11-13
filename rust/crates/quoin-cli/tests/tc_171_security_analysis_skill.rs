// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `spec-security-analysis` skill writes typed `control` and
//! `scope_exclusion` artifacts that map to installed clause sets (FR-171,
//! FR-172).
//!
//! The skill is LLM-run, so the expected output for the synthetic repository
//! under `tests/fixtures/security-analysis` is committed and checked here: it
//! validates with `quire validate`, every mapping resolves to a clause of an
//! installed clause set, and the review records the gaps as findings. The owning
//! modules are read in place or from the checkouts next to this repository;
//! nothing of them is copied here. Each variable follows the convention of the
//! existing tests that read the same module: `SPEC_OBJECTS_SECURITY_DIR` names the
//! `spec_objects_security` module directory (as in `tc_1515_soup_promote`) and
//! `SPEC_ARTIFACTS_PROCESS_DIR` names the repository root (as in
//! `tc_136_anomaly_command`). Validation needs `quire` on `PATH`.
//!
//! FR-172-AC-3 and FR-172-AC-4 need a live skill run over a synthetic licensed
//! pack; they are demonstration scenarios (`TC-EV-061`, `TC-EV-062` in
//! `spec/evals.md`) and no test here is tagged with them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;
use serde_json::Value;

fn repo_root() -> PathBuf {
    // Canonicalised: `quire` rejects `..` in a document path.
    std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))
        .expect("repository root")
}

fn skill_dir() -> PathBuf {
    repo_root().join("skills/spec-security-analysis")
}

fn fixture() -> PathBuf {
    repo_root().join("tests/fixtures/security-analysis")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn skill() -> String {
    read(&skill_dir().join("SKILL.md"))
}

fn module(var: &str, sibling: &str, probe: &str) -> PathBuf {
    let root = std::env::var_os(var).map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../..")
                .join(sibling)
        },
        PathBuf::from,
    );
    // Canonicalised: `quire` rejects `..` in `--module`.
    let root = std::fs::canonicalize(&root).unwrap_or_else(|error| {
        panic!(
            "{sibling} is not at {}: {error}. Check out agent-ix/{sibling} next to this \
             repository (at or after PLAT-1427), or set {var}",
            root.display()
        )
    });
    assert!(
        root.join(probe).is_file(),
        "{sibling} lacks {probe} at {}",
        root.display()
    );
    root
}

/// The `spec_objects_security` module directory.
fn security_module() -> PathBuf {
    module(
        "SPEC_OBJECTS_SECURITY_DIR",
        "spec-objects-security/spec_objects_security",
        "skeletons/scope_exclusion.md",
    )
}

/// The `spec_artifacts_process` module directory (it ships `SpecReview`).
fn process_module() -> PathBuf {
    let root = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    let dir = std::fs::canonicalize(root.join("spec_artifacts_process")).expect(
        "spec-artifacts-process checkout: set SPEC_ARTIFACTS_PROCESS_DIR to its repository root",
    );
    assert!(dir.join("manifest.yaml").is_file());
    dir
}

fn validate(module: &Path, file: &Path) {
    let output = Command::new("quire")
        .args(["validate", "--module"])
        .arg(module)
        .arg(file)
        .output()
        .expect("quire on PATH");
    assert!(
        output.status.success(),
        "{}: {}",
        file.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn md_files(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    found.sort();
    found
}

/// Every clause id of every clause set the module declares, by set key.
fn installed_clause_sets() -> BTreeMap<String, BTreeSet<String>> {
    let module = security_module();
    let manifest = quoin_yaml::from_str(&read(&module.join("manifest.yaml"))).unwrap();
    let files: Vec<String> = manifest["clause_sets"]
        .as_array()
        .expect("the module declares clause sets")
        .iter()
        .map(|f| f.as_str().unwrap().to_owned())
        .collect();
    assert!(!files.is_empty(), "the module declares clause sets");
    let mut sets = BTreeMap::new();
    for file in files {
        let doc: Value = serde_json::from_str(&read(&module.join(&file))).unwrap();
        let key = format!(
            "{}/{}@{}",
            doc["authority"].as_str().unwrap(),
            doc["id"].as_str().unwrap(),
            doc["version"].as_str().unwrap()
        );
        let ids = doc["clauses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap().to_owned())
            .collect();
        sets.insert(key, ids);
    }
    sets
}

/// The `(set, clause)` rows of a control's `## Mappings` table.
fn mapping_rows(text: &str) -> Vec<(String, String)> {
    let section = text.split("\n## Mappings\n").nth(1).unwrap_or("");
    let section = section.split("\n## ").next().unwrap_or("");
    section
        .lines()
        .filter(|l| l.starts_with('|'))
        .skip(2)
        .map(|l| {
            let cells: Vec<&str> = l
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            assert_eq!(
                cells.len(),
                2,
                "a mapping row is exactly Set and Clause: {l}"
            );
            (cells[0].to_owned(), cells[1].to_owned())
        })
        .collect()
}

/// The mapping rows that do not resolve to a clause of an installed set.
fn unresolved(rows: &[(String, String)]) -> Vec<(String, String)> {
    let sets = installed_clause_sets();
    rows.iter()
        .filter(|(set, clause)| !sets.get(set).is_some_and(|ids| ids.contains(clause)))
        .cloned()
        .collect()
}

/// Words of `text`, split on anything that cannot be part of a clause id, with
/// trailing sentence punctuation dropped.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "-().".contains(c)))
        .map(|w| w.trim_end_matches(['.', ',']).to_owned())
        .filter(|w| !w.is_empty())
        .collect()
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `AC-2` or `AC-2(1)`: two capitals, a hyphen, a number, an optional enhancement.
fn is_nist_id(w: &str) -> bool {
    let Some((family, rest)) = w.split_once('-') else {
        return false;
    };
    let number = rest.split_once('(').map_or(rest, |(n, enh)| {
        if enh.strip_suffix(')').is_some_and(digits) {
            n
        } else {
            ""
        }
    });
    family.len() == 2 && family.bytes().all(|b| b.is_ascii_uppercase()) && digits(number)
}

fn dotted_triple(w: &str) -> bool {
    let parts: Vec<&str> = w.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| digits(p))
}

/// `V8.2.1`: a `V` and three dotted numbers.
fn is_asvs_id(w: &str) -> bool {
    w.strip_prefix('V').is_some_and(dotted_triple)
}

/// The ASVS requirement ids a file cites, each as `V<chapter>.<section>.<item>`.
/// Prose cites them with the `V`; a chapter pack's table names them bare
/// (`6.1.1`) in its first column, which is read only in those packs because a
/// bare dotted triple elsewhere is a version string.
fn asvs_ids_cited(file: &Path, text: &str) -> BTreeSet<String> {
    let mut ids: BTreeSet<String> = words(text).into_iter().filter(|w| is_asvs_id(w)).collect();
    let in_chapter_pack = file
        .parent()
        .is_some_and(|d| d.file_name().is_some_and(|n| n == "owasp-asvs"))
        && file
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('V'));
    if in_chapter_pack {
        for line in text.lines().filter(|l| l.starts_with('|')) {
            let first = line
                .trim_start_matches('|')
                .split('|')
                .next()
                .unwrap()
                .trim();
            if dotted_triple(first) {
                ids.insert(format!("V{first}"));
            }
        }
    }
    ids
}

/// The frontmatter `id` of an authored artifact.
fn artifact_id(path: &Path) -> String {
    let text = read(path);
    let front = text
        .strip_prefix("---\n")
        .unwrap()
        .split_once("\n---")
        .unwrap()
        .0;
    quoin_yaml::from_str(front).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Each `Rev` token with the version that follows it, such as `Rev 5.2.0`.
fn revisions(text: &str) -> BTreeSet<String> {
    let words = words(text);
    words
        .windows(2)
        .filter(|w| w[0] == "Rev")
        .map(|w| format!("Rev {}", w[1]))
        .collect()
}

#[trace("FR-171-AC-1")]
#[test]
fn skill_text_writes_typed_artifacts_and_no_section_19() {
    let text = skill();
    assert!(!text.contains("Section 19"), "no Section 19 instruction");
    assert!(!text.contains("Compliance Standards Traceability"));
    assert!(text.contains("`control`") && text.contains("`scope_exclusion`"));
    assert!(text.contains("quoin write"));
}

#[trace("FR-171-AC-2")]
#[test]
fn shipped_templates_and_examples_validate() {
    let module = security_module();
    let assets = skill_dir().join("assets");
    let files = md_files(&assets);
    assert_eq!(
        files.len(),
        4,
        "template and example for control and scope_exclusion"
    );
    for file in files {
        validate(&module, &file);
    }
}

#[trace("FR-171-AC-3", "FR-171-AC-5", "FR-172-AC-5", "FR-172-AC-6")]
#[test]
fn skill_text_instructs_the_defaults_the_reply_and_the_set_check() {
    let text = skill();
    for needle in [
        "`not_assessed`",
        "write no exclusion",
        "no responsible party",
        "no covering requirement",
        "quire validate",
        "quoin module list",
        "not mapped, set not installed",
        "Never supply the text, title or summary",
        "Licensed standards come from an installed pack only",
        "nist/sp800-53@5.2.0",
        "owasp/asvs@5.0.0",
    ] {
        assert!(text.contains(needle), "SKILL.md lacks: {needle}");
    }
}

#[trace("FR-171-AC-7")]
#[test]
fn skill_names_nist_as_rev_5_2_0_and_no_other_revision() {
    let text = skill();
    assert!(text.contains("NIST SP 800-53 | Rev 5.2.0"));
    assert_eq!(revisions(&text), BTreeSet::from(["Rev 5.2.0".to_owned()]));
}

#[trace("FR-171-AC-6")]
#[test]
fn every_cited_nist_and_asvs_id_is_a_clause_of_the_cited_release() {
    let sets = installed_clause_sets();
    let nist = &sets["nist/sp800-53@5.2.0"];
    let asvs = &sets["owasp/asvs@5.0.0"];
    let mut files = md_files(&skill_dir().join("assets"));
    files.extend(md_files(&skill_dir().join("references")));
    files.push(skill_dir().join("SKILL.md"));
    for dir in ["nist-800-53", "owasp-asvs"] {
        files.extend(md_files(&skill_dir().join("references").join(dir)));
    }
    let families: BTreeSet<&str> = nist.iter().filter_map(|id| id.split('-').next()).collect();
    for file in files {
        let text = read(&file);
        for id in words(&text).into_iter().filter(|w| is_nist_id(w)) {
            let family = id.split('-').next().unwrap();
            assert!(
                !families.contains(family) || nist.contains(&id),
                "{}: {id} is not a NIST SP 800-53 Rev 5.2.0 control",
                file.display()
            );
        }
        for id in asvs_ids_cited(&file, &text) {
            assert!(
                asvs.contains(&id),
                "{}: {id} is not an ASVS 5.0.0 requirement",
                file.display()
            );
        }
    }
}

/// The expected output of the skill for the synthetic fixture repository.
#[trace("FR-171-AC-2")]
#[test]
fn expected_fixture_output_validates_and_every_mapping_resolves() {
    let expected = fixture().join("expected");
    let security = security_module();
    let controls = md_files(&expected.join("spec/security"));
    assert_eq!(controls.len(), 4, "three controls and one scope_exclusion");
    for file in &controls {
        validate(&security, file);
    }
    for file in md_files(&fixture().join("repo/spec/components")) {
        validate(&security, &file);
    }
    for file in md_files(&expected.join("reviews")) {
        validate(&process_module(), &file);
    }

    let requirement_ids: BTreeSet<String> = md_files(&fixture().join("repo/spec/functional"))
        .iter()
        .map(|p| artifact_id(p))
        .collect();
    let mut mapped = 0;
    for file in controls.iter().filter(|p| {
        p.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("CTRL-")
    }) {
        let text = read(file);
        let rows = mapping_rows(&text);
        assert!(!rows.is_empty(), "{} maps nothing", file.display());
        assert_eq!(unresolved(&rows), vec![], "{}", file.display());
        mapped += rows.len();
        assert!(text.contains("\n## Effectiveness\n\n`not_assessed`\n"));
        let linked: Vec<&str> = text
            .lines()
            .filter_map(|l| l.trim().strip_prefix("- target: "))
            .collect();
        assert!(
            !linked.is_empty(),
            "{} implements no requirement",
            file.display()
        );
        for id in linked {
            assert!(
                requirement_ids.contains(id),
                "{id} is not a fixture requirement"
            );
        }
    }
    assert_eq!(mapped, 9);

    // No mapping for a source whose set is not installed.
    let all: String = controls.iter().map(|p| read(p)).collect();
    assert!(!all.contains("soc2") && !all.contains("iso27001") && !all.contains("gdpr"));
}

#[trace("FR-171-AC-2")]
#[test]
fn the_exclusion_names_a_bundle_artifact_and_a_clause_that_resolves() {
    let text = read(&fixture().join("expected/spec/security/SCOPEX-001-physical-access.md"));
    let section = |name: &str| -> String {
        let body = text.split(&format!("\n## {name}\n")).nth(1).unwrap();
        body.split("\n## ")
            .next()
            .unwrap()
            .trim()
            .trim_matches('`')
            .to_owned()
    };
    let rows = [(section("Set"), section("Clause"))];
    assert_eq!(unresolved(&rows), vec![]);
    let responsible = section("Responsible");
    assert!(!responsible.contains("ix://"), "a bare bundle id");
    let component_ids: BTreeSet<String> = md_files(&fixture().join("repo/spec/components"))
        .iter()
        .map(|p| artifact_id(p))
        .collect();
    assert!(
        component_ids.contains(&responsible),
        "{responsible} is a bundle artifact"
    );
}

#[test]
fn a_mapping_to_a_clause_no_installed_set_holds_does_not_resolve() {
    let rows = vec![
        ("nist/sp800-53@5.2.0".to_owned(), "AC-3".to_owned()),
        ("nist/sp800-53@5.2.0".to_owned(), "AC-9999".to_owned()),
        ("example.invalid/std@1".to_owned(), "EX-1".to_owned()),
    ];
    assert_eq!(unresolved(&rows), rows[1..].to_vec());
}

#[test]
fn the_review_records_each_gap_as_a_finding() {
    let review = fixture().join("expected/reviews");
    let text = read(&md_files(&review)[0]);
    let findings = text.split("\n## Findings\n").nth(1).unwrap();
    let findings = findings.split("\n## ").next().unwrap();
    for needle in ["SC-28", "CP-9", "not mapped, set not installed"] {
        assert!(findings.contains(needle), "no finding for {needle}");
    }
    assert!(findings.contains("no requirement covers it"));
    assert!(findings.contains("no responsible party is known"));
}

#[test]
fn no_expected_mapping_row_carries_clause_text() {
    // A mapping row is exactly Set and Clause; the fixture adds no clause title.
    let sets = installed_clause_sets();
    assert!(sets.contains_key("nist/sp800-53@5.2.0"));
    for file in md_files(&fixture().join("expected/spec/security")) {
        let text = read(&file);
        for row in mapping_rows(&text) {
            assert!(!row.1.contains(' '), "a clause id, not text: {}", row.1);
        }
    }
}
