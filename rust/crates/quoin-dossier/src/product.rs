// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Product dossier documents and their closed JSON source contract (FR-229–233).

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_json::Value;

use crate::{
    error::SourceFailure,
    gap::{Gap, GapKind},
    markdown::{cell, prose},
    registry::{DocumentSpec, Generated, Generator, Inputs},
};

const SW_SCHEMA: &str = include_str!("../schemas/software-description-source-v1.schema.json");
const DL_SCHEMA: &str =
    include_str!("../schemas/documentation-level-decision-source-v1.schema.json");
const DEV_SCHEMA: &str = include_str!("../schemas/device-description-source-v1.schema.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Software {
    schema_version: u8,
    id: String,
    record_type: String,
    #[serde(rename = "software_id")]
    identity: String,
    name: String,
    purpose: Option<String>,
    functions: Option<Vec<String>>,
    operating_environment: Option<String>,
    interfaces: Option<Vec<String>>,
    architecture_part_ids: Option<Vec<String>>,
    architecture_decision_ids: Option<Vec<String>>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    schema_version: u8,
    id: String,
    record_type: String,
    software_id: String,
    level: Option<String>,
    decided_by: Option<String>,
    decided_on: Option<String>,
    rationale: Option<String>,
    source_records: Option<Vec<SourceRef>>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Device {
    schema_version: u8,
    id: String,
    record_type: String,
    #[serde(rename = "device_id")]
    identity: String,
    name: String,
    purpose: Option<String>,
    intended_use: Option<String>,
    operating_context: Option<String>,
    software_ids: Option<Vec<String>>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRef {
    id: String,
    record_type: String,
}

#[derive(Debug, Default)]
struct ProductRecords {
    software: Vec<Software>,
    decisions: Vec<Decision>,
    devices: Vec<Device>,
}

fn fail(path: &Path, reason: impl std::fmt::Display) -> SourceFailure {
    SourceFailure {
        source: path.display().to_string(),
        detail: reason.to_string(),
    }
}

fn read_kind<T: for<'de> Deserialize<'de>>(
    scope: &Path,
    kind: &str,
    schema: &str,
    record_type: &str,
) -> Result<Vec<T>, SourceFailure> {
    let dir = scope.join("dossier/sources").join(kind);
    for parent in [
        scope.join("dossier"),
        scope.join("dossier/sources"),
        dir.clone(),
    ] {
        match fs::symlink_metadata(&parent) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(fail(
                    &parent,
                    "source directory must be a directory within the selected scope",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(fail(&parent, error)),
        }
    }
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(fail(&dir, error)),
    };
    let mut paths = entries
        .map(|entry| entry.map(|e| e.path()).map_err(|e| fail(&dir, e)))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let schema_value: Value = serde_json::from_str(schema)
        .map_err(|e| fail(&dir, format!("invalid bundled schema: {e}")))?;
    let validator = quoin_jsonschema::SchemaValidator::compile_vendored(&dir, &schema_value, &[])
        .map_err(|e| fail(&dir, e))?;
    let mut records = Vec::new();
    let mut ids = BTreeSet::new();
    for path in paths {
        let metadata = fs::symlink_metadata(&path).map_err(|error| fail(&path, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(fail(
                &path,
                "source record must be a regular file within the selected scope",
            ));
        }
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            return Err(fail(&path, "source record must be a .json file"));
        }
        let bytes = fs::read(&path).map_err(|e| fail(&path, e))?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|e| fail(&path, format!("invalid JSON: {e}")))?;
        if let Some(error) = validator.errors(&value).first() {
            return Err(fail(
                &path,
                format!(
                    "schema validation failed at {} ({:?})",
                    error.instance_path, error.keyword
                ),
            ));
        }
        let id = value
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| fail(&path, "id is missing"))?;
        let record_kind = value
            .get("record_type")
            .and_then(Value::as_str)
            .ok_or_else(|| fail(&path, "record_type is missing"))?;
        if record_kind != record_type || path.file_stem().and_then(|s| s.to_str()) != Some(id) {
            return Err(fail(&path, "path, record_type, and id do not agree"));
        }
        if !ids.insert(id.to_owned()) {
            return Err(fail(&path, format!("duplicate {record_type} id {id}")));
        }
        records.push(serde_json::from_value(value).map_err(|e| fail(&path, e))?);
    }
    Ok(records)
}

