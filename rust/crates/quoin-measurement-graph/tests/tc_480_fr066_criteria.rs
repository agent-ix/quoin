// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-066 acceptance criterion the deleted `tests/graph-adapters.test.ts`
//! carried, restated against this crate.
//!
//! Trace: FR-066-AC-2, FR-066-AC-3, FR-066-AC-4, FR-066-AC-5, FR-066-AC-6
//! Trace: FR-066-AC-7, FR-066-AC-8, FR-066-AC-9, FR-066-AC-10, FR-066-AC-11
//! Trace: FR-066-AC-12
//! Provenance: quoin#480
//!
//! # Why this file exists
//!
//! `tests/graph-adapters.test.ts` was deleted in the same commit as
//! `src/measurement/graph-adapters.ts` (FR-101). It carried twelve
//! criterion-tagged assertions. `tc_475_parity.rs` proves this crate answers
//! what the retained module answered on 89 captured cases, but it is tagged
//! `FR-066-AC-1` only — a corpus-wide agreement gate is not a per-criterion
//! one. This file is: one test per criterion, named by the criterion, so a
//! criterion cannot be lost without a named test disappearing with it.
//!
//! # Where the inputs come from
//!
//! `tests/goldens/graph-adapter-verdicts.json` — the committed capture, not a
//! shape this file invented. Where a criterion needs an input the capture does
//! not hold (a permuted census, a deleted attestation member), the base case is
//! taken from the capture and mutated **here**, in Rust. No node process is
//! spawned; `tc_476_boundary.rs` asserts none can be.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
use std::path::Path;

use quoin_measurement::MeasurementPlan;
use quoin_measurement::types::plan::{LifecycleStatus, MeasurementStage};
use quoin_measurement_graph::assurance::premise::{
    AcceptedQuirePremises, ModulePremise, SourcePremise,
};
use quoin_measurement_graph::quality::adapt::{AdaptGraphQualityInput, TranscribedCollection};
use quoin_measurement_graph::{
    GraphAdapterError, GraphAdapterErrorCode, adapt_graph_quality_observation,
    adapt_quire_assurance, graph_quality_observation_id,
};
use serde_json::Value;

// ------------------------------------------------------------ the capture

/// Every captured case, by name.
fn cases() -> BTreeMap<String, Value> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/graph-adapter-verdicts.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
    let golden: Value = serde_json::from_str(&text).expect("the capture is JSON");
    golden["cases"]
        .as_array()
        .expect("the capture carries cases")
        .iter()
        .map(|case| {
            (
                case["name"].as_str().expect("a name").to_owned(),
                case["input"].clone(),
            )
        })
        .collect()
}

/// One captured case's input, by name.
fn case(name: &str) -> Value {
    cases()
        .remove(name)
        .unwrap_or_else(|| panic!("the capture carries no case `{name}`"))
}

/// Rebuild a measurement plan from the capture's JSON.
fn plan(value: &Value) -> MeasurementPlan {
    let text = |name: &str| -> quoin_measurement::types::ids::NonEmptyText {
        quoin_measurement::types::ids::NonEmptyText::parse(
            value[name].as_str().expect("a plan member"),
            quoin_measurement::MeasurementErrorCode::PlanInvalid,
            name,
        )
        .expect("a non-empty plan member")
    };
    MeasurementPlan {
        id: text("id"),
        title: text("title"),
        status: LifecycleStatus::from_wire(value["status"].as_str().expect("a status"))
            .expect("a known status"),
        stage: MeasurementStage::from_wire(value["stage"].as_str().expect("a stage"))
            .expect("a known stage"),
        metric: text("metric"),
        definition_version: text("definitionVersion"),
        path: value["path"].as_str().expect("a path").to_owned(),
        owner: None,
        action: None,
        preregistration: None,
        ground_truth_kind: None,
        statistical_design: None,
        objective: None,
    }
}

/// The bytes a capture recorded as an array of numbers.
fn scorer_bytes(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .expect("an array of byte values")
        .iter()
        .map(|byte| u8::try_from(byte.as_u64().expect("a byte")).expect("a byte"))
        .collect()
}

