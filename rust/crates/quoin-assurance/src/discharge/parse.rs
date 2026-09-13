// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Admitting an untrusted discharge fact (FR-046, quoin#436).
//!
//! Every predicate the retained `parseFact`/`parseAttestation` pair applies,
//! plus the readers they read through. The shape they produce is in
//! [`super::report`].

use crate::argument::{instant_epoch_millis, js_trim_is_empty};

use super::report::{
    Checked, DirectDischargeFact, DischargeAttestation, DischargeFact, DispositionDecision,
    DispositionFact, FactKind, reject,
};

/// `parseFact(value)`.
///
/// The reading order is the retained one and is load-bearing for which message
/// a caller sees: `kind`, then `clauseId`, then the whole attestation, and
/// only THEN the exact-keys check. So a fact carrying both an unknown field
/// and an empty `authority` is refused for the authority.
pub(super) fn parse_fact(value: &serde_json::Value) -> Checked<DischargeFact> {
    let fact = object("discharge fact", Some(value))?;
    let kind = one_of_wire(fact.get("kind"), "kind", FactKind::all(), FactKind::as_str)?;
    let clause_id = string_value("clauseId", fact.get("clauseId"))?;
    let attestation = parse_attestation(fact.get("attestation"))?;
    match kind {
        FactKind::Direct => {
            exact(
                fact,
                "direct discharge fact",
                &["kind", "clauseId", "evidenceRefs", "attestation"],
            )?;
            Ok(DischargeFact::Direct(DirectDischargeFact {
                clause_id,
                evidence_refs: non_empty_list("evidenceRefs", fact.get("evidenceRefs"))?,
                attestation,
            }))
        }
        FactKind::Disposition => {
            exact(
                fact,
                "disposition discharge fact",
                &[
                    "kind",
                    "clauseId",
                    "decision",
                    "rationale",
                    "approvalRef",
                    "attestation",
                ],
            )?;
            Ok(DischargeFact::Disposition(DispositionFact {
                clause_id,
                decision: one_of_wire(
                    fact.get("decision"),
                    "decision",
                    DispositionDecision::all(),
                    DispositionDecision::as_str,
                )?,
                rationale: string_value("rationale", fact.get("rationale"))?,
                approval_ref: string_value("approvalRef", fact.get("approvalRef"))?,
                attestation,
            }))
        }
    }
}

/// `parseAttestation(value)`.
pub(super) fn parse_attestation(
    value: Option<&serde_json::Value>,
) -> Checked<DischargeAttestation> {
    let attestation = object("attestation", value)?;
    exact(
        attestation,
        "attestation",
        &[
            "attestedBy",
            "authority",
            "attestedAt",
            "expiresAt",
            "sourceRevision",
            "evidenceDigest",
        ],
    )?;
    let attested_by = string_value("attestedBy", attestation.get("attestedBy"))?;
    let authority = string_value("authority", attestation.get("authority"))?;
    let source_revision = string_value("sourceRevision", attestation.get("sourceRevision"))?;
    let attested_at = string_value("attestedAt", attestation.get("attestedAt"))?;
    let expires_at = string_value("expiresAt", attestation.get("expiresAt"))?;
    instant("attestedAt", &attested_at)?;
    instant("expiresAt", &expires_at)?;
    let evidence_digest = string_value("evidenceDigest", attestation.get("evidenceDigest"))?;
    if !is_sha256_digest(&evidence_digest) {
        return reject("attestation evidenceDigest must be sha256:<64 lowercase hex>");
    }
    Ok(DischargeAttestation {
        attested_by,
        authority,
        attested_at,
        expires_at,
        source_revision,
        evidence_digest,
    })
}

/// `/^sha256:[0-9a-f]{64}$/`, without a regex engine.
///
/// Lowercase only: `is_ascii_hexdigit` would accept `SHA` in uppercase hex and
/// silently widen the digest alphabet on a field whose whole job is identity.
fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// `currentAttestation(attestation, asOf)`: `None` when the attestation is
/// current, otherwise the reason it is not.
///
/// # Errors
///
/// Never in practice — both instants were validated by `parseAttestation` —
/// but the retained code re-parses here too, so the fallible signature is
/// kept rather than a panic being introduced where the oracle has none.
pub(super) fn current_attestation(
    attestation: &DischargeAttestation,
    as_of: i64,
) -> Checked<Option<String>> {
    let attested_at = instant("attestedAt", &attestation.attested_at)?;
    let expires_at = instant("expiresAt", &attestation.expires_at)?;
    if expires_at <= attested_at {
        return Ok(Some(
            "attestation expiry is not after attestation".to_owned(),
        ));
    }
    if attested_at > as_of {
        return Ok(Some("attestation is in the future".to_owned()));
    }
    if expires_at <= as_of {
        return Ok(Some("discharge fact is expired".to_owned()));
    }
    Ok(None)
}

