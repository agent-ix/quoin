// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for the retained read-only `quoin assurance` view.

use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::completeness::read_frontmatter;
use crate::core_runtime::{Failure, invoke};

pub(crate) fn command() -> Command {
    Command::new("assurance")
        .about("Render the assurance case: claim → argument → evidence, from the store")
        .arg(Arg::new("repo").long("repo").default_value("."))
        .arg(Arg::new("module").long("module"))
        .arg(
            Arg::new("claim-type")
                .long("claim-type")
                .action(ArgAction::Append),
        )
        .arg(Arg::new("independence-policy").long("independence-policy"))
        .arg(Arg::new("argument").long("argument"))
        .arg(Arg::new("decisions").long("decisions"))
        .arg(Arg::new("discharge").long("discharge"))
        .arg(Arg::new("evidence").long("evidence"))
        .arg(
            Arg::new("formalization-records")
                .long("formalization-records")
                .value_name("FILE")
                .help("Consume a top-level array of Contract IR FR-071 records"),
        )
        .arg(
            Arg::new("run-observation")
                .long("run-observation")
                .action(ArgAction::Append)
                .num_args(1)
                .value_name("FILE"),
        )
        .arg(Arg::new("as-of").long("as-of"))
        .args(crate::formal_rules::trust_arguments())
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
        .arg(
            Arg::new("gsn")
                .long("gsn")
                .help("Export the case as a validated GSN model.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("gsn-diagram")
                .long("gsn-diagram")
                .help("Render the case as a Mermaid GSN diagram.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("sacm")
                .long("sacm")
                .help("Export the validated case as SACM 2.3 XMI.")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["json", "gsn", "gsn-diagram"]),
        )
        .arg(
            Arg::new("sacm-report")
                .long("sacm-report")
                .value_name("PATH")
                .help("Write the unmapped and unreachable items from the SACM export as JSON.")
                .requires("sacm")
                .conflicts_with_all(["json", "gsn", "gsn-diagram"]),
        )
        .arg(
            Arg::new("validation-report")
                .long("validation-report")
                .value_name("FILE")
                .help("Use a retained TVK validation-report JSON file for GSN tool-confidence source facts"),
        )
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_flag("gsn") && arguments.get_flag("json") {
        return Err("--gsn cannot be combined with --json".to_owned());
    }
    if arguments.get_flag("gsn-diagram")
        && (arguments.get_flag("json") || arguments.get_flag("gsn"))
    {
        return Err("--gsn-diagram cannot be combined with --json or --gsn".to_owned());
    }
    if arguments.get_one::<String>("argument").is_some() {
        if arguments.get_one::<String>("validation-report").is_some() {
            return Err("--validation-report applies only to derived GSN assurance".to_owned());
        }
        if arguments
            .get_one::<String>("formalization-records")
            .is_some()
        {
            return Err("--formalization-records applies only to derived assurance".to_owned());
        }
        return authored(arguments);
    }
    derived(arguments)
}

fn authored(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_many::<String>("run-observation").is_some() {
        return Err("--run-observation applies only to derived assurance".to_owned());
    }
    if arguments.get_many::<String>("claim-type").is_some() {
        return Err("--argument cannot be combined with --claim-type".to_owned());
    }
    let repo = required(arguments, "repo")?;
    let argument_id = required(arguments, "argument")?;
    let decisions_path = required(arguments, "decisions")
        .map_err(|_| "--argument requires --decisions and --as-of".to_owned())?;
    let as_of = required(arguments, "as-of")
        .map_err(|_| "--argument requires --decisions and --as-of".to_owned())?;
    let frontmatter = read_frontmatter(std::path::Path::new(&format!("{repo}/spec")))?;
    if !frontmatter.outcome.carries_payload() {
        return Ok(frontmatter);
    }
    let documents = field(&frontmatter.payload, "documents")?;
    let matches = documents
        .as_array()
        .ok_or_else(|| "completeness.read_frontmatter did not return documents".to_owned())?
        .iter()
        .filter(|document| {
            document
                .get("frontmatter")
                .and_then(|frontmatter| frontmatter.get("id"))
                .and_then(serde_json::Value::as_str)
                == Some(argument_id.as_str())
        })
        .collect::<Vec<_>>();
    let argument = match matches.as_slice() {
        [document] => field(document, "frontmatter")?,
        [] => {
            return Err(format!(
                "AssuranceArgument {argument_id} was not found under {repo}/spec"
            ));
        }
        _ => {
            return Err(format!(
                "AssuranceArgument {argument_id} is declared more than once"
            ));
        }
    };
    let decisions = json_file(&decisions_path, "sufficiency decisions")?;
    if !decisions.is_array() {
        return Err("sufficiency decisions must be a JSON array".to_owned());
    }
    let discharge = arguments
        .get_one::<String>("discharge")
        .map(|path| json_file(path, "discharge report"))
        .transpose()?;
    // PLAT-966: without an index every cited reference reads as unresolved,
    // so the top claim can only be `supported` when the caller supplies one.
    let evidence = match arguments.get_one::<String>("evidence") {
        Some(path) => {
            let evidence = json_file(path, "evidence index entries")?;
            if !evidence.is_array() {
                return Err("evidence index must be a JSON array".to_owned());
            }
            evidence
        }
        None => serde_json::json!([]),
    };
    let view = crate::core_runtime::invoke_verifying(
        "assurance.build_authored_argument",
        &serde_json::json!({ "argument": argument, "decisions": decisions, "asOf": as_of, "discharge": discharge, "evidence": evidence, "repo": repo, "approval": crate::discharge::approval_inputs(arguments)? }),
    )?;
    if !view.outcome.carries_payload() {
        return Ok(view);
    }
    if exports_gsn_model(arguments) {
        let model = quoin_assurance::gsn::from_authored(&view.payload)?;
        return render_case_export(&model, Path::new(&repo), arguments);
    }
    present(
        "assurance.render_authored_argument",
        &view.payload,
        arguments.get_flag("json"),
    )
}

fn derived(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let signature_verifier = crate::analysis_evidence::signature_verifier(arguments)?;
    let run_observations = read_run_observations(arguments)?;
    let modules = values(arguments, "module");
    let formalization_path = arguments.get_one::<String>("formalization-records");
    let mut coverage_request = serde_json::json!({ "scope": repo, "modules": modules });
    if formalization_path.is_some() {
        coverage_request
            .as_object_mut()
            .ok_or_else(|| "coverage request must be an object".to_owned())?
            .insert(
                "include_formalization_identities".to_owned(),
                serde_json::Value::Bool(true),
            );
    }
    let coverage = invoke("quire.coverage", &coverage_request)?;
    if !coverage.outcome.carries_payload() {
        return Ok(coverage);
    }
    let obligations = field(&coverage.payload, "obligations")?;
    let formalization_records = match read_formalization_records(
        formalization_path,
        &repo,
        &modules,
        &obligations,
        &coverage.payload,
    ) {
        Ok(records) => records,
        Err(failure) => return failure.settle(),
    };
    let policy = policy(arguments, &obligations)?;
    let store = invoke(
        "evidence.audit_inputs",
        &serde_json::json!({ "repo": repo, "independence_policy": policy }),
    )?;
    if !store.outcome.carries_payload() {
        return Ok(store);
    }
    let catalog = invoke(
        "catalog.methods",
        &if modules.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::json!({ "roots": modules })
        },
    )?;
    if !catalog.outcome.carries_payload() {
        return Ok(catalog);
    }
    let audit = run_audit(&obligations, &store.payload, &catalog.payload, &policy)?;
    if !audit.outcome.carries_payload() {
        return Ok(audit);
    }
    let frontmatter = read_frontmatter(std::path::Path::new(&format!("{repo}/spec")))?;
    if !frontmatter.outcome.carries_payload() {
        return Ok(frontmatter);
    }
    let trust = invoke(
        "evidence.trust_assessments",
        &serde_json::json!({ "repo": repo }),
    )?;
    if !trust.outcome.carries_payload() {
        return Ok(trust);
    }
    let case_request = match build_case_request(&CaseRequestInputs {
        arguments,
        repo: &repo,
        signature_verifier: signature_verifier.as_ref(),
        modules: &modules,
        run_observations: &run_observations,
        coverage: &coverage.payload,
        formalization_records: &formalization_records,
        store: &store.payload,
        audit: &audit.payload,
        frontmatter: &frontmatter.payload,
        trust: &trust.payload,
    }) {
        Ok(request) => request,
        Err(failure) => return failure.settle(),
    };
    let case = invoke("assurance.build_case", &case_request)?;
    if !case.outcome.carries_payload() {
        return Ok(case);
    }
    if exports_gsn_model(arguments) {
        let assurance: quoin_assurance::AssuranceCase =
            serde_json::from_value(case.payload.clone()).map_err(|error| {
                format!("assurance.build_case returned an invalid case: {error}")
            })?;
        let validation_report = arguments
            .get_one::<String>("validation-report")
            .map(|path| json_file(path, "TVK validation report"))
            .transpose()?;
        let run_records = field(&store.payload, "runs")?
            .as_array()
            .cloned()
            .unwrap_or_default();
        let model = quoin_assurance::gsn::from_case_with_validation_report(
            &assurance,
            &run_records,
            validation_report.as_ref(),
        )?;
        return render_case_export(&model, Path::new(&repo), arguments);
    }
    present(
        "assurance.render_case",
        &case.payload,
        arguments.get_flag("json"),
    )
}

