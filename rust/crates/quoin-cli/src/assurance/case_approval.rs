// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Offline verification of a caller-selected approval over the canonical GSN case.

use std::path::{self, Path};
use std::process::Output;

use clap::{Arg, ArgMatches};
use quoin_assurance::gsn::{GsnCase, SignedAcceptance};
use quoin_core::protocol::canonical_json;
use quoin_core::runtime::engineering_assurance_call_offline;
use quoin_store::attachment::sha256_hex;
use quoin_store::instant::{Instant, InstantGrammar};
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const TAG: &str = "case-approval-tag";
const VERIFIER: &str = "case-approval-verifier";
const ALLOWED_SIGNERS: &str = "case-approval-allowed-signers";
const REQUEST_PROTOCOL: &str = "engineering-assurance.approval-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.approval-verify-result/v1";
const RECORD_ID: &str = "quoin.gsn-case/v1";
const CASE_MEANING: &str = "assurance-case-accepted";

pub(super) fn options() -> [Arg; 3] {
    [
        Arg::new(TAG)
            .long(TAG)
            .value_name("TAG")
            .help("Verify one signed annotated tag over the GSN case."),
        Arg::new(VERIFIER)
            .long(VERIFIER)
            .value_name("ABSOLUTE-PATH")
            .help("Engineering Assurance executable for case-approval verification."),
        Arg::new(ALLOWED_SIGNERS)
            .long(ALLOWED_SIGNERS)
            .value_name("SOURCE")
            .help("Case-specific SSH allowed-signers file or committed <rev>:<path>."),
    ]
}

pub(super) fn supplied(args: &ArgMatches) -> bool {
    [TAG, VERIFIER, ALLOWED_SIGNERS]
        .iter()
        .any(|key| args.get_one::<String>(key).is_some())
}

pub(super) fn attach(model: &mut GsnCase, repo: &Path, args: &ArgMatches) -> Option<String> {
    let tag = args.get_one::<String>(TAG)?;
    match verify(model, repo, args, tag) {
        Ok(acceptance) => {
            model.signed_acceptance = Some(acceptance);
            None
        }
        Err(error) => Some(format!("case approval {tag:?}: {error}")),
    }
}

#[derive(Serialize)]
struct Request<'a> {
    protocol: &'static str,
    repository: &'a Path,
    tag: &'a str,
    record_id: &'static str,
    record_digest: &'a str,
    allowed_signers: &'a str,
}

