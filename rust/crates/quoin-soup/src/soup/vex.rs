// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin soup vex export`: serialize authored SOUP statements as `OpenVEX`
//! 0.2.0 (FR-216). No scan, advisory or vulnerability lifecycle is read.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use clap::ArgMatches;
use ix_trace_rs::implements;
use quoin_core::protocol::Response;
use quoin_quire::records::Contract;
use quoin_quire::{ModuleSelection, records};
use serde::Serialize;
use serde_json::{Map, Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use url::Url;

use super::{Existing, Stop, arg, read_all_records, refuse};

const OPENVEX_CONTEXT: &str = "https://openvex.dev/ns/v0.2.0";

struct Metadata {
    id: String,
    author: String,
    timestamp: String,
    version: u64,
    role: Option<String>,
}

/// Run the `OpenVEX` exporter after Clap has parsed its arguments.
#[implements("FR-156-AC-10")]
pub(super) fn run(arguments: &ArgMatches) -> Result<Response, Stop> {
    match arg(arguments, "format").unwrap_or("openvex") {
        "openvex" => run_openvex(arguments),
        "cyclonedx" => super::cyclonedx::run(arguments),
        other => Err(Stop::Fail(format!("unsupported VEX format {other}"))),
    }
}

fn run_openvex(arguments: &ArgMatches) -> Result<Response, Stop> {
    let repo = PathBuf::from(arg(arguments, "repo").unwrap_or("."));
    let records_dir =
        arg(arguments, "records").map_or_else(|| repo.join("spec/soup"), PathBuf::from);
    let output = arg(arguments, "out")
        .map_or_else(|| repo.join("spec/soup.vex.openvex.json"), PathBuf::from);
    let metadata = metadata(arguments)?;
    validate_source_contract(&records_dir)?;
    let records = read_all_records(&records_dir)?;
    let document = render(&records, &metadata)?;
    write_atomic(&output, &document, "OpenVEX")?;

    let statements = document
        .get("statements")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    Ok(Response::ok(json!({
        "path": output.display().to_string(),
        "statements": statements,
        "rendered": format!(
            "Wrote OpenVEX 0.2.0 document to {} ({statements} statements)",
            output.display()
        ),
    })))
}

/// Validate authored SOUP statements through the owning module's contract.
///
/// The module selection follows Quire's ambient resolution path, which reads
/// `IX_FILAMENT_MODULES_PATH` and the installed default module set. This keeps
/// the export aligned with `quire validate` and the current `SoupRecord`
/// schema instead of maintaining a second statement schema here.
pub(super) fn validate_source_contract(records_dir: &Path) -> Result<(), Stop> {
    let corpus = records::read(
        records_dir,
        &ModuleSelection::Ambient,
        &["soup_record"],
    )
    .map_err(|error| {
        Stop::Fail(format!(
            "cannot validate SOUP records in {} against the spec-objects-security contract: {error}",
            records_dir.display()
        ))
    })?;
    for document in &corpus.documents {
        if document.kind == "soup_record"
            && let Contract::Violated(reason) = &document.contract
        {
            return Err(refuse(
                "soup.vex_statement_invalid",
                format!(
                    "{} does not satisfy the spec-objects-security `soup_record` contract: {reason}",
                    document.path.display()
                ),
                &[("path", document.path.display().to_string())],
            ));
        }
    }
    Ok(())
}

fn metadata(arguments: &ArgMatches) -> Result<Metadata, Stop> {
    let required = |name: &str| {
        arg(arguments, name)
            .map(str::to_owned)
            .ok_or_else(|| invalid_metadata(name, "is required"))
    };
    let id = required("id")?;
    if Url::parse(&id).is_err() {
        return Err(invalid_metadata("id", "must be an absolute IRI"));
    }
    let author = required("author")?;
    if author.trim().is_empty() {
        return Err(invalid_metadata("author", "must not be blank"));
    }
    let timestamp = required("timestamp")?;
    if OffsetDateTime::parse(&timestamp, &Rfc3339).is_err() {
        return Err(invalid_metadata("timestamp", "must be RFC 3339"));
    }
    let version_text = required("version")?;
    let version = version_text
        .parse::<u64>()
        .ok()
        .filter(|version| *version > 0)
        .ok_or_else(|| invalid_metadata("version", "must be a positive integer"))?;
    let role = arg(arguments, "role").map(str::to_owned);
    Ok(Metadata {
        id,
        author,
        timestamp,
        version,
        role,
    })
}

fn invalid_metadata(field: &str, reason: &str) -> Stop {
    refuse(
        "soup.vex_metadata_invalid",
        format!("OpenVEX document {field} {reason}"),
        &[("field", field.to_owned())],
    )
}

/// Render a complete document in memory, refusing before any output file is
/// created or replaced.
fn render(records: &[Existing], metadata: &Metadata) -> Result<Value, Stop> {
    let mut statements = Vec::new();
    let mut unique_statements = BTreeSet::new();
    for record in records {
        let Some(source) = record.front.get("vex_statements") else {
            continue;
        };
        let Some(sources) = source.as_array() else {
            return Err(source_invalid(record, "vex_statements", "must be a list"));
        };
        if sources.is_empty() {
            continue;
        }
        let mut advisories = BTreeSet::new();
        if quoin_evidence::purl::PurlIdentity::parse(&record.purl).is_none() {
            return Err(source_invalid(
                record,
                "purl",
                "must be a canonical package URL",
            ));
        }
        // Parsing also checks that the source purl is an absolute URI. Its
        // original spelling remains the OpenVEX product identity.
        if Url::parse(&record.purl).is_err() {
            return Err(source_invalid(
                record,
                "purl",
                "must be a canonical package URL",
            ));
        }
        for source in sources {
            let statement = source_statement(record, source)?;
            if !advisories.insert(statement.advisory.clone()) {
                return Err(refuse(
                    "soup.vex_duplicate_advisory",
                    format!(
                        "{} has duplicate advisory {:?} in `vex_statements`; keep one authored statement per advisory in this record",
                        record.path.display(),
                        statement.advisory
                    ),
                    &[
                        ("path", record.path.display().to_string()),
                        ("field", "vex_statements".to_owned()),
                        ("advisory", statement.advisory),
                    ],
                ));
            }
            let mapped = statement.json(&record.purl);
            let identity = serde_json::to_string(&mapped).map_err(|error| {
                Stop::Fail(format!("cannot serialize OpenVEX statement: {error}"))
            })?;
            if !unique_statements.insert(identity) {
                return Err(source_invalid(
                    record,
                    "vex_statements",
                    "duplicates an identical OpenVEX statement already selected",
                ));
            }
            statements.push(mapped);
        }
    }
    if statements.is_empty() {
        return Err(refuse(
            "soup.vex_empty_statements",
            "OpenVEX 0.2.0 requires at least one statement; add an authored vex_statements entry or select records that contain one".to_owned(),
            &[("field", "vex_statements".to_owned())],
        ));
    }

    let mut document = Map::new();
    document.insert("@context".to_owned(), json!(OPENVEX_CONTEXT));
    document.insert("@id".to_owned(), json!(metadata.id));
    document.insert("author".to_owned(), json!(metadata.author));
    if let Some(role) = &metadata.role {
        document.insert("role".to_owned(), json!(role));
    }
    document.insert("timestamp".to_owned(), json!(metadata.timestamp));
    document.insert("version".to_owned(), json!(metadata.version));
    document.insert("statements".to_owned(), json!(statements));
    Ok(Value::Object(document))
}

impl SourceStatement {
    #[implements("FR-216-AC-1", "FR-216-AC-2")]
    fn json(&self, purl: &str) -> Value {
        let mut statement = Map::new();
        statement.insert("vulnerability".to_owned(), json!({"name": self.advisory}));
        statement.insert("status".to_owned(), json!(self.status));
        statement.insert("products".to_owned(), json!([{"@id": purl}]));
        for (field, value) in [
            ("justification", &self.justification),
            ("impact_statement", &self.impact),
            ("action_statement", &self.action),
        ] {
            if let Some(value) = value {
                statement.insert(field.to_owned(), json!(value));
            }
        }
        Value::Object(statement)
    }
}

pub(super) struct SourceStatement {
    pub(super) advisory: String,
    pub(super) status: String,
    pub(super) justification: Option<String>,
    pub(super) impact: Option<String>,
    pub(super) action: Option<String>,
    pub(super) assessed_by: String,
    pub(super) assessed_at: String,
    pub(super) evidence_refs: Vec<String>,
}

#[implements("FR-156-AC-3", "FR-156-AC-4", "FR-216-AC-6")]
pub(super) fn source_statement(record: &Existing, source: &Value) -> Result<SourceStatement, Stop> {
    let Some(object) = source.as_object() else {
        return Err(source_invalid(
            record,
            "vex_statements",
            "entries must be objects",
        ));
    };
    let required = |field: &str| -> Result<String, Stop> {
        object
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                source_invalid(
                    record,
                    field,
                    "must be text as required by the source contract",
                )
            })
    };
    let optional = |field: &str| -> Result<Option<String>, Stop> {
        match object.get(field) {
            None => Ok(None),
            Some(Value::String(value)) => Ok(Some(value.clone())),
            _ => Err(source_invalid(
                record,
                field,
                "must be text as required by the source contract",
            )),
        }
    };
    let advisory = required("advisory")?;
    let status = required("status")?;
    let justification = optional("justification")?;
    let impact_statement = optional("impact_statement")?;
    let action_statement = optional("action_statement")?;
    let evidence_refs = match object.get("evidence_refs") {
        None => Vec::new(),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| source_invalid(record, "evidence_refs", "entries must be text"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(source_invalid(record, "evidence_refs", "must be a list")),
    };
    Ok(SourceStatement {
        advisory,
        status,
        justification,
        impact: impact_statement,
        action: action_statement,
        assessed_by: required("assessed_by")?,
        assessed_at: required("assessed_at")?,
        evidence_refs,
    })
}

