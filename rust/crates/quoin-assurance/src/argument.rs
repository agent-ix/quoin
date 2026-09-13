// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Validating an authored assurance argument (quoin#384, ported from
//! `parseAssuranceArgument` in `src/assurance/argument.ts`).
//!
//! # Why this one is not a `#[derive(Deserialize)]`
//!
//! The two slices before this were transformers, and their Rust types carried
//! the work: [`crate::case::CaseInput`] deserialises and `build_case` runs.
//! This is a **validator**, and a validator's types carry almost none of it.
//!
//! The declared shape is 37 fields and every one is read — a field subset of
//! 100%, which is the number a validator always produces, because reading
//! every field IS the function. What the shape does not carry is **seventeen
//! predicates**: four format rules, three non-emptiness rules, six uniqueness
//! rules, and two graph reachability rules. A derive with
//! `deny_unknown_fields` reproduces none of them.
//!
//! So the port is written against [`serde_json::Value`] rather than against a
//! derived type. That is not a stylistic preference. Three of the behaviours
//! below are invisible to serde and would each have been a silent divergence:
//!
//! - `null` in an optional field is **asymmetric** in the retained source, and
//!   `Option<T>` collapses both halves. See [`Challenge`].
//! - JavaScript's `trim` and Rust's `str::trim` disagree on two code points,
//!   **in opposite directions**. See [`js_trim_is_empty`].
//! - An instant naming a day that does not exist must be refused, which
//!   `Date.parse` did not do until quoin#436 fixed the retained code.
//!
//! # Permissiveness runs both ways
//!
//! Recorded on quoin#384 and worth repeating where the code is: a port that
//! **accepts** input the retained implementation rejects is a divergence,
//! exactly as much as one that refuses what it accepts. Every deviation below
//! is annotated with which direction it would have gone.

use serde::{Deserialize, Serialize};

/// A rejection, carrying the retained implementation's own message.
///
/// The message is reproduced because it costs nothing and helps a human, but
/// it is deliberately **not** a contract: `quoin-difftest` compares the
/// diagnostic's `(code, context keys)` and never its prose (quoin#373).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentError(pub String);

impl std::fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ArgumentError {}

type Checked<T> = Result<T, ArgumentError>;

fn reject<T>(message: impl Into<String>) -> Checked<T> {
    Err(ArgumentError(message.into()))
}

/// The argument's lifecycle state. Closed, because the retained code validates.
///
/// quoin#425 asked whether a TypeScript union ports as a closed Rust type, and
/// answered "only where the retained code checks membership at run time".
/// `Finding.kind` was open because the retained renderer interpolates whatever
/// arrives. These go through `literal()`, which checks and **throws**, so
/// refusing an unlisted value IS the retained behaviour. Same test, different
/// answer, because the code differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgumentStatus {
    /// Authored but not yet in force.
    Proposed,
    /// In force.
    Active,
    /// Withdrawn.
    Retired,
}

/// An assumption's declared state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssumptionStatus {
    /// Stated and not yet decided.
    Open,
    /// Decided and standing.
    Accepted,
    /// Decided and no longer true.
    Invalidated,
}

/// A challenge's declared state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChallengeStatus {
    /// Raised and unanswered.
    Open,
    /// Answered, which requires a resolution reference.
    Resolved,
    /// Knowingly carried, which additionally requires a current expiry.
    AcceptedRisk,
}

/// How a relationship reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationshipType {
    /// The target supports this argument.
    Supports,
    /// The target challenges it.
    Challenges,
    /// The target is context.
    References,
}

/// The claim the whole argument exists to argue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopClaim {
    /// The claim's id, which every reasoning chain must reach.
    pub id: String,
    /// What is claimed.
    pub statement: String,
    /// What the claim is about.
    pub subject: String,
}

