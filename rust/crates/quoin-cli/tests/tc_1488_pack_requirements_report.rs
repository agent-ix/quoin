// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin discharge --rmm --pack` through the shipped binary (FR-140).
//!
//! Every pack is a synthetic `example.invalid` module installed into a temp
//! `IX_HOME`. Each clause text, clause title and set title carries its own
//! sentinel, so a leak of licensed content is a string search that can fail.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod pack_support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Output;

use ix_trace_rs::trace;
use pack_support::{git_init, git_porcelain, quoin, scratch, snapshot, text};

const PACK: &str = "gadget-pack";
const AS_OF: &str = "2026-08-15T00:00:00Z";
/// A clause id carrying every markup character the escape must neutralise.
const NASTY_ID: &str = "M|2`x`<b>[l](http://e)";

/// `(set id, version, [(clause id, force, applicability yaml)])`, in manifest
/// order, which is deliberately not key order.
type SetSpec = (
    &'static str,
    &'static str,
    Vec<(String, &'static str, &'static str)>,
);

fn pack_sets() -> Vec<SetSpec> {
    vec![
        (
            "zeta",
            "1",
            vec![
                ("Z-1".to_owned(), "mandatory", ""),
                ("Z-2".to_owned(), "recommended", ""),
            ],
        ),
        (
            "alpha",
            "2",
            vec![
                ("A-1".to_owned(), "mandatory", ""),
                (
                    "A-2".to_owned(),
                    "mandatory",
                    "    applicability: {op: eq, dimension: region, value: eu}\n",
                ),
            ],
        ),
        (
            "mid_set",
            "1",
            vec![
                ("M-1".to_owned(), "mandatory", ""),
                (NASTY_ID.to_owned(), "permitted", ""),
            ],
        ),
    ]
}

/// The sentinel of the clause at `index` of `set`; one per clause.
fn text_sentinel(set: &str, index: usize) -> String {
    format!("SENTINEL-text-{set}-{index}")
}

fn title_sentinel(set: &str, index: usize) -> String {
    format!("SENTINEL-title-{set}-{index}")
}

fn set_title_sentinel(set: &str) -> String {
    format!("SENTINEL-settitle-{set}")
}

/// Every sentinel the fixture pack carries.
fn sentinels() -> Vec<String> {
    pack_sets().iter().flat_map(set_sentinels).collect()
}

/// Every sentinel carried by one clause set.
fn set_sentinels(set: &SetSpec) -> Vec<String> {
    let (id, _, clauses) = set;
    let mut all = vec![set_title_sentinel(id)];
    for index in 0..clauses.len() {
        all.push(text_sentinel(id, index));
        all.push(title_sentinel(id, index));
    }
    all
}

/// The sentinels found in `output`; empty means none leaked.
fn leaked(output: &str) -> Vec<String> {
    let expected = sentinels();
    leaked_from(output, &expected)
}

/// The supplied sentinels found in `output`; empty means none leaked.
fn leaked_from(output: &str, expected: &[String]) -> Vec<String> {
    expected
        .iter()
        .filter(|sentinel| output.contains(sentinel.as_str()))
        .cloned()
        .collect()
}

fn set_yaml(set: &SetSpec) -> String {
    let (id, version, clauses) = set;
    let mut yaml = format!(
        "schemaVersion: clause-set-v1\nauthority: example.invalid\nid: {id}\n\
         title: \"{}\"\nversion: \"{version}\"\nrights:\n  structure: original\n  \
         text: licensed\nclassificationDimensions:\n  region:\n    values: [eu, us]\n\
         clauses:\n",
        set_title_sentinel(id)
    );
    for (index, (clause, force, applicability)) in clauses.iter().enumerate() {
        write!(
            yaml,
            "  - id: \"{clause}\"\n    force: {force}\n    title: \"{}\"\n    text: \"{}\"\n{applicability}",
            title_sentinel(id, index),
            text_sentinel(id, index)
        )
        .unwrap();
    }
    yaml
}

fn write_pack(parent: &Path, name: &str, sets: &[SetSpec]) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let mut manifest = format!("name: {name}\nversion: 0.1.0\nclause_sets:\n");
    for set in sets {
        writeln!(manifest, "  - {}.yaml", set.0).unwrap();
        std::fs::write(dir.join(format!("{}.yaml", set.0)), set_yaml(set)).unwrap();
    }
    std::fs::write(dir.join("manifest.yaml"), manifest).unwrap();
    dir
}

struct Env {
    _root: tempfile::TempDir,
    base: PathBuf,
    home: PathBuf,
    /// The working directory of every run, a git repository.
    cwd: PathBuf,
    /// `--repo`, a second git repository.
    repo: PathBuf,
}

fn env() -> Env {
    let root = scratch();
    let base = root.path().canonicalize().unwrap();
    let (home, cwd, repo) = (base.join("ix-home"), base.join("cwd"), base.join("repo"));
    git_init(&cwd);
    git_init(&repo);
    pack_support::semantic_root(&home);
    let env = Env {
        _root: root,
        base,
        home,
        cwd,
        repo,
    };
    let source = write_pack(&env.base.join("src"), PACK, &pack_sets());
    let installed = env.run(&["module", "install", &format!("path:{}", source.display())]);
    assert!(installed.status.success(), "{}", text(&installed));
    env
}

impl Env {
    fn run(&self, arguments: &[&str]) -> Output {
        quoin(&self.home, &self.cwd, arguments, &[])
    }

    fn report_in(&self, repo: &Path, pack: &str, extra: &[&str]) -> Output {
        let repo = repo.to_str().unwrap();
        let mut arguments = vec![
            "discharge",
            "--rmm",
            "--pack",
            pack,
            "--repo",
            repo,
            "--as-of",
            AS_OF,
        ];
        arguments.extend_from_slice(extra);
        self.run(&arguments)
    }

    fn report(&self, extra: &[&str]) -> Output {
        self.report_in(&self.repo, PACK, extra)
    }

    fn installed_file(&self, name: &str) -> PathBuf {
        self.home.join("filament/modules").join(PACK).join(name)
    }

    /// The binding the engine derives for one set of the installed pack.
    fn engine_binding(&self, set: &str, version: &str, context: &[&str]) -> String {
        let root =
            quoin_quire::ModuleRoot::open(self.home.join("filament/modules").join(PACK)).unwrap();
        let reference = quoin_quire::ClauseSetRef {
            authority: quoin_quire::ClauseSetAuthority::new("example.invalid").unwrap(),
            id: quoin_quire::ClauseSetId::new(set).unwrap(),
            version: quoin_quire::ClauseSetVersion::new(version).unwrap(),
        };
        let context = quoin_quire::clauses::Context::from_entries(context).unwrap();
        let report =
            quoin_quire::clauses::evaluate(&root, &reference, &context, &mut Vec::new()).unwrap();
        serde_json::to_string(&report).unwrap()
    }

    /// The FR-135 single-set stdout for a set of the pack.
    fn single_set(
        &self,
        repo: &Path,
        set: &str,
        version: &str,
        context: &[&str],
        json: bool,
    ) -> String {
        let binding = self.base.join(format!("single-{set}.json"));
        std::fs::write(&binding, self.engine_binding(set, version, context)).unwrap();
        let mut arguments = vec![
            "discharge",
            "--rmm",
            "--binding",
            binding.to_str().unwrap(),
            "--repo",
            repo.to_str().unwrap(),
            "--as-of",
            AS_OF,
        ];
        if json {
            arguments.push("--json");
        }
        let output = self.run(&arguments);
        assert!(output.status.success(), "{}", text(&output));
        String::from_utf8(output.stdout).unwrap()
    }
}

