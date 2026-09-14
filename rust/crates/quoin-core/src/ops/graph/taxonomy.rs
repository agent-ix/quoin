// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One `GraphError`, one exit status.
//!
//! The single place a `quoin-graph-analysis` failure becomes a number on the
//! boundary's exit taxonomy. Split from the operations in [`super`] for the
//! reason `ops::semantic::taxonomy` states: the operations decide *what to do*,
//! this decides *how a failure is reported*, and a mapping alone in a file can
//! be asserted against the error catalogue itself.
//!
//! A GAP is not an error here and never reaches this file. An absent or
//! unreadable bindings store, an unknown requirement seed, a binding whose
//! obligation the export does not carry — all of those are reported INSIDE the
//! report, as `state` and `gaps`, exactly as `src/graph-analysis/` reported
//! them. Only a refusal to read or accept one of the three DECLARED inputs
//! arrives here, and the retained commands exited 2 on every one of them.

use quoin_graph_analysis::{GraphError, GraphErrorCode, GraphInput};

use crate::error::{CoreError, CoreErrorCode};

/// Map a `quoin-graph-analysis` failure onto the boundary's exit taxonomy.
///
/// Two groups:
///
/// - **`Refused` (2)** — the request was understood and the world declined it:
///   a declared input that cannot be read, or one that does not satisfy its
///   contract. Both are the caller's FILES rather than the caller's request
///   shape, which is why neither is `BadRequest` (3): the request named four
///   paths and every one of them was well formed. `src/commands/graph/*` exited
///   2 on both, and this preserves that.
/// - **`Io` → Internal (4)** — a report with no canonical JSON spelling. No
///   analysis this crate produces can reach it — every number in a report is a
///   count — and if one ever does, it is quoin's own fault and not the
///   caller's.
///
/// `GraphErrorCode` is `#[non_exhaustive]`, so this match needs a `_` arm and
/// the compiler will NOT flag a code added upstream.
/// `the_error_mapping_covers_every_graph_code` pins `GraphErrorCode::all().len()`
/// instead — a loop over `all()` would only re-run this same match and agree
/// with itself (the quoin#443 failure mode).
pub(super) fn map_error(error: &GraphError, op: &'static str) -> CoreError {
    let code = error.code();
    let mapped = core_code(code);
    let mut envelope = CoreError::new(mapped.unwrap_or(CoreErrorCode::Io), error.to_string())
        .with_context("op", op)
        .with_context("graph_code", code.as_str());
    if let Some(input) = error.input() {
        // Which of the three declared inputs was refused is the one fact the
        // retained `GraphLoadFailure["input"]` carried to the caller, and
        // `tests/graph-command.test.ts` asserted it by name. It stays a context
        // KEY rather than only a word in the sentence, so a caller can still
        // branch on it without parsing prose.
        envelope = envelope.with_context("input", GraphInput::as_str(input));
    }
    match mapped {
        Some(_) => envelope,
        // A code this build has no opinion on. It exits Internal (4), and it
        // SAYS it was unmapped rather than posing as a considered answer.
        None => envelope.with_context("mapping", "unrecognised"),
    }
}

/// The exit-taxonomy code one `GraphErrorCode` maps to, or `None` for a code
/// this build does not know.
///
/// `Option` rather than a total function, because `GraphErrorCode` is
/// `#[non_exhaustive]`: "unknown upstream code" and "deliberately Internal" are
/// different facts, and collapsing them would make the wildcard arm
/// indistinguishable from a considered decision.
const fn core_code(code: GraphErrorCode) -> Option<CoreErrorCode> {
    Some(match code {
        GraphErrorCode::InputUnreadable | GraphErrorCode::InputInvalid => CoreErrorCode::Refused,
        GraphErrorCode::Canonicalization => CoreErrorCode::Io,
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
    use quoin_graph_analysis::{GraphError, GraphErrorCode, GraphInput};

    use super::{CoreErrorCode, core_code, map_error};

    /// `GraphErrorCode` is `#[non_exhaustive]`, so `map_error`'s `_` arm means
    /// the compiler cannot catch a code added upstream. This pins the COUNT
    /// instead, and lists every group by name.
    ///
    /// When this fails: a code was added to `quoin-graph-analysis`. Decide
    /// which of the two groups it belongs to, add it to that arm, and raise the
    /// count.
    ///
    /// Provenance: quoin#500
    #[test]
    fn the_error_mapping_covers_every_graph_code() {
        assert_eq!(
            GraphErrorCode::all().len(),
            3,
            "quoin-graph-analysis gained or lost an error code; map it in map_error deliberately"
        );

        let refused = [
            GraphErrorCode::InputUnreadable,
            GraphErrorCode::InputInvalid,
        ];
        let internal = [GraphErrorCode::Canonicalization];
        assert_eq!(
            refused.len() + internal.len(),
            GraphErrorCode::all().len(),
            "a code is in `all()` but in neither of the two groups"
        );

        for code in refused {
            assert_eq!(core_code(code), Some(CoreErrorCode::Refused), "{code}");
        }
        for code in internal {
            assert_eq!(core_code(code), Some(CoreErrorCode::Io), "{code}");
        }
    }

    /// The envelope carries the graph code and the refused input through, so an
    /// operator can tell which input declined without parsing the sentence.
    ///
    /// Provenance: quoin#500
    #[test]
    fn a_refusal_names_the_input_it_is_about() {
        let envelope = map_error(
            &GraphError::invalid(GraphInput::Audit, "format is not quoin-audit-envelope"),
            "graph.fan_out",
        );
        assert_eq!(envelope.code, CoreErrorCode::Refused);
        assert_eq!(envelope.outcome().code(), 2);
        assert_eq!(envelope.context["graph_code"], "QGA-1002");
        assert_eq!(envelope.context["input"], "audit");
        assert_eq!(envelope.context["op"], "graph.fan_out");
    }
}