/// Run one `adaptGraphQualityObservation` case input through this crate.
fn adapt_quality(input: &Value) -> Result<TranscribedCollection, GraphAdapterError> {
    let plans: Vec<MeasurementPlan> = input["plans"]
        .as_array()
        .expect("a plan list")
        .iter()
        .map(plan)
        .collect();
    let bytes = scorer_bytes(&input["scorerBytes"]);
    adapt_graph_quality_observation(&AdaptGraphQualityInput {
        record: &input["record"],
        scorer_bytes: &bytes,
        scorer_media_type: input["scorerMediaType"].as_str().expect("a media type"),
        attestation: &input["attestation"],
        plans: &plans,
    })
}

/// The code a refusal carried, or a panic naming what was accepted instead.
fn refusal(input: &Value, what: &str) -> GraphAdapterErrorCode {
    match adapt_quality(input) {
        Ok(_) => panic!("{what}: accepted, and the criterion says it is refused"),
        Err(error) => error.code(),
    }
}

/// Delete `value` at a path of member names, panicking if it is not there.
fn delete_at(value: &mut Value, path: &[&str]) {
    let Some((last, parents)) = path.split_last() else {
        panic!("an empty path deletes nothing");
    };
    let mut cursor = value;
    for step in parents {
        cursor = cursor
            .get_mut(*step)
            .unwrap_or_else(|| panic!("the base case has no `{step}` on the way to `{last}`"));
    }
    let object = cursor
        .as_object_mut()
        .unwrap_or_else(|| panic!("`{last}`'s parent is not an object"));
    assert!(
        object.remove(*last).is_some(),
        "the base case carries no `{last}` to delete"
    );
}

/// Re-mint the observation identity a mutated record must now declare, so that
/// a mutation is measured for what it changed rather than for the identity it
/// invalidated.
fn remint(record: &mut Value) {
    record["observation_id"] = Value::Null;
    let object = record.as_object_mut().expect("a record object");
    object.remove("observation_id");
    let id = graph_quality_observation_id(record).expect("the identity is computable");
    record["observation_id"] = Value::String(id.to_stored());
}

/// Every observation the transcription produced, keyed
/// `measure/dimension/key`.
fn facts(collection: &TranscribedCollection) -> BTreeMap<String, Option<f64>> {
    collection
        .collection()
        .observations
        .iter()
        .map(|row| {
            let identity = row.dimensions.entries();
            let member = |name: &str| -> String {
                match identity.get(name) {
                    Some(quoin_store::JsonValue::String(text)) => text.clone(),
                    _ => String::new(),
                }
            };
            (
                format!(
                    "{}/{}/{}",
                    member("measure"),
                    member("dimension"),
                    member("key")
                ),
                row.value,
            )
        })
        .collect()
}

// ------------------------------------------------------------- the gates

/// A valid Quire export is preserved and every premise drift is refused.
///
/// Trace: FR-066-AC-2
/// Provenance: quoin#480
#[test]
fn tc_480_020_a_valid_export_is_preserved_and_every_premise_drift_refused() {
    let base = case("assurance/base");
    let accepted = AcceptedQuirePremises {
        source: serde_json::from_value::<SourcePremise>(base["accepted"]["source"].clone())
            .expect("the capture's accepted source"),
        modules: serde_json::from_value::<Vec<ModulePremise>>(base["accepted"]["modules"].clone())
            .expect("the capture's accepted modules"),
    };
    let parsed = adapt_quire_assurance(&base["document"], &accepted)
        .expect("the captured base export is admissible");
    assert_eq!(parsed.source, accepted.source, "the source is carried");
    assert_eq!(parsed.modules, accepted.modules, "the modules are carried");

    // Each drift the criterion names, taken from the capture where it holds
    // one and minted here where it does not.
    for name in [
        "assurance/missing/format",
        "assurance/missing/format_version",
        "assurance/source-revision-disagrees",
        "assurance/modules-disagree",
        "assurance/unknown-key",
    ] {
        let drifted = case(name);
        let premises = AcceptedQuirePremises {
            source: serde_json::from_value::<SourcePremise>(drifted["accepted"]["source"].clone())
                .expect("an accepted source"),
            modules: serde_json::from_value::<Vec<ModulePremise>>(
                drifted["accepted"]["modules"].clone(),
            )
            .expect("accepted modules"),
        };
        let error = adapt_quire_assurance(&drifted["document"], &premises)
            .err()
            .unwrap_or_else(|| panic!("{name}: admitted, and the criterion says it is refused"));
        assert_eq!(
            error.code(),
            GraphAdapterErrorCode::InvalidPremise,
            "{name}: refused for the premise it drifted on"
        );
    }

    // A module schema digest the caller did not agree to. The capture holds no
    // such case; the criterion names it, so it is minted here.
    let mut drifted = base.clone();
    drifted["document"]["modules"][0]["schemas"][0]["schema_digest"] =
        Value::String("d".repeat(64));
    assert_eq!(
        adapt_quire_assurance(&drifted["document"], &accepted)
            .expect_err("a disagreeing schema digest is refused")
            .code(),
        GraphAdapterErrorCode::InvalidPremise,
        "a module schema digest is a premise"
    );
}

