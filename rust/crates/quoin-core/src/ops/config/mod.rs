// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `config`: resolving the authoring organization (quoin#446, Stage 7).
//!
//! Replaces `src/org.ts`. The TypeScript that stood here decided four things —
//! the precedence of flag over environment over stored config over git remote,
//! the layering and strict-schema validation of the stored config, which
//! `[remote "origin"]` url shapes name an org, and what "nobody said" means —
//! and every one of them is now decided in Rust.
//!
//! # No host capability, and how
//!
//! This module opens nothing. `config.resolve_org` is given the bytes of the
//! two config layers and of `.git/config`, and decides over them via
//! `quoin_config::resolve_org_from_documents`, which is the filesystem-free
//! twin of the `resolve_org` the CLI path uses. `tc_446_026` in `quoin-config`
//! pins the two against each other over a real temporary tree, so "the pure
//! path" cannot quietly become "a second implementation". See
//! `crate::capabilities` for the whole rule.
//!
//! The caller-side reads that remain — locating a worktree's git directory and
//! reading three small files — live in `src/core/org.ts`. That is deliberate
//! and it is not a leftover: a `.git` pointer chain is host state, and the
//! boundary's contract is that host state is acquired by the shell and decided
//! on by the library.
//!
//! [`taxonomy`] holds the one place a `quoin_config::ConfigError` becomes an
//! exit status, the twin of `ops::modules::taxonomy`, and the count pin that
//! makes a code added upstream fail a test rather than fall into a wildcard.

mod taxonomy;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use quoin_config::{
    FixedEnvironment, OrgDocumentReport, OrgOptions, ResolvedOrg, resolve_org_from_documents,
};

use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

/// The largest `.git/config` this domain will decide over, in bytes.
///
/// The same ceiling `quoin_config::org_from_git_config` applies when it reads
/// the file itself, and deliberately the same NUMBER: a repository whose config
/// the CLI path would refuse to read must not become resolvable by routing the
/// same bytes through the boundary instead. It is restated rather than cast,
/// because the crate constant is a `u64` file length and this is an in-memory
/// byte count; `the_bounds_are_the_crates_own_ceilings` pins the two equal, so
/// a change upstream fails here rather than widening the boundary silently.
pub const MAX_GIT_CONFIG_BYTES: usize = 4 << 20;

/// The largest config layer document this domain will decide over, in bytes.
///
/// `quoin_config::service::MAX_CONFIG_FILE_BYTES` is 1 MiB for the file the
/// service reads; the same number applies here, per layer, for the same reason.
/// A config file is a handful of scalar keys, and stdin is an untrusted stream
/// (rust-style §"Untrusted input"). Pinned equal to the crate constant by
/// `the_bounds_are_the_crates_own_ceilings`.
pub const MAX_CONFIG_LAYER_BYTES: usize = 1 << 20;

/// The largest `--org` value, repository root or environment value this domain
/// will accept, in bytes.
///
/// An org name is under 40 bytes and a path under 4 KiB on every platform quoin
/// ships for. The bound refuses a stream rather than truncating one.
pub const MAX_SCALAR_BYTES: usize = 4 * 1024;

/// The largest number of environment entries a request may carry.
///
/// `QUOIN_ENV_BINDINGS` declares one binding today. A caller that forwards its
/// whole environment is forwarding a few hundred entries; 4096 is far past that
/// and still refuses an unbounded map.
pub const MAX_ENV_ENTRIES: usize = 4096;

/// The request accepted by `config.resolve_org`.
///
/// Every field is state the caller already holds. Nothing here is a path this
/// operation would open.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ResolveOrgRequest {
    /// The explicit `--org` value, when one was passed.
    #[serde(default)]
    pub flag: Option<String>,
    /// The environment the resolution reads, supplied rather than read.
    ///
    /// `QUOIN_ORG` is a *declared binding* (`QUOIN_ENV_BINDINGS`), so it is
    /// layered over the config document by the schema machinery rather than
    /// consulted directly — one precedence rule in one place, which is the
    /// property `src/org.ts` went out of its way to keep and this preserves.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// The user-level config document, absent when the file does not exist.
    #[serde(default)]
    pub user_config: Option<String>,
    /// The project-level `.ix` config document, absent when no project layer
    /// applies or the file does not exist.
    #[serde(default)]
    pub project_config: Option<String>,
    /// The repository's `.git/config`, absent when there is none to read.
    #[serde(default)]
    pub git_config: Option<String>,
}

