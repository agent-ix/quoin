// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `src/measurement/graph-adapters.ts` and this crate reach the same verdict on
//! the same evidence, except where this file says they do not (quoin#475).
//!
//! # The corpus
//!
//! `tests/goldens/graph-adapter-verdicts.json` was captured **once** by
//! `oracle/capture-graph-adapter-verdicts.mjs`, which imports the retained
//! module rather than reimplementing it. FR-101 AC-5 forbids a Rust test
//! spawning node, so the capture is committed and this test reads it; the
//! capture script is deleted by the W12 cutover commit, with the TypeScript it
//! captures.
//!
//! The cases are not hand-picked happy paths. They are the two adapters, the
//! identity function, the adapter selector and the base64 boundary, each run
//! over a base document and a named mutation ladder: every top-level member of
//! the assurance export removed in turn, both premise members disagreed with,
//! every arm of the locator-path lookahead, the zod UUID grammar at its
//! version and variant boundaries, every population state, every refusal the
//! transcription can raise, five plan configurations, seven timestamps and
//! seventeen base64 spellings.
//!
//! [`CORPUS_FLOOR`] keeps this from passing by measuring nothing, and
//! [`tc_475_052_the_corpus_carries_both_verdicts_for_every_entry_point`]
//! keeps it from passing by measuring only refusals: a corpus that refuses
//! everything agrees perfectly with an implementation that refuses everything.
//!
//! # Divergences are declared, and must fire
//!
//! [`DECLARED_DIVERGENCES`] names every case where the two disagree, with the
//! reason. A declared divergence that does *not* fire is a failure too: it
//! means the corpus no longer exercises the thing the declaration was written
//! about, and the declaration has quietly become a licence.
//!
//! There is a third *difference* that is deliberately not declared, because a
//! declaration that cannot fire is the same licence by another route: the
//! observation identity's member ordering. It is proved unreachable instead,
//! by [`tc_475_054_the_identity_ordering_difference_is_unreachable`].
//!
//! Trace: FR-066-AC-1, FR-101-AC-5
//! Provenance: quoin#475, quoin#465

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;
use std::path::Path;

use quoin_measurement::MeasurementPlan;
use quoin_measurement::types::plan::{LifecycleStatus, MeasurementStage};
use quoin_measurement_graph::assurance::premise::{
    AcceptedQuirePremises, ModulePremise, SourcePremise,
};
use quoin_measurement_graph::quality::adapt::AdaptGraphQualityInput;
use quoin_measurement_graph::{
    GraphAdapterErrorCode, adapt_graph_quality_observation, adapt_quire_assurance, base64,
    canonical, graph_quality_observation_id, select_graph_adapter,
};
use serde_json::Value;

/// The count below which this measurement is measuring nothing.
const CORPUS_FLOOR: usize = 80;

/// The lowest number of accepted and of refused cases the corpus must carry.
///
/// Both, because agreement on a corpus of one kind is not agreement.
const VERDICT_FLOOR: usize = 20;

