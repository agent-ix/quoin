// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The two orders this projection sorts by, and neither of them is new.
//!
//! `graph-portfolio.ts:840-846` declares `compare` (JavaScript `<` on strings)
//! and `compareInstants` (`Date.parse(a) - Date.parse(b) || compare(a, b)`).
//! Both already exist in this workspace and are re-used rather than rewritten:
//! the string order is [`quoin_store::json::order::cmp_utf16`], and the instant
//! is read by [`quoin_measurement::Rfc3339DateTime`], the measurement domain's
//! single grammar.

use std::cmp::Ordering;

use quoin_measurement::Rfc3339DateTime;
pub use quoin_store::json::order::cmp_utf16 as compare_text;

/// `compareInstants` (`graph-portfolio.ts:844-846`).
///
/// # The `NaN` branch, stated
///
/// The retained code subtracts two `Date.parse` results. A timestamp this
/// crate's grammar refuses yields `NaN`, `NaN - x` is `NaN`, and `NaN` is
/// falsy, so the comparison **falls through** to the text comparison rather
/// than ordering anything. That is reproduced here, exactly as
/// `quoin_measurement::store::read::collection_order` reproduces it for the
/// same expression in `store.ts`.
#[must_use]
pub fn compare_instants(a: &str, b: &str) -> Ordering {
    let instants = Rfc3339DateTime::parse(a)
        .ok()
        .zip(Rfc3339DateTime::parse(b).ok())
        .map(|(left, right)| left.epoch_millis().cmp(&right.epoch_millis()));
    match instants {
        Some(Ordering::Equal) | None => compare_text(a, b),
        Some(order) => order,
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{compare_instants, compare_text};

    #[test]
    fn an_earlier_instant_orders_first_however_it_is_spelled() {
        assert_eq!(
            compare_instants("2026-01-01T00:00:00.000Z", "2026-01-02T00:00:00.000Z"),
            Ordering::Less
        );
        // Same instant, two spellings: the text comparison breaks the tie, so
        // the order is total rather than arbitrary.
        assert_eq!(
            compare_instants("2026-01-01T00:00:00Z", "2026-01-01T00:00:00.000Z"),
            compare_text("2026-01-01T00:00:00Z", "2026-01-01T00:00:00.000Z")
        );
    }

    #[test]
    fn an_unreadable_instant_falls_through_to_the_text_order() {
        assert_eq!(
            compare_instants("the-first-of-never", "2026-01-01T00:00:00Z"),
            Ordering::Greater
        );
        assert_eq!(compare_instants("b", "a"), Ordering::Greater);
    }
}