/// The payload `config.resolve_org` writes to stdout.
///
/// Field-for-field `ResolvedOrg` in the deleted `src/org.ts`, so the caller in
/// `src/core/org.ts` is a rename and not a reshaping: `org` is omitted rather
/// than null when unresolved, exactly as the TypeScript `org?: string` was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ResolveOrgPayload {
    /// The organization, omitted when nothing yielded one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    /// Which source won: `flag`, `env`, `config`, `git` or `none`.
    pub source: String,
    /// Whether a config layer fell back rather than contributing its content.
    ///
    /// New at the boundary, and not a widening of the contract: `src/org.ts`
    /// discarded this fact silently, which is why a malformed config file
    /// resolved to "no stored org" with nothing to show for it. It rides the
    /// payload; the run is still a success, because a broken config must not
    /// stop an author writing specs (FR-027-AC-5).
    pub degraded: bool,
}

/// The message shown when no source yielded an organization.
///
/// Served over the boundary so the sentence has ONE home. It was a `const` in
/// `src/org.ts` and a `const` in `quoin_config::org`, and two copies of a
/// user-facing sentence is exactly the drift FR-101 retires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UnresolvedOrgMessagePayload {
    /// The sentence.
    pub message: String,
}

/// Answer a `config.unresolved_org_message`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin carries any field at all.
/// - [`CoreErrorCode::Io`] when the payload cannot be serialised.
pub fn unresolved_org_message(request: &serde_json::Value) -> Result<Response, CoreError> {
    let _: EmptyRequest = serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string())
            .with_context("op", "config.unresolved_org_message")
    })?;
    let payload = serde_json::to_value(UnresolvedOrgMessagePayload {
        message: quoin_config::UNRESOLVED_ORG_MESSAGE.to_owned(),
    })
    .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;
    Ok(Response::ok(payload))
}

/// A request with no arguments.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EmptyRequest {}

