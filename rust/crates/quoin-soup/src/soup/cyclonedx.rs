// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `CycloneDX` 1.7 VEX export for authored SOUP statements (FR-156).
//!
//! The Syft SBOM is a path hint only. Statements and component facts come
//! exclusively from validated `soup_record` documents; retained advisory scans
//! are read only to label an authored statement stale.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use clap::ArgMatches;
use ix_trace_rs::implements;
use packageurl::PackageUrl;
use quoin_core::protocol::Response;
use quoin_evidence::DiskEvidence;
use quoin_evidence::purl::PurlIdentity;
use quoin_evidence::store::latest_scans;
use serde::Serialize;
use serde_json::{Value, json};

use super::advisory::{self, VexScanState};
use super::{Existing, Stop, arg, read_all_records, refuse};

const FORMAT_VERSION: &str = "1.7";
const STALE_PROPERTY: &str = "quoin:vex:stale";

/// Run the `CycloneDX` exporter after Clap has parsed its arguments.
#[implements("FR-156-AC-5", "FR-156-AC-10")]
pub(super) fn run(arguments: &ArgMatches) -> Result<Response, Stop> {
    let repo = PathBuf::from(arg(arguments, "repo").unwrap_or("."));
    let records_dir =
        arg(arguments, "records").map_or_else(|| repo.join("spec/soup"), PathBuf::from);
    let sbom = arg(arguments, "sbom").map(PathBuf::from);
    let output = match arg(arguments, "out") {
        Some(path) => PathBuf::from(path),
        None => default_output(sbom.as_deref())?,
    };

    super::vex::validate_source_contract(&records_dir)?;
    let mut records = read_all_records(&records_dir)?;
    if let Some((first, second)) = super::plan::duplicate_records(&records) {
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
    records.sort_by(|left, right| (&left.purl, &left.path).cmp(&(&right.purl, &right.path)));

    let mut unreadable_scans = Vec::new();
    let scans = latest_scans(&DiskEvidence::new(&repo), &mut unreadable_scans)
        .map_err(|error| format!("cannot read retained advisory scans: {error}"))?;
    let scan_states = if unreadable_scans.is_empty() {
        advisory::vex_scan_states(&records, &scans)
    } else {
        // An unreadable retained scan makes currentness unknown. It cannot be
        // used to turn a person-authored statement into a stale one.
        vec![VexScanState::default(); records.len()]
    };
    let document = render(&records, &scan_states)?;
    super::vex::write_atomic(&output, &document, "CycloneDX")?;

    let vulnerabilities = document.vulnerabilities.as_ref().map_or(0, Vec::len);
    Ok(Response::ok(json!({
        "path": output.display().to_string(),
        "vulnerabilities": vulnerabilities,
        "rendered": format!(
            "Wrote CycloneDX 1.7 VEX to {} ({vulnerabilities} vulnerabilities)",
            output.display()
        ),
    })))
}

#[implements("FR-156-AC-10")]
fn default_output(sbom: Option<&Path>) -> Result<PathBuf, Stop> {
    let Some(sbom) = sbom else {
        return Err(refuse(
            "soup.vex_sbom_required",
            "--sbom is required for CycloneDX export when --out is omitted".to_owned(),
            &[("field", "sbom".to_owned())],
        ));
    };
    let file_name = sbom
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            refuse(
                "soup.vex_sbom_invalid",
                "--sbom must name a file with a UTF-8 file name".to_owned(),
                &[("field", "sbom".to_owned())],
            )
        })?;
    let output_name = file_name.strip_suffix(".cdx.json").map_or_else(
        || format!("{file_name}.vex.cdx.json"),
        |stem| format!("{stem}.vex.cdx.json"),
    );
    Ok(sbom.with_file_name(output_name))
}

