// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The retained read-only `quoin evidence audit` adapter.

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_core::protocol::Response;
use quoin_evidence::types::{Binding, ObligationId, RunRecord};
use std::collections::BTreeSet;

use super::{json_arg, repo_arg, required};
use crate::core_runtime::invoke;

pub(super) fn command() -> Command {
    Command::new("audit")
        .about("Audit the evidence store: staleness, vacuity")
        .arg(repo_arg())
        .arg(module_arg())
        .arg(Arg::new("strict").long("strict").action(ArgAction::SetTrue))
        .arg(json_arg())
        .args(policy_args())
}

/// `--module`: the traceability model's roots, shared with `quoin matrix`.
pub(crate) fn module_arg() -> Arg {
    Arg::new("module").long("module").action(ArgAction::Append)
}

/// The auditor's policy inputs, shared with `quoin matrix` so both commands
/// read one flag grammar into one [`assemble`].
pub(crate) fn policy_args() -> [Arg; 4] {
    [
        Arg::new("multiplicity-requires")
            .long("multiplicity-requires")
            .action(ArgAction::Append),
        Arg::new("mutation-floor")
            .long("mutation-floor")
            .action(ArgAction::Append),
        Arg::new("independence-policy").long("independence-policy"),
        // FR-160: a person's file saying what evidence each software safety
        // class needs. Absent, the audit and the matrix read no class at all.
        Arg::new("class-policy").long("class-policy"),
    ]
}

/// An audit, with the two inputs a caller joins it against.
pub(crate) struct Audited {
    /// The `quire.coverage` payload the audit's obligations came from.
    pub(crate) coverage: serde_json::Value,
    /// The binding graph the audit read, as `evidence.audit_inputs` served it.
    pub(crate) bindings: serde_json::Value,
    /// The exact auditor input, used when a consumer projects another method
    /// through the FR-160 class evidence join.
    pub(crate) audit_input: serde_json::Value,
    /// The `auditor.audit` response, whose payload carries `report`.
    pub(crate) response: Response,
    /// The store's whole host input, used by the FR-153 evidence bridge.
    pub(crate) store_inputs: serde_json::Value,
}

/// What [`assemble`] produced.
pub(crate) enum Assembly {
    /// Every step answered.
    Audited(Box<Audited>),
    /// A step declined; its response is the command's answer.
    Declined(Response),
}

/// The auditor's inputs that come from flags.
#[derive(Default)]
pub(crate) struct AuditOptions {
    modules: Vec<String>,
    multiplicity_requires: Vec<String>,
    mutation_floor: Vec<String>,
    independence_policy: Option<String>,
    class_policy: Option<String>,
}

impl AuditOptions {
    /// The options a command with the shared flag grammar was given.
    pub(crate) fn from_arguments(arguments: &ArgMatches) -> Self {
        Self {
            modules: values(arguments, "module"),
            multiplicity_requires: values(arguments, "multiplicity-requires"),
            mutation_floor: values(arguments, "mutation-floor"),
            independence_policy: arguments.get_one::<String>("independence-policy").cloned(),
            class_policy: arguments.get_one::<String>("class-policy").cloned(),
        }
    }

    /// Options naming only the module roots, as a caller without the flag
    /// grammar supplies them.
    pub(crate) fn for_modules(modules: Vec<String>) -> Self {
        Self {
            modules,
            ..Self::default()
        }
    }

    /// The closed module roots supplied to the coverage walk.
    pub(crate) fn module_paths(&self) -> &[String] {
        &self.modules
    }

    pub(crate) fn modules(&self) -> &[String] {
        &self.modules
    }
}

/// Assemble and run the audit: coverage, the store, the catalog, the policy.
///
/// The one assembly `quoin evidence audit` and `quoin matrix` share, so the
/// two commands cannot audit the same working tree differently.
pub(crate) fn assemble(arguments: &ArgMatches, repo: &str) -> Result<Assembly, String> {
    assemble_with(&AuditOptions::from_arguments(arguments), repo)
}

/// [`assemble`] over options already read from the flags.
pub(crate) fn assemble_with(options: &AuditOptions, repo: &str) -> Result<Assembly, String> {
    assemble_with_coverage_operation(options, repo, "quire.coverage")
}

/// Assemble a matrix audit using quire-rs `design_matrix` for its coverage input.
pub(crate) fn assemble_for_matrix_with(
    options: &AuditOptions,
    repo: &str,
) -> Result<Assembly, String> {
    assemble_with_coverage_operation(options, repo, "quire.design_matrix")
}

