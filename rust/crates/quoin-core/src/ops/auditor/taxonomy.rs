// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One `AuditorError`, one exit status.
//!
//! Split from the operations in [`super`] for the reason
//! `ops::graph::taxonomy` states: the operations decide *what to do*, this
//! decides *how a failure is reported*, and a mapping alone in a file can be
//! asserted against the error catalogue itself.
//!
//! A FINDING is not an error here and never reaches this file. An unbound
//! obligation, a stale statement hash, a mocked binding, a vacuous scan — all
//! of those are the audit's ANSWER, reported inside the report, exactly as
//! `src/auditor/audit.ts` reported them. `quoin evidence audit` exits 0 with a
//! full report of violations; only a refusal to compute one arrives here.

use quoin_auditor::{AuditorError, AuditorErrorCode};

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-auditor` failure onto the boundary's exit taxonomy.
///
/// Two groups:
///
/// - **`Refused` (2)** — the request was understood and the auditor declined
///   it. A configuration space too large to count
///   ([`AuditorErrorCode::DemandTooLarge`]) is the caller's declaration, and a
///   module whose manifest cannot be read
///   ([`AuditorErrorCode::ManifestUnreadable`]) is the caller's tree. Neither
///   is `BadRequest` (3): the request parsed.
/// - **`Io` → Internal (4)** — a report that cannot be serialised. Unreachable
///   by construction (see the code's own documentation) and quoin's fault if
///   it ever happens.
///
/// `AuditorErrorCode` is `#[non_exhaustive]`, so this match needs a `_` arm and
/// the compiler will NOT flag a code added upstream.
/// `the_error_mapping_covers_every_auditor_code` pins `AuditorErrorCode::ALL.len()`
/// instead — a loop over `ALL` would only re-run this same match and agree with
/// itself (the quoin#443 failure mode).
pub(super) fn map_error(error: &AuditorError, op: &'static str) -> CoreError {
    let code = error.code;
    let mapped = core_code(code);
    let envelope = CoreError::new(mapped.unwrap_or(CoreErrorCode::Io), error.to_string())
        .with_context("op", op)
        .with_context("auditor_code", code.as_str());
    match mapped {
        Some(_) => envelope,
        // A code this build has no opinion on. It exits Internal (4), and it
        // SAYS it was unmapped rather than posing as a considered answer.
        None => envelope.with_context("mapping", "unrecognised"),
    }
}

/// The exit-taxonomy code one `AuditorErrorCode` maps to, or `None` for a code
/// this build does not know.
///
/// `Option` rather than a total function, because `AuditorErrorCode` is
/// `#[non_exhaustive]`: "unknown upstream code" and "deliberately Internal" are
/// different facts, and collapsing them would make the wildcard arm
/// indistinguishable from a considered decision.
const fn core_code(code: AuditorErrorCode) -> Option<CoreErrorCode> {
    Some(match code {
        AuditorErrorCode::DemandTooLarge | AuditorErrorCode::ManifestUnreadable => {
            CoreErrorCode::Refused
        }
        AuditorErrorCode::ReportNotSerialisable => CoreErrorCode::Io,
        // Required by `#[non_exhaustive]`. A code this build has never seen is
        // reported as unmapped and exits Internal — the safest answer, since an
        // unknown rule is not one the caller can act on.
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
    use quoin_auditor::{AuditorError, AuditorErrorCode};

    use super::{core_code, map_error};
    use crate::error::CoreErrorCode;

    /// Trace: FR-101-AC-3
    /// Provenance: quoin#501
    #[test]
    fn the_error_mapping_covers_every_auditor_code() {
        // Pinned rather than iterated: a loop over `ALL` would call the same
        // match this file defines and agree with itself. A code added upstream
        // breaks this number and forces a decision here (quoin#443).
        assert_eq!(AuditorErrorCode::ALL.len(), 3);
        assert_eq!(
            core_code(AuditorErrorCode::DemandTooLarge),
            Some(CoreErrorCode::Refused)
        );
        assert_eq!(
            core_code(AuditorErrorCode::ManifestUnreadable),
            Some(CoreErrorCode::Refused)
        );
        assert_eq!(
            core_code(AuditorErrorCode::ReportNotSerialisable),
            Some(CoreErrorCode::Io)
        );
    }

    /// Trace: FR-101-AC-3
    /// Provenance: quoin#501
    #[test]
    fn a_refusal_carries_its_code_and_its_operation() {
        let error = map_error(
            &AuditorError::manifest_unreadable("no such file"),
            "auditor.audit",
        );
        assert_eq!(error.code, CoreErrorCode::Refused);
        assert_eq!(
            error.context.get("auditor_code").map(String::as_str),
            Some("QAU-MANIFEST-UNREADABLE")
        );
        assert_eq!(
            error.context.get("op").map(String::as_str),
            Some("auditor.audit")
        );
        assert!(!error.context.contains_key("mapping"));
    }
}
