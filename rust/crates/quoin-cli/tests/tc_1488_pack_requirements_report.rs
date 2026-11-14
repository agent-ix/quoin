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
    let mut all = Vec::new();
    for (set, _, clauses) in pack_sets() {
        all.push(set_title_sentinel(set));
        for index in 0..clauses.len() {
            all.push(text_sentinel(set, index));
            all.push(title_sentinel(set, index));
        }
    }
    all
}

/// The sentinels found in `output`; empty means none leaked.
fn leaked(output: &str) -> Vec<String> {
    sentinels()
        .into_iter()
        .filter(|sentinel| output.contains(sentinel.as_str()))
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
    let output = env.run(&[
        "evidence",
        "record-discharge",
        "--repo",
        repo.to_str().unwrap(),
        "--binding",
        binding_path.to_str().unwrap(),
        "--facts",
        facts_path.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{}", text(&output));
    String::from_utf8(output.stdout).unwrap()
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

#[trace("FR-140-AC-5")]
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
    // A non-pack module and a pack that fails to load refuse without the text.
    let open = write_pack(
        &env.base.join("open-src"),
        "open-module",
        &[pack_sets().remove(0)],
    );
    std::fs::write(
        open.join("zeta.yaml"),
        set_yaml(&pack_sets()[0]).replace("text: licensed", "text: original"),
    )
    .unwrap();
    let installed = env.run(&["module", "install", &format!("path:{}", open.display())]);
    assert!(installed.status.success(), "{}", text(&installed));
    outputs.push(env.report_in(&env.repo, "open-module", &[]));
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
        assert_eq!(leaked(&printed), Vec::<String>::new(), "{printed}");
    }

    // The control: a real run that prints a sentinel is caught by the same
    // search, so the check above is able to fail.
    let control = std::process::Command::new("cat")
        .arg(env.installed_file("alpha.yaml"))
        .output()
        .unwrap();
    assert!(!leaked(&text(&control)).is_empty());
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
fn tc_1488_007_an_unknown_name_and_a_non_pack_module_exit_two_with_no_stdout() {
    let env = env();
    let unknown = env.report_in(&env.repo, "no-such-pack", &[]);
    assert_eq!(unknown.status.code(), Some(2), "{}", text(&unknown));
    assert!(unknown.stdout.is_empty());
    assert!(
        stderr(&unknown).contains("no-such-pack"),
        "{}",
        stderr(&unknown)
    );

    let open = write_pack(
        &env.base.join("open-src"),
        "open-module",
        &[pack_sets().remove(0)],
    );
    std::fs::write(
        open.join("zeta.yaml"),
        set_yaml(&pack_sets()[0]).replace("text: licensed", "text: original"),
    )
    .unwrap();
    let installed = env.run(&["module", "install", &format!("path:{}", open.display())]);
    assert!(installed.status.success(), "{}", text(&installed));
    let not_pack = env.report_in(&env.repo, "open-module", &[]);
    assert_eq!(not_pack.status.code(), Some(2), "{}", text(&not_pack));
    assert!(not_pack.stdout.is_empty());
    let message = stderr(&not_pack);
    assert!(message.contains("open-module"), "{message}");
    assert!(message.contains("is not a pack"), "{message}");
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