fn direct(clause: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "direct", "clauseId": clause, "evidenceRefs": ["evidence://runs/SUITE-1/one"],
        "attestation": {"attestedBy": "r", "authority": "lead",
            "attestedAt": "2026-08-01T00:00:00Z", "expiresAt": "2026-09-01T00:00:00Z"}
    })
}

/// Store `facts` for the set `(set, version)` of the fixture authority.
/// Returns the stored file paths printed by the command.
fn record(env: &Env, repo: &Path, set: &str, version: &str, facts: &[serde_json::Value]) -> String {
    let output = record_output(env, repo, set, version, facts);
    assert!(output.status.success(), "{}", text(&output));
    String::from_utf8(output.stdout).unwrap()
}

fn record_output(
    env: &Env,
    repo: &Path,
    set: &str,
    version: &str,
    facts: &[serde_json::Value],
) -> Output {
    record_output_with(env, repo, set, version, facts, &[])
}

/// A verifier that approves every request as `r` with the given meaning,
/// standing in for engineering-assurance.
fn stub_verifier(env: &Env, meaning: &str) -> (String, String) {
    use std::os::unix::fs::PermissionsExt as _;
    let script = env.base.join("stub-ea");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ncat >/dev/null\necho '{{\"protocol\":\"engineering-assurance.approval-verify-result/v1\",\"status\":\"approved\",\"meaning\":[\"{meaning}\"],\"approver\":\"r\"}}'\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let signers = env.base.join("allowed-signers");
    std::fs::write(&signers, "").unwrap();
    (
        script.to_str().unwrap().to_owned(),
        signers.to_str().unwrap().to_owned(),
    )
}

fn record_output_with(
    env: &Env,
    repo: &Path,
    set: &str,
    version: &str,
    facts: &[serde_json::Value],
    extra: &[&str],
) -> Output {
    let run = repo.join("spec/evidence/runs/SUITE-1");
    std::fs::create_dir_all(&run).unwrap();
    std::fs::write(run.join("one.json"), "{}").unwrap();
    let binding = serde_json::json!({
        "schemaVersion": "clause-binding-v1",
        "clauseSet": {"authority": "example.invalid", "id": set, "version": version},
        "context": {},
        "clauses": []
    });
    let (binding_path, facts_path) = (
        env.base.join(format!("record-binding-{set}.json")),
        env.base.join(format!("record-facts-{set}.json")),
    );
    std::fs::write(&binding_path, binding.to_string()).unwrap();
    std::fs::write(
        &facts_path,
        serde_json::Value::from(facts.to_vec()).to_string(),
    )
    .unwrap();
    let mut arguments = vec![
        "evidence",
        "record-discharge",
        "--repo",
        repo.to_str().unwrap(),
        "--binding",
        binding_path.to_str().unwrap(),
        "--facts",
        facts_path.to_str().unwrap(),
    ];
    arguments.extend_from_slice(extra);
    env.run(&arguments)
}

fn approval_clause_set(id: &str, approval_roles: &str) -> String {
    format!(
        "schemaVersion: clause-set-v1\nauthority: example.invalid\nid: {id}\ntitle: Synthetic rules\nversion: \"1\"\nrights:\n  structure: original\n  text: none\nclauses:\n  - id: C-1\n    force: mandatory\n    approvalRoles: {approval_roles}\n"
    )
}

fn one_clause_binding(set: &str) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": "clause-binding-v1",
        "clauseSet": {"authority": "example.invalid", "id": set, "version": "1"},
        "context": {},
        "clauses": [{
            "clauseId": "C-1", "force": "mandatory", "outcome": "binding",
            "reasons": [], "expectedOutputs": []
        }]
    })
}

fn discharge_output(env: &Env, repo: &Path, set: &str) -> Output {
    discharge_output_with(env, repo, set, &[])
}

fn discharge_output_with(env: &Env, repo: &Path, set: &str, extra: &[&str]) -> Output {
    let binding_path = env.base.join(format!("discharge-binding-{set}.json"));
    std::fs::write(&binding_path, one_clause_binding(set).to_string()).unwrap();
    let mut arguments = vec![
        "discharge",
        "--binding",
        binding_path.to_str().unwrap(),
        "--repo",
        repo.to_str().unwrap(),
        "--as-of",
        AS_OF,
        "--json",
    ];
    arguments.extend_from_slice(extra);
    env.run(&arguments)
}

fn discharge_report_with_binding(env: &Env, repo: &Path, binding: &Path, extra: &[&str]) -> Output {
    let mut arguments = vec![
        "discharge",
        "--binding",
        binding.to_str().unwrap(),
        "--repo",
        repo.to_str().unwrap(),
        "--as-of",
        AS_OF,
        "--json",
    ];
    arguments.extend_from_slice(extra);
    env.run(&arguments)
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json(output: &Output) -> serde_json::Value {
    assert!(output.status.success(), "{}", text(output));
    serde_json::from_slice(&output.stdout).unwrap()
}

fn install_approval_fixture(env: &Env) {
    std::fs::write(
        env.installed_file("manifest.yaml"),
        format!("name: {PACK}\nversion: 0.1.0\nclause_sets:\n  - gdpr.yaml\n"),
    )
    .unwrap();
    std::fs::write(
        env.installed_file("gdpr.yaml"),
        r"schemaVersion: clause-set-v1
authority: example.invalid
id: gdpr
title: Synthetic GDPR clauses
version: 1.0.0
rights:
  structure: original
  text: none
clauses:
  - id: art-35(1)
    force: mandatory
    approvalRoles: [risk-approver]
  - id: no-role
    force: mandatory
    approvalRoles: []
",
    )
    .unwrap();
}

/// The per-set matrices of a pack report, each from its heading to the next.
fn matrices(markdown: &str) -> Vec<&str> {
    const HEADING: &str = "# Requirements mapping matrix: ";
    let mut starts: Vec<usize> = markdown.match_indices(HEADING).map(|(at, _)| at).collect();
    starts.push(markdown.len());
    starts.windows(2).map(|w| &markdown[w[0]..w[1]]).collect()
}

/// The part of the Markdown the pack report writes itself.
fn header(markdown: &str) -> &str {
    &markdown[..markdown.find("# Requirements mapping matrix: ").unwrap()]
}

const KEYS: [&str; 3] = [
    "example.invalid/alpha@2",
    "example.invalid/mid_set@1",
    "example.invalid/zeta@1",
];

#[trace("FR-140-AC-1")]
#[test]
fn tc_1488_001_one_matrix_per_set_in_key_order_each_equal_to_the_single_set_render() {
    let env = env();
    record(&env, &env.repo, "alpha", "2", &[direct("A-1")]);
    let context = ["region=eu"];

    let markdown = stdout(&env.report(&["--context", "region=eu"]));
    let each = matrices(&markdown);
    assert_eq!(each.len(), 3, "{markdown}");
    let index: Vec<usize> = KEYS
        .iter()
        .map(|key| {
            markdown
                .find(&format!("| {} |", key.replace('_', "\\_")))
                .unwrap()
        })
        .collect();
    assert!(index.windows(2).all(|w| w[0] < w[1]), "{markdown}");
    for (matrix, (set, version)) in
        each.iter()
            .zip([("alpha", "2"), ("mid_set", "1"), ("zeta", "1")])
    {
        let single = env.single_set(&env.repo, set, version, &context, false);
        assert_eq!(*matrix, single, "{set}");
    }

    let value = json(&env.report(&["--context", "region=eu", "--json"]));
    assert_eq!(value["index"].as_array().unwrap().len(), 3);
    let matrices = value["matrices"].as_array().unwrap();
    assert_eq!(matrices.len(), 3);
    for (matrix, (set, version)) in
        matrices
            .iter()
            .zip([("alpha", "2"), ("mid_set", "1"), ("zeta", "1")])
    {
        let single: serde_json::Value =
            serde_json::from_str(&env.single_set(&env.repo, set, version, &context, true)).unwrap();
        assert_eq!(*matrix, single, "{set}");
    }
}

#[trace("FR-140-AC-2")]
#[test]
fn tc_1488_002_changing_the_context_changes_only_the_set_that_uses_the_dimension() {
    let env = env();
    let eu = json(&env.report(&["--context", "region=eu", "--json"]));
    let us = json(&env.report(&["--context", "region=us", "--json"]));
    let row = |report: &serde_json::Value, set: usize, clause: &str| {
        report["matrices"][set]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["clauseId"] == clause)
            .unwrap()
            .clone()
    };
    // Sets are alpha, mid_set, zeta; only alpha reads `region`.
    assert_eq!(row(&eu, 0, "A-2")["applicability"], "applicable");
    assert_eq!(row(&us, 0, "A-2")["applicability"], "not_applicable");
    assert_eq!(row(&us, 0, "A-2")["compliance"], "not_applicable");
    assert_eq!(row(&eu, 0, "A-1"), row(&us, 0, "A-1"));
    assert_eq!(eu["matrices"][1], us["matrices"][1]);
    assert_eq!(eu["matrices"][2], us["matrices"][2]);

    let md_us = stdout(&env.report(&["--context", "region=us"]));
    assert!(
        matrices(&md_us)[0].contains("| A-2 | Not applicable | Not applicable |"),
        "{md_us}"
    );
}

