// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Read-only formalization state across requirement and criterion ids (VER-104).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;
use quoin_quire::evidence::{Outcome as AnalysisOutcome, Query};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const PROTOCOL: &str = "quoin.formalization-coverage/v1";
const REQUIREMENT_TYPES: [&str; 3] = ["FR", "NFR", "StR"];

pub(crate) fn command() -> Command {
    Command::new("coverage")
        .about("Report formalization state for requirements and criteria")
        .arg(Arg::new("repo").long("repo").default_value("."))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    symbol: String,
    state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    id: String,
    kind: String,
    document: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    criticality: Option<String>,
    state: String,
    declarations: Vec<Declaration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    analysis_disposition: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Counts {
    none: usize,
    draft: usize,
    approved: usize,
    analysed: usize,
}

impl Counts {
    fn increment(&mut self, state: &str) -> Result<(), String> {
        match state {
            "none" => self.none += 1,
            "draft" => self.draft += 1,
            "approved" => self.approved += 1,
            "analysed" => self.analysed += 1,
            _ => return Err(format!("unknown formalization state {state}")),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Totals {
    by_document: BTreeMap<String, Counts>,
    by_criticality: BTreeMap<String, Counts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    protocol: String,
    items: Vec<Item>,
    totals: Totals,
}

#[allow(
    clippy::too_many_lines,
    reason = "keep the coverage input joins auditable"
)]
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = arguments
        .get_one::<String>("repo")
        .ok_or_else(|| "--repo is unavailable".to_owned())?
        .to_owned();
    let repo = std::fs::canonicalize(&repo)
        .map_err(|error| format!("repository is unavailable: {error}"))?;
    let repo_text = repo
        .to_str()
        .ok_or_else(|| "--repo is not valid UTF-8".to_owned())?;

    let status = crate::formal_rules::status_report(arguments)?;
    let status_rules = status
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "formal-rules status omitted rules".to_owned())?;
    let mut approvals = BTreeMap::new();
    for rule in status_rules {
        let symbol = required_text(rule, "symbol")?;
        let state = required_text(rule, "state")?;
        let reason = rule
            .get("reason")
            .and_then(Value::as_str)
            .map(str::to_owned);
        approvals.insert(
            symbol.to_owned(),
            Declaration {
                symbol: symbol.to_owned(),
                state: state.to_owned(),
                reason,
            },
        );
    }

    let modules = values(arguments, "module");
    let coverage = crate::core_runtime::invoke(
        "quire.coverage",
        &json!({"scope": repo_text, "modules": modules}),
    )?;
    if !coverage.outcome.carries_payload() {
        return Ok(coverage);
    }
    let obligation_value = coverage
        .payload
        .get("obligations")
        .cloned()
        .ok_or_else(|| "quire.coverage omitted obligations".to_owned())?;
    let obligations: Vec<quoin_quire_types::Obligation> =
        serde_json::from_value(obligation_value.clone())
            .map_err(|error| format!("quire.coverage obligations are malformed: {error}"))?;
    let store = crate::core_runtime::invoke(
        "evidence.audit_inputs",
        &json!({"repo": repo_text, "independence_policy": Value::Null}),
    )?;
    if !store.outcome.carries_payload() {
        return Ok(store);
    }
    let run_observations =
        crate::run_observation::read_run_observations(&values(arguments, "run-observation"))?;
    let analysis = crate::analysis_evidence::compute(
        repo_text,
        &modules,
        &store.payload,
        &obligations,
        &run_observations,
    )?;

    let frontmatter = crate::completeness::read_frontmatter(&repo.join("spec"))?;
    if !frontmatter.outcome.carries_payload() {
        return Ok(frontmatter);
    }

    let mut source_items = Vec::new();
    let documents = frontmatter
        .payload
        .get("documents")
        .and_then(Value::as_array)
        .ok_or_else(|| "completeness.read_frontmatter omitted documents".to_owned())?;
    for document in documents {
        let metadata = document
            .get("frontmatter")
            .ok_or_else(|| "frontmatter reader omitted a document header".to_owned())?;
        let Some(id) = metadata.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(kind) = metadata.get("type").and_then(Value::as_str) else {
            continue;
        };
        if REQUIREMENT_TYPES.contains(&kind) {
            let path = required_text(document, "path")?;
            let path = relative_spec_path(&repo, path)?;
            let criticality = metadata
                .get("criticality")
                .filter(|value| !value.is_null())
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| format!("{path}: criticality must be a string when present"))
                })
                .transpose()?;
            source_items.push(SourceItem {
                id: id.to_owned(),
                kind: "requirement".to_owned(),
                document: path,
                criticality,
            });
        }
    }
    let obligation_rows = obligation_value
        .as_array()
        .ok_or_else(|| "quire.coverage obligations must be an array".to_owned())?;
    for row in obligation_rows {
        let id = required_text(row, "id")?;
        if !is_criterion_id(id) {
            continue;
        }
        let document = relative_spec_path(&repo, required_text(row, "document")?)?;
        let criticality =
            row.get("criticality")
                .filter(|value| !value.is_null())
                .map(|value| {
                    value.as_str().map(str::to_owned).ok_or_else(|| {
                        format!("{document}: criterion criticality must be a string")
                    })
                })
                .transpose()?;
        source_items.push(SourceItem {
            id: id.to_owned(),
            kind: "criterion".to_owned(),
            document,
            criticality,
        });
    }

    let rule_rows = status_rules;
    let report = build_report(source_items, rule_rows, &approvals, &analysis)?;
    let rendered = if arguments.get_flag("json") {
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    } else {
        report
            .items
            .iter()
            .map(|item| {
                let disposition = item
                    .analysis_disposition
                    .as_deref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default();
                format!("{} {} {}{disposition}", item.state, item.id, item.document)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(Response::ok(json!({"rendered": rendered})))
}

#[derive(Debug, Clone)]
struct SourceItem {
    id: String,
    kind: String,
    document: String,
    criticality: Option<String>,
}

fn build_report(
    source_items: Vec<SourceItem>,
    rule_rows: &[Value],
    approvals: &BTreeMap<String, Declaration>,
    analysis: &AnalysisOutcome,
) -> Result<Report, String> {
    let mut by_id: BTreeMap<String, Vec<Declaration>> = BTreeMap::new();
    for rule in rule_rows {
        let symbol = required_text(rule, "symbol")?;
        let linked = rule
            .get("formalizes")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("formal rule {symbol} omitted formalizes"))?;
        let Some(declaration) = approvals.get(symbol) else {
            return Err(format!("formal rule {symbol} has no approval status"));
        };
        for id in linked.iter().filter_map(Value::as_str) {
            by_id
                .entry(id.to_owned())
                .or_default()
                .push(declaration.clone());
        }
    }
    let mut seen = BTreeSet::new();
    let mut items = Vec::with_capacity(source_items.len());
    for source in source_items {
        if !seen.insert((source.document.clone(), source.id.clone())) {
            return Err(format!(
                "formalization inventory contains duplicate id {} in {}",
                source.id, source.document
            ));
        }
        let mut declarations = by_id.remove(&source.id).unwrap_or_default();
        declarations.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        declarations.dedup_by(|a, b| a.symbol == b.symbol);
        let analysis_disposition = if declarations.is_empty()
            || declarations
                .iter()
                .any(|declaration| declaration.state == "draft")
        {
            None
        } else {
            Some(disposition_for(&source.id, analysis))
        };
        let state = derive_state(&declarations, analysis_disposition.as_deref());
        items.push(Item {
            id: source.id,
            kind: source.kind,
            document: source.document,
            criticality: source.criticality,
            state: state.to_owned(),
            declarations,
            analysis_disposition,
        });
    }
    items.sort_by(|a, b| (&a.document, &a.id).cmp(&(&b.document, &b.id)));
    let mut by_document = BTreeMap::<String, Counts>::new();
    let mut by_criticality = BTreeMap::<String, Counts>::new();
    for item in &items {
        by_document
            .entry(item.document.clone())
            .or_default()
            .increment(&item.state)?;
        by_criticality
            .entry(
                item.criticality
                    .clone()
                    .unwrap_or_else(|| "unassigned".to_owned()),
            )
            .or_default()
            .increment(&item.state)?;
    }
    Ok(Report {
        protocol: PROTOCOL.to_owned(),
        items,
        totals: Totals {
            by_document,
            by_criticality,
        },
    })
}

fn derive_state(declarations: &[Declaration], disposition: Option<&str>) -> &'static str {
    if declarations.is_empty() {
        "none"
    } else if declarations
        .iter()
        .any(|declaration| declaration.state == "draft")
    {
        "draft"
    } else if disposition == Some("qualified-record") {
        "analysed"
    } else {
        "approved"
    }
}