/// Every Quire graph collection is handed through without translation.
///
/// Trace: FR-066-AC-3
/// Provenance: quoin#480
#[test]
fn tc_480_021_every_quire_collection_passes_through_untranslated() {
    let base = case("assurance/base");
    let accepted = AcceptedQuirePremises {
        source: serde_json::from_value::<SourcePremise>(base["accepted"]["source"].clone())
            .expect("the capture's accepted source"),
        modules: serde_json::from_value::<Vec<ModulePremise>>(base["accepted"]["modules"].clone())
            .expect("the capture's accepted modules"),
    };
    let parsed = adapt_quire_assurance(&base["document"], &accepted).expect("admissible");

    // The six collections the export carries, each the length the document
    // stated: an adapter that summarised, filtered or re-ordered one would
    // fail here.
    let stated = |member: &str| -> usize {
        base["document"][member]
            .as_array()
            .unwrap_or_else(|| panic!("the export carries `{member}` as an array"))
            .len()
    };
    assert_eq!(parsed.artifacts.len(), stated("artifacts"));
    assert_eq!(parsed.obligations.len(), stated("obligations"));
    assert_eq!(parsed.symbols.len(), stated("symbols"));
    assert_eq!(parsed.relation_kinds.len(), stated("relation_kinds"));
    assert_eq!(parsed.relations.len(), stated("relations"));
    assert_eq!(
        parsed.relation_observations.len(),
        stated("relation_observations")
    );
    assert!(
        !parsed.artifacts.is_empty() && !parsed.relation_kinds.is_empty(),
        "a pass-through measured over empty collections measures nothing"
    );

    // An artifact the adapter has never seen travels through unchanged.
    let mut grown = base.clone();
    let extra = serde_json::json!({
        "id": "FR-001",
        "artifact_type": "FR",
        "locator": { "path": "spec/FR-001.md", "line": 1, "digest": "d".repeat(64) },
    });
    grown["document"]["artifacts"]
        .as_array_mut()
        .expect("artifacts is an array")
        .push(extra);
    let grown_parsed = adapt_quire_assurance(&grown["document"], &accepted)
        .expect("an extra artifact is still admissible");
    assert_eq!(
        grown_parsed.artifacts.len(),
        parsed.artifacts.len() + 1,
        "the added artifact reaches the other side"
    );
}