fn exports_gsn_model(arguments: &ArgMatches) -> bool {
    ["gsn", "gsn-diagram", "sacm"]
        .iter()
        .any(|flag| arguments.get_flag(flag))
}

fn render_case_export(
    model: &quoin_assurance::GsnCase,
    repo: &Path,
    arguments: &ArgMatches,
) -> Result<Response, String> {
    if arguments.get_flag("sacm") {
        return render_sacm_response(
            model,
            repo,
            arguments.get_one::<String>("sacm-report").map(Path::new),
        );
    }
    render_gsn_response(model, arguments.get_flag("gsn-diagram"))
}

fn render_sacm_response(
    model: &quoin_assurance::GsnCase,
    repo: &Path,
    report_path: Option<&Path>,
) -> Result<Response, String> {
    use quoin_evidence::source::DiskEvidence;
    use quoin_store::store::{Content, StorePath, publish};

    let export = quoin_assurance::sacm::export(model, &DiskEvidence::new(repo))
        .map_err(|error| error.to_string())?;
    let rendered = String::from_utf8(export.xmi).map_err(|error| error.to_string())?;
    let diagnostics: Vec<_> = export.report.diagnostics().collect();
    if let Some(path) = report_path {
        let report =
            serde_json::to_vec_pretty(&export.report).map_err(|error| error.to_string())?;
        let destination = Path::new(".").join(path);
        publish(&StorePath::Named(destination), Content::Artifact(&report)).map_err(|error| {
            format!("could not write SACM report at {}: {error}", path.display())
        })?;
    }
    let warning = (!diagnostics.is_empty()).then(|| diagnostics.join("\n"));
    Ok(Response::ok(
        serde_json::json!({ "rendered": rendered, "warning": warning }),
    ))
}

