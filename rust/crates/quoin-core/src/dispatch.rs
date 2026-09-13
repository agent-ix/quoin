// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Routing `<domain>.<op>` to an operation, and nothing else.
//!
//! This module holds no domain logic and must not acquire any. It is the one
//! place that knows which operations exist, so "which operations exist" is one
//! fact in one place — an exhaustive `match` the compiler checks, not a
//! registry assembled at run time.

use std::io::Read;

use crate::capabilities::Capabilities;
use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::{MAX_REQUEST_BYTES, Response};

/// Every operation this build answers, in the wire spelling.
///
/// Exposed so a caller — and `quoin-difftest` — can enumerate the surface
/// rather than discover it by trying names.
pub const OPERATIONS: &[&str] = &[
    "assurance.build_authored_argument",
    "assurance.build_case",
    "assurance.build_discharge",
    "assurance.parse_argument",
    "assurance.render_authored_argument",
    "assurance.render_case",
    "assurance.render_discharge",
    "assurance.requirement_of",
    "completeness.assess_bundle",
    "completeness.read_frontmatter",
    "completeness.schema_refs",
    "config.resolve_org",
    "config.unresolved_org_message",
    "core.ping",
    "modules.ensure_defaults",
    "modules.install",
    "modules.list",
    "modules.remove",
    "validators.run",
];

/// Route one request.
///
/// `op` is the sole command-line argument; `request` is the parsed stdin
/// document; `capabilities` is what `main.rs` granted this invocation.
///
/// `capabilities` is threaded through every arm rather than only the arms that
/// need one, so the grant is visible at the routing table instead of being
/// reached for inside an operation. An operation that needs a capability it was
/// not granted says so as an internal fault (see `ops::modules`), never by
/// acquiring one itself — `tests/tc_library_containment.rs` is what makes that
/// a rule rather than a habit.
///
/// # Errors
///
/// [`CoreErrorCode::UnknownOp`] when `op` names nothing this build implements,
/// or whatever the operation itself returns.
pub fn dispatch(
    op: &str,
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    match op {
        "assurance.requirement_of" => crate::ops::assurance::requirement_of(request),
        "assurance.build_case" => crate::ops::assurance::build_case(request),
        "assurance.render_case" => crate::ops::assurance::render_case(request),
        "assurance.parse_argument" => crate::ops::assurance::parse_argument(request),
        "config.resolve_org" => crate::ops::config::resolve_org(request),
        "config.unresolved_org_message" => crate::ops::config::unresolved_org_message(request),
        "assurance.build_authored_argument" => {
            crate::ops::assurance::build_authored_argument(request)
        }
        "assurance.render_authored_argument" => {
            crate::ops::assurance::render_authored_argument(request)
        }
        "assurance.build_discharge" => crate::ops::assurance::build_discharge(request),
        "assurance.render_discharge" => crate::ops::assurance::render_discharge(request),
        "completeness.assess_bundle" => crate::ops::completeness::assess_bundle(request),
        "completeness.read_frontmatter" => crate::ops::completeness::read_frontmatter(request),
        "completeness.schema_refs" => crate::ops::completeness::schema_refs(request),
        "core.ping" => crate::ops::core::ping(request),
        "modules.ensure_defaults" => crate::ops::modules::ensure_defaults(request, capabilities),
        "modules.install" => crate::ops::modules::install(request, capabilities),
        "modules.list" => crate::ops::modules::list(request, capabilities),
        "modules.remove" => crate::ops::modules::remove(request, capabilities),
        "validators.run" => crate::ops::validators::run(request),
        _ => Err(
            CoreError::new(CoreErrorCode::UnknownOp, "no such operation in this build")
                .with_context("op", op)
                .with_context("known", OPERATIONS.join(",")),
        ),
    }
}

