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
    kind: Kind,
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

/// The activity kinds quoin knows (FR-161). A policy entry of any other kind is
/// refused when the policy is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    DesignTrace,
    Evidence,
    DetailedDesign,
    SoupRequirements,
    Segregation,
}

impl Kind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::DesignTrace => "design-trace",
            Self::Evidence => "evidence",
            Self::DetailedDesign => "detailed-design",
            Self::SoupRequirements => "soup-requirements",
            Self::Segregation => "segregation",
        }
    }
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
    check_relations_declared(&policy, &design.payload, &methods)?;
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
        let activity_rows = activities.iter().map(|activity| {
            let status = match activity.kind {
                Kind::DesignTrace
                | Kind::DetailedDesign
                | Kind::SoupRequirements
                | Kind::Segregation => relation_status(relations, document, activity),
                Kind::Evidence => evidence_status(activity, &class_text, &obligation_ids, missing_methods),
            };
            let mut row = serde_json::json!({"id": activity.id, "kind": activity.kind.as_str(), "status": status.value});
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
        let kind = activity.kind;
        let name = kind.as_str();
        let id = activity.id.as_str();
        let has_relation_keys =
            activity.relation.is_some() || !activity.element_archetypes.is_empty();
        match kind {
            Kind::DesignTrace => {
                if activity.method.is_some() {
                    return Err(format!(
                        "class `{class}` design-trace `{id}` must not name a method"
                    ));
                }
                if !activity.element_archetypes.is_empty() {
                    return Err(format!(
                        "class `{class}` design-trace `{id}` must not name element archetypes"
                    ));
                }
                if activity
                    .relation
                    .as_deref()
                    .is_some_and(|name| name.trim().is_empty())
                {
                    return Err(format!(
                        "class `{class}` design-trace `{id}` has a blank relation"
                    ));
                }
            }
            Kind::Evidence => {
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
            Kind::DetailedDesign | Kind::SoupRequirements | Kind::Segregation => {
                if activity.method.is_some() {
                    return Err(format!(
                        "class `{class}` {name} `{id}` must not name a method"
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
                        "class `{class}` {name} `{id}` has a blank relation or element archetype"
                    ));
                }
                if kind == Kind::DetailedDesign && activity.element_archetypes.is_empty() {
                    return Err(format!(
                        "class `{class}` detailed-design `{id}` must name element_archetypes"
                    ));
                }
                if kind != Kind::DetailedDesign && activity.relation.is_none() {
                    return Err(format!(
                        "class `{class}` {name} `{id}` must name a relation"
                    ));
                }
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

fn not_available(reason: String) -> Status {
    Status {
        value: "not-available",
        missing: vec![],
        reason: Some(reason),
    }
}

/// How `design-trace` and the relation-backed kinds read FR-086 join rows, in
/// one place: a row is `covered`, `uncovered`, or states neither and is not
/// read. `unmet` if any read row is not satisfied, `met` if at least one is and
/// none is not, `None` (not available) if no row was read. A row is satisfied
/// when it is `covered` and, if `archetypes` is non-empty, links an element of
/// one of them.
fn fold_rows(rows: &[&Value], archetypes: &[String]) -> Option<&'static str> {
    let mut read = false;
    let mut unmet = false;
    for row in rows {
        let covered = match row.get("state").and_then(Value::as_str) {
            Some("covered") => true,
            Some("uncovered") => false,
            _ => continue,
        };
        read = true;
        let linked = archetypes.is_empty()
            || row
                .get("linked")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|element| element.get("archetype").and_then(Value::as_str))
                .any(|archetype| archetypes.iter().any(|wanted| wanted == archetype));
        unmet |= !(covered && linked);
    }
    match (read, unmet) {
        (false, _) => None,
        (true, true) => Some("unmet"),
        (true, false) => Some("met"),
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
    let relations = relations.map(Vec::as_slice).unwrap_or_default();
    if relations.is_empty() {
        return not_available("design join has no relations".to_owned());
    }
    let named = activity.relation.as_deref();
    let mut rows = Vec::new();
    let mut uncovered_relations = Vec::new();
    for relation in relations.iter().filter(|relation| {
        named.is_none_or(|name| relation.get("name").and_then(Value::as_str) == Some(name))
    }) {
        let own = relation
            .get("rows")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|row| row_is_of(row, document))
            .collect::<Vec<_>>();
        if let Some(name) = relation.get("name").and_then(Value::as_str)
            && own
                .iter()
                .any(|row| row.get("state").and_then(Value::as_str) == Some("uncovered"))
        {
            uncovered_relations.push(name.to_owned());
        }
        rows.extend(own);
    }
    match fold_rows(&rows, &activity.element_archetypes) {
        Some(value) => Status {
            value,
            // Only `design-trace` lists the relations it found uncovered (FR-161).
            missing: if value == "unmet" && activity.kind == Kind::DesignTrace {
                uncovered_relations
            } else {
                vec![]
            },
            reason: None,
        },
        None if activity.kind == Kind::DesignTrace && named.is_none() => {
            not_available("requirement has no design relation row".to_owned())
        }
        None => not_available("requirement has no row in the design relation".to_owned()),
    }
}

/// A `relation` no loaded module declares as a design relation is a policy
/// error, refused as an undeclared method is: a typo or a module that is not
/// loaded would otherwise read as `not-available` forever.
fn check_relations_declared(
    policy: &ClassPolicy,
    design: &Value,
    methods: &BTreeMap<String, String>,
) -> Result<(), String> {
    let declared = design
        .get("relations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|relation| relation.get("name").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    for (class, floors) in &policy.classes {
        for activity in parse_activities(class, floors, methods)? {
            if let Some(name) = activity.relation.as_deref()
                && !declared.contains(name)
            {
                return Err(format!(
                    "class `{class}` {kind} `{id}` names relation `{name}`, which no loaded module declares as a design relation",
                    kind = activity.kind.as_str(),
                    id = activity.id,
                ));
            }
        }
    }
    Ok(())
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
        Activity, ClassFloors, ClassPolicy, build_rows, check_relations_declared, has_unmet,
        parse_activities, relation_status, render_markdown,
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
        let trace_all = activity(json!({"id":"d","kind":"design-trace"}));
        let row = |state: &str| json!({"document":{"path":"spec/FR-1.md"},"state":state});
        let status = |relations: Option<&Vec<serde_json::Value>>, document: &str| {
            relation_status(relations, document, &trace_all).value
        };
        let one = vec![relation("a", &json!([row("covered")]))];
        let two = vec![
            relation("a", &json!([row("covered")])),
            relation("b", &json!([row("uncovered")])),
        ];
        let empty = Vec::new();
        assert_eq!(status(Some(&one), "spec/FR-1.md"), "met");
        assert_eq!(status(Some(&two), "spec/FR-1.md"), "unmet");
        assert_eq!(status(Some(&one), "spec/FR-9.md"), "not-available");
        assert_eq!(status(Some(&empty), "spec/FR-1.md"), "not-available");
        assert_eq!(status(None, "spec/FR-1.md"), "not-available");
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
                .contains("unknown variant `other`")
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
        // A requirement with no row in the named relation is not a gap.
        let elsewhere = json!([{"document":{"path":"spec/FR-2.md"},"state":"uncovered"}]);
        let relations = vec![
            relation("soup-rel", &elsewhere),
            relation("other", &covered),
        ];
        let no_row = relation_status(Some(&relations), "spec/FR-1.md", &soup);
        assert_eq!(no_row.value, "not-available");
    }

    #[trace("FR-161-AC-14")]
    #[test]
    fn a_relation_no_loaded_module_declares_is_refused_naming_it() {
        let methods = BTreeMap::new();
        let policy = ClassPolicy {
            class_field: "class".to_owned(),
            classes: BTreeMap::from([(
                "class-3".to_owned(),
                floors(vec![
                    json!({"id":"s","kind":"soup-requirements","relation":"typo"}),
                ]),
            )]),
            gate: false,
        };
        let declared = json!({"relations":[{"name":"real","rows":[]}]});
        let error = check_relations_declared(&policy, &declared, &methods).unwrap_err();
        assert!(
            error.contains("`typo`") && error.contains("no loaded module"),
            "{error}"
        );
        let empty = json!({"relations":[]});
        assert!(check_relations_declared(&policy, &empty, &methods).is_err());
        let ok = ClassPolicy {
            classes: BTreeMap::from([(
                "class-3".to_owned(),
                floors(vec![
                    json!({"id":"s","kind":"segregation","relation":"real"}),
                ]),
            )]),
            ..policy
        };
        assert!(check_relations_declared(&ok, &declared, &methods).is_ok());
    }

    #[trace("FR-161-AC-4", "FR-161-AC-10")]
    #[test]
    fn design_trace_and_relation_kinds_read_row_states_identically() {
        let rows = |states: &[&str]| {
            let list: Vec<_> = states
                .iter()
                .map(|state| json!({"document":{"path":"spec/FR-1.md"},"state":state}))
                .collect();
            vec![relation("r", &json!(list))]
        };
        let by_relation = activity(json!({"id":"s","kind":"segregation","relation":"r"}));
        let trace_all = activity(json!({"id":"d","kind":"design-trace"}));
        for states in [
            &["covered"][..],
            &["uncovered"][..],
            &["covered", "uncovered"][..],
            &["other"][..],
            &["other", "covered"][..],
        ] {
            let relations = rows(states);
            assert_eq!(
                relation_status(Some(&relations), "spec/FR-1.md", &trace_all).value,
                relation_status(Some(&relations), "spec/FR-1.md", &by_relation).value,
                "{states:?}"
            );
        }
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
            refused(json!({"id":"x","kind":"design-trace","element_archetypes":["u"]}))
                .contains("must not")
        );
        assert!(refused(json!({"id":"x","kind":"design-trace","relation":" "})).contains("blank"));
        let by_method = refused(json!({"id":"x","kind":"design-trace","method":"unit-testing"}));
        assert!(
            by_method.contains("must not name a method") && !by_method.contains("archetypes"),
            "{by_method}"
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
                json!({"id":"d","kind":"design-trace","relation":"r"}),
            ]),
            &methods,
        );
        assert_eq!(ok.unwrap().len(), 4);
    }

    #[trace("FR-161-AC-15")]
    #[test]
    fn a_pinned_design_trace_reads_only_its_relation_and_lists_it_missing() {
        let pinned = activity(json!({"id":"d","kind":"design-trace","relation":"r"}));
        let covered = json!([{"document":{"path":"spec/FR-1.md"},"state":"covered"}]);
        let uncovered = json!([{"document":{"path":"spec/FR-1.md"},"state":"uncovered"}]);
        let elsewhere = json!([{"document":{"path":"spec/FR-2.md"},"state":"covered"}]);
        let status = |relations: Vec<serde_json::Value>| {
            relation_status(Some(&relations), "spec/FR-1.md", &pinned)
        };
        assert_eq!(
            status(vec![relation("r", &covered), relation("s", &uncovered)]).value,
            "met"
        );
        let unmet = status(vec![relation("r", &uncovered), relation("s", &uncovered)]);
        assert_eq!(
            (unmet.value, unmet.missing),
            ("unmet", vec!["r".to_owned()])
        );
        assert_eq!(
            status(vec![relation("r", &elsewhere), relation("s", &uncovered)]).value,
            "not-available"
        );
        let policy = ClassPolicy {
            class_field: "class".to_owned(),
            classes: BTreeMap::from([(
                "class-3".to_owned(),
                floors(vec![
                    json!({"id":"d","kind":"design-trace","relation":"typo"}),
                ]),
            )]),
            gate: false,
        };
        let join = json!({"relations":[{"name":"r","rows":[]}]});
        let error = check_relations_declared(&policy, &join, &BTreeMap::new()).unwrap_err();
        assert!(
            error.contains("`typo`") && error.contains("design-trace"),
            "{error}"
        );
    }

    #[trace("FR-161-AC-16")]
    #[test]
    fn a_relation_a_module_adds_turns_an_unpinned_design_trace_unmet_naming_it() {
        let unpinned = activity(json!({"id":"d","kind":"design-trace"}));
        let row = |path: &str, state: &str| json!({"document":{"path":path},"state":state});
        let realization = relation(
            "realized-by",
            &json!([row("spec/X.md", "covered"), row("spec/Y.md", "covered")]),
        );
        let without = vec![realization.clone()];
        let with = vec![
            realization,
            relation("soup", &json!([row("spec/X.md", "uncovered")])),
        ];
        let read = |relations: &Vec<serde_json::Value>, document: &str| {
            let status = relation_status(Some(relations), document, &unpinned);
            (status.value, status.missing)
        };
        assert_eq!(read(&without, "spec/X.md"), ("met", vec![]));
        assert_eq!(read(&with, "spec/X.md"), ("unmet", vec!["soup".to_owned()]));
        assert_eq!(read(&with, "spec/Y.md"), read(&without, "spec/Y.md"));
    }

    #[trace("FR-161-AC-17")]
    #[test]
    fn design_trace_soup_and_segregation_entries_report_one_row_each() {
        let policy = ClassPolicy {
            class_field: "class".to_owned(),
            classes: BTreeMap::from([(
                "class-3".to_owned(),
                floors(vec![
                    json!({"id":"trace","kind":"design-trace"}),
                    json!({"id":"soup","kind":"soup-requirements","relation":"S"}),
                    json!({"id":"seg","kind":"segregation","relation":"G"}),
                ]),
            )]),
            gate: false,
        };
        let row = |path: &str, state: &str| json!({"document":{"path":path},"state":state});
        let join = json!({"relations":[
            {"name":"real","rows":[row("spec/FR-1.md", "covered"), row("spec/FR-2.md", "covered")]},
            {"name":"S","rows":[row("spec/FR-1.md", "uncovered"), row("spec/FR-2.md", "covered")]},
            {"name":"G","rows":[row("spec/FR-2.md", "covered"), row("spec/FR-3.md", "uncovered")]}
        ]});
        check_relations_declared(&policy, &join, &BTreeMap::new()).unwrap();
        let matrix = ["spec/FR-1.md", "spec/FR-2.md", "spec/FR-3.md"].map(
            |document| json!({"document":document,"carried":{"class":"class-3"},"criteria":[]}),
        );
        let rows = build_rows(&matrix, &join, &policy, &BTreeMap::new(), &BTreeMap::new()).unwrap();
        let activities = |index: usize| rows.requirements[index]["activities"].as_array().unwrap();
        let statuses = |index| {
            activities(index)
                .iter()
                .map(|row| (row["id"].as_str().unwrap(), row["status"].as_str().unwrap()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            statuses(0),
            [
                ("trace", "unmet"),
                ("soup", "unmet"),
                ("seg", "not-available")
            ]
        );
        assert_eq!(activities(0)[0]["missing"], json!(["S"]));
        assert!(activities(0)[1].get("missing").is_none());
        assert_eq!(
            statuses(1),
            [("trace", "met"), ("soup", "met"), ("seg", "met")]
        );
        assert_eq!(
            statuses(2),
            [
                ("trace", "unmet"),
                ("soup", "not-available"),
                ("seg", "unmet")
            ]
        );
        assert_eq!(activities(2)[0]["missing"], json!(["G"]));
        assert!(activities(2)[2].get("missing").is_none());
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
