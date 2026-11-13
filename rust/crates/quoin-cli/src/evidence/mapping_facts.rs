// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence facts-from-mappings` (FR-173, NFR-038): the discharge facts
//! that a corpus's control mappings and scope exclusions support, ready for
//! `quoin evidence record-discharge`.
//!
//! The command states what people wrote. The actor, authority and instants of
//! every fact are the supplied attestation, a clause is answered only by a
//! control whose own `## Effectiveness` says `effective`, and nothing is
//! written to disk.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_assurance::{
    DischargeAttestation, DischargeFact, RecordDischargeRequest, read_discharge_facts,
    record_discharge_facts,
};
use quoin_core::protocol::{Diagnostic, Response};
use quoin_evidence::{DiskEvidence, EvidenceError, EvidenceSource};
use quoin_quire::ModuleSelection;
use quoin_quire::records::{Contract, Document, read};
use quoin_quire_types::{ClauseBindingOutcome, ClauseBindingReport};
use serde_json::{Value, json};

use super::{json_arg, required};

/// The types whose declared contract the command checks.
const CHECKED: [&str; 2] = ["control", "scope_exclusion"];

/// One `(set, clause)` row of a control's `## Mappings` table.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Mapping {
    set: String,
    clause: String,
}

/// A `control` artifact, reduced to what a fact needs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Control {
    id: String,
    effective: bool,
    mappings: Vec<Mapping>,
}

/// A `scope_exclusion` artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Exclusion {
    id: String,
    set: String,
    clause: String,
    responsible: String,
    rationale: String,
}

/// What the command reads from the corpus.
#[derive(Debug, Default)]
struct Corpus {
    controls: Vec<Control>,
    exclusions: Vec<Exclusion>,
}

/// The facts and the report lines of a successful run.
#[derive(Debug, PartialEq)]
struct Proposal {
    facts: Vec<Value>,
    report: Vec<String>,
}

pub(super) fn command() -> Command {
    Command::new("facts-from-mappings")
        .about("Propose discharge facts from effective controls and scope exclusions")
        .arg(
            Arg::new("binding")
                .long("binding")
                .required(true)
                .help("Quire clause-binding-v1 JSON path, or - for stdin."),
        )
        .arg(
            Arg::new("attestation")
                .long("attestation")
                .required(true)
                .help("JSON object with attestedBy, authority, attestedAt and expiresAt, written by a person."),
        )
        .arg(
            Arg::new("spec")
                .long("spec")
                .default_value("spec")
                .help("Directory of spec artifacts to read."),
        )
        .arg(
            Arg::new("approval-ref")
                .long("approval-ref")
                .help("Reference of the approval behind scope exclusions. Without it no disposition is emitted."),
        )
        .arg(
            Arg::new("repo")
                .long("repo")
                .help("Read the stored facts to name the stale ones."),
        )
        .arg(json_arg().hide(true))
}

pub(super) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    match propose_from(arguments) {
        Ok(proposal) => {
            let rendered =
                serde_json::to_string_pretty(&proposal.facts).map_err(|error| error.to_string())?;
            let mut payload = serde_json::Map::new();
            payload.insert("rendered".to_owned(), Value::String(rendered));
            if !proposal.report.is_empty() {
                payload.insert(
                    "warning".to_owned(),
                    Value::String(proposal.report.join("\n")),
                );
            }
            Ok(Response {
                payload: Value::Object(payload),
                diagnostics: Vec::new(),
                outcome: Outcome::Ok,
            })
        }
        Err(message) => Ok(refused(&message)),
    }
}

fn propose_from(arguments: &ArgMatches) -> Result<Proposal, String> {
    let binding_source = required(arguments, "binding")?;
    let attestation = read_json(&required(arguments, "attestation")?, "attestation")?;
    let binding: ClauseBindingReport =
        serde_json::from_value(read_json(&binding_source, "clause binding")?).map_err(|error| {
            format!("clause binding is not a clause-binding-v1 report: {error}")
        })?;
    let spec = arguments
        .get_one::<String>("spec")
        .map_or_else(|| PathBuf::from("spec"), PathBuf::from);
    let corpus = read_corpus(&spec)?;
    let stored = arguments.get_one::<String>("repo").map(DiskEvidence::new);
    propose(
        &binding,
        &attestation,
        &corpus,
        arguments
            .get_one::<String>("approval-ref")
            .map(String::as_str),
        stored.as_ref(),
    )
}