/// One step of reasoning, with the criteria that would make it sufficient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reasoning {
    /// This step's id.
    pub id: String,
    /// The reasoning itself.
    pub statement: String,
    /// The id this step argues toward — another step, or the top claim.
    pub supports: String,
    /// Non-empty, and its entries unique.
    pub sufficiency_criteria: Vec<String>,
}

/// Something the argument relies on without arguing for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assumption {
    /// This assumption's id.
    pub id: String,
    /// What is assumed.
    pub statement: String,
    /// Who owns it.
    pub owner: String,
    /// Its declared state.
    pub status: AssumptionStatus,
    /// When it must be revisited. An instant.
    pub review_by: String,
}

/// A named actor and the authority they hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    /// This participant's id, referenced by sufficiency decisions.
    pub id: String,
    /// Their role.
    pub role: String,
    /// What they may decide.
    pub authority: String,
    /// What they are independent of.
    pub independence: String,
}

/// An objection raised against a node of the argument.
///
/// # The two optional fields are not optional in the same way
///
/// This is the asymmetry `Option<T>` erases. The retained source reads them
/// through different mechanisms:
///
/// ```ignore
/// const expiresAt = optionalStringAt(row, "expires_at");
/// //  -> `if (!(key in object)) return undefined;` then a string check
/// const resolutionRefs = row.resolution_refs ? nonEmptyStrings(...) : undefined;
/// //  -> truthiness
/// ```
///
/// So `{"expires_at": null}` **throws** (`expires_at must be a string`, because
/// the key IS present), while `{"resolution_refs": null}` is silently treated
/// as absent. A derived `Option<String>` maps JSON `null` to `None` in both
/// places — correct for the second and **more permissive** than the retained
/// implementation for the first. Nothing about the two declarations says which
/// is which; only the reading mechanism does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    /// This challenge's id.
    pub id: String,
    /// The argument node it targets. Must resolve.
    pub target: String,
    /// The objection.
    pub statement: String,
    /// Its declared state.
    pub status: ChallengeStatus,
    /// Who owns it.
    pub owner: String,
    /// Evidence that it was answered. Emitted only when authored.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resolution_refs: Option<Vec<String>>,
    /// When an accepted risk stops being current. Emitted only when authored.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expires_at: Option<String>,
}

/// An edge to a document outside this argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationship {
    /// An `ix://` reference.
    pub target: String,
    /// How it reads.
    #[serde(rename = "type")]
    pub kind: RelationshipType,
}

/// A validated authored assurance argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssuranceArgument {
    /// `AA-<digits>`.
    pub id: String,
    /// The argument's title.
    pub title: String,
    /// Always `AssuranceArgument`; re-emitted so the output is a complete
    /// document rather than one that has to be reassembled by its reader.
    #[serde(rename = "type")]
    pub kind: ArgumentKind,
    /// Its lifecycle state.
    pub status: ArgumentStatus,
    /// Who owns the argument.
    pub owner: String,
    /// An `ix://` reference to the profile that governs it.
    pub profile: String,
    /// The claim being argued.
    pub top_claim: TopClaim,
    /// Non-empty. Every entry reaches [`AssuranceArgument::top_claim`].
    pub reasoning: Vec<Reasoning>,
    /// May be empty.
    pub assumptions: Vec<Assumption>,
    /// Non-empty.
    pub participants: Vec<Participant>,
    /// May be empty. Every entry targets a node that exists.
    pub challenges: Vec<Challenge>,
    /// May be empty.
    pub relationships: Vec<Relationship>,
}

/// The one-variant `type` discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArgumentKind {
    /// The only accepted value.
    AssuranceArgument,
}

/// The twelve keys the retained implementation admits at the top level.
const ALLOWED: [&str; 12] = [
    "id",
    "title",
    "type",
    "status",
    "owner",
    "profile",
    "top_claim",
    "reasoning",
    "assumptions",
    "participants",
    "challenges",
    "relationships",
];

