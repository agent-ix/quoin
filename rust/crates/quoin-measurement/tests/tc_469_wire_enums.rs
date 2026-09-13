// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every closed string union round-trips through its wire spelling.
//!
//! The spellings are the API. `InterventionIntakeError`'s code set is the one
//! quoin#469 names explicitly — a caller that learned `raw_evidence_mismatch`
//! keeps it forever — but the same contract binds every union in these four
//! files, so every union is checked here rather than one.
//!
//! Four things are asserted per union: `from_code(as_str(v)) == Some(v)`, no
//! two variants share a spelling, an unknown spelling is `None` rather than a
//! default, and serde agrees with `as_str` in both directions. The last matters
//! because the spelling is written once in the `wire_enum!` table and used by
//! both the accessors and the derive; if the macro ever let those diverge, a
//! record would serialize under a name `from_code` could not read back.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;
use std::fmt::Debug;

use quoin_measurement::common::wire_enum::WireEnum;
use quoin_measurement::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};
use quoin_measurement::intervention::record::{
    AttributionConfidence, InterventionAssignmentMethod, InterventionConclusionKind,
    InterventionDesignKind, InterventionDisposition, InterventionRecordType, InterventionStatus,
};
use quoin_measurement::intervention::report::{AdverseDisposition, CounterevidenceKind};
use quoin_measurement::operational::record::{
    CapabilityStatus, ClockStatus, ExerciseMode, ExerciseOutcome, NotApplicableClockStatus,
    OperationalControlKind, OperationalRecordShape, OperationalRecordType, VersionPinKind,
    WithClockApplicability,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The number of unions these four files declare. A union added without being
/// registered below leaves this count behind and the census fails.
const UNION_FLOOR: usize = 20;
/// The number of variants across them, so a union that lost its table is seen.
const VARIANT_FLOOR: usize = 71;

/// Asserts the four properties for one union, and returns its variant count.
fn round_trips<T>() -> usize
where
    T: WireEnum + Debug + Serialize + DeserializeOwned,
{
    let name = T::union_name();
    let all = T::all();
    assert!(!all.is_empty(), "{name} declares no variants");

    let mut spellings: BTreeSet<&'static str> = BTreeSet::new();
    for variant in all {
        let spelling = variant.as_str();
        assert!(
            spellings.insert(spelling),
            "{name}: two variants share the spelling {spelling:?}; a code is never reused"
        );
        assert_eq!(
            T::from_code(spelling),
            Some(*variant),
            "{name}: {spelling:?} does not round-trip"
        );
        assert_eq!(
            serde_json::to_string(variant).unwrap(),
            format!("\"{spelling}\""),
            "{name}: serde spells {variant:?} differently from as_str"
        );
        let parsed: T = serde_json::from_str(&format!("\"{spelling}\"")).unwrap();
        assert_eq!(
            parsed, *variant,
            "{name}: serde does not read {spelling:?} back"
        );
    }
    assert!(
        T::from_code("__no_such_code__").is_none(),
        "{name}: an unknown spelling must be None, never a default"
    );
    all.len()
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#469
#[test]
fn tc_469_every_wire_union_round_trips_through_its_spelling() {
    let counts = [
        round_trips::<InterventionDisposition>(),
        round_trips::<InterventionDesignKind>(),
        round_trips::<InterventionAssignmentMethod>(),
        round_trips::<InterventionStatus>(),
        round_trips::<InterventionConclusionKind>(),
        round_trips::<AttributionConfidence>(),
        round_trips::<InterventionRecordType>(),
        round_trips::<InterventionRefusalCode>(),
        round_trips::<CounterevidenceKind>(),
        round_trips::<AdverseDisposition>(),
        round_trips::<OperationalControlKind>(),
        round_trips::<VersionPinKind>(),
        round_trips::<OperationalRecordType>(),
        round_trips::<CapabilityStatus>(),
        round_trips::<ExerciseMode>(),
        round_trips::<ExerciseOutcome>(),
        round_trips::<ClockStatus>(),
        round_trips::<NotApplicableClockStatus>(),
        round_trips::<OperationalRecordShape>(),
        round_trips::<WithClockApplicability>(),
    ];
    assert_eq!(
        counts.len(),
        UNION_FLOOR,
        "anti-vacuity floor: {UNION_FLOOR} unions expected in this census"
    );
    assert_eq!(
        counts.iter().sum::<usize>(),
        VARIANT_FLOOR,
        "the variant total changed; say which union gained or lost one"
    );
}

/// The intake refusal set, written out rather than re-derived from the enum.
///
/// `intervention-types.ts:104-110`. This is the set quoin#469 names as the
/// gate, so it is asserted against the literal spellings rather than against
/// `all()`, which would agree with whatever the code says.
///
/// Trace: FR-100-AC-4
/// Provenance: quoin#469
#[test]
fn tc_469_the_intake_refusal_codes_are_the_typescript_codes() {
    let spellings: Vec<&str> = InterventionRefusalCode::all()
        .iter()
        .map(|code| code.as_str())
        .collect();
    assert_eq!(
        spellings,
        [
            "invalid_record",
            "raw_evidence_mismatch",
            "governing_plan_absent",
            "definition_mismatch",
            "intake_busy",
            "record_id_collision",
        ]
    );
}

/// A refusal carries its code, its findings, and the message they compose to.
///
/// Trace: FR-100-AC-4
/// Provenance: quoin#469
#[test]
fn tc_469_an_intake_refusal_carries_its_code_and_findings() {
    let error = InterventionIntakeError::new(
        InterventionRefusalCode::GoverningPlanAbsent,
        vec!["no plan admits p-one".to_owned()],
    );
    assert_eq!(error.code(), InterventionRefusalCode::GoverningPlanAbsent);
    assert_eq!(error.findings(), ["no plan admits p-one"]);
    assert_eq!(
        error.to_string(),
        "governing_plan_absent: no plan admits p-one"
    );
    assert_eq!(
        InterventionRefusalCode::from_code("governing_plan_absent"),
        Some(InterventionRefusalCode::GoverningPlanAbsent)
    );
}
