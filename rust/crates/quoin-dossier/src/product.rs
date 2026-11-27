// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Product dossier documents and their closed JSON source contract (FR-229–233).

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

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

#[derive(Debug, Default)]
struct ProductRecords {
    software: Vec<Value>,
    decisions: Vec<Value>,
    devices: Vec<Value>,
}

fn string_field<'a>(record: &'a Value, name: &str) -> Option<&'a str> {
    record.get(name).and_then(Value::as_str)
}

fn required_field<'a>(record: &'a Value, name: &str) -> &'a str {
    string_field(record, name).unwrap_or_default()
}

fn string_list<'a>(record: &'a Value, name: &str) -> Vec<&'a str> {
    record
        .get(name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn object_string_pairs<'a>(record: &'a Value, name: &str) -> Vec<(&'a str, &'a str)> {
    record
        .get(name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some((
                string_field(item, "record_type")?,
                string_field(item, "id")?,
            ))
        })
        .collect()
}

fn fail(path: &Path, reason: impl std::fmt::Display) -> SourceFailure {
    SourceFailure {
        source: path.display().to_string(),
        detail: reason.to_string(),
    }
}

fn read_kind(
    scope: &Path,
    kind: &str,
    schema: &str,
    record_type: &str,
) -> Result<Vec<Value>, SourceFailure> {
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
        records.push(value);
    }
    Ok(records)
}

#[cfg(test)]
fn read_all(scope: &Path) -> Result<ProductRecords, SourceFailure> {
    read_records(scope, true, true, true)
}