fn verify(
    model: &GsnCase,
    repo: &Path,
    args: &ArgMatches,
    tag: &str,
) -> Result<SignedAcceptance, String> {
    let verifier = args
        .get_one::<String>(VERIFIER)
        .map(Path::new)
        .ok_or("--case-approval-verifier is required with a selected tag")?;
    if !verifier.is_absolute() {
        return Err("--case-approval-verifier must be an absolute executable path".to_owned());
    }
    let trust = args
        .get_one::<String>(ALLOWED_SIGNERS)
        .filter(|source| !source.is_empty())
        .ok_or("--case-approval-allowed-signers is required with a selected tag")?;
    let absolute_repo = path::absolute(repo)
        .map_err(|error| format!("cannot resolve the case repository: {error}"))?;
    let canonical = canonical_json(model)
        .map_err(|error| format!("cannot encode the unsigned case: {error}"))?;
    let digest = format!("sha256:{}", sha256_hex(canonical.as_bytes()));
    let request = Request {
        protocol: REQUEST_PROTOCOL,
        repository: &absolute_repo,
        tag,
        record_id: RECORD_ID,
        record_digest: &digest,
        allowed_signers: trust,
    };
    let request_json = serde_json::to_value(&request).map_err(|error| error.to_string())?;
    let output = engineering_assurance_call_offline(verifier, "approval-verify", &request_json)?;
    approved(output, &request)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Approved {
    protocol: String,
    status: String,
    tag: String,
    record_id: String,
    record_digest: String,
    trust: ApprovedTrust,
    binding: Binding,
    signature: Signature,
    approver: String,
    meaning: Vec<String>,
    signature_kind: String,
    signed_at: Option<String>,
    signed_at_source: Option<String>,
    signed_at_missing: Option<String>,
    trusted_at_missing: String,
    #[serde(rename = "tag_object_id")]
    _tag_object_id: Option<IgnoredAny>,
    #[serde(rename = "gesture")]
    _gesture: Option<IgnoredAny>,
    #[serde(rename = "ea_version")]
    _ea_version: Option<IgnoredAny>,
    #[serde(rename = "ea_source_revision")]
    _ea_source_revision: Option<IgnoredAny>,
    #[serde(rename = "generated_at")]
    _generated_at: Option<IgnoredAny>,
    #[serde(rename = "generated_at_source")]
    _generated_at_source: Option<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovedTrust {
    allowed_signers: String,
    #[serde(rename = "allowed_signers_sha256")]
    _allowed_signers_sha256: Option<IgnoredAny>,
    #[serde(rename = "allowed_signers_commit")]
    _allowed_signers_commit: Option<IgnoredAny>,
    #[serde(rename = "ea_version")]
    _ea_version: Option<IgnoredAny>,
    #[serde(rename = "git_version")]
    _git_version: Option<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    record_id: String,
    record_digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Signature {
    status: String,
    kind: String,
    signature_kind: String,
    signer: Signer,
    #[serde(rename = "verified_commit_id")]
    _verified_commit_id: Option<IgnoredAny>,
    #[serde(rename = "signed_off_meaning")]
    _signed_off_meaning: Option<IgnoredAny>,
    #[serde(rename = "gesture")]
    _gesture: Option<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Signer {
    source: String,
    mapping: String,
    principal: String,
}

fn approved(output: Output, request: &Request<'_>) -> Result<SignedAcceptance, String> {
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
    if !output.status.success() {
        let message = summary
            .get("code")
            .and_then(Value::as_str)
            .or_else(|| summary.get("message").and_then(Value::as_str));
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = message.unwrap_or_else(|| stderr.trim());
        return Err(format!("EA returned {}: {detail}", output.status));
    }
    if summary.get("protocol").and_then(Value::as_str) != Some(RESULT_PROTOCOL) {
        return Err("EA returned an unsupported or malformed approval result".to_owned());
    }
    if let Some(status) = summary.get("status").and_then(Value::as_str)
        && status != "approved"
    {
        let reason = summary
            .get("reason")
            .and_then(Value::as_str)
            .or_else(|| summary.pointer("/signature/reason").and_then(Value::as_str));
        return Err(match reason {
            Some(reason) => format!("EA status {status}: {reason}"),
            None => format!("EA status {status}"),
        });
    }
    let result: Approved = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("malformed EA approval result: {error}"))?;
    result.accept(request)
}

impl Approved {
    fn accept(self, request: &Request<'_>) -> Result<SignedAcceptance, String> {
        let expected_tag = request
            .tag
            .strip_prefix("refs/tags/")
            .unwrap_or(request.tag);
        if self.protocol != RESULT_PROTOCOL
            || self.status != "approved"
            || self.tag != expected_tag
            || self.record_id != request.record_id
            || self.record_digest != request.record_digest
            || self.trust.allowed_signers != request.allowed_signers
            || self.binding.record_id != "match"
            || self.binding.record_digest != "match"
        {
            return Err(
                "EA status approved but approval subject or trust does not match the request"
                    .to_owned(),
            );
        }

        let signer = self.signature.signer;
        if self.signature.status != "signed_valid"
            || self.signature.kind != "tag"
            || self.signature.signature_kind != "ssh"
            || self.signature_kind != "ssh"
            || signer.source != "allowed_signers"
            || signer.mapping != "mapped"
            || signer.principal != self.approver
            || self.approver.trim().is_empty()
            || !is_export_text(&self.approver)
            || !is_export_text(&self.tag)
        {
            return Err(
                "EA status approved but the result does not identify a mapped SSH approver"
                    .to_owned(),
            );
        }

        if self.meaning.as_slice() != [CASE_MEANING] {
            return Err("EA status approved but signed meaning must be exactly one assurance-case-accepted value".to_owned());
        }
        let signing_time = (
            self.signed_at.as_deref(),
            self.signed_at_source.as_deref(),
            self.signed_at_missing.as_deref(),
        );
        let valid_time = match signing_time {
            (Some(instant), Some("tagger_date"), None) => {
                instant.len() == 20
                    && instant.ends_with('Z')
                    && Instant::parse(instant, InstantGrammar::CalendarInstant).is_some()
            }
            (None, None, Some("tagger_date_unreadable")) => true,
            _ => false,
        };
        if !valid_time || self.trusted_at_missing != "ssh_signature" {
            return Err(
                "EA status approved but the offline SSH signing-time fields are invalid".to_owned(),
            );
        }

        Ok(SignedAcceptance {
            label: SignedAcceptance::LABEL.to_owned(),
            record_id: self.record_id,
            record_digest: self.record_digest,
            tag: self.tag,
            approver: self.approver,
            meaning: self.meaning,
            signer_source: signer.source,
            signed_at: self.signed_at,
            signed_at_source: self.signed_at_source,
            signed_at_missing: self.signed_at_missing,
            trusted_at_missing: self.trusted_at_missing,
        })
    }
}

fn is_export_text(value: &str) -> bool {
    value
        .chars()
        .all(|ch| !ch.is_control() && !matches!(ch, '\u{fffe}' | '\u{ffff}'))
}