/// The graph-quality schema is closed and the observation identity is the
/// canonical digest of the record without it.
///
/// Trace: FR-066-AC-4
/// Provenance: quoin#480
#[test]
fn tc_480_022_the_schema_is_closed_and_the_identity_is_canonical() {
    // The identity is computed over canonical bytes: member order in does not
    // change the digest out.
    let sorted = graph_quality_observation_id(&serde_json::json!({
        "a": [{ "z": 2, "a": 1 }],
        "": "bmp",
        "𐀀": "astral",
        "observation_id": format!("sha256:{}", "a".repeat(64)),
    }))
    .expect("an identity");
    let permuted = graph_quality_observation_id(&serde_json::json!({
        "observation_id": format!("sha256:{}", "b".repeat(64)),
        "𐀀": "astral",
        "a": [{ "a": 1, "z": 2 }],
        "": "bmp",
    }))
    .expect("an identity");
    assert_eq!(
        sorted.to_stored(),
        permuted.to_stored(),
        "the identity ignores member order and the declared identity itself"
    );

    // A record that declares an identity it does not have is refused.
    assert_eq!(
        refusal(
            &case("quality/observation-id-disagrees"),
            "a disagreeing observation_id"
        ),
        GraphAdapterErrorCode::InvalidObservation,
    );
    // A record carrying a member the schema does not declare is refused.
    assert_eq!(
        refusal(&case("quality/unknown-member"), "an undeclared member"),
        GraphAdapterErrorCode::InvalidObservation,
    );
}

/// The scorer bytes are retained exactly and a digest mismatch is refused.
///
/// Trace: FR-066-AC-5
/// Provenance: quoin#480
#[test]
fn tc_480_023_scorer_bytes_are_retained_and_a_mismatch_refused() {
    let base = case("quality/measured");
    let bytes = scorer_bytes(&base["scorerBytes"]);
    let transcribed = adapt_quality(&base).expect("the captured base record is admissible");
    let scorer = transcribed
        .collection()
        .raw_evidence
        .as_object()
        .expect("rawEvidence is an object")
        .get("scorer")
        .and_then(|value| value.as_object().ok())
        .expect("the transcription retains the scorer attachment");
    let member = |name: &str| -> String {
        scorer
            .get(name)
            .and_then(quoin_store::JsonValue::as_str)
            .unwrap_or_else(|| panic!("the scorer attachment carries `{name}` as a string"))
            .to_owned()
    };
    assert_eq!(
        member("bytesBase64"),
        quoin_measurement_graph::base64::encode(&bytes),
        "the retained bytes are the caller's, unaltered"
    );
    assert_eq!(
        member("mediaType"),
        base["scorerMediaType"].as_str().unwrap()
    );

    assert_eq!(
        refusal(&case("quality/digest-mismatch"), "a digest mismatch"),
        GraphAdapterErrorCode::AttachmentDigestMismatch,
    );
    assert_eq!(
        refusal(&case("quality/no-media-type"), "an attachment with no type"),
        GraphAdapterErrorCode::AttachmentMissing,
    );
}

/// Every attestation and producer field the criterion names is required, one
/// at a time.
///
/// Trace: FR-066-AC-6
/// Provenance: quoin#480
#[test]
fn tc_480_024_every_attestation_and_producer_field_is_required() {
    /// Sixteen of the nineteen attestation paths `graph-adapters.test.ts`
    /// deleted — `verificationStack.toolchains.{node,rust,python}` are no
    /// longer independently required (PLAT-930, `tc_480_031` below).
    const ATTESTATION: [&[&str]; 16] = [
        &["subject"],
        &["scope"],
        &["timestamp"],
        &["environment"],
        &["verificationStack"],
        &["verificationStack", "schemaVersion"],
        &["verificationStack", "lockDigest"],
        &["verificationStack", "executableDigest"],
        &["verificationStack", "buildProfile"],
        &["verificationStack", "toolchains"],
        // `toolchains.{node,rust,python}` are deliberately absent from this
        // list: PLAT-930 made each optional (a measurement rarely touches
        // every language), so omitting one is admitted, not refused. See
        // `tc_480_031` below for that acceptance, and for the still-refused
        // case of omitting all three at once.
        &["verificationStack", "sources"],
        &[
            "verificationStack",
            "sources",
            "agent-ix/quire-code-rs",
            "revision",
        ],
        &[
            "verificationStack",
            "sources",
            "agent-ix/quire-code-rs",
            "sourceState",
        ],
        &[
            "verificationStack",
            "sources",
            "agent-ix/quire-code-rs",
            "remote",
        ],
        &["verificationStack", "capabilities"],
        &["verificationStack", "artifacts"],
    ];

    /// The ten producer paths it deleted.
    const PRODUCER: [&[&str]; 10] = [
        &["producer", "extractor_revision"],
        &["producer", "producer_contract_version"],
        &["producer", "parser_grammars"],
        &["producer", "parser_grammars", "0", "language"],
        &["producer", "parser_grammars", "0", "grammar"],
        &["producer", "parser_grammars", "0", "revision"],
        &["producer", "configuration_digest"],
        &["producer", "source_revision"],
        &["producer", "corpus_revision"],
        &["producer", "scorer_version"],
    ];

    let base = case("quality/measured");
    adapt_quality(&base).expect("the base case is admissible before anything is deleted");

    for path in ATTESTATION {
        let mut mutated = base.clone();
        delete_at(&mut mutated["attestation"], path);
        assert_eq!(
            refusal(&mutated, &format!("attestation without {}", path.join("."))),
            GraphAdapterErrorCode::InvalidAttestation,
            "attestation.{} is required",
            path.join(".")
        );
    }

    for path in PRODUCER {
        let mut mutated = base.clone();
        // A path step that is an array index rather than a member name.
        if let Some(position) = path.iter().position(|step| *step == "0") {
            let (head, tail) = path.split_at(position);
            let mut cursor = &mut mutated["record"];
            for step in head {
                cursor = &mut cursor[*step];
            }
            let element = &mut cursor[0];
            delete_at(element, &tail[1..]);
        } else {
            delete_at(&mut mutated["record"], path);
        }
        remint(&mut mutated["record"]);
        assert_eq!(
            refusal(&mutated, &format!("record without {}", path.join("."))),
            GraphAdapterErrorCode::InvalidObservation,
            "record.{} is required",
            path.join(".")
        );
    }
}

