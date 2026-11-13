// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin soup promote`: write one `soup_record` per purl of an ingested SBOM
//! (FR-141).
//!
//! The components are the FR-041 adapter's (`quoin_evidence::adapters::sbom`);
//! this command has no SBOM reader. The record type, skeleton and schema belong
//! to `spec-objects-security`, found through the module catalog.

mod plan;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_core::protocol::{Diagnostic, Response};
use quoin_evidence::adapters::parse_sbom;

use crate::core_runtime::invoke;
use plan::{Existing, NewRecord, Plan};

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str = "Promote ingested SBOM components to SOUP records.";

/// The module that owns the `soup_record` object type.
const OWNER: &str = "spec-objects-security";
const OBJECT: &str = "soup_record";
/// A record larger than this is not read as one.
const MAX_RECORD_BYTES: u64 = 1024 * 1024;

pub(crate) fn command() -> Command {
    Command::new("soup")
        .about(ABOUT)
        .subcommand_required(true)
        .subcommand(
            Command::new("promote")
                .about("Write one soup_record per purl of an SBOM, leaving existing records alone")
                .arg(
                    Arg::new("sbom")
                        .value_name("SBOM")
                        .required(true)
                        .help("CycloneDX or SPDX JSON inventory"),
                )
                .arg(
                    Arg::new("repo")
                        .long("repo")
                        .value_name("DIR")
                        .default_value(".")
                        .help("Repository root; --out defaults to <repo>/spec/soup"),
                )
                .arg(
                    Arg::new("out")
                        .long("out")
                        .value_name("DIR")
                        .help("Directory new records are written to"),
                )
                .arg(
                    Arg::new("records")
                        .long("records")
                        .value_name("DIR")
                        .help("Directory searched for existing records; defaults to --out"),
                )
                .arg(
                    Arg::new("discharges")
                        .long("discharges")
                        .value_name("OBLIGATION")
                        .help("Obligation id written as disposition_clause on new records"),
                )
                .arg(
                    Arg::new("module")
                        .long("module")
                        .value_name("DIR")
                        .action(clap::ArgAction::Append)
                        .help("Module root to look for the soup_record type in; repeat to add roots. Without it the installed modules are searched"),
                ),
        )
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    match arguments.subcommand() {
        Some(("promote", promote)) => run_promote(promote),
        _ => Err("an unknown soup command reached dispatch".to_owned()),
    }
}

/// A refusal: no payload, one diagnostic, the refused status.
fn refuse(code: &str, message: String, context: &[(&str, String)]) -> Response {
    Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic {
            code: code.to_owned(),
            message,
            context: context
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect(),
        }],
        outcome: Outcome::Refused,
    }
}

fn arg<'a>(arguments: &'a ArgMatches, name: &str) -> Option<&'a str> {
    arguments.get_one::<String>(name).map(String::as_str)
}