fn disposition_for(id: &str, analysis: &AnalysisOutcome) -> String {
    if !analysis.trace(Query::Obligation(id)).is_empty() {
        return "qualified-record".to_owned();
    }
    let results = analysis
        .report
        .evidence_results
        .as_deref()
        .unwrap_or_default();
    let cause = results
        .iter()
        .filter(|result| result.requested_target == id)
        .map(|result| result.cause.as_str())
        .min_by_key(|cause| disposition_rank(cause));
    match cause {
        Some("failed") => "failed".to_owned(),
        Some("errored") => "errored".to_owned(),
        Some("absent" | "skipped") => "absent".to_owned(),
        Some("stale" | "wrong-revision") => "stale".to_owned(),
        Some("unevaluated" | "inventory-incomplete") | None => "unevaluated".to_owned(),
        Some(_) => "qualification-rejected".to_owned(),
    }
}

fn disposition_rank(cause: &str) -> u8 {
    match cause {
        "failed" => 0,
        "errored" => 1,
        "stale" | "wrong-revision" => 2,
        "absent" | "skipped" => 3,
        "unevaluated" | "inventory-incomplete" => 4,
        _ => 5,
    }
}

fn is_criterion_id(id: &str) -> bool {
    id.split_once("-AC-").is_some_and(|(_, number)| {
        !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn relative_spec_path(repo: &Path, path: &str) -> Result<String, String> {
    let path = Path::new(path);
    let spec = repo.join("spec");
    let relative = if path.is_absolute() {
        path.strip_prefix(&spec)
            .map_err(|_| format!("document path '{}' is outside --repo/spec", path.display()))?
    } else if let Ok(relative) = path.strip_prefix("spec") {
        relative
    } else {
        path
    };
    relative
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| "document path is not valid UTF-8".to_owned())
}

fn required_text<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("record omitted string field {field}"))
}