fn read_all(scope: &Path) -> Result<ProductRecords, SourceFailure> {
    read_records(scope, true, true, true)
}

fn read_records(
    scope: &Path,
    include_software: bool,
    include_decisions: bool,
    include_devices: bool,
) -> Result<ProductRecords, SourceFailure> {
    let software: Vec<Software> = if include_software {
        read_kind(
            scope,
            "software_description",
            SW_SCHEMA,
            "software_description",
        )?
    } else {
        Vec::new()
    };
    let decisions: Vec<Decision> = if include_decisions {
        read_kind(
            scope,
            "documentation_level_decision",
            DL_SCHEMA,
            "documentation_level_decision",
        )?
    } else {
        Vec::new()
    };
    let devices: Vec<Device> = if include_devices {
        read_kind(
            scope,
            "device_description",
            DEV_SCHEMA,
            "device_description",
        )?
    } else {
        Vec::new()
    };
    let mut identities = BTreeSet::new();
    for r in &software {
        if r.schema_version != 1
            || r.record_type != "software_description"
            || !identities.insert(r.identity.clone())
        {
            return Err(fail(
                &scope.join("dossier/sources/software_description"),
                "invalid version/type or duplicate software_id",
            ));
        }
    }
    identities.clear();
    for r in &devices {
        if r.schema_version != 1
            || r.record_type != "device_description"
            || !identities.insert(r.identity.clone())
        {
            return Err(fail(
                &scope.join("dossier/sources/device_description"),
                "invalid version/type or duplicate device_id",
            ));
        }
    }
    if decisions.iter().any(|r| {
        r.schema_version != 1
            || r.record_type != "documentation_level_decision"
            || r.decided_on
                .as_deref()
                .is_some_and(|date| !valid_date(date))
    }) {
        return Err(fail(
            &scope.join("dossier/sources/documentation_level_decision"),
            "invalid version, record_type, or decided_on calendar date",
        ));
    }
    Ok(ProductRecords {
        software,
        decisions,
        devices,
    })
}

fn valid_date(value: &str) -> bool {
    let mut parts = value.split('-');
    let year = parts.next().and_then(|v| v.parse::<i32>().ok());
    let month = parts.next().and_then(|v| v.parse::<u8>().ok());
    let day = parts.next().and_then(|v| v.parse::<u8>().ok());
    matches!((year, month, day, parts.next()), (Some(y), Some(m), Some(d), None) if time::Date::from_calendar_date(y, time::Month::try_from(m).unwrap_or(time::Month::January), d).is_ok() && value.len() == 10)
}

fn label(
    body: &mut String,
    name: &str,
    value: Option<&str>,
    record: &str,
    gaps: &mut Vec<Gap>,
    gap: (GapKind, &str),
) {
    if let Some(value) = value {
        let _ = writeln!(
            body,
            "**{name}:** {} _(source: `{}`)_\n",
            prose(value),
            cell(record)
        );
    } else {
        let _ = writeln!(body, "**{name}:** Not stated.\n");
        gaps.push(Gap::new(
            gap.0,
            gap.1,
            format!("{name} is not stated in source record {record}"),
        ));
    }
}

pub(crate) struct SoftwareDescription;
impl Generator for SoftwareDescription {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "software-description",
            id: "DOS-SW",
            title: "Software description",
            needs_as_of: false,
        }
    }
    fn generate(&self, i: &Inputs) -> Result<Generated, SourceFailure> {
        let r = read_records(&i.scope, true, false, false)?;
        Ok(render_sw(&r.software))
    }
}
pub(crate) struct DocumentationDecision;
impl Generator for DocumentationDecision {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "documentation-level-decision",
            id: "DOS-DL",
            title: "Documentation level decision",
            needs_as_of: false,
        }
    }
    fn generate(&self, i: &Inputs) -> Result<Generated, SourceFailure> {
        let r = read_records(&i.scope, true, true, true)?;
        Ok(render_dl(&r))
    }
}
pub(crate) struct SoftwareDesign;
impl Generator for SoftwareDesign {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "software-design-specification",
            id: "DOS-SDS",
            title: "Software design specification",
            needs_as_of: false,
        }
    }
    fn generate(&self, i: &Inputs) -> Result<Generated, SourceFailure> {
        let r = read_records(&i.scope, true, false, false)?;
        render_sds(&r.software, &i.scope, &i.modules)
    }
}
pub(crate) struct DeviceDescription;
impl Generator for DeviceDescription {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "device-description",
            id: "DOS-DEV",
            title: "Device description",
            needs_as_of: false,
        }
    }
    fn generate(&self, i: &Inputs) -> Result<Generated, SourceFailure> {
        let r = read_records(&i.scope, true, false, true)?;
        Ok(render_dev(&r))
    }
}