/// Every case where the retained module and this crate disagree, with why.
///
/// `(case name, the retained module accepts, this crate accepts, why)`.
///
/// Three reasons, and no fourth. Each is argued at the module it arises in;
/// the sentences here are the short form.
const DECLARED_DIVERGENCES: &[(&str, bool, bool, &str)] = &[
    // 1. `Date.parse` is ECMA-262's implementation-defined fallback parser.
    // Beyond the Date Time String Format it accepts whatever V8's heuristic
    // accepts. This workspace has one instant grammar —
    // `quoin_measurement::Rfc3339DateTime`, unified on the permissive RFC 3339
    // reading by quoin#440 — and the Stage 6 plan §5 forbids a seventh. An
    // attestation whose instant only one implementation can read is not an
    // attestation of when.
    (
        "quality/timestamp/date-only",
        true,
        false,
        "`2026-01-02` is a date, not an instant: Date.parse reads it as midnight UTC",
    ),
    (
        "quality/timestamp/v8-heuristic-slashes",
        true,
        false,
        "`2026/01/02 10:00` is V8's Date.parse heuristic, not RFC 3339",
    ),
    (
        "quality/timestamp/v8-heuristic-words",
        true,
        false,
        "`Jan 1 2026` is V8's Date.parse heuristic, not RFC 3339",
    ),
    // 2. Node's `Buffer.from(x, "base64")` silently discards every character
    // outside the alphabet and every length rule, so it turns malformed input
    // into plausible bytes rather than an error. This is quoin#465. A strict
    // RFC 4648 §4 decoder refuses what Node accepted; the bytes it *does*
    // return are identical wherever Node's input was well formed.
    (
        "base64/decode/\"aGVsbG8\"",
        true,
        false,
        "quoin#465: unpadded length 7 — Node pads silently",
    ),
    (
        "base64/decode/\"aGV sbG8=\"",
        true,
        false,
        "quoin#465: an interior space — Node discards it",
    ),
    (
        "base64/decode/\"aGVsbG8=extra\"",
        true,
        false,
        "quoin#465: trailing text after the padding — Node stops reading",
    ),
    (
        "base64/decode/\"a GVsbG8=\"",
        true,
        false,
        "quoin#465: an interior space — Node discards it",
    ),
    (
        "base64/decode/\"aGVsbG8==\"",
        true,
        false,
        "quoin#465: three padding characters — Node discards the excess",
    ),
    (
        "base64/decode/\"!!!!\"",
        true,
        false,
        "quoin#465: no alphabet character at all — Node returns zero bytes",
    ),
    (
        "base64/decode/\"aGVs*bG8=\"",
        true,
        false,
        "quoin#465: an interior `*` — Node discards it",
    ),
    (
        "base64/decode/\"=aGVsbG8\"",
        true,
        false,
        "quoin#465: padding first — Node discards it",
    ),
    (
        "base64/decode/\"AA=\"",
        true,
        false,
        "quoin#465: length 3 — Node pads silently",
    ),
    (
        "base64/decode/\"A\"",
        true,
        false,
        "quoin#465: a single character encodes no byte — Node returns zero bytes",
    ),
    (
        "base64/decode/\"aGVsbG8\\n\"",
        true,
        false,
        "quoin#465: a trailing newline — Node discards it",
    ),
    (
        "base64/decode/\"aGVs\\nbG8=\"",
        true,
        false,
        "quoin#465: an interior newline — Node discards it",
    ),
    (
        "base64/decode/\"-_8=\"",
        true,
        false,
        "quoin#465: base64url characters in a base64 field — Node accepts both alphabets",
    ),
];

/// The three divergence families, so that losing one is a failure rather than
/// a smaller list.
///
/// `(family, the case-name prefix it is recognised by, why it exists)`.
const DIVERGENCE_FAMILIES: &[(&str, &str, &str)] = &[
    (
        "Date.parse heuristic",
        "quality/timestamp/",
        "one instant grammar in this workspace, per the Stage 6 plan §5",
    ),
    (
        "lenient base64 decode",
        "base64/decode/",
        "quoin#465: Node's decoder discards what it cannot read",
    ),
];

/// One captured case.
struct Case {
    name: String,
    function: String,
    input: Value,
    accepted: bool,
    code: Option<String>,
    output: Option<Value>,
    message: Option<String>,
}

impl Case {
    /// The retained refusal sentence, for diagnostics only.
    fn message(&self) -> &str {
        self.message.as_deref().unwrap_or("")
    }
}

/// Read the committed capture.
fn corpus() -> Vec<Case> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/graph-adapter-verdicts.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
    let golden: Value = serde_json::from_str(&text).expect("the capture is JSON");
    golden["cases"]
        .as_array()
        .expect("the capture carries cases")
        .iter()
        .map(|case| Case {
            name: case["name"].as_str().expect("a name").to_owned(),
            function: case["fn"].as_str().expect("a function").to_owned(),
            input: case["input"].clone(),
            accepted: case["verdict"] == "accepted",
            code: case["code"].as_str().map(str::to_owned),
            output: case.get("output").cloned().filter(|out| !out.is_null()),
            message: case["message"].as_str().map(str::to_owned),
        })
        .collect()
}