/// Absent, inactive, duplicated or mismatched plans are refused, and a retired
/// plan alongside the active one is not.
///
/// Trace: FR-066-AC-7
/// Provenance: quoin#480
#[test]
fn tc_480_025_the_plan_must_be_exactly_one_active_matching_plan() {
    for name in [
        "quality/plans/none",
        "quality/plans/only-other-metric",
        "quality/plans/proposed",
        "quality/plans/two-active",
        "quality/plans/wrong-definition",
    ] {
        assert_eq!(
            refusal(&case(name), name),
            GraphAdapterErrorCode::InactivePlan,
            "{name}: refused for the plan"
        );
    }

    // A historical, retired plan for the same metric does not shadow the
    // active one, in either order.
    let base = case("quality/measured");
    let active = base["plans"][0].clone();
    let historical = serde_json::json!({
        "id": "MP-RETIRED",
        "title": "Graph quality",
        "status": "retired",
        "stage": "observe",
        "metric": "graph_quality",
        "definitionVersion": "quire-code.graph-quality-v0",
        "path": "spec/assurance/MP-RETIRED.md",
    });
    for order in [
        vec![historical.clone(), active.clone()],
        vec![active.clone(), historical.clone()],
    ] {
        let mut mutated = base.clone();
        mutated["plans"] = Value::Array(order);
        let transcribed = adapt_quality(&mutated)
            .expect("a retired plan beside the active one is not an inactive plan");
        assert!(
            !transcribed.collection().observations.is_empty(),
            "the transcription is not empty"
        );
    }
}

