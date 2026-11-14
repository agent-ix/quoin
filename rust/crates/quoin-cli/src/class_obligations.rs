// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Per-requirement activities declared by a software class policy (FR-161).

use std::collections::{BTreeMap, BTreeSet};

use clap::{Arg, ArgMatches, Command};
use quoin_auditor::audit::{ClassFloors, ClassPolicy};
use quoin_core::protocol::{Response, canonical_json};
use serde::Deserialize;
use serde_json::Value;

use crate::core_runtime::invoke;
use crate::evidence::audit::{
    Assembly, AuditOptions, assemble_with, load_class_policy, module_arg,
};
use crate::evidence::required;

pub(crate) fn command() -> Command {
    Command::new("class")
        .about("Report activities required by a requirement's stated software safety class")
        .subcommand_required(true)
        .subcommand(
            Command::new("obligations")
                .about("Report class activities and their evidence")
                .arg(Arg::new("repo").value_name("SCOPE").required(true))
                .arg(module_arg())
                .args(crate::evidence::audit::policy_args().map(|argument| {
                    if argument.get_id() == "class-policy" {
                        argument.required(true)
                    } else {
                        argument
                    }
                }))
                .arg(
                    Arg::new("format")
                        .long("format")
                        .value_parser(["markdown", "json"])
                        .default_value("markdown"),
                ),
        )
}

pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let Some(("obligations", arguments)) = matches.subcommand() else {
        return Err("unknown class command".to_owned());
    };
    obligations(arguments)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Activity {
    id: String,
    kind: String,
    #[serde(default)]
    method: Option<String>,
}

fn obligations(arguments: &ArgMatches) -> Result<Response, String> {
    let scope = required(arguments, "repo")?;
    let policy_path = required(arguments, "class-policy")?;
    let policy_value = load_class_policy(&policy_path)?;
    let policy: ClassPolicy = serde_json::from_value(policy_value)
        .map_err(|error| format!("{policy_path} is not a class policy: {error}"))?;
    let options = AuditOptions::from_arguments(arguments);
    let audited = match assemble_with(&options, &scope)? {
        Assembly::Audited(audited) => audited,
        Assembly::Declined(response) => return Ok(response),
    };
    let design = invoke(
        "quire.design_join",
        &serde_json::json!({ "scope": scope, "modules": options.modules() }),
    )?;
    if !design.outcome.carries_payload() {
        return Ok(design);
    }
    let methods = catalog_methods(&audited.audit_input)?;
    let missing_methods = evidence_gaps(&audited.audit_input, &policy, &methods)?;
    let requirements = audited
        .coverage
        .get("coverage_matrix")
        .and_then(Value::as_array)
        .ok_or_else(|| "quire.coverage did not return coverage_matrix".to_owned())?;
    let rows = build_rows(
        requirements,
        &design.payload,
        &policy,
        &methods,
        &missing_methods,
    )?;
    let output = serde_json::json!({"requirements": rows.requirements, "unassigned": rows.unassigned, "unconfigured": rows.unconfigured});
    let format = arguments
        .get_one::<String>("format")
        .map_or("markdown", String::as_str);
    let rendered = if format == "json" {
        canonical_json(&output).map_err(|error| error.to_string())?
    } else {
        render_markdown(&rows)
    };
    Ok(Response {
        payload: serde_json::json!({"rendered": rendered}),
        diagnostics: audited.response.diagnostics,
        outcome: audited.response.outcome,
    })
}

struct Rows {
    requirements: Vec<Value>,
    unassigned: Vec<String>,
    unconfigured: Vec<String>,
}

