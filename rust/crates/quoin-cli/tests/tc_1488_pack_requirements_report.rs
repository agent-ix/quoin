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
        "kind": "direct", "clauseId": clause, "evidenceRefs": ["evidence://run/one"],
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
    env.run(&[
        "evidence",
        "record-discharge",
        "--repo",
        repo.to_str().unwrap(),
        "--binding",
        binding_path.to_str().unwrap(),
        "--facts",
        facts_path.to_str().unwrap(),
    ])
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
    let binding_path = env.base.join(format!("discharge-binding-{set}.json"));
    std::fs::write(&binding_path, one_clause_binding(set).to_string()).unwrap();
    env.run(&[
        "discharge",
        "--binding",
        binding_path.to_str().unwrap(),
        "--repo",
        repo.to_str().unwrap(),
        "--as-of",
        AS_OF,
        "--json",
    ])
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
        markdown.contains("| A-1 | Applicable | Compliant | evidence://run/one | - | - |"),
        "{markdown}"
    );
    assert!(
        markdown.contains("| Z-1 | Applicable | Not compliant - undischarged"),
        "{markdown}"
    );
}

#[trace("FR-208-AC-1", "FR-208-AC-2", "FR-208-AC-3")]
#[test]
fn tc_208_003_clause_approval_roles_gate_pack_discharge_without_changing_generic_sets() {
    let env = env();
    install_approval_fixture(&env);

    record(
        &env,
        &env.repo,
        "gdpr",
        "1.0.0",
        &[direct("art-35(1)"), direct("no-role")],
    );
    let missing = json(&env.report(&["--json"]));
    let rows = missing["matrices"][0]["rows"].as_array().unwrap();
    let row = |clause_id: &str| {
        rows.iter()
            .find(|row| row["clauseId"] == clause_id)
            .unwrap()
    };
    assert_eq!(row("art-35(1)")["compliance"], "undischarged");
    assert_eq!(
        row("art-35(1)")["reason"],
        "required approval reference is missing"
    );
    assert_eq!(row("no-role")["compliance"], "compliant");

    let mut blank_required_approval = direct("art-35(1)");
    blank_required_approval["approvalRef"] = serde_json::json!(" \t\n");
    record(&env, &env.repo, "gdpr", "1.0.0", &[blank_required_approval]);
    let blank_report = json(&env.report(&["--json"]));
    let blank_rows = blank_report["matrices"][0]["rows"].as_array().unwrap();
    let art_35 = blank_rows
        .iter()
        .find(|row| row["clauseId"] == "art-35(1)")
        .unwrap();
    assert_eq!(art_35["compliance"], "undischarged");
    assert_eq!(art_35["reason"], "required approval reference is missing");

    let mut blank_generic_approval = direct("no-role");
    blank_generic_approval["approvalRef"] = serde_json::json!("");
    let rejected = record_output(&env, &env.repo, "gdpr", "1.0.0", &[blank_generic_approval]);
    assert!(!rejected.status.success(), "{}", text(&rejected));
    assert!(
        stderr(&rejected).contains("approvalRef must not be empty"),
        "{}",
        text(&rejected)
    );

    let mut approved = direct("art-35(1)");
    approved["approvalRef"] = serde_json::json!("person://example.invalid/people/privacy-officer");
    record(&env, &env.repo, "gdpr", "1.0.0", &[approved]);
    let accepted = json(&env.report(&["--json"]));
    let rows = accepted["matrices"][0]["rows"].as_array().unwrap();
    let art_35 = rows
        .iter()
        .find(|row| row["clauseId"] == "art-35(1)")
        .unwrap();
    assert_eq!(art_35["compliance"], "compliant");
    assert_eq!(art_35["evidenceRefs"][0], "evidence://run/one");
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

    let missing_approval =
        record_output(&env, &env.repo, "cross-vocabulary", "1", &[direct("C-1")]);
    assert!(
        missing_approval.status.success(),
        "{}",
        text(&missing_approval)
    );
    let missing_report = json(&discharge_output(&env, &env.repo, "cross-vocabulary"));
    assert!(
        missing_report["binding"]["open"][0]["reason"] == "required approval reference is missing",
        "{missing_report}"
    );

    let mut fact = direct("C-1");
    fact["approvalRef"] = serde_json::json!("person://example.invalid/privacy-officer");
    let recorded = record_output(&env, &env.repo, "cross-vocabulary", "1", &[fact]);
    assert!(recorded.status.success(), "{}", text(&recorded));
    assert!(stdout(&recorded).contains("C-1"), "{}", stdout(&recorded));
    let approved_report = json(&discharge_output(&env, &env.repo, "cross-vocabulary"));
    assert_eq!(approved_report["binding"]["direct"][0]["clauseId"], "C-1");
}

#[trace("FR-208-AC-7")]
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