/// Answer a `config.resolve_org`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`ResolveOrgRequest`].
/// - [`CoreErrorCode::Refused`] when any bounded field exceeds its ceiling.
/// - [`CoreErrorCode::Io`] when the payload cannot be serialised.
pub fn resolve_org(request: &serde_json::Value) -> Result<Response, CoreError> {
    let request: ResolveOrgRequest = serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string())
            .with_context("op", "config.resolve_org")
    })?;

    check_bound("flag", request.flag.as_deref(), MAX_SCALAR_BYTES)?;
    check_bound(
        "user_config",
        request.user_config.as_deref(),
        MAX_CONFIG_LAYER_BYTES,
    )?;
    check_bound(
        "project_config",
        request.project_config.as_deref(),
        MAX_CONFIG_LAYER_BYTES,
    )?;
    check_bound(
        "git_config",
        request.git_config.as_deref(),
        MAX_GIT_CONFIG_BYTES,
    )?;
    if request.env.len() > MAX_ENV_ENTRIES {
        return Err(refusal("env", MAX_ENV_ENTRIES, request.env.len()));
    }
    for (name, value) in &request.env {
        check_bound("env_value", Some(value.as_str()), MAX_SCALAR_BYTES)
            .map_err(|e| e.with_context("env_name", name.clone()))?;
    }

    let mut environment = FixedEnvironment::new();
    for (name, value) in &request.env {
        environment = environment.with_var(name.clone(), value.clone());
    }

    let (resolved, report) = resolve_org_from_documents(
        &OrgOptions {
            flag: request.flag.as_deref(),
        },
        &environment,
        request.user_config.as_deref(),
        request.project_config.as_deref(),
        request.git_config.as_deref(),
    );

    let payload = serde_json::to_value(payload_of(&resolved, &report))
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    // A degraded read is a SUCCESS carrying a diagnostic, not a failure:
    // `src/org.ts` swallowed the problem entirely, and the contract this
    // preserves (FR-027-AC-5) is that a malformed config never stops an author.
    // Exit 1 is the status that says "complete payload, and something to say"
    // — the case `runCoreAllowFailure` exists to distinguish (quoin#103).
    //
    // The condition is the resolver's own `degraded` flag, not `!issues
    // .is_empty()`: the flag is what the payload reports, and a payload saying
    // `degraded: true` with an exit 0 and no diagnostic would be the boundary
    // stating a problem in a field while its own exit status denied it.
    if !report.degraded && report.issues.is_empty() {
        Ok(Response::ok(payload))
    } else {
        let mut diagnostic = CoreError::new(
            CoreErrorCode::Degraded,
            "a config layer did not contribute its content; schema defaults were used",
        )
        .with_context("op", "config.resolve_org")
        .with_context("issue_count", report.issues.len().to_string());
        for (index, issue) in report.issues.iter().enumerate() {
            diagnostic = diagnostic.with_context(
                format!("issue_{index}"),
                format!(
                    "{}: expected {}, {}",
                    if issue.key_path.is_empty() {
                        "<root>"
                    } else {
                        &issue.key_path
                    },
                    issue.expected,
                    issue.message
                ),
            );
        }
        Ok(Response::partial(payload, &diagnostic))
    }
}

/// Build the wire payload from a resolution.
fn payload_of(resolved: &ResolvedOrg, report: &OrgDocumentReport) -> ResolveOrgPayload {
    ResolveOrgPayload {
        org: resolved.org.as_ref().map(|org| org.as_str().to_owned()),
        // `OrgSource::as_str` and not a spelling restated here: these five
        // strings are what `src/write.ts` keys `ORG_SOURCE_LABEL` on, so they
        // have exactly one home.
        source: resolved.source.as_str().to_owned(),
        // The resolver's flag, carried rather than re-derived: see
        // `quoin_config::OrgDocumentReport`.
        degraded: report.degraded,
    }
}

/// Refuse an oversized field before any work is done on it.
fn check_bound(field: &str, value: Option<&str>, limit: usize) -> Result<(), CoreError> {
    match value {
        Some(text) if text.len() > limit => Err(refusal(field, limit, text.len())),
        _ => Ok(()),
    }
}

