// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

// The crates deny `panic!` and indexing because a request path must not take a
// process down. A test *is* the failure path: a golden that will not load is a
// failing test by design, and an out-of-range index is the assertion doing its
// job. Scoped to this file, never to the library.
#![allow(
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]

//! Parity with the TypeScript oracle, over the committed golden corpus.
//!
//! Every expectation here was captured once from `src/completeness/` by
//! `scripts/capture-semantic-goldens.mjs` and committed under `tests/goldens/`
//! with the quoin revision it ran at. **Nothing in this file invokes Node.**
//!
//! Verdicts, finding kinds, severities and rollup counts are compared exactly.
//! Message text is not: it is human-readable detail, and the port's acceptance
//! criterion says so.

use std::collections::BTreeMap;
use std::path::Path;

use quoin_completeness::{
    ArtifactTypeName, AssessInput, AssessOptions, CompletenessFinding, DocumentClaims,
    DocumentSource, FindingKind, FrontmatterField, ModuleSource, SchemaSource, Severity, Verdict,
    VocabularyDeclaration, VocabularyName, VocabularyValue, assess_bundle, assess_sources,
    assess_vocabulary, claims_for, load_vocabulary_coverage, locate_module_root,
    read_bundle_frontmatter, schema_refs_of, verdict_for, written_reason_for,
};
use serde_json::Value;

fn golden(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(name);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("golden {name} must be readable: {error}"),
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => panic!("golden {name} must be JSON: {error}"),
    }
}

fn array<'a>(value: &'a Value, key: &str) -> &'a Vec<Value> {
    match value.get(key).and_then(Value::as_array) {
        Some(items) => items,
        None => panic!("golden has no array at {key}"),
    }
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn declaration_from(value: &Value) -> VocabularyDeclaration {
    VocabularyDeclaration {
        name: VocabularyName::new(text(value, "name")),
        from: ArtifactTypeName::new(text(value, "from")),
        field: FrontmatterField::new(text(value, "field")),
        check: text(value, "check").to_owned(),
        justified_absence_field: value
            .get("justifiedAbsenceField")
            .and_then(Value::as_str)
            .map(str::to_owned),
        values: strings(value, "values"),
        module_name: text(value, "moduleName").to_owned(),
    }
}

/// `(kind, severity, value, document)` — the identity a finding is compared on.
fn finding_identity(finding: &CompletenessFinding) -> (String, String, String, String) {
    (
        finding.kind.as_str().to_owned(),
        finding.severity.as_str().to_owned(),
        finding.value.as_str().to_owned(),
        finding.document.clone().unwrap_or_default(),
    )
}

fn golden_finding_identity(value: &Value) -> (String, String, String, String) {
    (
        text(value, "kind").to_owned(),
        text(value, "severity").to_owned(),
        text(value, "value").to_owned(),
        text(value, "document").to_owned(),
    )
}

/// Rebuild a file tree from a golden's `{ path: contents }` map.
fn materialize(root: &Path, files: &Value) {
    let Some(map) = files.as_object() else {
        return;
    };
    for (relative, body) in map {
        let path = root.join(relative);
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            panic!("mkdir {}: {error}", parent.display());
        }
        if let Err(error) = std::fs::write(&path, body.as_str().unwrap_or_default()) {
            panic!("write {}: {error}", path.display());
        }
    }
}

/// Trace: FR-037
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_300_written_reason_matches_the_oracle() {
    let corpus = golden("written-reason.json");
    let cases = array(&corpus, "cases");
    assert!(cases.len() >= 20, "the corpus must be a population");

    let mut found = 0usize;
    let mut rejected = 0usize;
    for case in cases {
        let id = text(case, "id");
        let expected = case.get("reason").and_then(Value::as_str);
        let actual = written_reason_for(text(case, "value"), text(case, "body"));
        assert_eq!(expected, actual.as_deref(), "{id}");
        if expected.is_some() {
            found += 1;
        } else {
            rejected += 1;
        }
    }
    // A check over an empty population is not evidence: both outcomes must occur.
    assert!(found >= 4, "only {found} corpus cases carry a real reason");
    assert!(rejected >= 8, "only {rejected} corpus cases are refused");
}