/// What one case produced here: the verdict, the retained code spelling, and
/// the output rendered the way the capture rendered it.
struct Outcome {
    accepted: bool,
    code: Option<&'static str>,
    output: Option<Value>,
    /// The refusal sentence. Never asserted — diagnostics are not contractual —
    /// but reported when an assertion fails, so a disagreement names itself.
    detail: String,
}

impl Outcome {
    fn accepted(output: Value) -> Self {
        Self {
            accepted: true,
            code: None,
            output: Some(output),
            detail: String::new(),
        }
    }

    fn refused(error: &quoin_measurement_graph::GraphAdapterError) -> Self {
        Self {
            accepted: false,
            code: error.code().retained_spelling(),
            output: None,
            detail: error.to_string(),
        }
    }
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
    }
}

/// The bytes a capture recorded as an array of numbers.
fn bytes(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .expect("an array of byte values")
        .iter()
        .map(|byte| u8::try_from(byte.as_u64().expect("a byte")).expect("a byte"))
        .collect()
}

/// Run one case through this crate.
fn run(case: &Case) -> Outcome {
    match case.function.as_str() {
        "selectGraphAdapter" => {
            match select_graph_adapter(case.input["name"].as_str().expect("a name")) {
                Ok(name) => Outcome::accepted(Value::String(name.as_str().to_owned())),
                Err(error) => Outcome::refused(&error),
            }
        }
        "graphQualityObservationId" => match graph_quality_observation_id(&case.input["value"]) {
            Ok(digest) => Outcome::accepted(Value::String(digest.to_stored())),
            Err(error) => Outcome::refused(&error),
        },
        "adaptQuireAssurance" => {
            let accepted = AcceptedQuirePremises {
                source: serde_json::from_value::<SourcePremise>(
                    case.input["accepted"]["source"].clone(),
                )
                .expect("the capture's accepted source is well formed"),
                modules: serde_json::from_value::<Vec<ModulePremise>>(
                    case.input["accepted"]["modules"].clone(),
                )
                .expect("the capture's accepted modules are well formed"),
            };
            match adapt_quire_assurance(&case.input["document"], &accepted) {
                // The retained function returns the zod-parsed document, and
                // `.strict()` means the parse neither drops nor adds a member,
                // so its canonical text is the canonical text of what went in.
                Ok(_) => Outcome::accepted(Value::String(
                    canonical::pretty_text(&case.input["document"]).expect("canonical text"),
                )),
                Err(error) => Outcome::refused(&error),
            }
        }
        "adaptGraphQualityObservation" => {
            let plans: Vec<MeasurementPlan> = case.input["plans"]
                .as_array()
                .expect("a plan list")
                .iter()
                .map(plan)
                .collect();
            let scorer = bytes(&case.input["scorerBytes"]);
            let input = AdaptGraphQualityInput {
                record: &case.input["record"],
                scorer_bytes: &scorer,
                scorer_media_type: case.input["scorerMediaType"]
                    .as_str()
                    .expect("a media type"),
                attestation: &case.input["attestation"],
                plans: &plans,
            };
            match adapt_graph_quality_observation(&input) {
                Ok(transcribed) => Outcome::accepted(Value::String(
                    quoin_store::canonical_json(transcribed.document()).expect("canonical text"),
                )),
                Err(error) => Outcome::refused(&error),
            }
        }
        "base64Encode" => {
            Outcome::accepted(Value::String(base64::encode(&bytes(&case.input["bytes"]))))
        }
        "base64Decode" => match base64::decode(case.input["text"].as_str().expect("text")) {
            Ok(decoded) => Outcome::accepted(Value::Array(
                decoded.into_iter().map(Value::from).collect(),
            )),
            Err(error) => Outcome::refused(&error),
        },
        other => panic!("the capture carries an unhandled function `{other}`"),
    }
}