fn assemble_with_coverage_operation(
    options: &AuditOptions,
    repo: &str,
    coverage_operation: &str,
) -> Result<Assembly, String> {
    let modules = options.modules.clone();
    let coverage_request = &serde_json::json!({ "scope": repo, "modules": modules });
    let coverage = if coverage_operation == "quire.design_matrix" {
        crate::core_runtime::invoke_declining(coverage_operation, coverage_request)?
    } else {
        invoke(coverage_operation, coverage_request)?
    };
    if !coverage.outcome.carries_payload() {
        return Ok(Assembly::Declined(coverage));
    }
    let obligations = field(&coverage.payload, "obligations")?;
    let policy = policy(options.independence_policy.as_deref(), &obligations)?;
    let store = invoke(
        "evidence.audit_inputs",
        &serde_json::json!({ "repo": repo, "independence_policy": policy }),
    )?;
    if !store.outcome.carries_payload() {
        return Ok(Assembly::Declined(store));
    }
    let catalog_request = if modules.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::json!({ "roots": modules })
    };
    let catalog = invoke("catalog.methods", &catalog_request)?;
    if !catalog.outcome.carries_payload() {
        return Ok(Assembly::Declined(catalog));
    }
    let mut bindings = field(&store.payload, "bindings")?;
    let runs = field(&store.payload, "runs")?;
    if coverage_operation == "quire.design_matrix" {
        bindings = trace_bindings(&bindings, &runs, &obligations)?;
    }
    let scans = field(&store.payload, "scans")?;
    let mut input = serde_json::json!({
        "obligations": obligations,
        "bindings": bindings,
        "runs": runs,
        "scans": scans,
        "injections": field(&store.payload, "injections")?,
        "mockInspectionSuites": field(&store.payload, "mock_inspection_suites")?,
        "vacuousScanSuites": field(&store.payload, "vacuous_scan_suites")?,
        "independence": field(&store.payload, "independence")?,
        "catalog": catalog.payload,
        "multiplicityRequires": options.multiplicity_requires,
        "mutationFloor": mutation_floor(&options.mutation_floor)?,
        "independencePolicy": policy,
    });
    if let Some(path) = options.class_policy.as_deref() {
        add_class_inputs(&mut input, path, &coverage.payload)?;
    }
    let audit_input = input.clone();
    let request = serde_json::json!({ "input": input });
    let audited = invoke("auditor.audit", &request)?;
    if !audited.outcome.carries_payload() {
        return Ok(Assembly::Declined(audited));
    }
    Ok(Assembly::Audited(Box::new(Audited {
        coverage: coverage.payload,
        bindings,
        audit_input,
        response: audited,
        store_inputs: store.payload,
    })))
}

/// Add in-memory matrix bindings for criterion ids carried by retained runs.
/// The evidence store remains append-only and `bindings.json` is not changed.
fn trace_bindings(
    bindings: &serde_json::Value,
    runs: &serde_json::Value,
    obligations: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut bindings: Vec<Binding> = serde_json::from_value(bindings.clone())
        .map_err(|error| format!("evidence.audit_inputs bindings are malformed: {error}"))?;
    let runs: Vec<RunRecord> = serde_json::from_value(runs.clone())
        .map_err(|error| format!("evidence.audit_inputs runs are malformed: {error}"))?;
    let mut known = BTreeSet::new();
    collect_ids(obligations, &mut known);

    for run in runs {
        for entry in run.entries {
            for obligation in entry.trace_ids.unwrap_or_default() {
                if !known.contains(obligation.as_str()) {
                    continue;
                }
                let binding = bindings.iter_mut().find(|binding| {
                    binding.obligation.as_str() == obligation && binding.suite == run.suite
                });
                if let Some(binding) = binding {
                    if !binding.symbols.contains(&entry.symbol) {
                        binding.symbols.push(entry.symbol.clone());
                        binding.symbols.sort();
                    }
                } else {
                    bindings.push(Binding {
                        obligation: ObligationId::new(obligation),
                        suite: run.suite.clone(),
                        symbols: vec![entry.symbol.clone()],
                        lineage: None,
                    });
                }
            }
        }
    }
    bindings.sort_by(|left, right| {
        left.obligation
            .cmp(&right.obligation)
            .then_with(|| left.suite.cmp(&right.suite))
    });
    serde_json::to_value(bindings)
        .map_err(|error| format!("cannot serialize in-memory trace bindings: {error}"))
}

fn collect_ids(value: &serde_json::Value, ids: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                collect_ids(value, ids);
            }
        }
        serde_json::Value::Object(object) => {
            if let Some(id) = object.get("id").and_then(serde_json::Value::as_str) {
                ids.insert(id.to_owned());
            }
            for value in object.values() {
                collect_ids(value, ids);
            }
        }
        _ => {}
    }
}

