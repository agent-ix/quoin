// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin profile evaluate` through the shipped binary (FR-180, FR-181,
//! FR-182, NFR-037).
//!
//! Every module is a synthetic `example.invalid` module installed into a temp
//! `IX_HOME`; the repository under test is a temp directory with a
//! `.ix/source-profile.yaml`.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod pack_support;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Output;

use ix_trace_rs::trace;
use pack_support::{SENTINEL, quoin, scratch, snapshot, text, write_module};
use serde_json::Value;

fn set_yaml(id: &str, version: &str, values: &str) -> String {
    format!(
        "schemaVersion: clause-set-v1\nauthority: example.invalid\nid: {id}\n\
         title: Synthetic {id}\nversion: \"{version}\"\nrights:\n  structure: original\n  \
         text: none\nclassificationDimensions:\n  region:\n    values: {values}\n  class:\n    \
         values: [\"1\", \"2\"]\nclauses:\n  - id: {id}-1\n    force: mandatory\n  - id: {id}-2\n    \
         force: mandatory\n    applicability: {{op: eq, dimension: class, value: \"2\"}}\n  \
         - id: {id}-3\n    force: recommended\n    applicability: {{op: eq, dimension: region, value: eu}}\n"
    )
}

fn profile_yaml(name: &str, ids: &[(&str, &str)], context: &str) -> String {
    let mut yaml = format!(
        "schemaVersion: source-profile-v1\nname: {name}\ntitle: Profile {name}\n\
         description: Synthetic profile\nclauseSets:\n"
    );
    for (id, version) in ids {
        writeln!(
            yaml,
            "  - {{authority: example.invalid, id: {id}, version: \"{version}\"}}"
        )
        .unwrap();
    }
    yaml.push_str(context);
    yaml
}

/// A module with sets `alpha` (region eu/us) and `beta` (region eu/uk) and a
/// set of profiles.
fn write_sets_module(parent: &Path, name: &str, profiles: &[(&str, String)]) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(dir.join("profiles")).unwrap();
    std::fs::write(
        dir.join("manifest.yaml"),
        format!("name: {name}\nversion: 0.1.0\nclause_sets:\n  - alpha.yaml\n  - beta.yaml\n"),
    )
    .unwrap();
    std::fs::write(dir.join("alpha.yaml"), set_yaml("alpha", "1", "[eu, us]")).unwrap();
    std::fs::write(dir.join("beta.yaml"), set_yaml("beta", "1", "[eu, uk]")).unwrap();
    for (file, body) in profiles {
        std::fs::write(dir.join("profiles").join(file), body).unwrap();
    }
    dir
}

struct Env {
    _root: tempfile::TempDir,
    base: PathBuf,
    home: PathBuf,
    cwd: PathBuf,
}

fn env() -> Env {
    let root = scratch();
    let base = root.path().canonicalize().unwrap();
    let (home, cwd) = (base.join("ix-home"), base.join("cwd"));
    std::fs::create_dir_all(&cwd).unwrap();
    pack_support::semantic_root(&home);
    let env = Env {
        _root: root,
        base,
        home,
        cwd,
    };
    let profiles = vec![
        (
            "p.yaml",
            profile_yaml(
                "p",
                &[("alpha", "1"), ("beta", "1")],
                "context:\n  region: us\n",
            ),
        ),
        (
            "q.yaml",
            profile_yaml("q", &[("alpha", "1"), ("ghost", "1"), ("alpha", "9")], ""),
        ),
        ("bad.yaml", profile_yaml("other", &[("alpha", "1")], "")),
    ];
    let module = write_sets_module(&env.base.join("src"), "sets-mod", &profiles);
    env.install(&module);
    env
}

impl Env {
    fn install(&self, source: &Path) {
        let output = self.run(&["module", "install", &format!("path:{}", source.display())]);
        assert!(output.status.success(), "{}", text(&output));
    }

    fn run(&self, arguments: &[&str]) -> Output {
        quoin(&self.home, &self.cwd, arguments, &[])
    }