fn read_json(source: &str, label: &str) -> Result<Value, String> {
    let bytes = if source == "-" {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut bytes)
            .map_err(|error| format!("cannot read {label}: {error}"))?;
        bytes
    } else {
        std::fs::read(source).map_err(|error| format!("cannot read {label} {source}: {error}"))?
    };
    serde_json::from_slice(&bytes).map_err(|error| format!("{label} is not JSON: {error}"))
}

/// A refusal: exit status 2, one diagnostic and nothing on stdout.
fn refused(message: &str) -> Response {
    Response {
        payload: Value::Null,
        diagnostics: vec![Diagnostic {
            code: "evidence.facts_from_mappings_refused".to_owned(),
            message: message.to_owned(),
            context: BTreeMap::new(),
        }],
        outcome: Outcome::Refused,
    }
}

fn read_corpus(spec: &Path) -> Result<Corpus, String> {
    if !spec.is_dir() {
        return Err(format!("--spec {} is not a directory", spec.display()));
    }
    let corpus = read(spec, &ModuleSelection::Ambient, &CHECKED)
        .map_err(|error| format!("{} cannot be read: {error}", spec.display()))?;
    if let Some(file) = corpus.unreadable.first() {
        return Err(format!(
            "{} fails validation: {}",
            file.path.display(),
            file.reason
        ));
    }
    if let Some(path) = corpus.malformed.first() {
        return Err(format!(
            "{} fails validation: malformed frontmatter",
            path.display()
        ));
    }
    let mut found = Corpus::default();
    for document in &corpus.documents {
        if !CHECKED.contains(&document.kind.as_str()) {
            continue;
        }
        if let Contract::Violated(reason) = &document.contract {
            return Err(format!("{} fails validation: {reason}", document.id));
        }
        if document.kind == "control" {
            found.controls.push(control(document)?);
        } else {
            found.exclusions.push(exclusion(document)?);
        }
    }
    found.controls.sort_by(|a, b| a.id.cmp(&b.id));
    found.exclusions.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(found)
}

/// The inline-code value of a section: its text, without the backtick pair.
fn inline_value(text: &str) -> &str {
    let text = text.trim();
    text.strip_prefix('`')
        .and_then(|rest| rest.strip_suffix('`'))
        .unwrap_or(text)
        .trim()
}

fn control(document: &Document) -> Result<Control, String> {
    let mappings = match document.tables.get("Mappings") {
        None => Vec::new(),
        Some(table) => {
            let column = |name: &str| table.headers.iter().position(|header| header == name);
            let (Some(set), Some(clause)) = (column("Set"), column("Clause")) else {
                return Err(format!(
                    "control {} has a Mappings table without Set and Clause columns",
                    document.id
                ));
            };
            table
                .rows
                .iter()
                .map(|row| Mapping {
                    set: row
                        .get(set)
                        .map(|cell| cell.trim().to_owned())
                        .unwrap_or_default(),
                    clause: row
                        .get(clause)
                        .map(|cell| cell.trim().to_owned())
                        .unwrap_or_default(),
                })
                .collect()
        }
    };
    let effective = document
        .sections
        .get("Effectiveness")
        .is_some_and(|text| inline_value(text) == "effective");
    Ok(Control {
        id: document.id.clone(),
        effective,
        mappings,
    })
}

fn exclusion(document: &Document) -> Result<Exclusion, String> {
    let value = |heading: &str| {
        document
            .sections
            .get(heading)
            .map(|text| inline_value(text).to_owned())
            .filter(|text| !text.is_empty())
            .ok_or_else(|| format!("scope_exclusion {} has no {heading} value", document.id))
    };
    Ok(Exclusion {
        id: document.id.clone(),
        set: value("Set")?,
        clause: value("Clause")?,
        responsible: value("Responsible")?,
        rationale: value("Rationale")?,
    })
}

fn set_key(binding: &ClauseBindingReport) -> String {
    let set = &binding.clause_set;
    format!("{}/{}@{}", set.authority, set.id, set.version)
}