fn values(arguments: &ArgMatches, name: &str) -> Vec<String> {
    arguments
        .get_many::<String>(name)
        .map_or_else(Vec::new, |values| values.cloned().collect())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test failures should name their fixture"
)]
mod tests {
    use std::path::Path;

    use super::{Counts, Declaration, derive_state};
    use ix_trace_rs::trace;

    #[trace("FR-204-AC-2", "FR-204-AC-3")]
    #[test]
    fn draft_dominates_approved_declarations_and_qualified_analysis() {
        let declarations = [
            Declaration {
                symbol: "qsl:one::P1".into(),
                state: "approved".into(),
                reason: None,
            },
            Declaration {
                symbol: "qsl:two::P2".into(),
                state: "draft".into(),
                reason: Some("changed_since_approval".into()),
            },
        ];
        assert_eq!(
            derive_state(&declarations, Some("qualified-record")),
            "draft"
        );
        assert_eq!(
            declarations[1].reason.as_deref(),
            Some("changed_since_approval")
        );
    }

    #[trace("FR-204-AC-1", "FR-204-AC-4", "FR-204-AC-5")]
    #[test]
    fn all_four_states_and_empty_analysis_disposition_are_distinct() {
        assert_eq!(derive_state(&[], None), "none");
        let approved = [Declaration {
            symbol: "qsl:a::P1".into(),
            state: "approved".into(),
            reason: None,
        }];
        assert_eq!(derive_state(&approved, Some("unevaluated")), "approved");
        assert_eq!(
            derive_state(&approved, Some("qualified-record")),
            "analysed"
        );
        let draft = [Declaration {
            symbol: "qsl:a::P1".into(),
            state: "draft".into(),
            reason: Some("no_approval".into()),
        }];
        assert_eq!(derive_state(&draft, None), "draft");
    }

    #[trace("FR-204-AC-6")]
    #[test]
    fn requirement_and_criterion_states_are_derived_independently() {
        let approved = [Declaration {
            symbol: "qsl:a::P1".into(),
            state: "approved".into(),
            reason: None,
        }];
        assert_eq!(derive_state(&approved, Some("unevaluated")), "approved");
        assert_eq!(derive_state(&[], None), "none");
    }

    #[trace("FR-204-AC-7", "FR-204-AC-8")]
    #[test]
    fn totals_count_every_state_without_scores() {
        let mut counts = Counts::default();
        for state in ["none", "draft", "approved", "analysed"] {
            counts.increment(state).expect("known state");
        }
        assert_eq!(
            counts,
            Counts {
                none: 1,
                draft: 1,
                approved: 1,
                analysed: 1
            }
        );
    }

    #[trace("FR-204-AC-7", "FR-204-AC-8")]
    #[test]
    fn coverage_json_obeys_the_shipped_closed_schema() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/formalization-coverage-v1.schema.json"
        ))
        .expect("the shipped schema parses");
        let validator = quoin_jsonschema::validator::SchemaValidator::compile_vendored(
            Path::new("formalization-coverage-v1"),
            &schema,
            &[],
        )
        .expect("the shipped schema compiles");
        let report = serde_json::json!({
            "protocol": "quoin.formalization-coverage/v1",
            "items": [{
                "id": "FR-204-AC-1",
                "kind": "criterion",
                "document": "functional/FR-204-formalization-coverage-state.md",
                "state": "approved",
                "declarations": [{"symbol": "qsl:rules.native::P1", "state": "approved"}],
                "analysis_disposition": "unevaluated"
            }],
            "totals": {
                "by_document": {"functional/FR-204-formalization-coverage-state.md": {"none": 0, "draft": 0, "approved": 1, "analysed": 0}},
                "by_criticality": {"unassigned": {"none": 0, "draft": 0, "approved": 1, "analysed": 0}}
            }
        });
        assert!(
            validator.is_valid(&report),
            "{:?}",
            validator.errors(&report)
        );
        let mut with_unknown = report;
        with_unknown["items"][0]["score"] = serde_json::json!(100);
        assert!(!validator.is_valid(&with_unknown));
    }
}