fn render_sw(records: &[Software]) -> Generated {
    let mut body = String::from("# Software description\n\n");
    let mut gaps = Vec::new();
    if records.is_empty() {
        gaps.push(Gap::new(
            GapKind::NoSoftwareDescriptionSource,
            "DOS-SW",
            "no software description source was selected",
        ));
    }
    for r in records {
        render_sw_section(r, &mut body, &mut gaps);
    }
    body.push_str("## Sources\n\n");
    for r in records {
        let _ = writeln!(body, "- `software_description/{}`", cell(&r.id));
    }
    Generated {
        body,
        document_version: None,
        gaps,
    }
}

fn render_sw_section(r: &Software, body: &mut String, gaps: &mut Vec<Gap>) {
    let _ = writeln!(
        body,
        "## {} — {}\n\n**Source:** `software_description/{}`\n",
        cell(&r.identity),
        cell(&r.name),
        cell(&r.id)
    );
    let s = r.id.as_str();
    label(
        body,
        "Purpose",
        r.purpose.as_deref(),
        s,
        gaps,
        (GapKind::SoftwareDescriptionFieldUnstated, &r.id),
    );
    label(
        body,
        "Operating environment",
        r.operating_environment.as_deref(),
        s,
        gaps,
        (GapKind::SoftwareDescriptionFieldUnstated, &r.id),
    );
    for (field, values) in [
        ("Functions", r.functions.as_deref()),
        ("Interfaces", r.interfaces.as_deref()),
    ] {
        match values {
            Some(v) if !v.is_empty() => {
                let _ = writeln!(body, "**{field}:** _(source: `{}`)_", cell(s));
                for x in v {
                    let _ = writeln!(body, "- {}", prose(x));
                }
                body.push('\n');
            }
            _ => gaps.push(Gap::new(
                GapKind::SoftwareDescriptionFieldUnstated,
                &r.id,
                format!("{field} is not stated in source record {s}"),
            )),
        }
    }
}

fn render_dl(records: &ProductRecords) -> Generated {
    let mut body = String::from("# Documentation level decision\n\n");
    let mut gaps = Vec::new();
    if records.decisions.is_empty() {
        gaps.push(Gap::new(
            GapKind::NoDocumentationLevelDecision,
            "DOS-DL",
            "no documentation-level decision source was selected",
        ));
    }
    let mut decisions: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let index = ProductIndex::new(records);
    for r in &records.decisions {
        let decision_signature = render_decision(r, &index, &mut body, &mut gaps);
        decisions
            .entry(&r.software_id)
            .or_default()
            .insert(decision_signature);
    }
    for (software, levels) in decisions {
        if levels.len() > 1 {
            gaps.push(Gap::new(
                GapKind::ConflictingDocumentationLevelDecisions,
                software,
                "selected decision records state different documentation levels",
            ));
        }
    }
    body.push_str("## Sources\n\n");
    for r in &records.decisions {
        let _ = writeln!(body, "- `documentation_level_decision/{}`", cell(&r.id));
        if let Some(refs) = &r.source_records {
            for src in refs {
                let _ = writeln!(body, "- `{}/{}`", cell(&src.record_type), cell(&src.id));
            }
        }
    }
    Generated {
        body,
        document_version: None,
        gaps,
    }
}

struct ProductIndex<'a> {
    software: BTreeSet<&'a str>,
    source_ids: BTreeSet<(&'a str, &'a str)>,
}

