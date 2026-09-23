// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What one FR looks like as input to the lens (PLAT-837), and how much of
//! it is allowed onto the wire (PLAT-983).
//!
//! This lens "reads spec text only" (PLAT-837's own words): no code, no
//! trace tags, no AST. [`FrContext`] is what a caller read off disk;
//! [`BoundedContext`] is what this crate sends to Jev as `state`. The only
//! way from one to the other is [`FrContext::bound`] under a
//! [`ContextPolicy`], and [`crate::lens`] accepts only the bounded form, so
//! no FR prose reaches a request without a stated byte cap.
//!
//! # Why the wire is bounded (PLAT-983)
//!
//! PLAT-917's live evaluation of the criterion-strength lens measured, on
//! the shipped question set against `jev-latest`, 2026-09-21:
//!
//! | variant | FR context sent | agreement | tokens/request |
//! | --- | --- | --- | --- |
//! | `v0` | the fixture corpus's own fields | 46.7% | 1521 |
//! | `v1` | full statement, Description, Behavior, Constraints, verbatim from the cited spec file | 33.3% | 2015 |
//!
//! "Full FR context made the shipped question *worse* (46.7% to 33.3%)",
//! and the vendor's own jaggedness notes name "Noisy State -- large
//! irrelevant context acts as a distractor" as a known failure mode. Until
//! this module, that finding lived only in a test-module doc, and nothing
//! stopped the next caller from sending a 10 KB Description again.
//!
//! What the two variants measurably differed in is size. The largest prose
//! field `v0` sent was 232 bytes (a Behavior bullet); `v1`'s prose reached
//! 10,340 bytes (one Description), and 17 of its 26 prose fields were over
//! 256 bytes. [`DEFAULT_MAX_PROSE_BYTES`] sits just above `v0`'s largest
//! field, so every field value the better-scoring configuration sent passes
//! unchanged and the worse one's oversized sections do not. (The `state`
//! object itself is not byte-identical to `v0`'s: `v0` sent an absent
//! section as `null`, and this module sends no key for it.) `v1` changed all four
//! fields at once, so the experiment does not attribute the harm to any one
//! of them; that is why the default caps every prose section rather than
//! omitting one, and why omission ([`ContextPolicy::omitting`]) is available
//! but unused by default.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Map, Value};
use typesafe_sdk_questions::Entry;

/// Default cap on each prose section (Description, Behaviour,
/// Constraints), in UTF-8 bytes, truncation marker included.
///
/// 256: the largest prose field PLAT-917's better-scoring `v0` sent was 232
/// bytes, so every `v0` field value passes unchanged; `v1`'s prose, which
/// scored 13.4 points lower, reached 10,340 bytes. See the module doc.
pub const DEFAULT_MAX_PROSE_BYTES: usize = 256;

/// Default cap on the FR statement, in UTF-8 bytes, truncation marker
/// included.
///
/// 512: the statement is the FR's `shall` sentence, which the shipped
/// `restates_requirement` question compares each AC against, so it is
/// bounded more loosely than prose rather than dropped. The longest
/// statement either PLAT-917 variant sent was 393 bytes.
pub const DEFAULT_MAX_STATEMENT_BYTES: usize = 512;

/// Opens the marker appended to a truncated field. A reader (or Jev) can
/// find every cut by searching for this string; the full marker reads
/// `" [truncated by quoin-jev: kept K of N bytes]"`.
pub const TRUNCATION_MARKER: &str = " [truncated by quoin-jev: kept ";

/// The smallest cap a [`ContextPolicy`] accepts: the longest marker any cut
/// can produce, with both counts at `usize::MAX`'s 20 digits. A cap is a
/// bound on the whole emitted field, marker included, so a cap too small to
/// hold the marker could not say a cut happened without breaking itself.
pub const MIN_MAX_BYTES: usize =
    TRUNCATION_MARKER.len() + " of ".len() + " bytes]".len() + 2 * USIZE_DIGITS;

/// Decimal digits in `usize::MAX`.
const USIZE_DIGITS: usize = usize::MAX.ilog10() as usize + 1;

