// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence facts-from-mappings`, `record-discharge` and
//! `discharge --rmm` over a corpus of typed controls (FR-173, FR-174, NFR-038).
//!
//! Generated minimal security-record archetypes keep the tests independent of
//! another repository. The synthetic clause set `example.invalid/std@1` carries
//! a clause-text sentinel so the rendered and stored chain can be checked for
//! accidental disclosure.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

mod pack_support;

const SET: &str = "example.invalid/std@1";
const AS_OF: &str = "2026-08-15T00:00:00Z";

fn attestation() -> Value {
    json!({"attestedBy": "reviewer-1", "authority": "security lead",
        "attestedAt": "2026-08-01T00:00:00Z", "expiresAt": "2026-09-01T00:00:00Z"})
}

fn binding(clauses: &[(&str, &str)]) -> Value {
    json!({
        "schemaVersion": "clause-binding-v1",
        "clauseSet": {"authority": "example.invalid", "id": "std", "version": "1"},
        "context": {},
        "clauses": clauses.iter().map(|(id, outcome)| json!({
            "clauseId": id, "force": "mandatory", "outcome": outcome,
            "reasons": [], "expectedOutputs": []})).collect::<Vec<_>>(),
    })
}

/// A control artifact; `effectiveness` is `None` for a control that states none.
fn control(id: &str, effectiveness: Option<&str>, mappings: &[(&str, &str)]) -> String {
    let mut rows = String::new();
    for (set, clause) in mappings {
        writeln!(rows, "| {set} | {clause} |").unwrap();
    }
    let effectiveness = effectiveness.map_or_else(String::new, |value| {
        format!("## Effectiveness\n\n`{value}`\n\n")
    });
    format!(
        "---\nid: {id}\ntitle: \"Control{id}\"\ntype: control\nobject: control\n---\n# [{id}] Control{id}\n\n## Mappings\n\n| Set | Clause |\n|---|---|\n{rows}\n{effectiveness}## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| control_id | UUID | 1..1 | identity |\n| effectiveness | String | 1..1 | minLength: 1 |\n\n## Invariants\n\n### EffectivenessIsStated\n\n```ocl\ncontext Control{id}\ninv EffectivenessIsStated:\n  self.effectiveness->notEmpty()\n```\n"
    )
}

fn exclusion(id: &str, set: &str, clause: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: \"Exclusion{id}\"\ntype: scope_exclusion\nobject: scope_exclusion\n---\n# [{id}] Exclusion{id}\n\n## Set\n\n`{set}`\n\n## Clause\n\n`{clause}`\n\n## Responsible\n\n`COMP-001`\n\n## Rationale\n\n`The platform owns this clause.`\n"
    )
}