impl<'a> ProductIndex<'a> {
    fn new(records: &'a ProductRecords) -> Self {
        let software = records
            .software
            .iter()
            .map(|r| r.identity.as_str())
            .collect();
        let source_ids = records
            .software
            .iter()
            .map(|r| ("software_description", r.id.as_str()))
            .chain(
                records
                    .decisions
                    .iter()
                    .map(|r| ("documentation_level_decision", r.id.as_str())),
            )
            .chain(
                records
                    .devices
                    .iter()
                    .map(|r| ("device_description", r.id.as_str())),
            )
            .collect();
        Self {
            software,
            source_ids,
        }
    }
}

fn render_decision(
    r: &Decision,
    index: &ProductIndex<'_>,
    body: &mut String,
    gaps: &mut Vec<Gap>,
) -> String {
    let _ = writeln!(
        body,
        "## {}\n\n**Source:** `documentation_level_decision/{}`\n\n**Software:** `{}`\n",
        cell(&r.software_id),
        cell(&r.id),
        cell(&r.software_id)
    );
    if !index.software.contains(r.software_id.as_str()) {
        gaps.push(Gap::new(
            GapKind::SourceReferenceUnresolved,
            &r.id,
            format!(
                "software_id {} expected software_description",
                r.software_id
            ),
        ));
    }
    for (key, value) in [
        ("Level", r.level.as_deref()),
        ("Decided by", r.decided_by.as_deref()),
        ("Decided on", r.decided_on.as_deref()),
        ("Rationale", r.rationale.as_deref()),
    ] {
        label(
            body,
            key,
            value,
            &r.id,
            gaps,
            (GapKind::DocumentationLevelDecisionFieldUnstated, &r.id),
        );
    }
    match &r.source_records {
        Some(rows) if rows.is_empty() => gaps.push(Gap::new(
            GapKind::DocumentationLevelDecisionFieldUnstated,
            &r.id,
            "source_records is empty",
        )),
        Some(rows) => {
            let _ = writeln!(body, "**Referenced sources:** _(from `{}`)_", cell(&r.id));
            for src in rows {
                let _ = writeln!(body, "- `{}/{}`", cell(&src.record_type), cell(&src.id));
                if !index
                    .source_ids
                    .contains(&(src.record_type.as_str(), src.id.as_str()))
                {
                    gaps.push(Gap::new(
                        GapKind::SourceReferenceUnresolved,
                        &r.id,
                        format!("referenced id {} expected {}", src.id, src.record_type),
                    ));
                }
            }
            body.push('\n');
        }
        None => gaps.push(Gap::new(
            GapKind::DocumentationLevelDecisionFieldUnstated,
            &r.id,
            "source_records is not stated",
        )),
    }
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}",
        r.level, r.decided_by, r.decided_on, r.rationale, r.source_records
    )
}

fn render_dev(records: &ProductRecords) -> Generated {
    let mut body = String::from("# Device description\n\n");
    let mut gaps = Vec::new();
    if records.devices.is_empty() {
        gaps.push(Gap::new(
            GapKind::NoDeviceDescriptionSource,
            "DOS-DEV",
            "no device description source was selected",
        ));
    }
    let software: BTreeSet<&str> = records
        .software
        .iter()
        .map(|r| r.identity.as_str())
        .collect();
    for r in &records.devices {
        let _ = writeln!(
            body,
            "## {} — {}\n\n**Source:** `device_description/{}`\n",
            cell(&r.identity),
            cell(&r.name),
            cell(&r.id)
        );
        for (k, v) in [
            ("Purpose", r.purpose.as_deref()),
            ("Intended use", r.intended_use.as_deref()),
            ("Operating context", r.operating_context.as_deref()),
        ] {
            label(
                &mut body,
                k,
                v,
                &r.id,
                &mut gaps,
                (GapKind::DeviceFieldUnstated, &r.id),
            );
        }
        match &r.software_ids {
            Some(ids) if !ids.is_empty() => {
                let _ = writeln!(body, "**Software items:** _(source: `{}`)_", cell(&r.id));
                for id in ids {
                    if software.contains(id.as_str()) {
                        let _ = writeln!(body, "- `{}`", cell(id));
                    } else {
                        gaps.push(Gap::new(
                            GapKind::SourceReferenceUnresolved,
                            &r.id,
                            format!("referenced id {id} expected software_description"),
                        ));
                    }
                }
                body.push('\n');
            }
            _ => gaps.push(Gap::new(
                GapKind::DeviceFieldUnstated,
                &r.id,
                "software_ids is absent or empty",
            )),
        }
    }
    body.push_str("## Sources\n\n");
    for r in &records.devices {
        let _ = writeln!(body, "- `device_description/{}`", cell(&r.id));
    }
    Generated {
        body,
        document_version: None,
        gaps,
    }
}