pub(super) fn source_invalid(record: &Existing, field: &str, reason: &str) -> Stop {
    refuse(
        "soup.vex_statement_invalid",
        format!("{} has invalid {field}: {reason}", record.path.display()),
        &[
            ("path", record.path.display().to_string()),
            ("field", field.to_owned()),
        ],
    )
}

/// Persist complete bytes with a same-directory temporary file and atomic
/// rename, after all source validation and serialization has succeeded.
pub(super) fn write_atomic<T: Serialize>(
    output: &Path,
    document: &T,
    format: &str,
) -> Result<(), Stop> {
    let mut bytes = serde_json::to_vec_pretty(document)
        .map_err(|error| Stop::Fail(format!("cannot serialize {format} document: {error}")))?;
    bytes.push(b'\n');
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|error| Stop::Fail(format!("cannot create {}: {error}", parent.display())))?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        Stop::Fail(format!(
            "cannot stage {format} document at {}: {error}",
            output.display()
        ))
    })?;
    staged
        .write_all(&bytes)
        .map_err(|error| Stop::Fail(format!("cannot write staged {format} document: {error}")))?;
    staged
        .as_file()
        .sync_all()
        .map_err(|error| Stop::Fail(format!("cannot flush staged {format} document: {error}")))?;
    staged.persist(output).map_err(|error| {
        Stop::Fail(format!(
            "cannot replace {}: {}",
            output.display(),
            error.error
        ))
    })?;
    Ok(())
}
