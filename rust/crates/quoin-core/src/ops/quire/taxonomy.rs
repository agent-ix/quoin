// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One `quoin_quire::ErrorCode`, one exit status.
//!
//! Split from the operations in [`super`] for the reason `ops::semantic`'s
//! taxonomy gives: the operations decide *what to do*, this decides *how a
//! failure is reported*, and a mapping alone in a file can be asserted against
//! the error catalogue rather than against whichever variants a test double can
//! construct.
//!
//! # What the retained TypeScript did, and why this is not the same shape
//!
//! `runQuire` had one failure channel: a non-zero exit, whose stderr it pasted
//! into an `Error`. An operator got "quire exited 1" and a sentence. quoin#106
//! is that failure — the engine said "no module declares a traceability model"
//! and the sentence was discarded on the way through. Here the condition is a
//! variant with a stable code, and the code rides on the envelope.

use quoin_quire::ErrorCode;

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-quire` failure onto the boundary's exit taxonomy.
///
/// Three groups, and the split is "who can act on it":
///
/// - **`Refused` (2)** — the request was understood and the world declined it.
///   A scope that is not a directory, a repository with no `spec/`, a module
///   set that does not load or resolves to nothing, no declared traceability
///   model, an unknown archetype, a clause set that is not loaded: fixing any
///   of them means changing the repository, not the request.
/// - **`BadRequest` (3)** — the caller's own spelling. A malformed context
///   entry, a duplicated context key, an empty repository identity, a revision
///   that is not 40 hex digits, a payload that is not JSON or not the declared
///   shape.
/// - **`Io` → Internal (4)** — a filesystem call failed under us, a payload
///   outgrew its ceiling, or quoin's own vendored `assurance-v1` schema will
///   not compile. No caller can act on any of these.
///
/// `ErrorCode` is `#[non_exhaustive]`, so this match needs a `_` arm and the
/// compiler will NOT flag a code added upstream.
/// `the_error_mapping_covers_every_quire_code` pins `ErrorCode::all().len()`,
/// which a loop over `all()` could not do — such a loop re-runs this same match
/// and agrees with itself (the quoin#443 failure mode).
pub(super) fn map_error(error: &quoin_quire::Error, op: &'static str) -> CoreError {
    let code = error.code();
    let mapped = core_code(code);
    let envelope = CoreError::new(mapped.unwrap_or(CoreErrorCode::Io), error.to_string())
        .with_context("op", op)
        .with_context("quire_code", code.as_str());
    match mapped {
        Some(_) => envelope,
        // A code this build has no opinion on. It exits Internal (4), and it
        // SAYS it was unmapped rather than posing as a considered answer.
        None => envelope.with_context("mapping", "unrecognised"),
    }
}

/// The exit-taxonomy code one [`ErrorCode`] maps to, or `None` for a code this
/// build does not know.
///
/// `Option` rather than a total function, because [`ErrorCode`] is
/// `#[non_exhaustive]`: "unknown upstream code" and "deliberately Internal" are
/// different facts, and collapsing them would make the wildcard arm
/// indistinguishable from a considered decision.
const fn core_code(code: ErrorCode) -> Option<CoreErrorCode> {
    Some(match code {
        ErrorCode::ScopeNotADirectory
        | ErrorCode::DocumentRootMissing
        | ErrorCode::ModuleLoad
        | ErrorCode::ModuleSetEmpty
        | ErrorCode::TraceabilityModelUndeclared
        | ErrorCode::ArchetypeUndeclared
        | ErrorCode::ArchetypeUnknown
        | ErrorCode::ClauseSetNotLoaded
        | ErrorCode::ClauseSetNotComparable
        | ErrorCode::AssuranceContract
        | ErrorCode::EnginePremise
        | ErrorCode::EngineRevisionMismatch => CoreErrorCode::Refused,

        ErrorCode::PathEscapesRoot
        | ErrorCode::ContextEntryMalformed
        | ErrorCode::ContextKeyDuplicated
        | ErrorCode::RepositoryEmpty
        | ErrorCode::RevisionMalformed
        | ErrorCode::PayloadNotJson
        | ErrorCode::PayloadShape => CoreErrorCode::BadRequest,

        ErrorCode::Io
        | ErrorCode::PayloadTooLarge
        | ErrorCode::Assurance
        | ErrorCode::VendoredSchemaInvalid => CoreErrorCode::Io,

        // Required by `#[non_exhaustive]`. A code this build has never seen is
        // reported as unmapped and exits Internal — the safest answer, since an
        // unknown rule is not one the caller can act on. The count pin below is
        // what makes reaching this arm a test failure rather than a silent
        // widening.
        _ => return None,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use quoin_quire::ErrorCode;

    use super::{CoreErrorCode, core_code, map_error};

    /// [`ErrorCode`] is `#[non_exhaustive]`, so `map_error`'s `_` arm means the
    /// compiler cannot catch a code added upstream. This pins the COUNT
    /// instead, and lists every group by name.
    ///
    /// When this fails: a code was added to `quoin-quire`. Decide which of the
    /// three groups it belongs to, add it to that arm, and raise the count.
    #[test]
    fn the_error_mapping_covers_every_quire_code() {
        assert_eq!(
            ErrorCode::all().len(),
            23,
            "quoin-quire gained or lost an error code; map it in map_error deliberately"
        );

        let refused = [
            ErrorCode::ScopeNotADirectory,
            ErrorCode::DocumentRootMissing,
            ErrorCode::ModuleLoad,
            ErrorCode::ModuleSetEmpty,
            ErrorCode::TraceabilityModelUndeclared,
            ErrorCode::ArchetypeUndeclared,
            ErrorCode::ArchetypeUnknown,
            ErrorCode::ClauseSetNotLoaded,
            ErrorCode::ClauseSetNotComparable,
            ErrorCode::AssuranceContract,
            ErrorCode::EnginePremise,
            ErrorCode::EngineRevisionMismatch,
        ];
        let bad_request = [
            ErrorCode::PathEscapesRoot,
            ErrorCode::ContextEntryMalformed,
            ErrorCode::ContextKeyDuplicated,
            ErrorCode::RepositoryEmpty,
            ErrorCode::RevisionMalformed,
            ErrorCode::PayloadNotJson,
            ErrorCode::PayloadShape,
        ];
        let internal = [
            ErrorCode::Io,
            ErrorCode::PayloadTooLarge,
            ErrorCode::Assurance,
            ErrorCode::VendoredSchemaInvalid,
        ];
        assert_eq!(
            refused.len() + bad_request.len() + internal.len(),
            ErrorCode::all().len(),
            "a code is in `all()` but in none of the three groups"
        );

        for code in refused {
            assert_eq!(core_code(code), Some(CoreErrorCode::Refused), "{code:?}");
        }
        for code in bad_request {
            assert_eq!(core_code(code), Some(CoreErrorCode::BadRequest), "{code:?}");
        }
        for code in internal {
            assert_eq!(core_code(code), Some(CoreErrorCode::Io), "{code:?}");
        }
    }

    /// The envelope carries the engine's code through, so an operator can tell
    /// which rule declined without parsing the sentence — which is the whole
    /// of what "quire exited 1" could not say (quoin#106).
    #[test]
    fn the_envelope_names_the_quire_code_that_declined() {
        let envelope = map_error(
            &quoin_quire::Error::TraceabilityModelUndeclared,
            "quire.coverage",
        );
        assert_eq!(envelope.context["quire_code"], "QQ-1020");
        assert_eq!(envelope.code, CoreErrorCode::Refused);
        assert_eq!(envelope.outcome().code(), 2);
    }
}