fn render_sds(
    records: &[Software],
    scope: &Path,
    modules: &[PathBuf],
) -> Result<Generated, SourceFailure> {
    let mut body = String::from("# Software design specification\n\n");
    let mut gaps = Vec::new();
    if records.is_empty() {
        gaps.push(Gap::new(
            GapKind::NoSoftwareDesignSource,
            "DOS-SDS",
            "no software design source was selected",
        ));
    }
    let (systems, adr_documents) = if modules.is_empty() {
        (
            quoin_quire::systems::Systems::default(),
            Vec::<quoin_quire::records::Document>::new(),
        )
    } else {
        let loaded = modules
            .iter()
            .map(quoin_quire::ids::ModuleRoot::open)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| fail(scope, format!("module open failed: {e}")))?;
        let selection = quoin_quire::ModuleSelection::Closed(loaded);
        let request = quoin_quire::coverage::Request {
            scope: quoin_quire::ids::ScopeRoot::open(scope).map_err(|e| fail(scope, e))?,
            modules: selection,
        };
        let systems = quoin_quire::systems::load(&request).map_err(|e| fail(scope, e))?;
        let adrs = quoin_quire::records::read(scope, &request.modules, &["ADR"])
            .map_err(|e| fail(scope, e))?;
        (systems, adrs.documents)
    };
    let parts: BTreeSet<&str> = systems.parts.iter().map(|p| p.document.as_str()).collect();
    let decisions: BTreeSet<&str> = adr_documents.iter().map(|d| d.id.as_str()).collect();
    let mut any_design = false;
    for r in records {
        any_design |= render_sds_item(r, &systems, &parts, &decisions, &mut body, &mut gaps);
    }
    if !records.is_empty() && !any_design {
        gaps.push(Gap::new(
            GapKind::NoSoftwareDesignSource,
            "DOS-SDS",
            "no selected architecture part, decision, or related design record exists",
        ));
    }
    body.push_str("## Sources\n\n");
    let mut source_rows = BTreeSet::new();
    let mut selected_parts = BTreeSet::new();
    for r in records {
        source_rows.insert(format!("software_description/{}", r.id));
        for id in r.architecture_part_ids.iter().flatten() {
            if parts.contains(id.as_str()) {
                source_rows.insert(format!("part/{id}"));
                selected_parts.insert(id.as_str());
            }
        }
        for id in r.architecture_decision_ids.iter().flatten() {
            if decisions.contains(id.as_str()) {
                source_rows.insert(format!("ADR/{id}"));
            }
        }
    }
    for port in systems
        .ports
        .iter()
        .filter(|port| selected_parts.contains(port.owner.as_str()))
    {
        source_rows.insert(format!("port/{}", port.document));
    }
    for edge in systems.connections.iter().filter(|edge| {
        selected_parts.contains(edge.source.as_str())
            || selected_parts.contains(edge.target.as_str())
    }) {
        source_rows.insert(format!("connection/{}", edge.document));
    }
    for edge in systems.allocations.iter().filter(|edge| {
        selected_parts.contains(edge.source.as_str())
            || selected_parts.contains(edge.target.as_str())
    }) {
        source_rows.insert(format!("allocation/{}", edge.document));
    }
    for source in source_rows {
        let _ = writeln!(body, "- `{}`", cell(&source));
    }
    Ok(Generated {
        body,
        document_version: None,
        gaps,
    })
}

