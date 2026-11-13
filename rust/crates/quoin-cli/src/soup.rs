// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin soup promote`: write one `soup_record` per purl of an ingested SBOM
//! (FR-141); `quoin soup report`: render them with their advisories and
//! support (FR-144).
//!
//! The components are the FR-041 adapter's (`quoin_evidence::adapters::sbom`);
//! this command has no SBOM reader. The record type, skeleton and schema belong
//! to `spec-objects-security`, found through the module catalog.

mod advisory;
mod plan;
mod report;
mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use quoin_core::protocol::{Diagnostic, Response};
use quoin_evidence::adapters::parse_sbom;

use crate::core_runtime::invoke;
use plan::{Existing, NewRecord, Plan};

/// The longest file name the common filesystems accept.
const MAX_FILE_NAME: usize = 255;

/// The one-line summary, shared by the grammar and the root help catalogue.
pub(crate) const ABOUT: &str = "Promote SBOM components to SOUP records and report on them.";

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
                        .help("Where new records are written"),
                )
                .arg(
                    Arg::new("records")
                        .long("records")
                        .value_name("DIR")
                        .help("Where existing records are looked up; defaults to --out"),
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
        .subcommand(
            Command::new("report")
                .about("Render one row per soup_record with its advisories and support status")
                .arg(
                    Arg::new("repo")
                        .long("repo")
                        .value_name("DIR")
                        .default_value(".")
                        .help("Repository root; --records defaults to <repo>/spec/soup"),
                )
                .arg(
                    Arg::new("records")
                        .long("records")
                        .value_name("DIR")
                        .help("Where the soup_record files are read"),
                )
                .arg(
                    Arg::new("binding")
                        .long("binding")
                        .value_name("FILE")
                        .required(true)
                        .help("Quire clause-binding-v1 JSON report whose clauses the records' disposition_clause names"),
                )
                .arg(
                    Arg::new("as-of")
                        .long("as-of")
                        .value_name("INSTANT")
                        .required(true)
                        .help("Explicit evaluation instant; its UTC date is the end-of-support comparison date"),
                )
                .arg(
                    Arg::new("support")
                        .long("support")
                        .value_name("FILE")
                        .help("JSON file of public support statements: [{purl, support_status, end_of_support, source}]"),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Emit the report as JSON instead of Markdown"),
                ),
        )
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let outcome = match arguments.subcommand() {
        Some(("promote", promote)) => run_promote(promote),
        Some(("report", report)) => report::run(report),
        _ => return Err("an unknown soup command reached dispatch".to_owned()),
    };
    match outcome {
        Ok(response) => Ok(response),
        Err(Stop::Refuse(response)) => Ok(*response),
        Err(Stop::Fail(message)) => Err(message),
    }
}

/// Why a promotion ended before it wrote anything.
enum Stop {
    /// A response to return as is: a coded refusal, or a refusal Quire stated.
    Refuse(Box<Response>),
    /// An operational failure (exit 1).
    Fail(String),
}

impl From<String> for Stop {
    fn from(message: String) -> Self {
        Self::Fail(message)
    }
}

/// A refusal: no payload, one diagnostic, the refused status.
fn refuse(code: &str, message: String, context: &[(&str, String)]) -> Stop {
    Stop::Refuse(Box::new(Response {
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
    }))
}

fn arg<'a>(arguments: &'a ArgMatches, name: &str) -> Option<&'a str> {
    arguments.get_one::<String>(name).map(String::as_str)
}