#[trace("FR-140-AC-3")]
#[test]
fn tc_1488_003_facts_for_one_set_leave_the_others_undischarged_and_counts_match_rows() {
    let env = env();
    record(&env, &env.repo, "alpha", "2", &[direct("A-1")]);
    let value = json(&env.report(&["--context", "region=eu", "--json"]));
    let rows = |set: usize| value["matrices"][set]["rows"].as_array().unwrap().clone();
    assert_eq!(rows(0)[0]["compliance"], "compliant");
    assert_eq!(rows(0)[1]["compliance"], "undischarged");
    for set in [1, 2] {
        for row in rows(set) {
            if row["applicability"] == "applicable" {
                assert_eq!(row["compliance"], "undischarged", "{row}");
            }
        }
    }
    for (position, entry) in value["index"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            usize::try_from(entry["rowCount"].as_u64().unwrap()).unwrap(),
            rows(position).len()
        );
    }
    let markdown = stdout(&env.report(&["--context", "region=eu"]));
    assert!(
        markdown.contains("| A-1 | Applicable | Compliant | evidence://runs/SUITE-1/one | - | - |"),
        "{markdown}"
    );
    assert!(
        markdown.contains("| Z-1 | Applicable | Not compliant - undischarged"),
        "{markdown}"
    );
}

#[trace("FR-208-AC-1", "FR-208-AC-2", "FR-208-AC-3", "FR-208-AC-5")]
#[test]
fn tc_208_003_clause_approval_roles_reject_missing_reference_and_keep_generic_clause() {
    let env = env();
    install_approval_fixture(&env);

    for blank in [None, Some(" \t\n")] {
        let mut fact = direct("art-35(1)");
        if let Some(blank) = blank {
            fact["approvalRef"] = serde_json::json!(blank);
        }
        let refused = record_output(&env, &env.repo, "gdpr", "1.0.0", &[fact, direct("no-role")]);
        assert_eq!(refused.status.code(), Some(3), "{}", text(&refused));
        assert!(
            stderr(&refused).contains("approval-missing"),
            "{}",
            text(&refused)
        );
    }

    let no_approval_ref = record_output(&env, &env.repo, "gdpr", "1.0.0", &[direct("no-role")]);
    assert!(
        no_approval_ref.status.success(),
        "a generic clause must accept a direct fact without approvalRef: {}",
        text(&no_approval_ref)
    );
    let report = json(&env.report(&["--json"]));
    let row = report["matrices"][0]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["clauseId"] == "no-role")
        .unwrap();
    assert_eq!(row["compliance"], "compliant");

    let binding_path = env.base.join("no-approval-binding.json");
    std::fs::write(&binding_path, env.engine_binding("gdpr", "1.0.0", &[])).unwrap();
    let binding = json(&discharge_report_with_binding(
        &env,
        &env.repo,
        &binding_path,
        &[],
    ));
    let fact = binding["binding"]["direct"]
        .as_array()
        .unwrap()
        .iter()
        .find(|fact| fact["clauseId"] == "no-role")
        .unwrap();
    assert!(
        !fact["fact"]
            .as_object()
            .unwrap()
            .contains_key("approvalRef"),
        "a generic direct fact without an approval reference acquired one: {fact}"
    );
}

#[trace("FR-208-AC-1", "FR-208-AC-2", "FR-208-AC-3", "FR-208-AC-5")]
#[test]
fn tc_208_003_generic_approval_reference_is_not_verified() {
    let env = env();
    install_approval_fixture(&env);
    let approval_ref = "person://example.invalid/people/generic-reviewer";
    let mut generic = direct("no-role");
    generic["approvalRef"] = serde_json::json!(approval_ref);
    record(&env, &env.repo, "gdpr", "1.0.0", &[generic]);

    let report = json(&env.report(&["--json"]));
    let rows = report["matrices"][0]["rows"].as_array().unwrap();
    let role_row = rows
        .iter()
        .find(|row| row["clauseId"] == "art-35(1)")
        .unwrap();
    let generic_row = rows
        .iter()
        .find(|row| row["clauseId"] == "no-role")
        .unwrap();
    assert_eq!(role_row["compliance"], "undischarged");
    assert_eq!(role_row["reason"], "no discharge fact");
    assert_eq!(generic_row["compliance"], "compliant");

    let binding_path = env.base.join("approval-binding.json");
    std::fs::write(&binding_path, env.engine_binding("gdpr", "1.0.0", &[])).unwrap();
    let binding = json(&discharge_report_with_binding(
        &env,
        &env.repo,
        &binding_path,
        &[],
    ));
    let fact = binding["binding"]["direct"]
        .as_array()
        .unwrap()
        .iter()
        .find(|fact| fact["clauseId"] == "no-role")
        .unwrap();
    assert_eq!(fact["fact"]["approvalRef"], approval_ref);
    assert!(
        !fact.as_object().unwrap().contains_key("verifiedApprover"),
        "a caller-supplied approvalRef for a generic clause was presented as verified: {fact}"
    );

    let mut blank = direct("no-role");
    blank["approvalRef"] = serde_json::json!("");
    let rejected = record_output(&env, &env.repo, "gdpr", "1.0.0", &[blank]);
    assert!(!rejected.status.success(), "{}", text(&rejected));
    assert!(
        stderr(&rejected).contains("approvalRef must not be empty"),
        "{}",
        text(&rejected)
    );
}