/// Both implementations reach the same verdict, except where declared.
#[test]
fn tc_475_050_the_two_implementations_agree_on_every_captured_case() {
    let corpus = corpus();
    assert!(
        corpus.len() >= CORPUS_FLOOR,
        "anti-vacuity floor: {} cases, under {CORPUS_FLOOR} — a parity measurement over a corpus \
         this small measures nothing",
        corpus.len()
    );

    let mut fired: BTreeSet<&str> = BTreeSet::new();
    for case in &corpus {
        let outcome = run(case);
        if let Some((name, retained, ported, why)) = DECLARED_DIVERGENCES
            .iter()
            .find(|(name, _, _, _)| *name == case.name)
        {
            assert_eq!(
                (case.accepted, outcome.accepted),
                (*retained, *ported),
                "`{name}` is declared to diverge as retained={retained}/ported={ported} ({why}), \
                 but the measurement disagrees. Fix the code or restate the divergence; do not \
                 widen the declaration to fit."
            );
            fired.insert(name);
            continue;
        }
        assert_eq!(
            outcome.accepted,
            case.accepted,
            "`{}` ({}): the retained module {} and this crate {} ({}). An undeclared divergence \
             is a defect, not a nuance.",
            case.name,
            case.function,
            if case.accepted { "accepted" } else { "refused" },
            if outcome.accepted {
                "accepted"
            } else {
                "refused"
            },
            if outcome.accepted {
                case.message()
            } else {
                outcome.detail.as_str()
            }
        );
        if case.accepted {
            assert_eq!(
                outcome.output.as_ref(),
                case.output.as_ref(),
                "`{}` ({}): both accepted, and the outputs differ.",
                case.name,
                case.function
            );
        } else {
            assert_eq!(
                outcome.code,
                case.code.as_deref(),
                "`{}` ({}): both refused, and the codes differ. The retained code spelling is \
                 what a caller reads.",
                case.name,
                case.function
            );
        }
    }

    let declared: BTreeSet<&str> = DECLARED_DIVERGENCES
        .iter()
        .map(|(name, _, _, _)| *name)
        .collect();
    assert_eq!(
        declared, fired,
        "every declared divergence must fire. One that does not means the corpus stopped \
         exercising it, and the declaration has become a licence rather than a measurement."
    );
}

/// Each declared divergence family is represented, and none has been quietly
/// emptied.
#[test]
fn tc_475_051_every_divergence_family_is_represented() {
    for (family, prefix, why) in DIVERGENCE_FAMILIES {
        let count = DECLARED_DIVERGENCES
            .iter()
            .filter(|(name, _, _, _)| name.starts_with(prefix))
            .count();
        assert!(
            count > 0,
            "the `{family}` divergence family ({why}) has no declared case left. Either the \
             divergence was closed — then delete the family — or the corpus stopped covering it."
        );
    }
    let covered: usize = DIVERGENCE_FAMILIES
        .iter()
        .map(|(_, prefix, _)| {
            DECLARED_DIVERGENCES
                .iter()
                .filter(|(name, _, _, _)| name.starts_with(prefix))
                .count()
        })
        .sum();
    assert_eq!(
        covered,
        DECLARED_DIVERGENCES.len(),
        "a declared divergence belongs to no family; every divergence has a reason and the \
         families are the reasons"
    );
}

/// The corpus carries both verdicts, for every entry point.
#[test]
fn tc_475_052_the_corpus_carries_both_verdicts_for_every_entry_point() {
    let corpus = corpus();
    let accepted = corpus.iter().filter(|case| case.accepted).count();
    let refused = corpus.len() - accepted;
    assert!(
        accepted >= VERDICT_FLOOR && refused >= VERDICT_FLOOR,
        "the corpus carries {accepted} accepted and {refused} refused, and needs at least \
         {VERDICT_FLOOR} of each: a corpus that refuses everything agrees perfectly with an \
         implementation that refuses everything"
    );

    for entry_point in [
        "selectGraphAdapter",
        "adaptQuireAssurance",
        "adaptGraphQualityObservation",
    ] {
        let cases: Vec<&Case> = corpus
            .iter()
            .filter(|case| case.function == entry_point)
            .collect();
        assert!(
            cases.iter().any(|case| case.accepted),
            "{entry_point} has no accepted case; parity on refusals alone proves nothing"
        );
        assert!(
            cases.iter().any(|case| !case.accepted),
            "{entry_point} has no refused case; an adapter that accepts everything is not an \
             adapter"
        );
    }

    let codes: BTreeSet<&str> = corpus
        .iter()
        .filter_map(|case| case.code.as_deref())
        .collect();
    let retained: BTreeSet<&str> = GraphAdapterErrorCode::ALL
        .into_iter()
        .filter_map(GraphAdapterErrorCode::retained_spelling)
        .collect();
    assert_eq!(
        codes, retained,
        "the corpus exercises every retained refusal code exactly once over, and no other. A \
         code with no captured case is a refusal no measurement covers."
    );
}

