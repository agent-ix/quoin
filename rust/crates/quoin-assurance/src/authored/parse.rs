// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Admitting an untrusted sufficiency decision, and reading the instants the
//! view compares against (FR-047, quoin#436).
//!
//! The predicates; the shape they produce is in [`super::view`] and the
//! walk that consumes them is in [`super::build`].

use crate::argument::{
    ArgumentError, Challenge, ChallengeStatus, exact_keys, instant_epoch_millis, literal,
    optional_string_at, record, reject, string_array, string_at,
};

use super::view::{ChallengeView, ChallengeViewStatus, DecisionState, SufficiencyDecision};

/// `instant(name, value)`: the shared grammar, with the retained message.
pub(super) fn instant_or_reject(name: &str, value: &str) -> Result<i64, ArgumentError> {
    match instant_epoch_millis(value) {
        Some(millis) => Ok(millis),
        None => reject(format!("{name} must be an ISO-8601 instant")),
    }
}

/// `evaluateChallenge(challenge, asOf)`.
///
/// The three tests are an `else if` CHAIN in the retained source, so an open
/// challenge never reaches the reference check and a resolved one never
/// reaches the expiry check. Flattening them into independent `if`s would have
/// changed which reason a reader is given.
pub(super) fn evaluate_challenge(
    challenge: &Challenge,
    as_of: i64,
) -> Result<ChallengeView, ArgumentError> {
    let refs = challenge.resolution_refs.clone().unwrap_or_default();
    let reason = if challenge.status == ChallengeStatus::Open {
        Some("challenge is open".to_owned())
    } else if refs.is_empty() {
        Some("challenge has no resolution reference".to_owned())
    } else if challenge.status == ChallengeStatus::AcceptedRisk {
        match &challenge.expires_at {
            None => Some("accepted risk has no expiry".to_owned()),
            Some(expires_at) if instant_or_reject("expires_at", expires_at)? <= as_of => {
                Some("accepted risk is expired".to_owned())
            }
            Some(_) => None,
        }
    } else {
        None
    };
    Ok(ChallengeView {
        id: challenge.id.clone(),
        target: challenge.target.clone(),
        statement: challenge.statement.clone(),
        owner: challenge.owner.clone(),
        status: if reason.is_some() {
            ChallengeViewStatus::Open
        } else {
            ChallengeViewStatus::Resolved
        },
        declared_status: challenge.status,
        resolution_refs: refs,
        expires_at: challenge.expires_at.clone(),
        reason,
    })
}

/// The ten required keys of a sufficiency decision, plus the one optional.
const DECISION_KEYS: [&str; 11] = [
    "reasoningId",
    "criterion",
    "state",
    "evidenceRefs",
    "decidedBy",
    "authority",
    "decidedAt",
    "expiresAt",
    "sourceRevision",
    "evidenceDigest",
    "rationale",
];

/// `parseSufficiencyDecision(value)`.
///
/// Reads in the retained order, because the ORDER decides which rejection a
/// malformed decision reports and FR-047-AC-5 asserts specific messages.
pub(super) fn parse_sufficiency_decision(
    value: &serde_json::Value,
) -> Result<SufficiencyDecision, ArgumentError> {
    let decision = record("sufficiency decision", Some(value))?;
    // `exactShape(required, optional)` and `exactKeys(keys, optional)` are the
    // same predicate over `required ∪ optional`, so the retained pair maps
    // onto the one helper rather than a second copy of it.
    exact_keys(
        decision,
        "sufficiency decision",
        &DECISION_KEYS,
        &["rationale"],
    )?;
    let state = literal(
        decision.get("state"),
        "decision state",
        &[
            ("satisfied", DecisionState::Satisfied),
            ("open", DecisionState::Open),
        ],
    )?;
    // A decision that says "satisfied" must cite something; one that says
    // "open" may cite nothing, and an empty list is then admissible.
    let evidence_refs = string_array(
        "evidenceRefs",
        decision.get("evidenceRefs"),
        state == DecisionState::Satisfied,
    )?;
    let decided_at = string_at(decision, "decidedAt")?;
    let expires_at = string_at(decision, "expiresAt")?;
    instant_or_reject("decidedAt", &decided_at)?;
    instant_or_reject("expiresAt", &expires_at)?;
    let evidence_digest = string_at(decision, "evidenceDigest")?;
    if !is_sha256_digest(&evidence_digest) {
        return reject("decision evidenceDigest must be sha256:<64 lowercase hex>");
    }
    Ok(SufficiencyDecision {
        reasoning_id: string_at(decision, "reasoningId")?,
        criterion: string_at(decision, "criterion")?,
        state,
        evidence_refs,
        decided_by: string_at(decision, "decidedBy")?,
        authority: string_at(decision, "authority")?,
        decided_at,
        expires_at,
        source_revision: string_at(decision, "sourceRevision")?,
        evidence_digest,
        rationale: optional_string_at(decision, "rationale")?,
    })
}