fn render_gsn_response(
    model: &quoin_assurance::GsnCase,
    diagram: bool,
) -> Result<Response, String> {
    quoin_assurance::validate_gsn(model)?;
    let rendered = if diagram {
        quoin_assurance::render_gsn_diagram(model)?
    } else {
        serde_json::to_string_pretty(model).map_err(|error| error.to_string())?
    };
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "invalid test graph should report why it was refused"
)]
mod gsn_refusal_tests {
    use super::{render_gsn_response, render_sacm_response};
    use ix_trace_rs::trace;
    use quoin_assurance::{GsnCase, GsnEdge, GsnNode};

    fn node(id: &str, kind: &str) -> GsnNode {
        GsnNode {
            id: id.to_owned(),
            kind: kind.to_owned(),
            source: id.to_owned(),
            statement: id.to_owned(),
            status: "open".to_owned(),
            open_reason: None,
            undeveloped: Some(true),
            evidence: None,
            source_data: None,
        }
    }

    /// Failed WF checks use the command-error path: nonzero exit, no stdout,
    /// and a diagnostic naming the rule and both endpoints.
    #[trace("FR-222-AC-1", "FR-222-AC-2")]
    #[test]
    fn invalid_wf_models_fail_cli_with_empty_stdout_and_endpoint_diagnostic() {
        for (from, from_kind, to, to_kind, edge_kind, rule) in [
            ("Sn1", "solution", "G1", "goal", "supported_by", "WF-1"),
            ("A1", "assumption", "G1", "goal", "in_context_of", "WF-2"),
        ] {
            let model = GsnCase {
                format: "quoin.gsn-case/v1".to_owned(),
                nodes: vec![node(from, from_kind), node(to, to_kind)],
                edges: vec![GsnEdge {
                    kind: edge_kind.to_owned(),
                    from: from.to_owned(),
                    to: to.to_owned(),
                }],
                roots: vec![to.to_owned()],
                unreachable: vec![],
                unmapped: vec![],
            };
            let error = render_gsn_response(&model, false).expect_err("invalid graph is refused");
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let exit =
                crate::emit_error_to(crate::ShellError::command(error), &mut stdout, &mut stderr);
            assert_ne!(exit, std::process::ExitCode::SUCCESS);
            assert!(stdout.is_empty());
            let diagnostic = String::from_utf8(stderr).expect("diagnostic is UTF-8");
            assert!(diagnostic.contains(rule), "{diagnostic}");
            assert!(diagnostic.contains(from), "{diagnostic}");
            assert!(diagnostic.contains(to), "{diagnostic}");
        }
    }