struct World {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl World {
    fn new(binding: &Value, files: &[(&str, String)]) -> Self {
        let world = Self {
            dir: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(world.spec()).unwrap();
        for (name, text) in files {
            std::fs::write(world.spec().join(name), text).unwrap();
        }
        std::fs::write(world.path("binding.json"), binding.to_string()).unwrap();
        std::fs::write(world.path("attestation.json"), attestation().to_string()).unwrap();
        synthetic_types_module(&world.path("modules"));
        world
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn spec(&self) -> PathBuf {
        self.path("spec")
    }

    fn run(&self, arguments: &[&str], stdin: Option<&str>) -> Output {
        self.run_with_installed_modules(arguments, stdin, false)
    }

    fn run_with_installed_modules(
        &self,
        arguments: &[&str],
        stdin: Option<&str>,
        installed: bool,
    ) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
        command
            .args(arguments)
            .env("IX_HOME", self.home.path())
            .current_dir(self.dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if installed {
            command.env_remove("IX_FILAMENT_MODULES_PATH");
        } else {
            command.env("IX_FILAMENT_MODULES_PATH", self.path("modules"));
        }
        let mut child = command.spawn().unwrap();
        if let Some(stdin) = stdin {
            std::io::Write::write_all(&mut child.stdin.take().unwrap(), stdin.as_bytes()).unwrap();
        }
        child.wait_with_output().unwrap()
    }

    fn facts_with(&self, extra: &[&str]) -> Output {
        let binding = self.path("binding.json");
        let attestation = self.path("attestation.json");
        let spec = self.spec();
        let mut arguments = vec![
            "evidence",
            "facts-from-mappings",
            "--binding",
            binding.to_str().unwrap(),
            "--attestation",
            attestation.to_str().unwrap(),
            "--spec",
            spec.to_str().unwrap(),
        ];
        arguments.extend_from_slice(extra);
        self.run(&arguments, None)
    }

    fn facts(&self) -> Output {
        self.facts_with(&[])
    }

    fn facts_with_installed_modules(&self, extra: &[&str]) -> Output {
        let binding = self.path("binding.json");
        let attestation = self.path("attestation.json");
        let spec = self.spec();
        let mut arguments = vec![
            "evidence",
            "facts-from-mappings",
            "--binding",
            binding.to_str().unwrap(),
            "--attestation",
            attestation.to_str().unwrap(),
            "--spec",
            spec.to_str().unwrap(),
        ];
        arguments.extend_from_slice(extra);
        self.run_with_installed_modules(&arguments, None, true)
    }

    fn record(&self, facts: &str) -> Output {
        let binding = self.path("binding.json");
        self.run(
            &[
                "evidence",
                "record-discharge",
                "--repo",
                self.dir.path().to_str().unwrap(),
                "--binding",
                binding.to_str().unwrap(),
                "--facts",
                "-",
            ],
            Some(facts),
        )
    }

    fn record_installed(&self, facts: &str) -> Output {
        let binding = self.path("binding.json");
        self.run_with_installed_modules(
            &[
                "evidence",
                "record-discharge",
                "--repo",
                self.dir.path().to_str().unwrap(),
                "--binding",
                binding.to_str().unwrap(),
                "--facts",
                "-",
            ],
            Some(facts),
            true,
        )
    }

    fn rmm(&self, as_of: &str) -> String {
        let binding = self.path("binding.json");
        let output = self.run(
            &[
                "discharge",
                "--binding",
                binding.to_str().unwrap(),
                "--repo",
                self.dir.path().to_str().unwrap(),
                "--as-of",
                as_of,
                "--rmm",
            ],
            None,
        );
        assert!(output.status.success(), "{output:?}");
        out(&output)
    }

    fn rmm_installed(&self, as_of: &str) -> String {
        let binding = self.path("binding.json");
        let output = self.run_with_installed_modules(
            &[
                "discharge",
                "--binding",
                binding.to_str().unwrap(),
                "--repo",
                self.dir.path().to_str().unwrap(),
                "--as-of",
                as_of,
                "--rmm",
            ],
            None,
            true,
        );
        assert!(output.status.success(), "{output:?}");
        out(&output)
    }

    fn install_pack(&self, source: &Path) {
        let source = format!("path:{}", source.display());
        let output = self.run(&["module", "install", &source], None);
        assert!(output.status.success(), "{output:?}");
    }

    fn install_synthetic_clause_set(&self) {
        let pack_source = synthetic_pack_source_dir();
        let pack = synthetic_pack(pack_source.path());
        self.install_pack(&pack);
        let module_root = self
            .home
            .path()
            .join("filament/modules/synthetic-discharge-pack");
        let root = quoin_quire::ModuleRoot::open(&module_root).unwrap();
        let reference = quoin_quire::ClauseSetRef {
            authority: quoin_quire::ClauseSetAuthority::new("example.invalid").unwrap(),
            id: quoin_quire::ClauseSetId::new("std").unwrap(),
            version: quoin_quire::ClauseSetVersion::new("1").unwrap(),
        };
        let binding = quoin_quire::clauses::evaluate(
            &root,
            &reference,
            &quoin_quire::clauses::Context::default(),
            &mut Vec::new(),
        )
        .unwrap();
        std::fs::write(
            self.path("binding.json"),
            serde_json::to_string(&binding).unwrap(),
        )
        .unwrap();
    }

    fn discharge_files(&self) -> Vec<String> {
        let mut pending = vec![self.path("spec/evidence/discharge")];
        let mut contents = Vec::new();
        while let Some(directory) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                } else if let Ok(text) = std::fs::read_to_string(path) {
                    contents.push(text);
                }
            }
        }
        contents
    }
}

