// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Per-requirement activities declared by a software class policy (FR-161).

use std::collections::{BTreeMap, BTreeSet};

use clap::{Arg, ArgMatches, Command, ValueEnum};
use quoin_auditor::audit::class::{ClassFloors, ClassPolicy};
use quoin_core::protocol::{Response, canonical_json};
use serde::Deserialize;
use serde_json::Value;

use crate::core_runtime::invoke;
use crate::evidence::required;
use crate::evidence::audit::{Assembly, AuditOptions, assemble_with, load_class_policy, module_arg};

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum Format { #[default] Markdown, Json }

pub(crate) fn command() -> Command {
    Command::new("class").about("Report activities required by a requirement's stated software safety class")
        .subcommand_required(true)
        .subcommand(Command::new("obligations").about("Report class activities and their evidence")
            .arg(Arg::new("repo").value_name("SCOPE").required(true)).arg(module_arg())
            .arg(Arg::new("class-policy").long("class-policy").required(true))
            .arg(Arg::new("format").long("format").value_parser(clap::value_parser!(Format)).default_value("markdown")))
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let Some(("obligations", arguments)) = matches.subcommand() else { return Err("unknown class command".to_owned()); };
    obligations(arguments)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Activity { id: String, kind: String, #[serde(default)] method: Option<String> }

fn obligations(arguments: &ArgMatches) -> Result<Response, String> {
    let scope = required(arguments, "repo")?;
    let policy_path = required(arguments, "class-policy")?;
    let policy_value = load_class_policy(&policy_path)?;
    let policy: ClassPolicy = serde_json::from_value(policy_value)
        .map_err(|error| format!("{policy_path} is not a class policy: {error}"))?;
    let options = AuditOptions::from_arguments(arguments);
    let audited = match assemble_with(&options, &scope)? { Assembly::Audited(audited) => audited, Assembly::Declined(response) => return Ok(response) };
    let design = invoke("quire.design_join", &serde_json::json!({ "scope": scope, "modules": options.modules() }))?;
    if !design.outcome.carries_payload() { return Ok(design); }
    let catalog_request = if options.modules().is_empty() { serde_json::json!({}) } else { serde_json::json!({ "roots": options.modules() }) };
    let catalog = invoke("catalog.methods", &catalog_request)?;
    if !catalog.outcome.carries_payload() { return Ok(catalog); }
    let methods = catalog_methods(&catalog.payload)?;
    let requirements = audited.coverage.get("coverage_matrix").and_then(Value::as_array)
        .ok_or_else(|| "quire.coverage did not return coverage_matrix".to_owned())?;
    let rows = build_rows(requirements, &design.payload, &policy, &methods, &audited.bindings, &audited.runs, &audited.scans)?;
    let output = serde_json::json!({"requirements": rows.requirements, "unassigned": rows.unassigned, "unconfigured": rows.unconfigured});
    let format = arguments.get_one::<Format>("format").copied().unwrap_or_default();
    let rendered = match format { Format::Json => canonical_json(&output).map_err(|error| error.to_string())?, Format::Markdown => render_markdown(&rows) };
    Ok(Response { payload: serde_json::json!({"rendered": rendered}), diagnostics: audited.response.diagnostics, outcome: audited.response.outcome })
}

struct Rows { requirements: Vec<Value>, unassigned: Vec<String>, unconfigured: Vec<String> }

fn build_rows(matrix: &[Value], design: &Value, policy: &ClassPolicy, methods: &BTreeMap<String, String>, bindings: &Value, runs: &Value, scans: &Value) -> Result<Rows, String> {
    let mut requirements = Vec::new(); let mut unassigned = Vec::new(); let mut unconfigured = Vec::new();
    let relations = design.get("relations").and_then(Value::as_array);
    for item in matrix {
        let document = string(item, "document")?;
        let class = item.get("carried").and_then(|carried| carried.get(&policy.class_field));
        let Some(class) = class else { unassigned.push(document.to_owned()); continue; };
        let class_text = class.as_str().map_or_else(|| class.to_string(), str::to_owned);
        let Some(floors) = policy.classes.get(&class_text) else { unconfigured.push(document.to_owned()); continue; };
        let activities = parse_activities(&class_text, floors, methods)?;
        let criteria = item.get("criteria").and_then(Value::as_array).ok_or_else(|| format!("coverage matrix row `{document}` has no criteria"))?;
        let obligation_ids = criteria.iter().filter_map(|criterion| criterion.get("id").and_then(Value::as_str)).collect::<Vec<_>>();
        let relation_rows = design_rows(relations, document);
        let activity_rows = activities.iter().map(|activity| {
            let status = match activity.kind.as_str() {
                "design-trace" => design_status(relations, &relation_rows),
                "evidence" => evidence_status(activity, methods, &obligation_ids, bindings, runs, scans),
                _ => Status { status: "not-available", missing: vec![], reason: Some("unknown activity kind".to_owned()) },
            };
            let mut row = serde_json::json!({"id": activity.id, "kind": activity.kind, "status": status.status});
            if let (Some(object), Some(method)) = (row.as_object_mut(), &activity.method) { object.insert("method".to_owned(), Value::String(method.clone())); }
            if !status.missing.is_empty() { if let Some(object) = row.as_object_mut() { object.insert("missing".to_owned(), serde_json::json!(status.missing)); } }
            if let (Some(object), Some(reason)) = (row.as_object_mut(), status.reason) { object.insert("reason".to_owned(), Value::String(reason)); }
            row
        }).collect::<Vec<_>>();
        requirements.push(serde_json::json!({"requirement": document, "class": class_text, "activities": activity_rows}));
    }
    Ok(Rows { requirements, unassigned, unconfigured })
}

fn parse_activities(class: &str, floors: &ClassFloors, methods: &BTreeMap<String, String>) -> Result<Vec<Activity>, String> {
    let mut ids = BTreeSet::new(); let mut activities = Vec::new();
    for value in &floors.obligations {
        let activity: Activity = serde_json::from_value(value.clone()).map_err(|error| format!("class `{class}` obligation entry is invalid: {error}"))?;
        if !ids.insert(activity.id.clone()) { return Err(format!("class `{class}` has duplicate obligation id `{}`", activity.id)); }
        match activity.kind.as_str() {
            "design-trace" if activity.method.is_none() => (),
            "evidence" => {
                let Some(method) = activity.method.as_deref() else { return Err(format!("class `{class}` evidence `{}` is missing method", activity.id)); };
                if !methods.contains_key(method) { return Err(format!("class `{class}` evidence `{}` names undeclared method `{method}`", activity.id)); }
            }
            "design-trace" => return Err(format!("class `{class}` design-trace `{}` must not name a method", activity.id)),
            other => return Err(format!("class `{class}` obligation `{}` has unknown kind `{other}`", activity.id)),
        }
        activities.push(activity);
    }
    Ok(activities)
}

struct Status { status: &'static str, missing: Vec<String>, reason: Option<String> }

fn design_status(relations: Option<&Vec<Value>>, rows: &[&Value]) -> Status {
    if relations.map_or(true, Vec::is_empty) { return Status { status: "not-available", missing: vec![], reason: Some("design join has no relations".to_owned()) }; }
    if rows.is_empty() { return Status { status: "not-available", missing: vec![], reason: Some("requirement has no design relation row".to_owned()) }; }
    if rows.iter().any(|row| row.get("state").and_then(Value::as_str) == Some("uncovered")) { Status { status: "unmet", missing: vec![], reason: None } }
    else if rows.iter().any(|row| row.get("state").and_then(Value::as_str) == Some("covered")) { Status { status: "met", missing: vec![], reason: None } }
    else { Status { status: "not-available", missing: vec![], reason: Some("requirement has no design relation row".to_owned()) } }
}

fn design_rows<'a>(relations: Option<&'a Vec<Value>>, document: &str) -> Vec<&'a Value> {
    relations.into_iter().flatten().flat_map(|relation| relation.get("rows").and_then(Value::as_array).into_iter().flatten())
        .filter(|row| row.get("document").and_then(|doc| doc.get("path")).and_then(Value::as_str) == Some(document).or_else(|| row.get("document").and_then(|doc| doc.get("id")).and_then(Value::as_str) == Some(document)))
        .collect()
}

fn evidence_status(activity: &Activity, methods: &BTreeMap<String, String>, obligations: &[&str], bindings: &Value, runs: &Value, scans: &Value) -> Status {
    let kind = activity.method.as_ref().and_then(|method| methods.get(method));
    let mut missing = obligations.iter().filter(|id| !has_evidence(id, kind.map(String::as_str), bindings, runs, scans)).map(|id| (*id).to_owned()).collect::<Vec<_>>();
    missing.sort();
    Status { status: if missing.is_empty() {"met"} else {"unmet"}, missing, reason: None }
}

fn has_evidence(id: &str, kind: Option<&str>, bindings: &Value, runs: &Value, scans: &Value) -> bool {
    let Some(kind) = kind else { return false; };
    bindings.get("bindings").and_then(Value::as_array).into_iter().flatten().any(|binding| {
        if binding.get("obligation").and_then(Value::as_str) != Some(id) { return false; }
        binding.get("suite").and_then(Value::as_str).is_some_and(|suite| contains_kind(runs, suite, kind) || contains_kind(scans, suite, kind))
    })
}

fn contains_kind(records: &Value, suite: &str, kind: &str) -> bool {
    records.as_array().into_iter().flatten().any(|record| record.get("suite").and_then(Value::as_str) == Some(suite) && record.get("evidence_kind").and_then(Value::as_str) == Some(kind))
}

fn catalog_methods(payload: &Value) -> Result<BTreeMap<String, String>, String> {
    payload.get("methods").and_then(Value::as_array).into_iter().flatten().try_fold(BTreeMap::new(), |mut methods, row| {
        let id = string(row, "id")?.to_owned(); if let Some(kind) = row.get("evidence_kind").and_then(Value::as_str) { methods.insert(id, kind.to_owned()); } Ok(methods)
    })
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> { value.get(key).and_then(Value::as_str).ok_or_else(|| format!("expected `{key}` string")) }

fn render_markdown(rows: &Rows) -> String {
    let mut lines = vec!["| Requirement | Class | Activity | Status | Missing obligations |".to_owned(), "|---|---|---|---|---|".to_owned()];
    for requirement in &rows.requirements {
        let id = value_string(requirement, "requirement"); let class = value_string(requirement, "class");
        if let Some(activities) = requirement.get("activities").and_then(Value::as_array) { for activity in activities {
            let missing = activity.get("missing").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect::<Vec<_>>().join(", ");
            lines.push(format!("| {id} | {class} | {} | {} | {missing} |", value_string(activity, "id"), value_string(activity, "status")));
        }}
    }
    for id in &rows.unassigned { lines.push(format!("| {id} | unassigned |  |  |  |")); }
    for id in &rows.unconfigured { lines.push(format!("| {id} | unconfigured |  |  |  |")); }
    lines.join("\n")
}

fn value_string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