/// The census maps bijectively and the transcription does not depend on the
/// order the producer wrote its census members in.
///
/// Trace: FR-066-AC-8
/// Provenance: quoin#480
#[test]
fn tc_480_026_the_census_maps_bijectively_under_every_permutation() {
    /// The four census members, whose 24 orderings must all transcribe alike.
    const MEMBERS: [&str; 4] = [
        "languages",
        "node_kinds",
        "relation_kinds",
        "resolver_tiers",
    ];

    let base = case("quality/measured");
    let expected = quoin_store::canonical_json(
        adapt_quality(&base)
            .expect("the base case is admissible")
            .document(),
    )
    .expect("canonical text");

    let mut permutations = 0_usize;
    for order in permutations_of(&MEMBERS) {
        let mut mutated = base.clone();
        let census = mutated["record"]["population"]["census"].clone();
        let mut rewritten = serde_json::Map::new();
        for member in &order {
            rewritten.insert((*member).to_owned(), census[*member].clone());
        }
        mutated["record"]["population"]["census"] = Value::Object(rewritten);
        remint(&mut mutated["record"]);
        let transcribed = adapt_quality(&mutated).expect("a permuted census is still admissible");
        assert_eq!(
            quoin_store::canonical_json(transcribed.document()).expect("canonical text"),
            expected,
            "census order {order:?} changed the transcription"
        );
        permutations += 1;
    }
    assert_eq!(permutations, 24, "every ordering of four members is tried");

    // Bijection: each census entry the record declares has exactly one
    // observation, and no observation was invented.
    let transcribed = adapt_quality(&base).expect("admissible");
    let rows = facts(&transcribed);
    for member in MEMBERS {
        for entry in base["record"]["population"]["census"][member]
            .as_array()
            .expect("a census list")
        {
            let key = entry["key"].as_str().expect("a census key");
            let count = entry["count"].as_f64().expect("a census count");
            assert_eq!(
                rows.get(&format!("census/{member}/{key}")).copied(),
                Some(Some(count)),
                "census/{member}/{key} is carried with its count"
            );
        }
    }
    let declared: usize = MEMBERS
        .iter()
        .map(|member| {
            base["record"]["population"]["census"][*member]
                .as_array()
                .map_or(0, Vec::len)
        })
        .sum();
    assert_eq!(
        rows.keys().filter(|key| key.starts_with("census/")).count(),
        declared,
        "no census observation was invented"
    );
}

/// Every ordering of a four-member list.
fn permutations_of(members: &[&'static str; 4]) -> Vec<Vec<&'static str>> {
    let mut out = Vec::new();
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let order = [a, b, c, d];
                    if order
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        == 4
                    {
                        out.push(order.iter().map(|index| members[*index]).collect());
                    }
                }
            }
        }
    }
    out
}

/// Result facts map bijectively and a duplicate partition is refused.
///
/// Trace: FR-066-AC-9
/// Provenance: quoin#480
#[test]
fn tc_480_027_result_facts_map_bijectively_and_duplicates_are_refused() {
    let base = case("quality/measured");
    let transcribed = adapt_quality(&base).expect("the base case is admissible");
    let rows = facts(&transcribed);

    // Every confusion-matrix component the record declares reaches exactly one
    // observation, under its own key.
    let mut expected = 0_usize;
    for matrix in base["record"]["results"]["confusion_matrices"]
        .as_array()
        .expect("the matrices")
    {
        let dimension = matrix["dimension"].as_str().expect("a dimension");
        let key = matrix["key"].as_str().expect("a key");
        for component in [
            "false_negative",
            "false_positive",
            "true_negative",
            "true_positive",
        ] {
            let value = matrix[component].as_f64().expect("a count");
            assert_eq!(
                rows.get(&format!("confusion_matrix/{dimension}/{key}.{component}"))
                    .copied(),
                Some(Some(value)),
                "confusion_matrix/{dimension}/{key}.{component} is carried"
            );
            expected += 1;
        }
    }
    for (measure, member) in [
        ("unresolved", "unresolved"),
        ("ambiguous", "ambiguous"),
        ("recall", "recall"),
    ] {
        for row in base["record"]["results"][member]
            .as_array()
            .expect("a result list")
        {
            let dimension = row["dimension"].as_str().expect("a dimension");
            let key = row["key"].as_str().expect("a key");
            assert!(
                rows.contains_key(&format!("{measure}/{dimension}/{key}")),
                "{measure}/{dimension}/{key} is carried"
            );
            expected += 1;
        }
    }
    let produced = rows
        .keys()
        .filter(|key| {
            ["ambiguous/", "confusion_matrix/", "recall/", "unresolved/"]
                .iter()
                .any(|prefix| key.starts_with(prefix))
        })
        .count();
    assert_eq!(produced, expected, "no result fact was invented or lost");
    assert!(expected > 0, "a bijection over nothing measures nothing");

    assert_eq!(
        refusal(
            &case("quality/duplicate-partition"),
            "a duplicate partition"
        ),
        GraphAdapterErrorCode::DuplicatePartition,
    );
}