/// Trace: FR-037
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_301_assess_vocabulary_matches_the_oracle() {
    let corpus = golden("assess-vocabulary.json");
    let cases = array(&corpus, "cases");
    assert!(cases.len() >= 10);

    let mut kinds_seen = std::collections::BTreeSet::new();
    for case in cases {
        let id = text(case, "id");
        let declaration = declaration_from(case.get("declaration").unwrap_or(&Value::Null));
        let documents: Vec<DocumentClaims> = array(case, "documents")
            .iter()
            .map(|d| DocumentClaims {
                path: text(d, "path").to_owned(),
                claims: strings(d, "claims"),
                excuses: strings(d, "excuses"),
                body: text(d, "body").to_owned(),
            })
            .collect();

        let assessed = assess_vocabulary(&declaration, &documents);

        let expected_rollup = case.get("rollup").unwrap_or(&Value::Null);
        assert_eq!(
            expected_rollup.get("declared").and_then(Value::as_u64),
            Some(assessed.rollup.declared as u64),
            "{id}: declared"
        );
        assert_eq!(
            expected_rollup.get("owned").and_then(Value::as_u64),
            Some(assessed.rollup.owned as u64),
            "{id}: owned"
        );
        assert_eq!(
            expected_rollup.get("excused").and_then(Value::as_u64),
            Some(assessed.rollup.excused as u64),
            "{id}: excused"
        );
        assert_eq!(
            expected_rollup.get("unowned").and_then(Value::as_u64),
            Some(assessed.rollup.unowned as u64),
            "{id}: unowned"
        );

        let mut expected: Vec<_> = array(case, "findings")
            .iter()
            .map(golden_finding_identity)
            .collect();
        let mut actual: Vec<_> = assessed.findings.iter().map(finding_identity).collect();
        for finding in &assessed.findings {
            kinds_seen.insert(finding.kind);
        }
        expected.sort();
        actual.sort();
        assert_eq!(expected, actual, "{id}: findings");
    }

    for kind in [
        FindingKind::Unowned,
        FindingKind::UnjustifiedExclusion,
        FindingKind::UndeclaredExclusion,
    ] {
        assert!(
            kinds_seen.contains(&kind),
            "no corpus case produces {kind:?}; a check over an empty population is not evidence"
        );
    }
}

/// Trace: FR-037
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_302_verdict_matches_the_oracle_across_the_whole_grid() {
    let corpus = golden("verdict.json");
    let cases = array(&corpus, "cases");
    assert_eq!(
        cases.len(),
        24,
        "the grid is 3 counts x 2 strictness x 4 finding sets"
    );

    let mut verdicts_seen = std::collections::BTreeSet::new();
    for case in cases {
        let id = text(case, "id");
        let findings: Vec<CompletenessFinding> = array(case, "findings")
            .iter()
            .map(|f| CompletenessFinding {
                vocabulary: VocabularyName::new(text(f, "vocabulary")),
                value: VocabularyValue::new(text(f, "value")),
                kind: match text(f, "kind") {
                    "unowned" => FindingKind::Unowned,
                    "unjustified-exclusion" => FindingKind::UnjustifiedExclusion,
                    "undeclared-exclusion" => FindingKind::UndeclaredExclusion,
                    other => panic!("{id}: unknown finding kind {other}"),
                },
                severity: match text(f, "severity") {
                    "medium" => Severity::Medium,
                    "high" => Severity::High,
                    other => panic!("{id}: unknown severity {other}"),
                },
                message: text(f, "message").to_owned(),
                document: None,
            })
            .collect();

        let strict = case
            .get("strict")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        let checked = case
            .get("vocabulariesChecked")
            .and_then(Value::as_u64)
            .unwrap_or_default() as usize;
        let verdict = verdict_for(&findings, strict, checked);
        verdicts_seen.insert(verdict);
        assert_eq!(text(case, "verdict"), verdict.as_str(), "{id}");
    }

    for verdict in [
        Verdict::Pass,
        Verdict::Conditional,
        Verdict::Fail,
        Verdict::Unchecked,
    ] {
        assert!(
            verdicts_seen.contains(&verdict),
            "the grid never produces {verdict:?}"
        );
    }
}