fn build_rows(
    matrix: &[Value],
    design: &Value,
    policy: &ClassPolicy,
    methods: &BTreeMap<String, String>,
    missing_methods: &BTreeMap<(String, String), BTreeSet<String>>,
) -> Result<Rows, String> {
    let mut requirements = Vec::new();
    let mut unassigned = Vec::new();
    let mut unconfigured = Vec::new();
    let relations = design.get("relations").and_then(Value::as_array);
    for item in matrix {
        let document = string(item, "document")?;
        let class = item
            .get("carried")
            .and_then(|carried| carried.get(&policy.class_field));
        let Some(class) = class else {
            unassigned.push(document.to_owned());
            continue;
        };
        let class_text = class
            .as_str()
            .map_or_else(|| class.to_string(), str::to_owned);
        let Some(floors) = policy.classes.get(&class_text) else {
            unconfigured.push(document.to_owned());
            continue;
        };
        let activities = parse_activities(&class_text, floors, methods)?;
        let criteria = item
            .get("criteria")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("coverage matrix row `{document}` has no criteria"))?;
        let obligation_ids = criteria
            .iter()
            .filter_map(|criterion| criterion.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        let relation_rows = design_rows(relations, document);
        let activity_rows = activities.iter().map(|activity| {
            let status = match activity.kind.as_str() {
                "design-trace" => design_status(relations, &relation_rows),
                "evidence" => evidence_status(activity, &class_text, &obligation_ids, missing_methods),
                _ => Status { value: "not-available", missing: vec![], reason: Some("unknown activity kind".to_owned()) },
            };
            let mut row = serde_json::json!({"id": activity.id, "kind": activity.kind, "status": status.value});
            if let (Some(object), Some(method)) = (row.as_object_mut(), &activity.method) { object.insert("method".to_owned(), Value::String(method.clone())); }
            if !status.missing.is_empty() && let Some(object) = row.as_object_mut() { object.insert("missing".to_owned(), serde_json::json!(status.missing)); }
            if let (Some(object), Some(reason)) = (row.as_object_mut(), status.reason) { object.insert("reason".to_owned(), Value::String(reason)); }
            row
        }).collect::<Vec<_>>();
        requirements.push(serde_json::json!({"requirement": document, "class": class_text, "activities": activity_rows}));
    }
    Ok(Rows {
        requirements,
        unassigned,
        unconfigured,
    })
}

fn parse_activities(
    class: &str,
    floors: &ClassFloors,
    methods: &BTreeMap<String, String>,
) -> Result<Vec<Activity>, String> {
    let mut ids = BTreeSet::new();
    let mut activities = Vec::new();
    for value in &floors.obligations {
        let activity: Activity = serde_json::from_value(value.clone())
            .map_err(|error| format!("class `{class}` obligation entry is invalid: {error}"))?;
        if activity.id.trim().is_empty() {
            return Err(format!("class `{class}` obligation id must not be empty"));
        }
        if !ids.insert(activity.id.clone()) {
            return Err(format!(
                "class `{class}` has duplicate obligation id `{}`",
                activity.id
            ));
        }
        match activity.kind.as_str() {
            "design-trace" if activity.method.is_none() => (),
            "evidence" => {
                let Some(method) = activity.method.as_deref() else {
                    return Err(format!(
                        "class `{class}` evidence `{}` is missing method",
                        activity.id
                    ));
                };
                if !methods.contains_key(method) {
                    return Err(format!(
                        "class `{class}` evidence `{}` names undeclared method `{method}`",
                        activity.id
                    ));
                }
            }
            "design-trace" => {
                return Err(format!(
                    "class `{class}` design-trace `{}` must not name a method",
                    activity.id
                ));
            }
            other => {
                return Err(format!(
                    "class `{class}` obligation `{}` has unknown kind `{other}`",
                    activity.id
                ));
            }
        }
        activities.push(activity);
    }
    Ok(activities)
}

struct Status {
    value: &'static str,
    missing: Vec<String>,
    reason: Option<String>,
}

