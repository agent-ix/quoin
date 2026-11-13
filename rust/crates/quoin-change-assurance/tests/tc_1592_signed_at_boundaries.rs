// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The edges of the instant a claimed approval may report (FR-063, FR-065).
//!
//! The reader checks the shape `YYYY-MM-DDTHH:MM:SSZ` and that the instant
//! names a real second; it does not judge the instant plausible.

#![allow(
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;

use quoin_change_assurance::error::Subject;
use quoin_change_assurance::model::approval_time::SignedAt;

fn reads(text: &str) -> bool {
    SignedAt::parse(text, Subject::Receipt, "signed_at").is_ok()
}

/// Every field's last valid value and the first value past it.
#[trace("FR-063-AC-17", "FR-065-AC-21")]
#[test]
fn tc_1592_the_fields_are_accepted_to_their_edge_and_refused_past_it() {
    for accepted in [
        "0000-01-01T00:00:00Z",
        "2026-12-31T23:59:59Z",
        "2026-01-31T23:59:59Z",
        "2026-04-30T12:00:00Z",
        "2026-08-31T10:20:30Z",
    ] {
        assert!(reads(accepted), "{accepted} must be accepted");
    }
    for refused in [
        "2026-08-31T10:20:60Z",
        "2026-08-31T10:60:30Z",
        "2026-08-31T24:20:30Z",
        "2026-08-00T10:20:30Z",
        "2026-08-32T10:20:30Z",
        "2026-00-31T10:20:30Z",
        "2026-13-31T10:20:30Z",
        "2026-08-31T23:59:60Z",
        "2026-08-31T10:20:30",
        "2026-08-31T10:20:30z",
        "2026-08-31 10:20:30Z",
        "2026-08-31T10:20:30+00:00",
        "+026-08-31T10:20:30Z",
        "",
    ] {
        assert!(!reads(refused), "{refused:?} must be refused");
    }
}

/// Days per month, with leap years: the 31st of a 30-day month and the 29th of
/// February in a common year are refused; the 29th in a leap year (including
/// a year divisible by 400) is accepted and in a century year is refused.
#[trace("FR-063-AC-17", "FR-065-AC-21")]
#[test]
fn tc_1592_the_day_must_exist_in_its_month() {
    for refused in [
        "2026-02-31T00:00:00Z",
        "2026-02-29T00:00:00Z",
        "2025-02-29T00:00:00Z",
        "2026-04-31T00:00:00Z",
        "2026-06-31T00:00:00Z",
        "2026-09-31T00:00:00Z",
        "2026-11-31T00:00:00Z",
        "1900-02-29T00:00:00Z",
        "2100-02-29T00:00:00Z",
        "2024-02-30T00:00:00Z",
    ] {
        assert!(!reads(refused), "{refused} must be refused");
    }
    for accepted in [
        "2024-02-29T00:00:00Z",
        "2000-02-29T00:00:00Z",
        "2026-02-28T00:00:00Z",
        "2026-04-30T00:00:00Z",
        "2026-12-31T00:00:00Z",
    ] {
        assert!(reads(accepted), "{accepted} must be accepted");
    }
}