/// Trace: FR-037
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_303_bundle_frontmatter_matches_the_oracle() {
    let corpus = golden("bundle-frontmatter.json");
    let Ok(temp) = tempfile::tempdir() else {
        panic!("tempdir");
    };
    materialize(temp.path(), corpus.get("files").unwrap_or(&Value::Null));

    let read = read_bundle_frontmatter(temp.path());

    let expected_paths: Vec<&str> = array(&corpus, "documents")
        .iter()
        .map(|d| text(d, "path"))
        .collect();
    let actual_paths: Vec<&str> = read.documents.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(expected_paths, actual_paths, "documents read, in order");

    let expected_unreadable: Vec<&str> = array(&corpus, "unreadable")
        .iter()
        .map(|d| text(d, "path"))
        .collect();
    let actual_unreadable: Vec<&str> = read.unreadable.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(
        expected_unreadable, actual_unreadable,
        "documents reported unreadable"
    );
    assert!(
        !expected_unreadable.is_empty(),
        "the corpus must exercise the unreadable path"
    );

    // D4 (../../quoin-semantic/DIVERGENCE.md section 6). A check over an empty
    // population is not evidence: the ambiguous-scalar documents must still be
    // in the corpus, and `017`/`010`/`0b101` must still be the tokens where a
    // YAML 1.1 reader would disagree with this oracle. Under YAML 1.1 `017`
    // and `010` are octal integers and `0b101` is the integer 5; under the
    // YAML 1.2 core schema the first two are decimal integers (so they are
    // dropped from a string-typed field) and the third is a plain string (so
    // it is kept). Read them with libyaml and tc_378_304's advisory verdict
    // flips from PASS to FAIL.
    let Some(ambiguous) = array(&corpus, "claims")
        .iter()
        .find(|c| text(c, "path") == "ambiguous/claims.md")
    else {
        panic!("the ambiguous-scalar document must be in the corpus");
    };
    let ambiguous_claims = strings(ambiguous, "claims");
    for token in ["on", "no", "1:30", "2026-09-12", "0b101"] {
        assert!(
            ambiguous_claims.iter().any(|c| c == token),
            "the oracle keeps '{token}' as a claim, so this reader must too"
        );
    }
    for token in ["017", "010"] {
        assert!(
            !ambiguous_claims.iter().any(|c| c == token),
            "the oracle resolves '{token}' to a number and drops it, so this \
             reader must too"
        );
    }

    // Bodies are compared because the written-reason search runs over them.
    for (expected, actual) in array(&corpus, "documents").iter().zip(&read.documents) {
        assert_eq!(text(expected, "body"), actual.body, "{}", actual.path);
    }

    let declaration = VocabularyDeclaration {
        name: VocabularyName::from("quality-characteristics"),
        from: ArtifactTypeName::from("NFR"),
        field: FrontmatterField::from("quality_attribute"),
        check: "vocabulary-coverage".to_owned(),
        justified_absence_field: Some("quality_attributes_not_applicable".to_owned()),
        values: vec![
            "security".to_owned(),
            "reliability".to_owned(),
            "safety".to_owned(),
            "compliance".to_owned(),
        ],
        module_name: "iso".to_owned(),
    };
    let claims = claims_for(&read.documents, &declaration);
    let expected_claims: Vec<(String, Vec<String>, Vec<String>)> = array(&corpus, "claims")
        .iter()
        .map(|c| {
            (
                text(c, "path").to_owned(),
                strings(c, "claims"),
                strings(c, "excuses"),
            )
        })
        .collect();
    let actual_claims: Vec<(String, Vec<String>, Vec<String>)> = claims
        .iter()
        .map(|c| (c.path.clone(), c.claims.clone(), c.excuses.clone()))
        .collect();
    assert_eq!(expected_claims, actual_claims, "projected claims");
}