pub(super) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let assembled = match assemble(arguments, &repo)? {
        Assembly::Audited(assembled) => assembled,
        Assembly::Declined(response) => return Ok(response),
    };
    let class_gate = class_gate_set(&assembled.audit_input);
    let audited = assembled.response;
    let report = audited
        .payload
        .get("report")
        .ok_or_else(|| "auditor.audit did not return report".to_owned())?;
    let reported = report
        .get("findings")
        .ok_or_else(|| "auditor.audit did not return findings".to_owned())?;
    let unevaluated = report
        .get("unevaluated")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let findings = reported.as_array().map_or(0, Vec::len);
    let rendered = if arguments.get_flag("json") {
        let mut output = serde_json::json!({
            "findings": reported,
            "healthy": report.get("healthy"),
            "unevaluated": report.get("unevaluated"),
            "independence": audited.payload.get("independence"),
        });
        // Present only under `--class-policy` (FR-160-AC-1).
        if let (Some(object), Some(class)) = (output.as_object_mut(), report.get("classReport")) {
            object.insert("classReport".to_owned(), class.clone());
            object.insert(
                "unclassified".to_owned(),
                serde_json::json!(unassigned(class)),
            );
            object.insert(
                "classIndependence".to_owned(),
                report.get("classIndependence").cloned().unwrap_or_default(),
            );
        }
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    } else {
        render_text(report, reported)?
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: audited.diagnostics,
        outcome: if (arguments.get_flag("strict") && (findings > 0 || unevaluated > 0))
            || (class_gate && class_findings(reported) > 0)
        {
            Outcome::Partial
        } else {
            audited.outcome
        },
    })
}

/// Whether the class policy the audit ran under asks for class findings to
/// gate (FR-160). A policy that does not say so reports and gates nothing.
pub(crate) fn class_gate_set(audit_input: &serde_json::Value) -> bool {
    audit_input
        .get("classPolicy")
        .and_then(|policy| policy.get("gate"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

/// How many findings a class policy caused: those naming a `class`.
fn class_findings(findings: &serde_json::Value) -> usize {
    findings
        .as_array()
        .into_iter()
        .flatten()
        .filter(|finding| finding.get("class").is_some())
        .count()
}

fn policy(
    path: Option<&str>,
    obligations: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let Some(path) = path else {
        return Ok(serde_json::Value::Null);
    };
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let known_obligations = obligations
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    let parsed = invoke(
        "evidence.parse_policy",
        &serde_json::json!({ "text": text, "known_obligations": known_obligations }),
    )?;
    if !parsed.outcome.carries_payload() {
        return Err("independence policy was refused".to_owned());
    }
    field(&parsed.payload, "policy")
}

/// Hand the auditor the class policy and the three coverage facts it joins it
/// to (FR-160). The file is read as written: validating it against the report's
/// vocabulary and the catalog is the auditor's refusal to make, not this
/// adapter's.
fn add_class_inputs(
    input: &mut serde_json::Value,
    path: &str,
    coverage: &serde_json::Value,
) -> Result<(), String> {
    let policy = load_class_policy(path)?;
    let members = [
        ("classPolicy", Some(&policy)),
        ("carriedVocabulary", coverage.get("carried_vocabulary")),
        ("coverageMatrix", coverage.get("coverage_matrix")),
    ];
    if let Some(object) = input.as_object_mut() {
        for (name, value) in members {
            if let Some(value) = value {
                object.insert(name.to_owned(), value.clone());
            }
        }
    }
    Ok(())
}

/// Read the same strict YAML/JSON class policy used by the auditor adapter.
pub(crate) fn load_class_policy(path: &str) -> Result<serde_json::Value, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    quoin_yaml::from_str_without_aliases(&text)
        .map_err(|error| format!("{path} is not valid YAML or JSON: {error}"))
}

fn mutation_floor(entries: &[String]) -> Result<serde_json::Value, String> {
    if entries.is_empty() {
        return Ok(serde_json::Value::Null);
    }
    let mut floor = serde_json::Map::new();
    for entry in entries {
        let Some((criticality, raw)) = entry.split_once('=') else {
            return Err(format!(
                "--mutation-floor expects <criticality>=<score>, got '{entry}'. Example: --mutation-floor P0=0.8"
            ));
        };
        let score = raw.parse::<f64>().map_err(|_| format!("--mutation-floor expects <criticality>=<score>, got '{entry}'. Example: --mutation-floor P0=0.8"))?;
        if criticality.is_empty() || !(0.0..=1.0).contains(&score) {
            return Err(format!(
                "--mutation-floor score must be a ratio in [0, 1], got '{raw}'. 80% is 0.8, not 80."
            ));
        }
        floor.insert(criticality.to_owned(), serde_json::json!(score));
    }
    Ok(serde_json::Value::Object(floor))
}

fn values(arguments: &ArgMatches, name: &str) -> Vec<String> {
    arguments
        .get_many::<String>(name)
        .map_or_else(Vec::new, |values| values.cloned().collect())
}

fn field(value: &serde_json::Value, name: &str) -> Result<serde_json::Value, String> {
    value
        .get(name)
        .cloned()
        .ok_or_else(|| format!("core response did not return {name}"))
}

fn render_text(report: &serde_json::Value, reported: &serde_json::Value) -> Result<String, String> {
    let mut lines = Vec::new();
    let findings = reported
        .as_array()
        .ok_or_else(|| "auditor.audit did not return findings".to_owned())?;
    let unevaluated = report
        .get("unevaluated")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "auditor.audit did not return unevaluated checks".to_owned())?;
    if findings.is_empty() && unevaluated.is_empty() {
        lines.push(format!(
            "{} obligation(s) with healthy evidence; nothing to report",
            report
                .get("healthy")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len)
        ));
    } else {
        for finding in findings {
            lines.push(format!(
                "[{}] {}: {}",
                finding
                    .get("severity")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                finding
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                finding
                    .get("summary")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            ));
        }
        for check in unevaluated {
            lines.push(format!(
                "[not-evaluated] {}: {}: {}",
                check
                    .get("check")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                check
                    .get("obligation")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                check
                    .get("reason")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            ));
        }
        lines.push(String::new());
        lines.push(format!(
            "{} finding(s), {} check(s) not evaluated, {} healthy",
            findings.len(),
            unevaluated.len(),
            report
                .get("healthy")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len),
        ));
    }
    if let Some(class) = report.get("classReport") {
        lines.push(String::new());
        lines.push("class".to_owned());
        lines.extend(class_rows(class));
        lines.push(format!("{} obligation(s) unclassified", unassigned(class)));
    }
    Ok(lines.join("\n"))
}