fn render_sds_item(
    r: &Software,
    systems: &quoin_quire::systems::Systems,
    parts: &BTreeSet<&str>,
    decisions: &BTreeSet<&str>,
    body: &mut String,
    gaps: &mut Vec<Gap>,
) -> bool {
    let _ = writeln!(
        body,
        "## {}\n\n**Source:** `software_description/{}`\n",
        cell(&r.identity),
        cell(&r.id)
    );
    let part_ids = r.architecture_part_ids.as_deref().unwrap_or(&[]);
    let decision_ids = r.architecture_decision_ids.as_deref().unwrap_or(&[]);
    let mut any_design = render_design_ids(
        body,
        gaps,
        part_ids,
        parts,
        r,
        DesignIdList {
            heading: "Architecture parts",
            gap_kind: GapKind::SoftwareDesignPartListUnstated,
            field: "architecture_part_ids",
            expected: "architecture part",
        },
    );
    any_design |= render_design_ids(
        body,
        gaps,
        decision_ids,
        decisions,
        r,
        DesignIdList {
            heading: "Architecture decisions",
            gap_kind: GapKind::SoftwareDesignDecisionListUnstated,
            field: "architecture_decision_ids",
            expected: "ADR",
        },
    );
    let selected_parts: BTreeSet<&str> = part_ids
        .iter()
        .filter_map(|id| parts.contains(id.as_str()).then_some(id.as_str()))
        .collect();
    any_design |= render_selected_relationships(systems, &selected_parts, body);
    any_design
}

#[derive(Clone, Copy)]
struct DesignIdList {
    heading: &'static str,
    gap_kind: GapKind,
    field: &'static str,
    expected: &'static str,
}

fn render_design_ids(
    body: &mut String,
    gaps: &mut Vec<Gap>,
    ids: &[String],
    known: &BTreeSet<&str>,
    r: &Software,
    list: DesignIdList,
) -> bool {
    if ids.is_empty() {
        let _ = writeln!(body, "**{}:** Not stated.\n", list.heading);
        gaps.push(Gap::new(
            list.gap_kind,
            &r.identity,
            format!("source record {} has no {}", r.id, list.field),
        ));
        return false;
    }
    let _ = writeln!(body, "**{}:**", list.heading);
    let mut resolved = false;
    for id in ids {
        if known.contains(id.as_str()) {
            resolved = true;
            let _ = writeln!(body, "- `{}`", cell(id));
        } else {
            gaps.push(Gap::new(
                GapKind::SourceReferenceUnresolved,
                &r.id,
                format!("referenced id {id} expected {}", list.expected),
            ));
        }
    }
    body.push('\n');
    resolved
}

