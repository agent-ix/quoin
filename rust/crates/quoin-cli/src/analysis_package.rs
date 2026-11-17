// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter from the published checked-package interface to analyzer packages.

use std::collections::BTreeSet;

use filament_contract_model::{
    CheckedNodeId, CheckedNodeKind, CheckedNodeTag, CheckedPackageEvidence,
    CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult, ClaimForm,
    CompleteLoweringProfileV2, CompleteLoweringRecordV2, LoweringProfileRef,
};

const LOWERING_WORK_LIMIT: u64 = 1_000_000;

/// One source interval for an admitted native analysis claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CheckedAnalysisClaimSpan {
    /// Node selected for lowering into the analyzer package.
    pub node_id: CheckedNodeId,
    /// Authority that owns the source document.
    pub authority: String,
    /// Identity of the source document.
    pub identity: String,
    /// Inclusive source byte start.
    pub start: u64,
    /// Exclusive source byte end.
    pub end: u64,
}

/// Reads checked-package claims and their exact admitted source intervals.
pub(crate) fn claim_spans(bytes: &[u8]) -> Result<Vec<CheckedAnalysisClaimSpan>, String> {
    let package = admit_checked_package(bytes)?;
    let mut spans = Vec::new();
    for entry in package.source_map() {
        if package.node_kind(&entry.node_id)
            != Some(CheckedNodeKind::Claim(ClaimForm::AnalysisClaim))
        {
            continue;
        }
        for region in &entry.regions {
            spans.push(CheckedAnalysisClaimSpan {
                node_id: entry.node_id.clone(),
                authority: region.source.authority.to_string(),
                identity: region.source.identity.to_string(),
                start: region.start,
                end: region.end,
            });
        }
    }
    if spans.is_empty() {
        return Err("checked package contains no source-mapped analysis claims".to_owned());
    }
    Ok(spans)
}

/// Lowers selected claims from an admitted checked package for formal analysis.
///
/// The caller supplies explicit selected claim identifiers. Quoin supplies its
/// supported feature evidence to the published reader. A refusal is returned
/// before an analyzer report can be created. Only claims that the published
/// lowering interface fully lowers are included in the returned package bytes.
pub(crate) fn lower_selected_claims(
    bytes: &[u8],
    selected_claims: &[CheckedNodeId],
) -> Result<Vec<u8>, String> {
    if selected_claims.is_empty() {
        return Err("formal-rule analysis requires at least one selected claim".to_owned());
    }
    let mut unique = BTreeSet::new();
    for claim in selected_claims {
        if !unique.insert(claim) {
            return Err("formal-rule claim selection contains a duplicate node".to_owned());
        }
    }

    let package = admit_checked_package(bytes)?;

    let profile = CompleteLoweringProfileV2 {
        reference: LoweringProfileRef {
            authority: "quoin".into(),
            identity: "formal-rule-analysis/v1".into(),
        },
        required_source_selections: Vec::new(),
        supported_tags: CheckedNodeTag::ALL.iter().copied().collect(),
        require_bounds: true,
        work_limit: LOWERING_WORK_LIMIT,
    };
    let lowered = package.lower(selected_claims, &profile);
    for record in &lowered.records {
        if !matches!(record, CompleteLoweringRecordV2::Lowered { .. }) {
            return Err(format!("selected claim could not be lowered: {record:?}"));
        }
    }
    lowered
        .package
        .canonical_bytes()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "lowered contract package exceeded its byte limit".to_owned())
}

fn admit_checked_package(bytes: &[u8]) -> Result<CheckedPackageV2, String> {
    let mut evidence = CheckedPackageEvidence::new();
    evidence.support_feature("filament.value.complete/v1");
    match CheckedPackageV2::read(bytes, CheckedPackageReadLimits::bounded(), &evidence) {
        CheckedPackageV2ReadResult::Admitted(package) => Ok(*package),
        CheckedPackageV2ReadResult::Refused(refusal) => {
            Err(format!("checked package was refused: {refusal}"))
        }
        CheckedPackageV2ReadResult::Incomplete(incomplete) => Err(format!(
            "checked package exceeded a reader limit: {incomplete:?}"
        )),
        CheckedPackageV2ReadResult::CatalogUnavailable(error) => {
            Err(format!("checked-package reader is unavailable: {error}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{claim_spans, lower_selected_claims};

    fn fixture_id(path: &str) -> filament_contract_model::CheckedNodeId {
        serde_json::from_value(json!({
            "package": "test/formal-rules",
            "path": [path],
            "ordinal": 0
        }))
        .expect("fixture node id is valid")
    }

    #[test]
    fn lowers_only_explicit_approved_and_draft_claims() {
        let bytes =
            include_bytes!("../tests/fixtures/formal-rules/checked-package-v2-success.json");
        let spans = claim_spans(bytes).expect("admitted claim spans");
        assert_eq!(spans.len(), 2);
        assert_eq!(
            spans
                .iter()
                .map(|span| span.identity.as_str())
                .collect::<Vec<_>>(),
            ["approved/rule.native", "draft/rule.native"]
        );
        let lowered = lower_selected_claims(
            bytes,
            &[fixture_id("claim/approved"), fixture_id("claim/draft")],
        )
        .expect("fixture lowers");
        let package: Value = serde_json::from_slice(&lowered).expect("lowered JSON");
        assert_eq!(package["contract_version"], "filament.contract-package/v1");
        let claims = package["semantic_graph"]["nodes"]
            .as_array()
            .expect("lowered nodes")
            .iter()
            .filter(|node| node["node_tag"] == "claim")
            .count();
        assert_eq!(claims, 2);
    }

    #[test]
    fn refuses_invalid_checked_package_before_producing_analyzer_input() {
        let bytes =
            include_bytes!("../tests/fixtures/formal-rules/checked-package-v2-invalid.json");
        let refusal = lower_selected_claims(bytes, &[fixture_id("claim/draft")])
            .expect_err("unsupported package is refused");
        assert!(refusal.contains("checked package was refused"));
    }
}