/// Each not-computed population state is retained distinctly, and the census
/// survives it.
///
/// Trace: FR-066-AC-10
/// Provenance: quoin#480
#[test]
fn tc_480_028_each_not_computed_state_is_retained_distinctly() {
    // The captured not-computed populations, plus the third state the criterion
    // names, minted here from the second because the capture holds no case for
    // it.
    let mut unreadable = case("quality/unsupported-population");
    {
        let population = &mut unreadable["record"]["population"];
        population["state"] = Value::String("unreadable".to_owned());
        population["unsupported_files"] = Value::from(0);
        population["unreadable_files"] = Value::from(4);
    }
    remint(&mut unreadable["record"]);

    let inputs = [
        ("quality/empty-population", case("quality/empty-population")),
        (
            "quality/unsupported-population",
            case("quality/unsupported-population"),
        ),
        ("quality/unreadable-population", unreadable),
    ];

    let mut seen = std::collections::BTreeSet::new();
    let mut with_a_census = 0_usize;
    for (name, input) in inputs {
        let state = input["record"]["population"]["state"]
            .as_str()
            .expect("a population state")
            .to_owned();
        let transcribed = adapt_quality(&input).unwrap_or_else(|error| {
            panic!("{name}: a not-computed population is admissible: {error}")
        });
        let rows = facts(&transcribed);
        let quality_state: Vec<&String> = rows
            .keys()
            .filter(|key| key.starts_with("quality_state/"))
            .collect();
        assert_eq!(
            quality_state.len(),
            1,
            "{name}: exactly one quality_state observation"
        );
        assert!(
            quality_state[0].ends_with(&format!("/{state}")),
            "{name}: the retained state is `{state}`, not `{}`",
            quality_state[0]
        );

        // Whatever census the producer declared is carried, entry for entry —
        // a not-computed population does not discard it.
        let declared: usize = [
            "languages",
            "node_kinds",
            "relation_kinds",
            "resolver_tiers",
        ]
        .iter()
        .map(|member| {
            input["record"]["population"]["census"][*member]
                .as_array()
                .map_or(0, Vec::len)
        })
        .sum();
        assert_eq!(
            rows.keys().filter(|key| key.starts_with("census/")).count(),
            declared,
            "{name}: the census is carried exactly as declared"
        );
        if declared > 0 {
            with_a_census += 1;
        }
        assert!(seen.insert(state), "{name}: the states are distinct");
    }
    assert_eq!(seen.len(), 3, "all three not-computed states are measured");
    assert!(
        with_a_census > 0,
        "a census assertion over none but empty censuses measures nothing"
    );
}

/// Adapted intake is idempotent and refuses a collision.
///
/// Trace: FR-066-AC-11
/// Provenance: quoin#480
#[test]
fn tc_480_029_adapted_intake_is_idempotent_and_collision_safe() {
    let transcribed = adapt_quality(&case("quality/measured")).expect("admissible");
    let root = tempfile::tempdir().expect("a temporary repository");
    let assurance = root.path().join("spec/assurance");
    std::fs::create_dir_all(&assurance).expect("the plan directory");
    std::fs::write(
        assurance.join("MP-001.md"),
        "---\ntype: MeasurementPlan\nid: MP-001\ntitle: Graph quality\nstatus: active\nstage: \
         observe\nmetric: graph_quality\ndefinition_version: \"quire-code.graph-quality-v1\"\n\
         ---\n\n# Graph quality\n",
    )
    .expect("the plan is written");

    let first =
        quoin_measurement::write_measurement_collection(root.path(), transcribed.document())
            .expect("the transcription is publishable");
    let again =
        quoin_measurement::write_measurement_collection(root.path(), transcribed.document())
            .expect("republishing identical bytes is idempotent");
    assert_eq!(first, again, "the same collection lands at the same path");

    let read = quoin_measurement::read_measurement_collections(
        &quoin_measurement::source::DiskMeasurement::new(root.path()),
    )
    .expect("the store is readable");
    assert_eq!(read.len(), 1, "one publication, one retained collection");
    assert_eq!(
        read[0].collection_id,
        transcribed.collection().collection_id,
        "what was read back is what was written"
    );

    let mut changed = quoin_measurement::json_bridge::to_serde(transcribed.document())
        .expect("the document crosses the bridge");
    changed["subject"] = Value::String("changed".to_owned());
    let collided = quoin_measurement::json_bridge::from_serde(&changed)
        .expect("the changed document crosses the bridge");
    let error = quoin_measurement::write_measurement_collection(root.path(), &collided)
        .expect_err("a retained id holding different bytes is a collision");
    assert_eq!(
        error.code(),
        quoin_measurement::MeasurementErrorCode::CollectionIdCollision,
        "the collision is named, not silently overwritten"
    );
}