/// `instant(name, value)`: the shape and calendar gate, then the number.
///
/// [`instant_epoch_millis`] is the whole implementation. The retained
/// `instant()` in `discharge.ts` and the one in `argument.ts` are the same
/// eight lines — the same regex, the same finiteness check — so a second
/// parser here would be a second thing to keep in agreement with quoin#436's
/// impossible-day fix, on the exact predicate whose divergence that ticket
/// records.
pub(super) fn instant(name: &str, value: &str) -> Checked<i64> {
    match instant_epoch_millis(value) {
        Some(millis) => Ok(millis),
        None => reject(format!("{name} must be an ISO-8601 instant")),
    }
}

/// `object(name, value)`: an object, and not an array.
fn object<'a>(
    name: &str,
    value: Option<&'a serde_json::Value>,
) -> Checked<&'a serde_json::Map<String, serde_json::Value>> {
    match value.and_then(serde_json::Value::as_object) {
        Some(object) => Ok(object),
        None => reject(format!("{name} must be an object")),
    }
}

/// `exact(value, name, fields)`: no unknown key, and none missing.
fn exact(
    value: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    fields: &[&str],
) -> Checked<()> {
    // `serde_json::Map` is a `BTreeMap` here (`preserve_order` is deliberately
    // off, rust-style), so two unknown keys are reported in sorted order where
    // the retained code reports them in insertion order. The ACCEPTANCE
    // decision is identical; only which of several bad keys is named differs,
    // and quoin#373 records that verdicts are contractual and prose is not.
    for key in value.keys() {
        if !fields.contains(&key.as_str()) {
            return reject(format!("{name} has unknown field {key}"));
        }
    }
    for field in fields {
        if !value.contains_key(*field) {
            return reject(format!("{name} is missing {field}"));
        }
    }
    Ok(())
}

/// `stringValue(name, value)`: a string, non-empty once JavaScript's `trim`
/// has run — and the UNTRIMMED value is what comes back.
///
/// Both halves matter. `"  reviewer-1  "` is accepted and stored with its
/// spaces, so a port that returned `value.trim()` would have emitted a
/// different `attestedBy` into the report than the oracle does. And the
/// emptiness test is JavaScript's set, not Rust's: see
/// [`crate::argument::js_trim_is_empty`].
///
/// One message for both failures, unlike [`crate::argument`]'s `string_at`,
/// which splits them — the retained `stringValue` really does say
/// `must not be empty` for a number.
fn string_value(name: &str, value: Option<&serde_json::Value>) -> Checked<String> {
    match value.and_then(serde_json::Value::as_str) {
        Some(text) if !js_trim_is_empty(text) => Ok(text.to_owned()),
        _ => reject(format!("{name} must not be empty")),
    }
}

/// `nonEmptyList(name, value)`: a non-empty array of non-empty, unique strings.
fn non_empty_list(name: &str, value: Option<&serde_json::Value>) -> Checked<Vec<String>> {
    let Some(array) = value.and_then(serde_json::Value::as_array) else {
        return reject(format!("{name} must contain non-empty values"));
    };
    if array.is_empty() {
        return reject(format!("{name} must contain non-empty values"));
    }
    let mut values = Vec::with_capacity(array.len());
    for item in array {
        values.push(string_value(name, Some(item))?);
    }
    let mut seen: Vec<&String> = Vec::with_capacity(values.len());
    for text in &values {
        if seen.contains(&text) {
            return reject(format!("{name} must contain unique values"));
        }
        seen.push(text);
    }
    Ok(values)
}