fn render_selected_relationships(
    systems: &quoin_quire::systems::Systems,
    selected_parts: &BTreeSet<&str>,
    body: &mut String,
) -> bool {
    let mut rendered = false;
    for port in systems
        .ports
        .iter()
        .filter(|port| selected_parts.contains(port.owner.as_str()))
    {
        rendered = true;
        let _ = writeln!(
            body,
            "**Port** `{}`: {} / {} / {} _(source: architecture part `{}`)_",
            cell(&port.document),
            cell(&port.direction),
            cell(&port.interface),
            cell(&port.multiplicity),
            cell(&port.owner)
        );
    }
    for edge in systems.connections.iter().filter(|edge| {
        selected_parts.contains(edge.source.as_str())
            || selected_parts.contains(edge.target.as_str())
    }) {
        rendered = true;
        let _ = writeln!(
            body,
            "**Connection** `{}`: `{}` → `{}` ({})",
            cell(&edge.document),
            cell(&edge.source),
            cell(&edge.target),
            cell(&edge.direction)
        );
    }
    for edge in systems.allocations.iter().filter(|edge| {
        selected_parts.contains(edge.source.as_str())
            || selected_parts.contains(edge.target.as_str())
    }) {
        rendered = true;
        let _ = writeln!(
            body,
            "**Allocation** `{}`: `{}` → `{}`",
            cell(&edge.document),
            cell(&edge.source),
            cell(&edge.target)
        );
    }
    rendered
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn scope() -> tempfile::TempDir {
        tempfile::tempdir().expect("temporary scope")
    }
    fn put(root: &Path, kind: &str, id: &str, value: &str) {
        let dir = root.join("dossier/sources").join(kind);
        fs::create_dir_all(&dir).expect("source directory");
        fs::write(dir.join(format!("{id}.json")), value).expect("source record");
    }

    #[trace("FR-233-AC-1")]
    #[test]
    fn rejects_unknown_members_and_path_identity_mismatch() {
        let t = scope();
        put(
            t.path(),
            "software_description",
            "s1",
            r#"{"schema_version":1,"id":"s2","record_type":"software_description","software_id":"S","name":"Name"}"#,
        );
        assert!(
            read_all(t.path())
                .expect_err("mismatched path rejected")
                .detail
                .contains("path, record_type, and id")
        );
        put(
            t.path(),
            "software_description",
            "s1",
            r#"{"schema_version":1,"id":"s1","record_type":"software_description","software_id":"S","name":"Name","extra":true}"#,
        );
        assert!(
            read_all(t.path())
                .expect_err("unknown member rejected")
                .detail
                .contains("schema validation")
        );
    }

    #[trace("FR-233-AC-2")]
    #[test]
    fn rejects_duplicate_domain_identity() {
        let t = scope();
        put(
            t.path(),
            "software_description",
            "s1",
            r#"{"schema_version":1,"id":"s1","record_type":"software_description","software_id":"S","name":"One"}"#,
        );
        put(
            t.path(),
            "software_description",
            "s2",
            r#"{"schema_version":1,"id":"s2","record_type":"software_description","software_id":"S","name":"Two"}"#,
        );
        assert!(
            read_all(t.path())
                .expect_err("duplicate software identity rejected")
                .detail
                .contains("duplicate software_id")
        );
    }

    #[trace("FR-233-AC-1")]
    #[test]
    fn rejects_malformed_json_and_empty_optional_values() {
        let t = scope();
        put(t.path(), "software_description", "sw", "{");
        assert!(
            read_all(t.path())
                .expect_err("malformed JSON rejected")
                .detail
                .contains("invalid JSON")
        );
        put(
            t.path(),
            "software_description",
            "sw",
            r#"{"schema_version":1,"id":"sw","record_type":"software_description","software_id":"SW","name":"Name","purpose":""}"#,
        );
        assert!(
            read_all(t.path())
                .expect_err("empty strings rejected")
                .detail
                .contains("schema validation")
        );
    }

    #[trace("FR-229-AC-1", "FR-229-AC-2", "FR-229-AC-3")]
    #[test]
    fn software_output_preserves_authored_values_and_reports_absence() {
        let t = scope();
        put(
            t.path(),
            "software_description",
            "sw-src",
            r#"{"schema_version":1,"id":"sw-src","record_type":"software_description","software_id":"SW-1","name":" Name ","purpose":" purpose "}"#,
        );
        let r = read_all(t.path()).expect("valid records");
        let doc = render_sw(&r.software);
        assert!(doc.body.contains(" Name "));
        assert!(doc.body.contains(" purpose "));
        assert!(doc.body.contains("software_description/sw-src"));
        assert_eq!(
            doc.gaps
                .iter()
                .filter(|g| g.gap_kind == GapKind::SoftwareDescriptionFieldUnstated)
                .count(),
            3
        );
        assert_eq!(
            render_sw(&[]).gaps[0].gap_kind,
            GapKind::NoSoftwareDescriptionSource
        );
    }

    #[trace("FR-233-AC-4")]
    #[test]
    fn field_order_does_not_change_the_normalized_generated_document() {
        let first = scope();
        let second = scope();
        put(
            first.path(),
            "software_description",
            "sw",
            r#"{"schema_version":1,"id":"sw","record_type":"software_description","software_id":"SW","name":"Name","purpose":"Purpose","functions":["Function"]}"#,
        );
        put(
            second.path(),
            "software_description",
            "sw",
            r#"{"functions":["Function"],"purpose":"Purpose","name":"Name","software_id":"SW","record_type":"software_description","id":"sw","schema_version":1}"#,
        );
        let one = render_sw(&read_all(first.path()).expect("first valid record").software);
        let two = render_sw(
            &read_all(second.path())
                .expect("second valid record")
                .software,
        );
        assert_eq!(one, two);
    }

    #[trace("FR-233-AC-1")]
    #[test]
    fn generator_reads_only_its_selected_source_kinds() {
        let t = scope();
        put(
            t.path(),
            "software_description",
            "sw",
            r#"{"schema_version":1,"id":"sw","record_type":"software_description","software_id":"SW","name":"Name"}"#,
        );
        put(t.path(), "device_description", "dev", "{");
        let records = read_records(t.path(), true, false, false)
            .expect("unselected device records are not read");
        assert_eq!(records.software.len(), 1);
        assert!(records.devices.is_empty());
    }

    #[trace("FR-231-AC-2")]
    #[test]
    fn missing_design_lists_are_visible_and_scoped_per_software_item() {
        let t = scope();
        put(
            t.path(),
            "software_description",
            "sw-src",
            r#"{"schema_version":1,"id":"sw-src","record_type":"software_description","software_id":"SW-1","name":"Name"}"#,
        );
        let r = read_all(t.path()).expect("valid records");
        let doc = render_sds(&r.software, t.path(), &[]).expect("render");
        assert!(doc.body.contains("Architecture parts:** Not stated."));
        assert!(doc.body.contains("Architecture decisions:** Not stated."));
        assert!(
            doc.gaps
                .iter()
                .any(|g| g.gap_kind == GapKind::SoftwareDesignPartListUnstated
                    && g.subject == "SW-1")
        );
        assert!(doc.gaps.iter().any(
            |g| g.gap_kind == GapKind::SoftwareDesignDecisionListUnstated && g.subject == "SW-1"
        ));
    }

    #[trace("FR-230-AC-1", "FR-230-AC-2", "FR-230-AC-3")]
    #[test]
    fn decisions_preserve_authored_fields_gaps_and_conflicts() {
        let t = scope();
        put(
            t.path(),
            "software_description",
            "sw",
            r#"{"schema_version":1,"id":"sw","record_type":"software_description","software_id":"SW","name":"Name"}"#,
        );
        put(
            t.path(),
            "documentation_level_decision",
            "d1",
            r#"{"schema_version":1,"id":"d1","record_type":"documentation_level_decision","software_id":"SW","level":" Level A ","decided_by":"Person","decided_on":"2026-02-28","rationale":"why"}"#,
        );
        put(
            t.path(),
            "documentation_level_decision",
            "d2",
            r#"{"schema_version":1,"id":"d2","record_type":"documentation_level_decision","software_id":"SW","level":"Level B"}"#,
        );
        let records = read_all(t.path()).expect("valid records");
        let doc = render_dl(&records);
        assert!(doc.body.contains(" Level A "));
        assert!(doc.body.contains("Person"));
        assert!(doc.body.contains("2026-02-28"));
        assert!(
            doc.gaps
                .iter()
                .any(|g| g.gap_kind == GapKind::ConflictingDocumentationLevelDecisions)
        );
        assert_eq!(
            doc.gaps
                .iter()
                .filter(|g| g.gap_kind == GapKind::DocumentationLevelDecisionFieldUnstated)
                .count(),
            5
        );
    }

    #[trace("FR-232-AC-1", "FR-233-AC-3")]
    #[test]
    fn device_software_association_requires_a_resolved_explicit_id() {
        let t = scope();
        put(
            t.path(),
            "device_description",
            "dev",
            r#"{"schema_version":1,"id":"dev","record_type":"device_description","device_id":"D","name":"Device","software_ids":["missing"]}"#,
        );
        let records = read_all(t.path()).expect("valid shape without resolution");
        let doc = render_dev(&records);
        assert!(
            doc.gaps
                .iter()
                .any(|g| g.gap_kind == GapKind::SourceReferenceUnresolved
                    && g.detail.contains("missing"))
        );
        assert!(!doc.body.contains("- `missing`"));
    }

    #[trace("FR-231-AC-5")]
    #[test]
    fn design_does_not_infer_unlisted_architecture_ids() {
        let t = scope();
        put(
            t.path(),
            "software_description",
            "sw",
            r#"{"schema_version":1,"id":"sw","record_type":"software_description","software_id":"SW","name":"Name","architecture_part_ids":["same-name"],"architecture_decision_ids":["same-name"]}"#,
        );
        let records = read_all(t.path()).expect("valid reference shape");
        let doc = render_sds(&records.software, t.path(), &[]).expect("render");
        assert!(!doc.body.contains("- `same-name`"));
        assert_eq!(
            doc.gaps
                .iter()
                .filter(|g| g.gap_kind == GapKind::SourceReferenceUnresolved)
                .count(),
            2
        );
    }
}