fn synthetic_pack_source_dir() -> tempfile::TempDir {
    let outside_worktree = |directory: &Path| {
        directory
            .ancestors()
            .all(|ancestor| !ancestor.join(".git").exists())
    };
    let candidate = std::env::var_os("RUNNER_TEMP")
        .map(PathBuf::from)
        .filter(|directory| directory.is_dir() && outside_worktree(directory))
        .or_else(|| {
            let directory = PathBuf::from("/run/user/1000");
            (directory.is_dir() && outside_worktree(&directory)).then_some(directory)
        })
        .unwrap_or_else(pack_support::clean_base);
    tempfile::tempdir_in(candidate).expect("pack source is outside git")
}

fn synthetic_object_types() -> &'static str {
    r"object_types:
  - name: control
    data_schema: {type: object}
    body_extraction:
      yield_pattern:
        match:
          id: {from: frontmatter_field, path: [id], required: true}
          title: {from: frontmatter_field, path: [title], required: true}
          type: {from: frontmatter_field, path: [type], required: true}
  - name: scope_exclusion
    data_schema: {type: object}
    body_extraction:
      yield_pattern:
        match:
          id: {from: frontmatter_field, path: [id], required: true}
          title: {from: frontmatter_field, path: [title], required: true}
          type: {from: frontmatter_field, path: [type], required: true}
"
}

fn synthetic_types_module(parent: &Path) -> PathBuf {
    let module = parent.join("synthetic-security-types");
    std::fs::create_dir_all(&module).unwrap();
    std::fs::write(
        module.join("manifest.yaml"),
        format!(
            "manifest_version: 1.0.0\nname: synthetic-security-types\n\
             version: 0.1.0\n{}",
            synthetic_object_types()
        ),
    )
    .unwrap();
    module
}

fn out(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn err(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn parsed(output: &Output) -> Vec<Value> {
    assert!(output.status.success(), "{output:?}");
    serde_json::from_str(&out(output)).unwrap()
}

fn two_clauses() -> Value {
    binding(&[("C1", "binding"), ("C2", "binding")])
}

fn synthetic_pack(parent: &Path) -> PathBuf {
    let module = parent.join("synthetic-discharge-pack");
    std::fs::create_dir_all(&module).unwrap();
    std::fs::write(
        module.join("manifest.yaml"),
        format!(
            "manifest_version: 1.0.0\nname: synthetic-discharge-pack\n\
             version: 0.1.0\nclause_sets:\n  - std.yaml\n{}",
            synthetic_object_types()
        ),
    )
    .unwrap();
    std::fs::write(
        module.join("std.yaml"),
        format!(
            r#"schemaVersion: clause-set-v1
authority: example.invalid
id: std
title: Synthetic discharge set
version: "1"
rights:
  structure: original
  text: licensed
clauses:
  - id: C1
    force: mandatory
    title: C1
    text: "{CLAUSE_TEXT_SENTINEL}"
  - id: C2
    force: mandatory
    title: C2
    text: "{CLAUSE_TEXT_SENTINEL}"
  - id: C3
    force: mandatory
    title: C3
    text: "{CLAUSE_TEXT_SENTINEL}"
"#
        ),
    )
    .unwrap();
    module
}

const CLAUSE_TEXT_SENTINEL: &str = "SYNTHETIC-CLAUSE-TEXT-MUST-NOT-LEAK";

fn assert_no_compliant_count_or_clause_text(text: &str) {
    assert!(!text.contains(CLAUSE_TEXT_SENTINEL), "{text}");
    assert!(!text.contains('%'), "{text}");
    for line in text
        .lines()
        .filter(|line| line.to_ascii_lowercase().contains("compliant"))
    {
        assert!(
            line.trim_start().starts_with('|'),
            "unexpected compliance summary: {line}"
        );
    }
}

#[trace("FR-173-AC-1", "NFR-038-AC-1", "NFR-038-AC-3")]
#[test]
fn tc_173_an_effective_control_yields_a_direct_fact_with_the_supplied_attestation() {
    let world = World::new(
        &two_clauses(),
        &[(
            "CTRL-001.md",
            control("CTRL-001", Some("effective"), &[(SET, "C1")]),
        )],
    );
    let facts = parsed(&world.facts());
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0]["kind"], "direct");
    assert_eq!(facts[0]["clauseId"], "C1");
    assert_eq!(facts[0]["evidenceRefs"], json!(["ix://CTRL-001"]));
    assert_eq!(facts[0]["attestation"], attestation());
    assert!(!out(&world.facts()).contains("effectiveness"));
}