    #[trace("FR-226-AC-6")]
    #[test]
    fn sacm_wf6_failure_emits_no_xmi_or_report() {
        let repo = tempfile::tempdir().expect("scratch repository");
        let report_path = repo.path().join("sacm-report.json");
        let model = GsnCase {
            format: "quoin.gsn-case/v1".to_owned(),
            nodes: vec![],
            edges: vec![GsnEdge {
                kind: "supported_by".to_owned(),
                from: "G1".to_owned(),
                to: "Sn2".to_owned(),
            }],
            roots: vec![],
            unreachable: vec![],
            unmapped: vec![],
        };
        let error = render_sacm_response(&model, repo.path(), Some(&report_path))
            .expect_err("invalid model is refused");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit =
            crate::emit_error_to(crate::ShellError::command(error), &mut stdout, &mut stderr);
        assert_ne!(exit, std::process::ExitCode::SUCCESS);
        assert!(stdout.is_empty());
        assert!(!report_path.exists());
        let diagnostic = String::from_utf8(stderr).expect("diagnostic is UTF-8");
        assert!(
            diagnostic.contains("WF-6") && diagnostic.contains("G1") && diagnostic.contains("Sn2")
        );
    }

    #[trace("FR-227-AC-1", "FR-227-AC-2", "FR-227-AC-3")]
    #[test]
    fn sacm_renders_one_sorted_diagnostic_and_json_item_per_omitted_source() {
        let repo = tempfile::tempdir().expect("scratch repository");
        let report_path = repo.path().join("sacm-report.json");
        let unmapped = quoin_assurance::Unmapped {
            source: "unmapped-source".to_owned(),
            reason: "no defined mapping".to_owned(),
        };
        let model = GsnCase {
            format: "quoin.gsn-case/v1".to_owned(),
            nodes: vec![node("G1", "goal")],
            edges: vec![],
            roots: vec!["G1".to_owned()],
            unreachable: vec![
                "FR-270".to_owned(),
                "FR-260".to_owned(),
                "FR-260".to_owned(),
            ],
            unmapped: vec![unmapped.clone(), unmapped],
        };
        let response = render_sacm_response(&model, repo.path(), Some(&report_path))
            .expect("valid model exports");
        assert_eq!(
            response.payload["warning"],
            "unmapped: unmapped-source: no defined mapping\nunreachable: FR-260\nunreachable: FR-270"
        );
        let xmi = response.payload["rendered"].as_str().expect("rendered XMI");
        assert!(
            !xmi.contains("unmapped-source") && !xmi.contains("FR-260") && !xmi.contains("FR-270")
        );
        let first_bytes = std::fs::read(&report_path).expect("read retained report");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&first_bytes).expect("valid report"),
            serde_json::json!({
                "unmapped": [{"source": "unmapped-source", "reason": "no defined mapping"}],
                "unreachable": ["FR-260", "FR-270"],
            })
        );
        render_sacm_response(&model, repo.path(), Some(&report_path)).expect("repeat export");
        assert_eq!(
            first_bytes,
            std::fs::read(report_path).expect("read repeat report")
        );
    }

    /// A failed export aggregates violations, names every affected id, and
    /// does not expose a partial rendered model on stdout.
    #[trace("FR-222-AC-9")]
    #[test]
    fn aggregate_wf_refusal_has_nonzero_exit_and_no_partial_stdout() {
        let mut undeveloped_strategy = node("S2", "strategy");
        undeveloped_strategy.undeveloped = None;
        let model = GsnCase {
            format: "quoin.gsn-case/v1".to_owned(),
            nodes: vec![
                node("G1", "goal"),
                node("G1", "goal"),
                node("S1", "strategy"),
                undeveloped_strategy,
            ],
            edges: vec![
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "S1".to_owned(),
                    to: "S1".to_owned(),
                },
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "S1".to_owned(),
                    to: "missing".to_owned(),
                },
            ],
            roots: vec!["G1".to_owned()],
            unreachable: vec![],
            unmapped: vec![],
        };
        let error = render_gsn_response(&model, false).expect_err("invalid model is refused");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit =
            crate::emit_error_to(crate::ShellError::command(error), &mut stdout, &mut stderr);
        assert_ne!(exit, std::process::ExitCode::SUCCESS);
        assert!(stdout.is_empty());
        let diagnostic = String::from_utf8(stderr).expect("diagnostic is UTF-8");
        for rule in ["WF-4", "WF-5", "WF-6"] {
            assert!(diagnostic.contains(rule), "{diagnostic}");
        }
        for id in ["G1", "S1", "S2", "missing"] {
            assert!(diagnostic.contains(id), "{diagnostic}");
        }
    }
}