#[trace("FR-208-AC-1", "FR-208-AC-2", "FR-208-AC-3", "FR-208-AC-5")]
#[test]
fn tc_208_003_approval_role_is_verified_only_with_verifier() {
    let env = env();
    install_approval_fixture(&env);
    let generic_ref = "person://example.invalid/people/generic-reviewer";
    let mut generic = direct("no-role");
    generic["approvalRef"] = serde_json::json!(generic_ref);
    let (verifier, signers) = stub_verifier(&env, "risk-approver");
    let flags = [
        "--engineering-assurance",
        &verifier,
        "--allowed-signers",
        &signers,
    ];
    let mut approved = direct("art-35(1)");
    approved["approvalRef"] = serde_json::json!("person://example.invalid/people/privacy-officer");
    let recorded = record_output_with(
        &env,
        &env.repo,
        "gdpr",
        "1.0.0",
        &[approved, generic],
        &flags,
    );
    assert!(recorded.status.success(), "{}", text(&recorded));

    let mut arguments = vec!["--json"];
    arguments.extend_from_slice(&flags);
    let report = json(&env.report(&arguments));
    let rows = report["matrices"][0]["rows"].as_array().unwrap();
    let approved_row = rows
        .iter()
        .find(|row| row["clauseId"] == "art-35(1)")
        .unwrap();
    assert_eq!(approved_row["compliance"], "compliant");
    assert_eq!(
        approved_row["evidenceRefs"][0],
        "evidence://runs/SUITE-1/one"
    );

    let binding_path = env.base.join("approval-binding.json");
    std::fs::write(&binding_path, env.engine_binding("gdpr", "1.0.0", &[])).unwrap();
    let binding = json(&discharge_report_with_binding(
        &env,
        &env.repo,
        &binding_path,
        &flags,
    ));
    let direct = binding["binding"]["direct"].as_array().unwrap();
    let approved_fact = direct
        .iter()
        .find(|fact| fact["clauseId"] == "art-35(1)")
        .unwrap();
    assert_eq!(approved_fact["verifiedApprover"], "r");
    assert_eq!(
        approved_fact["fact"]["approvalRef"],
        "person://example.invalid/people/privacy-officer"
    );
    let generic_fact = direct
        .iter()
        .find(|fact| fact["clauseId"] == "no-role")
        .unwrap();
    assert_eq!(generic_fact["fact"]["approvalRef"], generic_ref);
    assert!(
        !generic_fact
            .as_object()
            .unwrap()
            .contains_key("verifiedApprover"),
        "the verifier was applied to a clause with no approval roles: {generic_fact}"
    );

    let unverified = json(&env.report(&["--json"]));
    let rows = unverified["matrices"][0]["rows"].as_array().unwrap();
    let approved_row = rows
        .iter()
        .find(|row| row["clauseId"] == "art-35(1)")
        .unwrap();
    assert_eq!(approved_row["compliance"], "undischarged");
    assert!(
        approved_row["reason"]
            .as_str()
            .unwrap()
            .starts_with("approval-verifier-unavailable"),
        "{approved_row}"
    );
}

/// An engineering-assurance stand-in that writes the request it reads to
/// `request.json` and then runs `body`. Returns the script and an
/// allowed-signers file.
fn scripted_verifier(env: &Env, body: &str) -> (String, String) {
    use std::os::unix::fs::PermissionsExt as _;
    let script = env.base.join("scripted-ea");
    let log = env.base.join("request.json");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ncat > '{}'\n{body}\n", log.display()),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let signers = env.base.join("allowed-signers");
    std::fs::write(&signers, "").unwrap();
    (
        script.to_str().unwrap().to_owned(),
        signers.to_str().unwrap().to_owned(),
    )
}

fn approved_art_35() -> serde_json::Value {
    let mut fact = direct("art-35(1)");
    fact["approvalRef"] = serde_json::json!("approval-1");
    fact
}