#[trace("FR-173-AC-2", "NFR-038-AC-2")]
#[test]
fn tc_173_a_clause_without_an_effective_control_yields_no_fact_and_an_open_line() {
    let files = [
        (
            "A.md",
            control("CTRL-A", Some("not_assessed"), &[(SET, "C1")]),
        ),
        (
            "B.md",
            control("CTRL-B", Some("ineffective"), &[(SET, "C1")]),
        ),
        (
            "C.md",
            control("CTRL-C", Some("partially_effective"), &[(SET, "C2")]),
        ),
        ("D.md", control("CTRL-D", None, &[(SET, "C2")])),
    ];
    let world = World::new(&two_clauses(), &files);
    let output = world.facts();
    assert!(parsed(&output).is_empty());
    let report = err(&output);
    assert!(report.contains("open C1 CTRL-A CTRL-B"), "{report}");
    assert!(report.contains("open C2 CTRL-C CTRL-D"), "{report}");
}

#[trace("FR-173-AC-3")]
#[test]
fn tc_173_mappings_to_non_binding_clauses_and_other_sets_are_unused() {
    let world = World::new(
        &binding(&[("C1", "binding"), ("C2", "not_binding")]),
        &[(
            "A.md",
            control(
                "CTRL-A",
                Some("effective"),
                &[(SET, "C2"), ("other.example/std@1", "X-1")],
            ),
        )],
    );
    let output = world.facts();
    assert!(parsed(&output).is_empty());
    let report = err(&output);
    assert!(
        report.contains("unused CTRL-A example.invalid/std@1 C2"),
        "{report}"
    );
    assert!(
        report.contains("unused CTRL-A other.example/std@1 X-1"),
        "{report}"
    );
}

#[trace("FR-173-AC-4")]
#[test]
fn tc_173_a_scope_exclusion_becomes_a_delegated_disposition_only_with_an_approval() {
    let world = World::new(
        &two_clauses(),
        &[("X.md", exclusion("SCOPEX-001", SET, "C2"))],
    );
    let facts = parsed(&world.facts_with(&["--approval-ref", "APPR-7"]));
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0]["kind"], "disposition");
    assert_eq!(facts[0]["decision"], "delegated");
    assert_eq!(facts[0]["approvalRef"], "APPR-7");
    assert_eq!(
        facts[0]["rationale"],
        "The platform owns this clause. (responsible: COMP-001)"
    );
    let without = world.facts();
    assert!(parsed(&without).is_empty());
    assert!(err(&without).contains("needs-approval C2 SCOPEX-001"));
}

