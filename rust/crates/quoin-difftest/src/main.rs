// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin-difftest` — the retained TypeScript and `quoin-core`, one request,
//! one verdict.
//!
//! FR-101 says a capability is retired only after old and new implementations
//! passed at ONE candidate revision. This is the thing that checks that. It
//! feeds the same JSON request to both entry points and compares three facts:
//!
//! 1. **canonical stdout** — byte identity after key sorting;
//! 2. **exit status** — the taxonomy in [`quoin_core::protocol::Outcome`];
//! 3. **normalised diagnostic shape** — the ordered list of `(code, context
//!    keys)`. Deliberately NOT the message text: quoin#373 records that
//!    verdicts are contractual and error text is not, and comparing prose
//!    would make every reworded sentence a false parity failure.
//!
//! It is small on purpose. It covers the operations that exist; when Stage 1
//! lands `quoin-quire` it gains cases, not a framework.
//!
//! ```text
//! quoin-difftest --core rust/target/debug/quoin-core \
//!                --ts  scripts/core-reference.mjs
//! ```

use std::io::Write as _;
use std::process::{Command, Stdio};

use quoin_core::protocol::Outcome;

/// One request, run against both sides.
struct Case {
    name: &'static str,
    op: &'static str,
    request: Request,
}

/// How a case's stdin is produced.
///
/// A bound-breaching request is described rather than written out: a 4 KiB
/// literal in this table would be unreadable, and the interesting fact about
/// it is its size.
enum Request {
    /// Exactly these bytes.
    Literal(&'static str),
    /// `{"echo": "x" * n}`.
    EchoOfBytes(usize),
    /// A well-formed `assurance.build_case` request of exactly `n` bytes.
    ///
    /// Generated rather than written because the bound it probes is 16 MiB,
    /// and "too large to put in the table" is a reason to generate the input,
    /// not a reason to leave the boundary unasserted. Same class as the 64 MiB
    /// `maxBuffer` incident, where the failure only appeared at a real payload
    /// size — a limit nothing ever reaches is a limit nobody has tested.
    ///
    /// Both sides measure the RE-SERIALISED request, and `serde_json` sorts
    /// object keys where `JSON.stringify` preserves insertion order. The two
    /// orderings differ; their byte counts do not, which is why this works.
    BuildCaseOfBytes(usize),
}

/// The `assurance.build_case` request [`Request::BuildCaseOfBytes`] pads.
///
/// Documents are empty on purpose: the padding rides in an obligation's
/// statement, so a 16 MiB request still produces a few-hundred-byte payload
/// and a difference is readable rather than being a wall of `x`.
const BUILD_CASE_ENVELOPE: &str =
    r#"{"documents":[],"obligations":[{"id":"FR-001-AC-1","statement":""}],"findings":[]}"#;

impl Request {
    fn text(&self) -> String {
        match *self {
            Self::Literal(text) => text.to_owned(),
            Self::EchoOfBytes(n) => format!(r#"{{"echo":"{}"}}"#, "x".repeat(n)),
            Self::BuildCaseOfBytes(n) => format!(
                r#"{{"documents":[],"obligations":[{{"id":"FR-001-AC-1","statement":"{}"}}],"findings":[]}}"#,
                "x".repeat(n.saturating_sub(BUILD_CASE_ENVELOPE.len()))
            ),
        }
    }
}

/// The Stage-0 case set: every exit status `core.ping` can reach.
///
/// A harness that only exercises the success path proves the two
/// implementations agree about success, which is not the interesting claim.
const CASES: &[Case] = &[
    Case {
        name: "ok/empty",
        op: "core.ping",
        request: Request::Literal("{}"),
    },
    Case {
        name: "ok/no-stdin",
        op: "core.ping",
        request: Request::Literal(""),
    },
    Case {
        name: "ok/echo",
        op: "core.ping",
        request: Request::Literal(r#"{"echo":"corr-7"}"#),
    },
    Case {
        name: "ok/echo-at-the-limit",
        op: "core.ping",
        request: Request::EchoOfBytes(4096),
    },
    Case {
        name: "ok/protocol-agrees",
        op: "core.ping",
        request: Request::Literal(r#"{"expect_protocol":1}"#),
    },
    Case {
        name: "partial/protocol-skew",
        op: "core.ping",
        request: Request::Literal(r#"{"expect_protocol":99}"#),
    },
    Case {
        name: "refused/echo-over-the-limit",
        op: "core.ping",
        request: Request::EchoOfBytes(4097),
    },
    Case {
        name: "invalid/unknown-field",
        op: "core.ping",
        request: Request::Literal(r#"{"eco":1}"#),
    },
    Case {
        name: "invalid/wrong-type",
        op: "core.ping",
        request: Request::Literal(r#"{"echo":7}"#),
    },
    Case {
        name: "invalid/malformed",
        op: "core.ping",
        request: Request::Literal("{oops"),
    },
    Case {
        name: "invalid/non-object",
        op: "core.ping",
        request: Request::Literal("[1,2]"),
    },
    // ── assurance.requirement_of (quoin#384) ──
    //
    // The first cases comparing a REAL retained capability. The TypeScript
    // side calls `src/assurance/graph.ts`'s own `requirementOf`; it does not
    // reimplement it. Everything above this line compares two implementations
    // of a purpose-built ping.
    Case {
        name: "assurance/ac-suffix",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"FR-001-AC-3"}"#),
    },
    Case {
        name: "assurance/metric-suffix",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"NFR-010-M-2"}"#),
    },
    Case {
        name: "assurance/unknown-suffix-is-still-a-prefix-match",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"FR-042-QQQ-9"}"#),
    },
    Case {
        name: "assurance/bare-requirement-id",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"FR-001"}"#),
    },
    Case {
        name: "assurance/unrecognised-returns-unchanged",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"not-an-id"}"#),
    },
    Case {
        name: "assurance/empty-id",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":""}"#),
    },
    Case {
        name: "assurance/non-ascii-falls-to-unchanged",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"é-001-AC-1"}"#),
    },
    Case {
        name: "assurance/invalid-unknown-field",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":"FR-001","extra":1}"#),
    },
    Case {
        name: "assurance/invalid-wrong-type",
        op: "assurance.requirement_of",
        request: Request::Literal(r#"{"obligation_id":7}"#),
    },
    // These two were missing from the nine, and their absence hid a real
    // divergence: the reference parsed stdin INSIDE each handler and answered
    // `CORE_BAD_REQUEST`, where `quoin-core` parses in the dispatcher and
    // answers `CORE_BAD_JSON`. Malformed input is a transport verdict for
    // every operation, so every operation must carry the cases that say so.
    Case {
        name: "assurance/requirement-of-invalid-malformed",
        op: "assurance.requirement_of",
        request: Request::Literal("{oops"),
    },
    Case {
        name: "assurance/requirement-of-invalid-non-object",
        op: "assurance.requirement_of",
        request: Request::Literal("[1,2]"),
    },
    // ── assurance.build_case (quoin#384) ──
    //
    // The whole view, against the retained `src/assurance/graph.ts`. The
    // TypeScript side CALLS it; it does not reimplement it.
    Case {
        name: "assurance/build-case-no-claim-declares-itself",
        op: "assurance.build_case",
        request: Request::Literal(r#"{"documents":[],"obligations":[],"findings":[]}"#),
    },
    Case {
        name: "assurance/build-case-searched-types-are-named-back",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[],"obligations":[],"findings":[],"claim_types":["Hazard","Threat"]}"#,
        ),
    },
    Case {
        name: "assurance/build-case-a-claim-with-nothing-under-it-is-open",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"The claim"}}],"obligations":[],"findings":[]}"#,
        ),
    },
    Case {
        name: "assurance/build-case-claim-type-is-case-insensitive",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"The claim"}}],"obligations":[],"findings":[],"claim_types":["str"]}"#,
        ),
    },
    Case {
        name: "assurance/build-case-evidence-under-a-traced-requirement",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"The claim"}},{"path":"f.md","body":"","frontmatter":{"id":"FR-001","type":"FR","title":"A requirement","relationships":[{"type":"traces_to","target":"ix://org/comp/StR-001"}]}}],"obligations":[{"id":"FR-001-AC-2","statement":"second"},{"id":"FR-001-AC-1","statement":"first"}],"findings":[]}"#,
        ),
    },
    Case {
        name: "assurance/build-case-a-finding-opens-the-whole-path",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"The claim"}},{"path":"f.md","body":"","frontmatter":{"id":"FR-001","type":"FR","title":"A requirement","relationships":[{"type":"traces_to","target":"ix://org/comp/StR-001"}]}}],"obligations":[{"id":"FR-001-AC-1","statement":"first"}],"findings":[{"obligation":"FR-001-AC-1","kind":"undischarged","summary":"nothing binds it","severity":"error","path":"src/x.ts","line":7}]}"#,
        ),
    },
    Case {
        // The property `quoin_finding_types::Finding::kind` argues for: a
        // closed 12-variant enum would refuse this, and the retained
        // implementation renders it. A difference here is the port being
        // stricter than the thing it replaces.
        name: "assurance/build-case-unknown-finding-kind",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"The claim"}},{"path":"f.md","body":"","frontmatter":{"id":"FR-001","type":"FR","title":"A requirement","relationships":[{"type":"traces_to","target":"StR-001"}]}}],"obligations":[{"id":"FR-001-AC-1","statement":"first"}],"findings":[{"obligation":"FR-001-AC-1","kind":"kind-invented-tomorrow","summary":"s"}]}"#,
        ),
    },
    Case {
        // The first finding per obligation wins, because the auditor has
        // already ordered by severity. Two findings, and only one `because`.
        name: "assurance/build-case-first-finding-per-obligation-wins",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"The claim"}},{"path":"f.md","body":"","frontmatter":{"id":"FR-001","type":"FR","title":"A requirement","relationships":[{"type":"traces_to","target":"StR-001"}]}}],"obligations":[{"id":"FR-001-AC-1","statement":"first"}],"findings":[{"obligation":"FR-001-AC-1","kind":"undischarged","summary":"most serious"},{"obligation":"FR-001-AC-1","kind":"stale-evidence","summary":"less serious"}]}"#,
        ),
    },
    Case {
        // quoin#170: a requirement refining TWO claims must appear under both.
        // Sharing one `ancestors` set across claims made the second report
        // "no sub-claim traces to this claim" about an edge the author wrote.
        name: "assurance/build-case-a-requirement-under-two-claims",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"a.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"First"}},{"path":"b.md","body":"","frontmatter":{"id":"StR-002","type":"StR","title":"Second"}},{"path":"f.md","body":"","frontmatter":{"id":"FR-001","type":"FR","title":"Shared","relationships":[{"type":"refines","target":"StR-001"},{"type":"refines","target":"StR-002"}]}}],"obligations":[{"id":"FR-001-AC-1","statement":"first"}],"findings":[]}"#,
        ),
    },
    Case {
        // A cycle must terminate, and only a node on THIS path is one.
        name: "assurance/build-case-a-cycle-terminates",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"a.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"First","relationships":[{"type":"refines","target":"FR-001"}]}},{"path":"f.md","body":"","frontmatter":{"id":"FR-001","type":"FR","title":"Cyclic","relationships":[{"type":"refines","target":"StR-001"}]}}],"obligations":[],"findings":[]}"#,
        ),
    },
    Case {
        // A requirement with obligations that no claim reaches. Visible, not
        // dropped: the gap IS the finding.
        name: "assurance/build-case-unreachable-requirement",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"The claim"}},{"path":"f.md","body":"","frontmatter":{"id":"FR-009","type":"FR","title":"Orphan"}}],"obligations":[{"id":"FR-009-AC-1","statement":"unreached"}],"findings":[]}"#,
        ),
    },
    Case {
        // An edge verb outside the downward set is not decomposition.
        name: "assurance/build-case-a-non-downward-edge-is-not-a-child",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":"The claim"}},{"path":"f.md","body":"","frontmatter":{"id":"FR-001","type":"FR","title":"Merely related","relationships":[{"type":"references","target":"StR-001"}]}}],"obligations":[{"id":"FR-001-AC-1","statement":"first"}],"findings":[]}"#,
        ),
    },
    Case {
        // `String(title ?? id)`: an explicit null falls through to the id
        // rather than rendering an empty statement in an assurance case.
        name: "assurance/build-case-null-title-falls-through-to-the-id",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":"StR-001","type":"StR","title":null}}],"obligations":[],"findings":[]}"#,
        ),
    },
    Case {
        // A non-string id is not an id, and must not be coerced into one.
        name: "assurance/build-case-a-numeric-id-is-not-an-id",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[{"path":"s.md","body":"","frontmatter":{"id":7,"type":"StR","title":"Coerced?"}},{"path":"t.md","body":"","frontmatter":{"id":"","type":"StR","title":"Empty"}}],"obligations":[],"findings":[]}"#,
        ),
    },
    Case {
        // The condition of carrying these two through as opaque values: a
        // non-trivial assessment, sorted by one key and emitted unchanged.
        // Without this the pass-through is a gate that cannot fail.
        name: "assurance/build-case-assessments-pass-through",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[],"obligations":[],"findings":[],"producer_trust":[{"id":"T-002","useId":"U-2","producer":"p","status":"accepted-with-limitations","permittedDecisions":["release"],"limitations":["only on linux"],"triggeredBy":[{"kind":"version-change","detail":{"from":"1","to":"2"}}],"owner":"o"},{"id":"T-001","useId":"U-1","producer":"p","status":"unobserved","permittedDecisions":[],"limitations":[],"triggeredBy":[],"owner":"o"}],"evidence_independence":[{"profile":"P","requirement":"FR-002","obligation":"FR-002-AC-1","status":"insufficient","dimensions":[{"dimension":"author","status":"insufficient"}],"summary":"same author"},{"profile":"P","requirement":"FR-001","obligation":"FR-001-AC-1","status":"satisfied","dimensions":[],"satisfiedBy":["a","b"],"summary":"ok"}]}"#,
        ),
    },
    Case {
        // Ties keep input order: a stable sort, like `Array.prototype.sort`.
        name: "assurance/build-case-equal-sort-keys-keep-input-order",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[],"obligations":[],"findings":[],"producer_trust":[{"id":"T-001","marker":"second-in"},{"id":"T-001","marker":"third-in"},{"id":"T-000","marker":"first-by-key"}]}"#,
        ),
    },
    Case {
        name: "assurance/build-case-unreadable-documents-are-carried",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[],"obligations":[],"findings":[],"unreadable":[{"path":"broken.md","reason":"unterminated frontmatter"}]}"#,
        ),
    },
    Case {
        name: "assurance/build-case-invalid-unknown-field",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[],"obligations":[],"findings":[],"claimTypes":["StR"]}"#,
        ),
    },
    Case {
        name: "assurance/build-case-invalid-missing-findings",
        op: "assurance.build_case",
        request: Request::Literal(r#"{"documents":[],"obligations":[]}"#),
    },
    Case {
        // Parity on malformed input, which is the half a well-formed-only
        // gate never reaches: serde refuses a numeric obligation id, so the
        // reference must too rather than coercing it.
        name: "assurance/build-case-invalid-obligation-id-not-a-string",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[],"obligations":[{"id":7,"statement":"s"}],"findings":[]}"#,
        ),
    },
    Case {
        name: "assurance/build-case-invalid-finding-missing-kind",
        op: "assurance.build_case",
        request: Request::Literal(
            r#"{"documents":[],"obligations":[],"findings":[{"obligation":"FR-001-AC-1","summary":"s"}]}"#,
        ),
    },
    Case {
        // The 16 MiB bound, from both directions. Until these existed the two
        // sides' agreement about refusing an oversize request was asserted by
        // nobody: the Rust unit test proved Rust refuses, and nothing proved
        // the reference refuses the same byte.
        name: "assurance/build-case-at-the-limit",
        op: "assurance.build_case",
        request: Request::BuildCaseOfBytes(16 * 1024 * 1024),
    },
    Case {
        name: "assurance/build-case-refused-over-the-limit",
        op: "assurance.build_case",
        request: Request::BuildCaseOfBytes(16 * 1024 * 1024 + 1),
    },
    Case {
        name: "assurance/build-case-invalid-malformed",
        op: "assurance.build_case",
        request: Request::Literal("{oops"),
    },
    Case {
        name: "assurance/build-case-invalid-non-object",
        op: "assurance.build_case",
        request: Request::Literal("[1,2]"),
    },
    // ── assurance.render_case (quoin#384) ──
    //
    // The renderer, against the retained `src/assurance/render.ts`. This
    // operation's input is `build_case`'s OUTPUT, and the three assessment
    // types that were opaque there are read field-by-field here — fourteen
    // fields across three types `render.ts` never names (quoin#425).
    Case {
        name: "assurance/render-no-case-is-not-an-empty-case",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"reason":"no document declares itself a top-level claim (searched claim types: StR); declare one or pass --claim-type","unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        name: "assurance/render-one-supported-claim",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"The system is usable","status":"supported","children":[{"id":"FR-001-AC-1","kind":"solution","statement":"It holds.","status":"supported","children":[]}]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // `because` renders as a nested `↳` line, and open propagates.
        name: "assurance/render-an-open-branch-carries-its-reason",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"The system is usable","status":"open","children":[{"id":"FR-001-AC-1","kind":"solution","statement":"It holds.","status":"open","because":"undischarged: nothing binds it","children":[]}]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // A claim with nothing under it: `because` on the claim itself, and
        // depth 0 contributes no bullet because the `##` heading is its line.
        name: "assurance/render-a-bare-claim-states-why-it-is-open",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"Nothing under it","status":"open","because":"no sub-claim and no obligation traces to this claim","children":[]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // Nesting depth, indentation, and the `([...])` vs `[...]` shapes.
        name: "assurance/render-nested-goals-indent-and-change-shape",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"Top","status":"open","children":[{"id":"FR-001","kind":"goal","statement":"Middle","status":"open","children":[{"id":"FR-001-AC-1","kind":"solution","statement":"Leaf","status":"open","because":"stale-evidence: old","children":[]}]}]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // Two claims, so the `N claim(s), M with an open branch` count is not
        // trivially 1 and 1.
        name: "assurance/render-counts-open-branches-across-claims",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"First","status":"open","because":"no sub-claim and no obligation traces to this claim","children":[]},{"id":"StR-002","kind":"goal","statement":"Second","status":"supported","children":[{"id":"FR-002-AC-1","kind":"solution","statement":"It holds.","status":"supported","children":[]}]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // The three mermaid punctuation rules, all in one statement.
        name: "assurance/render-mermaid-survives-punctuation",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"Top","status":"open","children":[{"id":"FR-001-AC-1","kind":"solution","statement":"Rejects (bad) input; logs \"why\" and `how`","status":"open","because":"x: y","children":[]}]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // quoin#432, from the Rust side. The retained renderer truncates on a
        // CODE POINT now; before #433 it counted UTF-16 units and this input
        // produced a lone high surrogate, which Rust's String cannot hold.
        // The port could not have matched the old behaviour at all.
        name: "assurance/render-truncates-on-a-code-point",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"Top","status":"supported","children":[{"id":"FR-001-AC-1","kind":"solution","statement":"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx😀 tail","status":"supported","children":[]}]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // `id.replace(/[^A-Za-z0-9]/g, "_")` matches UTF-16 CODE UNITS, so an
        // astral character in an id becomes TWO underscores. `chars()` was the
        // obvious port and would have produced one.
        name: "assurance/render-node-id-sanitises-per-code-unit",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"Top","status":"supported","children":[{"id":"FR-😀-AC-1","kind":"solution","statement":"Astral in the id","status":"supported","children":[]}]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // A node reached twice is declared once but gets both edges.
        name: "assurance/render-a-shared-child-is-declared-once",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"Top","status":"open","children":[{"id":"FR-001","kind":"goal","statement":"A","status":"open","children":[{"id":"FR-009-AC-1","kind":"solution","statement":"Shared","status":"open","because":"x: y","children":[]}]},{"id":"FR-002","kind":"goal","statement":"B","status":"open","children":[{"id":"FR-009-AC-1","kind":"solution","statement":"Shared","status":"open","because":"x: y","children":[]}]}]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        name: "assurance/render-unreachable-and-unreadable-sections",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"reason":"none","unreachable":["FR-009","FR-010"],"unreadable":[{"path":"broken.md","reason":"unterminated frontmatter"}],"producerTrust":[]}"#,
        ),
    },
    Case {
        // THE TYPE-SUBSET CASE. Five of TrustAssessment's eight fields are
        // rendered; the other three are present here and must not appear in
        // the output. `triggeredBy` is a STRING union, so `join(", ")` yields
        // prose — had it been objects it would render `[object Object]`.
        name: "assurance/render-producer-trust-fields",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"reason":"none","unreachable":[],"unreadable":[],"producerTrust":[{"id":"T-001","useId":"U-1","producer":"vitest","status":"accepted-with-limitations","permittedDecisions":["release"],"limitations":["linux only","x86 only"],"triggeredBy":["producer-version","configuration"],"owner":"qa"},{"id":"T-002","useId":"U-2","producer":"pytest","status":"unobserved","permittedDecisions":[],"limitations":[],"triggeredBy":[],"owner":"qa"}]}"#,
        ),
    },
    Case {
        // Six of IndependenceAssessment's seven, and all three of the nested
        // dimension type. `satisfiedBy` is present and must not render.
        name: "assurance/render-independence-fields",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"reason":"none","unreachable":[],"unreadable":[],"producerTrust":[],"evidenceIndependence":[{"profile":"P","requirement":"FR-001","obligation":"FR-001-AC-1","status":"satisfied","dimensions":[{"dimension":"author","values":["a","b"],"missingSuites":[]},{"dimension":"tooling","values":[],"missingSuites":["unit","e2e"]}],"satisfiedBy":["unit","integration"],"summary":"two authors"}]}"#,
        ),
    },
    Case {
        // `(x?.length ?? 0) > 0`: an EMPTY list skips the heading entirely.
        // A naive `is_some()` would emit a section with nothing under it.
        name: "assurance/render-empty-independence-emits-no-heading",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"reason":"none","unreachable":[],"unreadable":[],"producerTrust":[],"evidenceIndependence":[]}"#,
        ),
    },
    Case {
        // `evidenceIndependence` ABSENT rather than empty. The retained
        // renderer reaches it through `?.`, so absence is legitimate.
        name: "assurance/render-absent-independence",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"reason":"none","unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // `producerTrust` absent, which is NOT the same case. The retained
        // renderer reads `.length` on it directly and throws, so the boundary
        // must refuse rather than default it to empty — a port that renders
        // what the retained implementation cannot is as much a difference as
        // one that refuses what it accepts. The harness found this.
        name: "assurance/render-invalid-absent-producer-trust",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"reason":"none","unreachable":[],"unreadable":[]}"#,
        ),
    },
    Case {
        // An unknown status string. Same rule as `Finding.kind`: the renderer
        // interpolates and does not validate, so a closed enum here would
        // refuse input the retained implementation renders.
        name: "assurance/render-unknown-trust-status",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"reason":"none","unreachable":[],"unreadable":[],"producerTrust":[{"id":"T","useId":"U","status":"status-invented-tomorrow","triggeredBy":["trigger-invented-tomorrow"],"limitations":[]}]}"#,
        ),
    },
    Case {
        // `kind` and `status` on a CaseNode ARE closed — build_case mints
        // them, so an unrecognised one is a bad request on both sides.
        name: "assurance/render-invalid-node-kind",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"premise","statement":"Top","status":"open","children":[]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        name: "assurance/render-invalid-node-status",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"Top","status":"maybe","children":[]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        // `children` carries no serde default, so it is required at depth.
        name: "assurance/render-invalid-nested-node-missing-children",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[{"id":"StR-001","kind":"goal","statement":"Top","status":"open","children":[{"id":"FR-001","kind":"goal","statement":"No children key","status":"open"}]}],"unreachable":[],"unreadable":[],"producerTrust":[]}"#,
        ),
    },
    Case {
        name: "assurance/render-invalid-missing-unreachable",
        op: "assurance.render_case",
        request: Request::Literal(r#"{"claims":[],"unreadable":[],"producerTrust":[]}"#),
    },
    Case {
        // A rendered field missing from an assessment. `Option` would have
        // printed a blank row; required makes it a bad request.
        name: "assurance/render-invalid-trust-missing-use-id",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"unreachable":[],"unreadable":[],"producerTrust":[{"id":"T","status":"accepted","triggeredBy":[],"limitations":[]}]}"#,
        ),
    },
    Case {
        name: "assurance/render-invalid-dimension-missing-missing-suites",
        op: "assurance.render_case",
        request: Request::Literal(
            r#"{"claims":[],"unreachable":[],"unreadable":[],"producerTrust":[],"evidenceIndependence":[{"profile":"P","requirement":"FR-001","obligation":"FR-001-AC-1","status":"satisfied","dimensions":[{"dimension":"author","values":[]}],"summary":"s"}]}"#,
        ),
    },
    Case {
        name: "assurance/render-invalid-malformed",
        op: "assurance.render_case",
        request: Request::Literal("{oops"),
    },
    Case {
        name: "invalid/unknown-op",
        op: "evidence.record",
        request: Request::Literal("{}"),
    },
];

