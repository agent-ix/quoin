// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Per-requirement activities declared by a software class policy (FR-161).

use std::collections::{BTreeMap, BTreeSet};

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
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
    /// The design relation a relation-backed kind reads (FR-161).
    #[serde(default)]
    relation: Option<String>,
    /// Archetypes of linked design elements that count, as the policy author
    /// declares them (FR-161).
    #[serde(default)]
    element_archetypes: Vec<String>,
}

/// The kinds that read the design-element join through a named relation and
/// an optional set of linked-element archetypes (FR-161).
const RELATION_KINDS: [&str; 3] = ["detailed-design", "soup-requirements", "segregation"];

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
    // A project that sets `gate: true` in its class policy makes an unmet
    // activity fail the command (FR-161). `not-available` is no gap: nothing
    // asked for the activity. Without the key the report gates nothing.
    let gated = policy.gate && rows.requirements.iter().any(has_unmet);
    Ok(Response {
        payload: serde_json::json!({"rendered": rendered}),
        diagnostics: audited.response.diagnostics,
        outcome: if gated {
            Outcome::Partial
        } else {
            audited.response.outcome
        },
    })
}

fn has_unmet(requirement: &Value) -> bool {
    requirement
        .get("activities")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|activity| activity.get("status").and_then(Value::as_str) == Some("unmet"))
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
                kind if RELATION_KINDS.contains(&kind) => {
                    relation_status(relations, document, activity)
                }
                "evidence" => evidence_status(activity, &class_text, &obligation_ids, missing_methods),
                _ => Status { value: "not-available", missing: vec![], reason: Some("unknown activity kind".to_owned()) },
            };
            let mut row = serde_json::json!({"id": activity.id, "kind": activity.kind, "status": status.value});
            if let (Some(object), Some(method)) = (row.as_object_mut(), &activity.method) { object.insert("method".to_owned(), Value::String(method.clone())); }
            if let (Some(object), Some(relation)) = (row.as_object_mut(), &activity.relation) { object.insert("relation".to_owned(), Value::String(relation.clone())); }
            if !activity.element_archetypes.is_empty() && let Some(object) = row.as_object_mut() { object.insert("element_archetypes".to_owned(), serde_json::json!(activity.element_archetypes)); }
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
        let kind = activity.kind.as_str();
        let id = activity.id.as_str();
        let has_relation_keys =
            activity.relation.is_some() || !activity.element_archetypes.is_empty();
        match kind {
            "design-trace" if activity.method.is_some() || has_relation_keys => {
                return Err(format!(
                    "class `{class}` design-trace `{id}` must not name a method, relation or element archetypes"
                ));
            }
            "design-trace" => (),
            "evidence" => {
                let Some(method) = activity.method.as_deref() else {
                    return Err(format!("class `{class}` evidence `{id}` is missing method"));
                };
                if !methods.contains_key(method) {
                    return Err(format!(
                        "class `{class}` evidence `{id}` names undeclared method `{method}`"
                    ));
                }
                if has_relation_keys {
                    return Err(format!(
                        "class `{class}` evidence `{id}` must not name a relation or element archetypes"
                    ));
                }
            }
            kind if RELATION_KINDS.contains(&kind) => {
                if activity.method.is_some() {
                    return Err(format!(
                        "class `{class}` {kind} `{id}` must not name a method"
                    ));
                }
                let relation_blank = activity
                    .relation
                    .as_deref()
                    .is_some_and(|name| name.trim().is_empty());
                if relation_blank
                    || activity
                        .element_archetypes
                        .iter()
                        .any(|name| name.trim().is_empty())
                {
                    return Err(format!(
                        "class `{class}` {kind} `{id}` has a blank relation or element archetype"
                    ));
                }
                if kind == "detailed-design" && activity.element_archetypes.is_empty() {
                    return Err(format!(
                        "class `{class}` detailed-design `{id}` must name element_archetypes"
                    ));
                }
                if kind != "detailed-design" && activity.relation.is_none() {
                    return Err(format!(
                        "class `{class}` {kind} `{id}` must name a relation"
                    ));
                }
            }
            other => {
                return Err(format!(
                    "class `{class}` obligation `{id}` has unknown kind `{other}`"
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

/// A relation-backed kind (FR-161): the requirement's rows in the named design
/// relation, or in every design relation when none is named, each held to the
/// declared linked-element archetypes.
///
/// A row is satisfied when it is `covered` and, if archetypes are declared, links
/// at least one element of one of them. `unmet` if any row is not satisfied, else
/// `met`. Nothing to read is `not-available`, and the row says which input is
/// missing: a requirement no relation obliges has not been asked for the activity.
fn relation_status(relations: Option<&Vec<Value>>, document: &str, activity: &Activity) -> Status {
    let not_available = |reason: String| Status {
        value: "not-available",
        missing: vec![],
        reason: Some(reason),
    };
    let relations = relations.map(Vec::as_slice).unwrap_or_default();
    if relations.is_empty() {
        return not_available("design join has no relations".to_owned());
    }
    let named = activity.relation.as_deref();
    let selected = relations
        .iter()
        .filter(|relation| {
            named.is_none_or(|name| relation.get("name").and_then(Value::as_str) == Some(name))
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        let name = named.unwrap_or_default();
        return not_available(format!("design join has no relation named `{name}`"));
    }
    let rows = selected
        .iter()
        .flat_map(|relation| {
            relation
                .get("rows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter(|row| row_is_of(row, document))
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return not_available("requirement has no row in the design relation".to_owned());
    }
    let satisfied = |row: &&Value| {
        row.get("state").and_then(Value::as_str) == Some("covered")
            && (activity.element_archetypes.is_empty()
                || row
                    .get("linked")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|element| element.get("archetype").and_then(Value::as_str))
                    .any(|archetype| activity.element_archetypes.iter().any(|a| a == archetype)))
    };
    Status {
        value: if rows.iter().all(satisfied) {
            "met"
        } else {
            "unmet"
        },
        missing: vec![],
        reason: None,
    }
}

/// Whether a join row is about this requirement, by path or by id.
fn row_is_of(row: &Value, document: &str) -> bool {
    let named = |key: &str| {
        row.get("document")
            .and_then(|doc| doc.get(key))
            .and_then(Value::as_str)
            == Some(document)
    };
    named("path") || named("id")
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
        .filter(|row| row_is_of(row, document))
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
        "| Requirement | Class | Activity | Kind | Method | Status | Reason | Missing obligations |".to_owned(),
        "|---|---|---|---|---|---|---|---|".to_owned(),
    ];
    for requirement in &rows.requirements {
        let id = value_string(requirement, "requirement");
        let class = value_string(requirement, "class");
        if let Some(activities) = requirement.get("activities").and_then(Value::as_array) {
            if activities.is_empty() {
                lines.push(format!("| {id} | {class} |  |  |  |  |  |  |"));
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
                let kind = value_string(activity, "kind");
                let method = value_string(activity, "method");
                let reason = value_string(activity, "reason");
                lines.push(format!(
                    "| {id} | {class} | {} | {kind} | {method} | {} | {reason} | {missing} |",
                    value_string(activity, "id"),
                    value_string(activity, "status")
                ));
            }
        }
    }
    for id in &rows.unassigned {
        lines.push(format!("| {id} | unassigned |  |  |  |  |  |  |"));
    }
    for id in &rows.unconfigured {
        lines.push(format!("| {id} | unconfigured |  |  |  |  |  |  |"));
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
        Activity, ClassFloors, ClassPolicy, build_rows, design_status, has_unmet, parse_activities,
        relation_status, render_markdown,
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
            gate: false,
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
        assert!(
            first.contains("| unit | evidence | unit-testing | met |  |  |"),
            "{first}"
        );
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
            gate: false,
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
        assert!(render_markdown(&rows).contains("| spec/FR-1.md | class-1 |  |  |  |  |  |  |"));
        let serialized = serde_json::to_string(&json!({"requirements":rows.requirements,"unassigned":rows.unassigned,"unconfigured":rows.unconfigured})).unwrap();
        for forbidden in ["score", "rank", "verdict", "approval", "recommendation"] {
            assert!(!serialized.contains(forbidden), "{serialized}");
        }
        assert!(!serialized.contains("\"class\":null"), "{serialized}");
    }

    fn activity(value: serde_json::Value) -> Activity {
        serde_json::from_value(value).unwrap()
    }

    fn relation(name: &str, rows: &serde_json::Value) -> serde_json::Value {
        json!({"name": name, "rows": rows})
    }

    #[trace("FR-161-AC-10")]
    #[test]
    fn detailed_design_needs_a_covered_row_linking_a_declared_archetype() {
        let unit =
            activity(json!({"id":"d","kind":"detailed-design","element_archetypes":["unit"]}));
        let row = |state: &str, archetype: &str| {
            json!({"document":{"path":"spec/FR-1.md"},"state":state,
                   "linked":[{"id":"X","archetype":archetype}]})
        };
        let status = |rows: serde_json::Value, document: &str| {
            let relations = vec![relation("r", &rows)];
            relation_status(Some(&relations), document, &unit).value
        };
        assert_eq!(
            status(json!([row("covered", "unit")]), "spec/FR-1.md"),
            "met"
        );
        assert_eq!(
            status(json!([row("covered", "part")]), "spec/FR-1.md"),
            "unmet"
        );
        assert_eq!(
            status(json!([row("uncovered", "unit")]), "spec/FR-1.md"),
            "unmet"
        );
        assert_eq!(
            status(
                json!([row("covered", "unit"), row("covered", "part")]),
                "spec/FR-1.md"
            ),
            "unmet"
        );
        assert_eq!(
            status(json!([row("covered", "unit")]), "spec/FR-9.md"),
            "not-available"
        );
        assert_eq!(
            relation_status(None, "spec/FR-1.md", &unit).value,
            "not-available"
        );
    }

    #[trace("FR-161-AC-11")]
    #[test]
    fn a_named_relation_reads_only_its_own_rows() {
        let soup = activity(json!({"id":"s","kind":"soup-requirements","relation":"soup-rel"}));
        let covered = json!([{"document":{"path":"spec/FR-1.md"},"state":"covered"}]);
        let uncovered = json!([{"document":{"path":"spec/FR-1.md"},"state":"uncovered"}]);
        let relations = vec![
            relation("soup-rel", &covered),
            relation("other", &uncovered),
        ];
        assert_eq!(
            relation_status(Some(&relations), "spec/FR-1.md", &soup).value,
            "met"
        );
        let relations = vec![
            relation("soup-rel", &uncovered),
            relation("other", &covered),
        ];
        assert_eq!(
            relation_status(Some(&relations), "spec/FR-1.md", &soup).value,
            "unmet"
        );
        let relations = vec![relation("other", &covered)];
        let missing = relation_status(Some(&relations), "spec/FR-1.md", &soup);
        assert_eq!(missing.value, "not-available");
        assert!(missing.reason.unwrap().contains("soup-rel"));
    }

    #[trace("FR-161-AC-12")]
    #[test]
    fn relation_backed_entries_are_refused_when_malformed() {
        let methods = BTreeMap::from([("unit-testing".to_owned(), "test-run".to_owned())]);
        let refused = |value: serde_json::Value| {
            parse_activities("class-3", &floors(vec![value]), &methods).unwrap_err()
        };
        assert!(refused(json!({"id":"x","kind":"detailed-design"})).contains("element_archetypes"));
        assert!(refused(json!({"id":"x","kind":"soup-requirements"})).contains("relation"));
        assert!(refused(json!({"id":"x","kind":"segregation"})).contains("relation"));
        assert!(
            refused(json!({"id":"x","kind":"segregation","relation":"r","method":"unit-testing"}))
                .contains("method")
        );
        assert!(refused(json!({"id":"x","kind":"segregation","relation":" "})).contains("blank"));
        assert!(
            refused(json!({"id":"x","kind":"detailed-design","element_archetypes":[""]}))
                .contains("blank")
        );
        assert!(
            refused(json!({"id":"x","kind":"design-trace","relation":"r"})).contains("must not")
        );
        assert!(
            refused(json!({"id":"x","kind":"evidence","method":"unit-testing","element_archetypes":["u"]}))
                .contains("must not")
        );
        let ok = parse_activities(
            "class-3",
            &floors(vec![
                json!({"id":"a","kind":"detailed-design","element_archetypes":["unit"]}),
                json!({"id":"b","kind":"soup-requirements","relation":"r"}),
                json!({"id":"c","kind":"segregation","relation":"r","element_archetypes":["part"]}),
            ]),
            &methods,
        );
        assert_eq!(ok.unwrap().len(), 3);
    }

    #[trace("FR-161-AC-13")]
    #[test]
    fn only_an_unmet_row_counts_toward_the_gate() {
        let met = json!({"activities":[{"status":"met"},{"status":"not-available"}]});
        let unmet = json!({"activities":[{"status":"met"},{"status":"unmet"}]});
        assert!(!has_unmet(&met));
        assert!(has_unmet(&unmet));
        assert!(!has_unmet(&json!({"activities":[]})));
    }
}
