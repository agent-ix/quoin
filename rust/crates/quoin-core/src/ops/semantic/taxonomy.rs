// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One `SemanticError`, one exit status.
//!
//! The single place a `quoin-semantic` failure becomes a number on the
//! boundary's exit taxonomy. Split from the operations in [`super`] because it
//! is a different job: the operations decide *what to do*, this decides *how a
//! failure is reported*, and keeping the mapping alone in a file is what lets
//! it be asserted against the error catalogue itself rather than against
//! whichever variants a test double happens to be able to construct.
//!
//! A diagnostic is NOT an error here and never reaches this file. A module
//! whose `semantic` block violates the contract is read successfully and
//! reported as a [`quoin_semantic::SemanticDiagnostic`] in the payload, exactly
//! as `readSemanticBlock` did — the caller prints it. Only a failure to READ
//! an input arrives here.

use quoin_semantic::{SemanticError, SemanticErrorCode};

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-semantic` failure onto the boundary's exit taxonomy.
///
/// The mapping is deliberate and total, and it is the ONE place a
/// `SemanticError` becomes an exit status. Three groups:
///
/// - **`Refused` (2)** — the request was understood and the world declined it:
///   a module root with no readable `manifest.yaml`, a manifest that is not
///   YAML or not a mapping, a corpus root that cannot be walked, or no
///   vendored contract to judge against. Fixing it means changing the world,
///   not the request.
/// - **`Io` → Internal (4)** — quoin's own shipped data is broken: a vendored
///   schema that will not read, will not parse, will not compile, or lacks the
///   sub-schema this build reads from it. No caller can act on any of these.
/// - There is deliberately **no `BadRequest` group**. Everything a caller can
///   get wrong in this domain — a field of the wrong type, a path past the
///   ceiling — is refused in [`super`] before `quoin-semantic` is reached, so a
///   `SemanticError` is never the caller's spelling.
///
/// `SemanticErrorCode` is `#[non_exhaustive]`, so this match needs a `_` arm
/// and the compiler will NOT flag a code added upstream.
/// `the_error_mapping_covers_every_semantic_code` pins
/// `SemanticErrorCode::all().len()`, which a loop over `all()` could not do —
/// such a loop re-runs this same match and agrees with itself (the quoin#443
/// failure mode).
pub(super) fn map_error(error: &SemanticError, op: &'static str) -> CoreError {
    let code = error.code();
    let mapped = core_code(code);
    let envelope = CoreError::new(mapped.unwrap_or(CoreErrorCode::Io), error.to_string())
        .with_context("op", op)
        .with_context("semantic_code", code.as_str());
    match mapped {
        Some(_) => envelope,
        // A code this build has no opinion on. It exits Internal (4), and it
        // SAYS it was unmapped rather than posing as a considered answer.
        None => envelope.with_context("mapping", "unrecognised"),
    }
}

/// The exit-taxonomy code one `SemanticErrorCode` maps to, or `None` for a code
/// this build does not know.
///
/// `Option` rather than a total function, because `SemanticErrorCode` is
/// `#[non_exhaustive]`: "unknown upstream code" and "deliberately Internal" are
/// different facts, and collapsing them would make the wildcard arm
/// indistinguishable from a considered decision — to a reader and to the
/// linter both.
const fn core_code(code: SemanticErrorCode) -> Option<CoreErrorCode> {
    Some(match code {
        SemanticErrorCode::ManifestUnreadable
        | SemanticErrorCode::ManifestNotYaml
        | SemanticErrorCode::ManifestNotAMapping
        | SemanticErrorCode::CorpusUnreadable
        | SemanticErrorCode::ContractRootUnset => CoreErrorCode::Refused,

        SemanticErrorCode::VendoredSchemaUnreadable
        | SemanticErrorCode::VendoredSchemaNotJson
        | SemanticErrorCode::VendoredSchemaInvalid
        | SemanticErrorCode::VendoredSchemaIncomplete => CoreErrorCode::Io,

        // Required by `#[non_exhaustive]`. A code this build has never seen is
        // reported as unmapped and exits Internal — the safest answer, since an
        // unknown rule is not one the caller can act on. The count pin in
        // `the_error_mapping_covers_every_semantic_code` is what makes reaching
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
    use quoin_semantic::{SemanticError, SemanticErrorCode};

    use super::{CoreErrorCode, core_code, map_error};

    /// `SemanticErrorCode` is `#[non_exhaustive]`, so `map_error`'s `_` arm
    /// means the compiler cannot catch a code added upstream. This pins the
    /// COUNT instead. A loop over `all()` would only re-run the same match and
    /// agree with itself — the quoin#443 failure mode — so the number is
    /// written out here, and every group is listed by name.
    ///
    /// When this fails: a code was added to `quoin-semantic`. Decide which of
    /// the two groups it belongs to, add it to that arm, and raise the count.
    #[test]
    fn the_error_mapping_covers_every_semantic_code() {
        assert_eq!(
            SemanticErrorCode::all().len(),
            9,
            "quoin-semantic gained or lost an error code; map it in map_error deliberately"
        );

        let refused = [
            SemanticErrorCode::ManifestUnreadable,
            SemanticErrorCode::ManifestNotYaml,
            SemanticErrorCode::ManifestNotAMapping,
            SemanticErrorCode::CorpusUnreadable,
            SemanticErrorCode::ContractRootUnset,
        ];
        let internal = [
            SemanticErrorCode::VendoredSchemaUnreadable,
            SemanticErrorCode::VendoredSchemaNotJson,
            SemanticErrorCode::VendoredSchemaInvalid,
            SemanticErrorCode::VendoredSchemaIncomplete,
        ];
        assert_eq!(
            refused.len() + internal.len(),
            SemanticErrorCode::all().len(),
            "a code is in `all()` but in neither of the two groups"
        );

        for code in refused {
            assert_eq!(core_code(code), Some(CoreErrorCode::Refused), "{code}");
        }
        for code in internal {
            assert_eq!(core_code(code), Some(CoreErrorCode::Io), "{code}");
        }

        // And the envelope carries the semantic code through, so an operator
        // can tell which rule declined without parsing the sentence.
        let envelope = map_error(&SemanticError::ContractRootUnset, "semantic.read_blocks");
        assert_eq!(envelope.context["semantic_code"], "QSEM-009");
        assert_eq!(envelope.code, CoreErrorCode::Refused);
        assert_eq!(envelope.outcome().code(), 2);
    }
}