/// What one side produced.
struct Observed {
    stdout: String,
    diagnostics: Vec<(String, Vec<String>)>,
    status: i32,
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(core) = flag(&args, "--core") else {
        return usage("--core <path to quoin-core>");
    };
    let Some(ts) = flag(&args, "--ts") else {
        return usage("--ts <path to the TypeScript entry point>");
    };
    let node = flag(&args, "--node").unwrap_or_else(|| "node".to_owned());

    let mut failures = 0_usize;
    for case in CASES {
        let request = case.request.text();
        let rust = observe(Command::new(&core).arg(case.op), &request);
        let typescript = observe(Command::new(&node).arg(&ts).arg(case.op), &request);
        match (rust, typescript) {
            (Ok(rust), Ok(typescript)) => {
                let differences = compare(&rust, &typescript);
                if differences.is_empty() {
                    println!("  ok   {} [{}]", case.name, describe(rust.status));
                } else {
                    failures += 1;
                    println!("  DIFF {}", case.name);
                    for difference in differences {
                        println!("         {difference}");
                    }
                }
            }
            (rust, typescript) => {
                failures += 1;
                println!("  ERR  {}", case.name);
                for (side, result) in [("quoin-core", rust), ("typescript", typescript)] {
                    if let Err(message) = result {
                        println!("         {side} could not be run: {message}");
                    }
                }
            }
        }
    }