fn design_status(relations: Option<&Vec<Value>>, rows: &[&Value]) -> Status {
    if relations.is_none_or(Vec::is_empty) {
        return Status {
            value: "not-available",
            missing: vec![],
            reason: Some("design join has no relations".to_owned()),
        };
    }
    if rows.is_empty() {
        return Status {
            value: "not-available",
            missing: vec![],
            reason: Some("requirement has no design relation row".to_owned()),
        };
    }
    if rows
        .iter()
        .any(|row| row.get("state").and_then(Value::as_str) == Some("uncovered"))
    {
        Status {
            value: "unmet",
            missing: vec![],
            reason: None,
        }
    } else if rows
        .iter()
        .any(|row| row.get("state").and_then(Value::as_str) == Some("covered"))
    {
        Status {
            value: "met",
            missing: vec![],
            reason: None,
        }
    } else {
        Status {
            value: "not-available",
            missing: vec![],
            reason: Some("requirement has no design relation row".to_owned()),
        }
    }
}

fn design_rows<'a>(relations: Option<&'a Vec<Value>>, document: &str) -> Vec<&'a Value> {
    relations
        .into_iter()
        .flatten()
        .flat_map(|relation| {
            relation
                .get("rows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter(|row| {
            row.get("document")
                .and_then(|doc| doc.get("path"))
                .and_then(Value::as_str)
                == Some(document)
                || row
                    .get("document")
                    .and_then(|doc| doc.get("id"))
                    .and_then(Value::as_str)
                    == Some(document)
        })
        .collect()
}

fn evidence_status(
    activity: &Activity,
    class: &str,
    obligations: &[&str],
    missing_methods: &BTreeMap<(String, String), BTreeSet<String>>,
) -> Status {
    let key = activity
        .method
        .as_ref()
        .map(|method| (class.to_owned(), method.clone()));
    let missing_set = key.as_ref().and_then(|key| missing_methods.get(key));
    let mut missing = obligations
        .iter()
        .filter(|id| missing_set.is_some_and(|set| set.contains(**id)))
        .map(|id| (*id).to_owned())
        .collect::<Vec<_>>();
    missing.sort();
    Status {
        value: if missing.is_empty() { "met" } else { "unmet" },
        missing,
        reason: None,
    }
}

fn evidence_gaps(
    audit_input: &Value,
    policy: &ClassPolicy,
    methods: &BTreeMap<String, String>,
) -> Result<BTreeMap<(String, String), BTreeSet<String>>, String> {
    let mut gaps = BTreeMap::new();
    for (class, floors) in &policy.classes {
        for activity in parse_activities(class, floors, methods)? {
            let Some(method) = activity.method else {
                continue;
            };
            let key = (class.clone(), method.clone());
            if gaps.contains_key(&key) {
                continue;
            }
            gaps.insert(key.clone(), audit_method_gaps(audit_input, class, &method)?);
        }
    }
    Ok(gaps)
}

