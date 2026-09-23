// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The criterion-strength lens's Jev (`typesafe.ai` System One) client wiring
//! (PLAT-837).
//!
//! This crate is the "client wiring and the lens execution path" surface of
//! PLAT-837: given an FR's spec text ([`context::FrContext`]) and the
//! question vocabulary the sibling skill owns
//! (`skills/spec-criterion-strength-analysis/assets/question-set.json`,
//! read unchanged through [`question_set::QuestionSet`]), it builds one
//! batched Jev request per FR ([`lens::build_request`]), sends it through a
//! [`typesafe_sdk_client::Client`] ([`client`]), and turns the response into
//! typed findings ([`verdict::extract`]) a caller renders into a
//! `SpecReview` document body ([`report::render`]).
//!
//! [`span_ballot`] (PLAT-980) is the reusable piece for a lens that needs
//! Jev to point at part of a text: candidate spans generated in Rust,
//! offered as one closed `choice`, and any label not issued rejected.
//!
//! It does not itself decide an AC, author a requirement, or write a matrix
//! row -- the ticket's "findings only" rule holds by construction: nothing
//! in this crate's public API takes a spec document path to modify.
//!
//! # Async, and why there is no runtime in this crate
//!
//! Every network-facing function here is `async fn`, because
//! `typesafe-sdk-client 0.6.2`'s own `Client::system_one`/`system_one_with`
//! are. This crate does not itself build a tokio `Runtime` or call
//! `block_on`: `quoin-cli`'s existing surface is entirely synchronous
//! (`reqwest::blocking`, see `quoin-delivery`), and this PR does not add a
//! CLI subcommand -- see this crate's PR description for why. Whoever wires
//! a CLI entry point onto this crate owns the sanctioned async/blocking
//! bridge at that boundary (`rust-review`'s own guidance: "check the
//! sanctioned bridge the repo uses"), rather than this library picking one
//! preemptively for a caller that does not exist yet.
//!
//! # Exercising this crate without a key or the network
//!
//! Every test in this crate's modules runs against
//! [`typesafe_sdk_http::Mock`] (the scripted transport shipped inside the
//! pinned `typesafe-sdk-http` crate itself -- see `client.rs`'s module doc
//! for a correction of a claim in this ticket's brief about where that seam
//! lives) and a [`typesafe_sdk_env::Fixed`] environment. None opens a socket
//! and none reads the real `TYPESAFE_API_KEY`. A test that needs a live key
//! does not exist in this crate; per the ticket's own instruction, that is a
//! property to keep, not relax once a key exists.

pub mod cassette;
pub mod client;
pub mod config;
pub mod context;
pub mod corpus_check;
pub mod error;
pub mod lens;
mod model_pin;
pub mod question_set;
pub mod report;
pub mod schema_gate;
pub mod span_ballot;
pub mod verdict;

pub use cassette::Cassette;
pub use context::{AcRow, Bound, BoundedContext, ContextPolicy, FrContext};
pub use corpus_check::{
    AdequacyFinding, check_answerability, check_required_fields, check_stated_counts,
};
pub use error::{JevError, JevErrorCode, Result};
pub use question_set::QuestionSet;
pub use report::{FrReport, render as render_findings};
pub use span_ballot::{BallotOutcome, Granularity, Span, SpanBallot, candidate_spans};
pub use verdict::{
    Certainty, CoverageVerdict, Finding, FrVerdict, Severity, SubQuestionCheck, Thresholds,
};
