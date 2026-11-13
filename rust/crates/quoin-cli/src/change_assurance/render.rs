// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The plain-text receipt for `quoin change-assurance receipt` and
//! `verify-receipt` (FR-068): outcome, reasons, proofs, and the event writer's
//! claimed approval with its approver, time and meaning together.

pub(super) fn receipt_text(receipt: &serde_json::Value, verified: bool) -> Result<String, String> {
    let record_id = receipt
        .get("record_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "receipt has no record_id".to_owned())?;
    let revision = receipt
        .get("record_revision")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "receipt has no record_revision".to_owned())?;
    let outcome = receipt
        .get("outcome")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "receipt has no outcome".to_owned())?;
    let mut lines = vec![
        if verified {
            format!("receipt for {record_id} revision {revision} verified")
        } else {
            format!("receipt for {record_id} revision {revision}")
        },
        format!("  outcome: {outcome}"),
    ];
    if let Some(reasons) = receipt.get("reasons").and_then(serde_json::Value::as_array) {
        let reasons = reasons
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        if !reasons.is_empty() {
            lines.push(format!("  reasons: {}", reasons.join(", ")));
        }
    }
    if !verified && let Some(proofs) = receipt.get("proofs").and_then(serde_json::Value::as_array) {
        for proof in proofs {
            let id = proof
                .get("proof_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let result = proof
                .get("outcome")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let reasons = proof
                .get("reasons")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            lines.push(if reasons.is_empty() {
                format!("  {id}: {result}")
            } else {
                format!("  {id}: {result} ({})", reasons.join(", "))
            });
        }
    }
    if let Some(claim) = receipt
        .get("decision_event")
        .and_then(|event| event.get("claimed_approval"))
        .filter(|claim| !claim.is_null())
    {
        let text = |name: &str| {
            claim
                .get(name)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
        };
        let meaning = claim
            .get("meaning")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();
        lines.push(
            "  claimed approval, the event writer's claim and not verified by Quoin:".to_owned(),
        );
        lines.push(format!("    approver: {}", text("approver")));
        lines.push(format!("    {}", claimed_time_line(claim)));
        lines.push(format!("    meaning: {meaning}"));
        lines.push(format!(
            "    via {} ({}), tag {}",
            text("signature_kind"),
            text("record_digest"),
            text("tag"),
        ));
        lines.push(
            "    re-verify with Engineering Assurance approval-verify before relying on it"
                .to_owned(),
        );
        lines.push(
            "    trust comes from the relying party, never from the claim's trust, the record or a revision it names; a claim under a record whose signed_approval.required is false is not relied on"
                .to_owned(),
        );
    }
    Ok(lines.join("\n"))
}

/// The line that shows a claimed approval's time: the instant with its source
/// as the signer's claim, or `time missing`. A missing time is never replaced
/// by another instant.
fn claimed_time_line(claim: &serde_json::Value) -> String {
    let member = |name: &str| claim.get(name).and_then(serde_json::Value::as_str);
    match (member("signed_at"), member("signed_at_source")) {
        (Some(instant), Some(source)) => {
            format!("signing time claimed by the signer: {instant} (source: {source})")
        }
        _ => match member("signed_at_missing") {
            Some(reason) => format!("signing time claimed by the signer: time missing ({reason})"),
            None => "signing time claimed by the signer: time missing".to_owned(),
        },
    }
}