/// Trace: FR-037
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_304_assess_bundle_matches_the_oracle_end_to_end() {
    let corpus = golden("assess-bundle.json");
    let Ok(temp) = tempfile::tempdir() else {
        panic!("tempdir");
    };
    let bundle_root = temp.path().join("bundle");
    let module_root = temp.path().join("module");
    materialize(
        &bundle_root,
        corpus.get("bundleFiles").unwrap_or(&Value::Null),
    );
    materialize(
        &module_root,
        corpus.get("moduleFiles").unwrap_or(&Value::Null),
    );

    // Declarations first: the resolved/unresolved split is where a silently
    // dropped vocabulary would hide.
    let loaded = load_vocabulary_coverage(std::slice::from_ref(&module_root));
    let expected_declarations = corpus.get("declarations").unwrap_or(&Value::Null);
    let expected_names: Vec<&str> = array(expected_declarations, "declarations")
        .iter()
        .map(|d| text(d, "name"))
        .collect();
    let actual_names: Vec<&str> = loaded
        .declarations
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(expected_names, actual_names, "resolved declarations");

    let expected_unresolved: Vec<&str> = array(expected_declarations, "unresolved")
        .iter()
        .map(|d| text(d, "name"))
        .collect();
    let actual_unresolved: Vec<&str> = loaded.unresolved.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(expected_unresolved, actual_unresolved, "unresolved");
    assert_eq!(
        expected_unresolved.len(),
        2,
        "the corpus must exercise both unresolved reasons"
    );

    let expected_values: Vec<Vec<String>> = array(expected_declarations, "declarations")
        .iter()
        .map(|d| strings(d, "values"))
        .collect();
    let actual_values: Vec<Vec<String>> = loaded
        .declarations
        .iter()
        .map(|d| d.values.clone())
        .collect();
    assert_eq!(expected_values, actual_values, "resolved vocabularies");

    for case in array(&corpus, "cases") {
        let id = text(case, "id");
        let strict = case
            .get("strict")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        let has_modules = case
            .get("hasModules")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        let root_exists = case
            .get("bundleRootExists")
            .and_then(Value::as_bool)
            .unwrap_or_default();

        let assessment = assess_bundle(&AssessOptions {
            bundle_root: if root_exists {
                bundle_root.clone()
            } else {
                temp.path().join("does-not-exist")
            },
            strict,
            module_roots: if has_modules {
                vec![module_root.clone()]
            } else {
                Vec::new()
            },
        });

        let expected = case.get("assessment").unwrap_or(&Value::Null);
        assert_eq!(
            text(expected, "verdict"),
            assessment.verdict.as_str(),
            "{id}: verdict"
        );

        let expected_vocabularies: Vec<&str> = expected
            .get("vocabularies")
            .and_then(Value::as_array)
            .map(|v| v.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let actual_vocabularies: Vec<&str> = assessment
            .vocabularies
            .iter()
            .map(VocabularyName::as_str)
            .collect();
        assert_eq!(
            expected_vocabularies, actual_vocabularies,
            "{id}: vocabularies"
        );

        let expected_findings: Vec<_> = array(expected, "findings")
            .iter()
            .map(golden_finding_identity)
            .collect();
        let actual_findings: Vec<_> = assessment.findings.iter().map(finding_identity).collect();
        assert_eq!(
            expected_findings, actual_findings,
            "{id}: findings, in sorted order"
        );

        let expected_rollups: BTreeMap<String, (u64, u64, u64, u64)> = array(expected, "rollups")
            .iter()
            .map(|r| {
                (
                    text(r, "vocabulary").to_owned(),
                    (
                        r.get("declared")
                            .and_then(Value::as_u64)
                            .unwrap_or_default(),
                        r.get("owned").and_then(Value::as_u64).unwrap_or_default(),
                        r.get("excused").and_then(Value::as_u64).unwrap_or_default(),
                        r.get("unowned").and_then(Value::as_u64).unwrap_or_default(),
                    ),
                )
            })
            .collect();
        let actual_rollups: BTreeMap<String, (u64, u64, u64, u64)> = assessment
            .rollups
            .iter()
            .map(|r| {
                (
                    r.vocabulary.as_str().to_owned(),
                    (
                        r.declared as u64,
                        r.owned as u64,
                        r.excused as u64,
                        r.unowned as u64,
                    ),
                )
            })
            .collect();
        assert_eq!(expected_rollups, actual_rollups, "{id}: rollups");

        let expected_unreadable: Vec<&str> = array(expected, "unreadable")
            .iter()
            .map(|d| text(d, "path"))
            .collect();
        let actual_unreadable: Vec<&str> = assessment
            .unreadable
            .iter()
            .map(|d| d.path.as_str())
            .collect();
        assert_eq!(expected_unreadable, actual_unreadable, "{id}: unreadable");
    }
}

/// Trace: FR-096
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_305_the_goldens_name_the_revision_they_were_captured_from() {
    for name in [
        "written-reason.json",
        "assess-vocabulary.json",
        "verdict.json",
        "bundle-frontmatter.json",
        "assess-bundle.json",
    ] {
        let golden = golden(name);
        let provenance = golden.get("provenance").unwrap_or(&Value::Null);
        assert_eq!(
            text(provenance, "quoinRevision").len(),
            40,
            "{name}: provenance must name a full quoin revision"
        );
    }
}

/// Every `*.md` under `root`, as the command shell reads them: relative paths
/// with `/` separators, sorted, content as text.
///
/// This is the walk `src/commands/completeness.ts` performs before it calls
/// `completeness.assess_bundle`. It lives in the test because the library half
/// must not own it (quoin#445) and the criterion — that moving the walk out
/// does not move the answer — has to exercise the same order the shell uses.
fn sources_under(root: &Path) -> Vec<DocumentSource> {
    fn walk(root: &Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(root.join(prefix)) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            // Case-sensitive on purpose, exactly as `markdown_under` and the
            // TypeScript `entry.endsWith(".md")` filter are: a walk that saw
            // more documents than the oracle did would not be the same walk.
            #[allow(
                clippy::case_sensitive_file_extension_comparisons,
                reason = "parity with the oracle's own filter is the point"
            )]
            let is_markdown = relative.ends_with(".md");
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => walk(root, &relative, out),
                Ok(_) if is_markdown => out.push(relative),
                _ => {}
            }
        }
    }
    let mut relative: Vec<String> = Vec::new();
    walk(root, "", &mut relative);
    relative.sort();
    relative
        .into_iter()
        .filter_map(|path| {
            std::fs::read_to_string(root.join(&path))
                .ok()
                .map(|raw| DocumentSource { path, raw })
        })
        .collect()
}