#[trace("FR-173-AC-5", "NFR-038-AC-1")]
#[test]
fn tc_173_every_refusal_exits_non_zero_with_empty_stdout_and_names_its_subject() {
    let effective = |id: &str, clause: &str| control(id, Some("effective"), &[(SET, clause)]);
    let refusal = |world: &World, extra: &[&str], needles: &[&str]| {
        let output = world.facts_with(extra);
        assert!(!output.status.success(), "{needles:?} {output:?}");
        assert!(out(&output).is_empty());
        let message = err(&output);
        for needle in needles {
            assert!(message.contains(needle), "{needle} not in {message}");
        }
    };

    let world = World::new(&two_clauses(), &[("A.md", effective("CTRL-A", "C1"))]);
    std::fs::remove_file(world.path("attestation.json")).unwrap();
    refusal(&world, &[], &["attestation"]);
    let mut extra = attestation();
    extra["inventedScore"] = json!(1);
    std::fs::write(world.path("attestation.json"), extra.to_string()).unwrap();
    refusal(&world, &[], &["attestation", "inventedScore"]);

    let unclosed = "---\nid: CTRL-A\ntype: control\n# no closing fence\n".to_owned();
    let world = World::new(&two_clauses(), &[("A.md", unclosed)]);
    refusal(&world, &[], &["A.md"]);
    let headers = effective("CTRL-A", "C1").replace("| Set | Clause |", "| Standard | Id |");
    let world = World::new(&two_clauses(), &[("A.md", headers)]);
    refusal(&world, &[], &["CTRL-A"]);

    let world = World::new(&two_clauses(), &[("A.md", effective("CTRL-A", "NOPE"))]);
    refusal(&world, &[], &["CTRL-A", SET, "NOPE"]);

    let world = World::new(
        &two_clauses(),
        &[(
            "A.md",
            control("CTRL-A", Some("effective"), &[(SET, "C1"), (SET, "C1")]),
        )],
    );
    refusal(&world, &[], &["CTRL-A", SET, "C1"]);

    let world = World::new(
        &two_clauses(),
        &[
            ("X.md", exclusion("SCOPEX-001", SET, "C2")),
            ("Y.md", exclusion("SCOPEX-002", SET, "C2")),
        ],
    );
    refusal(&world, &[], &["SCOPEX-001", "SCOPEX-002", SET, "C2"]);

    let world = World::new(
        &two_clauses(),
        &[
            ("A.md", effective("CTRL-A", "C2")),
            ("X.md", exclusion("SCOPEX-001", SET, "C2")),
        ],
    );
    refusal(&world, &[], &["CTRL-A", "SCOPEX-001", SET, "C2"]);

    let folded = binding(&[("C1", "binding"), ("c1", "binding")]);
    let world = World::new(
        &folded,
        &[
            ("A.md", effective("CTRL-A", "C1")),
            ("B.md", effective("CTRL-B", "c1")),
        ],
    );
    refusal(&world, &[], &["case-collision", SET]);
}

#[trace("FR-173-AC-6")]
#[test]
fn tc_173_output_does_not_depend_on_the_order_of_the_corpus_files() {
    let files = [
        ("A.md", control("CTRL-A", Some("effective"), &[(SET, "C1")])),
        (
            "B.md",
            control("CTRL-B", Some("effective"), &[(SET, "C1"), (SET, "C2")]),
        ),
        (
            "C.md",
            control(
                "CTRL-C",
                Some("ineffective"),
                &[(SET, "C2"), ("o/s@1", "Z")],
            ),
        ),
    ];
    let forward = World::new(&two_clauses(), &files);
    let reversed: Vec<_> = files.iter().rev().cloned().collect();
    let backward = World::new(&two_clauses(), &reversed);
    let (first, second) = (forward.facts(), backward.facts());
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}

#[trace("FR-173-AC-7")]
#[test]
fn tc_173_the_output_is_accepted_by_record_discharge() {
    let world = World::new(
        &two_clauses(),
        &[
            ("A.md", control("CTRL-A", Some("effective"), &[(SET, "C1")])),
            ("X.md", exclusion("SCOPEX-001", SET, "C2")),
        ],
    );
    let facts = world.facts_with(&["--approval-ref", "APPR-7"]);
    let recorded = world.record(&out(&facts));
    assert!(recorded.status.success(), "{recorded:?}");
    assert_eq!(out(&recorded).lines().count(), 2);
}

fn listing(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(next).unwrap().flatten() {
            let path = entry.path();
            let meta = std::fs::metadata(&path).unwrap();
            found.push(format!("{} {}", path.display(), meta.len()));
            if path.is_dir() {
                pending.push(path);
            }
        }
    }
    found.sort();
    found
}