/// One acceptance-criterion row, read verbatim from the spec.
#[derive(Debug, Clone, Serialize)]
pub struct AcRow {
    /// The row's id, e.g. `FR-079-AC-4`.
    pub id: String,
    /// The row's own text, unmodified.
    pub text: String,
}

/// One FR's context: its statement, prose, and AC set, as read.
///
/// PLAT-837's request shape: "State: the FR statement, the AC row, the FR's
/// Description/Behaviour/Constraints." This is the unbounded input; it goes
/// to Jev only through [`Self::bound`].
#[derive(Debug, Clone, Serialize)]
pub struct FrContext {
    /// The FR's id, e.g. `FR-079`.
    pub fr_id: String,
    /// The FR's normative statement (its `shall` sentence).
    pub statement: String,
    /// The FR's Description section, when present.
    pub description: Option<String>,
    /// The FR's Behaviour section, when present.
    pub behaviour: Option<String>,
    /// The FR's Constraints section, when present.
    pub constraints: Option<String>,
    /// Every AC row under this FR, in document order.
    pub acceptance_criteria: Vec<AcRow>,
}

impl FrContext {
    /// Every AC id under this FR, in order -- what
    /// [`crate::question_set::QuestionSet::questions_for_fr`] and
    /// [`crate::verdict::extract`] both key off.
    #[must_use]
    pub fn ac_ids(&self) -> Vec<String> {
        ids_of(&self.acceptance_criteria)
    }

    /// Applies `policy`, producing the only form [`crate::lens`] will send.
    ///
    /// AC rows are never cut: they are what every question is asked about,
    /// and a truncated criterion would be a different criterion.
    #[must_use]
    pub fn bound(&self, policy: &ContextPolicy) -> BoundedContext {
        let mut bounds = Vec::new();
        let statement = cap(
            Field::Statement,
            &self.statement,
            policy.max_statement_bytes,
            &mut bounds,
        );
        let mut prose = |section: Section, text: Option<&String>| {
            let text = text?;
            if policy.omitted.contains(&section) {
                bounds.push(Bound::Omitted { section });
                return None;
            }
            Some(cap(
                section.into(),
                text,
                policy.max_prose_bytes,
                &mut bounds,
            ))
        };
        let description = prose(Section::Description, self.description.as_ref());
        let behaviour = prose(Section::Behaviour, self.behaviour.as_ref());
        let constraints = prose(Section::Constraints, self.constraints.as_ref());
        BoundedContext {
            fr_id: self.fr_id.clone(),
            statement,
            description,
            behaviour,
            constraints,
            acceptance_criteria: self.acceptance_criteria.clone(),
            bounds,
        }
    }
}

/// A free-text field of [`FrContext`] that bounding can cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Field {
    /// [`FrContext::statement`].
    Statement,
    /// [`FrContext::description`].
    Description,
    /// [`FrContext::behaviour`].
    Behaviour,
    /// [`FrContext::constraints`].
    Constraints,
}

impl Field {
    /// The field's key in the `state` object sent to Jev.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Statement => "statement",
            Self::Description => "description",
            Self::Behaviour => "behaviour",
            Self::Constraints => "constraints",
        }
    }
}

/// A prose section a [`ContextPolicy`] may omit outright.
///
/// The statement is not one: without it the `restates_requirement` question
/// has nothing to compare an AC against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    /// [`FrContext::description`].
    Description,
    /// [`FrContext::behaviour`].
    Behaviour,
    /// [`FrContext::constraints`].
    Constraints,
}

impl From<Section> for Field {
    fn from(section: Section) -> Self {
        match section {
            Section::Description => Self::Description,
            Section::Behaviour => Self::Behaviour,
            Section::Constraints => Self::Constraints,
        }
    }
}

