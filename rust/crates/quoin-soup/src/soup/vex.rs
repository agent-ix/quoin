// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin soup vex export`: serialize authored SOUP statements as OpenVEX
//! 0.2.0 (FR-216). No scan, advisory or vulnerability lifecycle is read.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use clap::ArgMatches;
use quoin_core::protocol::Response;
use serde_json::{Map, Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use url::Url;

use super::{Existing, Stop, arg, read_all_records, refuse};

const OPENVEX_CONTEXT: &str = "https://openvex.dev/ns/v0.2.0";
const STATUSES: [&str; 4] = ["affected", "not_affected", "under_investigation", "fixed"];
const JUSTIFICATIONS: [&str; 5] = [
    "component_not_present",
    "vulnerable_code_not_present",
    "vulnerable_code_not_in_execute_path",
    "vulnerable_code_cannot_be_controlled_by_adversary",
    "inline_mitigations_already_exist",
];

struct Metadata {
    id: String,
    author: String,
    timestamp: String,
    version: u64,
    role: Option<String>,
}

/// Run the OpenVEX exporter after Clap has parsed its arguments.
pub(super) fn run(arguments: &ArgMatches) -> Result<Response, Stop> {
    let repo = PathBuf::from(arg(arguments, "repo").unwrap_or("."));
    let records_dir =
        arg(arguments, "records").map_or_else(|| repo.join("spec/soup"), PathBuf::from);
    let output = arg(arguments, "out")
        .map_or_else(|| repo.join("spec/soup.vex.openvex.json"), PathBuf::from);
    let metadata = metadata(arguments)?;
    let records = read_all_records(&records_dir)?;
    let document = render(&records, &metadata)?;
    write_atomic(&output, &document)?;

    let statements = document
        .get("statements")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    Ok(Response::ok(json!({
        "path": output.display().to_string(),
        "statements": statements,
    })))
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

struct Statement {
    advisory: String,
    status: String,
    justification: Option<String>,
    impact_statement: Option<String>,
    action_statement: Option<String>,
}

impl Statement {
    fn json(&self, purl: &str) -> Value {
        let mut statement = Map::new();
        statement.insert("vulnerability".to_owned(), json!({"name": self.advisory}));
        statement.insert("status".to_owned(), json!(self.status));
        statement.insert("products".to_owned(), json!([{"@id": purl}]));
        for (field, value) in [
            ("justification", &self.justification),
            ("impact_statement", &self.impact_statement),
            ("action_statement", &self.action_statement),
        ] {
            if let Some(value) = value {
                statement.insert(field.to_owned(), json!(value));
            }
        }
        Value::Object(statement)
    }
}

fn source_statement(record: &Existing, source: &Value) -> Result<Statement, Stop> {
    let Some(object) = source.as_object() else {
        return Err(source_invalid(
            record,
            "vex_statements",
            "entries must be objects",
        ));
    };
    let required = |field: &str| -> Result<String, Stop> {
        match object.get(field) {
            Some(Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
            _ => Err(source_invalid(record, field, "must be non-blank text")),
        }
    };
    let optional = |field: &str| -> Result<Option<String>, Stop> {
        match object.get(field) {
            None => Ok(None),
            Some(Value::String(value)) => Ok(Some(value.clone())),
            _ => Err(source_invalid(record, field, "must be text when present")),
        }
    };
    let advisory = required("advisory")?;
    let status = required("status")?;
    if !STATUSES.contains(&status.as_str()) {
        return Err(source_invalid(record, "status", "is not an OpenVEX status"));
    }
    let justification = optional("justification")?;
    if let Some(value) = &justification
        && !JUSTIFICATIONS.contains(&value.as_str())
    {
        return Err(source_invalid(
            record,
            "justification",
            "is not an OpenVEX justification",
        ));
    }
    let impact_statement = optional("impact_statement")?;
    let action_statement = optional("action_statement")?;
    if status == "not_affected" && justification.is_none() && impact_statement.is_none() {
        return Err(source_invalid(
            record,
            "justification",
            "not_affected requires justification or impact_statement",
        ));
    }
    if status == "affected" && action_statement.is_none() {
        return Err(source_invalid(
            record,
            "action_statement",
            "affected requires an action statement",
        ));
    }
    Ok(Statement {
        advisory,
        status,
        justification,
        impact_statement,
        action_statement,
    })
}

fn source_invalid(record: &Existing, field: &str, reason: &str) -> Stop {
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
fn write_atomic(output: &Path, document: &Value) -> Result<(), Stop> {
    let mut bytes = serde_json::to_vec_pretty(document)
        .map_err(|error| Stop::Fail(format!("cannot serialize OpenVEX document: {error}")))?;
    bytes.push(b'\n');
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|error| Stop::Fail(format!("cannot create {}: {error}", parent.display())))?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| Stop::Fail(format!("cannot stage {}: {error}", output.display())))?;
    staged
        .write_all(&bytes)
        .map_err(|error| Stop::Fail(format!("cannot write staged OpenVEX document: {error}")))?;
    staged
        .as_file()
        .sync_all()
        .map_err(|error| Stop::Fail(format!("cannot flush staged OpenVEX document: {error}")))?;
    staged.persist(output).map_err(|error| {
        Stop::Fail(format!(
            "cannot replace {}: {}",
            output.display(),
            error.error
        ))
    })?;
    Ok(())
}