fn read_records(
    scope: &Path,
    include_software: bool,
    include_decisions: bool,
    include_devices: bool,
) -> Result<ProductRecords, SourceFailure> {
    let software: Vec<Value> = if include_software {
        read_kind(
            scope,
            "software_description",
            SW_SCHEMA,
            "software_description",
        )?
    } else {
        Vec::new()
    };
    let decisions: Vec<Value> = if include_decisions {
        read_kind(
            scope,
            "documentation_level_decision",
            DL_SCHEMA,
            "documentation_level_decision",
        )?
    } else {
        Vec::new()
    };
    let devices: Vec<Value> = if include_devices {
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
        if !identities.insert(required_field(r, "software_id").to_owned()) {
            return Err(fail(
                &scope.join("dossier/sources/software_description"),
                "invalid version/type or duplicate software_id",
            ));
        }
    }
    identities.clear();
    for r in &devices {
        if !identities.insert(required_field(r, "device_id").to_owned()) {
            return Err(fail(
                &scope.join("dossier/sources/device_description"),
                "invalid version/type or duplicate device_id",
            ));
        }
    }
    for record in &decisions {
        if string_field(record, "decided_on").is_some_and(|date| !valid_date(date)) {
            return Err(fail(
                &scope.join("dossier/sources/documentation_level_decision"),
                "invalid decided_on calendar date",
            ));
        }
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

fn render_sw(records: &[Value]) -> Generated {
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
        let _ = writeln!(
            body,
            "- `software_description/{}`",
            cell(required_field(r, "id"))
        );
    }
    Generated {
        body,
        document_version: None,
        gaps,
    }
}

fn render_sw_section(r: &Value, body: &mut String, gaps: &mut Vec<Gap>) {
    let source_id = required_field(r, "id");
    let software_id = required_field(r, "software_id");
    let _ = writeln!(
        body,
        "## {} — {}\n\n**Source:** `software_description/{}`\n",
        cell(software_id),
        cell(required_field(r, "name")),
        cell(source_id)
    );
    let s = source_id;
    label(
        body,
        "Purpose",
        string_field(r, "purpose"),
        s,
        gaps,
        (GapKind::SoftwareDescriptionFieldUnstated, source_id),
    );
    label(
        body,
        "Operating environment",
        string_field(r, "operating_environment"),
        s,
        gaps,
        (GapKind::SoftwareDescriptionFieldUnstated, source_id),
    );
    for (field, values) in [
        ("Functions", string_list(r, "functions")),
        ("Interfaces", string_list(r, "interfaces")),
    ] {
        if values.is_empty() {
            gaps.push(Gap::new(
                GapKind::SoftwareDescriptionFieldUnstated,
                source_id,
                format!("{field} is not stated in source record {s}"),
            ));
        } else {
            let _ = writeln!(body, "**{field}:** _(source: `{}`)_", cell(s));
            for x in values {
                let _ = writeln!(body, "- {}", prose(x));
            }
            body.push('\n');
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
            .entry(required_field(r, "software_id"))
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
        let _ = writeln!(
            body,
            "- `documentation_level_decision/{}`",
            cell(required_field(r, "id"))
        );
        for (record_type, id) in object_string_pairs(r, "source_records") {
            let _ = writeln!(body, "- `{}/{}`", cell(record_type), cell(id));
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
            .map(|r| required_field(r, "software_id"))
            .collect();
        let source_ids = records
            .software
            .iter()
            .map(|r| ("software_description", required_field(r, "id")))
            .chain(
                records
                    .decisions
                    .iter()
                    .map(|r| ("documentation_level_decision", required_field(r, "id"))),
            )
            .chain(
                records
                    .devices
                    .iter()
                    .map(|r| ("device_description", required_field(r, "id"))),
            )
            .collect();
        Self {
            software,
            source_ids,
        }
    }
}

fn render_decision(
    r: &Value,
    index: &ProductIndex<'_>,
    body: &mut String,
    gaps: &mut Vec<Gap>,
) -> String {
    let id = required_field(r, "id");
    let software_id = required_field(r, "software_id");
    let _ = writeln!(
        body,
        "## {}\n\n**Source:** `documentation_level_decision/{}`\n\n**Software:** `{}`\n",
        cell(software_id),
        cell(id),
        cell(software_id)
    );
    if !index.software.contains(software_id) {
        gaps.push(Gap::new(
            GapKind::SourceReferenceUnresolved,
            id,
            format!("software_id {software_id} expected software_description"),
        ));
    }
    for (key, value) in [
        ("Level", string_field(r, "level")),
        ("Decided by", string_field(r, "decided_by")),
        ("Decided on", string_field(r, "decided_on")),
        ("Rationale", string_field(r, "rationale")),
    ] {
        label(
            body,
            key,
            value,
            id,
            gaps,
            (GapKind::DocumentationLevelDecisionFieldUnstated, id),
        );
    }
    let refs = object_string_pairs(r, "source_records");
    let array_stated = r.get("source_records").is_some();
    if !array_stated || refs.is_empty() {
        gaps.push(Gap::new(
            GapKind::DocumentationLevelDecisionFieldUnstated,
            id,
            if array_stated {
                "source_records is empty"
            } else {
                "source_records is not stated"
            },
        ));
    } else {
        let _ = writeln!(body, "**Referenced sources:** _(from `{}`)_", cell(id));
        for (record_type, source_id) in &refs {
            let _ = writeln!(body, "- `{}/{}`", cell(record_type), cell(source_id));
            if !index.source_ids.contains(&(*record_type, *source_id)) {
                gaps.push(Gap::new(
                    GapKind::SourceReferenceUnresolved,
                    id,
                    format!("referenced id {source_id} expected {record_type}"),
                ));
            }
        }
        body.push('\n');
    }
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}",
        string_field(r, "level"),
        string_field(r, "decided_by"),
        string_field(r, "decided_on"),
        string_field(r, "rationale"),
        refs
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
    let software: BTreeMap<&str, &str> = records
        .software
        .iter()
        .map(|r| (required_field(r, "software_id"), required_field(r, "id")))
        .collect();
    let mut source_rows = BTreeSet::new();
    for r in &records.devices {
        let id = required_field(r, "id");
        let _ = writeln!(
            body,
            "## {} — {}\n\n**Source:** `device_description/{}`\n",
            cell(required_field(r, "device_id")),
            cell(required_field(r, "name")),
            cell(id)
        );
        for (k, v) in [
            ("Purpose", string_field(r, "purpose")),
            ("Intended use", string_field(r, "intended_use")),
            ("Operating context", string_field(r, "operating_context")),
        ] {
            label(
                &mut body,
                k,
                v,
                id,
                &mut gaps,
                (GapKind::DeviceFieldUnstated, id),
            );
        }
        let software_ids = string_list(r, "software_ids");
        if software_ids.is_empty() {
            gaps.push(Gap::new(
                GapKind::DeviceFieldUnstated,
                id,
                "software_ids is absent or empty",
            ));
        } else {
            let _ = writeln!(body, "**Software items:** _(source: `{}`)_", cell(id));
            for software_id in software_ids {
                if let Some(source_id) = software.get(software_id) {
                    let _ = writeln!(
                        body,
                        "- `{}` _(source: `software_description/{}`)_",
                        cell(software_id),
                        cell(source_id)
                    );
                    source_rows.insert(format!("software_description/{source_id}"));
                } else {
                    gaps.push(Gap::new(
                        GapKind::SourceReferenceUnresolved,
                        id,
                        format!("referenced id {software_id} expected software_description"),
                    ));
                }
            }
            body.push('\n');
        }
        source_rows.insert(format!("device_description/{id}"));
    }
    body.push_str("## Sources\n\n");
    for source in source_rows {
        let _ = writeln!(body, "- `{}`", cell(&source));
    }
    Generated {
        body,
        document_version: None,
        gaps,
    }
}

fn render_sds(
    records: &[Value],
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
    let mut context = SdsContext {
        systems: &systems,
        parts: systems.parts.iter().map(|p| p.document.as_str()).collect(),
        decisions: adr_documents.iter().map(|d| d.id.as_str()).collect(),
        selected_parts: BTreeSet::new(),
    };
    let mut any_design = false;
    for r in records {
        any_design |= render_sds_item(r, &mut context, &mut body, &mut gaps);
    }
    if !records.is_empty() && !any_design {
        gaps.push(Gap::new(
            GapKind::NoSoftwareDesignSource,
            "DOS-SDS",
            "no selected architecture part, decision, or related design record exists",
        ));
    }
    render_sds_sources(records, &context, &mut body);
    Ok(Generated {
        body,
        document_version: None,
        gaps,
    })
}

struct SdsContext<'a> {
    systems: &'a quoin_quire::systems::Systems,
    parts: BTreeSet<&'a str>,
    decisions: BTreeSet<&'a str>,
    selected_parts: BTreeSet<String>,
}

fn render_sds_sources(records: &[Value], context: &SdsContext<'_>, body: &mut String) {
    body.push_str("## Sources\n\n");
    let mut source_rows = BTreeSet::new();
    for record in records {
        source_rows.insert(format!(
            "software_description/{}",
            required_field(record, "id")
        ));
        source_rows.extend(
            string_list(record, "architecture_part_ids")
                .into_iter()
                .filter(|id| context.parts.contains(id))
                .map(|id| format!("part/{id}")),
        );
        source_rows.extend(
            string_list(record, "architecture_decision_ids")
                .into_iter()
                .filter(|id| context.decisions.contains(id))
                .map(|id| format!("ADR/{id}")),
        );
    }
    for port in context
        .systems
        .ports
        .iter()
        .filter(|port| context.selected_parts.contains(port.owner.as_str()))
    {
        source_rows.insert(format!("port/{}", port.document));
    }
    for edge in &context.systems.connections {
        if selected_connection(edge, context) {
            source_rows.insert(format!("connection/{}", edge.document));
            source_rows.extend(
                context
                    .systems
                    .ports
                    .iter()
                    .filter(|port| connection_has_port(edge, port))
                    .flat_map(|port| {
                        [
                            format!("part/{}", port.owner),
                            format!("port/{}", port.document),
                        ]
                    }),
            );
        }
    }
    for edge in context.systems.allocations.iter().filter(|edge| {
        context.selected_parts.contains(edge.source.as_str())
            || context.selected_parts.contains(edge.target.as_str())
    }) {
        source_rows.insert(format!("allocation/{}", edge.document));
    }
    for source in source_rows {
        let _ = writeln!(body, "- `{}`", cell(&source));
    }
}

fn connection_has_port(
    edge: &quoin_quire::systems::Connection,
    port: &quoin_quire::systems::Port,
) -> bool {
    edge.source == port.document || edge.target == port.document
}

fn selected_connection(edge: &quoin_quire::systems::Connection, context: &SdsContext<'_>) -> bool {
    let candidate = context.systems.ports.iter().any(|port| {
        context.selected_parts.contains(port.owner.as_str()) && connection_has_port(edge, port)
    }) || context.selected_parts.contains(edge.source.as_str())
        || context.selected_parts.contains(edge.target.as_str());
    let source_matches = context
        .systems
        .ports
        .iter()
        .filter(|port| edge.source == port.document)
        .count();
    let target_matches = context
        .systems
        .ports
        .iter()
        .filter(|port| edge.target == port.document)
        .count();
    candidate && source_matches == 1 && target_matches == 1
}

fn render_sds_item(
    r: &Value,
    context: &mut SdsContext<'_>,
    body: &mut String,
    gaps: &mut Vec<Gap>,
) -> bool {
    let _ = writeln!(
        body,
        "## {}\n\n**Source:** `software_description/{}`\n",
        cell(required_field(r, "software_id")),
        cell(required_field(r, "id"))
    );
    let part_ids = string_list(r, "architecture_part_ids");
    let decision_ids = string_list(r, "architecture_decision_ids");
    let mut any_design = render_design_ids(
        body,
        gaps,
        &part_ids,
        &context.parts,
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
        &decision_ids,
        &context.decisions,
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
        .filter_map(|id| context.parts.contains(id).then_some(*id))
        .collect();
    context
        .selected_parts
        .extend(selected_parts.iter().map(|id| (*id).to_owned()));
    any_design |= render_selected_relationships(context.systems, &selected_parts, body, gaps);
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
    ids: &[&str],
    known: &BTreeSet<&str>,
    r: &Value,
    list: DesignIdList,
) -> bool {
    if ids.is_empty() {
        let _ = writeln!(body, "**{}:** Not stated.\n", list.heading);
        gaps.push(Gap::new(
            list.gap_kind,
            required_field(r, "software_id"),
            format!(
                "source record {} has no {}",
                required_field(r, "id"),
                list.field
            ),
        ));
        return false;
    }
    let _ = writeln!(body, "**{}:**", list.heading);
    let mut resolved = false;
    for id in ids {
        if known.contains(id) {
            resolved = true;
            let _ = writeln!(body, "- `{}`", cell(id));
        } else {
            gaps.push(Gap::new(
                GapKind::SourceReferenceUnresolved,
                required_field(r, "id"),
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
    gaps: &mut Vec<Gap>,
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
            "**Port** `{}`: {} / {} / {} _(source: architecture port `{}`)_",
            cell(&port.document),
            cell(&port.direction),
            cell(&port.interface),
            cell(&port.multiplicity),
            cell(&port.document)
        );
    }
    for edge in &systems.connections {
        let candidate = systems.ports.iter().any(|port| {
            selected_parts.contains(port.owner.as_str()) && connection_has_port(edge, port)
        }) || selected_parts.contains(edge.source.as_str())
            || selected_parts.contains(edge.target.as_str());
        if !candidate {
            continue;
        }
        let source_matches = systems
            .ports
            .iter()
            .filter(|port| edge.source == port.document)
            .count();
        let target_matches = systems
            .ports
            .iter()
            .filter(|port| edge.target == port.document)
            .count();
        if source_matches != 1 || target_matches != 1 {
            let invalid = if source_matches == 1 {
                &edge.target
            } else {
                &edge.source
            };
            gaps.push(Gap::new(
                GapKind::SourceReferenceUnresolved,
                edge.document.as_str(),
                format!(
                    "connection {} endpoint {invalid} expected architecture port",
                    edge.document
                ),
            ));
            continue;
        }
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

    fn write_system_artifact(
        root: &Path,
        id: &str,
        title: &str,
        kind: &str,
        headers: &str,
        row: &str,
    ) {
        let fixture_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../quoin-quire/tests/fixtures/systems-repo/manifest.yaml");
        fs::copy(fixture_manifest, root.join("manifest.yaml")).expect("module manifest copied");
        let dir = root.join("spec");
        fs::create_dir_all(&dir).expect("architecture source directory");
        let heading = match kind {
            "part" => "Part",
            "port" => "Port",
            "connection" => "Connection",
            _ => unreachable!("test supplies a systems kind"),
        };
        let contents = format!(
            "---\nid: {id}\ntitle: {title:?}\ntype: {kind}\nobject: {kind}\n---\n\n## {heading}\n\n{headers}\n{row}\n"
        );
        fs::write(dir.join(format!("{id}.md")), contents).expect("architecture artifact");
    }

    fn write_product_source(root: &Path, architecture_part_ids: &[&str]) {
        let part_ids = serde_json::to_string(architecture_part_ids).expect("JSON ids");
        put(
            root,
            "software_description",
            "sw-src",
            &format!(
                "{{\"schema_version\":1,\"id\":\"sw-src\",\"record_type\":\"software_description\",\"software_id\":\"SW\",\"name\":\"Software\",\"architecture_part_ids\":{part_ids},\"architecture_decision_ids\":[]}}"
            ),
        );
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

        let devices = scope();
        put(
            devices.path(),
            "software_description",
            "shared",
            r#"{"schema_version":1,"id":"shared","record_type":"software_description","software_id":"SW","name":"Name"}"#,
        );
        put(
            devices.path(),
            "device_description",
            "shared",
            r#"{"schema_version":1,"id":"shared","record_type":"device_description","device_id":"D","name":"Device"}"#,
        );
        put(
            devices.path(),
            "device_description",
            "dev2",
            r#"{"schema_version":1,"id":"dev2","record_type":"device_description","device_id":"D","name":"Other"}"#,
        );
        assert!(
            read_all(devices.path())
                .expect_err("duplicate device identity rejected")
                .detail
                .contains("duplicate device_id")
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
        assert_eq!(records.devices, [] as [serde_json::Value; 0]);
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
        put(
            t.path(),
            "software_description",
            "sw-empty",
            r#"{"schema_version":1,"id":"sw-empty","record_type":"software_description","software_id":"SW-2","name":"Name","architecture_part_ids":[],"architecture_decision_ids":[]}"#,
        );
        let r = read_all(t.path()).expect("valid records");
        let doc = render_sds(&r.software, t.path(), &[]).expect("render");
        assert!(doc.body.contains("Architecture parts:** Not stated."));
        assert!(doc.body.contains("Architecture decisions:** Not stated."));
        assert_eq!(
            doc.gaps
                .iter()
                .filter(|g| g.gap_kind == GapKind::SoftwareDesignPartListUnstated)
                .count(),
            2
        );
        assert_eq!(
            doc.gaps
                .iter()
                .filter(|g| g.gap_kind == GapKind::SoftwareDesignDecisionListUnstated)
                .count(),
            2
        );
    }

    #[trace("FR-230-AC-1", "FR-230-AC-2", "FR-230-AC-3", "FR-233-AC-3")]
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
            r#"{"schema_version":1,"id":"d1","record_type":"documentation_level_decision","software_id":"SW","level":" Level A ","decided_by":"Person","decided_on":"2026-02-28","rationale":"why","source_records":[{"id":"missing","record_type":"device_description"}]}"#,
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
        assert!(
            doc.gaps
                .iter()
                .any(|g| g.gap_kind == GapKind::SourceReferenceUnresolved
                    && g.detail.contains("missing"))
        );
        assert_eq!(
            doc.gaps
                .iter()
                .filter(|g| g.gap_kind == GapKind::DocumentationLevelDecisionFieldUnstated)
                .count(),
            4
        );
    }

    #[trace("FR-232-AC-3", "FR-233-AC-3")]
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

    #[trace("FR-232-AC-1")]
    #[test]
    fn device_lists_explicit_resolved_software_and_all_source_identities() {
        let t = scope();
        put(
            t.path(),
            "software_description",
            "sw-src",
            r#"{"schema_version":1,"id":"sw-src","record_type":"software_description","software_id":"SW-1","name":"Software"}"#,
        );
        put(
            t.path(),
            "device_description",
            "dev-src",
            r#"{"schema_version":1,"id":"dev-src","record_type":"device_description","device_id":"D-1","name":"Device","purpose":"purpose","intended_use":"use","operating_context":"context","software_ids":["SW-1"]}"#,
        );
        let records = read_all(t.path()).expect("valid linked records");
        let doc = render_dev(&records);
        assert!(doc.body.contains("`SW-1`"));
        assert!(doc.body.contains("software_description/sw-src"));
        assert!(doc.body.contains("device_description/dev-src"));
        assert!(
            !doc.gaps
                .iter()
                .any(|g| g.gap_kind == GapKind::SourceReferenceUnresolved)
        );
    }

    #[trace("FR-232-AC-2")]
    #[test]
    fn missing_device_facts_each_create_an_unstated_gap() {
        let t = scope();
        put(
            t.path(),
            "device_description",
            "dev",
            r#"{"schema_version":1,"id":"dev","record_type":"device_description","device_id":"D","name":"Device","software_ids":["SW"]}"#,
        );
        let doc = render_dev(&read_all(t.path()).expect("valid device record"));
        assert_eq!(
            doc.gaps
                .iter()
                .filter(|g| g.gap_kind == GapKind::DeviceFieldUnstated)
                .count(),
            3
        );
    }

    #[trace("FR-232-AC-5")]
    #[test]
    fn absent_device_source_is_incomplete() {
        let doc = render_dev(&ProductRecords::default());
        assert_eq!(doc.gaps[0].gap_kind, GapKind::NoDeviceDescriptionSource);
    }

    #[trace("FR-233-AC-5")]
    #[test]
    fn shipped_schemas_encode_the_closed_v1_source_contract() {
        for (schema_text, record_type) in [
            (SW_SCHEMA, "software_description"),
            (DL_SCHEMA, "documentation_level_decision"),
            (DEV_SCHEMA, "device_description"),
        ] {
            let schema: Value = serde_json::from_str(schema_text).expect("valid shipped schema");
            assert_eq!(
                schema["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
            assert_eq!(schema["type"], "object");
            assert_eq!(schema["additionalProperties"], false);
            assert_eq!(schema["properties"]["schema_version"]["const"], 1);
            assert_eq!(schema["properties"]["record_type"]["const"], record_type);
            assert_eq!(
                schema["properties"]["id"]["pattern"],
                "^[A-Za-z0-9][A-Za-z0-9._-]*$"
            );
            assert!(schema["required"].as_array().is_some_and(|items| {
                items.iter().any(|item| item == "schema_version")
                    && items.iter().any(|item| item == "id")
                    && items.iter().any(|item| item == "record_type")
            }));
            assert!(schema["properties"].as_object().is_some_and(|properties| {
                properties.values().all(|definition| {
                    definition.get("type").is_some() || definition.get("const").is_some()
                })
            }));
        }
    }

    #[trace("FR-230-AC-4")]
    #[test]
    fn decision_contract_rejects_safety_or_acceptability_assignments() {
        let t = scope();
        put(
            t.path(),
            "documentation_level_decision",
            "decision",
            r#"{"schema_version":1,"id":"decision","record_type":"documentation_level_decision","software_id":"SW","safety_class":"Class A"}"#,
        );
        assert!(
            read_all(t.path())
                .expect_err("unknown assignment rejected")
                .detail
                .contains("schema validation")
        );
    }

    #[trace("FR-191-AC-32", "FR-231-AC-1", "FR-231-AC-3")]
    #[test]
    fn render_sds_resolves_authored_owner_port_and_connection_references() {
        let t = scope();
        write_product_source(t.path(), &["scoring_engine", "planner"]);
        write_system_artifact(
            t.path(),
            "scoring_engine",
            "Scoring Engine",
            "part",
            "| Owner | Declared Type | Multiplicity |\n|---|---|---|",
            "| search_service | quant_scoring_engine | 1..1 |",
        );
        write_system_artifact(
            t.path(),
            "planner",
            "Planner",
            "part",
            "| Owner | Declared Type | Multiplicity |\n|---|---|---|",
            "| search_service | query_planner | 1..1 |",
        );
        write_system_artifact(
            t.path(),
            "planner_query_out",
            "Planner Query Output",
            "port",
            "| Owner | Direction | Interface | Multiplicity |\n|---|---|---|---|",
            "| planner | out | quant_query | 1..1 |",
        );
        write_system_artifact(
            t.path(),
            "score_query_in",
            "Score Query Input",
            "port",
            "| Owner | Direction | Interface | Multiplicity |\n|---|---|---|---|",
            "| scoring_engine | in | quant_query | 1..1 |",
        );
        write_system_artifact(
            t.path(),
            "planner_to_scoring",
            "Planner To Scoring Connection",
            "connection",
            "| Source | Source Multiplicity | Target | Target Multiplicity | Direction |\n|---|---|---|---|---|",
            "| planner_query_out | 1..1 | score_query_in | 1..1 | source-to-target |",
        );
        let records = read_all(t.path()).expect("software source is valid");
        let doc = render_sds(&records.software, t.path(), &[t.path().to_path_buf()])
            .expect("production SDS renderer");
        assert!(
            doc.body.contains("**Port** `planner_query_out`"),
            "{}",
            doc.body
        );
        assert!(
            doc.body
                .contains("source: architecture port `planner_query_out`")
        );
        assert!(doc.body.contains(
            "**Connection** `planner_to_scoring`: `planner_query_out` → `score_query_in`"
        ));
        for identity in [
            "software_description/sw-src",
            "part/scoring_engine",
            "part/planner",
            "port/planner_query_out",
            "port/score_query_in",
            "connection/planner_to_scoring",
        ] {
            assert!(
                doc.body.contains(identity),
                "missing source identity {identity}"
            );
        }
        assert!(
            !doc.gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::SourceReferenceUnresolved)
        );
        assert!(!doc.body.contains("safety class"));
    }

    #[trace("FR-231-AC-1")]
    #[test]
    fn design_output_contains_only_explicit_part_and_decision_ids() {
        let record = serde_json::json!({"id":"sw-src", "software_id":"SW"});
        let known_parts = BTreeSet::from(["PART-A", "PART-B"]);
        let known_decisions = BTreeSet::from(["ADR-1", "ADR-2"]);
        let mut body = String::new();
        let mut gaps = Vec::new();
        assert!(render_design_ids(
            &mut body,
            &mut gaps,
            &["PART-A"],
            &known_parts,
            &record,
            DesignIdList {
                heading: "Architecture parts",
                gap_kind: GapKind::SoftwareDesignPartListUnstated,
                field: "architecture_part_ids",
                expected: "architecture part",
            },
        ));
        assert!(render_design_ids(
            &mut body,
            &mut gaps,
            &["ADR-1"],
            &known_decisions,
            &record,
            DesignIdList {
                heading: "Architecture decisions",
                gap_kind: GapKind::SoftwareDesignDecisionListUnstated,
                field: "architecture_decision_ids",
                expected: "ADR",
            },
        ));
        assert!(body.contains("- `PART-A`"));
        assert!(body.contains("- `ADR-1`"));
        assert!(!body.contains("PART-B"));
        assert!(!body.contains("ADR-2"));
        assert_eq!(gaps, [] as [crate::gap::Gap; 0]);
    }

    #[trace("FR-231-AC-5")]
    #[test]
    fn wrong_kind_or_missing_connection_endpoint_is_gapped_and_not_rendered() {
        for (target, invalid_id) in [
            ("scoring_engine", "scoring_engine"),
            ("missing_port", "missing_port"),
        ] {
            let t = scope();
            write_product_source(t.path(), &["scoring_engine"]);
            write_system_artifact(
                t.path(),
                "scoring_engine",
                "Scoring Engine",
                "part",
                "| Owner | Declared Type | Multiplicity |\n|---|---|---|",
                "| search_service | quant_scoring_engine | 1..1 |",
            );
            write_system_artifact(
                t.path(),
                "planner_query_out",
                "Planner Query Output",
                "port",
                "| Owner | Direction | Interface | Multiplicity |\n|---|---|---|---|",
                "| scoring_engine | out | quant_query | 1..1 |",
            );
            write_system_artifact(
                t.path(),
                "planner_to_scoring",
                "Planner To Scoring Connection",
                "connection",
                "| Source | Source Multiplicity | Target | Target Multiplicity | Direction |\n|---|---|---|---|---|",
                &format!("| planner_query_out | 1..1 | {target} | 1..1 | source-to-target |"),
            );
            let records = read_all(t.path()).expect("software source is valid");
            let doc = render_sds(&records.software, t.path(), &[t.path().to_path_buf()])
                .expect("production SDS renderer");
            assert!(!doc.body.contains("**Connection** `planner_to_scoring`"));
            assert!(doc.gaps.iter().any(|gap| {
                gap.gap_kind == GapKind::SourceReferenceUnresolved
                    && gap.subject == "planner_to_scoring"
                    && gap.detail.contains(invalid_id)
                    && gap.detail.contains("expected architecture port")
            }));
        }
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
