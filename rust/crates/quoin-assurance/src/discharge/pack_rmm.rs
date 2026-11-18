// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The pack-level requirements mapping report (FR-140): one FR-135 matrix per
//! clause set of a pack, behind an index of set keys and row counts.
//!
//! This module states no row, compliance, tailoring or escape rule of its own.
//! Every matrix is built and rendered by [`super::rmm`], so the pack view
//! cannot disagree with the single-set view. The report names clauses by id
//! and sets by key; nothing here reads a clause's text or title.

use std::collections::{BTreeMap, BTreeSet};

use quoin_evidence::source::EvidenceSource;
use quoin_quire_types::{ClauseBindingReport, ClauseSetKey};
use serde::{Deserialize, Serialize};

use super::rmm::{
    RequirementsMapping, build_requirements_mapping, escape, render_requirements_mapping,
};
use super::store::{
    DischargeStoreError, discharge_report_from_store,
    discharge_report_from_store_with_approval_requirements,
};

/// The one accepted `schemaVersion` of a pack report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PackRequirementsMappingSchemaVersion {
    /// `requirements-mapping-pack-v1`.
    #[serde(rename = "requirements-mapping-pack-v1")]
    V1,
}

/// Approval-required clauses for one exact clause set, loaded from the module
/// that owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClauseApprovalPolicy {
    /// The clause-set authority.
    pub authority: String,
    /// The clause-set identifier.
    pub id: String,
    /// The exact clause-set version.
    pub version: String,
    /// Clause ids whose metadata declares one or more approval roles.
    pub required_clause_ids: BTreeSet<String>,
}

/// One index entry: a set and the number of rows of its matrix, nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PackIndexEntry {
    /// The clause set.
    pub clause_set: ClauseSetKey,
    /// The number of rows of the set's matrix.
    pub row_count: usize,
}

/// The pack report, never scored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PackRequirementsMapping {
    /// Always `requirements-mapping-pack-v1`.
    pub schema_version: PackRequirementsMappingSchemaVersion,
    /// The installed module name.
    pub pack: String,
    /// The evaluation instant.
    pub as_of: String,
    /// The context every set was evaluated against.
    pub context: BTreeMap<String, String>,
    /// One entry per set, in report order.
    pub index: Vec<PackIndexEntry>,
    /// The `requirements-mapping-v1` matrices, in index order.
    pub matrices: Vec<RequirementsMapping>,
}

/// The set's key as the report names it everywhere: `authority/id@version`.
fn key_text(set: &ClauseSetKey) -> String {
    format!("{}/{}@{}", set.authority, set.id, set.version)
}

/// Build the pack report from each set's live binding report and the facts
/// stored for that set's key.
///
/// Sets are ordered by `authority/id@version`, compared byte-wise. A set with
/// no stored facts is rendered as FR-135 renders a binding report with none.
///
/// # Errors
///
/// [`DischargeStoreError`] as [`discharge_report_from_store`] and
/// [`build_requirements_mapping`] return it; the first failure refuses the
/// whole report.
pub fn build_pack_requirements_mapping<S: EvidenceSource + ?Sized>(
    source: &S,
    pack: &str,
    as_of: &str,
    context: &BTreeMap<String, String>,
    bindings: &[ClauseBindingReport],
    approval_policies: &[ClauseApprovalPolicy],
) -> Result<PackRequirementsMapping, DischargeStoreError> {
    let mut ordered: Vec<(String, &ClauseBindingReport)> = bindings
        .iter()
        .map(|binding| (key_text(&binding.clause_set), binding))
        .collect();
    ordered.sort_by(|a, b| a.0.cmp(&b.0));
    let mut index = Vec::with_capacity(ordered.len());
    let mut matrices = Vec::with_capacity(ordered.len());
    for (_, binding) in ordered {
        let policies = approval_policies.iter().filter(|policy| {
            policy.authority == binding.clause_set.authority
                && policy.id == binding.clause_set.id
                && policy.version == binding.clause_set.version
        });
        let mut matching = policies;
        let policy = matching.next();
        if matching.next().is_some() {
            return Err(DischargeStoreError::Discharge(
                super::report::DischargeError(format!(
                    "multiple approval policies match clause set {}/{}/{}",
                    binding.clause_set.authority, binding.clause_set.id, binding.clause_set.version
                )),
            ));
        }
        let report = if let Some(policy) = policy {
            discharge_report_from_store_with_approval_requirements(
                source,
                binding,
                as_of,
                &policy.required_clause_ids,
            )?
        } else {
            discharge_report_from_store(source, binding, as_of)?
        };
        let matrix = build_requirements_mapping(binding, &report)?;
        index.push(PackIndexEntry {
            clause_set: binding.clause_set.clone(),
            row_count: matrix.rows.len(),
        });
        matrices.push(matrix);
    }
    Ok(PackRequirementsMapping {
        schema_version: PackRequirementsMappingSchemaVersion::V1,
        pack: pack.to_owned(),
        as_of: as_of.to_owned(),
        context: context.clone(),
        index,
        matrices,
    })
}

/// Render the pack report as deterministic Markdown: title, context, index,
/// then each set's FR-135 Markdown in index order.
#[must_use]
pub fn render_pack_requirements_mapping(report: &PackRequirementsMapping) -> String {
    let mut lines = vec![
        format!("# Requirements mapping pack: {}", escape(&report.pack)),
        String::new(),
        format!("- Evaluated as of: {}", escape(&report.as_of)),
        String::new(),
        "## Context".to_owned(),
        String::new(),
    ];
    if report.context.is_empty() {
        lines.push("none".to_owned());
    } else {
        for (key, value) in &report.context {
            lines.push(format!("- {}={}", escape(key), escape(value)));
        }
    }
    lines.push(String::new());
    lines.push("## Index".to_owned());
    lines.push(String::new());
    lines.push("| Clause set | Rows |".to_owned());
    lines.push("| --- | --- |".to_owned());
    for entry in &report.index {
        lines.push(format!(
            "| {} | {} |",
            escape(&key_text(&entry.clause_set)),
            entry.row_count
        ));
    }
    lines.push(String::new());
    let mut text = lines.join("\n");
    for matrix in &report.matrices {
        text.push('\n');
        text.push_str(&render_requirements_mapping(matrix));
    }
    text
}
