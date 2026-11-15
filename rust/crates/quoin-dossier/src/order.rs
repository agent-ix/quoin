// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Dossier order over ids (FR-191, "Ordering").
//!
//! An id is split at `-` into segments. Two digit segments compare by numeric
//! value, any other two by byte value, a digit segment sorts before a
//! non-digit one, and an id that is a prefix of another sorts first. So
//! `FR-99` precedes `FR-100`, and `FR-100-AC-2` precedes `FR-100-AC-10`.

use std::cmp::Ordering;

/// Compare two ids in dossier order.
#[must_use]
pub(crate) fn dossier_cmp(left: &str, right: &str) -> Ordering {
    let mut left_segments = left.split('-');
    let mut right_segments = right.split('-');
    loop {
        match (left_segments.next(), right_segments.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) => match segment_cmp(a, b) {
                Ordering::Equal => {}
                other => return other,
            },
        }
    }
}

/// Compare two optional ids; an absent id sorts first.
#[must_use]
pub(crate) fn optional_cmp(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => dossier_cmp(a, b),
    }
}

fn is_digits(segment: &str) -> bool {
    !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit())
}

fn segment_cmp(left: &str, right: &str) -> Ordering {
    match (is_digits(left), is_digits(right)) {
        (true, true) => {
            // Compared as digit strings so an id of any length is ordered
            // without overflow. `007` and `7` are numerically equal; the byte
            // tie-break keeps the order total.
            let a = left.trim_start_matches('0');
            let b = right.trim_start_matches('0');
            a.len()
                .cmp(&b.len())
                .then_with(|| a.cmp(b))
                .then_with(|| left.cmp(right))
        }
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => left.as_bytes().cmp(right.as_bytes()),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-191-AC-15")]
    #[test]
    fn numeric_segments_compare_by_value() {
        assert_eq!(dossier_cmp("FR-99", "FR-100"), Ordering::Less);
        assert_eq!(dossier_cmp("FR-100-AC-2", "FR-100-AC-10"), Ordering::Less);
        assert_eq!(
            dossier_cmp("FR-99999999999999999999999", "FR-100000000000000000000000"),
            Ordering::Less
        );
    }

    #[trace("FR-191-AC-15")]
    #[test]
    fn digit_segments_precede_other_segments_and_prefixes_precede_longer_ids() {
        assert_eq!(dossier_cmp("FR-1", "FR-A"), Ordering::Less);
        assert_eq!(dossier_cmp("FR-100", "FR-100-AC-1"), Ordering::Less);
        assert_eq!(dossier_cmp("NFR-1", "FR-1"), Ordering::Greater);
        assert_eq!(dossier_cmp("FR-7", "FR-7"), Ordering::Equal);
    }

    #[trace("FR-191-AC-15")]
    #[test]
    fn an_absent_id_sorts_first() {
        assert_eq!(optional_cmp(None, Some("FR-1")), Ordering::Less);
        assert_eq!(optional_cmp(Some("FR-1"), None), Ordering::Greater);
        assert_eq!(optional_cmp(None, None), Ordering::Equal);
    }

    #[trace("FR-191-AC-15")]
    #[test]
    fn leading_zeros_keep_the_order_total() {
        assert_eq!(dossier_cmp("FR-7", "FR-007"), Ordering::Greater);
        assert_eq!(dossier_cmp("FR-007", "FR-8"), Ordering::Less);
    }
}
