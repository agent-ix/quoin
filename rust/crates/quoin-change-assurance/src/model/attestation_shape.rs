// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Shape checks for the two fields FR-064 requires be immutable and real:
//! `tool.version` and `observed_at`.
//!
//! Split out of `model/attestation.rs` (quoin#464, quoin#457's precedent for
//! `model/record`) so that module stays under the 700-line hard ceiling after
//! PLAT-972 added the `strength` member; nothing here changed behaviour.

/// Whether `version` is an exact semantic version, the one spelling the
/// schema admits for a tool identity.
///
/// `^v?\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$`.
#[must_use]
pub fn is_an_immutable_version(version: &str) -> bool {
    let core = version.strip_prefix('v').unwrap_or(version);
    let (numbers, suffix) = match core.find(['-', '+']) {
        Some(at) => {
            let (head, tail) = core.split_at(at);
            (head, Some(&tail[1..]))
        }
        None => (core, None),
    };
    let mut parts = numbers.split('.');
    let triple = [parts.next(), parts.next(), parts.next()];
    if parts.next().is_some() {
        return false;
    }
    for part in triple {
        match part {
            Some(digits) if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {}
            _ => return false,
        }
    }
    match suffix {
        None => true,
        Some(tail) => {
            !tail.is_empty()
                && tail
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
        }
    }
}

/// Whether `observed` is both the oracle's timestamp shape and an instant a
/// calendar has.
///
/// The oracle runs the regex **and** `Date.parse`, and the second refuses what
/// the first cannot see: `2026-02-30T00:00:00Z` matches the pattern and is not
/// a date. This is the whole of that pair, including ECMAScript's admission of
/// hour `24` only at exactly midnight.
#[must_use]
pub fn is_a_real_instant(observed: &str) -> bool {
    quoin_store::instant::Instant::parse(
        observed,
        quoin_store::instant::InstantGrammar::CalendarInstantEndOfDay,
    )
    .is_some()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use super::{is_a_real_instant, is_an_immutable_version};

    #[test]
    fn a_tool_version_is_an_exact_semantic_version_and_nothing_else() {
        assert!(is_an_immutable_version("1.2.3"));
        assert!(is_an_immutable_version("v1.2.3"));
        assert!(is_an_immutable_version("1.2.3-rc.1"));
        assert!(is_an_immutable_version("1.2.3+build.5"));
        assert!(!is_an_immutable_version(&"a".repeat(40)));
        assert!(!is_an_immutable_version(&"0".repeat(64)));
        assert!(!is_an_immutable_version("latest"));
        assert!(!is_an_immutable_version("main"));
        assert!(!is_an_immutable_version("1.2"));
        assert!(!is_an_immutable_version("1.2.3."));
        assert!(!is_an_immutable_version("1.2.3-"));
        assert!(!is_an_immutable_version(&"A".repeat(40)));
    }

    #[test]
    fn a_timestamp_must_be_an_instant_a_calendar_has() {
        assert!(is_a_real_instant("2026-02-28T00:00:00Z"));
        assert!(is_a_real_instant("2024-02-29T23:59:59.123Z"));
        assert!(is_a_real_instant("2026-01-01T00:00:00+05:30"));
        assert!(is_a_real_instant("2026-01-01T24:00:00Z"));
        assert!(!is_a_real_instant("2026-02-30T00:00:00Z"));
        assert!(!is_a_real_instant("2026-13-01T00:00:00Z"));
        assert!(!is_a_real_instant("2026-01-01T24:00:01Z"));
        assert!(!is_a_real_instant("2026-01-01T00:60:00Z"));
        assert!(!is_a_real_instant("2026-01-01T00:00:00"));
        assert!(!is_a_real_instant("2026-01-01T00:00:00+24:00"));
        assert!(!is_a_real_instant("2026-01-01 00:00:00Z"));
    }
}