fn run_promote(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = PathBuf::from(arg(arguments, "repo").unwrap_or("."));
    let out = arg(arguments, "out").map_or_else(|| repo.join("spec/soup"), PathBuf::from);
    let records = arg(arguments, "records").map_or_else(|| out.clone(), PathBuf::from);
    let roots: Option<Vec<String>> = arguments
        .get_many::<String>("module")
        .map(|roots| roots.cloned().collect());

    let skeleton = match owning_skeleton(roots.as_deref())? {
        Ok(skeleton) => skeleton,
        Err(refusal) => return Ok(refusal),
    };

    let source = arg(arguments, "sbom").ok_or_else(|| "<SBOM> is required".to_owned())?;
    let raw = std::fs::read_to_string(source)
        .map_err(|error| format!("cannot read {source}: {error}"))?;
    let parsed = match parse_sbom(&raw) {
        Ok(parsed) => parsed,
        Err(error) => {
            return Ok(refuse(
                "soup.sbom_refused",
                error.to_string(),
                &[("path", source.to_owned())],
            ));
        }
    };
    let symbols: Vec<String> = parsed
        .entries
        .into_iter()
        .map(|e| e.symbol.as_str().to_owned())
        .collect();

    let existing = read_records(&records)?;
    if let Some((first, second)) = plan::duplicate_records(&existing) {
        return Ok(refuse(
            "soup.duplicate_records",
            format!(
                "two records carry the purl {}: {} and {}",
                first.purl,
                first.path.display(),
                second.path.display()
            ),
            &[
                ("first", first.path.display().to_string()),
                ("second", second.path.display().to_string()),
            ],
        ));
    }

    let (clause, unmatched) = match arg(arguments, "discharges") {
        None => (None, None),
        Some(id) => {
            let id = id.trim();
            if stated_obligations(&repo, id)? {
                (Some(id.to_owned()), None)
            } else {
                (None, Some(id.to_owned()))
            }
        }
    };

    let planned = plan::plan(&symbols, &existing);
    let written = write_new(&planned, &skeleton, &out, &records, clause.as_deref())?;
    let rendered = report(&planned, &written, unmatched.as_deref());
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

/// The `soup_record` skeleton of the installed owning module, or the refusal
/// that names the module when it is absent.
fn owning_skeleton(roots: Option<&[String]>) -> Result<Result<String, Response>, String> {
    let request = roots.map_or_else(
        || serde_json::json!({}),
        |roots| serde_json::json!({ "roots": roots }),
    );
    let catalog = invoke("catalog.load", &request)?;
    if !catalog.outcome.carries_payload() {
        return Err("the module catalog could not be loaded".to_owned());
    }
    let skeleton_path = catalog
        .payload
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|entry| entry.get("name").and_then(serde_json::Value::as_str) == Some(OBJECT))
        .map(|entry| {
            entry
                .get("skeletonPath")
                .and_then(serde_json::Value::as_str)
        });
    let missing = |what: &str| {
        refuse(
            "soup.module_missing",
            format!("{what}: install the {OWNER} module"),
            &[("module", OWNER.to_owned())],
        )
    };
    let Some(skeleton_path) = skeleton_path else {
        return Ok(Err(missing(
            "the `soup_record` object type is not installed",
        )));
    };
    let Some(skeleton_path) = skeleton_path else {
        return Ok(Err(missing("the `soup_record` type has no skeleton")));
    };
    std::fs::read_to_string(skeleton_path)
        .map(Ok)
        .map_err(|error| format!("cannot read the soup_record skeleton {skeleton_path}: {error}"))
}

/// Whether some obligation states `id`, asked of Quire as `evidence record` does.
fn stated_obligations(repo: &Path, id: &str) -> Result<bool, String> {
    if id.is_empty() {
        return Err("--discharges needs an obligation id".to_owned());
    }
    let coverage = invoke(
        "quire.coverage",
        &serde_json::json!({ "scope": repo, "modules": Vec::<String>::new() }),
    )?;
    if !coverage.outcome.carries_payload() {
        return Err("quire could not derive the obligations".to_owned());
    }
    Ok(coverage
        .payload
        .get("obligations")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .any(|obligation| obligation.get("id").and_then(serde_json::Value::as_str) == Some(id)))
}

/// Every `soup_record` below `dir`, in path order. A missing directory holds none.
fn read_records(dir: &Path) -> Result<Vec<Existing>, String> {
    let mut found = Vec::new();
    collect_records(dir, &mut found)?;
    found.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

fn collect_records(dir: &Path, found: &mut Vec<Existing>) -> Result<(), String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("cannot read {}: {error}", dir.display())),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            collect_records(&path, found)?;
        } else if kind.is_file()
            && path.extension().is_some_and(|ext| ext == "md")
            && let Some(purl) = record_purl(&path)
        {
            found.push(Existing { path, purl });
        }
    }
    Ok(())
}