/// Ask the FR-160 auditor to evaluate one declared method through its shared
/// class evidence join. Only the projected `required_methods` changes; the
/// caller's policy file and all evidence inputs remain unchanged.
fn audit_method_gaps(
    audit_input: &Value,
    class: &str,
    method: &str,
) -> Result<BTreeSet<String>, String> {
    let mut input = audit_input.clone();
    let classes = input
        .get_mut("classPolicy")
        .and_then(|policy| policy.get_mut("classes"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "auditor input has no classPolicy.classes".to_owned())?;
    for (name, floor) in classes {
        if let Some(object) = floor.as_object_mut() {
            object.insert(
                "required_methods".to_owned(),
                if name == class {
                    serde_json::json!([method])
                } else {
                    serde_json::json!([])
                },
            );
        }
    }
    let response = invoke("auditor.audit", &serde_json::json!({ "input": input }))?;
    if !response.outcome.carries_payload() {
        return Err("FR-160 evidence join declined a declared class policy".to_owned());
    }
    let findings = response
        .payload
        .get("report")
        .and_then(|report| report.get("findings"))
        .and_then(Value::as_array)
        .ok_or_else(|| "auditor.audit did not return report.findings".to_owned())?;
    Ok(findings
        .iter()
        .filter(|finding| {
            finding.get("kind").and_then(Value::as_str) == Some("class-method-missing")
        })
        .filter_map(|finding| {
            finding
                .get("obligation")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect())
}

fn catalog_methods(input: &Value) -> Result<BTreeMap<String, String>, String> {
    let payload = input
        .get("catalog")
        .ok_or_else(|| "auditor input did not include catalog methods".to_owned())?;
    payload
        .get("methods")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .try_fold(BTreeMap::new(), |mut methods, row| {
            let id = string(row, "id")?.to_owned();
            if let Some(kind) = row.get("evidenceKind").and_then(Value::as_str) {
                methods.insert(id, kind.to_owned());
            }
            Ok(methods)
        })
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("expected `{key}` string"))
}

fn render_markdown(rows: &Rows) -> String {
    let mut lines = vec![
        "| Requirement | Class | Activity | Status | Missing obligations |".to_owned(),
        "|---|---|---|---|---|".to_owned(),
    ];
    for requirement in &rows.requirements {
        let id = value_string(requirement, "requirement");
        let class = value_string(requirement, "class");
        if let Some(activities) = requirement.get("activities").and_then(Value::as_array) {
            if activities.is_empty() {
                lines.push(format!("| {id} | {class} |  |  |  |"));
            }
            for activity in activities {
                let missing = activity
                    .get("missing")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!(
                    "| {id} | {class} | {} | {} | {missing} |",
                    value_string(activity, "id"),
                    value_string(activity, "status")
                ));
            }
        }
    }
    for id in &rows.unassigned {
        lines.push(format!("| {id} | unassigned |  |  |  |"));
    }
    for id in &rows.unconfigured {
        lines.push(format!("| {id} | unconfigured |  |  |  |"));
    }
    lines.join("\n")
}

fn value_string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions report mismatches as panics"
)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::{
        Activity, ClassFloors, ClassPolicy, build_rows, design_status, parse_activities,
        render_markdown,
    };
    use std::collections::BTreeMap;

    fn floors(activities: Vec<serde_json::Value>) -> ClassFloors {
        ClassFloors {
            required_methods: Vec::new(),
            mutation_floor: None,
            independence: Vec::new(),
            obligations: activities,
            rationale: "recorded decision".to_owned(),
        }
    }

    #[trace("FR-161-AC-4")]
    #[test]
    fn design_trace_combines_every_relation_with_uncovered_precedence() {
        let covered = json!({"document":{"path":"spec/FR-1.md"},"state":"covered"});
        let uncovered = json!({"document":{"path":"spec/FR-1.md"},"state":"uncovered"});
        let covered_only = design_status(Some(&vec![json!({})]), &[&covered]);
        let mixed = design_status(Some(&vec![json!({})]), &[&covered, &uncovered]);
        let no_row = design_status(Some(&vec![json!({})]), &[]);
        let empty = design_status(Some(&Vec::new()), &[&uncovered]);
        let absent = design_status(None, &[&uncovered]);
        assert_eq!(covered_only.value, "met");
        assert_eq!(mixed.value, "unmet");
        assert_eq!(no_row.value, "not-available");
        assert_eq!(empty.value, "not-available");
        assert_eq!(absent.value, "not-available");
    }

    #[trace("FR-161-AC-1", "FR-161-AC-2", "FR-161-AC-6", "FR-161-AC-7")]
    #[test]
    fn rows_read_only_carried_class_and_share_method_evidence_by_kind() {
        let activity_data = vec![
            json!({"id":"design","kind":"design-trace"}),
            json!({"id":"unit","kind":"evidence","method":"unit-testing"}),
        ];
        let policy = ClassPolicy {
            class_field: "class".to_owned(),
            classes: BTreeMap::from([("class-3".to_owned(), floors(activity_data))]),
        };
        let matrix = vec![
            json!({"document":"spec/FR-1.md","carried":{"class":"class-3"},"criteria":[{"id":"FR-1-AC-1"},{"id":"FR-1-AC-2"}]}),
            json!({"document":"spec/FR-2.md","criteria":[{"id":"FR-2-AC-1"}]}),
        ];
        let design = json!({"relations":[{"rows":[
            {"document":{"path":"spec/FR-1.md"},"state":"covered","allocated_class":"class-1"},
            {"document":{"path":"spec/FR-2.md"},"state":"uncovered"}
        ]}]});
        let methods = BTreeMap::from([("unit-testing".to_owned(), "test-run".to_owned())]);
        let rows = build_rows(&matrix, &design, &policy, &methods, &BTreeMap::new()).unwrap();
        assert_eq!(rows.requirements.len(), 1);
        assert_eq!(rows.requirements[0]["class"], "class-3");
        assert_eq!(rows.requirements[0]["activities"][0]["status"], "met");
        assert_eq!(rows.requirements[0]["activities"][1]["status"], "met");
        assert_eq!(rows.unassigned, ["spec/FR-2.md"]);
        let first = render_markdown(&rows);
        assert_eq!(first, render_markdown(&rows));
        assert!(first.contains("| unit | met |"), "{first}");
    }

    #[trace("FR-161-AC-8")]
    #[test]
    fn policy_activity_kinds_and_method_declarations_are_closed() {
        let methods = BTreeMap::from([("unit-testing".to_owned(), "test-run".to_owned())]);
        let parse = |activity| parse_activities("class-3", &floors(vec![activity]), &methods);
        assert!(
            parse(json!({"id":"x","kind":"other"}))
                .unwrap_err()
                .contains("unknown kind")
        );
        assert!(
            parse(json!({"id":"x","kind":"evidence"}))
                .unwrap_err()
                .contains("missing method")
        );
        assert!(
            parse(json!({"id":"x","kind":"evidence","method":"missing"}))
                .unwrap_err()
                .contains("undeclared method")
        );
        assert!(
            parse(json!({"id":"","kind":"design-trace"}))
                .unwrap_err()
                .contains("must not be empty")
        );
        assert!(
            parse_activities(
                "class-3",
                &floors(vec![
                    json!({"id":"duplicate","kind":"design-trace"}),
                    json!({"id":"duplicate","kind":"design-trace"}),
                ]),
                &methods
            )
            .unwrap_err()
            .contains("duplicate")
        );
        let malformed: Result<Activity, _> = serde_json::from_value(
            json!({"id":"x","kind":"evidence","method":"unit-testing","extra":true}),
        );
        assert!(malformed.is_err());
    }

    #[trace("FR-161-AC-3", "FR-161-AC-5", "FR-161-CON-1")]
    #[test]
    fn empty_policy_class_has_no_rows_and_unassigned_has_no_class_key() {
        let policy = ClassPolicy {
            class_field: "class".to_owned(),
            classes: BTreeMap::from([
                ("class-1".to_owned(), floors(Vec::new())),
                (
                    "class-3".to_owned(),
                    floors(vec![json!({"id":"design","kind":"design-trace"})]),
                ),
            ]),
        };
        let matrix = vec![
            json!({"document":"spec/FR-1.md","carried":{"class":"class-1"},"criteria":[]}),
            json!({"document":"spec/FR-2.md","carried":{"class":"class-9"},"criteria":[]}),
            json!({"document":"spec/FR-3.md","criteria":[]}),
        ];
        let rows = build_rows(
            &matrix,
            &json!({"relations":[]}),
            &policy,
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(rows.requirements.len(), 1);
        assert_eq!(rows.requirements[0]["activities"], json!([]));
        assert_eq!(rows.unconfigured, ["spec/FR-2.md"]);
        assert_eq!(rows.unassigned, ["spec/FR-3.md"]);
        assert!(render_markdown(&rows).contains("| spec/FR-1.md | class-1 |  |  |  |"));
        let serialized = serde_json::to_string(&json!({"requirements":rows.requirements,"unassigned":rows.unassigned,"unconfigured":rows.unconfigured})).unwrap();
        for forbidden in ["score", "rank", "verdict", "approval", "recommendation"] {
            assert!(!serialized.contains(forbidden), "{serialized}");
        }
        assert!(!serialized.contains("\"class\":null"), "{serialized}");
    }
}
