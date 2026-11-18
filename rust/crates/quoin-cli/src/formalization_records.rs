// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Strict FR-071 input validation and tuple joins for VER-105.

use std::collections::BTreeMap;

use engineering_assurance::evidence::EvidenceEnvelope;
use filament_contract_model::{PackageId, SourceIdentity, SourceLocation, SourceSpan};
use quoin_assurance::FormalizationRecordView;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CriterionId {
    obligation_declaration: String,
    scoped_document: String,
    criterion_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClauseAddress {
    package_identity: String,
    criterion_id: CriterionId,
    clause_id: String,
    source_span: SpanWire,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpanWire {
    start: LocationWire,
    end: LocationWire,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocationWire {
    source: SourceIdentity,
    line: u32,
    column: u32,
    byte_offset: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    criterion_id: CriterionId,
    state: String,
    #[serde(default)]
    clause_address: Option<ClauseAddress>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    evidence: Option<EvidenceEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Identity(String, String, String);

impl From<&CriterionId> for Identity {
    fn from(value: &CriterionId) -> Self {
        Self(
            value.obligation_declaration.clone(),
            value.scoped_document.clone(),
            value.criterion_id.clone(),
        )
    }
}

#[derive(Debug)]
struct Row {
    id: String,
    state: String,
    draft_reason: Option<String>,
}

pub(crate) fn read_and_join(
    path: &str,
    obligations: &Value,
    rules: &[Value],
) -> Result<Vec<FormalizationRecordView>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("formalization records are not readable JSON: {error}"))?;
    let raw: Value = serde_json::from_str(&text)
        .map_err(|error| format!("formalization records are not readable JSON: {error}"))?;
    let values = raw
        .as_array()
        .ok_or_else(|| "formalization records must be a top-level JSON array".to_owned())?;
    let rows = obligations
        .as_array()
        .ok_or_else(|| "quire.coverage obligations must be an array".to_owned())?;
    let mut inventory = BTreeMap::new();
    for row in rows {
        let id = text_field(row, "id")?;
        let Some(source) = row.get("source").and_then(Value::as_str) else {
            return Err(format!("Quire row {id} omitted source"));
        };
        let Some(document) = row.get("document").and_then(Value::as_str) else {
            return Err(format!("Quire row {id} omitted document"));
        };
        let identity = Identity(source.to_owned(), document.to_owned(), id.to_owned());
        let (state, draft_reason) = formalization_item(id, rules)?;
        if inventory
            .insert(
                identity.clone(),
                Row {
                    id: id.to_owned(),
                    state,
                    draft_reason,
                },
            )
            .is_some()
        {
            return Err(format!(
                "Quire inventory contains duplicate criterion tuple for {id}"
            ));
        }
    }
    let mut records = BTreeMap::<Identity, (Value, Option<String>)>::new();
    for value in values {
        if value.as_object().is_some_and(|object| {
            object.get("reason").is_some_and(Value::is_null)
                || object.get("clause_address").is_some_and(Value::is_null)
        }) {
            return Err("optional FR-071 members must be omitted rather than null".to_owned());
        }
        let record: Record = serde_json::from_value(value.clone())
            .map_err(|error| format!("record violates FR-071: {error}"))?;
        validate_record(&record)?;
        let identity = Identity::from(&record.criterion_id);
        if !inventory.contains_key(&identity) {
            return Err(format!(
                "formalization record does not match a Quire criterion: {}",
                identity.2
            ));
        }
        if records
            .insert(identity.clone(), (value.clone(), None))
            .is_some()
        {
            return Err(format!("duplicate formalization record for {}", identity.2));
        }
    }
    if records.len() != inventory.len() {
        let missing = inventory
            .keys()
            .find(|identity| !records.contains_key(*identity))
            .map_or("unknown", |identity| identity.2.as_str());
        return Err(format!(
            "formalization records missing Quire criterion {missing}"
        ));
    }
    // Align the supplied producer record with FR-204's derived item state.
    let mut result = Vec::with_capacity(records.len());
    for (identity, (record, _)) in records {
        let row = inventory
            .get(&identity)
            .ok_or_else(|| "joined criterion disappeared".to_owned())?;
        let state = record
            .get("state")
            .and_then(Value::as_str)
            .ok_or_else(|| "validated record omitted state".to_owned())?;
        let reason = record.get("reason").and_then(Value::as_str);
        if row.state == "none"
            && state != "eligible"
            && !(state == "not-applicable" && reason.is_some())
        {
            return Err(format!(
                "FR-204 none criterion {} requires eligible or justified not-applicable",
                row.id
            ));
        }
        if row.state == "draft" && state != "authored" {
            return Err(format!(
                "FR-204 draft criterion {} requires authored",
                row.id
            ));
        }
        result.push(FormalizationRecordView {
            obligation_id: row.id.clone(),
            obligation_declaration: identity.0.clone(),
            scoped_document: identity.1.clone(),
            criterion_id: identity.2.clone(),
            record,
            draft_reason: row.draft_reason.clone(),
        });
    }
    Ok(result)
}

fn formalization_item(id: &str, rules: &[Value]) -> Result<(String, Option<String>), String> {
    let mut declarations = Vec::new();
    for rule in rules {
        let symbol = rule
            .get("symbol")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        let linked = rule
            .get("formalizes")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("formal rule {symbol} omitted formalizes"))?;
        if linked.iter().any(|target| target.as_str() == Some(id)) {
            declarations.push((
                rule.get("state").and_then(Value::as_str).unwrap_or("draft"),
                rule.get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ));
        }
    }
    if declarations.is_empty() {
        return Ok(("none".to_owned(), None));
    }
    if let Some((_, reason)) = declarations.iter().find(|(state, _)| *state == "draft") {
        return Ok(("draft".to_owned(), reason.clone()));
    }
    let analysed = declarations.iter().any(|(state, _)| *state == "analysed");
    Ok((
        if analysed { "analysed" } else { "approved" }.to_owned(),
        None,
    ))
}

fn validate_record(record: &Record) -> Result<(), String> {
    if [
        &record.criterion_id.obligation_declaration,
        &record.criterion_id.scoped_document,
        &record.criterion_id.criterion_id,
    ]
    .iter()
    .any(|part| part.is_empty())
    {
        return Err("FR-071 criterion identity members must be non-empty".to_owned());
    }
    let reason_required = matches!(
        record.state.as_str(),
        "rejected" | "inconclusive" | "unknown" | "unavailable" | "not-applicable"
    );
    let reason_forbidden = matches!(
        record.state.as_str(),
        "eligible"
            | "authored"
            | "parsed"
            | "typechecked"
            | "lowered"
            | "backend_supported"
            | "proved"
    );
    if !matches!(
        record.state.as_str(),
        "eligible"
            | "authored"
            | "parsed"
            | "typechecked"
            | "lowered"
            | "backend_supported"
            | "proved"
            | "inconclusive"
            | "rejected"
            | "unknown"
            | "unavailable"
            | "not-applicable"
    ) {
        return Err(format!("invalid FR-071 state {}", record.state));
    }
    if reason_required && record.reason.as_deref().is_none_or(str::is_empty) {
        return Err(format!(
            "FR-071 state {} requires a non-empty reason",
            record.state
        ));
    }
    if reason_forbidden && record.reason.is_some() {
        return Err(format!("FR-071 state {} forbids reason", record.state));
    }
    let address_required = matches!(
        record.state.as_str(),
        "authored"
            | "parsed"
            | "typechecked"
            | "lowered"
            | "backend_supported"
            | "proved"
            | "rejected"
            | "inconclusive"
    );
    if address_required && record.clause_address.is_none() {
        return Err(format!(
            "FR-071 state {} requires clause_address",
            record.state
        ));
    }
    if matches!(record.state.as_str(), "eligible" | "not-applicable")
        && record.clause_address.is_some()
    {
        return Err(format!(
            "FR-071 state {} forbids clause_address",
            record.state
        ));
    }
    if matches!(record.state.as_str(), "unknown" | "unavailable")
        && record
            .clause_address
            .as_ref()
            .is_some_and(|address| address.clause_id.is_empty())
    {
        return Err("clause_id must be non-empty".to_owned());
    }
    if let Some(address) = &record.clause_address {
        PackageId::new(address.package_identity.clone()).map_err(|error| error.to_string())?;
        filament_contract_model::ClauseId::new(address.clause_id.clone())
            .map_err(|error| error.to_string())?;
        if address.clause_id.is_empty()
            || Identity::from(&address.criterion_id) != Identity::from(&record.criterion_id)
        {
            return Err("clause_address identity must equal record criterion_id".to_owned());
        }
        let start = SourceLocation::new(
            address.source_span.start.source.clone(),
            address.source_span.start.line,
            address.source_span.start.column,
            address.source_span.start.byte_offset,
        )
        .map_err(|e| e.to_string())?;
        let end = SourceLocation::new(
            address.source_span.end.source.clone(),
            address.source_span.end.line,
            address.source_span.end.column,
            address.source_span.end.byte_offset,
        )
        .map_err(|e| e.to_string())?;
        SourceSpan::new(start, end).map_err(|e| e.to_string())?;
    }
    // Invalid producer observations remain evidence gaps. FR-071 preserves
    // the shared envelope, including its validation errors, for inspection.
    let _ = &record.evidence;
    Ok(())
}

fn text_field<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("Quire coverage row requires non-empty {name}"))
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::read_and_join;
    use ix_trace_rs::trace;
    use serde_json::json;

    #[trace("FR-205-AC-8")]
    #[test]
    fn colliding_flat_labels_keep_distinct_source_document_tuples() {
        let file = tempfile::NamedTempFile::new().expect("temporary records file");
        std::fs::write(file.path(), r#"[
          {"criterion_id":{"obligation_declaration":"decl-a","scoped_document":"a.md","criterion_id":"FR-1-AC-1"},"state":"eligible"},
          {"criterion_id":{"obligation_declaration":"decl-b","scoped_document":"b.md","criterion_id":"FR-1-AC-1"},"state":"eligible"}
        ]"#).expect("write records");
        let obligations = json!([
            {"source":"decl-a","document":"a.md","id":"FR-1-AC-1"},
            {"source":"decl-b","document":"b.md","id":"FR-1-AC-1"}
        ]);
        let records = read_and_join(
            file.path().to_str().expect("path is UTF-8"),
            &obligations,
            &[],
        )
        .expect("exact tuple joins");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].obligation_declaration, "decl-a");
        assert_eq!(records[1].obligation_declaration, "decl-b");
    }

    #[trace("FR-205-AC-9")]
    #[test]
    fn clause_package_identity_must_be_a_package_id_string() {
        let input = json!({
            "criterion_id":{"obligation_declaration":"decl","scoped_document":"doc.md","criterion_id":"FR-1-AC-1"},
            "state":"authored",
            "clause_address":{
                "package_identity":17,
                "criterion_id":{"obligation_declaration":"decl","scoped_document":"doc.md","criterion_id":"FR-1-AC-1"},
                "clause_id":"clause-1",
                "source_span":{
                    "start":{"source":{"document":"source.md","revision":1},"line":1,"column":1,"byte_offset":0},
                    "end":{"source":{"document":"source.md","revision":1},"line":1,"column":2,"byte_offset":1}
                }
            }
        });
        assert!(serde_json::from_value::<super::Record>(input).is_err());
    }

    #[trace("FR-205-AC-10")]
    #[test]
    fn state_specific_reason_and_address_rules_are_enforced() {
        let eligible_with_reason = json!({
            "criterion_id":{"obligation_declaration":"decl","scoped_document":"doc.md","criterion_id":"FR-1-AC-1"},
            "state":"eligible","reason":"should be absent"
        });
        let record = serde_json::from_value::<super::Record>(eligible_with_reason)
            .expect("shape is syntactically valid");
        assert!(super::validate_record(&record).is_err());

        let authored_without_address = json!({
            "criterion_id":{"obligation_declaration":"decl","scoped_document":"doc.md","criterion_id":"FR-1-AC-1"},
            "state":"authored"
        });
        let record = serde_json::from_value::<super::Record>(authored_without_address)
            .expect("shape is syntactically valid");
        assert!(super::validate_record(&record).is_err());

        let unknown_without_reason = json!({
            "criterion_id":{"obligation_declaration":"decl","scoped_document":"doc.md","criterion_id":"FR-1-AC-1"},
            "state":"unknown"
        });
        let record = serde_json::from_value::<super::Record>(unknown_without_reason)
            .expect("shape is syntactically valid");
        assert!(super::validate_record(&record).is_err());
    }

    #[trace("FR-205-AC-11")]
    #[test]
    fn invalid_shared_envelope_is_preserved_as_a_record() {
        let envelope = json!({
            "producer_id":"producer",
            "availability":null,
            "observation":{"command":["formalizer"],"elapsed_ms":0,"exit_code":null,"outcome":"not-run","diagnostic_category":null},
            "governing":null,
            "next_action":null,
            "owner":null,
            "boundary_rationale":null,
            "quoin_reference":null,
            "validation_errors":["producer output is invalid"]
        });
        let result: Result<super::Record, _> = serde_json::from_value(json!({
            "criterion_id":{"obligation_declaration":"decl","scoped_document":"doc.md","criterion_id":"FR-1-AC-1"},
            "state":"proved",
            "clause_address":{
                "package_identity":"package",
                "criterion_id":{"obligation_declaration":"decl","scoped_document":"doc.md","criterion_id":"FR-1-AC-1"},
                "clause_id":"clause-1",
                "source_span":{
                    "start":{"source":{"document":"source.md","revision":1},"line":1,"column":1,"byte_offset":0},
                    "end":{"source":{"document":"source.md","revision":1},"line":1,"column":2,"byte_offset":1}
                }
            },
            "evidence":envelope
        }));
        assert!(
            result.is_ok(),
            "shared envelope validation errors remain consumable"
        );
    }
}