/// The purl of the `soup_record` in `path`, when it is one.
fn record_purl(path: &Path) -> Option<String> {
    if std::fs::metadata(path).ok()?.len() > MAX_RECORD_BYTES {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let (yaml, _) = text.strip_prefix("---\n")?.split_once("\n---")?;
    let front = quoin_yaml::from_str(yaml).ok()?;
    (front.get("type")?.as_str()? == OBJECT)
        .then(|| front.get("purl")?.as_str().map(str::to_owned))
        .flatten()
}

/// Create each new record and return where each landed. Nothing is overwritten:
/// a file already at the destination is accepted only when it is the record
/// of the same purl, and is then reported as unchanged.
fn write_new(
    planned: &Plan,
    skeleton: &str,
    out: &Path,
    records: &Path,
    clause: Option<&str>,
) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut ids = Vec::new();
    for dir in [records, out] {
        for record in read_ids(dir) {
            ids.push(record);
        }
    }
    let mut next = plan::next_id_number(ids.iter().map(String::as_str));
    let mut written = BTreeMap::new();
    if planned.created.is_empty() {
        return Ok(written);
    }
    std::fs::create_dir_all(out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    for record in &planned.created {
        let path = out.join(&record.file);
        let id = format!("SOUP-{next:03}");
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                let text = plan::render(skeleton, &id, record, clause);
                file.write_all(text.as_bytes())
                    .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
                next = next.saturating_add(1);
                written.insert(record.purl.clone(), path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if record_purl(&path).as_deref() != Some(record.purl.as_str()) {
                    return Err(format!(
                        "{} exists and is not the soup_record for {}",
                        path.display(),
                        record.purl
                    ));
                }
            }
            Err(error) => return Err(format!("cannot write {}: {error}", path.display())),
        }
    }
    Ok(written)
}

/// The `id` of every record below `dir`.
fn read_ids(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let _ = collect_records(dir, &mut found);
    found
        .iter()
        .filter_map(|record| {
            let text = std::fs::read_to_string(&record.path).ok()?;
            let (yaml, _) = text.strip_prefix("---\n")?.split_once("\n---")?;
            let front = quoin_yaml::from_str(yaml).ok()?;
            front.get("id")?.as_str().map(str::to_owned)
        })
        .collect()
}

/// The deterministic report: counts first, then each list, sorted.
fn report(planned: &Plan, written: &BTreeMap<String, PathBuf>, unmatched: Option<&str>) -> String {
    let created: Vec<&NewRecord> = planned
        .created
        .iter()
        .filter(|record| written.contains_key(&record.purl))
        .collect();
    // A destination already holding the purl's record is unchanged, not created.
    let adopted = planned.created.len() - created.len();
    let mut text = String::new();
    let _ = writeln!(text, "components read: {}", planned.read);
    let _ = writeln!(text, "records created: {}", created.len());
    for record in &created {
        let path = written
            .get(&record.purl)
            .map_or_else(String::new, |p| p.display().to_string());
        let _ = writeln!(text, "  {} {path}", record.purl);
    }
    let _ = writeln!(
        text,
        "records unchanged: {}",
        planned.unchanged.len() + adopted
    );
    for record in &planned.unchanged {
        let _ = writeln!(text, "  {} {}", record.purl, record.path.display());
    }
    for record in planned
        .created
        .iter()
        .filter(|r| !written.contains_key(&r.purl))
    {
        let _ = writeln!(text, "  {}", record.purl);
    }
    let _ = writeln!(text, "superseded candidates: {}", planned.superseded.len());
    for old in &planned.superseded {
        let _ = writeln!(
            text,
            "  {} {} superseded by {}",
            old.path.display(),
            old.purl,
            old.by
        );
    }
    let _ = writeln!(text, "merged duplicates: {}", planned.merged.len());
    for purl in &planned.merged {
        let _ = writeln!(text, "  {purl}");
    }
    let _ = writeln!(text, "components skipped: {}", planned.skipped.len());
    for symbol in &planned.skipped {
        let _ = writeln!(text, "  {symbol}");
    }
    if let Some(id) = unmatched {
        let _ = writeln!(text, "unmatched: {id}");
    }
    text.trim_end().to_owned()
}
