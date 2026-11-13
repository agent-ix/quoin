// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use std::collections::BTreeMap;
use std::path::PathBuf;

use ix_trace_rs::trace;
use quoin_quire_types::ClauseSetKey;
use serde_json::{Value, json};

use super::{Engine, EvaluateError, Evaluation, SetStatus, evaluate};
use crate::profile::model::{EffectiveProfile, Layer, ModuleRef};

fn key(id: &str) -> ClauseSetKey {
    ClauseSetKey {
        authority: "example.invalid".to_owned(),
        id: id.to_owned(),
        version: "1".to_owned(),
    }
}

fn module(name: &str) -> ModuleRef {
    ModuleRef {
        name: name.to_owned(),
        root: PathBuf::from(format!("/m/{name}")),
    }
}

#[derive(Default)]
struct Fake {
    declared: Vec<(&'static str, Vec<&'static str>)>,
    packs: Vec<&'static str>,
    fail: Option<&'static str>,
    malformed: bool,
}

impl Engine for Fake {
    fn declared(&self, module: &ModuleRef) -> Result<Vec<ClauseSetKey>, String> {
        Ok(self
            .declared
            .iter()
            .filter(|(name, _)| *name == module.name)
            .flat_map(|(_, ids)| ids.iter().map(|id| key(id)))
            .collect())
    }

    fn is_pack(&self, module: &ModuleRef) -> Result<bool, String> {
        Ok(self.packs.contains(&module.name.as_str()))
    }

    fn evaluate(
        &self,
        _module: &ModuleRef,
        key: &ClauseSetKey,
        context: &BTreeMap<String, String>,
    ) -> Result<Value, String> {
        if self.fail == Some(key.id.as_str()) {
            return Err("engine broke".to_owned());
        }
        let mut report = json!({
            "schemaVersion": "clause-binding-v1",
            "clauseSet": {"authority": key.authority, "id": key.id, "version": key.version},
            "context": context,
            "clauses": [
                {"clauseId": "C-1", "force": "mandatory", "outcome": "binding",
                 "reasons": [], "expectedOutputs": []},
                {"clauseId": "C-2", "force": "mandatory", "outcome": "unresolved",
                 "reasons": [{"code": "missing-dimension", "dimension": "class",
                              "message": "class is not declared"}],
                 "expectedOutputs": []},
                {"clauseId": "C-3", "force": "permitted", "outcome": "not_binding",
                 "reasons": [], "expectedOutputs": []}
            ]
        });
        if self.malformed {
            report["unexpected"] = json!(true);
        }
        Ok(report)
    }
}

fn profile(ids: &[&str]) -> EffectiveProfile {
    EffectiveProfile {
        name: "p".to_owned(),
        context: BTreeMap::from([("market".to_owned(), "eu".to_owned())]),
        sets: ids.iter().map(|id| (key(id), Layer::Profile)).collect(),
    }
}

fn fake() -> Fake {
    Fake {
        declared: vec![("mod-a", vec!["a"]), ("mod-b", vec!["b"])],
        ..Fake::default()
    }
}