/// Parse a stdin document.
///
/// An empty or whitespace-only stdin means the empty request `{}`. A request
/// that is not a JSON **object** is refused: the unit of IPC is a named
/// operation with named arguments, so a bare array or scalar is a caller
/// mistake worth naming rather than coercing.
///
/// # Errors
///
/// [`CoreErrorCode::BadJson`] when stdin is neither empty nor a JSON object.
pub fn parse_request(stdin: &str) -> Result<serde_json::Value, CoreError> {
    if stdin.trim().is_empty() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }
    let value: serde_json::Value = serde_json::from_str(stdin)
        .map_err(|e| CoreError::new(CoreErrorCode::BadJson, e.to_string()))?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(
            CoreError::new(CoreErrorCode::BadJson, "a request must be a JSON object")
                .with_context("observed_type", json_type_name(&value).to_owned()),
        )
    }
}

/// Read one request off an untrusted stream, bounded.
///
/// The ceiling is [`MAX_REQUEST_BYTES`] and it is applied to the READ, not to
/// what the read produced: the reader is capped at one byte past the ceiling,
/// so a stream of any size beyond it costs `MAX_REQUEST_BYTES + 1` bytes and is
/// refused without a parse, without a re-serialisation and without a clone.
///
/// This is the whole difference between a bound and a remark. Before quoin#448's
/// review, `main.rs` did an uncapped `read_to_string`, `parse_request` built the
/// entire `serde_json::Value`, and only then did an operation measure the
/// request by re-serialising it — a 2 GiB stdin was read, parsed and copied
/// before anything said "too large". `core.ping` never carried more than a
/// token so it never showed; `validators.run` carries a repository.
///
/// A stream that is not UTF-8 is [`CoreErrorCode::BadJson`], the same as any
/// other undecodable request: the boundary speaks JSON text, and a caller
/// cannot act differently on "bad bytes" than on "bad syntax".
///
/// # Errors
///
/// [`CoreErrorCode::Refused`] when the stream exceeds [`MAX_REQUEST_BYTES`],
/// [`CoreErrorCode::BadJson`] when what arrived is not a JSON object, and
/// [`CoreErrorCode::Io`] when the stream itself fails.
pub fn read_request(reader: impl Read) -> Result<serde_json::Value, CoreError> {
    // `+ 1`: reading exactly the ceiling cannot distinguish "exactly at the
    // limit" from "the first `MAX_REQUEST_BYTES` of something larger".
    let ceiling = u64::try_from(MAX_REQUEST_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    // Read BYTES, compare BYTES, decode LAST.
    //
    // `take` cuts at a byte offset, and that offset can fall inside a
    // multi-byte character. Decoding during the read therefore made the
    // classification of an oversized request depend on the CALLER'S ALPHABET:
    // an all-ASCII request one byte over the ceiling was `CORE_REFUSED` (exit
    // 2), while the same request from a caller whose text happened to put a
    // `€` across the cut was `CORE_BAD_JSON` (exit 3) — reported as malformed
    // when it was merely too large, and the message was false besides, because
    // the stream did contain valid UTF-8. `src/core/exec.ts` branches on those
    // statuses, so this is a behaviour difference and not a wording one.
    //
    // Deciding the size first removes the dependency: the length test cannot
    // see an encoding, and the decode below only ever runs on bytes already
    // known to be within the bound (agent-ix/quoin#447; tc_445_104, tc_445_105).
    let mut bytes = Vec::new();
    let read = reader.take(ceiling).read_to_end(&mut bytes).map_err(|e| {
        CoreError::new(CoreErrorCode::Io, e.to_string()).with_context("stream", "stdin")
    })?;

    if read > MAX_REQUEST_BYTES {
        return Err(
            CoreError::new(CoreErrorCode::Refused, "request exceeds the accepted size")
                .with_context("limit_bytes", MAX_REQUEST_BYTES.to_string())
                // What was READ, not what was sent: the stream was abandoned at
                // the ceiling, so the sender's total is a number nobody here
                // observed and naming it would be inventing a fact.
                .with_context("read_bytes", read.to_string()),
        );
    }

    // Within the bound and undecodable is a GENUINE encoding fault, and keeps
    // the malformed-request classification the size test must not borrow.
    let text = String::from_utf8(bytes).map_err(|e| {
        CoreError::new(CoreErrorCode::BadJson, e.to_string()).with_context("stream", "stdin")
    })?;

    parse_request(&text)
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Read the operation name out of the argument vector.
///
/// # Errors
///
/// [`CoreErrorCode::BadUsage`] for anything but exactly one argument shaped
/// `<domain>.<op>`.
pub fn parse_operation(args: &[String]) -> Result<&str, CoreError> {
    let [op] = args else {
        return Err(CoreError::new(
            CoreErrorCode::BadUsage,
            "usage: quoin-core <domain>.<op>  (JSON request on stdin)",
        )
        .with_context("argument_count", args.len().to_string()));
    };
    if op.split('.').filter(|part| !part.is_empty()).count() == 2 && op.matches('.').count() == 1 {
        Ok(op)
    } else {
        Err(CoreError::new(
            CoreErrorCode::BadUsage,
            "an operation is spelled <domain>.<op>",
        )
        .with_context("argument", op.clone()))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    /// Every `"…" =>` arm of `dispatch`'s match, read out of this file's own
    /// source.
    ///
    /// `OPERATIONS` is a hand-written list standing beside an exhaustive
    /// `match`, and the compiler checks neither against the other. It drifted
    /// on four consecutive additions (#426, #431, #435, #441) and stayed green
    /// throughout, because the test that was supposed to catch it asserted a
    /// literal — `known == "core.ping"` — which is exactly the stale value. A
    /// test that reads the same hand-written string it is guarding cannot
    /// notice it going stale (#443).
    ///
    /// Reading the source is the only instrument available: Rust cannot
    /// enumerate a match's arms at compile time, and enumerating them at run
    /// time would mean the run-time registry this module's own doc comment
    /// refuses.
    fn routed_operations() -> Vec<String> {
        let source = include_str!("dispatch.rs");
        let body = source
            .split_once("pub fn dispatch(")
            .expect("dispatch is defined in this file")
            .1
            .split_once("\n}")
            .expect("dispatch's body is brace-terminated")
            .0;
        let mut routed: Vec<String> = body
            .lines()
            .filter_map(|line| line.trim().strip_prefix('"'))
            .filter_map(|rest| rest.split_once("\" =>"))
            .map(|(name, _)| name.to_owned())
            .collect();
        routed.sort();
        routed
    }

    #[test]
    fn the_operations_const_is_every_operation_dispatch_routes() {
        let mut declared: Vec<String> = OPERATIONS.iter().map(|op| (*op).to_owned()).collect();
        declared.sort();
        assert_eq!(
            declared,
            routed_operations(),
            "`OPERATIONS` and `dispatch`'s match disagree. Whichever was edited, \
             edit the other: the const is what a caller and `quoin-difftest` \
             enumerate, and the unknown-op error reports it to the user."
        );
    }

    #[test]
    fn the_drift_guard_can_see_the_arms_it_guards() {
        // Without this, a refactor that changes `dispatch`'s formatting turns
        // the guard above into a comparison of two empty lists, which passes.
        // A check over an empty population is the failure this ticket is about.
        assert!(
            routed_operations().len() >= 2,
            "the source scan found {} arm(s); it has stopped reading `dispatch`",
            routed_operations().len()
        );
    }

    #[test]
    fn an_unknown_operation_names_the_ones_that_exist() {
        let error = dispatch(
            "evidence.record",
            &serde_json::json!({}),
            &Capabilities::none(),
        )
        .unwrap_err();
        assert_eq!(error.code, CoreErrorCode::UnknownOp);
        assert_eq!(error.outcome().code(), 3);
        assert_eq!(error.context["known"], OPERATIONS.join(","));
        // Not a tautology against the line above: this is the fact a user
        // reads off a mistyped operation, and `dispatch` builds the string
        // itself. The value is pinned by the drift guard, not by this test.
        assert!(
            error.context["known"].contains("assurance.build_case"),
            "known: {}",
            error.context["known"]
        );
    }

    /// The ceiling is on the READ. A stream past it is refused without the
    /// document ever being parsed — which is why the payload below is
    /// deliberately UNPARSEABLE: a `CORE_BAD_JSON` here would mean the parser
    /// ran first, and the bound was a remark about an allocation that had
    /// already happened (quoin#448 FND-003).
    #[test]
    fn a_stream_past_the_ceiling_is_refused_before_it_is_parsed() {
        let unparseable = format!(r#"{{"echo":"{}"#, "x".repeat(MAX_REQUEST_BYTES));
        assert!(unparseable.len() > MAX_REQUEST_BYTES);
        let error = read_request(unparseable.as_bytes()).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Refused);
        assert_eq!(error.outcome().code(), 2);
        assert_eq!(error.context["limit_bytes"], MAX_REQUEST_BYTES.to_string());
    }

    /// And the read really stops: the stream is not drained to find out how big
    /// it was.
    ///
    /// A counting reader rather than a comment. Without the cap this consumes
    /// every byte offered — four times the ceiling here, unbounded in
    /// production, where the offer is whatever a caller pipes in.
    #[test]
    fn the_stream_is_abandoned_at_the_ceiling_not_drained() {
        struct Counting<R> {
            inner: R,
            read: std::rc::Rc<std::cell::Cell<usize>>,
        }
        impl<R: Read> Read for Counting<R> {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                let n = self.inner.read(buffer)?;
                self.read.set(self.read.get() + n);
                Ok(n)
            }
        }

        let read = std::rc::Rc::new(std::cell::Cell::new(0));
        let offered = MAX_REQUEST_BYTES * 4;
        let error = read_request(Counting {
            inner: std::io::repeat(b'x').take(u64::try_from(offered).unwrap_or(u64::MAX)),
            read: std::rc::Rc::clone(&read),
        })
        .unwrap_err();

        assert_eq!(error.code, CoreErrorCode::Refused);
        assert_eq!(
            error.context["read_bytes"],
            (MAX_REQUEST_BYTES + 1).to_string()
        );
        assert!(
            read.get() <= MAX_REQUEST_BYTES + 1,
            "the reader consumed {} bytes of the {offered} offered; the ceiling \
             is on the read, not on what the read produced",
            read.get()
        );
    }

    /// Exactly at the ceiling is accepted. A bound that refuses its own limit
    /// is a different bound from the one documented, and the boundary between
    /// "accepted" and "refused" is the only part of a ceiling anyone meets.
    #[test]
    fn a_request_of_exactly_the_ceiling_is_accepted() {
        let padding = MAX_REQUEST_BYTES - r#"{"echo":""}"#.len();
        let at_the_limit = format!(r#"{{"echo":"{}"}}"#, "x".repeat(padding));
        assert_eq!(at_the_limit.len(), MAX_REQUEST_BYTES);
        let request = read_request(at_the_limit.as_bytes()).unwrap();
        assert_eq!(request["echo"].as_str().map(str::len), Some(padding));
    }

    /// One byte more is refused, and the refusal names what it read.
    #[test]
    fn one_byte_past_the_ceiling_is_refused() {
        let padding = MAX_REQUEST_BYTES - r#"{"echo":""}"#.len() + 1;
        let over = format!(r#"{{"echo":"{}"}}"#, "x".repeat(padding));
        assert_eq!(over.len(), MAX_REQUEST_BYTES + 1);
        let error = read_request(over.as_bytes()).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Refused);
        assert_eq!(
            error.context["read_bytes"],
            (MAX_REQUEST_BYTES + 1).to_string()
        );
    }

    /// The transport ceiling stays strictly above every domain bound.
    ///
    /// Each `ops::*` bound refuses with the `op` that refused and the quantity
    /// it measured. The transport refusal carries neither, because it fires
    /// before the bytes are a request at all. If this ceiling ever met or fell
    /// below a domain bound, the transport verdict would shadow that domain's
    /// and the domain's own refusal would become unreachable through the
    /// binary — compiled, unit-tested, and dead. The failure mode is silent,
    /// so it is asserted rather than commented (quoin#448 FND-003).
    ///
    /// `ops::validators::MAX_RUN_REQUEST_BYTES` is the one this is really for:
    /// it is the largest request the boundary accepts, and the transport
    /// ceiling above it is what stops the read before that bound can be
    /// measured on something unbounded.
    #[test]
    fn tc_412_the_transport_ceiling_stays_above_every_domain_bound() {
        // Every domain bound in the crate, named one by one: a scan would find
        // whatever it found, and find nothing once the constants are renamed.
        let domain_bounds = [
            (
                "ops::core::MAX_ECHO_BYTES",
                crate::ops::core::MAX_ECHO_BYTES,
            ),
            (
                "ops::assurance::MAX_BUILD_CASE_BYTES",
                crate::ops::assurance::MAX_BUILD_CASE_BYTES,
            ),
            (
                "ops::validators::MAX_RUN_REQUEST_BYTES",
                crate::ops::validators::MAX_RUN_REQUEST_BYTES,
            ),
            (
                "ops::assurance::MAX_OBLIGATION_ID_BYTES",
                crate::ops::assurance::MAX_OBLIGATION_ID_BYTES,
            ),
            (
                "ops::completeness::MAX_MANIFEST_BYTES",
                crate::ops::completeness::MAX_MANIFEST_BYTES,
            ),
            (
                "ops::completeness::MAX_ASSESS_BUNDLE_BYTES",
                crate::ops::completeness::MAX_ASSESS_BUNDLE_BYTES,
            ),
        ];
        // The count is pinned beside the list because the list is hand-written:
        // a bound added to an `ops` module and not added here would leave this
        // test green over a population that no longer includes it, which is the
        // "agrees with itself" shape quoin#443 records. `ops::mod` declares one
        // module per domain, so the number moves when a domain does.
        assert_eq!(
            domain_bounds.len(),
            6,
            "a domain bound was added or removed without this census moving"
        );
        for (name, bound) in domain_bounds {
            assert!(
                bound < MAX_REQUEST_BYTES,
                "{name} is {bound} and the transport ceiling is {MAX_REQUEST_BYTES}: \
                 a request that hits {name} would be refused by the transport first, \
                 and {name}'s refusal could no longer be reached"
            );
        }
    }

    /// A stream that is not UTF-8 is a malformed request, not a failing stream:
    /// the boundary speaks JSON text and a caller cannot act differently on
    /// "bad bytes" than on "bad syntax".
    #[test]
    fn a_non_utf8_stream_is_bad_json_not_io() {
        let error = read_request(&[0xff, 0xfe, 0x00][..]).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadJson);
        assert_eq!(error.outcome().code(), 3);
    }

    #[test]
    fn empty_stdin_is_the_empty_request() {
        assert_eq!(parse_request("   \n ").unwrap(), serde_json::json!({}));
    }

    #[test]
    fn malformed_json_is_invalid_not_internal() {
        let error = parse_request("{oops").unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadJson);
        assert_eq!(error.outcome().code(), 3);
    }

    #[test]
    fn a_non_object_request_is_refused_by_type() {
        let error = parse_request("[1,2]").unwrap_err();
        assert_eq!(error.context["observed_type"], "array");
    }

    #[test]
    fn usage_needs_exactly_one_dotted_argument() {
        assert_eq!(
            parse_operation(&["core.ping".to_owned()]).unwrap(),
            "core.ping"
        );
        for bad in [
            vec![],
            vec!["core".to_owned()],
            vec!["core.ping".to_owned(), "x".to_owned()],
        ] {
            assert_eq!(
                parse_operation(&bad).unwrap_err().code,
                CoreErrorCode::BadUsage
            );
        }
        assert_eq!(
            parse_operation(&["core.".to_owned()]).unwrap_err().code,
            CoreErrorCode::BadUsage
        );
    }
}