    println!("\n{} case(s), {failures} difference(s)", CASES.len());
    if failures == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

fn compare(rust: &Observed, typescript: &Observed) -> Vec<String> {
    let mut out = Vec::new();
    if rust.status != typescript.status {
        out.push(format!(
            "exit status: quoin-core {} ({}), typescript {} ({})",
            rust.status,
            describe(rust.status),
            typescript.status,
            describe(typescript.status)
        ));
    }
    if rust.stdout != typescript.stdout {
        out.push(format!(
            "canonical stdout:\n           quoin-core  {}\n           typescript  {}",
            show(&rust.stdout),
            show(&typescript.stdout)
        ));
    }
    if rust.diagnostics != typescript.diagnostics {
        out.push(format!(
            "diagnostic shape: quoin-core {:?}, typescript {:?}",
            rust.diagnostics, typescript.diagnostics
        ));
    }
    out
}

/// Run one side and reduce it to the three comparable facts.
fn observe(command: &mut Command, request: &str) -> Result<Observed, String> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .as_mut()
        .ok_or("stdin was not piped")?
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let stderr = String::from_utf8(output.stderr).map_err(|e| e.to_string())?;
    Ok(Observed {
        stdout: canonicalise(&stdout)?,
        diagnostics: normalise_diagnostics(&stderr)?,
        status: output
            .status
            .code()
            .ok_or("terminated by a signal, not a status")?,
    })
}