fn run_promote(arguments: &ArgMatches) -> Result<Response, Stop> {
    let repo = PathBuf::from(arg(arguments, "repo").unwrap_or("."));
    let out = arg(arguments, "out").map_or_else(|| repo.join("spec/soup"), PathBuf::from);
    let records = arg(arguments, "records").map_or_else(|| out.clone(), PathBuf::from);
    let roots: Option<Vec<String>> = arguments
        .get_many::<String>("module")
        .map(|roots| roots.cloned().collect());

    for (what, path) in std::iter::once(("--repo", repo.as_path())).chain(
        roots
            .iter()
            .flatten()
            .map(|root| ("--module", Path::new(root))),
    ) {
        if !path.exists() {
            return Err(refuse(
                "soup.path_missing",
                format!(
                    "{what} {} does not exist (no such file or directory)",
                    path.display()
                ),
                &[("path", path.display().to_string())],
            ));
        }
    }

    let skeleton = owning_skeleton(roots.as_deref())?;

    let source = arg(arguments, "sbom").ok_or_else(|| "<SBOM> is required".to_owned())?;
    let raw = std::fs::read_to_string(source)
        .map_err(|error| format!("cannot read {source}: {error}"))?;
    let parsed = parse_sbom(&raw).map_err(|error| {
        refuse(
            "soup.sbom_refused",
            error.to_string(),
            &[("path", source.to_owned())],
        )
    })?;
    let symbols: Vec<String> = parsed
        .entries
        .into_iter()
        .map(|e| e.symbol.as_str().to_owned())
        .collect();

    let existing = read_records(&records)?;
    if let Some((first, second)) = plan::duplicate_records(&existing) {
        return Err(refuse(
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
    let mut ids: Vec<&str> = existing.iter().filter_map(|r| r.id.as_deref()).collect();
    let beside = if out == records {
        Vec::new()
    } else {
        read_records(&out)?
    };
    ids.extend(beside.iter().filter_map(|r| r.id.as_deref()));

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
    let first_id = plan::next_id_number(ids);
    let adopted = check_destinations(&planned, &out)?;
    let written = write_new(
        &planned,
        &skeleton,
        &out,
        first_id,
        &adopted,
        clause.as_deref(),
    )?;
    let rendered = report(&planned, &written, unmatched.as_deref());
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

/// The `soup_record` skeleton of the installed owning module, or the refusal
/// that names the module when it is absent.
fn owning_skeleton(roots: Option<&[String]>) -> Result<String, Stop> {
    let request = roots.map_or_else(
        || serde_json::json!({}),
        |roots| serde_json::json!({ "roots": roots }),
    );
    let catalog = invoke("catalog.load", &request)?;
    if !catalog.outcome.carries_payload() {
        return Err(Stop::Fail(
            "the module catalog could not be loaded".to_owned(),
        ));
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
        return Err(missing("the `soup_record` object type is not installed"));
    };
    let Some(skeleton_path) = skeleton_path else {
        return Err(missing("the `soup_record` type has no skeleton"));
    };
    std::fs::read_to_string(skeleton_path).map_err(|error| {
        Stop::Fail(format!(
            "cannot read the soup_record skeleton {skeleton_path}: {error}"
        ))
    })
}

/// Whether some obligation states `id`, asked of Quire as `evidence record`
/// does: a coverage Quire refuses is returned as Quire stated it.
fn stated_obligations(repo: &Path, id: &str) -> Result<bool, Stop> {
    if id.is_empty() {
        return Err(Stop::Fail("--discharges needs an obligation id".to_owned()));
    }
    let coverage = invoke(
        "quire.coverage",
        &serde_json::json!({ "scope": repo, "modules": Vec::<String>::new() }),
    )?;
    if !coverage.outcome.carries_payload() {
        return Err(Stop::Refuse(Box::new(coverage)));
    }
    Ok(coverage
        .payload
        .get("obligations")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .any(|obligation| obligation.get("id").and_then(serde_json::Value::as_str) == Some(id)))
}

/// Every `soup_record` below `dir`, in path order. A missing directory holds
/// none; a `.md` file that cannot be read in full is a refusal, since a record
/// that cannot be seen could be written a second time.
fn read_records(dir: &Path) -> Result<Vec<Existing>, Stop> {
    let mut found = Vec::new();
    collect_records(dir, &mut found)?;
    found.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

fn unreadable(path: &Path, why: &str) -> Stop {
    refuse(
        "soup.record_unreadable",
        format!("{} {why}", path.display()),
        &[("path", path.display().to_string())],
    )
}

fn collect_records(dir: &Path, found: &mut Vec<Existing>) -> Result<(), Stop> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Stop::Fail(format!(
                "cannot read {}: {error}",
                dir.display()
            )));
        }
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
            && let Some(record) = read_record(&path)?
        {
            found.push(record);
        }
    }
    Ok(())
}

/// The `soup_record` in `path`, or `None` for any other document.
fn read_record(path: &Path) -> Result<Option<Existing>, Stop> {
    let length = std::fs::metadata(path)
        .map_err(|error| unreadable(path, &format!("cannot be read: {error}")))?
        .len();
    if length > MAX_RECORD_BYTES {
        return Err(unreadable(path, "is larger than 1 MiB"));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| unreadable(path, &format!("cannot be read: {error}")))?;
    let Some(front) = quoin_quire::validate::frontmatter(&text) else {
        return Ok(None);
    };
    let text_of = |key: &str| front.get(key).and_then(serde_json::Value::as_str);
    if text_of("type") != Some(OBJECT) {
        return Ok(None);
    }
    Ok(text_of("purl").map(|purl| Existing {
        path: path.to_path_buf(),
        purl: purl.to_owned(),
        id: text_of("id").map(str::to_owned),
        disposition_clause: text_of("disposition_clause").map(str::to_owned),
    }))
}

/// Check every destination before anything is written. Returns the purls whose
/// destination already holds their own record, which are unchanged.
fn check_destinations(planned: &Plan, out: &Path) -> Result<BTreeSet<String>, Stop> {
    let mut adopted = BTreeSet::new();
    let mut claimed: BTreeMap<&str, &str> = BTreeMap::new();
    for record in &planned.created {
        let path = out.join(&record.file);
        if record.file.len() > MAX_FILE_NAME {
            return Err(refuse(
                "soup.file_name_too_long",
                format!("{} is longer than {MAX_FILE_NAME} bytes", path.display()),
                &[("path", path.display().to_string())],
            ));
        }
        if let Some(other) = claimed.insert(record.file.as_str(), record.purl.as_str()) {
            return Err(occupied(
                &path,
                &format!("{other} and {} share it", record.purl),
            ));
        }
        if path.exists() {
            match read_record(&path)? {
                Some(existing) if existing.purl == record.purl => {
                    adopted.insert(record.purl.clone());
                }
                _ => return Err(occupied(&path, "is not the soup_record for the purl")),
            }
        }
    }
    Ok(adopted)
}

fn occupied(path: &Path, why: &str) -> Stop {
    refuse(
        "soup.destination_occupied",
        format!("{} {why}", path.display()),
        &[("path", path.display().to_string())],
    )
}

/// Create each new record and return where each landed. Every destination was
/// checked first, and `create_new` still refuses to replace a file.
fn write_new(
    planned: &Plan,
    skeleton: &str,
    out: &Path,
    first_id: u32,
    adopted: &BTreeSet<String>,
    clause: Option<&str>,
) -> Result<BTreeMap<String, PathBuf>, Stop> {
    let mut written = BTreeMap::new();
    let fresh: Vec<&NewRecord> = planned
        .created
        .iter()
        .filter(|record| !adopted.contains(&record.purl))
        .collect();
    if fresh.is_empty() {
        return Ok(written);
    }
    std::fs::create_dir_all(out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    for (offset, record) in fresh.into_iter().enumerate() {
        let path = out.join(&record.file);
        let number = usize::try_from(first_id)
            .unwrap_or(usize::MAX)
            .saturating_add(offset);
        let id = format!("SOUP-{number:03}");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
        file.write_all(plan::render(skeleton, &id, record, clause).as_bytes())
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        written.insert(record.purl.clone(), path);
    }
    Ok(written)
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