/// How much of an [`FrContext`] may reach the wire.
///
/// A policy value passed to [`FrContext::bound`], rather than a builder on
/// `FrContext` or an extra argument to an `Entry` conversion, so that the
/// bounded result is its own type: [`crate::lens::build_request`] and
/// [`crate::lens::run`] accept only [`BoundedContext`], which makes "was
/// this bounded?" a compile-time fact, and the [`Bound`] record of what was
/// cut travels with the value that was cut.
///
/// [`Default`] is the measured guard (see the module doc). A caller with a
/// documented reason for more context opts in explicitly with
/// [`Self::with_max_prose_bytes`]; one with evidence that a section hurts a
/// given lens drops it with [`Self::omitting`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPolicy {
    max_statement_bytes: usize,
    max_prose_bytes: usize,
    omitted: BTreeSet<Section>,
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            max_statement_bytes: DEFAULT_MAX_STATEMENT_BYTES,
            max_prose_bytes: DEFAULT_MAX_PROSE_BYTES,
            omitted: BTreeSet::new(),
        }
    }
}

impl ContextPolicy {
    /// Caps the emitted statement at `bytes` UTF-8 bytes, marker included.
    /// A value below [`MIN_MAX_BYTES`] is raised to it.
    #[must_use]
    pub fn with_max_statement_bytes(mut self, bytes: usize) -> Self {
        self.max_statement_bytes = bytes.max(MIN_MAX_BYTES);
        self
    }

    /// Caps each emitted prose section at `bytes` UTF-8 bytes, marker
    /// included. A value below [`MIN_MAX_BYTES`] is raised to it.
    /// `usize::MAX` sends sections whole -- PLAT-917's `v1`, which scored
    /// lower.
    #[must_use]
    pub fn with_max_prose_bytes(mut self, bytes: usize) -> Self {
        self.max_prose_bytes = bytes.max(MIN_MAX_BYTES);
        self
    }

    /// Drops `section` from the wire entirely.
    #[must_use]
    pub fn omitting(mut self, section: Section) -> Self {
        self.omitted.insert(section);
        self
    }
}

/// One thing [`FrContext::bound`] withheld from the wire.
///
/// Never silent: a truncation is also marked in the text itself with
/// [`TRUNCATION_MARKER`], so the request is self-describing even to a
/// reader who never sees this record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bound {
    /// `field` was cut to its first `kept_bytes` of `original_bytes`.
    Truncated {
        /// The field that was cut.
        field: Field,
        /// Its length as read, in UTF-8 bytes.
        original_bytes: usize,
        /// How many of those bytes were sent, before the marker.
        kept_bytes: usize,
    },
    /// `section` was present in the spec and omitted by policy.
    Omitted {
        /// The section that was dropped.
        section: Section,
    },
}

/// An [`FrContext`] after a [`ContextPolicy`]: what the lens sends as
/// `state`. Built only by [`FrContext::bound`].
#[derive(Debug, Clone)]
pub struct BoundedContext {
    fr_id: String,
    statement: String,
    description: Option<String>,
    behaviour: Option<String>,
    constraints: Option<String>,
    acceptance_criteria: Vec<AcRow>,
    bounds: Vec<Bound>,
}

impl BoundedContext {
    /// Every AC id, in document order; identical to [`FrContext::ac_ids`],
    /// since AC rows are never bounded.
    #[must_use]
    pub fn ac_ids(&self) -> Vec<String> {
        ids_of(&self.acceptance_criteria)
    }

    /// What the policy withheld, in field order. Empty when the whole
    /// context went out unchanged.
    #[must_use]
    pub fn bounds(&self) -> &[Bound] {
        &self.bounds
    }
}

impl From<&BoundedContext> for Entry {
    /// Builds the `state` entry PLAT-837's request shape asks for.
    ///
    /// A plain JSON object, not a formatted string: the API accepts rich
    /// JSON anywhere it accepts a description (`typesafe-sdk-questions`'s own
    /// `Entry` doc), and a structured object keeps the FR statement, its
    /// prose sections and its AC rows addressable rather than concatenated
    /// into text Jev would have to re-parse. A section the spec lacks, or
    /// the policy omitted, has no key at all -- an absent key costs Jev
    /// nothing to read.
    fn from(context: &BoundedContext) -> Self {
        let mut state = Map::new();
        state.insert("fr_id".to_owned(), Value::from(context.fr_id.as_str()));
        state.insert(
            Field::Statement.key().to_owned(),
            Value::from(context.statement.as_str()),
        );
        for (section, text) in [
            (Section::Description, &context.description),
            (Section::Behaviour, &context.behaviour),
            (Section::Constraints, &context.constraints),
        ] {
            if let Some(text) = text {
                state.insert(
                    Field::from(section).key().to_owned(),
                    Value::from(text.as_str()),
                );
            }
        }
        let rows = context
            .acceptance_criteria
            .iter()
            .map(|row| {
                let mut object = Map::new();
                object.insert("id".to_owned(), Value::from(row.id.as_str()));
                object.insert("text".to_owned(), Value::from(row.text.as_str()));
                Value::Object(object)
            })
            .collect();
        state.insert("acceptance_criteria".to_owned(), Value::Array(rows));
        Value::Object(state).into()
    }
}

