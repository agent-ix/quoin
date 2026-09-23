// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Shape checks for the two fields FR-064 requires be immutable and real:
//! `tool.version` and `observed_at`.
//!
//! Split out of `model/attestation.rs` (quoin#464, quoin#457's precedent for
//! `model/record`) so that module stays under the 700-line hard ceiling after
//! PLAT-972 added the `strength` member; nothing here changed behaviour.

/// Whether `version` is one of the three immutable spellings the oracle admits.
///
/// `^(?:v?\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|[a-f0-9]{64})$`.
#[must_use]
pub fn is_an_immutable_version(version: &str) -> bool {
    if is_lowercase_hex(version, 40) || is_lowercase_hex(version, 64) {
        return true;
    }
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

fn is_lowercase_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
    let bytes = observed.as_bytes();
    let digits = |from: usize, count: usize| -> Option<u32> {
        let slice = observed.get(from..from.checked_add(count)?)?;
        if slice.len() == count && slice.bytes().all(|byte| byte.is_ascii_digit()) {
            slice.parse().ok()
        } else {
            None
        }
    };
    if bytes.len() < 20 {
        return false;
    }
    let (Some(year), Some(month), Some(day)) = (digits(0, 4), digits(5, 2), digits(8, 2)) else {
        return false;
    };
    let (Some(hour), Some(minute), Some(second)) = (digits(11, 2), digits(14, 2), digits(17, 2))
    else {
        return false;
    };
    if observed.get(4..5) != Some("-")
        || observed.get(7..8) != Some("-")
        || observed.get(10..11) != Some("T")
        || observed.get(13..14) != Some(":")
        || observed.get(16..17) != Some(":")
    {
        return false;
    }
    let Some(rest) = observed.get(19..) else {
        return false;
    };
    let (fraction, zone) = match rest.strip_prefix('.') {
        Some(tail) => {
            let end = tail
                .find(|character: char| !character.is_ascii_digit())
                .unwrap_or(tail.len());
            if end == 0 {
                return false;
            }
            let (Some(digits), Some(zone)) = (tail.get(..end), tail.get(end..)) else {
                return false;
            };
            (digits, zone)
        }
        None => ("", rest),
    };
    if !is_a_valid_zone(zone) {
        return false;
    }
    if month == 0 || month > 12 || day == 0 || day > days_in_month(year, month) {
        return false;
    }
    if minute > 59 || second > 59 {
        return false;
    }
    // ECMAScript's Date Time String Format admits hour 24, and only as exactly
    // midnight: `T24:00:00` is the following day, `T24:00:01` is not a time.
    if hour > 24
        || (hour == 24 && (minute != 0 || second != 0 || fraction.bytes().any(|b| b != b'0')))
    {
        return false;
    }
    true
}

fn is_a_valid_zone(zone: &str) -> bool {
    if zone == "Z" {
        return true;
    }
    let Some(rest) = zone.strip_prefix(['+', '-']) else {
        return false;
    };
    if rest.len() != 5 || rest.get(2..3) != Some(":") {
        return false;
    }
    let (Some(hours), Some(minutes)) = (rest.get(..2), rest.get(3..)) else {
        return false;
    };
    let digits = |text: &str| -> Option<u32> {
        if text.bytes().all(|byte| byte.is_ascii_digit()) {
            text.parse().ok()
        } else {
            None
        }
    };
    matches!((digits(hours), digits(minutes)), (Some(hours), Some(minutes)) if hours <= 23 && minutes <= 59)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
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
    fn immutable_versions_are_releases_commits_and_digests_and_nothing_else() {
        assert!(is_an_immutable_version("1.2.3"));
        assert!(is_an_immutable_version("v1.2.3"));
        assert!(is_an_immutable_version("1.2.3-rc.1"));
        assert!(is_an_immutable_version("1.2.3+build.5"));
        assert!(is_an_immutable_version(&"a".repeat(40)));
        assert!(is_an_immutable_version(&"0".repeat(64)));
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
