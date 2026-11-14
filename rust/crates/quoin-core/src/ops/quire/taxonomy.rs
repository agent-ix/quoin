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

use quoin_auditor::AuditorErrorCode;
use quoin_quire::ErrorCode;

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-quire` failure onto the boundary's exit taxonomy.
///
/// The split is "who can act on it":
///
/// - **`Refused` (2)** — the request was understood and the world declined it.
///   A scope that is not a directory, a repository with no `spec/`, a module
///   set that does not load or resolves to nothing, no declared traceability
///   model, an unknown archetype, a clause set that is not loaded: fixing any
///   of them means changing the repository, not the request.
/// - **`BadRequest` (3)** — the caller's own spelling. A malformed context
///   entry, a duplicated context key, a payload that is not JSON or not the declared
///   shape.
/// - **`Io` → Internal (4)** — a filesystem call failed under us, a payload
///   outgrew its ceiling, or quoin's own vendored `assurance-v1` schema will
///   not compile. A retained-run audit's unreachable serialization failure is
///   also internal. No caller can act on any of these.
/// - **Retained-run audit failures** — the wrapped auditor code decides who can
///   act: malformed or uncountable input is `Refused`; serialization failure
///   is `Io`.
///
/// `ErrorCode` is `#[non_exhaustive]`, so this match needs a `_` arm and the
/// compiler will NOT flag a code added upstream.
/// `the_error_mapping_covers_every_quire_code` pins `ErrorCode::all().len()`,
/// which a loop over `all()` could not do — such a loop re-runs this same match
/// and agrees with itself (the quoin#443 failure mode).
pub(super) fn map_error(error: &quoin_quire::Error, op: &'static str) -> CoreError {
    let code = error.code();
    let mapped = match error {
        quoin_quire::Error::AuditFailed {
            code: auditor_code, ..
        } => core_code(code, Some(*auditor_code)),
        _ => core_code(code, None),
    };
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
const fn core_code(
    code: ErrorCode,
    auditor_code: Option<AuditorErrorCode>,
) -> Option<CoreErrorCode> {
    Some(match code {
        ErrorCode::ScopeNotADirectory
        | ErrorCode::DocumentRootMissing
        | ErrorCode::ModuleLoad
        | ErrorCode::ModuleSetEmpty
        | ErrorCode::TraceabilityModelUndeclared
        | ErrorCode::ExtractionUnavailable
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
        | ErrorCode::PayloadNotJson
        | ErrorCode::PayloadShape => CoreErrorCode::BadRequest,

        ErrorCode::Io
        | ErrorCode::PayloadTooLarge
        | ErrorCode::Assurance
        | ErrorCode::VendoredSchemaInvalid => CoreErrorCode::Io,

        ErrorCode::AuditFailed => match auditor_code {
            Some(
                AuditorErrorCode::DemandTooLarge
                | AuditorErrorCode::ManifestUnreadable
                | AuditorErrorCode::ClassPolicyRefused,
            ) => CoreErrorCode::Refused,
            Some(AuditorErrorCode::ReportNotSerialisable) => CoreErrorCode::Io,
            // The wrapped code is also non-exhaustive. An unknown inner code
            // stays unmapped instead of inheriting a known refusal category.
            _ => return None,
        },

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
    use quoin_auditor::AuditorErrorCode;
    use quoin_quire::ErrorCode;

    use super::{CoreErrorCode, core_code, map_error};

    /// [`ErrorCode`] is `#[non_exhaustive]`, so `map_error`'s `_` arm means the
    /// compiler cannot catch a code added upstream. This lists every group by
    /// name and requires each code in `all()` to be in exactly one.
    ///
    /// When this fails: a code was added to `quoin-quire`. Decide which group
    /// it belongs to, or delegate it to the wrapped auditor code.
    #[test]
    fn the_error_mapping_covers_every_quire_code() {
        let refused = [
            ErrorCode::ScopeNotADirectory,
            ErrorCode::DocumentRootMissing,
            ErrorCode::ModuleLoad,
            ErrorCode::ModuleSetEmpty,
            ErrorCode::TraceabilityModelUndeclared,
            ErrorCode::ExtractionUnavailable,
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
            ErrorCode::PayloadNotJson,
            ErrorCode::PayloadShape,
        ];
        let internal = [
            ErrorCode::Io,
            ErrorCode::PayloadTooLarge,
            ErrorCode::Assurance,
            ErrorCode::VendoredSchemaInvalid,
        ];
        let delegated = [ErrorCode::AuditFailed];
        assert_eq!(
            refused.len() + bad_request.len() + internal.len() + delegated.len(),
            ErrorCode::all().len(),
            "a code is in `all()` but in none of the mapping groups"
        );

        for code in refused {
            assert_eq!(
                core_code(code, None),
                Some(CoreErrorCode::Refused),
                "{code:?}"
            );
        }
        for code in bad_request {
            assert_eq!(
                core_code(code, None),
                Some(CoreErrorCode::BadRequest),
                "{code:?}"
            );
        }
        for code in internal {
            assert_eq!(core_code(code, None), Some(CoreErrorCode::Io), "{code:?}");
        }
        for code in delegated {
            for auditor_code in [
                AuditorErrorCode::DemandTooLarge,
                AuditorErrorCode::ManifestUnreadable,
                AuditorErrorCode::ClassPolicyRefused,
            ] {
                assert_eq!(
                    core_code(code, Some(auditor_code)),
                    Some(CoreErrorCode::Refused),
                    "{auditor_code:?}"
                );
            }
            assert_eq!(
                core_code(code, Some(AuditorErrorCode::ReportNotSerialisable)),
                Some(CoreErrorCode::Io)
            );
            assert_eq!(core_code(code, None), None);
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

    #[test]
    fn a_retained_audit_failure_uses_the_wrapped_auditor_code() {
        let refused = map_error(
            &quoin_quire::Error::AuditFailed {
                code: AuditorErrorCode::ClassPolicyRefused,
                reason: "policy names an unknown class".to_owned(),
            },
            "quire.coverage",
        );
        assert_eq!(refused.code, CoreErrorCode::Refused);
        assert_eq!(refused.outcome().code(), 2);
        assert_eq!(refused.context["quire_code"], "QQ-1090");

        let internal = map_error(
            &quoin_quire::Error::AuditFailed {
                code: AuditorErrorCode::ReportNotSerialisable,
                reason: "audit report could not be serialised".to_owned(),
            },
            "quire.coverage",
        );
        assert_eq!(internal.code, CoreErrorCode::Io);
        assert_eq!(internal.outcome().code(), 4);
        assert_eq!(internal.context["quire_code"], "QQ-1090");
    }
}