/// The ids whose requirement states no class.
fn unassigned_ids(class: &serde_json::Value) -> impl Iterator<Item = &str> {
    class
        .get("unassigned")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
}

/// How many obligations state no class.
fn unassigned(class: &serde_json::Value) -> usize {
    unassigned_ids(class).count()
}

/// The class column, one row per obligation in id order: the value as written,
/// or `unassigned` (FR-160-AC-6, FR-160-AC-11).
fn class_rows(class: &serde_json::Value) -> Vec<String> {
    let mut rows: Vec<(&str, &str)> = class
        .get("classes")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(id, value)| Some((id.as_str(), value.as_str()?)))
        .collect();
    rows.extend(unassigned_ids(class).map(|id| (id, "unassigned")));
    rows.sort_unstable();
    rows.into_iter()
        .map(|(id, value)| format!("  {id}  {value}"))
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use ix_trace_rs::trace;

    use super::{command, trace_bindings, values};

    /// Repeatable `--module` arguments reach the producer one by one and in the
    /// order supplied, so the catalog loader and native coverage read one array.
    #[trace("FR-032-AC-12")]
    #[test]
    fn repeatable_module_arguments_keep_their_supplied_order() {
        let matches = command()
            .try_get_matches_from([
                "audit", "--module", "/zeta", "--module", "/alpha", "--module", "/mid",
            ])
            .expect("repeatable --module parses");
        assert_eq!(values(&matches, "module"), ["/zeta", "/alpha", "/mid"]);

        let none = command()
            .try_get_matches_from(["audit"])
            .expect("--module is optional");
        assert_eq!(values(&none, "module").len(), 0);
    }

    #[trace("FR-031-AC-1")]
    #[test]
    fn matrix_bindings_are_derived_in_memory_from_known_run_trace_ids() {
        let bindings = serde_json::json!([]);
        let runs = serde_json::json!([{
            "schemaVersion": 1,
            "suite": "SUITE-004",
            "tool": "cargo test 1.0",
            "timestamp": "2026-11-24T00:00:00Z",
            "entries": [{
                "symbol": "cli_export::exports",
                "outcome": "pass",
                "traceIds": ["FR-031-AC-1", "FR-999-AC-1"]
            }]
        }]);
        let obligations = serde_json::json!([{"id": "FR-031-AC-1"}]);

        assert_eq!(
            trace_bindings(&bindings, &runs, &obligations).expect("trace bindings"),
            serde_json::json!([{
                "obligation": "FR-031-AC-1",
                "suite": "SUITE-004",
                "symbols": ["cli_export::exports"]
            }])
        );
    }
}