/// One module as the command shell reads it: the manifest, plus exactly the
/// frontmatter schemas the manifest names.
fn module_under(root: &Path) -> Option<ModuleSource> {
    let module_root = locate_module_root(root)?;
    let manifest = std::fs::read_to_string(module_root.join("manifest.yaml")).ok()?;
    let schemas = schema_refs_of(&manifest)
        .into_iter()
        .map(|reference| {
            let source = match std::fs::read_to_string(module_root.join(&reference)) {
                Ok(text) => SchemaSource::Text(text),
                Err(cause) => SchemaSource::Unreadable(cause.to_string()),
            };
            (reference, source)
        })
        .collect();
    Some(ModuleSource {
        label: module_root.to_string_lossy().into_owned(),
        manifest,
        schemas,
    })
}

/// Trace: FR-037, FR-096
/// Provenance: agent-ix/quoin#445
///
/// The content-in entry point answers what the filesystem entry point answers.
///
/// Not a fixture this test invented: the corpus is the one
/// `scripts/capture-semantic-goldens.mjs` captured from the TypeScript oracle,
/// and `tc_378_304` has already pinned the filesystem path to the oracle's
/// answers over it. Agreeing with that path over the same corpus is therefore
/// agreement with the oracle, which is what makes moving the walk into the
/// command shell a no-op for the user.
#[test]
fn tc_445_310_the_content_in_path_answers_what_the_filesystem_path_answers() {
    let corpus = golden("assess-bundle.json");
    let Ok(temp) = tempfile::tempdir() else {
        panic!("tempdir");
    };
    let bundle_root = temp.path().join("bundle");
    let module_root = temp.path().join("module");
    materialize(
        &bundle_root,
        corpus.get("bundleFiles").unwrap_or(&Value::Null),
    );
    materialize(
        &module_root,
        corpus.get("moduleFiles").unwrap_or(&Value::Null),
    );

    let mut cases = 0usize;
    for case in array(&corpus, "cases") {
        let id = text(case, "id");
        let strict = case
            .get("strict")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        let has_modules = case
            .get("hasModules")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        let root_exists = case
            .get("bundleRootExists")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        let root = if root_exists {
            bundle_root.clone()
        } else {
            temp.path().join("does-not-exist")
        };
        let module_roots = if has_modules {
            vec![module_root.clone()]
        } else {
            Vec::new()
        };

        let by_path = assess_bundle(&AssessOptions {
            bundle_root: root.clone(),
            strict,
            module_roots: module_roots.clone(),
        });
        let by_content = assess_sources(&AssessInput {
            bundle_root: root.to_string_lossy().into_owned(),
            strict,
            documents: sources_under(&root),
            unreadable: Vec::new(),
            modules: module_roots
                .iter()
                .filter_map(|r| module_under(r))
                .collect(),
        });

        assert_eq!(by_path, by_content, "{id}");
        cases += 1;
    }

    // A pass over an empty population proves nothing, and this program has
    // shipped that four times.
    assert!(
        cases >= 4,
        "the corpus must carry cases; it carried {cases}"
    );
    assert!(
        !sources_under(&bundle_root).is_empty(),
        "the corpus bundle must carry documents"
    );
    assert!(
        module_under(&module_root).is_some_and(|m| !m.schemas.is_empty()),
        "the corpus module must name frontmatter schemas"
    );
}