/// `oneOf` over a closed enum's OWN spellings.
///
/// The membership table is built from `all()` and `as_str()` rather than
/// restated beside the call. quoin#447 found both tables written out a second
/// time here: nothing linked them to the `#[serde(rename_all)]` that emits the
/// same words, so flipping `DispositionDecision` from `snake_case` to
/// `camelCase` would have left every accept-side test green while the emitted
/// `clause-discharge-v1` document started saying `temporaryException`. Accept
/// and emit now read the same function, and `tc_447_452` pins that function
/// against what serde writes.
fn one_of_wire<T: Copy>(
    value: Option<&serde_json::Value>,
    name: &str,
    all: &[T],
    spelling: fn(T) -> &'static str,
) -> Checked<T> {
    let allowed: Vec<(&'static str, T)> = all.iter().map(|item| (spelling(*item), *item)).collect();
    one_of(value, name, &allowed)
}

/// `oneOf(value, name, allowed)`: membership, checked at run time.
fn one_of<T: Copy>(
    value: Option<&serde_json::Value>,
    name: &str,
    allowed: &[(&str, T)],
) -> Checked<T> {
    if let Some(text) = value.and_then(serde_json::Value::as_str) {
        for (candidate, mapped) in allowed {
            if *candidate == text {
                return Ok(*mapped);
            }
        }
    }
    let names: Vec<&str> = allowed.iter().map(|(name, _)| *name).collect();
    reject(format!("{name} must be one of {}", names.join(", ")))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::super::fixtures::digest;
    use super::{instant, is_sha256_digest, string_value};

    /// `currentAttestation` orders instants, so `instant()` must return the
    /// same NUMBER `Date.parse` does, not merely agree about validity.
    ///
    /// The literals are `Date.parse`'s own output, read out of node and
    /// written down — a re-derivation would agree with itself no matter what
    /// the code did.
    /// Trace: FR-046-AC-1, FR-046-AC-4
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_430_instant_returns_the_number_date_parse_returns() {
        let parsed = |value: &str| instant("asOf", value).expect("a valid instant");
        assert_eq!(parsed("1970-01-01T00:00:00Z"), 0);
        assert_eq!(parsed("2026-08-15T00:00:00.000Z"), 1_786_752_000_000);
        assert_eq!(parsed("2026-08-01T00:00:00.000Z"), 1_785_542_400_000);
        assert_eq!(parsed("1969-12-31T23:59:59.999Z"), -1);
        assert_eq!(parsed("2028-02-29T00:00:00Z"), 1_835_395_200_000);
        // `+05:30` is ahead of UTC, so the UTC instant is earlier than the
        // digits read. Getting this sign backwards would move an expiry by
        // twice the offset and silently reopen or discharge a clause.
        assert_eq!(parsed("2026-08-15T05:30:00+05:30"), 1_786_752_000_000);
        assert_eq!(parsed("2026-08-14T19:00:00-05:00"), 1_786_752_000_000);
        // A `Date` holds integer milliseconds; the rest is truncated.
        assert_eq!(parsed("2026-08-15T00:00:00.0009Z"), 1_786_752_000_000);
        // quoin#436: `Date.parse` ROLLED this to March 2 rather than refusing.
        assert_eq!(
            instant("asOf", "2026-02-30T00:00:00Z")
                .expect_err("an impossible day")
                .0,
            "asOf must be an ISO-8601 instant"
        );
    }

    /// The retained `stringValue` trims only to TEST; it returns the original.
    /// Trace: FR-046-AC-1, FR-046-AC-5
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_431_string_value_tests_the_trimmed_string_and_returns_the_untrimmed_one() {
        let padded = serde_json::json!("  reviewer-1  ");
        assert_eq!(
            string_value("attestedBy", Some(&padded)).expect("padding is not emptiness"),
            "  reviewer-1  "
        );
        // JavaScript trims U+FEFF and Rust does not: the oracle refuses this.
        assert!(string_value("attestedBy", Some(&serde_json::json!("\u{FEFF}"))).is_err());
        // JavaScript does NOT trim U+0085 and Rust does: the oracle accepts it.
        assert!(string_value("attestedBy", Some(&serde_json::json!("\u{0085}"))).is_ok());
        // A non-string gets the same message the retained code gives.
        assert_eq!(
            string_value("authority", Some(&serde_json::json!(7)))
                .unwrap_err()
                .0,
            "authority must not be empty"
        );
    }

    /// Trace: FR-046-AC-1, FR-046-AC-5
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_432_the_digest_alphabet_is_lowercase_hex_only() {
        assert!(is_sha256_digest(&digest()));
        assert!(!is_sha256_digest(&format!("sha256:{}", "A".repeat(64))));
        assert!(!is_sha256_digest(&format!("sha256:{}", "a".repeat(63))));
        assert!(!is_sha256_digest("not-a-digest"));
    }
}