/// The one refusal shape this domain uses.
fn refusal(field: &str, limit: usize, observed: usize) -> CoreError {
    CoreError::new(
        CoreErrorCode::Refused,
        "a request field exceeds the accepted size",
    )
    .with_context("op", "config.resolve_org")
    .with_context("field", field.to_owned())
    .with_context("limit_bytes", limit.to_string())
    .with_context("observed_bytes", observed.to_string())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use quoin_config::OrgSource;

    use super::*;

    fn payload(response: &Response) -> &serde_json::Value {
        &response.payload
    }

    /// The four sources, in the precedence order `src/org.ts` documented, each
    /// asserted while the LOWER-precedence sources are all present and would
    /// each have answered differently. A precedence test in which only one
    /// source is populated proves nothing about precedence.
    #[test]
    fn precedence_is_flag_then_env_then_config_then_git() {
        let git = "[remote \"origin\"]\n\turl = git@github.com:from-git/repo.git\n";
        let stored = "org: from-config\n";

        let all_present = |flag: Option<&str>, env_org: Option<&str>| {
            let mut env = BTreeMap::new();
            if let Some(value) = env_org {
                env.insert("QUOIN_ORG".to_owned(), value.to_owned());
            }
            let request = serde_json::to_value(serde_json::json!({
                "flag": flag,
                "env": env,
                "user_config": stored,
                "git_config": git,
            }))
            .unwrap();
            let response = resolve_org(&request).unwrap();
            (
                payload(&response)["org"].as_str().map(str::to_owned),
                payload(&response)["source"].as_str().unwrap().to_owned(),
            )
        };

        assert_eq!(
            all_present(Some("from-flag"), Some("from-env")),
            (Some("from-flag".to_owned()), "flag".to_owned())
        );
        assert_eq!(
            all_present(None, Some("from-env")),
            (Some("from-env".to_owned()), "env".to_owned())
        );
        assert_eq!(
            all_present(None, None),
            (Some("from-config".to_owned()), "config".to_owned())
        );

        let request = serde_json::json!({ "git_config": git });
        let response = resolve_org(&request).unwrap();
        assert_eq!(payload(&response)["org"], "from-git");
        assert_eq!(payload(&response)["source"], "git");
    }

    /// `org` is OMITTED, not null. `src/write.ts` spreads
    /// `...(org ? { org } : {})`, so a `null` would render "Org: null".
    #[test]
    fn an_unresolved_org_omits_the_key_rather_than_nulling_it() {
        let response = resolve_org(&serde_json::json!({})).unwrap();
        assert_eq!(
            crate::protocol::canonical_json(payload(&response)).unwrap(),
            r#"{"degraded":false,"source":"none"}"#
        );
    }

    /// A whitespace-only flag is not a flag. The TypeScript trimmed before
    /// testing truthiness, so `--org "   "` fell through to the next source.
    #[test]
    fn a_blank_flag_falls_through_rather_than_winning_empty() {
        let request = serde_json::json!({
            "flag": "   ",
            "user_config": "org: stored\n",
        });
        let response = resolve_org(&request).unwrap();
        assert_eq!(payload(&response)["org"], "stored");
        assert_eq!(payload(&response)["source"], "config");
    }

    /// The project layer outranks the user layer, and both are validated as ONE
    /// merged document — an unknown key in EITHER invalidates the whole config,
    /// which is what the strict schema means.
    #[test]
    fn the_project_layer_wins_and_strictness_spans_the_merge() {
        let response = resolve_org(&serde_json::json!({
            "user_config": "org: user\n",
            "project_config": "org: project\n",
        }))
        .unwrap();
        assert_eq!(payload(&response)["org"], "project");

        let response = resolve_org(&serde_json::json!({
            "user_config": "org: user\n",
            "project_config": "nope: 1\n",
        }))
        .unwrap();
        assert_eq!(payload(&response)["degraded"], true);
        assert!(payload(&response)["org"].is_null());
        assert_eq!(response.outcome.code(), 1);
    }

    /// A malformed config is a COMPLETE payload with a diagnostic — exit 1, the
    /// `Partial` case — not a failure. An author must keep writing specs
    /// (FR-027-AC-5), and the boundary must still say what went wrong.
    #[test]
    fn a_malformed_layer_is_partial_not_refused() {
        let response = resolve_org(&serde_json::json!({
            "user_config": "org: [unclosed\n",
            "git_config": "[remote \"origin\"]\n\turl = https://h/o/r.git\n",
        }))
        .unwrap();
        assert_eq!(response.outcome.code(), 1);
        assert!(response.outcome.carries_payload());
        assert_eq!(payload(&response)["degraded"], true);
        // The git remote still answers: a broken config file does not stop
        // resolution, it only removes one source from it.
        assert_eq!(payload(&response)["org"], "o");
        assert_eq!(response.diagnostics.len(), 1);
        assert_eq!(response.diagnostics[0].code, "CORE_DEGRADED");
    }

    /// A misspelled field is refused by name, not silently dropped.
    #[test]
    fn an_unknown_field_is_bad_request_not_ignored() {
        let error = resolve_org(&serde_json::json!({ "user_conifg": "org: x" })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
        assert_eq!(error.outcome().code(), 3);
        assert_eq!(error.context["op"], "config.resolve_org");
    }

    /// Every bound is stated in BYTES and refused BEFORE the work, and each is
    /// checked at its own ceiling rather than one standing in for the rest.
    #[test]
    fn each_bound_refuses_at_its_own_ceiling() {
        for (field, limit) in [
            ("flag", MAX_SCALAR_BYTES),
            ("user_config", MAX_CONFIG_LAYER_BYTES),
            ("project_config", MAX_CONFIG_LAYER_BYTES),
            ("git_config", MAX_GIT_CONFIG_BYTES),
        ] {
            let mut request = serde_json::Map::new();
            request.insert(
                field.to_owned(),
                serde_json::Value::String("x".repeat(limit + 1)),
            );
            let error = resolve_org(&serde_json::Value::Object(request)).unwrap_err();
            assert_eq!(error.code, CoreErrorCode::Refused, "{field}");
            assert_eq!(error.outcome().code(), 2, "{field}");
            assert_eq!(error.context["field"], field);
            assert_eq!(error.context["limit_bytes"], limit.to_string());
            assert_eq!(error.context["observed_bytes"], (limit + 1).to_string());
        }
    }

    /// Bytes, not characters. `"é"` is one JavaScript string unit and two UTF-8
    /// bytes, so a limit measured in `.length` would make the boundary's
    /// behaviour depend on the caller's alphabet.
    #[test]
    fn the_bound_is_bytes_not_characters() {
        let at_limit = "é".repeat(MAX_SCALAR_BYTES / 2);
        assert_eq!(at_limit.len(), MAX_SCALAR_BYTES);
        assert_eq!(at_limit.chars().count(), MAX_SCALAR_BYTES / 2);
        assert!(resolve_org(&serde_json::json!({ "flag": at_limit })).is_ok());

        let over = format!("{at_limit}é");
        let error = resolve_org(&serde_json::json!({ "flag": over })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Refused);
        assert_eq!(
            error.context["observed_bytes"],
            (MAX_SCALAR_BYTES + 2).to_string()
        );
    }

    /// The sentence crosses the boundary rather than being re-typed on the
    /// TypeScript side. Asserted against the crate constant AND against a
    /// literal fragment, so a test that only compares the value to itself
    /// cannot pass while the sentence is empty.
    #[test]
    fn the_unresolved_message_is_served_not_restated() {
        let response = unresolved_org_message(&serde_json::json!({})).unwrap();
        let served = response.payload["message"].as_str().unwrap();
        assert_eq!(served, quoin_config::UNRESOLVED_ORG_MESSAGE);
        assert!(served.starts_with("could not determine the authoring organization"));
        assert!(served.contains("quoin config set org <name>"));

        let error = unresolved_org_message(&serde_json::json!({ "x": 1 })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
    }

    /// The five wire spellings `src/write.ts` keys `ORG_SOURCE_LABEL` on,
    /// written out as literals rather than re-derived from the enum — a test
    /// that maps `as_str` over the variants only proves `as_str` agrees with
    /// itself.
    #[test]
    fn the_source_spellings_are_the_ones_the_pack_renders() {
        assert_eq!(
            [
                OrgSource::Flag,
                OrgSource::Env,
                OrgSource::Config,
                OrgSource::Git,
                OrgSource::None,
            ]
            .map(OrgSource::as_str),
            ["flag", "env", "config", "git", "none"]
        );
    }

    /// The boundary's ceilings ARE the crate's ceilings. Restated as `usize`
    /// literals above for the in-memory count; pinned here so a change to
    /// either number fails rather than silently letting the boundary accept
    /// what the CLI path would refuse.
    #[test]
    fn the_bounds_are_the_crates_own_ceilings() {
        assert_eq!(
            u64::try_from(MAX_GIT_CONFIG_BYTES).unwrap(),
            quoin_config::MAX_GIT_CONFIG_BYTES
        );
        assert_eq!(
            u64::try_from(MAX_CONFIG_LAYER_BYTES).unwrap(),
            quoin_config::service::MAX_CONFIG_FILE_BYTES
        );
    }
}