fn ids_of(rows: &[AcRow]) -> Vec<String> {
    rows.iter().map(|row| row.id.clone()).collect()
}

/// The marker closing a field cut to `kept` of `total` bytes.
fn marker(kept: usize, total: usize) -> String {
    format!("{TRUNCATION_MARKER}{kept} of {total} bytes]")
}

/// Returns `text` unchanged when it fits in `max` bytes. Otherwise returns
/// its longest character-boundary prefix that, with the truncation marker
/// appended, still fits in `max` bytes, and records the cut in `bounds`.
///
/// The marker's length depends on the kept count, which is not known until
/// the budget is, so the budget reserves the marker for a kept count of
/// `max`: `kept <= max`, so the real marker is never longer. With
/// `max >= MIN_MAX_BYTES` (every [`ContextPolicy`] holds that) the result is
/// at most `max` bytes.
fn cap(field: Field, text: &str, max: usize, bounds: &mut Vec<Bound>) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let budget = max.saturating_sub(marker(max, text.len()).len());
    let kept = text.floor_char_boundary(budget);
    // `kept` is a character boundary by construction, so `get` cannot miss;
    // it is used over slicing only to stay inside the panic-free lint set.
    let head = text.get(..kept).unwrap_or_default();
    bounds.push(Bound::Truncated {
        field,
        original_bytes: text.len(),
        kept_bytes: kept,
    });
    format!("{head}{}", marker(kept, text.len()))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{
        AcRow, Bound, ContextPolicy, DEFAULT_MAX_PROSE_BYTES, DEFAULT_MAX_STATEMENT_BYTES, Field,
        FrContext, MIN_MAX_BYTES, Section, TRUNCATION_MARKER,
    };
    use serde_json::Value;
    use typesafe_sdk_questions::Entry;

    const CORPUS: &str = include_str!(
        "../../../../skills/spec-criterion-strength-analysis/assets/fixtures/criterion-strength-fixtures.json"
    );

    fn context(description: Option<String>) -> FrContext {
        FrContext {
            fr_id: "FR-001".to_owned(),
            statement: "The system SHALL do a thing.".to_owned(),
            description,
            behaviour: None,
            constraints: None,
            acceptance_criteria: vec![AcRow {
                id: "FR-001-AC-1".to_owned(),
                text: "first".to_owned(),
            }],
        }
    }

    fn state(context: &FrContext, policy: &ContextPolicy) -> Value {
        let entry: Entry = (&context.bound(policy)).into();
        entry.0
    }

    fn keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<_> = value
            .as_object()
            .expect("state is a JSON object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    /// Provenance: PLAT-837
    #[test]
    fn ac_ids_preserve_document_order() {
        let mut context = context(None);
        context.acceptance_criteria.push(AcRow {
            id: "FR-001-AC-2".to_owned(),
            text: "second".to_owned(),
        });
        assert_eq!(context.ac_ids(), vec!["FR-001-AC-1", "FR-001-AC-2"]);
        assert_eq!(
            context.bound(&ContextPolicy::default()).ac_ids(),
            vec!["FR-001-AC-1", "FR-001-AC-2"]
        );
    }

    /// Provenance: PLAT-837, PLAT-983. `state` carries exactly the fields the
    /// request shape names -- FR statement, AC rows, Description/Behaviour/
    /// Constraints -- and nothing this lens was not told to send. Since
    /// PLAT-983 a section the spec lacks has no key rather than a `null`
    /// one, so `behaviour` and `constraints` are absent here.
    #[test]
    fn state_carries_the_documented_fields_only() {
        let value = state(&context(Some("desc".to_owned())), &ContextPolicy::default());
        assert_eq!(
            keys(&value),
            vec!["acceptance_criteria", "description", "fr_id", "statement"]
        );
        assert_eq!(
            value["acceptance_criteria"],
            serde_json::json!([{"id": "FR-001-AC-1", "text": "first"}])
        );
    }

    /// Provenance: PLAT-983. A field within its cap passes through
    /// unchanged and records no bound.
    #[test]
    fn a_field_within_bounds_passes_through_unchanged() {
        let text = "d".repeat(DEFAULT_MAX_PROSE_BYTES);
        let context = context(Some(text.clone()));
        let bounded = context.bound(&ContextPolicy::default());
        assert!(bounded.bounds().is_empty());
        let value = state(&context, &ContextPolicy::default());
        assert_eq!(value["description"], Value::from(text));
        assert_eq!(value["statement"], "The system SHALL do a thing.");
    }

    /// Provenance: PLAT-983. A field over its cap is cut, marked in the text
    /// itself, and recorded -- never silently shortened. The cap bounds the
    /// whole emitted field, marker included: 206 source bytes plus the
    /// 50-byte marker is exactly 256.
    #[test]
    fn an_oversized_field_is_truncated_with_a_visible_marker() {
        // The size of PLAT-917 v1's largest Description.
        let context = context(Some("x".repeat(10_340)));
        let bounded = context.bound(&ContextPolicy::default());
        assert_eq!(
            bounded.bounds(),
            [Bound::Truncated {
                field: Field::Description,
                original_bytes: 10_340,
                kept_bytes: 206,
            }]
        );
        let value = state(&context, &ContextPolicy::default());
        let description = value["description"].as_str().expect("a string");
        assert_eq!(
            description,
            format!("{}{TRUNCATION_MARKER}206 of 10340 bytes]", "x".repeat(206))
        );
        assert_eq!(description.len(), DEFAULT_MAX_PROSE_BYTES);
    }

    /// Provenance: PLAT-983. The cut never splits a UTF-8 character: `é` is
    /// two bytes, so a budget landing between them keeps one byte fewer.
    /// At a cap of 101, the budget is 101 - 48 = 53 bytes, floored to 52.
    #[test]
    fn truncation_lands_on_a_character_boundary() {
        let context = context(Some("é".repeat(200)));
        let policy = ContextPolicy::default().with_max_prose_bytes(101);
        assert_eq!(
            context.bound(&policy).bounds(),
            [Bound::Truncated {
                field: Field::Description,
                original_bytes: 400,
                kept_bytes: 52,
            }]
        );
        let value = state(&context, &policy);
        let description = value["description"].as_str().expect("a string");
        assert_eq!(
            description,
            format!("{}{TRUNCATION_MARKER}52 of 400 bytes]", "é".repeat(26))
        );
        assert!(description.len() <= 101);
    }

    /// Provenance: PLAT-983. The statement has its own, looser cap, which
    /// also includes the marker.
    #[test]
    fn the_statement_is_capped_separately() {
        let mut context = context(None);
        context.statement = "s".repeat(DEFAULT_MAX_STATEMENT_BYTES + 1);
        let bounded = context.bound(&ContextPolicy::default());
        assert_eq!(
            bounded.bounds(),
            [Bound::Truncated {
                field: Field::Statement,
                original_bytes: DEFAULT_MAX_STATEMENT_BYTES + 1,
                kept_bytes: 464,
            }]
        );
        let value = state(&context, &ContextPolicy::default());
        assert_eq!(
            value["statement"].as_str().expect("a string").len(),
            DEFAULT_MAX_STATEMENT_BYTES
        );
    }

    /// Provenance: PLAT-983. No emitted field is ever longer than its cap,
    /// at any cap from the minimum up and any text length around it --
    /// including lengths whose count gains a digit, which lengthens the
    /// marker.
    #[test]
    fn no_emitted_field_exceeds_its_cap() {
        for cap in [MIN_MAX_BYTES, MIN_MAX_BYTES + 1, 99, 100, 101, 256, 1000] {
            let policy = ContextPolicy::default().with_max_prose_bytes(cap);
            for length in [cap + 1, cap + 2, 999, 1000, 1001, 10_340, 100_000] {
                if length <= cap {
                    continue;
                }
                for unit in ["x", "é", "\u{1F600}"] {
                    let text = unit.repeat(length.div_ceil(unit.len()));
                    let value = state(&context(Some(text.clone())), &policy);
                    let emitted = value["description"].as_str().expect("a string");
                    assert!(
                        emitted.len() <= cap,
                        "cap {cap}, source {} bytes of {unit:?}: emitted {} bytes",
                        text.len(),
                        emitted.len()
                    );
                    assert!(emitted.contains(TRUNCATION_MARKER));
                }
            }
        }
    }

    /// Provenance: PLAT-983. A cap too small to hold the marker is raised to
    /// [`MIN_MAX_BYTES`], so a cut can always say it happened; the defaults
    /// are above that floor.
    #[test]
    fn a_cap_below_the_marker_is_raised_to_the_floor() {
        const { assert!(DEFAULT_MAX_PROSE_BYTES >= MIN_MAX_BYTES) };
        const { assert!(DEFAULT_MAX_STATEMENT_BYTES >= MIN_MAX_BYTES) };
        let policy = ContextPolicy::default()
            .with_max_prose_bytes(5)
            .with_max_statement_bytes(0);
        assert_eq!(
            policy,
            ContextPolicy::default()
                .with_max_prose_bytes(MIN_MAX_BYTES)
                .with_max_statement_bytes(MIN_MAX_BYTES)
        );
        let value = state(&context(Some("x".repeat(1000))), &policy);
        let emitted = value["description"].as_str().expect("a string");
        assert!(emitted.len() <= MIN_MAX_BYTES);
        assert!(emitted.contains(TRUNCATION_MARKER));
    }

    /// Provenance: PLAT-983. An omitted section has no key in `state`, and
    /// the omission is recorded; omitting a section the spec lacks records
    /// nothing, because nothing was withheld.
    #[test]
    fn an_omitted_section_is_absent_and_recorded() {
        let mut context = context(Some("desc".to_owned()));
        context.behaviour = Some("beh".to_owned());
        let policy = ContextPolicy::default()
            .omitting(Section::Description)
            .omitting(Section::Constraints);
        assert_eq!(
            context.bound(&policy).bounds(),
            [Bound::Omitted {
                section: Section::Description
            }]
        );
        let value = state(&context, &policy);
        assert_eq!(
            keys(&value),
            vec!["acceptance_criteria", "behaviour", "fr_id", "statement"]
        );
    }

    /// Provenance: PLAT-983. A caller with a reason for more context can
    /// still send a section whole; the default just no longer does.
    #[test]
    fn a_caller_can_opt_into_whole_sections() {
        let text = "x".repeat(10_340);
        let context = context(Some(text.clone()));
        let policy = ContextPolicy::default().with_max_prose_bytes(usize::MAX);
        assert!(context.bound(&policy).bounds().is_empty());
        assert_eq!(state(&context, &policy)["description"], Value::from(text));
    }

    /// Provenance: PLAT-983. The default cap is grounded in PLAT-917's `v0`,
    /// the better-scoring configuration: every prose field the shipped corpus
    /// carries fits under it, so every `v0` field value passes unchanged.
    /// If the corpus grows a longer field, this fails and the cap is
    /// re-decided against a measurement, not raised to fit.
    #[test]
    fn every_v0_corpus_prose_field_fits_the_default_cap() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("the corpus parses");
        let mut measured = 0;
        for list in ["weakness_kind_fixtures", "adverse_case_coverage_fixtures"] {
            for fixture in corpus[list].as_array().expect("a fixture list") {
                for key in [
                    "fr_description",
                    "fr_behavior",
                    "fr_behavior_bullet",
                    "fr_constraint",
                ] {
                    if let Some(text) = fixture["source"][key].as_str() {
                        measured += 1;
                        assert!(
                            text.len() <= DEFAULT_MAX_PROSE_BYTES,
                            "{}.{key} is {} bytes",
                            fixture["fixture_id"],
                            text.len()
                        );
                    }
                }
            }
        }
        assert!(measured > 0, "no prose field was measured at all");
    }
}
