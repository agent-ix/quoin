// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Ordering the members of a document that has not been validated.
//!
//! Runs before a record is read, so it assumes no shape: every helper below
//! leaves a member it does not recognise exactly where it found it. Sorting
//! here is what makes two callers who listed the same scope in a different
//! order seal to the same digest.

use quoin_store::JsonValue;

use crate::model::json::{compare_utf16, sort_utf16, string_values};

/// Sort every collection a record declares, as `normalizeRecord` does.
///
/// Operates on the JSON rather than on a typed value because the oracle sorts
/// **before** validating: an input whose collections are out of order is
/// accepted by `sealChangeRecord` and refused by `verifyChangeRecord`, and a
/// port that normalized after parsing could never reproduce that.
///
/// Shapes this does not recognise are left alone; the validation pass that
/// follows is what refuses them, and it produces the better message.
#[must_use]
pub fn normalize_record(value: &JsonValue) -> JsonValue {
    let JsonValue::Object(root) = value else {
        return value.clone();
    };
    let mut normalized = root.clone();
    normalized.remove("digest");
    if let Some(JsonValue::Object(subject)) = normalized.get("subject") {
        let mut subject = subject.clone();
        sort_string_member(&mut subject, "scope");
        normalized.set("subject", JsonValue::Object(subject));
    }
    sort_object_array(&mut normalized, "source_connections", "source_id");
    if let Some(JsonValue::Object(impact)) = normalized.get("impact_snapshot") {
        let mut impact = impact.clone();
        sort_string_member(&mut impact, "gaps");
        normalized.set("impact_snapshot", JsonValue::Object(impact));
    }
    if let Some(JsonValue::Object(definition)) = normalized.get("definition") {
        let mut definition = definition.clone();
        sort_object_array(&mut definition, "requirements", "id");
        sort_object_array(&mut definition, "preservation_constraints", "id");
        sort_object_array(&mut definition, "proof_obligations", "proof_id");
        sort_object_array(&mut definition, "unknowns", "id");
        sort_nested_string_member(&mut definition, "requirements", "source_ids");
        sort_nested_string_member(&mut definition, "preservation_constraints", "source_ids");
        sort_nested_string_member(&mut definition, "proof_obligations", "obligation_ids");
        normalized.set("definition", JsonValue::Object(definition));
    }
    JsonValue::Object(normalized)
}

fn sort_string_member(owner: &mut quoin_store::JsonObject, member: &str) {
    let Some(JsonValue::Array(entries)) = owner.get(member) else {
        return;
    };
    let mut texts: Vec<String> = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(text) = entry.as_str() else {
            return;
        };
        texts.push(text.to_owned());
    }
    sort_utf16(&mut texts);
    owner.set(member, string_values(&texts));
}

fn sort_object_array(owner: &mut quoin_store::JsonObject, member: &str, key: &str) {
    let Some(JsonValue::Array(entries)) = owner.get(member) else {
        return;
    };
    let mut sortable: Vec<(String, JsonValue)> = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(text) = entry
            .as_object()
            .ok()
            .and_then(|fields| fields.get(key))
            .and_then(JsonValue::as_str)
        else {
            return;
        };
        sortable.push((text.to_owned(), entry.clone()));
    }
    sortable.sort_by(|left, right| compare_utf16(&left.0, &right.0));
    owner.set(
        member,
        JsonValue::Array(sortable.into_iter().map(|(_, entry)| entry).collect()),
    );
}

fn sort_nested_string_member(owner: &mut quoin_store::JsonObject, member: &str, nested: &str) {
    let Some(JsonValue::Array(entries)) = owner.get(member) else {
        return;
    };
    let mut rebuilt = Vec::with_capacity(entries.len());
    for entry in entries {
        match entry {
            JsonValue::Object(fields) => {
                let mut fields = fields.clone();
                sort_string_member(&mut fields, nested);
                rebuilt.push(JsonValue::Object(fields));
            }
            other => rebuilt.push(other.clone()),
        }
    }
    owner.set(member, JsonValue::Array(rebuilt));
}