/// The request the production verifier writes carries exactly the members
/// engineering-assurance reads, and its `record_digest` is the one
/// `approval-subject` prints (FR-211-AC-6, AC-17).
#[trace("FR-211-AC-6", "FR-211-AC-17")]
#[test]
fn tc_211_the_verifier_is_sent_the_request_engineering_assurance_reads() {
    let env = env();
    install_approval_fixture(&env);
    let (verifier, signers) = scripted_verifier(
        &env,
        r#"echo '{"protocol":"engineering-assurance.approval-verify-result/v1","status":"approved","meaning":["risk-approver"],"approver":"r"}'"#,
    );
    let flags = [
        "--engineering-assurance",
        &verifier,
        "--allowed-signers",
        &signers,
        "--gitsign-certificate-identity",
        "id@example.invalid",
        "--gitsign-certificate-oidc-issuer",
        "https://issuer.example.invalid",
        "--gitsign-principal",
        "principal@example.invalid",
    ];
    let recorded = record_output_with(
        &env,
        &env.repo,
        "gdpr",
        "1.0.0",
        &[approved_art_35()],
        &flags,
    );
    assert!(recorded.status.success(), "{}", text(&recorded));
    let request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(env.base.join("request.json")).unwrap()).unwrap();

    // The identity and digest a person signs, from the dedicated command.
    let binding = env.base.join("subject-binding.json");
    let fact = env.base.join("subject-fact.json");
    let mut report = one_clause_binding("gdpr");
    report["clauseSet"]["version"] = serde_json::json!("1.0.0");
    report["clauses"][0]["clauseId"] = serde_json::json!("art-35(1)");
    std::fs::write(&binding, report.to_string()).unwrap();
    std::fs::write(&fact, approved_art_35().to_string()).unwrap();
    let subject = json(&env.run(&[
        "evidence",
        "approval-subject",
        "--binding",
        binding.to_str().unwrap(),
        "--fact",
        fact.to_str().unwrap(),
    ]));

    assert_eq!(
        request,
        serde_json::json!({
            "protocol": "engineering-assurance.approval-verify-request/v1",
            "repository": env.repo.to_str().unwrap(),
            "tag": "approval-1",
            "record_id": subject["record_id"],
            "record_digest": subject["record_digest"],
            "allowed_signers": signers,
            "gitsign": {
                "certificate_identity": "id@example.invalid",
                "certificate_oidc_issuer": "https://issuer.example.invalid",
                "principal": "principal@example.invalid",
            },
        })
    );
    assert_eq!(subject["record_id"], "example.invalid/gdpr/1.0.0/art-35(1)");
    assert!(
        subject["record_digest"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
}

/// A result whose `meaning` holds a non-string value is not read as a single
/// role: the verifier is unavailable, exit 3 (FR-211-AC-7).
#[trace("FR-211-AC-7", "FR-211-AC-11")]
#[test]
fn tc_211_a_non_string_meaning_is_not_a_role() {
    let env = env();
    install_approval_fixture(&env);
    let (verifier, signers) = scripted_verifier(
        &env,
        r#"echo '{"protocol":"engineering-assurance.approval-verify-result/v1","status":"approved","meaning":["risk-approver",1],"approver":"r"}'"#,
    );
    let flags = [
        "--engineering-assurance",
        &verifier,
        "--allowed-signers",
        &signers,
    ];
    let refused = record_output_with(
        &env,
        &env.repo,
        "gdpr",
        "1.0.0",
        &[approved_art_35()],
        &flags,
    );
    assert_eq!(refused.status.code(), Some(3), "{}", text(&refused));
    assert!(
        stderr(&refused).contains("non-string"),
        "{}",
        text(&refused)
    );
}

/// An engineering-assurance refusal reaches the user with its own message,
/// which it writes on stdout (FR-211-AC-11).
#[trace("FR-211-AC-11")]
#[test]
fn tc_211_a_refusal_names_the_message_engineering_assurance_gave() {
    let env = env();
    install_approval_fixture(&env);
    let (verifier, signers) = scripted_verifier(
        &env,
        r#"echo '{"protocol":"engineering-assurance.error/v1","code":"approval_verify_request_invalid","message":"the tag is not a plain revision"}'; exit 1"#,
    );
    let flags = [
        "--engineering-assurance",
        &verifier,
        "--allowed-signers",
        &signers,
    ];
    let refused = record_output_with(
        &env,
        &env.repo,
        "gdpr",
        "1.0.0",
        &[approved_art_35()],
        &flags,
    );
    assert_eq!(refused.status.code(), Some(3), "{}", text(&refused));
    let message = stderr(&refused);
    assert!(
        message.contains("approval_verify_request_invalid"),
        "{message}"
    );
    assert!(
        message.contains("the tag is not a plain revision"),
        "{message}"
    );
}

/// The dossier RMM never reaches engineering-assurance: an approval-class
/// clause reads `not-verified-offline`, not an unavailable verifier.
#[trace("FR-195-AC-1", "FR-195-AC-5", "FR-211-AC-13")]
#[test]
fn tc_211_the_dossier_rmm_leaves_an_approval_class_clause_not_verified_offline() {
    let env = env();
    install_approval_fixture(&env);
    let (verifier, signers) = stub_verifier(&env, "risk-approver");
    let flags = [
        "--engineering-assurance",
        &verifier,
        "--allowed-signers",
        &signers,
    ];
    let mut approved = direct("art-35(1)");
    approved["approvalRef"] = serde_json::json!("person://example.invalid/people/privacy-officer");
    let recorded = record_output_with(&env, &env.repo, "gdpr", "1.0.0", &[approved], &flags);
    assert!(recorded.status.success(), "{}", text(&recorded));

    let out = env.base.join("dossier");
    let generated = env.run(&[
        "dossier",
        "generate",
        "--doc",
        "rmm",
        "--version",
        "1",
        "--pack",
        PACK,
        "--scope",
        env.repo.to_str().unwrap(),
        "--as-of",
        AS_OF,
        "--out",
        out.to_str().unwrap(),
        "--json",
    ]);
    let rmm = std::fs::read_to_string(out.join("DOS-RMM.md"));
    assert!(rmm.is_ok(), "{}", text(&generated));
    let rmm = rmm.unwrap();
    let row = rmm
        .lines()
        .find(|line| line.starts_with("| art-35(1)"))
        .expect("the RMM has an art-35(1) row");
    assert!(row.contains("not-verified-offline"), "{row}");
    assert!(!row.contains("approval-verifier-unavailable"), "{row}");
}

/// A trust file engineering-assurance refuses exits 2, an absent verifier
/// exits 3, and neither writes a fact.
#[trace("FR-211-AC-11")]
#[test]
fn tc_211_a_refused_trust_file_exits_two_and_an_absent_verifier_exits_three() {
    use std::os::unix::fs::PermissionsExt as _;
    let env = env();
    install_approval_fixture(&env);
    let signers = env.base.join("allowed-signers");
    std::fs::write(&signers, "").unwrap();
    let refusing = env.base.join("refusing-ea");
    std::fs::write(
        &refusing,
        "#!/bin/sh\ncat >/dev/null\necho '{\"protocol\":\"engineering-assurance.error/v1\",\"code\":\"approval_verify_allowed_signers_invalid\"}'\nexit 2\n",
    )
    .unwrap();
    std::fs::set_permissions(&refusing, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut fact = direct("art-35(1)");
    fact["approvalRef"] = serde_json::json!("tag-1");
    let signers = signers.to_str().unwrap();
    for (verifier, status) in [
        (refusing.to_str().unwrap().to_owned(), 2),
        ("/nonexistent/engineering-assurance".to_owned(), 3),
    ] {
        let output = record_output_with(
            &env,
            &env.repo,
            "gdpr",
            "1.0.0",
            &[fact.clone()],
            &[
                "--engineering-assurance",
                &verifier,
                "--allowed-signers",
                signers,
            ],
        );
        assert_eq!(output.status.code(), Some(status), "{}", text(&output));
        assert!(output.stdout.is_empty(), "{}", text(&output));
    }
    assert!(!env.repo.join("spec/evidence/discharge").exists());
}

#[trace("FR-208-AC-6")]
#[test]
fn tc_208_005_record_discharge_reads_approval_metadata_with_the_full_module_set() {
    let env = env();
    let owner = env.base.join("src/cross-vocabulary-pack");
    std::fs::create_dir_all(&owner).unwrap();
    std::fs::write(
        owner.join("manifest.yaml"),
        r"name: cross-vocabulary-pack
version: 0.1.0
clause_sets:
  - rules.yaml
object_types:
  - name: cross_object
    data_schema: {type: object}
    allowed_links: [references]
",
    )
    .unwrap();
    std::fs::write(
        owner.join("rules.yaml"),
        r#"schemaVersion: clause-set-v1
authority: example.invalid
id: cross-vocabulary
title: Cross-vocabulary rules
version: "1"
rights:
  structure: original
  text: none
clauses:
  - id: C-1
    force: mandatory
    approvalRoles: [privacy-officer]
"#,
    )
    .unwrap();
    let owner_install = env.run(&["module", "install", &format!("path:{}", owner.display())]);
    assert!(owner_install.status.success(), "{}", text(&owner_install));

    let provider = env.base.join("src/edge-vocabulary");
    std::fs::create_dir_all(&provider).unwrap();
    std::fs::write(
        provider.join("manifest.yaml"),
        r"name: edge-vocabulary
version: 0.1.0
edge_types:
  references: {description: Relationship reference, category: dependency}
",
    )
    .unwrap();
    let provider_install = env.run(&["module", "install", &format!("path:{}", provider.display())]);
    assert!(
        provider_install.status.success(),
        "{}",
        text(&provider_install)
    );

    // The roles come from the owner set, found through the full module set:
    // a fact without an approval reference is refused by name.
    let missing_approval =
        record_output(&env, &env.repo, "cross-vocabulary", "1", &[direct("C-1")]);
    assert_eq!(
        missing_approval.status.code(),
        Some(3),
        "{}",
        text(&missing_approval)
    );
    assert!(
        stderr(&missing_approval).contains("approval-missing"),
        "{}",
        text(&missing_approval)
    );

    let (verifier, signers) = stub_verifier(&env, "privacy-officer");
    let flags = [
        "--engineering-assurance",
        &verifier,
        "--allowed-signers",
        &signers,
    ];
    let mut fact = direct("C-1");
    fact["approvalRef"] = serde_json::json!("person://example.invalid/privacy-officer");
    let recorded = record_output_with(&env, &env.repo, "cross-vocabulary", "1", &[fact], &flags);
    assert!(recorded.status.success(), "{}", text(&recorded));
    assert!(stdout(&recorded).contains("C-1"), "{}", stdout(&recorded));
    let approved_report = json(&discharge_output_with(
        &env,
        &env.repo,
        "cross-vocabulary",
        &flags,
    ));
    assert_eq!(approved_report["binding"]["direct"][0]["clauseId"], "C-1");
    assert_eq!(
        approved_report["binding"]["direct"][0]["verifiedApprover"],
        "r"
    );
}

#[trace("FR-208-AC-7", "FR-208-CON-1")]
#[test]
fn tc_208_006_vocabulary_provider_approval_roles_do_not_apply_to_owner_set() {
    let env = env();
    let owner = env.base.join("src/attributed-owner");
    std::fs::create_dir_all(&owner).unwrap();
    std::fs::write(
        owner.join("manifest.yaml"),
        "name: attributed-owner\nversion: 0.1.0\nclause_sets:\n  - owner.yaml\nobject_types:\n  - name: owner_object\n    data_schema: {type: object}\n    allowed_links: [references]\n",
    )
    .unwrap();
    std::fs::write(
        owner.join("owner.yaml"),
        approval_clause_set("owner-set", "[]"),
    )
    .unwrap();
    let owner_install = env.run(&["module", "install", &format!("path:{}", owner.display())]);
    assert!(owner_install.status.success(), "{}", text(&owner_install));

    let provider = env.base.join("src/policy-provider");
    std::fs::create_dir_all(&provider).unwrap();
    std::fs::write(
        provider.join("manifest.yaml"),
        "name: policy-provider\nversion: 0.1.0\nedge_types:\n  references: {description: Relationship reference, category: dependency}\nclause_sets:\n  - provider.yaml\n",
    )
    .unwrap();
    std::fs::write(
        provider.join("provider.yaml"),
        approval_clause_set("provider-set", "[privacy-officer]"),
    )
    .unwrap();
    let provider_install = env.run(&["module", "install", &format!("path:{}", provider.display())]);
    assert!(
        provider_install.status.success(),
        "{}",
        text(&provider_install)
    );

    let provider_fact = record_output(&env, &env.repo, "provider-set", "1", &[direct("C-1")]);
    assert_eq!(
        provider_fact.status.code(),
        Some(3),
        "{}",
        text(&provider_fact)
    );
    assert!(
        stderr(&provider_fact).contains("approval-missing"),
        "{}",
        text(&provider_fact)
    );

    // Both sets name C-1; only the exact owner-set identity selects the policy
    // that permits a direct fact without an approval reference.
    let recorded = record_output(&env, &env.repo, "owner-set", "1", &[direct("C-1")]);
    assert!(recorded.status.success(), "{}", text(&recorded));
    let report = json(&discharge_output(&env, &env.repo, "owner-set"));
    assert_eq!(report["binding"]["direct"][0]["clauseId"], "C-1");
}

#[trace("FR-208-AC-8")]
#[test]
fn tc_208_007_record_discharge_refuses_an_unknown_edge_type() {
    let env = env();
    let owner = env.base.join("src/unknown-vocabulary-pack");
    std::fs::create_dir_all(&owner).unwrap();
    std::fs::write(
        owner.join("manifest.yaml"),
        "name: unknown-vocabulary-pack\nversion: 0.1.0\nclause_sets:\n  - rules.yaml\nobject_types:\n  - name: owner_object\n    data_schema: {type: object}\n    allowed_links: [references]\n",
    )
    .unwrap();
    std::fs::write(
        owner.join("rules.yaml"),
        approval_clause_set("unknown-vocabulary", "[privacy-officer]"),
    )
    .unwrap();
    let installed = env.run(&["module", "install", &format!("path:{}", owner.display())]);
    assert!(installed.status.success(), "{}", text(&installed));

    let refused = record_output(&env, &env.repo, "unknown-vocabulary", "1", &[direct("C-1")]);
    assert!(!refused.status.success(), "{}", text(&refused));
    assert!(
        text(&refused).contains("UnknownEdgeType") && text(&refused).contains("references"),
        "{}",
        text(&refused)
    );
}

#[trace("FR-208-AC-4")]
#[test]
fn tc_208_008_record_discharge_refuses_duplicate_clause_set_owners() {
    let env = env();
    for name in ["first-owner", "duplicate-owner"] {
        let module = env.base.join("src").join(name);
        std::fs::create_dir_all(&module).unwrap();
        std::fs::write(
            module.join("manifest.yaml"),
            format!("name: {name}\nversion: 0.1.0\nclause_sets:\n  - rules.yaml\n"),
        )
        .unwrap();
        std::fs::write(
            module.join("rules.yaml"),
            approval_clause_set("collision-set", "[privacy-officer]"),
        )
        .unwrap();
        let installed = env.run(&["module", "install", &format!("path:{}", module.display())]);
        assert!(installed.status.success(), "{}", text(&installed));
    }

    let refused = record_output(&env, &env.repo, "collision-set", "1", &[direct("C-1")]);
    assert!(!refused.status.success(), "{}", text(&refused));
    assert!(
        text(&refused).contains(
            "multiple installed modules declare clause set example.invalid/collision-set/1"
        ),
        "{}",
        text(&refused)
    );
}

#[trace("FR-208-AC-4")]
#[test]
fn tc_208_004_duplicate_clause_set_owner_refuses_pack_discharge() {
    let env = env();
    install_approval_fixture(&env);
    let duplicate = env.base.join("src/duplicate-pack");
    std::fs::create_dir_all(&duplicate).unwrap();
    std::fs::write(
        duplicate.join("manifest.yaml"),
        "name: duplicate-pack\nversion: 0.1.0\nclause_sets:\n  - gdpr.yaml\n",
    )
    .unwrap();
    std::fs::copy(env.installed_file("gdpr.yaml"), duplicate.join("gdpr.yaml")).unwrap();
    let installed = env.run(&[
        "module",
        "install",
        &format!("path:{}", duplicate.display()),
    ]);
    assert!(installed.status.success(), "{}", text(&installed));

    let report = env.report(&["--json"]);
    assert!(!report.status.success(), "{}", text(&report));
    assert!(
        text(&report)
            .contains("multiple installed modules declare clause set example.invalid/gdpr/1.0.0"),
        "{}",
        text(&report)
    );
}

#[trace("FR-140-AC-4")]
#[test]
fn tc_1488_004_the_index_title_and_json_members_carry_no_score_or_count_per_value() {
    let env = env();
    record(&env, &env.repo, "alpha", "2", &[direct("A-1")]);
    let markdown = stdout(&env.report(&["--context", "region=eu"]));
    let own = header(&markdown).to_lowercase();
    for word in [
        "%",
        "score",
        "ratio",
        "verdict",
        "pass",
        "fail",
        "compliant",
    ] {
        assert!(!own.contains(word), "{word} in {own}");
    }
    let table: Vec<&str> = own.lines().filter(|l| l.starts_with("| ")).collect();
    assert_eq!(table.len(), 5, "{own}");
    assert_eq!(table[0], "| clause set | rows |");
    assert_eq!(table[2], "| example.invalid/alpha@2 | 2 |");
    assert_eq!(table[3], "| example.invalid/mid\\_set@1 | 2 |");
    assert_eq!(table[4], "| example.invalid/zeta@1 | 2 |");

    let value = json(&env.report(&["--json"]));
    let keys = |v: &serde_json::Value| -> Vec<String> {
        let mut keys: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    };
    assert_eq!(
        keys(&value),
        [
            "asOf",
            "context",
            "index",
            "matrices",
            "pack",
            "schemaVersion"
        ]
    );
    for entry in value["index"].as_array().unwrap() {
        assert_eq!(keys(entry), ["clauseSet", "rowCount"]);
    }
    assert_eq!(value["schemaVersion"], "requirements-mapping-pack-v1");
}

#[trace(
    "FR-140-AC-2",
    "FR-140-AC-3",
    "FR-140-AC-5",
    "FR-140-AC-6",
    "FR-140-AC-13",
    "FR-140-AC-14"
)]
#[test]
fn tc_1488_005_no_licensed_field_reaches_any_output_and_the_search_can_fail() {
    let env = env();
    record(&env, &env.repo, "alpha", "2", &[direct("A-1")]);
    let mut outputs = vec![
        env.report(&[]),
        env.report(&["--json"]),
        env.report(&["--context", "region=eu"]),
        env.report_in(&env.repo, "no-such-pack", &[]),
    ];
    // A module with original text and a module that fails to load refuse without the text.
    let mut open_set = pack_sets().remove(0);
    open_set.0 = "open-zeta";
    let mut expected_sentinels = sentinels();
    expected_sentinels.extend(set_sentinels(&open_set));
    let open = write_pack(
        &env.base.join("open-src"),
        "open-module",
        &[open_set.clone()],
    );
    std::fs::write(
        open.join("open-zeta.yaml"),
        set_yaml(&open_set).replace("text: licensed", "text: original"),
    )
    .unwrap();
    let installed = env.run(&["module", "install", &format!("path:{}", open.display())]);
    assert!(installed.status.success(), "{}", text(&installed));
    let original = env.report_in(&env.repo, "open-module", &["--json"]);
    assert!(original.status.success(), "{}", text(&original));
    let report: serde_json::Value = serde_json::from_slice(&original.stdout).unwrap();
    assert_eq!(report["index"].as_array().unwrap().len(), 1);
    assert_eq!(report["matrices"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["index"][0]["clauseSet"]["authority"],
        "example.invalid"
    );
    assert_eq!(report["index"][0]["clauseSet"]["id"], "open-zeta");
    assert_eq!(report["index"][0]["clauseSet"]["version"], "1");
    let listed = env.run(&["module", "list"]);
    assert!(listed.status.success(), "{}", text(&listed));
    let modules: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(
        modules["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|module| module["name"] == "open-module")
            .unwrap()["pack"],
        false
    );
    outputs.push(original);
    let broken = std::fs::read_to_string(env.installed_file("zeta.yaml"))
        .unwrap()
        .replace(
            &format!("text: \"{}\"", text_sentinel("zeta", 0)),
            &format!("text: [\"{}\"]", text_sentinel("zeta", 0)),
        );
    std::fs::write(env.installed_file("zeta.yaml"), broken).unwrap();
    outputs.push(env.report(&[]));
    for output in &outputs {
        let printed = text(output);
        assert_eq!(
            leaked_from(&printed, &expected_sentinels),
            Vec::<String>::new(),
            "{printed}"
        );
    }

    // The control: a real run that prints a sentinel is caught by the same
    // search, so the check above is able to fail.
    let control = std::process::Command::new("cat")
        .arg(env.installed_file("alpha.yaml"))
        .output()
        .unwrap();
    assert!(!leaked_from(&text(&control), &expected_sentinels).is_empty());
}

/// Everything under the sandbox, plus both repositories' porcelain status.
fn observe(env: &Env) -> (BTreeMap<PathBuf, Vec<u8>>, String, String) {
    (
        snapshot(&env.base),
        git_porcelain(&env.cwd),
        git_porcelain(&env.repo),
    )
}

#[trace("FR-140-AC-6")]
#[test]
fn tc_1488_006_a_report_run_changes_nothing_and_the_oracle_detects_a_write() {
    let env = env();
    record(&env, &env.repo, "alpha", "2", &[direct("A-1")]);
    let before = observe(&env);
    for extra in [&[][..], &["--json"][..], &["--context", "region=us"][..]] {
        let output = env.report(extra);
        assert!(output.status.success(), "{}", text(&output));
    }
    assert!(
        observe(&env) == before,
        "a pack report run changed the sandbox"
    );

    // The control: a deliberate write into a repository is seen by the oracle.
    std::fs::write(env.repo.join("note.txt"), "leaked").unwrap();
    let after = observe(&env);
    assert_ne!(after.0, before.0);
    assert_ne!(after.2, before.2);
}

#[trace("FR-140-AC-7")]
#[test]
fn tc_1488_007_an_unknown_name_and_module_without_clause_sets_exit_two() {
    let env = env();
    let unknown = env.report_in(&env.repo, "no-such-pack", &[]);
    assert_eq!(unknown.status.code(), Some(2), "{}", text(&unknown));
    assert!(unknown.stdout.is_empty());
    assert!(
        stderr(&unknown).contains("no-such-pack"),
        "{}",
        stderr(&unknown)
    );

    let empty = env.base.join("empty-src");
    std::fs::create_dir_all(&empty).unwrap();
    std::fs::write(
        empty.join("manifest.yaml"),
        "name: empty-module\nversion: 0.1.0\n",
    )
    .unwrap();
    let installed = env.run(&["module", "install", &format!("path:{}", empty.display())]);
    assert!(installed.status.success(), "{}", text(&installed));
    let no_sets = env.report_in(&env.repo, "empty-module", &[]);
    assert_eq!(no_sets.status.code(), Some(2), "{}", text(&no_sets));
    assert!(no_sets.stdout.is_empty());
    let message = stderr(&no_sets);
    assert!(message.contains("empty-module"), "{message}");
    assert!(message.contains("declares no clause sets"), "{message}");
}

#[trace("FR-140-AC-8")]
#[test]
fn tc_1488_008_a_set_that_fails_to_load_refuses_the_whole_report_without_the_sentinel() {
    let env = env();
    let sentinel = text_sentinel("zeta", 0);
    let broken = std::fs::read_to_string(env.installed_file("zeta.yaml"))
        .unwrap()
        .replace(
            &format!("text: \"{sentinel}\""),
            &format!("text: [\"{sentinel}\"]"),
        );
    assert!(broken.contains(&format!("[\"{sentinel}\"]")));
    std::fs::write(env.installed_file("zeta.yaml"), broken).unwrap();

    for extra in [&[][..], &["--json"][..]] {
        let output = env.report(extra);
        assert_eq!(output.status.code(), Some(2), "{}", text(&output));
        assert!(output.stdout.is_empty(), "no index and no matrix");
        let message = stderr(&output);
        assert!(message.contains("zeta.yaml"), "{message}");
        assert_eq!(leaked(&message), Vec::<String>::new(), "{message}");
    }
}

#[trace("FR-140-AC-9")]
#[test]
fn tc_1488_009_misused_flags_are_usage_errors_and_a_blank_rationale_refuses_the_report() {
    let env = env();
    let repo = env.repo.to_str().unwrap();
    let binding = env.base.join("binding.json");
    std::fs::write(&binding, env.engine_binding("zeta", "1", &[])).unwrap();
    let binding = binding.to_str().unwrap();
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (
            vec![
                "--rmm",
                "--pack",
                PACK,
                "--repo",
                repo,
                "--binding",
                binding,
            ],
            "--binding",
        ),
        (
            vec!["--rmm", "--pack", PACK, "--repo", repo, "--facts", binding],
            "--facts",
        ),
        (
            vec!["--rmm", "--pack", PACK, "--repo", repo, "--inward"],
            "--inward",
        ),
        (vec!["--rmm", "--pack", PACK], "--repo"),
        (vec!["--pack", PACK, "--repo", repo], "--rmm"),
        (
            vec![
                "--rmm",
                "--repo",
                repo,
                "--binding",
                binding,
                "--context",
                "a=b",
            ],
            "--context",
        ),
    ];
    for (arguments, named) in cases {
        let mut full = vec!["discharge", "--as-of", AS_OF];
        full.extend(arguments.iter());
        let output = env.run(&full);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{arguments:?}: {}",
            text(&output)
        );
        assert!(output.stdout.is_empty());
        assert!(
            stderr(&output).contains(named),
            "{arguments:?}: {}",
            stderr(&output)
        );
    }

    let without = env.run(&["discharge", "--as-of", AS_OF, "--rmm", "--repo", repo]);
    assert!(!without.status.success());
    assert!(
        text(&without).contains("Missing required flag binding"),
        "{}",
        text(&without)
    );
}