struct CaseRequestInputs<'a> {
    arguments: &'a ArgMatches,
    repo: &'a str,
    signature_verifier: Option<&'a crate::analysis_evidence::SignatureVerifier>,
    modules: &'a [String],
    run_observations: &'a [crate::run_observation::RunObservationFile],
    coverage: &'a serde_json::Value,
    formalization_records: &'a [quoin_assurance::FormalizationRecordView],
    store: &'a serde_json::Value,
    audit: &'a serde_json::Value,
    frontmatter: &'a serde_json::Value,
    trust: &'a serde_json::Value,
}

fn build_case_request(input: &CaseRequestInputs<'_>) -> Result<serde_json::Value, Failure> {
    let CaseRequestInputs {
        arguments,
        repo,
        signature_verifier,
        modules,
        run_observations,
        coverage,
        formalization_records,
        store,
        audit,
        frontmatter,
        trust,
    } = input;
    let mut unreadable = field(frontmatter, "unreadable")?
        .as_array()
        .cloned()
        .unwrap_or_default();
    unreadable.extend(field(trust, "unreadable")?.as_array().into_iter().flatten().filter_map(serde_json::Value::as_str).map(|path| serde_json::json!({ "path": path, "reason": "trust decision is unreadable or invalid" })));
    let observations: Vec<serde_json::Value> = ["runs", "scans"]
        .iter()
        .filter_map(|key| store.get(*key)?.as_array())
        .flatten()
        .filter_map(|record| {
            Some(serde_json::json!({
                "suite": record.get("suite")?,
                "timestamp": record.get("timestamp")?,
            }))
        })
        .collect();
    let (case_obligations, analysis_evidence) = analysis_case_inputs(
        repo,
        modules,
        store,
        field(coverage, "obligations")?,
        run_observations,
        *signature_verifier,
    )?;
    let mut case_request = serde_json::json!({
        "bindings": field(store, "bindings")?, "observations": observations,
        "healthy": field(&field(audit, "report")?, "healthy")?,
        "unevaluated": field(&field(audit, "report")?, "unevaluated")?,
        "documents": field(frontmatter, "documents")?, "obligations": case_obligations,
        "findings": field(&field(audit, "report")?, "findings")?,
        "unreadable": unreadable, "producer_trust": field(trust, "assessments")?,
        "evidence_independence": audit.get("independence"),
        "formalization_records": formalization_records
    });
    let object = case_request
        .as_object_mut()
        .ok_or_else(|| "assurance case request must be an object".to_owned())?;
    object.insert("analysis_evidence".to_owned(), analysis_evidence);
    if let Some(claim_types) = arguments.get_many::<String>("claim-type") {
        object.insert(
            "claim_types".to_owned(),
            serde_json::json!(claim_types.cloned().collect::<Vec<_>>()),
        );
    }
    Ok(case_request)
}