/// Is this string empty once JavaScript's `trim` has run?
///
/// **Not `str::trim`.** The two disagree on exactly two code points, and they
/// disagree in OPPOSITE directions, which is why one call to `str::trim` would
/// have been wrong twice:
///
/// | code point | `String.prototype.trim` | `char::is_whitespace` | if ported naively |
/// |---|---|---|---|
/// | `U+FEFF` zero-width no-break space | trims | keeps | port **accepts** what the oracle refuses |
/// | `U+0085` next line | keeps | trims | port **refuses** what the oracle accepts |
///
/// ECMA-262 defines the trimmed set as `WhiteSpace` (TAB, VT, FF, SP, NBSP,
/// ZWNBSP and category `Zs`) plus `LineTerminator` (LF, CR, LS, PS). Unicode's
/// `White_Space` property, which Rust uses, includes `U+0085` and excludes
/// `U+FEFF`. So the set is transcribed here rather than borrowed.
fn js_trim_is_empty(value: &str) -> bool {
    value.chars().all(|c| {
        matches!(c,
            '\u{0009}'..='\u{000D}'
                | '\u{0020}'
                | '\u{00A0}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
        )
    })
}

/// One authored instant, validated exactly as the retained `instant()` does.
///
/// Two gates, and the split is the retained code's (quoin#436):
///
/// 1. The module's own shape regex, which is **stricter than RFC 3339 on
///    case** — an uppercase `T` and `Z` only.
/// 2. The ranges, which the retained code delegates to the shared strict
///    reader in `src/measurement/date-time.ts`.
///
/// Before quoin#436 the second gate was `Date.parse`, which does not reject an
/// impossible value — it ROLLS it, so `2026-02-30T00:00:00Z` became March 2
/// and the rolled number then decided whether an assumption was due for
/// review. That was fixed in the retained tree before this port was written,
/// precisely so this function is written against a rule rather than against a
/// rollover.
///
/// A leap second (`:60`) is refused, because both sides refuse it. No date
/// crate's default behaviour matches this combination, which is why it is
/// written out.
fn is_instant(value: &str) -> bool {
    // The shape is pure ASCII, so a multi-byte character can only be a
    // rejection — and establishing that up front makes every `get` below a
    // character boundary by construction rather than by argument.
    if !value.is_ascii() {
        return false;
    }
    let (Some(head), Some(rest)) = (value.get(..19), value.get(19..)) else {
        return false;
    };
    if head.get(4..5) != Some("-")
        || head.get(7..8) != Some("-")
        || head.get(10..11) != Some("T")
        || head.get(13..14) != Some(":")
        || head.get(16..17) != Some(":")
    {
        return false;
    }

    let mut fields = [0u32; 6];
    for (slot, (from, to)) in
        fields
            .iter_mut()
            .zip([(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)])
    {
        let Some(text) = head.get(from..to) else {
            return false;
        };
        if !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        let Ok(parsed) = text.parse::<u32>() else {
            return false;
        };
        *slot = parsed;
    }
    let [year, month, day, hour, minute, second] = fields;

    // `(?:\.\d+)?` — a dot with no digit after it is not a fraction.
    let mut tail = rest;
    if let Some(fraction) = tail.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        let Some(remainder) = fraction.get(digits..) else {
            return false;
        };
        if digits == 0 {
            return false;
        }
        tail = remainder;
    }

    // `Z` or `±HH:MM`, and nothing else. Lowercase `z` is refused: the shared
    // reader admits it and this module never has, so delegating the whole
    // check would have loosened the spelling while tightening the ranges.
    if tail != "Z" {
        let Some(offset) = tail.strip_prefix(['+', '-']) else {
            return false;
        };
        let (Some(offset_hour), Some(":"), Some(offset_minute)) =
            (offset.get(..2), offset.get(2..3), offset.get(3..))
        else {
            return false;
        };
        let (Ok(offset_hour), Ok(offset_minute)) =
            (offset_hour.parse::<u32>(), offset_minute.parse::<u32>())
        else {
            return false;
        };
        // `parse` admits a leading sign and whitespace; the digit check does
        // not, and the pattern is `\d{2}:\d{2}` exactly.
        if !offset
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b':')
            || offset.len() != 5
            || offset_hour > 23
            || offset_minute > 59
        {
            return false;
        }
    }

    (1..=12).contains(&month)
        && day >= 1
        && day <= days_in_month(year, month)
        && hour <= 23
        && minute <= 59
        && second <= 59
}