    fn select(&self, yaml: &str) {
        std::fs::create_dir_all(self.cwd.join(".ix")).unwrap();
        std::fs::write(self.cwd.join(".ix/source-profile.yaml"), yaml).unwrap();
    }

    fn user_file(&self, yaml: &str) {
        std::fs::write(self.home.join("source-profile.yaml"), yaml).unwrap();
    }

    fn evaluate(&self, extra: &[&str]) -> Output {
        let mut arguments = vec!["profile", "evaluate"];
        arguments.extend_from_slice(extra);
        self.run(&arguments)
    }

    fn report(&self) -> Value {
        let output = self.evaluate(&[]);
        assert!(output.status.success(), "{}", text(&output));
        serde_json::from_slice(&output.stdout).expect("stdout is the JSON report")
    }
}

fn outcomes(entry: &Value) -> Vec<(String, String)> {
    entry["report"]["clauses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["clauseId"].as_str().unwrap().to_owned(),
                c["outcome"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[trace(
    "FR-180-AC-1",
    "FR-180-AC-2",
    "FR-181-AC-1",
    "FR-181-AC-2",
    "FR-181-AC-6",
    "FR-181-AC-9"
)]
#[test]
fn a_selected_profile_is_evaluated_with_the_effective_context() {
    let env = env();
    env.select("profile: p\ncontext:\n  class: \"2\"\n");
    let report = env.report();
    let mut top: Vec<&str> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    top.sort_unstable();
    assert_eq!(
        top,
        ["context", "profile", "schemaVersion", "sets", "summary"]
    );
    assert_eq!(report["schemaVersion"], "source-profile-evaluation-v1");
    // Profile context `region: us` is the base; the repository adds `class`.
    assert_eq!(report["context"]["region"], "us");
    assert_eq!(report["context"]["class"], "2");
    let sets = report["sets"].as_array().unwrap();
    let ids: Vec<&str> = sets
        .iter()
        .map(|s| s["key"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["alpha", "beta"]);
    assert_eq!(sets[0]["layer"], "profile");
    assert_eq!(sets[0]["report"]["context"], report["context"]);
    // `us` is admitted by alpha and not by beta (FR-181-AC-10): the value is
    // not rewritten, and beta cannot decide the region clause.
    let alpha = outcomes(&sets[0]);
    assert!(
        alpha.contains(&("alpha-3".to_owned(), "not_binding".to_owned())),
        "{alpha:?}"
    );
    let beta = outcomes(&sets[1]);
    assert!(
        beta.contains(&("beta-3".to_owned(), "unresolved".to_owned())),
        "{beta:?}"
    );
    assert_eq!(report["summary"]["setsEvaluated"], 2);
    assert_eq!(report["summary"]["setsUnavailable"], 0);
    assert!(report["summary"]["clauses"]["unresolved"].as_u64().unwrap() >= 1);
}

#[trace("FR-181-AC-10")]
#[test]
fn a_value_one_set_admits_and_another_does_not_is_not_rewritten() {
    let env = env();
    env.select("profile: p\n");
    let report = env.report();
    // `region: us` comes from the profile. alpha admits it, beta (eu, uk) does not.
    assert_eq!(report["context"]["region"], "us");
    let sets = report["sets"].as_array().unwrap();
    assert_eq!(sets[0]["report"]["context"]["region"], "us");
    assert_eq!(sets[1]["report"]["context"]["region"], "us");
    let alpha = outcomes(&sets[0]);
    assert!(
        alpha.contains(&("alpha-3".to_owned(), "not_binding".to_owned())),
        "{alpha:?}"
    );
    let beta = outcomes(&sets[1]);
    assert!(
        beta.contains(&("beta-3".to_owned(), "unresolved".to_owned())),
        "{beta:?}"
    );
}

#[trace("FR-181-AC-2")]
#[test]
fn a_context_lacking_a_needed_dimension_leaves_the_clause_unresolved() {
    let env = env();
    env.select("profile: p\n");
    let report = env.report();
    let alpha = outcomes(&report["sets"][0]);
    assert!(
        alpha.contains(&("alpha-2".to_owned(), "unresolved".to_owned())),
        "{alpha:?}"
    );
}

#[trace("FR-181-AC-3", "FR-180-AC-12")]
#[test]
fn an_undeclared_set_or_version_is_unavailable_and_kept_at_its_version() {
    let env = env();
    env.select("profile: q\n");
    let report = env.report();
    let sets = report["sets"].as_array().unwrap();
    assert_eq!(sets[0]["status"], "evaluated");
    assert_eq!(sets[1]["status"], "unavailable");
    assert_eq!(sets[1]["key"]["id"], "ghost");
    assert_eq!(sets[2]["status"], "unavailable");
    assert_eq!(sets[2]["key"]["version"], "9");
    assert!(sets[2].get("report").is_none());
    assert_eq!(report["summary"]["setsUnavailable"], 2);
}

#[trace("FR-180-AC-3")]
#[test]
fn project_add_follows_profile_sets_and_exact_duplicates_appear_once() {
    let env = env();
    env.select(
        "profile: p\nadd:\n  - {authority: example.invalid, id: alpha, version: \"1\"}\n  \
         - {authority: example.invalid, id: extra, version: \"1\"}\n",
    );
    let report = env.report();
    let sets = report["sets"].as_array().unwrap();
    let seen: Vec<(&str, &str)> = sets
        .iter()
        .map(|s| {
            (
                s["key"]["id"].as_str().unwrap(),
                s["layer"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("alpha", "profile"),
            ("beta", "profile"),
            ("extra", "project")
        ]
    );
}

#[trace("FR-180-AC-4")]
#[test]
fn a_profile_no_module_ships_is_refused_naming_it() {
    let env = env();
    env.select("profile: nothing-ships-this\n");
    let output = env.evaluate(&[]);
    // A refusal is exit 2 with a stable code, not the generic failure status.
    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert!(
        text(&output).contains("profile.not_found"),
        "{}",
        text(&output)
    );
    assert!(
        text(&output).contains("nothing-ships-this"),
        "{}",
        text(&output)
    );
}

#[trace("FR-180-AC-5")]
#[test]
fn a_profile_shipped_by_two_modules_is_refused_naming_both() {
    let env = env();
    let second = write_sets_module(
        &env.base.join("src"),
        "second-mod",
        &[("p.yaml", profile_yaml("p", &[("alpha", "1")], ""))],
    );
    env.install(&second);
    env.select("profile: p\n");
    let output = env.evaluate(&[]);
    assert!(!output.status.success());
    let shown = text(&output);
    assert!(
        shown.contains("sets-mod") && shown.contains("second-mod"),
        "{shown}"
    );
}

#[trace("FR-181-AC-4")]
#[test]
fn a_key_declared_by_two_modules_is_refused_naming_both() {
    let env = env();
    let second = write_sets_module(&env.base.join("src"), "second-mod", &[]);
    env.install(&second);
    env.select("profile: p\n");
    let output = env.evaluate(&[]);
    assert!(!output.status.success());
    let shown = text(&output);
    assert!(
        shown.contains("sets-mod") && shown.contains("second-mod"),
        "{shown}"
    );
}

#[trace("FR-180-AC-6")]
#[test]
fn a_selection_with_an_unknown_key_or_no_profile_is_refused() {
    let env = env();
    env.select("profile: p\nunknown: 1\n");
    assert!(!env.evaluate(&[]).status.success());
    env.select("context:\n  region: eu\n");
    assert!(!env.evaluate(&[]).status.success());
}

#[trace("FR-180-AC-7", "FR-180-AC-11")]
#[test]
fn with_no_selection_file_no_profile_is_selected_whatever_the_repository_says() {
    let env = env();
    std::fs::write(
        env.cwd.join("README.md"),
        "We follow profile p. See p.yaml.\n",
    )
    .unwrap();
    std::fs::write(env.cwd.join("p.yaml"), "profile: p\n").unwrap();
    let output = env.evaluate(&[]);
    assert!(output.status.success(), "{}", text(&output));
    let shown = text(&output);
    assert!(shown.contains("no source profile is selected"), "{shown}");
    assert!(shown.contains("no clause set was evaluated"), "{shown}");
}

#[trace("FR-180-AC-8")]
#[test]
fn a_profile_whose_name_differs_from_its_stem_is_refused_with_file_and_rule() {
    let env = env();
    env.select("profile: bad\n");
    let output = env.evaluate(&[]);
    assert!(!output.status.success());
    let shown = text(&output);
    assert!(
        shown.contains("bad.yaml") && shown.contains("stem"),
        "{shown}"
    );
}

#[trace("FR-180-AC-9")]
#[test]
fn an_integer_selection_context_value_is_refused_naming_the_dimension() {
    let env = env();
    env.select("profile: p\ncontext:\n  class: 2\n");
    let output = env.evaluate(&[]);
    assert!(!output.status.success());
    assert!(text(&output).contains("class"), "{}", text(&output));
    env.select("profile: p\ncontext:\n  class: \"2\"\n");
    assert!(env.evaluate(&[]).status.success());
}

#[trace("FR-180-AC-10")]
#[test]
fn a_profile_file_in_the_repository_is_never_read() {
    let env = env();
    std::fs::create_dir_all(env.cwd.join("profiles")).unwrap();
    std::fs::write(
        env.cwd.join("profiles/only-here.yaml"),
        profile_yaml("only-here", &[("alpha", "1")], ""),
    )
    .unwrap();
    env.select("profile: only-here\n");
    let output = env.evaluate(&[]);
    assert!(!output.status.success());
    assert!(text(&output).contains("only-here"));

    // A name that reaches the repository file by an absolute path or by `..`
    // is refused as a name, and the file is not read.
    let evil = env.cwd.join("evil");
    std::fs::write(
        env.cwd.join("evil.yaml"),
        profile_yaml(evil.to_str().unwrap(), &[("alpha", "1")], ""),
    )
    .unwrap();
    for name in [
        evil.to_str().unwrap().to_owned(),
        "../../../../../../../../tmp/x".to_owned(),
        "sub/evil".to_owned(),
        ".hidden".to_owned(),
    ] {
        env.select(&format!("profile: \"{name}\"\n"));
        let output = env.evaluate(&[]);
        assert_eq!(output.status.code(), Some(2), "{name}: {}", text(&output));
        assert!(
            text(&output).contains("profile.invalid_name"),
            "{name}: {}",
            text(&output)
        );
    }
}

/// Provenance: the selection file is repository-controlled input; an invalid
/// one is refused before it can cost unbounded time or memory.
#[trace("FR-180-AC-6")]
#[test]
fn an_alias_bomb_or_an_oversized_selection_file_is_refused_cheaply() {
    let env = env();
    let mut bomb = String::from("profile: p\nx0: &a0 [y, y, y, y, y, y, y, y, y]\n");
    for level in 1..12 {
        let refs = vec![format!("*a{}", level - 1); 9].join(", ");
        writeln!(bomb, "x{level}: &a{level} [{refs}]").unwrap();
    }
    env.select(&bomb);
    let started = std::time::Instant::now();
    let output = env.evaluate(&[]);
    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert!(
        text(&output).contains("profile.invalid_selection"),
        "{}",
        text(&output)
    );
    assert!(
        started.elapsed().as_secs() < 20,
        "an alias must not be expanded"
    );

    env.select(&format!("profile: p\n# {}\n", "x".repeat(70 * 1024)));
    let output = env.evaluate(&[]);
    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert!(
        text(&output).contains("profile.file_too_large"),
        "{}",
        text(&output)
    );
}

#[trace("FR-181-AC-7")]
#[test]
fn out_dir_files_equal_the_engine_report_and_discharge_accepts_them() {
    let env = env();
    env.select("profile: p\ncontext:\n  class: \"2\"\n");
    let out = env.base.join("out");
    let output = env.evaluate(&["--out-dir", out.to_str().unwrap()]);
    assert!(output.status.success(), "{}", text(&output));
    let mut files: Vec<PathBuf> = std::fs::read_dir(&out)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    assert_eq!(files.len(), 2);

    let root = quoin_quire::ModuleRoot::open(env.home.join("filament/modules/sets-mod")).unwrap();
    let reference = quoin_quire::ClauseSetRef {
        authority: quoin_quire::ClauseSetAuthority::new("example.invalid").unwrap(),
        id: quoin_quire::ClauseSetId::new("alpha").unwrap(),
        version: quoin_quire::ClauseSetVersion::new("1").unwrap(),
    };
    let context = quoin_quire::clauses::Context::from_entries(&["region=us", "class=2"]).unwrap();
    let engine =
        quoin_quire::clauses::evaluate(&root, &reference, &context, &mut Vec::new()).unwrap();
    let expected = quoin_core::protocol::canonical_json(&engine).unwrap();
    assert_eq!(std::fs::read_to_string(&files[0]).unwrap(), expected);

    let repo = env.base.join("repo");
    pack_support::git_init(&repo);
    let discharge = env.run(&[
        "discharge",
        "--rmm",
        "--binding",
        files[0].to_str().unwrap(),
        "--repo",
        repo.to_str().unwrap(),
        "--as-of",
        "2026-08-15T00:00:00Z",
    ]);
    assert!(discharge.status.success(), "{}", text(&discharge));
}

#[trace("FR-181-AC-11", "NFR-037-AC-1", "NFR-037-AC-2")]
#[test]
fn a_run_is_byte_identical_offline_and_changes_nothing() {
    let env = env();
    env.select("profile: p\n");
    env.user_file("add:\n  - {authority: example.invalid, id: extra, version: \"1\"}\n");
    std::fs::create_dir_all(env.cwd.join("spec")).unwrap();
    std::fs::write(env.cwd.join("spec/FR-1.md"), "requirement\n").unwrap();
    // The whole home holds the module roots and the user selection file.
    let watched = |env: &Env| {
        (
            snapshot(&env.home),
            snapshot(&env.cwd.join(".ix")),
            snapshot(&env.cwd.join("spec")),
        )
    };
    let before = watched(&env);
    let out_a = env.base.join("out-a");
    let out_b = env.base.join("out-b");
    let first = env.evaluate(&["--out-dir", out_a.to_str().unwrap()]);
    let proxy = Path::new("http://127.0.0.1:9");
    let offline = quoin(
        &env.home,
        &env.cwd,
        &["profile", "evaluate", "--out-dir", out_b.to_str().unwrap()],
        &[
            ("HTTP_PROXY", proxy),
            ("HTTPS_PROXY", proxy),
            ("ALL_PROXY", proxy),
        ],
    );
    assert!(first.status.success() && offline.status.success());
    assert_eq!(first.stdout, offline.stdout);
    let files = |dir: &Path| snapshot(dir).into_values().collect::<Vec<_>>();
    assert_eq!(files(&out_a), files(&out_b));
    assert_eq!(before, watched(&env));
}

/// The run inside an empty network namespace, where no socket can leave the
/// process. Skipped, loudly, where user namespaces are not permitted.
#[trace("NFR-037-AC-2")]
#[test]
fn a_run_with_no_network_namespace_gives_the_same_output() {
    let env = env();
    env.select("profile: p\n");
    let probe = std::process::Command::new("unshare")
        .args(["-rn", "true"])
        .status();
    if !probe.is_ok_and(|status| status.success()) {
        eprintln!("skipped: `unshare -rn` is not permitted on this host");
        return;
    }
    let with_network = env.evaluate(&[]);
    let semantic = pack_support::semantic_root(&env.home);
    let isolated = std::process::Command::new("unshare")
        .args(["-rn", env!("CARGO_BIN_EXE_quoin"), "profile", "evaluate"])
        .current_dir(&env.cwd)
        .env("IX_HOME", &env.home)
        .env("QUOIN_SEMANTIC_ROOT", semantic)
        .output()
        .unwrap();
    assert!(isolated.status.success(), "{}", text(&isolated));
    assert_eq!(with_network.stdout, isolated.stdout);
}

fn install_pack(env: &Env) {
    let dir = write_module(&env.base.join("src"), "licensed-pack", "licensed");
    env.install(&dir);
}

fn pack_key() -> &'static str {
    "  - {authority: example.invalid, id: rules, version: \"2\"}\n"
}

#[trace("FR-182-AC-1", "FR-182-AC-5")]
#[test]
fn a_user_added_pack_set_is_evaluated_last_with_layer_user_and_leaks_no_text() {
    let env = env();
    install_pack(&env);
    env.select("profile: p\nadd:\n  - {authority: example.invalid, id: extra, version: \"1\"}\n");
    env.user_file(&format!("add:\n{}", pack_key()));
    let output = env.evaluate(&[]);
    assert!(output.status.success(), "{}", text(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let sets = report["sets"].as_array().unwrap();
    let last = sets.last().unwrap();
    assert_eq!(last["key"]["id"], "rules");
    assert_eq!(last["layer"], "user");
    assert_eq!(last["status"], "evaluated");
    assert_eq!(sets[sets.len() - 2]["layer"], "project");
    let shown = text(&output);
    assert!(
        !shown.contains(SENTINEL) && !shown.contains("Synthetic set"),
        "{shown}"
    );
    assert!(!shown.contains("\"text\"") && !shown.contains("\"title\""));
}

#[trace("FR-182-AC-2")]
#[test]
fn the_same_repository_without_the_pack_reports_the_key_unavailable() {
    let env = env();
    env.select("profile: p\n");
    env.user_file(&format!("add:\n{}", pack_key()));
    let report = env.report();
    let last = report["sets"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["key"]["id"], "rules");
    assert_eq!(last["status"], "unavailable");
}

#[trace("FR-182-AC-3")]
#[test]
fn a_user_file_carrying_profile_or_context_is_refused() {
    let env = env();
    env.select("profile: p\n");
    env.user_file("profile: p\n");
    assert!(!env.evaluate(&[]).status.success());
    env.user_file("context:\n  region: eu\n");
    assert!(!env.evaluate(&[]).status.success());
}

#[trace("FR-182-AC-4")]
#[test]
fn adding_a_user_key_changes_no_module_file() {
    let env = env();
    install_pack(&env);
    env.select("profile: p\n");
    env.user_file(&format!("add:\n{}", pack_key()));
    let before = snapshot(&env.home.join("filament"));
    assert!(env.evaluate(&[]).status.success());
    assert_eq!(before, snapshot(&env.home.join("filament")));
}

#[trace("FR-182-AC-6")]
#[test]
fn a_key_in_both_layers_appears_once_with_layer_project() {
    let env = env();
    env.select("profile: p\nadd:\n  - {authority: example.invalid, id: extra, version: \"1\"}\n");
    env.user_file("add:\n  - {authority: example.invalid, id: extra, version: \"1\"}\n");
    let report = env.report();
    let extras: Vec<&Value> = report["sets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["key"]["id"] == "extra")
        .collect();
    assert_eq!(extras.len(), 1);
    assert_eq!(extras[0]["layer"], "project");
}

#[trace("FR-182-AC-7")]
#[test]
fn a_pack_key_in_the_project_layer_is_refused_naming_key_and_layer() {
    let env = env();
    install_pack(&env);
    env.select(&format!("profile: p\nadd:\n{}", pack_key()));
    let output = env.evaluate(&[]);
    assert!(!output.status.success());
    let shown = text(&output);
    assert!(
        shown.contains("rules") && shown.contains("project"),
        "{shown}"
    );
    // The same key in the user file is accepted.
    env.select("profile: p\n");
    env.user_file(&format!("add:\n{}", pack_key()));
    assert!(env.evaluate(&[]).status.success());
}

#[trace("FR-182-AC-8")]
#[test]
fn a_user_file_alone_selects_no_profile_and_is_not_read() {
    let env = env();
    // Unreadable as a user file; were it read, the run would be refused.
    env.user_file("profile: p\n");
    let output = env.evaluate(&[]);
    assert!(output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("no source profile is selected"));
}
