// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one ajv-parity JSON Schema validator in this workspace.
//!
//! # Why this crate exists
//!
//! `quoin-semantic` built an ajv-shaped adapter over the Rust `jsonschema`
//! crate for quoin#378: a `Draft202012` validator, an in-memory `Registry`, a
//! contractual `is_valid` verdict, and an `errors()` projection that reproduces
//! ajv's `instancePath` / `keyword` / `params` shape so the diagnostic mapping
//! written against ajv keeps working. When the measurement port (quoin#470)
//! became the second consumer of exactly that adapter, the choice was to copy
//! it or to move it. A copied validator is two validators, and this workspace
//! has already paid for that with six hand-rolled instant readers and two
//! RFC 3339 grammars.
//!
//! So it moved here, unchanged. `quoin_semantic::schema` re-exports every type
//! it used to define, and that crate's tests run against the re-export without
//! an edit.
//!
//! # What it owns
//!
//! - [`keyword`] — ajv's keyword vocabulary, [`SchemaKeyword`].
//! - [`validator`] — [`SchemaValidator`], the `jsonschema`-to-ajv error
//!   adapter, and the `format` policy.
//! - [`vendored`] — the committed measurement schema documents and
//!   [`VendoredValidator::parse`], which returns a [`ValidDocument`] rather
//!   than a `bool`.
//! - [`error`] — one `thiserror` enum with stable codes.
//!
//! # What it must never own
//!
//! An RFC 3339 grammar. `quoin_measurement::date_time` is the one grammar in
//! the measurement domain, and [`VendoredSchema::compile`] takes the
//! `date-time` check as a parameter so that this crate can sit *below*
//! `quoin-measurement` without either duplicating it or depending back on it.
//!
//! # Verdicts are contractual, error text is not
//!
//! ajv and `jsonschema` agree on whether a document satisfies a schema, and
//! that agreement is measured on a named corpus in `tests/tc_470_parity.rs`.
//! They do not agree on message wording, on how many errors one refusal
//! produces, or on the order they arrive in; the three reshapings that close
//! the gap are documented in [`validator`] and in `DIVERGENCE.md`.

#![forbid(unsafe_code)]

pub mod error;
pub mod keyword;
pub mod validator;
pub mod vendored;

pub use error::{JsonSchemaError, JsonSchemaErrorCode};
pub use keyword::SchemaKeyword;
pub use validator::{
    FormatCheck, SchemaError, SchemaErrorParams, SchemaValidator, identity_counts,
};
pub use vendored::{ValidDocument, VendoredSchema, VendoredValidator};
