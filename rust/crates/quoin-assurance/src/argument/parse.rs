// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The seventeen predicates: `parse_assurance_argument` (quoin#384).
//!
//! The validator proper. The shape it admits is in [`super::types`], the
//! readers it reads through are in [`super::read`], and the two host-language
//! grammars it defers to are in [`super::instant`].

use super::instant::is_instant;
use super::read::{
    array_at, exact_keys, is_argument_id, literal, optional_string_at, record, string_array,
    string_at, unique_ids,
};
use super::types::{
    ArgumentKind, ArgumentStatus, Assumption, AssumptionStatus, AssuranceArgument, Challenge,
    ChallengeStatus, Checked, Participant, Reasoning, Relationship, RelationshipType, TopClaim,
    reject,
};

/// The twelve keys the retained implementation admits at the top level.
const ALLOWED: [&str; 12] = [
    "id",
    "title",
    "type",
    "status",
    "owner",
    "profile",
    "top_claim",
    "reasoning",
    "assumptions",
    "participants",
    "challenges",
    "relationships",
];

/// Validate the module-owned `AssuranceArgument` frontmatter contract.
///
/// # Errors
///
/// [`crate::ArgumentError`] for any of the seventeen predicates in the module
/// documentation. The message reproduces the retained implementation's;
/// the acceptance decision is what is contractual.
#[expect(
    clippy::too_many_lines,
    reason = "the retained parseAssuranceArgument is one function and splitting \
              it here would put the reading order somewhere other than the \
              order the oracle validates in, which is what a reader compares"
)]
pub fn parse_assurance_argument(value: &serde_json::Value) -> Checked<AssuranceArgument> {
    let object = record("argument", Some(value))?;
    for key in object.keys() {
        if !ALLOWED.contains(&key.as_str()) {
            return reject(format!("argument has unknown field {key}"));
        }
    }

    let id = string_at(object, "id")?;
    if !is_argument_id(&id) {
        return reject("argument id must match AA-<number>");
    }
    literal(
        object.get("type"),
        "type",
        &[("AssuranceArgument", ArgumentKind::AssuranceArgument)],
    )?;
    let status = literal(
        object.get("status"),
        "status",
        &[
            ("proposed", ArgumentStatus::Proposed),
            ("active", ArgumentStatus::Active),
            ("retired", ArgumentStatus::Retired),
        ],
    )?;
    let profile = string_at(object, "profile")?;
    if !profile.starts_with("ix://") {
        return reject("profile must be an ix:// reference");
    }

    let top = record("top_claim", object.get("top_claim"))?;
    exact_keys(top, "top_claim", &["id", "statement", "subject"], &[])?;

    let mut reasoning = Vec::new();
    for (index, item) in array_at(object, "reasoning")?.iter().enumerate() {
        let name = format!("reasoning[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(
            row,
            &name,
            &["id", "statement", "supports", "sufficiency_criteria"],
            &[],
        )?;
        reasoning.push(Reasoning {
            id: string_at(row, "id")?,
            statement: string_at(row, "statement")?,
            supports: string_at(row, "supports")?,
            sufficiency_criteria: string_array(
                &format!("{name}.sufficiency_criteria"),
                row.get("sufficiency_criteria"),
                true,
            )?,
        });
    }
    if reasoning.is_empty() {
        return reject("reasoning must not be empty");
    }

    let mut assumptions = Vec::new();
    for (index, item) in array_at(object, "assumptions")?.iter().enumerate() {
        let name = format!("assumptions[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(
            row,
            &name,
            &["id", "statement", "owner", "status", "review_by"],
            &[],
        )?;
        let review_by = string_at(row, "review_by")?;
        if !is_instant(&review_by) {
            return reject("review_by must be an ISO-8601 instant");
        }
        assumptions.push(Assumption {
            id: string_at(row, "id")?,
            statement: string_at(row, "statement")?,
            owner: string_at(row, "owner")?,
            status: literal(
                row.get("status"),
                "assumption status",
                &[
                    ("open", AssumptionStatus::Open),
                    ("accepted", AssumptionStatus::Accepted),
                    ("invalidated", AssumptionStatus::Invalidated),
                ],
            )?,
            review_by,
        });
    }

    let mut participants = Vec::new();
    for (index, item) in array_at(object, "participants")?.iter().enumerate() {
        let name = format!("participants[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(
            row,
            &name,
            &["id", "role", "authority", "independence"],
            &[],
        )?;
        participants.push(Participant {
            id: string_at(row, "id")?,
            role: string_at(row, "role")?,
            authority: string_at(row, "authority")?,
            independence: string_at(row, "independence")?,
        });
    }
    if participants.is_empty() {
        return reject("participants must not be empty");
    }

    let mut challenges = Vec::new();
    for (index, item) in array_at(object, "challenges")?.iter().enumerate() {
        let name = format!("challenges[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(
            row,
            &name,
            &[
                "id",
                "target",
                "statement",
                "status",
                "owner",
                "resolution_refs",
                "expires_at",
            ],
            &["resolution_refs", "expires_at"],
        )?;
        // `optionalStringAt` keys off PRESENCE, so an explicit null is a hard
        // error here...
        let expires_at = optional_string_at(row, "expires_at")?;
        if let Some(instant) = &expires_at
            && !is_instant(instant)
        {
            return reject("expires_at must be an ISO-8601 instant");
        }
        // ...while this one keys off TRUTHINESS, so an explicit null is
        // silently absent. The asymmetry is the retained implementation's and
        // is reproduced rather than tidied.
        let resolution_refs = match row.get("resolution_refs") {
            None | Some(serde_json::Value::Null) => None,
            Some(_) => Some(string_array(
                &format!("{name}.resolution_refs"),
                row.get("resolution_refs"),
                true,
            )?),
        };
        challenges.push(Challenge {
            id: string_at(row, "id")?,
            target: string_at(row, "target")?,
            statement: string_at(row, "statement")?,
            status: literal(
                row.get("status"),
                "challenge status",
                &[
                    ("open", ChallengeStatus::Open),
                    ("resolved", ChallengeStatus::Resolved),
                    ("accepted-risk", ChallengeStatus::AcceptedRisk),
                ],
            )?,
            owner: string_at(row, "owner")?,
            resolution_refs,
            expires_at,
        });
    }

    let mut relationships = Vec::new();
    for (index, item) in array_at(object, "relationships")?.iter().enumerate() {
        let name = format!("relationships[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(row, &name, &["target", "type"], &[])?;
        let target = string_at(row, "target")?;
        if !target.starts_with("ix://") {
            return reject(format!("{name}.target must be an ix:// reference"));
        }
        relationships.push(Relationship {
            target,
            kind: literal(
                row.get("type"),
                "relationship type",
                &[
                    ("supports", RelationshipType::Supports),
                    ("challenges", RelationshipType::Challenges),
                    ("references", RelationshipType::References),
                ],
            )?,
        });
    }

    unique_ids(
        "reasoning",
        &reasoning.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
    )?;
    unique_ids(
        "assumptions",
        &assumptions
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    unique_ids(
        "participants",
        &participants
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    unique_ids(
        "challenges",
        &challenges.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
    )?;

    let top_claim_id = string_at(top, "id")?;
    let mut node_ids: Vec<&str> = vec![top_claim_id.as_str()];
    node_ids.extend(reasoning.iter().map(|r| r.id.as_str()));
    node_ids.extend(assumptions.iter().map(|a| a.id.as_str()));
    {
        let mut seen: Vec<&str> = Vec::with_capacity(node_ids.len());
        for id in &node_ids {
            if seen.contains(id) {
                return reject("top claim, reasoning, and assumption ids must be unique");
            }
            seen.push(id);
        }
    }

    // Every chain of `supports` reaches the top claim. Cycle detection is per
    // START NODE, matching the retained `visited` set: a node reachable from
    // two starts is walked twice, which is correct — being visited on another
    // node's walk is not evidence that THIS node reaches the claim. Same
    // distinction FR-040 records for shared sub-claims (SR-007 FND-001).
    for start in &reasoning {
        let mut cursor = start;
        let mut visited: Vec<&str> = Vec::new();
        while cursor.supports != top_claim_id {
            if visited.contains(&cursor.id.as_str()) {
                return reject(format!(
                    "reasoning cycle does not reach top claim from {}",
                    start.id
                ));
            }
            visited.push(cursor.id.as_str());
            let Some(parent) = reasoning.iter().find(|r| r.id == cursor.supports) else {
                return reject(format!(
                    "reasoning {} supports unknown target {}",
                    start.id, cursor.supports
                ));
            };
            cursor = parent;
        }
    }

    for challenge in &challenges {
        if !node_ids.contains(&challenge.target.as_str()) {
            return reject(format!(
                "challenge {} targets unknown argument node {}",
                challenge.id, challenge.target
            ));
        }
    }

    Ok(AssuranceArgument {
        id,
        title: string_at(object, "title")?,
        kind: ArgumentKind::AssuranceArgument,
        status,
        owner: string_at(object, "owner")?,
        profile,
        top_claim: TopClaim {
            id: top_claim_id,
            statement: string_at(top, "statement")?,
            subject: string_at(top, "subject")?,
        },
        reasoning,
        assumptions,
        participants,
        challenges,
        relationships,
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::parse_assurance_argument;

    fn base() -> serde_json::Value {
        serde_json::json!({
            "id": "AA-900",
            "title": "Synthetic widget release decision",
            "type": "AssuranceArgument",
            "status": "active",
            "owner": "release-owner",
            "profile": "ix://example.invalid/widget/AP-900",
            "top_claim": {
                "id": "CLAIM-900",
                "statement": "The bounded synthetic widget change is acceptable.",
                "subject": "widget revision 0123456789abcdef"
            },
            "reasoning": [{
                "id": "ARG-900",
                "statement": "Argue from the explicitly reviewed clause disposition.",
                "supports": "CLAIM-900",
                "sufficiency_criteria": ["Every binding clause has a disposition."]
            }],
            "assumptions": [],
            "participants": [{
                "id": "reviewer-900",
                "role": "decision reviewer",
                "authority": "may accept or reject this synthetic release",
                "independence": "did not produce the implementation evidence"
            }],
            "challenges": [],
            "relationships": []
        })
    }

    #[test]
    fn accepts_the_minimal_authored_argument() {
        let parsed = parse_assurance_argument(&base()).expect("the minimal argument is valid");
        assert_eq!(parsed.id, "AA-900");
        assert_eq!(parsed.reasoning.len(), 1);
    }

    #[test]
    fn an_owner_of_one_byte_order_mark_is_refused() {
        let mut argument = base();
        argument["owner"] = serde_json::json!("\u{FEFF}");
        assert!(parse_assurance_argument(&argument).is_err());
    }

    #[test]
    fn an_owner_of_one_next_line_is_accepted() {
        let mut argument = base();
        argument["owner"] = serde_json::json!("\u{0085}");
        assert!(
            parse_assurance_argument(&argument).is_ok(),
            "U+0085 is not whitespace to JavaScript, so the oracle accepts it"
        );
    }

    #[test]
    fn an_explicit_null_is_asymmetric_across_the_two_optional_fields() {
        let challenge = |extra: serde_json::Value| {
            let mut argument = base();
            let mut row = serde_json::json!({
                "id": "CH-900",
                "target": "CLAIM-900",
                "statement": "A bounded recovery case needed review.",
                "status": "open",
                "owner": "release-owner"
            });
            for (key, value) in extra.as_object().unwrap() {
                row[key] = value.clone();
            }
            argument["challenges"] = serde_json::json!([row]);
            parse_assurance_argument(&argument)
        };

        // `optionalStringAt` keys off `key in object`, so the key IS present
        // and reaches the string check.
        assert!(
            challenge(serde_json::json!({ "expires_at": null })).is_err(),
            "an explicit null expires_at must be refused"
        );
        // Truthiness, so null is indistinguishable from absent.
        let parsed = challenge(serde_json::json!({ "resolution_refs": null }))
            .expect("an explicit null resolution_refs is silently absent");
        assert_eq!(parsed.challenges[0].resolution_refs, None);
    }

    #[test]
    fn a_cycle_is_detected_per_start_node_rather_than_globally() {
        let mut argument = base();
        argument["reasoning"] = serde_json::json!([
            {"id": "A", "statement": "a", "supports": "B", "sufficiency_criteria": ["c"]},
            {"id": "B", "statement": "b", "supports": "A", "sufficiency_criteria": ["c"]}
        ]);
        assert!(parse_assurance_argument(&argument).is_err());
    }

    #[test]
    fn a_shared_intermediate_step_is_walked_from_both_starts() {
        // Neither start is a cycle, and a GLOBAL visited set would have made
        // the second walk look like one.
        let mut argument = base();
        argument["reasoning"] = serde_json::json!([
            {"id": "A", "statement": "a", "supports": "M", "sufficiency_criteria": ["c"]},
            {"id": "B", "statement": "b", "supports": "M", "sufficiency_criteria": ["c"]},
            {"id": "M", "statement": "m", "supports": "CLAIM-900", "sufficiency_criteria": ["c"]}
        ]);
        assert!(parse_assurance_argument(&argument).is_ok());
    }

    #[test]
    fn an_unlisted_status_is_refused_because_the_retained_code_checks() {
        // The other half of quoin#425. `Finding::kind` stays a String because
        // the renderer interpolates; this is checked, so it is closed.
        let mut argument = base();
        argument["status"] = serde_json::json!("archived");
        assert!(parse_assurance_argument(&argument).is_err());
    }
}
