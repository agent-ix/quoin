// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One `ModulesError`, one exit status.
//!
//! The single place a `quoin-modules` failure becomes a number on the
//! boundary's exit taxonomy. Split from the operations in [`super`] because it
//! is a different job: the operations decide *what to do*, this decides *how a
//! failure is reported*, and keeping the mapping alone in a file is what lets
//! it be asserted against the error catalogue itself rather than against
//! whichever variants a test double happens to be able to construct.

use quoin_modules::{ModulesError, ModulesErrorCode};

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-modules` failure onto the boundary's exit taxonomy.
///
/// The mapping is deliberate and total, and it is the ONE place a
/// `ModulesError` becomes an exit status. Three groups:
///
/// - **`BadRequest` (3)** — the caller's own words were wrong: an unparsable
///   source argument, an unknown source type, an invalid module name, a
///   malformed manifest. Fixing it means editing the request.
/// - **`Refused` (2)** — the request was understood and a stated rule declined
///   it: a source that does not exist, a module that is not installed, a
///   resource bound exceeded, a tree path that escapes, a semantic-contract
///   violation. Fixing it means changing the world, not the request.
/// - **`Io` → Internal (4)** — quoin's own machinery or the network failed.
///
/// `ModulesErrorCode` is `#[non_exhaustive]`, so this match needs a `_` arm and
/// the compiler will NOT flag a code added upstream. `the_error_mapping_covers_
/// every_modules_code` pins `ModulesErrorCode::all().len()`, which a loop over
/// `all()` could not do — such a loop re-runs this same match and agrees with
/// itself (the quoin#443 failure mode).
pub(super) fn map_error(error: &ModulesError, op: &'static str) -> CoreError {
    let code = error.code();
    let mapped = core_code(code);
    let envelope = CoreError::new(mapped.unwrap_or(CoreErrorCode::Io), error.to_string())
        .with_context("op", op)
        .with_context("modules_code", code.as_str());
    match mapped {
        Some(_) => envelope,
        // A code this build has no opinion on. It exits Internal (4), and it
        // SAYS it was unmapped rather than posing as a considered answer.
        None => envelope.with_context("mapping", "unrecognised"),
    }
}

/// The exit-taxonomy code one `ModulesErrorCode` maps to, or `None` for a code
/// this build does not know.
///
/// `Option` rather than a total function, because `ModulesErrorCode` is
/// `#[non_exhaustive]`: "unknown upstream code" and "deliberately Internal" are
/// different facts, and collapsing them would make the wildcard arm
/// indistinguishable from a considered decision — to a reader and to the
/// linter both.
///
/// Split out from [`map_error`] so the mapping can be asserted over the code
/// catalogue itself rather than over whichever error variants a test double
/// happens to be able to construct.
const fn core_code(code: ModulesErrorCode) -> Option<CoreErrorCode> {
    Some(match code {
        ModulesErrorCode::InvalidSource
        | ModulesErrorCode::UnsupportedSource
        | ModulesErrorCode::UnparsableSourceArg
        | ModulesErrorCode::InvalidManifest
        | ModulesErrorCode::InvalidModuleName => CoreErrorCode::BadRequest,

        ModulesErrorCode::PathSourceNotFound
        | ModulesErrorCode::ManifestNotFound
        | ModulesErrorCode::ManifestHasNoName
        | ModulesErrorCode::ManifestUnreadable
        | ModulesErrorCode::ModuleNotInstalled
        | ModulesErrorCode::GitRevisionNotFound
        | ModulesErrorCode::ResourceBoundExceeded
        | ModulesErrorCode::UnsafeTreePath
        | ModulesErrorCode::SemanticContractViolation
        | ModulesErrorCode::SemanticContractUnavailable => CoreErrorCode::Refused,

        ModulesErrorCode::RegistryUnreadable
        | ModulesErrorCode::RegistryUnwritable
        | ModulesErrorCode::GitTransport
        | ModulesErrorCode::GitObjectStore
        | ModulesErrorCode::GitTimeout
        | ModulesErrorCode::FetchDeadlineUnavailable
        | ModulesErrorCode::MaterializeFailed => CoreErrorCode::Io,

        // Required by `#[non_exhaustive]`. A code this build has never seen is
        // reported as unmapped and exits Internal — the safest answer, since an
        // unknown rule is not one the caller can act on. The count pin in
        // `the_error_mapping_covers_every_modules_code` is what makes reaching
        // this arm a test failure rather than a silent widening.
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
    use quoin_modules::ModulesErrorCode;

    use super::{CoreErrorCode, core_code, map_error};
    use crate::ops::modules::support::error_with_code;

    /// `ModulesErrorCode` is `#[non_exhaustive]`, so `map_error`'s `_` arm
    /// means the compiler cannot catch a code added upstream. This pins the
    /// COUNT instead. A loop over `all()` would only re-run the same match and
    /// agree with itself — the quoin#443 failure mode — so the number is
    /// written out here, and every group is listed by name.
    ///
    /// When this fails: a code was added to `quoin-modules`. Decide which of
    /// the three groups it belongs to, add it to that arm, and raise the count.
    #[test]
    fn the_error_mapping_covers_every_modules_code() {
        assert_eq!(
            ModulesErrorCode::all().len(),
            22,
            "quoin-modules gained or lost an error code; map it in map_error deliberately"
        );

        let bad_request = [
            ModulesErrorCode::InvalidSource,
            ModulesErrorCode::UnsupportedSource,
            ModulesErrorCode::UnparsableSourceArg,
            ModulesErrorCode::InvalidManifest,
            ModulesErrorCode::InvalidModuleName,
        ];
        let refused = [
            ModulesErrorCode::PathSourceNotFound,
            ModulesErrorCode::ManifestNotFound,
            ModulesErrorCode::ManifestHasNoName,
            ModulesErrorCode::ManifestUnreadable,
            ModulesErrorCode::ModuleNotInstalled,
            ModulesErrorCode::GitRevisionNotFound,
            ModulesErrorCode::ResourceBoundExceeded,
            ModulesErrorCode::UnsafeTreePath,
            ModulesErrorCode::SemanticContractViolation,
            ModulesErrorCode::SemanticContractUnavailable,
        ];
        let internal = [
            ModulesErrorCode::RegistryUnreadable,
            ModulesErrorCode::RegistryUnwritable,
            ModulesErrorCode::GitTransport,
            ModulesErrorCode::GitObjectStore,
            ModulesErrorCode::GitTimeout,
            ModulesErrorCode::FetchDeadlineUnavailable,
            ModulesErrorCode::MaterializeFailed,
        ];
        assert_eq!(
            bad_request.len() + refused.len() + internal.len(),
            ModulesErrorCode::all().len(),
            "a code is in `all()` but in none of the three groups"
        );

        for code in bad_request {
            assert_eq!(core_code(code), Some(CoreErrorCode::BadRequest), "{code}");
        }
        for code in refused {
            assert_eq!(core_code(code), Some(CoreErrorCode::Refused), "{code}");
        }
        for code in internal {
            assert_eq!(core_code(code), Some(CoreErrorCode::Io), "{code}");
        }

        // And the envelope carries the module code through, so an operator can
        // tell which rule declined without parsing the sentence.
        let envelope = map_error(
            &error_with_code(ModulesErrorCode::SemanticContractViolation),
            "modules.install",
        );
        assert_eq!(
            envelope.context["modules_code"],
            "QM020_SEMANTIC_CONTRACT_VIOLATION"
        );
    }
}