#[trace("FR-140-AC-10")]
#[test]
fn tc_1488_010_the_report_is_deterministic_and_untrusted_text_renders_literally() {
    let env = env();
    let second_repo = env.base.join("repo-two");
    git_init(&second_repo);
    let facts = [direct("A-1"), direct("A-2")];
    record(&env, &env.repo, "alpha", "2", &facts);
    record(
        &env,
        &second_repo,
        "alpha",
        "2",
        &[facts[1].clone(), facts[0].clone()],
    );
    let hostile = "note=a|b`c`[l](http://x)<i>y</i>\nsecond";
    for extra in [
        &["--context", hostile][..],
        &["--context", hostile, "--json"][..],
    ] {
        let first = env.report_in(&env.repo, PACK, extra);
        let again = env.report_in(&second_repo, PACK, extra);
        assert!(first.status.success(), "{}", text(&first));
        assert_eq!(first.stdout, again.stdout);
    }

    let markdown = stdout(&env.report(&["--context", hostile]));
    assert!(
        markdown.contains("- note=a\\|b\\`c\\`\\[l\\]\\(http://x\\)\\<i\\>y\\</i\\> second"),
        "{markdown}"
    );
    assert!(
        markdown.contains("| example.invalid/mid\\_set@1 | 2 |"),
        "{markdown}"
    );
    assert!(
        markdown.contains("| M\\-1 ") || markdown.contains("| M-1 "),
        "{markdown}"
    );
    assert!(
        markdown.contains("| M\\|2\\`x\\`\\<b\\>\\[l\\]\\(http://e\\) | Applicable"),
        "{markdown}"
    );
    assert!(
        !markdown.contains("<b>") && !markdown.contains("<i>"),
        "{markdown}"
    );
    assert!(
        markdown.starts_with("# Requirements mapping pack: gadget-pack\n"),
        "{markdown}"
    );
}

