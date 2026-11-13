// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quoin_quire_types::ClauseSetKey;

use super::{
    Layer, ModuleRef, ProfileError, effective, load_profile, parse_profile, parse_selection,
    parse_user_add,
};

const GOOD: &str = "schemaVersion: source-profile-v1\nname: p\ntitle: P\ndescription: D\n\
    clauseSets:\n  - {authority: example.invalid, id: a, version: \"1\"}\n  \
    - {authority: example.invalid, id: b, version: \"1\"}\ncontext:\n  market: eu\n  kind: device\n";

fn key(id: &str) -> ClauseSetKey {
    ClauseSetKey {
        authority: "example.invalid".to_owned(),
        id: id.to_owned(),
        version: "1".to_owned(),
    }
}

fn profile() -> super::Profile {
    parse_profile(Path::new("p.yaml"), "p", GOOD).expect("good profile")
}

fn rule_of(error: &ProfileError) -> String {
    error.to_string()
}

#[trace("FR-180-AC-1")]
#[test]
fn a_profile_lists_its_sets_in_profile_order() {
    let selection = parse_selection(Path::new("s.yaml"), "profile: p\n").expect("selection");
    let effective = effective(&profile(), &selection, &[]);
    let ids: Vec<&str> = effective.sets.iter().map(|(k, _)| k.id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
}

#[trace("FR-180-AC-2")]
#[test]
fn repository_context_replaces_and_extends_the_profile_context() {
    let selection = parse_selection(
        Path::new("s.yaml"),
        "profile: p\ncontext:\n  market: us\n  class: \"2\"\n",
    )
    .expect("selection");
    let context = effective(&profile(), &selection, &[]).context;
    assert_eq!(context["market"], "us");
    assert_eq!(context["class"], "2");
    assert_eq!(context["kind"], "device");
}

#[trace("FR-180-AC-3")]
#[test]
fn add_follows_the_profile_and_exact_duplicates_appear_once() {
    let selection = parse_selection(
        Path::new("s.yaml"),
        "profile: p\nadd:\n  - {authority: example.invalid, id: a, version: \"1\"}\n  \
         - {authority: example.invalid, id: c, version: \"1\"}\n",
    )
    .expect("selection");
    let sets = effective(&profile(), &selection, &[key("d"), key("c")]).sets;
    let seen: Vec<(&str, Layer)> = sets.iter().map(|(k, l)| (k.id.as_str(), *l)).collect();
    assert_eq!(
        seen,
        [
            ("a", Layer::Profile),
            ("b", Layer::Profile),
            ("c", Layer::Project),
            ("d", Layer::User)
        ]
    );
}

#[trace("FR-182-AC-1")]
#[test]
fn user_keys_follow_project_keys_in_the_user_layer() {
    let selection = parse_selection(Path::new("s.yaml"), "profile: p\n").expect("selection");
    let sets = effective(&profile(), &selection, &[key("z")]).sets;
    assert_eq!(sets.last().map(|(_, l)| *l), Some(Layer::User));
}

#[trace("FR-182-AC-6")]
#[test]
fn a_key_in_both_layers_appears_once_at_project() {
    let selection = parse_selection(
        Path::new("s.yaml"),
        "profile: p\nadd:\n  - {authority: example.invalid, id: c, version: \"1\"}\n",
    )
    .expect("selection");
    let sets = effective(&profile(), &selection, &[key("c")]).sets;
    assert_eq!(sets.iter().filter(|(k, _)| k.id == "c").count(), 1);
    assert!(sets.contains(&(key("c"), Layer::Project)));
}

#[trace("FR-180-AC-6")]
#[test]
fn a_selection_with_an_unknown_key_or_no_profile_is_refused() {
    assert!(parse_selection(Path::new("s.yaml"), "profile: p\nextra: 1\n").is_err());
    assert!(parse_selection(Path::new("s.yaml"), "context: {a: b}\n").is_err());
}

#[trace("FR-180-AC-9")]
#[test]
fn an_integer_context_value_is_refused_naming_the_dimension() {
    let error = parse_selection(Path::new("s.yaml"), "profile: p\ncontext:\n  class: 2\n")
        .expect_err("integer refused");
    assert!(rule_of(&error).contains("class"), "{error}");
    parse_selection(
        Path::new("s.yaml"),
        "profile: p\ncontext:\n  class: \"2\"\n",
    )
    .expect("quoted accepted");
}

#[trace("FR-182-AC-3")]
#[test]
fn a_user_file_with_profile_or_context_is_refused() {
    assert!(parse_user_add(Path::new("u.yaml"), "profile: p\n").is_err());
    assert!(parse_user_add(Path::new("u.yaml"), "context: {a: b}\n").is_err());
    assert_eq!(
        parse_user_add(
            Path::new("u.yaml"),
            "add:\n  - {authority: example.invalid, id: z, version: \"1\"}\n"
        )
        .expect("add only"),
        [key("z")]
    );
}

#[trace("FR-180-AC-8")]
#[test]
fn a_profile_breaking_a_format_rule_is_refused_naming_file_and_rule() {
    let sets = "clauseSets:\n  - {authority: example.invalid, id: a, version: \"1\"}\n";
    let head = "schemaVersion: source-profile-v1\nname: p\ntitle: T\ndescription: D\n";
    let cases = [
        (head.replace("name: p", "name: q") + sets, "stem"),
        (head.replace("-v1", "-v2") + sets, "schemaVersion"),
        (head.to_owned() + "clauseSets: []\n", "at least one"),
        (head.to_owned() + sets + "extra: 1\n", "extra"),
        (head.replace("title: T", "title: \"\"") + sets, "non-empty"),
        (
            head.to_owned() + sets + "context:\n  market: 2\n",
            "expected a string",
        ),
        (
            head.to_owned() + sets + "context:\n  market: \"\"\n",
            "market",
        ),
    ];
    for (text, expected) in cases {
        let error = parse_profile(Path::new("/m/profiles/p.yaml"), "p", &text)
            .expect_err(&format!("{text} must be refused"));
        let message = error.to_string();
        assert!(message.contains("/m/profiles/p.yaml"), "{message}");
        assert!(message.contains(expected), "{expected}: {message}");
    }
}

#[trace("FR-180-AC-8")]
#[test]
fn a_repeated_key_and_a_set_carrying_text_are_refused() {
    let repeated = GOOD.replace("id: b,", "id: a,");
    assert!(
        parse_profile(Path::new("p.yaml"), "p", &repeated)
            .expect_err("repeat")
            .to_string()
            .contains("repeated")
    );
    let texty = GOOD.replace("version: \"1\"}\n  -", "version: \"1\", text: x}\n  -");
    assert!(parse_profile(Path::new("p.yaml"), "p", &texty).is_err());
}

fn module(base: &Path, name: &str, with_profile: Option<&str>) -> ModuleRef {
    let root: PathBuf = base.join(name);
    std::fs::create_dir_all(root.join("profiles")).expect("dir");
    if let Some(text) = with_profile {
        std::fs::write(root.join("profiles/p.yaml"), text).expect("profile");
    }
    ModuleRef {
        name: name.to_owned(),
        root,
    }
}

#[trace("FR-180-AC-4", "FR-180-AC-5", "FR-180-AC-10")]
#[test]
fn profiles_are_found_only_in_module_profile_directories() {
    let base = tempfile::tempdir().expect("tmp");
    let a = module(base.path(), "mod-a", Some(GOOD));
    let b = module(base.path(), "mod-b", None);
    assert_eq!(
        load_profile("p", &[a.clone(), b.clone()])
            .expect("found")
            .name,
        "p"
    );

    let missing = load_profile("q", &[a.clone(), b.clone()]).expect_err("missing");
    assert!(matches!(missing, ProfileError::NotFound { ref name } if name == "q"));
    assert!(missing.to_string().contains('q'));

    // A repository file outside every module root is never read.
    let repo = base.path().join("repo/profiles");
    std::fs::create_dir_all(&repo).expect("dir");
    std::fs::write(repo.join("only-here.yaml"), GOOD).expect("file");
    assert!(load_profile("only-here", &[a.clone(), b.clone()]).is_err());

    let c = module(base.path(), "mod-c", Some(GOOD));
    let duplicate = load_profile("p", &[a, b, c]).expect_err("duplicate");
    let text = duplicate.to_string();
    assert!(text.contains("mod-a") && text.contains("mod-c"), "{text}");
}

#[trace("FR-180-AC-10")]
#[test]
fn a_name_that_is_not_one_plain_component_is_refused_before_any_path_is_built() {
    let base = tempfile::tempdir().expect("tmp");
    let a = module(base.path(), "mod-a", Some(GOOD));
    for name in ["/abs/evil", "../evil", "a/b", "..", ".hidden", "a\\b", ""] {
        let error = load_profile(name, std::slice::from_ref(&a)).expect_err(name);
        assert!(
            matches!(error, ProfileError::InvalidName { .. }),
            "{name}: {error}"
        );
        assert_eq!(error.code(), "profile.invalid_name");
    }
}