/// The capture records what produced it.
#[test]
fn tc_475_053_the_capture_records_its_provenance() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/graph-adapter-verdicts.json");
    let golden: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("readable")).expect("JSON");
    let provenance = &golden["provenance"];
    for member in [
        "captured_by",
        "module",
        "module_digest",
        "node",
        "zod",
        "quoin_revision",
    ] {
        let value = provenance[member].as_str();
        assert!(
            value.is_some_and(|text| !text.is_empty()),
            "the capture must record `{member}`: a golden whose origin is unrecorded cannot be \
             re-taken, and an unrepeatable measurement is an assertion"
        );
    }
    assert_eq!(
        provenance["module"], "src/measurement/graph-adapters.ts",
        "the capture is of the module this crate ports"
    );
    assert_ne!(
        provenance["quoin_revision"], "unrecorded",
        "the capture must record the revision it was taken at"
    );
}

/// The identity's member-ordering difference cannot be reached.
///
/// `compactCanonicalProducerJson` sorts member names by Unicode **code point**;
/// [`quoin_store::canonical_bytes`] sorts them by UTF-16 **code unit**, as RFC
/// 8785 §3.2.3 requires. The two orders disagree only when one of a pair of
/// names begins with an astral character (`U+10000` and above, a high surrogate
/// in UTF-16) and the other with a character in `U+E000..U+FFFF`.
///
/// That is why the difference is not in [`DECLARED_DIVERGENCES`]: a declared
/// divergence must fire, and this one provably cannot. Every member name a
/// `graph_quality_observation` may carry is fixed by the schema and is ASCII —
/// the producer chooses census *values*, never names — so no pair of names in
/// any admissible record can separate the two orders.
///
/// This test proves both halves: that the corpus's accepted records carry only
/// ASCII names, and that the orders really do differ, so the first half is a
/// measurement rather than a tautology.
#[test]
fn tc_475_054_the_identity_ordering_difference_is_unreachable() {
    fn names(value: &Value, into: &mut BTreeSet<String>) {
        match value {
            Value::Object(members) => {
                for (name, member) in members {
                    into.insert(name.clone());
                    names(member, into);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| names(item, into)),
            _ => {}
        }
    }

    let mut seen = BTreeSet::new();
    let mut records = 0_usize;
    for case in corpus() {
        if case.function == "adaptGraphQualityObservation" && case.accepted {
            records += 1;
            names(&case.input["record"], &mut seen);
        }
    }
    assert!(
        records > 0 && seen.len() > 20,
        "anti-vacuity: {records} accepted records carrying {} distinct member names",
        seen.len()
    );
    for name in &seen {
        assert!(
            name.is_ascii(),
            "`{name}` is outside ASCII, so the code-point and code-unit orders could differ on \
             it; the identity difference is no longer provably unreachable and must become a \
             declared divergence"
        );
    }

    // The orders do differ, so the assertion above is a measurement.
    assert!(
        "\u{10000}" > "\u{FFFD}",
        "code-point order puts the astral name second"
    );
    assert_eq!(
        quoin_store::json::order::cmp_utf16("\u{10000}", "\u{FFFD}"),
        std::cmp::Ordering::Less,
        "code-unit order puts the astral name first"
    );
}