#[trace("FR-140-AC-11")]
#[test]
fn tc_1488_011_the_json_validates_against_the_schema_of_the_pack_report_type() {
    let env = env();
    record(&env, &env.repo, "alpha", "2", &[direct("A-1")]);
    let value = json(&env.report(&["--context", "region=eu", "--json"]));
    let schema = serde_json::to_value(schemars::schema_for!(
        quoin_assurance::PackRequirementsMapping
    ))
    .unwrap();
    let validator = quoin_jsonschema::validator::SchemaValidator::compile_vendored(
        Path::new("pack"),
        &schema,
        &[],
    )
    .unwrap();
    assert!(validator.is_valid(&value), "{:?}", validator.errors(&value));
    assert_eq!(value["schemaVersion"], "requirements-mapping-pack-v1");
    for matrix in value["matrices"].as_array().unwrap() {
        assert_eq!(matrix["schemaVersion"], "requirements-mapping-v1");
    }
    // The schema can fail: a document missing a member is invalid.
    let mut broken = value.clone();
    broken.as_object_mut().unwrap().remove("index");
    assert!(!validator.is_valid(&broken));
}

#[trace("FR-140-AC-12")]
#[test]
fn tc_1488_012_the_retained_discharge_help_lists_pack_and_context() {
    let env = env();
    let help = env.run(&["discharge", "--help"]);
    assert!(help.status.success(), "{}", text(&help));
    let printed = stdout(&help);
    assert!(printed.contains("--pack"), "{printed}");
    assert!(printed.contains("--context"), "{printed}");
    assert!(printed.contains("[--binding <value>]"), "{printed}");
}

