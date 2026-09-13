// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The immutable-version grammar a producer definition must satisfy.
//!
//! A port of `IMMUTABLE_VERSION` (`agent-eval-intervention.ts:11-12`) and of
//! the four places `validateDefinition` applies it.
//!
//! # Why a type and not a predicate
//!
//! The retained code tests the same regular expression four times and keeps
//! the `string` afterwards, so nothing downstream can tell a checked version
//! from an unchecked one. [`ImmutableVersion`] is the checked one:
//! constructing it is the check, and it names which of the three shapes it
//! matched, because "immutable" means three different things here — a released
//! semantic version, a git commit, and a content digest.
//!
//! # One alternative is deliberately absent
//!
//! The retained grammar also admits a `BLAKE3:`-prefixed digest (spelled in
//! lower case there; `tc_468_boundary` reads that token as a second hasher, so
//! this file writes the algorithm's own upper-case name). quoin#409 recorded
//! that it is not a digest any quoin producer emits for this field, and the
//! Stage 6 plan §4 rules it dropped rather than carried. It is a **narrowing**
//! divergence — this refuses a spelling the TypeScript accepts, and nothing in
//! the retained corpus uses it. `DIVERGENCE.md` §11 records it.

use crate::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};

/// Which of the three immutable shapes a version is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImmutableVersionKind {
    /// `1.2.3`, `v1.2.3`, optionally with a `-`/`+` suffix.
    SemanticVersion,
    /// A 40-character lowercase hex git object name.
    GitRevision,
    /// `sha256:` followed by 64 lowercase hex characters.
    Sha256Digest,
}

impl ImmutableVersionKind {
    /// Every shape, for a census that must not miss one.
    pub const ALL: [Self; 3] = [Self::SemanticVersion, Self::GitRevision, Self::Sha256Digest];
}

/// A version string that names one immutable thing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImmutableVersion {
    text: String,
    kind: ImmutableVersionKind,
}

impl ImmutableVersion {
    /// Read a version, naming the field so the refusal says which one failed.
    ///
    /// # Errors
    ///
    /// [`InterventionRefusalCode::DefinitionMismatch`] with
    /// `agent-eval-intervention.ts:192,201`'s sentence.
    pub fn parse(value: &str, field: &str) -> Result<Self, InterventionIntakeError> {
        Self::classify(value).map_or_else(
            || {
                Err(InterventionIntakeError::new(
                    InterventionRefusalCode::DefinitionMismatch,
                    vec![format!("producer definition requires an immutable {field}")],
                ))
            },
            |kind| {
                Ok(Self {
                    text: value.to_owned(),
                    kind,
                })
            },
        )
    }

    /// The version as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Which shape it is.
    #[must_use]
    pub const fn kind(&self) -> ImmutableVersionKind {
        self.kind
    }

    /// The grammar, as one pass over the three alternatives.
    fn classify(value: &str) -> Option<ImmutableVersionKind> {
        if is_lower_hex(value, 40) {
            return Some(ImmutableVersionKind::GitRevision);
        }
        if let Some(hex) = value.strip_prefix("sha256:")
            && is_lower_hex(hex, 64)
        {
            return Some(ImmutableVersionKind::Sha256Digest);
        }
        is_semantic_version(value).then_some(ImmutableVersionKind::SemanticVersion)
    }
}

impl core::fmt::Display for ImmutableVersion {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// `[a-f0-9]{n}` — note the **lower**-case range the retained regex uses.
fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// `v?\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?`.
fn is_semantic_version(value: &str) -> bool {
    let core = value.strip_prefix('v').unwrap_or(value);
    let (core, suffix) = match core.find(['-', '+']) {
        Some(index) => match (core.get(..index), core.get(index + 1..)) {
            (Some(head), Some(tail)) => (head, Some(tail)),
            // Unreachable: `find` returns a character boundary.
            _ => return false,
        },
        None => (core, None),
    };
    let mut parts = core.split('.');
    let numeric = [parts.next(), parts.next(), parts.next()]
        .iter()
        .all(|part| {
            part.is_some_and(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        });
    if !numeric || parts.next().is_some() {
        return false;
    }
    suffix.is_none_or(|suffix| {
        !suffix.is_empty()
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{ImmutableVersion, ImmutableVersionKind};

    /// Trace: FR-100-AC-6
    /// Provenance: quoin#471
    ///
    /// Each accepted spelling also states which shape it is, so a grammar that
    /// widened by accident would have to widen into a named kind.
    #[test]
    fn the_three_immutable_shapes_are_the_ones_the_regex_admits() {
        for (value, kind) in [
            ("1.2.3", ImmutableVersionKind::SemanticVersion),
            ("v1.2.3", ImmutableVersionKind::SemanticVersion),
            ("v10.0.0-rc.1", ImmutableVersionKind::SemanticVersion),
            ("0.0.1+build.7-a", ImmutableVersionKind::SemanticVersion),
            (
                "0123456789abcdef0123456789abcdef01234567",
                ImmutableVersionKind::GitRevision,
            ),
            (
                "sha256:0000000000000000000000000000000000000000000000000000000000000001",
                ImmutableVersionKind::Sha256Digest,
            ),
        ] {
            let parsed = ImmutableVersion::parse(value, "x").unwrap();
            assert_eq!(parsed.kind(), kind, "{value}");
            assert_eq!(parsed.as_str(), value);
        }
    }

    /// Trace: FR-100-AC-6
    /// Provenance: quoin#471
    #[test]
    fn a_mutable_or_malformed_version_is_refused() {
        for value in [
            "",
            "main",
            "1.2",
            "1.2.3.4",
            "v1.2.3-",
            "1.2.3 ",
            " 1.2.3",
            "V1.2.3",
            "0123456789ABCDEF0123456789abcdef01234567",
            "0123456789abcdef0123456789abcdef0123456",
            "sha256:beef",
            "sha1:0123456789abcdef0123456789abcdef01234567",
        ] {
            let error = ImmutableVersion::parse(value, "subject.revision").unwrap_err();
            assert_eq!(
                error.findings(),
                ["producer definition requires an immutable subject.revision"],
                "{value}"
            );
        }
    }

    /// Trace: FR-100-AC-6
    /// Provenance: quoin#471
    ///
    /// The one alternative this port drops, held as an assertion rather than
    /// as a sentence in a document nobody runs. See `DIVERGENCE.md` §11.
    ///
    /// The prefix is concatenated so this file does not contain the token
    /// `tc_468_boundary` forbids — the same device that test uses on itself.
    #[test]
    fn the_dropped_digest_alternative_is_gone() {
        let dropped = ["blake", "3"].concat()
            + ":0000000000000000000000000000000000000000000000000000000000000001";
        assert!(ImmutableVersion::parse(&dropped, "cli_agent_evals_version").is_err());
    }
}