/// The clauses of the binding report, as the checks below need them.
struct Clauses<'a> {
    set: String,
    listed: BTreeSet<&'a str>,
    binding: BTreeSet<&'a str>,
}

impl<'a> Clauses<'a> {
    fn of(binding: &'a ClauseBindingReport) -> Self {
        let clauses = |only_binding: bool| {
            binding
                .clauses
                .iter()
                .filter(|clause| !only_binding || clause.outcome == ClauseBindingOutcome::Binding)
                .map(|clause| clause.clause_id.as_str())
                .collect()
        };
        Self {
            set: set_key(binding),
            listed: clauses(false),
            binding: clauses(true),
        }
    }
}

/// Per binding clause, the `(control id, effective)` of each control that maps it.
type Answers<'a> = BTreeMap<&'a str, Vec<(&'a str, bool)>>;

/// The controls that answer each binding clause of the report's set, as
/// `(control id, effective)`, and the `unused` lines of the other mappings.
fn answers<'a>(
    clauses: &Clauses<'_>,
    controls: &'a [Control],
    report: &mut Vec<String>,
) -> Result<Answers<'a>, String> {
    let mut answers: BTreeMap<&str, Vec<(&str, bool)>> = BTreeMap::new();
    for control in controls {
        let mut seen = BTreeSet::new();
        for mapping in &control.mappings {
            let (set, clause) = (mapping.set.as_str(), mapping.clause.as_str());
            if !seen.insert((set, clause)) {
                return Err(format!("control {} maps {set} {clause} twice", control.id));
            }
            if set == clauses.set && !clauses.listed.contains(clause) {
                return Err(format!(
                    "control {} maps {set} {clause}, which the binding report does not list",
                    control.id
                ));
            }
            if set == clauses.set && clauses.binding.contains(clause) {
                answers
                    .entry(clause)
                    .or_default()
                    .push((control.id.as_str(), control.effective));
            } else {
                report.push(format!("unused {} {set} {clause}", control.id));
            }
        }
    }
    Ok(answers)
}

/// The exclusions by `(set, clause)`; a clause named twice, or both mapped and
/// excluded, is refused.
fn exclusions(corpus: &Corpus) -> Result<BTreeMap<(&str, &str), &Exclusion>, String> {
    let mut excluded: BTreeMap<(&str, &str), &Exclusion> = BTreeMap::new();
    for exclusion in &corpus.exclusions {
        let key = (exclusion.set.as_str(), exclusion.clause.as_str());
        if let Some(first) = excluded.insert(key, exclusion) {
            return Err(format!(
                "scope exclusions {} and {} both name {} {}",
                first.id, exclusion.id, exclusion.set, exclusion.clause
            ));
        }
    }
    for control in &corpus.controls {
        for mapping in &control.mappings {
            if let Some(exclusion) = excluded.get(&(mapping.set.as_str(), mapping.clause.as_str()))
            {
                return Err(format!(
                    "control {} maps {} {}, which scope exclusion {} also names",
                    control.id, mapping.set, mapping.clause, exclusion.id
                ));
            }
        }
    }
    Ok(excluded)
}