/// `/^sha256:[0-9a-f]{64}$/`, without a regex engine.
///
/// Lowercase only, and exactly 64: an uppercase digest is a different string
/// to every consumer that compares digests as text, and accepting both here
/// would put the normalisation decision in the wrong place.
fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// `currentDecision(decision, asOf)`: the reason it is not current, or none.
///
/// All three comparisons are against the SAME `asOf` number the assumptions
/// and challenges use, which is why the instant reader is shared rather than
/// re-derived per call site.
pub(super) fn current_decision(
    decision: &SufficiencyDecision,
    as_of: i64,
) -> Result<Option<String>, ArgumentError> {
    let decided_at = instant_or_reject("decidedAt", &decision.decided_at)?;
    let expires_at = instant_or_reject("expiresAt", &decision.expires_at)?;
    if expires_at <= decided_at {
        return Ok(Some("decision expiry is not after decision".to_owned()));
    }
    if decided_at > as_of {
        return Ok(Some("decision is in the future".to_owned()));
    }
    if expires_at <= as_of {
        return Ok(Some("decision is expired".to_owned()));
    }
    Ok(None)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::super::fixtures::{AS_OF, argument, build, decision, request, with};
    use super::super::view::ChallengeViewStatus;
    use crate::argument::parse_assurance_argument;
    use crate::authored::build_authored_argument_view;

    /// The decision-shape half of AC-5 the retained suite reached only through
    /// the unknown-field case.
    ///
    /// Trace: FR-047-AC-5
    /// Provenance: quoin#445
    #[test]
    fn tc_1135_validates_the_decision_maker_timestamps_and_digest() {
        let cases: [(serde_json::Value, &str); 5] = [
            (
                with(&decision(), "decidedBy", serde_json::json!("nobody-900")),
                "sufficiency decision maker nobody-900 is not a declared participant",
            ),
            (
                with(
                    &decision(),
                    "evidenceDigest",
                    serde_json::json!("sha256:zz"),
                ),
                "decision evidenceDigest must be sha256:<64 lowercase hex>",
            ),
            (
                // Uppercase hex is a DIFFERENT string to every consumer that
                // compares digests as text.
                with(
                    &decision(),
                    "evidenceDigest",
                    serde_json::json!(format!("sha256:{}", "B".repeat(64))),
                ),
                "decision evidenceDigest must be sha256:<64 lowercase hex>",
            ),
            (
                with(&decision(), "decidedAt", serde_json::json!("2026-08-01")),
                "decidedAt must be an ISO-8601 instant",
            ),
            (
                // `satisfied` must cite something.
                with(&decision(), "evidenceRefs", serde_json::json!([])),
                "evidenceRefs must be a non-empty array",
            ),
        ];
        for (decision, message) in cases {
            let error = build_authored_argument_view(&request(argument(), vec![decision], AS_OF))
                .expect_err("the case is a rejection");
            assert_eq!(error.0, message);
        }

        // A decision that says "open" may cite nothing, and falls back to the
        // standing sentence when it gives no rationale.
        let view = build(
            argument(),
            vec![with(
                &with(&decision(), "state", serde_json::json!("open")),
                "evidenceRefs",
                serde_json::json!([]),
            )],
            AS_OF,
        );
        assert_eq!(
            view.reasoning[0].criteria[0].reason.as_deref(),
            Some("decision remains open")
        );
    }

    /// Trace: FR-047-AC-7
    /// Provenance: quoin#436, quoin#445
    #[test]
    fn tc_1712_refuses_an_instant_naming_a_day_or_hour_that_does_not_exist() {
        // `Date.parse` ROLLED each of these rather than rejecting it.
        for review_by in [
            "2026-02-30T00:00:00.000Z",
            "2026-06-31T00:00:00.000Z",
            "2025-02-29T00:00:00.000Z",
            "2026-08-15T24:00:00.000Z",
        ] {
            let assumption = with(
                &argument()["assumptions"][0],
                "review_by",
                serde_json::json!(review_by),
            );
            let error = build_authored_argument_view(&request(
                with(&argument(), "assumptions", serde_json::json!([assumption])),
                vec![decision()],
                AS_OF,
            ))
            .expect_err("an impossible instant is a rejection");
            assert_eq!(error.0, "review_by must be an ISO-8601 instant");
        }

        // Why acceptance is not the whole question. February 30 rolls to March
        // 2, which is AFTER this `asOf` — so the assumption used to read as
        // not yet due and the argument reported a status derived from a date
        // nobody can have authored.
        let assumption = with(
            &argument()["assumptions"][0],
            "review_by",
            serde_json::json!("2026-02-30T00:00:00.000Z"),
        );
        let error = build_authored_argument_view(&request(
            with(&argument(), "assumptions", serde_json::json!([assumption])),
            vec![decision()],
            "2026-03-01T00:00:00.000Z",
        ))
        .expect_err("the rollover case is a rejection");
        assert_eq!(error.0, "review_by must be an ISO-8601 instant");

        // The fix tightens impossible dates, not unusual ones.
        let leap = with(
            &argument()["assumptions"][0],
            "review_by",
            serde_json::json!("2028-02-29T00:00:00.000Z"),
        );
        assert!(
            parse_assurance_argument(&with(&argument(), "assumptions", serde_json::json!([leap])))
                .is_ok(),
            "a real leap day stays accepted"
        );
    }

    /// Trace: FR-047-AC-4
    /// Provenance: quoin#445
    #[test]
    fn tc_1134_requires_resolution_evidence_and_a_current_expiry_for_accepted_risk() {
        let risk = with(
            &with(
                &argument()["challenges"][0],
                "status",
                serde_json::json!("accepted-risk"),
            ),
            "resolution_refs",
            serde_json::json!(["decision://risk/900"]),
        );
        let view = build(
            with(&argument(), "challenges", serde_json::json!([risk.clone()])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(view.challenges[0].status, ChallengeViewStatus::Open);
        assert_eq!(
            view.challenges[0].reason.as_deref(),
            Some("accepted risk has no expiry")
        );
        assert_eq!(
            view.top_claim.reasons,
            vec!["one or more challenges are open or no longer current".to_owned()]
        );

        // An expiry that has passed is the same verdict for a different
        // reason; one still ahead of `asOf` resolves it.
        let expired = with(
            &risk,
            "expires_at",
            serde_json::json!("2026-08-10T00:00:00.000Z"),
        );
        let view = build(
            with(&argument(), "challenges", serde_json::json!([expired])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(
            view.challenges[0].reason.as_deref(),
            Some("accepted risk is expired")
        );

        let current = with(
            &risk,
            "expires_at",
            serde_json::json!("2026-12-01T00:00:00.000Z"),
        );
        let view = build(
            with(&argument(), "challenges", serde_json::json!([current])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(view.challenges[0].status, ChallengeViewStatus::Resolved);
        assert_eq!(view.challenges[0].reason, None);

        // A resolved challenge with no reference is open, and says so.
        let mut bare = argument()["challenges"][0].clone();
        bare.as_object_mut().unwrap().remove("resolution_refs");
        let view = build(
            with(&argument(), "challenges", serde_json::json!([bare])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(
            view.challenges[0].reason.as_deref(),
            Some("challenge has no resolution reference")
        );
        assert_eq!(view.challenges[0].resolution_refs, Vec::<String>::new());

        // An open challenge never reaches the reference check.
        let open = with(
            &argument()["challenges"][0],
            "status",
            serde_json::json!("open"),
        );
        let view = build(
            with(&argument(), "challenges", serde_json::json!([open])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(
            view.challenges[0].reason.as_deref(),
            Some("challenge is open")
        );
    }

    /// The other two ways a decision stops being current, which the retained
    /// suite left to the `expired` case alone.
    ///
    /// Trace: FR-047-AC-3
    /// Provenance: quoin#445
    #[test]
    fn tc_1133_a_future_decision_and_an_inverted_expiry_are_also_not_current() {
        let future = build(
            argument(),
            vec![with(
                &with(
                    &decision(),
                    "decidedAt",
                    serde_json::json!("2026-08-20T00:00:00.000Z"),
                ),
                "expiresAt",
                serde_json::json!("2026-09-20T00:00:00.000Z"),
            )],
            AS_OF,
        );
        assert_eq!(
            future.reasoning[0].criteria[0].reason.as_deref(),
            Some("decision is in the future")
        );

        let inverted = build(
            argument(),
            vec![with(
                &with(
                    &decision(),
                    "decidedAt",
                    serde_json::json!("2026-09-01T00:00:00.000Z"),
                ),
                "expiresAt",
                serde_json::json!("2026-09-01T00:00:00.000Z"),
            )],
            AS_OF,
        );
        assert_eq!(
            inverted.reasoning[0].criteria[0].reason.as_deref(),
            Some("decision expiry is not after decision")
        );
    }
}
