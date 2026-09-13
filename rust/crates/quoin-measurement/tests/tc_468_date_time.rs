// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The two retained RFC 3339 grammars disagree, and the port says which won.
//!
//! # The disagreement, stated
//!
//! `src/measurement/date-time.ts:1-2` accepts `[Tt]` and `[Zz]`:
//!
//! ```text
//! /^(\d{4})-(\d{2})-(\d{2})[Tt](\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:[Zz]|[+-](\d{2}):(\d{2}))$/
//! ```
//!
//! `src/measurement/intervention.ts:348-350` accepts only `T` and `Z`:
//!
//! ```text
//! /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:Z|([+-])(\d{2}):(\d{2}))$/
//! ```
//!
//! On `2026-01-01t00:00:00z` the first says yes and the second says no. Two
//! implementations of one grammar that disagree is a defect whichever way it is
//! resolved, so the Stage 6 plan §5 owner ruling picks the **permissive** one:
//! RFC 3339 §5.6 states that the `T` and `Z` characters "may alternatively be
//! lower case", so the strict regex is a narrowing of the standard, not an
//! enforcement of it.
//!
//! The narrowing is not lost. [`Rfc3339DateTime::narrowed_by_the_strict_grammar`]
//! reports it per value, so a caller that must keep the stricter contract can,
//! and `DIVERGENCE.md` records it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_measurement::{MeasurementErrorCode, Rfc3339DateTime};

/// The strict grammar of `intervention.ts:348-350`, as a predicate.
///
/// Stated here rather than shipped in `src/`, because the crate must contain
/// exactly one grammar. This is the *oracle* for the disagreement, and it lives
/// with the test that asserts it.
fn the_strict_grammar_would_accept(value: &str) -> bool {
    Rfc3339DateTime::parse(value).is_ok_and(|parsed| !parsed.narrowed_by_the_strict_grammar())
}

/// Trace: FR-100-AC-2, NFR-025-AC-3
/// Provenance: quoin#468
#[test]
fn tc_468_the_two_retained_grammars_disagree_on_a_lowercase_t_and_z() {
    // The one input the two retained implementations disagree about.
    let lowercase = "2026-01-01t00:00:00z";
    let parsed = Rfc3339DateTime::parse(lowercase).expect(
        "the permissive grammar of date-time.ts accepts a lowercase separator and zulu \
         designator, and it is the one this crate ports",
    );
    assert!(
        parsed.narrowed_by_the_strict_grammar(),
        "the value was accepted, and it must also report that intervention.ts's stricter regex \
         would have refused it"
    );
    assert!(!the_strict_grammar_would_accept(lowercase));

    // Each half of the disagreement on its own.
    assert!(Rfc3339DateTime::parse("2026-01-01t00:00:00Z").is_ok());
    assert!(Rfc3339DateTime::parse("2026-01-01T00:00:00z").is_ok());

    // The uppercase spelling both grammars accept is not reported as narrowed,
    // so the flag names the disagreement and nothing else.
    let upper = Rfc3339DateTime::parse("2026-01-01T00:00:00Z").expect("both grammars accept it");
    assert!(!upper.narrowed_by_the_strict_grammar());
    assert!(the_strict_grammar_would_accept("2026-01-01T00:00:00Z"));
    assert_eq!(upper.epoch_millis(), 1_767_225_600_000);
}

/// Trace: FR-100-AC-2
/// Provenance: quoin#468
#[test]
fn tc_468_both_grammars_still_refuse_what_date_parse_would_have_accepted() {
    // The reason `date-time.ts` exists at all: `Date.parse` accepts these and
    // the retained grammar does not. Unifying on the permissive separator did
    // not widen anything else.
    for refused in [
        "2026-01-01",
        "2026-01-01T00:00Z",
        "2026-01-01T00:00:00",
        "2026-02-30T00:00:00Z",
        "2026-13-01T00:00:00Z",
        "2026-01-01T24:00:00Z",
        "2026-01-01T00:00:00+24:00",
        "2026-01-01T00:00:00Z ",
        "January 1, 2026",
    ] {
        let error = Rfc3339DateTime::parse(refused).unwrap_err();
        assert_eq!(
            error.code(),
            MeasurementErrorCode::DateTimeInvalid,
            "{refused} was accepted"
        );
    }
}