fn read_formalization_records(
    path: Option<&String>,
    repo: &str,
    modules: &[String],
    obligations: &serde_json::Value,
    coverage: &serde_json::Value,
) -> Result<Vec<quoin_assurance::FormalizationRecordView>, Failure> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let rules = crate::formal_rules::assurance_declarations(std::path::Path::new(repo), modules)?;
    let identities = field(coverage, "formalization_identities")?;
    Ok(crate::formalization_records::read_and_join(
        path,
        obligations,
        &identities,
        &rules,
    )?)
}

fn read_run_observations(
    arguments: &ArgMatches,
) -> Result<Vec<crate::run_observation::RunObservationFile>, String> {
    let paths = arguments
        .get_many::<String>("run-observation")
        .map(|paths| paths.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    crate::run_observation::read_run_observations(&paths)
}

fn run_audit(
    obligations: &serde_json::Value,
    store: &serde_json::Value,
    catalog: &serde_json::Value,
    policy: &serde_json::Value,
) -> Result<Response, String> {
    invoke(
        "auditor.audit",
        &serde_json::json!({ "input": {
            "obligations": obligations,
            "bindings": field(store, "bindings")?, "runs": field(store, "runs")?,
            "scans": field(store, "scans")?, "injections": field(store, "injections")?,
            "mockInspectionSuites": field(store, "mock_inspection_suites")?,
            "vacuousScanSuites": field(store, "vacuous_scan_suites")?,
            "independence": field(store, "independence")?, "catalog": catalog,
            "independencePolicy": policy
        }}),
    )
}

fn analysis_case_inputs(
    repo: &str,
    modules: &[String],
    store_inputs: &serde_json::Value,
    obligation_value: serde_json::Value,
    observations: &[crate::run_observation::RunObservationFile],
    signature_verifier: Option<&crate::analysis_evidence::SignatureVerifier>,
) -> Result<(Vec<quoin_quire_types::Obligation>, serde_json::Value), Failure> {
    let mut obligations: Vec<quoin_quire_types::Obligation> =
        serde_json::from_value(obligation_value)
            .map_err(|error| format!("quire.coverage obligations are malformed: {error}"))?;
    let analysis = crate::analysis_evidence::compute(
        repo,
        modules,
        store_inputs,
        &obligations,
        observations,
        signature_verifier,
    )?;
    let report = serde_json::to_value(&analysis.report)
        .map_err(|error| format!("analysis evidence report is malformed: {error}"))?;
    crate::analysis_evidence::attach_unique_target_identities(
        &mut obligations,
        report
            .get("coverage_matrix")
            .and_then(serde_json::Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default(),
    )?;
    let items = serde_json::json!({
        "record_binders": report.get("record_binders").cloned().unwrap_or_else(|| serde_json::json!([])),
        "evidence_results": report.get("evidence_results").cloned().unwrap_or_else(|| serde_json::json!([])),
    });
    Ok((obligations, items))
}

fn present(operation: &str, value: &serde_json::Value, json: bool) -> Result<Response, String> {
    let rendered = if json {
        serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?
    } else {
        let response = invoke(operation, value)?;
        if !response.outcome.carries_payload() {
            return Ok(response);
        }
        field(&response.payload, "rendered")?
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

fn policy(
    arguments: &ArgMatches,
    obligations: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let Some(path) = arguments.get_one::<String>("independence-policy") else {
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
    let response = invoke(
        "evidence.parse_policy",
        &serde_json::json!({ "text": text, "known_obligations": known_obligations }),
    )?;
    if !response.outcome.carries_payload() {
        return Err("independence policy was refused".to_owned());
    }
    field(&response.payload, "policy")
}

fn json_file(path: &str, label: &str) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("{label} are not readable JSON: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{label} are not readable JSON: {error}"))
}
fn values(arguments: &ArgMatches, name: &str) -> Vec<String> {
    arguments
        .get_many::<String>(name)
        .map_or_else(Vec::new, |values| values.cloned().collect())
}
fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}
fn field(value: &serde_json::Value, name: &str) -> Result<serde_json::Value, String> {
    value
        .get(name)
        .cloned()
        .ok_or_else(|| format!("core response did not return {name}"))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::command;
    use ix_trace_rs::trace;

    #[trace("FR-040", "FR-062", "FR-153-AC-12")]
    #[test]
    fn tc_447_742_preserves_the_retained_assurance_grammar() {
        let matches = command()
            .try_get_matches_from([
                "assurance",
                "--repo",
                "other",
                "--module",
                "module",
                "--claim-type",
                "StR",
                "--claim-type",
                "hazard",
                "--independence-policy",
                "policy.json",
                "--run-observation",
                "one.json",
                "--run-observation",
                "two.json",
                "--json",
            ])
            .expect("the retained derived-view grammar parses");

        assert_eq!(
            matches.get_one::<String>("repo").map(String::as_str),
            Some("other")
        );
        assert_eq!(
            matches
                .get_many::<String>("claim-type")
                .expect("claim types")
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["StR", "hazard"]
        );
        assert!(matches.get_flag("json"));
        assert_eq!(
            matches
                .get_many::<String>("run-observation")
                .expect("run observations")
                .count(),
            2
        );
    }

    #[trace("FR-047", "FR-062")]
    #[test]
    fn tc_447_743_preserves_the_authored_argument_grammar() {
        let matches = command()
            .try_get_matches_from([
                "assurance",
                "--argument",
                "ARG-001",
                "--decisions",
                "decisions.json",
                "--discharge",
                "discharge.json",
                "--evidence",
                "evidence.json",
                "--as-of",
                "2026-09-14T00:00:00Z",
            ])
            .expect("the retained authored-argument grammar parses");

        assert_eq!(
            matches.get_one::<String>("argument").map(String::as_str),
            Some("ARG-001")
        );
        assert_eq!(
            matches.get_one::<String>("evidence").map(String::as_str),
            Some("evidence.json")
        );
        assert_eq!(
            matches.get_one::<String>("as-of").map(String::as_str),
            Some("2026-09-14T00:00:00Z")
        );
    }
}