#[trace("FR-173-AC-8", "NFR-038-AC-4")]
#[test]
fn tc_173_a_run_writes_no_file() {
    let world = World::new(
        &two_clauses(),
        &[("A.md", control("CTRL-A", Some("effective"), &[(SET, "C1")]))],
    );
    let before = listing(world.dir.path());
    let output = world.facts_with(&["--repo", world.dir.path().to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(before, listing(world.dir.path()));
}

#[trace("FR-173-AC-5")]
#[test]
fn tc_173_a_control_missing_its_id_or_type_is_rejected_as_an_invalid_corpus() {
    for (field, missing) in [("id", "id: CTRL-A\n"), ("type", "type: control\n")] {
        let world = World::new(
            &two_clauses(),
            &[("A.md", control("CTRL-A", Some("effective"), &[(SET, "C1")]))],
        );
        let path = world.spec().join("A.md");
        let original = std::fs::read_to_string(&path).unwrap();
        let malformed = original.replace(missing, "");
        assert_ne!(malformed, original, "fixture includes {field}");
        std::fs::write(path, malformed).unwrap();

        let output = world.facts();
        assert!(!output.status.success(), "missing {field}: {output:?}");
        assert!(output.stdout.is_empty(), "missing {field}: {output:?}");
        assert!(err(&output).contains("fails validation"), "{output:?}");
    }
}

#[trace("FR-173-AC-9")]
#[test]
fn tc_173_a_stored_direct_fact_whose_control_is_no_longer_effective_is_named_stale() {
    let world = World::new(
        &two_clauses(),
        &[("A.md", control("CTRL-A", Some("effective"), &[(SET, "C1")]))],
    );
    let recorded = world.record(&out(&world.facts()));
    assert!(recorded.status.success(), "{recorded:?}");
    std::fs::write(
        world.spec().join("A.md"),
        control("CTRL-A", Some("ineffective"), &[(SET, "C1")]),
    )
    .unwrap();
    let output = world.facts_with(&["--repo", world.dir.path().to_str().unwrap()]);
    assert!(parsed(&output).is_empty());
    assert!(err(&output).contains("stale C1"), "{}", err(&output));
}

fn three_clauses() -> World {
    World::new(
        &binding(&[("C1", "binding"), ("C2", "binding"), ("C3", "binding")]),
        &[
            ("A.md", control("CTRL-A", Some("effective"), &[(SET, "C1")])),
            (
                "B.md",
                control("CTRL-B", Some("ineffective"), &[(SET, "C3")]),
            ),
            ("X.md", exclusion("SCOPEX-001", SET, "C2")),
        ],
    )
}

fn chain(world: &World) {
    let facts = world.facts_with(&["--approval-ref", "APPR-7"]);
    assert!(facts.status.success(), "{facts:?}");
    let recorded = world.record(&out(&facts));
    assert!(recorded.status.success(), "{recorded:?}");
}

#[trace("FR-174-AC-1", "FR-174-AC-3")]
#[test]
fn tc_174_the_chain_renders_compliant_tailored_and_undischarged_rows_without_a_score() {
    let world = three_clauses();
    world.install_synthetic_clause_set();
    let facts = world.facts_with_installed_modules(&["--approval-ref", "APPR-7"]);
    assert!(facts.status.success(), "{facts:?}");
    assert_no_compliant_count_or_clause_text(&out(&facts));
    assert_no_compliant_count_or_clause_text(
        &std::fs::read_to_string(world.path("binding.json")).unwrap(),
    );
    let recorded = world.record_installed(&out(&facts));
    assert!(recorded.status.success(), "{recorded:?}");
    assert_no_compliant_count_or_clause_text(&out(&recorded));
    let report = world.rmm_installed(AS_OF);
    assert!(
        report.contains("| C1 | Applicable | Compliant | ix://CTRL-A |"),
        "{report}"
    );
    assert!(report.contains("| C2 | Applicable | Tailored"), "{report}");
    assert!(
        report.contains("| C3 | Applicable | Not compliant - undischarged"),
        "{report}"
    );
    assert_no_compliant_count_or_clause_text(&report);
    for text in world.discharge_files() {
        assert_no_compliant_count_or_clause_text(&text);
    }
}

#[trace("FR-174-AC-2")]
#[test]
fn tc_174_a_clause_renders_not_current_after_its_attestation_expires() {
    let world = three_clauses();
    chain(&world);
    let report = world.rmm("2026-10-01T00:00:00Z");
    assert!(
        report.contains("| C1 | Applicable | Not compliant - attestation not current"),
        "{report}"
    );
}

#[trace("FR-174-AC-5")]
#[test]
fn tc_174_a_stored_fact_stays_compliant_until_its_file_is_removed() {
    let world = three_clauses();
    chain(&world);
    std::fs::write(
        world.spec().join("A.md"),
        control("CTRL-A", Some("ineffective"), &[(SET, "C1")]),
    )
    .unwrap();
    let rerun = world.facts_with(&[
        "--approval-ref",
        "APPR-7",
        "--repo",
        world.dir.path().to_str().unwrap(),
    ]);
    assert!(err(&rerun).contains("stale C1"), "{}", err(&rerun));
    assert!(world.rmm(AS_OF).contains("| C1 | Applicable | Compliant |"));
    let stored = listing(world.dir.path())
        .into_iter()
        .find_map(|entry| {
            let path = entry.rsplit_once(' ')?.0.to_owned();
            path.ends_with("/c1.json").then_some(path)
        })
        .expect("the stored fact of C1");
    std::fs::remove_file(stored).unwrap();
    assert!(
        world
            .rmm(AS_OF)
            .contains("| C1 | Applicable | Not compliant - undischarged")
    );
}

/// The typed-control fixture of the `spec-security-analysis` skill, end to end:
/// the skill's committed output, a person's `effective` verdict on one control,
/// `facts-from-mappings`, `record-discharge` and `discharge --rmm`.
#[trace("FR-174-AC-4")]
#[test]
fn tc_174_the_chain_uses_a_synthetic_pack_installed_in_the_temporary_ix_home() {
    let world = three_clauses();
    world.install_synthetic_clause_set();

    let facts = world.facts_with_installed_modules(&["--approval-ref", "APPR-7"]);
    assert!(facts.status.success(), "{facts:?}");
    let recorded = world.record_installed(&out(&facts));
    assert!(recorded.status.success(), "{recorded:?}");
    let report = world.rmm_installed(AS_OF);
    assert!(
        report.contains("| C1 | Applicable | Compliant | ix://CTRL-A |"),
        "{report}"
    );
    assert!(report.contains("| C2 | Applicable | Tailored"), "{report}");
    assert!(
        report.contains("| C3 | Applicable | Not compliant - undischarged"),
        "{report}"
    );
    assert_no_compliant_count_or_clause_text(&report);
}

/// The typed-control fixture of the `spec-security-analysis` skill, end to end:
/// the skill's committed output, a person's `effective` verdict on one control,
/// `facts-from-mappings`, `record-discharge` and `discharge --rmm`.
#[trace("FR-174-AC-1")]
#[test]
fn tc_1525_skill_output_on_the_fixture_repo_flows_to_the_matrix() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/security-analysis");
    let mut nist = binding(&[
        ("AC-3", "binding"),
        ("AC-6", "binding"),
        ("AC-12", "binding"),
        ("IA-5", "binding"),
        ("AU-2", "binding"),
        ("AU-12", "binding"),
        ("PE-3", "binding"),
    ]);
    nist["clauseSet"] = json!({"authority": "nist", "id": "sp800-53", "version": "5.2.0"});
    let world = World::new(&nist, &[]);
    let security = fixture.join("expected/spec/security");
    for entry in std::fs::read_dir(&security).unwrap().flatten() {
        std::fs::copy(entry.path(), world.spec().join(entry.file_name())).unwrap();
    }
    // A person assesses CTRL-002 as effective; the skill never writes that.
    let path = world.spec().join("CTRL-002-request-authorization.md");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("`not_assessed`", "`effective`")).unwrap();

    chain(&world);
    let report = world.rmm(AS_OF);
    for clause in ["AC-3", "AC-6"] {
        assert!(
            report.contains(&format!(
                "| {clause} | Applicable | Compliant | ix://CTRL-002 "
            )),
            "{report}"
        );
    }
    assert!(
        report.contains("| PE-3 | Applicable | Tailored"),
        "{report}"
    );
    assert!(
        report.contains("| IA-5 | Applicable | Not compliant - undischarged"),
        "{report}"
    );
}