#[trace("FR-140-AC-9")]
#[test]
fn tc_1488_014_a_blank_stored_rationale_refuses_the_whole_report_as_fr_135_does() {
    let env = env();
    let repo = env.repo.to_str().unwrap();
    // A stored disposition whose rationale is blank, as the store would never
    // have written it.
    let disposition = serde_json::json!({
        "kind": "disposition", "clauseId": "A-1", "decision": "accepted_risk",
        "owner": "risk-owner-1", "rationale": "carried", "approvalRef": "approval://1",
        "attestation": direct("A-1")["attestation"].clone()
    });
    let recorded = record(&env, &env.repo, "alpha", "2", &[disposition]);
    let path = recorded.split_whitespace().last().unwrap().to_owned();
    let stored = env.repo.join(".ix/evidence").join(&path);
    let file = if stored.exists() {
        stored
    } else {
        std::fs::canonicalize(&path).unwrap_or_else(|_| PathBuf::from(&path))
    };
    let mut held: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(held["fact"]["rationale"], "carried");
    held["fact"]["rationale"] = "   ".into();
    std::fs::write(&file, held.to_string()).unwrap();
    let refused = env.report(&[]);
    assert_eq!(refused.status.code(), Some(1), "{}", text(&refused));
    assert!(refused.stdout.is_empty(), "no matrix for any set");

    // The single-set FR-135-AC-6 run over the same store refuses the same way.
    let single_binding = env.base.join("single-refusal.json");
    std::fs::write(&single_binding, env.engine_binding("alpha", "2", &[])).unwrap();
    let single = env.run(&[
        "discharge",
        "--rmm",
        "--binding",
        single_binding.to_str().unwrap(),
        "--repo",
        repo,
        "--as-of",
        AS_OF,
    ]);
    assert_eq!(
        single.status.code(),
        refused.status.code(),
        "{}",
        text(&single)
    );
    // The refusal kind and the store-file sentence are the single-set run's.
    let sentence = |output: &Output| -> String {
        let message = stderr(output);
        assert!(message.contains("CORE_REFUSED"), "{message}");
        let from = message
            .find("discharge/")
            .expect("the refusal names the file");
        let until = message[from..]
            .find(" at line")
            .map_or(message.len(), |at| from + at);
        message[from..until].to_owned()
    };
    assert!(
        sentence(&refused).contains("rationale must not be empty"),
        "{}",
        stderr(&refused)
    );
    assert_eq!(sentence(&refused), sentence(&single));
}