#[derive(Serialize)]
struct Bom {
    #[serde(rename = "bomFormat")]
    format: &'static str,
    #[serde(rename = "specVersion")]
    spec_version: &'static str,
    version: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    components: Vec<Component>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vulnerabilities: Option<Vec<Vulnerability>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Component {
    #[serde(rename = "bom-ref")]
    bom_ref: String,
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    name: String,
    purl: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Vulnerability {
    id: String,
    analysis: Analysis,
    affects: Vec<AffectedComponent>,
    properties: Vec<Property>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recommendation: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Analysis {
    state: AnalysisState,
    #[serde(skip_serializing_if = "Option::is_none")]
    justification: Option<AnalysisJustification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum AnalysisState {
    Exploitable,
    NotAffected,
    InTriage,
    Resolved,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum AnalysisJustification {
    CodeNotPresent,
    CodeNotReachable,
    ProtectedByMitigatingControl,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AffectedComponent {
    #[serde(rename = "ref")]
    component_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    versions: Option<Vec<AffectedVersion>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AffectedVersion {
    version: String,
    status: AffectedStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum AffectedStatus {
    Affected,
    Unaffected,
    Unknown,
}

#[derive(Serialize)]
struct Property {
    name: String,
    value: String,
}

#[implements("FR-156-AC-6", "FR-156-AC-9", "FR-156-AC-11")]
fn render(records: &[Existing], scan_states: &[VexScanState]) -> Result<Bom, Stop> {
    let mut components_by_ref = BTreeMap::new();
    let mut vulnerabilities = Vec::new();
    let mut unique_vulnerabilities = BTreeSet::new();
    for (position, record) in records.iter().enumerate() {
        let Some(items) = record.front.get("vex_statements") else {
            continue;
        };
        let Some(items) = items.as_array() else {
            return Err(super::vex::source_invalid(
                record,
                "vex_statements",
                "must be a list",
            ));
        };
        if items.is_empty() {
            continue;
        }
        let component = component(record)?;
        let state = scan_states.get(position).cloned().unwrap_or_default();
        components_by_ref.insert(component.bom_ref.clone(), component);
        for item in items {
            let statement = super::vex::source_statement(record, item)?;
            let vulnerability = vulnerability(record, &statement, &state)?;
            let unique = serde_json::to_vec(&vulnerability).map_err(|error| {
                Stop::Fail(format!("cannot compare CycloneDX entries: {error}"))
            })?;
            if !unique_vulnerabilities.insert(unique) {
                return Err(super::vex::source_invalid(
                    record,
                    "vex_statements",
                    "duplicate statements produce identical CycloneDX entries, which the schema forbids",
                ));
            }
            vulnerabilities.push(vulnerability);
        }
    }
    vulnerabilities.sort_by(|left, right| {
        let left_component = left
            .affects
            .first()
            .map_or("", |affected| affected.component_ref.as_str());
        let right_component = right
            .affects
            .first()
            .map_or("", |affected| affected.component_ref.as_str());
        (&left.id, left_component).cmp(&(&right.id, right_component))
    });
    let vulnerabilities = (!vulnerabilities.is_empty()).then_some(vulnerabilities);
    Ok(Bom {
        format: "CycloneDX",
        spec_version: FORMAT_VERSION,
        version: 1,
        components: components_by_ref.into_values().collect(),
        vulnerabilities,
    })
}

#[implements("FR-156-AC-1", "FR-156-AC-8")]
fn component(record: &Existing) -> Result<Component, Stop> {
    let identity = PurlIdentity::parse(&record.purl).ok_or_else(|| {
        super::vex::source_invalid(record, "purl", "must be a canonical package URL")
    })?;
    let parsed = PackageUrl::from_str(&record.purl).map_err(|_| {
        super::vex::source_invalid(
            record,
            "purl",
            "must follow the Package-URL type definition",
        )
    })?;
    // packageurl 0.7.1 lowercases npm names, but the current npm PURL type
    // definition marks both namespace and name case-sensitive. Keep the
    // decoded source components for npm and use packageurl's normalization
    // for the other PURL types.
    let (group, name) = if identity.kind == "npm" {
        (
            (!identity.namespace.is_empty()).then_some(identity.namespace),
            identity.name,
        )
    } else if identity.kind == "pypi" {
        (None, normalize_pypi_name(&identity.name))
    } else {
        (
            parsed.namespace().map(str::to_owned),
            parsed.name().to_owned(),
        )
    };
    Ok(Component {
        bom_ref: record.purl.clone(),
        kind: "library",
        group,
        name,
        purl: record.purl.clone(),
    })
}

#[implements("FR-156-AC-8")]
fn normalize_pypi_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut previous_was_separator = false;
    for character in name.chars() {
        if matches!(character, '-' | '_' | '.') {
            if !previous_was_separator {
                normalized.push('-');
            }
            previous_was_separator = true;
        } else {
            previous_was_separator = false;
            normalized.extend(character.to_lowercase());
        }
    }
    normalized
}

#[implements(
    "FR-156-AC-2",
    "FR-156-AC-3",
    "FR-156-AC-4",
    "FR-156-AC-7",
    "FR-156-AC-12"
)]
fn vulnerability(
    record: &Existing,
    statement: &super::vex::SourceStatement,
    scan_state: &VexScanState,
) -> Result<Vulnerability, Stop> {
    let (state, version_status) = match statement.status.as_str() {
        "affected" => (AnalysisState::Exploitable, AffectedStatus::Affected),
        "not_affected" => (AnalysisState::NotAffected, AffectedStatus::Unaffected),
        "under_investigation" => (AnalysisState::InTriage, AffectedStatus::Unknown),
        "fixed" => (AnalysisState::Resolved, AffectedStatus::Unaffected),
        _ => {
            return Err(super::vex::source_invalid(
                record,
                "status",
                "is not a supported VEX status",
            ));
        }
    };
    let justification = match statement.justification.as_deref() {
        Some("vulnerable_code_not_present") => Some(AnalysisJustification::CodeNotPresent),
        Some("vulnerable_code_not_in_execute_path") => {
            Some(AnalysisJustification::CodeNotReachable)
        }
        Some("inline_mitigations_already_exist") => {
            Some(AnalysisJustification::ProtectedByMitigatingControl)
        }
        Some("component_not_present" | "vulnerable_code_cannot_be_controlled_by_adversary")
        | None => None,
        Some(_) => {
            return Err(super::vex::source_invalid(
                record,
                "justification",
                "is not a supported VEX justification",
            ));
        }
    };
    let component_version = record
        .front
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            PackageUrl::from_str(&record.purl)
                .ok()
                .and_then(|purl| purl.version().map(str::to_owned))
        });
    if component_version
        .as_ref()
        .is_some_and(|version| version.chars().count() > 1024)
    {
        return Err(super::vex::source_invalid(
            record,
            "version",
            "exceeds CycloneDX 1.7's 1024-character affected-version limit",
        ));
    }
    let versions = component_version.map(|version| {
        vec![AffectedVersion {
            version,
            status: version_status,
        }]
    });
    let mut properties = vec![Property {
        name: "quoin:vex:source-status".to_owned(),
        value: statement.status.clone(),
    }];
    if let Some(justification) = &statement.justification {
        properties.push(Property {
            name: "quoin:vex:justification".to_owned(),
            value: justification.clone(),
        });
    }
    properties.push(Property {
        name: "quoin:vex:assessed-by".to_owned(),
        value: statement.assessed_by.clone(),
    });
    properties.push(Property {
        name: "quoin:vex:assessed-at".to_owned(),
        value: statement.assessed_at.clone(),
    });
    properties.extend(statement.evidence_refs.iter().map(|reference| Property {
        name: "quoin:vex:evidence-ref".to_owned(),
        value: reference.clone(),
    }));
    if scan_state.is_stale(&record.purl, &statement.advisory) {
        properties.push(Property {
            name: STALE_PROPERTY.to_owned(),
            value: "true".to_owned(),
        });
    }
    Ok(Vulnerability {
        id: statement.advisory.clone(),
        analysis: Analysis {
            state,
            justification,
            detail: statement.impact.clone(),
        },
        affects: vec![AffectedComponent {
            component_ref: record.purl.clone(),
            versions,
        }],
        properties,
        recommendation: statement.action.clone(),
    })
}