#[trace("FR-181-AC-1", "FR-181-AC-2", "FR-181-AC-6")]
#[test]
fn two_installed_sets_are_reported_in_order_with_counts_and_no_verdict() {
    let report = evaluate(
        &profile(&["a", "b"]),
        &[module("mod-a"), module("mod-b")],
        &fake(),
    )
    .expect("evaluates");
    let ids: Vec<&str> = report.sets.iter().map(|s| s.key.id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
    for entry in &report.sets {
        let set = entry.report.as_ref().expect("report");
        assert_eq!(set.context, report.context);
    }
    assert_eq!(report.summary.sets_evaluated, 2);
    assert_eq!(report.summary.sets_unavailable, 0);
    assert_eq!(report.summary.clauses.binding, 2);
    assert_eq!(report.summary.clauses.unresolved, 2);
    assert_eq!(report.summary.clauses.not_binding, 2);
}

#[trace("FR-181-AC-3")]
#[test]
fn an_undeclared_set_is_unavailable_and_the_others_still_evaluate() {
    let report = evaluate(
        &profile(&["a", "ghost", "b"]),
        &[module("mod-a"), module("mod-b")],
        &fake(),
    )
    .expect("evaluates");
    assert_eq!(report.sets[1].status, SetStatus::Unavailable);
    assert!(report.sets[1].report.is_none());
    assert_eq!(report.summary.sets_unavailable, 1);
    assert_eq!(report.summary.sets_evaluated, 2);
}

#[trace("FR-181-AC-4")]
#[test]
fn a_key_declared_by_two_modules_is_refused_naming_both() {
    let mut engine = fake();
    engine.declared.push(("mod-c", vec!["a"]));
    let error = evaluate(
        &profile(&["a"]),
        &[module("mod-a"), module("mod-c")],
        &engine,
    )
    .expect_err("ambiguous");
    assert!(matches!(error, EvaluateError::AmbiguousKey { .. }));
    let text = error.to_string();
    assert!(text.contains("mod-a") && text.contains("mod-c"), "{text}");
}

#[trace("FR-181-AC-5")]
#[test]
fn an_engine_failure_is_an_error_not_an_unavailable_set() {
    let mut engine = fake();
    engine.fail = Some("a");
    let error = evaluate(&profile(&["a"]), &[module("mod-a")], &engine).expect_err("fails");
    assert!(matches!(error, EvaluateError::Engine { .. }), "{error}");
}

#[trace("FR-181-AC-8")]
#[test]
fn a_report_that_is_not_clause_binding_v1_is_refused() {
    let mut engine = fake();
    engine.malformed = true;
    let error = evaluate(&profile(&["a"]), &[module("mod-a")], &engine).expect_err("invalid");
    assert!(matches!(error, EvaluateError::NotBinding { .. }), "{error}");
}

#[trace("FR-182-AC-7")]
#[test]
fn a_pack_key_is_refused_in_the_project_layer_and_accepted_in_the_user_layer() {
    let mut engine = fake();
    engine.packs.push("mod-a");
    let mut project = profile(&[]);
    project.sets.push((key("a"), Layer::Project));
    let error = evaluate(&project, &[module("mod-a")], &engine).expect_err("pack in project");
    let text = error.to_string();
    assert!(
        text.contains("example.invalid/a@1") && text.contains("project"),
        "{text}"
    );

    let mut user = profile(&[]);
    user.sets.push((key("a"), Layer::User));
    assert!(evaluate(&user, &[module("mod-a")], &engine).is_ok());
}

#[trace("FR-181-AC-9", "NFR-037-AC-3")]
#[test]
fn the_document_shape_is_closed_and_has_no_verdict_field() {
    let report = evaluate(&profile(&["a"]), &[module("mod-a")], &fake()).expect("evaluates");
    let mut value = serde_json::to_value(&report).expect("json");
    assert!(serde_json::from_value::<Evaluation>(value.clone()).is_ok());

    let mut names: Vec<String> = value.as_object().expect("object").keys().cloned().collect();
    names.extend(
        value["summary"]
            .as_object()
            .expect("summary")
            .keys()
            .cloned(),
    );
    names.extend(
        value["summary"]["clauses"]
            .as_object()
            .expect("clauses")
            .keys()
            .cloned(),
    );
    for name in names {
        let lower = name.to_lowercase();
        for word in ["verdict", "score", "grade", "percent", "pass", "compliant"] {
            assert!(!lower.contains(word), "{name} reads as a verdict");
        }
    }

    let mut extra = value.clone();
    extra["score"] = json!(1);
    assert!(serde_json::from_value::<Evaluation>(extra).is_err());
    value.as_object_mut().expect("object").remove("summary");
    assert!(serde_json::from_value::<Evaluation>(value).is_err());
}
