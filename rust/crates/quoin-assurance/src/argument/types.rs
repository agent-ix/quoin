// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The declared shape of an authored assurance argument, and the one
//! rejection type the whole module reports through.
//!
//! Types only: every predicate that decides whether a document may become
//! one of these lives in [`super::parse`] (quoin#384).

use serde::{Deserialize, Serialize};

/// A rejection, carrying the retained implementation's own message.
///
/// The message is reproduced because it costs nothing and helps a human, but
/// it is deliberately **not** a contract: the native fixture suite compares the
/// diagnostic's `(code, context keys)` and never its prose (quoin#373).
///
/// # Why one variant and not seventeen
///
/// rust-review §0b warns that an error whose payload is a string, built from a
/// different literal at each site, has moved the discriminant into prose. The
/// test it gives is whether a caller must distinguish the conditions. Here no
/// caller can: every one of the seventeen predicates this module checks leaves
/// the boundary as the single code `CORE_BAD_REQUEST`, because that is what
/// the retained `parseAssuranceArgument` does — it throws one `Error` — and
/// FR-101 parity is the whole point of the port. Seventeen typed variants
/// would be public API that nothing branches on (§3), and the first caller
/// that wanted one would be asking for a distinction the oracle does not make.
/// There is one discriminant, and it is spelled `Err`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentError(pub String);

impl std::fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ArgumentError {}

pub(crate) type Checked<T> = Result<T, ArgumentError>;

pub(crate) fn reject<T>(message: impl Into<String>) -> Checked<T> {
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
// Reachable as a REQUEST: `AuthoredArgumentView` carries participants through
// unchanged, and `assurance.render_authored_argument` reads that view off
// untrusted stdin. A field the boundary silently drops is a field the caller
// believes it sent (rust-style, untrusted input).
#[serde(deny_unknown_fields)]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
// Reachable as a request for the same reason [`Participant`] is.
#[serde(deny_unknown_fields)]
pub struct Relationship {
    /// An `ix://` reference.
    pub target: String,
    /// How it reads.
    #[serde(rename = "type")]
    pub kind: RelationshipType,
}

/// A validated authored assurance argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ArgumentKind {
    /// The only accepted value.
    AssuranceArgument,
}