/// Compute the facts and report lines. Pure over its inputs: the store is
/// only read, and `store` is `None` when no `--repo` was given.
fn propose(
    binding: &ClauseBindingReport,
    attestation: &Value,
    corpus: &Corpus,
    approval_ref: Option<&str>,
    store: Option<&DiskEvidence>,
) -> Result<Proposal, String> {
    serde_json::from_value::<DischargeAttestation>(attestation.clone())
        .map_err(|error| format!("attestation is refused: {error}"))?;
    let clauses = Clauses::of(binding);
    let mut report = Vec::new();
    let answers = answers(&clauses, &corpus.controls, &mut report)?;
    let excluded = exclusions(corpus)?;

    let mut facts: BTreeMap<&str, Value> = BTreeMap::new();
    for (clause, controls) in &answers {
        let mut effective: Vec<&str> = controls
            .iter()
            .filter(|(_, effective)| *effective)
            .map(|(id, _)| *id)
            .collect();
        effective.sort_unstable();
        effective.dedup();
        if effective.is_empty() {
            let mut ids: Vec<&str> = controls.iter().map(|(id, _)| *id).collect();
            ids.sort_unstable();
            ids.dedup();
            report.push(format!("open {clause} {}", ids.join(" ")));
        } else {
            let refs: Vec<String> = effective.iter().map(|id| format!("ix://{id}")).collect();
            facts.insert(
                clause,
                json!({
                    "kind": "direct", "clauseId": clause,
                    "evidenceRefs": refs, "attestation": attestation,
                }),
            );
        }
    }
    for exclusion in excluded.values() {
        if exclusion.set != clauses.set || !clauses.binding.contains(exclusion.clause.as_str()) {
            continue;
        }
        match approval_ref {
            Some(approval) => {
                facts.insert(
                    exclusion.clause.as_str(),
                    json!({
                        "kind": "disposition", "clauseId": exclusion.clause,
                        "decision": "delegated",
                        "rationale": format!("{} (responsible: {})", exclusion.rationale, exclusion.responsible),
                        "approvalRef": approval, "attestation": attestation,
                    }),
                );
            }
            None => report.push(format!(
                "needs-approval {} {}",
                exclusion.clause, exclusion.id
            )),
        }
    }

    let facts: Vec<Value> = facts.into_values().collect();
    check_identifiers(binding, &facts, store, &clauses.set)?;
    if let Some(store) = store {
        stale(binding, &clauses, &facts, store, &mut report)?;
    }
    report.sort();
    report.dedup();
    Ok(Proposal { facts, report })
}

/// `stale <clause>` for each stored `direct` fact of a binding clause that no
/// emitted `direct` fact replaces.
fn stale(
    binding: &ClauseBindingReport,
    clauses: &Clauses<'_>,
    facts: &[Value],
    store: &DiskEvidence,
    report: &mut Vec<String>,
) -> Result<(), String> {
    let held = read_discharge_facts(store, &binding.clause_set)
        .map_err(|error| format!("the stored facts cannot be read: {error}"))?;
    for fact in held {
        if let DischargeFact::Direct(direct) = fact {
            let replaced = facts.iter().any(|emitted| {
                emitted.get("kind") == Some(&json!("direct"))
                    && emitted.get("clauseId") == Some(&json!(direct.clause_id))
            });
            if clauses.binding.contains(direct.clause_id.as_str()) && !replaced {
                report.push(format!("stale {}", direct.clause_id));
            }
        }
    }
    Ok(())
}

/// A source that reads the real store and discards every write.
struct DryRun<'a>(Option<&'a DiskEvidence>);

impl EvidenceSource for DryRun<'_> {
    fn read(&self, path: &str) -> Result<Option<String>, EvidenceError> {
        self.0.map_or(Ok(None), |store| store.read(path))
    }
    fn list_files(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        self.0
            .map_or_else(|| Ok(Vec::new()), |store| store.list_files(directory))
    }
    fn list_directories(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        self.0
            .map_or_else(|| Ok(Vec::new()), |store| store.list_directories(directory))
    }
    fn write(&mut self, _path: &str, _bytes: &[u8]) -> Result<(), EvidenceError> {
        Ok(())
    }
    fn publish(&mut self, _path: &str, _bytes: &[u8]) -> Result<bool, EvidenceError> {
        Ok(true)
    }
    fn remove(&mut self, _path: &str) -> Result<(), EvidenceError> {
        Ok(())
    }
    fn source_files(&self) -> Result<Vec<String>, EvidenceError> {
        Ok(Vec::new())
    }
    fn source_text(&self, _path: &str) -> Result<String, EvidenceError> {
        Ok(String::new())
    }
}

/// Run the facts through `record-discharge`'s own admission against the real
/// store, discarding the writes, so that an identifier `record-discharge`
/// would refuse is refused here with the same status.
fn check_identifiers(
    binding: &ClauseBindingReport,
    facts: &[Value],
    store: Option<&DiskEvidence>,
    set: &str,
) -> Result<(), String> {
    let mut sink = DryRun(store);
    record_discharge_facts(
        &mut sink,
        &RecordDischargeRequest {
            binding: binding.clone(),
            facts: facts.to_vec(),
        },
    )
    .map(|_| ())
    .map_err(|error| format!("set {set}: {error}"))
}