/// Re-canonicalise whatever a side wrote, so the comparison is about content.
///
/// Both sides are REQUIRED to emit canonical JSON, and the test suites on each
/// side assert that separately. Parsing here means a formatting difference is
/// reported by those tests, by name, instead of arriving as an unreadable
/// byte diff from this harness.
fn canonicalise(stdout: &str) -> Result<String, String> {
    if stdout.trim().is_empty() {
        return Ok(String::new());
    }
    let value: serde_json::Value = serde_json::from_str(stdout).map_err(|e| e.to_string())?;
    serde_json::to_string(&value).map_err(|e| e.to_string())
}

/// `(code, sorted context keys)` per diagnostic, in order. Messages dropped.
fn normalise_diagnostics(stderr: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    if stderr.trim().is_empty() {
        return Ok(Vec::new());
    }
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(stderr).map_err(|e| format!("stderr is not a JSON array: {e}"))?;
    entries
        .iter()
        .map(|entry| {
            let code = entry
                .get("code")
                .and_then(serde_json::Value::as_str)
                .ok_or("diagnostic has no code")?;
            let context = entry
                .get("context")
                .and_then(serde_json::Value::as_object)
                .map(|map| map.keys().cloned().collect())
                .unwrap_or_default();
            Ok((code.to_owned(), context))
        })
        .collect()
}

fn describe(status: i32) -> String {
    u8::try_from(status)
        .ok()
        .and_then(Outcome::from_code)
        .map_or_else(
            || format!("status {status} is outside the taxonomy"),
            |o| format!("{o:?}"),
        )
}

fn show(text: &str) -> &str {
    if text.is_empty() {
        "<no payload>"
    } else {
        text
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn usage(missing: &str) -> std::process::ExitCode {
    eprintln!("quoin-difftest: missing {missing}");
    eprintln!("usage: quoin-difftest --core <path> --ts <path> [--node <node>]");
    std::process::ExitCode::from(2)
}