/// The adapters have no producer, network, Git, filesystem or frontmatter
/// dependency.
///
/// Trace: FR-066-AC-12
/// Provenance: quoin#480
#[test]
fn tc_480_030_the_adapters_have_no_execution_network_or_filesystem_dependency() {
    /// The modules that are `graph-adapters.ts`. `loader` and `structural` are
    /// deliberately absent: they are `graph-portfolio-load.ts`, the one module
    /// that is *allowed* to read bytes, and FR-067-AC-11 measures them.
    const ADAPTER_MODULES: [&str; 9] = [
        "src/adapter.rs",
        "src/assurance",
        "src/attestation.rs",
        "src/base64.rs",
        "src/canonical.rs",
        "src/error.rs",
        "src/fields.rs",
        "src/quality",
        "src/scalars.rs",
    ];

    /// What none of them may reach for.
    const FORBIDDEN: [&str; 6] = [
        "std::process",
        "std::net",
        "std::fs",
        "quoin_graph_analysis",
        "Command::new",
        "include_str!",
    ];

    /// Below this the census is reading an empty tree and proving nothing.
    const SOURCE_FLOOR: usize = 20;

    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut measured = 0_usize;
    for entry in ADAPTER_MODULES {
        for path in rust_files(&crate_root.join(entry)) {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: unreadable: {error}", path.display()));
            for needle in FORBIDDEN {
                assert!(
                    !text.contains(needle),
                    "{}: an adapter reaches for `{needle}`",
                    path.display()
                );
            }
            measured += 1;
        }
    }
    assert!(
        measured >= SOURCE_FLOOR,
        "the census read {measured} adapter modules, below the floor of {SOURCE_FLOOR}"
    );
}

/// Omitting one `verificationStack.toolchains` language is admitted as "not
/// applicable" (PLAT-930), but an object naming none of the three is refused
/// exactly as an absent `toolchains` member is — the same rule
/// `quoin-measurement`'s own type enforces, applied here because
/// `InvocationAttestation::parse` is a second, independent production path
/// that reads the identical business rule (review finding #2 and #3 on
/// quoin#580).
///
/// Trace: FR-066-AC-6
/// Provenance: PLAT-930
#[test]
fn tc_480_031_a_toolchain_language_may_be_omitted_but_not_all_three() {
    let base = case("quality/measured");

    for language in ["node", "rust", "python"] {
        let mut admitted = base.clone();
        delete_at(
            &mut admitted["attestation"],
            &["verificationStack", "toolchains", language],
        );
        adapt_quality(&admitted).unwrap_or_else(|error| {
            panic!("an attestation with no `{language}` identity must be admitted: {error}")
        });
    }

    let mut empty = base.clone();
    empty["attestation"]["verificationStack"]["toolchains"] = serde_json::json!({});
    assert_eq!(
        refusal(&empty, "an empty toolchains object"),
        GraphAdapterErrorCode::InvalidAttestation,
    );
}

/// Every `.rs` at or under a path.
fn rust_files(path: &Path) -> Vec<std::path::PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(path)
        .unwrap_or_else(|error| panic!("{}: unreadable: {error}", path.display()));
    for entry in entries {
        let entry = entry.expect("a directory entry");
        let child = entry.path();
        if child.is_dir() {
            out.extend(rust_files(&child));
        } else if child.extension().is_some_and(|extension| extension == "rs") {
            out.push(child);
        }
    }
    out.sort();
    out
}