/// Days in a month, with the full Gregorian leap rule.
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

/// `record(name, value)`: an object, and not an array.
fn record<'a>(
    name: &str,
    value: Option<&'a serde_json::Value>,
) -> Checked<&'a serde_json::Map<String, serde_json::Value>> {
    match value.and_then(serde_json::Value::as_object) {
        Some(object) => Ok(object),
        None => reject(format!("{name} must be an object")),
    }
}

/// `arrayAt(object, key)`.
fn array_at<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Checked<&'a Vec<serde_json::Value>> {
    match object.get(key).and_then(serde_json::Value::as_array) {
        Some(array) => Ok(array),
        None => reject(format!("{key} must be an array")),
    }
}

/// `stringAt(object, key)`: a string, non-empty once trimmed.
fn string_at(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Checked<String> {
    let Some(value) = object.get(key).and_then(serde_json::Value::as_str) else {
        return reject(format!("{key} must be a string"));
    };
    if js_trim_is_empty(value) {
        return reject(format!("{key} must not be empty"));
    }
    Ok(value.to_owned())
}

/// `optionalStringAt(object, key)`.
///
/// The distinction that matters is `key in object`, not `value != null`: an
/// explicit `null` reaches [`string_at`] and is refused. See [`Challenge`].
fn optional_string_at(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Checked<Option<String>> {
    if object.contains_key(key) {
        string_at(object, key).map(Some)
    } else {
        Ok(None)
    }
}

/// `exactKeys(object, name, keys)`: no unknown key, and none missing.
fn exact_keys(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    keys: &[&str],
    optional: &[&str],
) -> Checked<()> {
    for key in object.keys() {
        if !keys.contains(&key.as_str()) {
            return reject(format!("{name} has unknown field {key}"));
        }
    }
    for key in keys {
        if !optional.contains(key) && !object.contains_key(*key) {
            return reject(format!("{name} is missing {key}"));
        }
    }
    Ok(())
}

/// `stringArray(name, value, requireValue)`: strings, unique, optionally non-empty.
fn string_array(
    name: &str,
    value: Option<&serde_json::Value>,
    require_value: bool,
) -> Checked<Vec<String>> {
    let Some(array) = value.and_then(serde_json::Value::as_array) else {
        return reject(format!(
            "{name} must be {} array",
            if require_value { "a non-empty" } else { "an" }
        ));
    };
    if require_value && array.is_empty() {
        return reject(format!("{name} must be a non-empty array"));
    }
    let mut strings = Vec::with_capacity(array.len());
    for item in array {
        let Some(text) = item.as_str() else {
            return reject(format!("{name} must contain strings"));
        };
        if js_trim_is_empty(text) {
            return reject(format!("{name} must not be empty"));
        }
        strings.push(text.to_owned());
    }
    let mut seen: Vec<&String> = Vec::with_capacity(strings.len());
    for text in &strings {
        if seen.contains(&text) {
            return reject(format!("{name} must contain unique values"));
        }
        seen.push(text);
    }
    Ok(strings)
}

/// `literal(value, name, allowed)`: membership, checked at run time.
fn literal<T: Copy>(
    value: Option<&serde_json::Value>,
    name: &str,
    allowed: &[(&str, T)],
) -> Checked<T> {
    if let Some(text) = value.and_then(serde_json::Value::as_str) {
        for (candidate, mapped) in allowed {
            if *candidate == text {
                return Ok(*mapped);
            }
        }
    }
    let names: Vec<&str> = allowed.iter().map(|(name, _)| *name).collect();
    reject(format!("{name} must be one of {}", names.join(", ")))
}

/// `uniqueIds(name, values)`.
fn unique_ids(name: &str, ids: &[&str]) -> Checked<()> {
    let mut seen: Vec<&str> = Vec::with_capacity(ids.len());
    for id in ids {
        if seen.contains(id) {
            return reject(format!("{name} contains duplicate id {id}"));
        }
        seen.push(id);
    }
    Ok(())
}

/// `/^AA-[0-9]+$/`, without a regex engine.
fn is_argument_id(id: &str) -> bool {
    let Some(digits) = id.strip_prefix("AA-") else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// Validate the module-owned `AssuranceArgument` frontmatter contract.
///
/// # Errors
///
/// [`ArgumentError`] for any of the seventeen predicates in the module
/// documentation. The message reproduces the retained implementation's;
/// the acceptance decision is what is contractual.
#[expect(
    clippy::too_many_lines,
    reason = "the retained parseAssuranceArgument is one function and splitting \
              it here would put the reading order somewhere other than the \
              order the oracle validates in, which is what a reader compares"
)]
pub fn parse_assurance_argument(value: &serde_json::Value) -> Checked<AssuranceArgument> {
    let object = record("argument", Some(value))?;
    for key in object.keys() {
        if !ALLOWED.contains(&key.as_str()) {
            return reject(format!("argument has unknown field {key}"));
        }
    }

    let id = string_at(object, "id")?;
    if !is_argument_id(&id) {
        return reject("argument id must match AA-<number>");
    }
    literal(
        object.get("type"),
        "type",
        &[("AssuranceArgument", ArgumentKind::AssuranceArgument)],
    )?;
    let status = literal(
        object.get("status"),
        "status",
        &[
            ("proposed", ArgumentStatus::Proposed),
            ("active", ArgumentStatus::Active),
            ("retired", ArgumentStatus::Retired),
        ],
    )?;
    let profile = string_at(object, "profile")?;
    if !profile.starts_with("ix://") {
        return reject("profile must be an ix:// reference");
    }

    let top = record("top_claim", object.get("top_claim"))?;
    exact_keys(top, "top_claim", &["id", "statement", "subject"], &[])?;

    let mut reasoning = Vec::new();
    for (index, item) in array_at(object, "reasoning")?.iter().enumerate() {
        let name = format!("reasoning[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(
            row,
            &name,
            &["id", "statement", "supports", "sufficiency_criteria"],
            &[],
        )?;
        reasoning.push(Reasoning {
            id: string_at(row, "id")?,
            statement: string_at(row, "statement")?,
            supports: string_at(row, "supports")?,
            sufficiency_criteria: string_array(
                &format!("{name}.sufficiency_criteria"),
                row.get("sufficiency_criteria"),
                true,
            )?,
        });
    }
    if reasoning.is_empty() {
        return reject("reasoning must not be empty");
    }

    let mut assumptions = Vec::new();
    for (index, item) in array_at(object, "assumptions")?.iter().enumerate() {
        let name = format!("assumptions[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(
            row,
            &name,
            &["id", "statement", "owner", "status", "review_by"],
            &[],
        )?;
        let review_by = string_at(row, "review_by")?;
        if !is_instant(&review_by) {
            return reject("review_by must be an ISO-8601 instant");
        }
        assumptions.push(Assumption {
            id: string_at(row, "id")?,
            statement: string_at(row, "statement")?,
            owner: string_at(row, "owner")?,
            status: literal(
                row.get("status"),
                "assumption status",
                &[
                    ("open", AssumptionStatus::Open),
                    ("accepted", AssumptionStatus::Accepted),
                    ("invalidated", AssumptionStatus::Invalidated),
                ],
            )?,
            review_by,
        });
    }

    let mut participants = Vec::new();
    for (index, item) in array_at(object, "participants")?.iter().enumerate() {
        let name = format!("participants[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(
            row,
            &name,
            &["id", "role", "authority", "independence"],
            &[],
        )?;
        participants.push(Participant {
            id: string_at(row, "id")?,
            role: string_at(row, "role")?,
            authority: string_at(row, "authority")?,
            independence: string_at(row, "independence")?,
        });
    }
    if participants.is_empty() {
        return reject("participants must not be empty");
    }

    let mut challenges = Vec::new();
    for (index, item) in array_at(object, "challenges")?.iter().enumerate() {
        let name = format!("challenges[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(
            row,
            &name,
            &[
                "id",
                "target",
                "statement",
                "status",
                "owner",
                "resolution_refs",
                "expires_at",
            ],
            &["resolution_refs", "expires_at"],
        )?;
        // `optionalStringAt` keys off PRESENCE, so an explicit null is a hard
        // error here...
        let expires_at = optional_string_at(row, "expires_at")?;
        if let Some(instant) = &expires_at
            && !is_instant(instant)
        {
            return reject("expires_at must be an ISO-8601 instant");
        }
        // ...while this one keys off TRUTHINESS, so an explicit null is
        // silently absent. The asymmetry is the retained implementation's and
        // is reproduced rather than tidied.
        let resolution_refs = match row.get("resolution_refs") {
            None | Some(serde_json::Value::Null) => None,
            Some(_) => Some(string_array(
                &format!("{name}.resolution_refs"),
                row.get("resolution_refs"),
                true,
            )?),
        };
        challenges.push(Challenge {
            id: string_at(row, "id")?,
            target: string_at(row, "target")?,
            statement: string_at(row, "statement")?,
            status: literal(
                row.get("status"),
                "challenge status",
                &[
                    ("open", ChallengeStatus::Open),
                    ("resolved", ChallengeStatus::Resolved),
                    ("accepted-risk", ChallengeStatus::AcceptedRisk),
                ],
            )?,
            owner: string_at(row, "owner")?,
            resolution_refs,
            expires_at,
        });
    }

    let mut relationships = Vec::new();
    for (index, item) in array_at(object, "relationships")?.iter().enumerate() {
        let name = format!("relationships[{index}]");
        let row = record(&name, Some(item))?;
        exact_keys(row, &name, &["target", "type"], &[])?;
        let target = string_at(row, "target")?;
        if !target.starts_with("ix://") {
            return reject(format!("{name}.target must be an ix:// reference"));
        }
        relationships.push(Relationship {
            target,
            kind: literal(
                row.get("type"),
                "relationship type",
                &[
                    ("supports", RelationshipType::Supports),
                    ("challenges", RelationshipType::Challenges),
                    ("references", RelationshipType::References),
                ],
            )?,
        });
    }

    unique_ids(
        "reasoning",
        &reasoning.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
    )?;
    unique_ids(
        "assumptions",
        &assumptions
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    unique_ids(
        "participants",
        &participants
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    unique_ids(
        "challenges",
        &challenges.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
    )?;

    let top_claim_id = string_at(top, "id")?;
    let mut node_ids: Vec<&str> = vec![top_claim_id.as_str()];
    node_ids.extend(reasoning.iter().map(|r| r.id.as_str()));
    node_ids.extend(assumptions.iter().map(|a| a.id.as_str()));
    {
        let mut seen: Vec<&str> = Vec::with_capacity(node_ids.len());
        for id in &node_ids {
            if seen.contains(id) {
                return reject("top claim, reasoning, and assumption ids must be unique");
            }
            seen.push(id);
        }
    }

    // Every chain of `supports` reaches the top claim. Cycle detection is per
    // START NODE, matching the retained `visited` set: a node reachable from
    // two starts is walked twice, which is correct — being visited on another
    // node's walk is not evidence that THIS node reaches the claim. Same
    // distinction FR-040 records for shared sub-claims (SR-007 FND-001).
    for start in &reasoning {
        let mut cursor = start;
        let mut visited: Vec<&str> = Vec::new();
        while cursor.supports != top_claim_id {
            if visited.contains(&cursor.id.as_str()) {
                return reject(format!(
                    "reasoning cycle does not reach top claim from {}",
                    start.id
                ));
            }
            visited.push(cursor.id.as_str());
            let Some(parent) = reasoning.iter().find(|r| r.id == cursor.supports) else {
                return reject(format!(
                    "reasoning {} supports unknown target {}",
                    start.id, cursor.supports
                ));
            };
            cursor = parent;
        }
    }

    for challenge in &challenges {
        if !node_ids.contains(&challenge.target.as_str()) {
            return reject(format!(
                "challenge {} targets unknown argument node {}",
                challenge.id, challenge.target
            ));
        }
    }

    Ok(AssuranceArgument {
        id,
        title: string_at(object, "title")?,
        kind: ArgumentKind::AssuranceArgument,
        status,
        owner: string_at(object, "owner")?,
        profile,
        top_claim: TopClaim {
            id: top_claim_id,
            statement: string_at(top, "statement")?,
            subject: string_at(top, "subject")?,
        },
        reasoning,
        assumptions,
        participants,
        challenges,
        relationships,
    })
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{is_instant, js_trim_is_empty, parse_assurance_argument};

    fn base() -> serde_json::Value {
        serde_json::json!({
            "id": "AA-900",
            "title": "Synthetic widget release decision",
            "type": "AssuranceArgument",
            "status": "active",
            "owner": "release-owner",
            "profile": "ix://example.invalid/widget/AP-900",
            "top_claim": {
                "id": "CLAIM-900",
                "statement": "The bounded synthetic widget change is acceptable.",
                "subject": "widget revision 0123456789abcdef"
            },
            "reasoning": [{
                "id": "ARG-900",
                "statement": "Argue from the explicitly reviewed clause disposition.",
                "supports": "CLAIM-900",
                "sufficiency_criteria": ["Every binding clause has a disposition."]
            }],
            "assumptions": [],
            "participants": [{
                "id": "reviewer-900",
                "role": "decision reviewer",
                "authority": "may accept or reject this synthetic release",
                "independence": "did not produce the implementation evidence"
            }],
            "challenges": [],
            "relationships": []
        })
    }

    #[test]
    fn accepts_the_minimal_authored_argument() {
        let parsed = parse_assurance_argument(&base()).expect("the minimal argument is valid");
        assert_eq!(parsed.id, "AA-900");
        assert_eq!(parsed.reasoning.len(), 1);
    }

    /// The two code points where `str::trim` would have been wrong, in
    /// opposite directions. See [`super::js_trim_is_empty`].
    #[test]
    fn js_trim_disagrees_with_rust_trim_in_both_directions() {
        // U+FEFF: JavaScript trims it, Rust keeps it. A naive port ACCEPTS an
        // owner the retained implementation refuses.
        assert!(js_trim_is_empty("\u{FEFF}"));
        assert!(!"\u{FEFF}".trim().is_empty());

        // U+0085: JavaScript keeps it, Rust trims it. A naive port REFUSES an
        // owner the retained implementation accepts.
        assert!(!js_trim_is_empty("\u{0085}"));
        assert!("\u{0085}".trim().is_empty());

        // Where they agree, they agree.
        assert!(js_trim_is_empty(" \t\n\u{00A0}\u{3000}"));
        assert!(!js_trim_is_empty("\u{200B}"));
    }

    #[test]
    fn an_owner_of_one_byte_order_mark_is_refused() {
        let mut argument = base();
        argument["owner"] = serde_json::json!("\u{FEFF}");
        assert!(parse_assurance_argument(&argument).is_err());
    }

    #[test]
    fn an_owner_of_one_next_line_is_accepted() {
        let mut argument = base();
        argument["owner"] = serde_json::json!("\u{0085}");
        assert!(
            parse_assurance_argument(&argument).is_ok(),
            "U+0085 is not whitespace to JavaScript, so the oracle accepts it"
        );
    }

    #[test]
    fn an_explicit_null_is_asymmetric_across_the_two_optional_fields() {
        let challenge = |extra: serde_json::Value| {
            let mut argument = base();
            let mut row = serde_json::json!({
                "id": "CH-900",
                "target": "CLAIM-900",
                "statement": "A bounded recovery case needed review.",
                "status": "open",
                "owner": "release-owner"
            });
            for (key, value) in extra.as_object().unwrap() {
                row[key] = value.clone();
            }
            argument["challenges"] = serde_json::json!([row]);
            parse_assurance_argument(&argument)
        };

        // `optionalStringAt` keys off `key in object`, so the key IS present
        // and reaches the string check.
        assert!(
            challenge(serde_json::json!({ "expires_at": null })).is_err(),
            "an explicit null expires_at must be refused"
        );
        // Truthiness, so null is indistinguishable from absent.
        let parsed = challenge(serde_json::json!({ "resolution_refs": null }))
            .expect("an explicit null resolution_refs is silently absent");
        assert_eq!(parsed.challenges[0].resolution_refs, None);
    }

    #[test]
    fn instants_that_name_a_day_or_hour_that_does_not_exist_are_refused() {
        // quoin#436: `Date.parse` rolled these rather than rejecting them.
        assert!(!is_instant("2026-02-30T00:00:00Z"));
        assert!(!is_instant("2026-06-31T00:00:00Z"));
        assert!(!is_instant("2025-02-29T00:00:00Z"));
        assert!(!is_instant("2026-08-15T24:00:00Z"));
        // A leap second, which the retained code refuses and chrono accepts.
        assert!(!is_instant("2026-08-15T23:59:60Z"));
        // An offset out of range.
        assert!(!is_instant("2026-08-15T00:00:00+99:99"));
        // Lowercase, which this module has never accepted.
        assert!(!is_instant("2026-08-15t00:00:00Z"));
        assert!(!is_instant("2026-08-15T00:00:00z"));
        // A dot with no fraction after it.
        assert!(!is_instant("2026-08-15T00:00:00.Z"));

        // Accepted: a real leap day, a fraction, and an in-range offset.
        assert!(is_instant("2028-02-29T00:00:00Z"));
        assert!(is_instant("2026-08-15T00:00:00.000Z"));
        assert!(is_instant("2026-08-15T00:00:00+05:30"));
        assert!(is_instant("2026-08-15T23:59:59-11:00"));
    }

    #[test]
    fn a_cycle_is_detected_per_start_node_rather_than_globally() {
        let mut argument = base();
        argument["reasoning"] = serde_json::json!([
            {"id": "A", "statement": "a", "supports": "B", "sufficiency_criteria": ["c"]},
            {"id": "B", "statement": "b", "supports": "A", "sufficiency_criteria": ["c"]}
        ]);
        assert!(parse_assurance_argument(&argument).is_err());
    }

    #[test]
    fn a_shared_intermediate_step_is_walked_from_both_starts() {
        // Neither start is a cycle, and a GLOBAL visited set would have made
        // the second walk look like one.
        let mut argument = base();
        argument["reasoning"] = serde_json::json!([
            {"id": "A", "statement": "a", "supports": "M", "sufficiency_criteria": ["c"]},
            {"id": "B", "statement": "b", "supports": "M", "sufficiency_criteria": ["c"]},
            {"id": "M", "statement": "m", "supports": "CLAIM-900", "sufficiency_criteria": ["c"]}
        ]);
        assert!(parse_assurance_argument(&argument).is_ok());
    }

    #[test]
    fn an_unlisted_status_is_refused_because_the_retained_code_checks() {
        // The other half of quoin#425. `Finding::kind` stays a String because
        // the renderer interpolates; this is checked, so it is closed.
        let mut argument = base();
        argument["status"] = serde_json::json!("archived");
        assert!(parse_assurance_argument(&argument).is_err());
    }
}
