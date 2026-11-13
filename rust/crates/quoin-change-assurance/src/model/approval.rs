// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The claimed approval a review decision may carry (FR-063, FR-065).
//!
//! # A claim, never a check
//!
//! A [`VerifiedApproval`] is a projection of Engineering Assurance's
//! `approval-verify` result, written into the decision payload by whoever wrote
//! the event. Nothing binds it to a run of that command, and this crate never
//! verifies a signature, never runs Git and never contacts an identity
//! provider: the claim has the standing of the recorded actor label. The
//! relying party re-runs `approval-verify` with trust it supplies itself
//! (FR-068); `trust` in the claim records what the writer used and decides
//! nothing here.
//!
//! Both shapes are closed. The payload form ([`VerifiedApproval`]) keeps the
//! result's status and rejects any member outside the row its status allows;
//! the receipt form ([`ClaimedApproval`]) carries only the fields of an
//! `approved` result.

use quoin_store::JsonValue;

use crate::error::{ChangeAssuranceError, FieldFailure, Subject};
use crate::ids::{Identity, NonEmptyText};
use crate::model::json::{Fields, object, string_values};

/// The one `protocol` a claimed approval carries.
pub const APPROVAL_PROTOCOL: &str = "engineering-assurance.approval-verify-result/v1";

/// The longest `record_digest` a claim may carry, in bytes.
pub const MAX_DIGEST_BYTES: usize = 256;

/// The kind of key behind the claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SignatureKind {
    /// An SSH key.
    Ssh,
    /// A gitsign certificate.
    Gitsign,
}

impl SignatureKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 2] = [Self::Ssh, Self::Gitsign];

    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::Gitsign => "gitsign",
        }
    }

    /// Read a stored spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

/// The gitsign identity the claim's writer used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitsignTrust {
    /// The certificate identity.
    pub certificate_identity: NonEmptyText,
    /// The certificate's OIDC issuer.
    pub certificate_oidc_issuer: NonEmptyText,
    /// The principal the identity maps to.
    pub principal: NonEmptyText,
}

/// The trust input the claim's writer used. Information only: nothing here
/// compares it with anything.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimTrust {
    /// The allowed-signers path the writer used.
    pub allowed_signers: NonEmptyText,
    /// The gitsign identity, when the writer gave one.
    pub gitsign: Option<GitsignTrust>,
}

impl ClaimTrust {
    fn read(fields: Fields<'_>) -> Result<Self, ChangeAssuranceError> {
        if fields.optional("gitsign").is_some() {
            fields.exact(&["allowed_signers", "gitsign"])?;
        } else {
            fields.exact(&["allowed_signers"])?;
        }
        let gitsign = if fields.optional("gitsign").is_some() {
            let gitsign = fields.nested("gitsign")?;
            gitsign.exact(&[
                "certificate_identity",
                "certificate_oidc_issuer",
                "principal",
            ])?;
            Some(GitsignTrust {
                certificate_identity: gitsign.non_empty("certificate_identity")?,
                certificate_oidc_issuer: gitsign.non_empty("certificate_oidc_issuer")?,
                principal: gitsign.non_empty("principal")?,
            })
        } else {
            None
        };
        Ok(Self {
            allowed_signers: fields.non_empty("allowed_signers")?,
            gitsign,
        })
    }

    fn to_json(&self) -> JsonValue {
        let mut members = vec![(
            "allowed_signers",
            JsonValue::string(self.allowed_signers.as_str()),
        )];
        if let Some(gitsign) = &self.gitsign {
            members.push((
                "gitsign",
                object(vec![
                    (
                        "certificate_identity",
                        JsonValue::string(gitsign.certificate_identity.as_str()),
                    ),
                    (
                        "certificate_oidc_issuer",
                        JsonValue::string(gitsign.certificate_oidc_issuer.as_str()),
                    ),
                    ("principal", JsonValue::string(gitsign.principal.as_str())),
                ]),
            ));
        }
        object(members)
    }
}

/// What an `approved` result adds: who, on what terms, and how.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedFacts {
    /// The principal the writer's trust input names for the signer.
    pub approver: NonEmptyText,
    /// The `Signed-off-meaning` trailer values, in order. Never empty.
    pub meaning: Vec<String>,
    /// The kind of key behind the claim.
    pub signature_kind: SignatureKind,
}

/// The status the result reported, with the members only `approved` carries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovalStatus {
    /// The tag is bound, trusted and carries a meaning.
    Approved(ApprovedFacts),
    /// The tag names another record.
    WrongRecord,
    /// The signer is not mapped by the writer's trust input.
    UntrustedSigner,
    /// The tag carries no meaning.
    MeaningMissing,
    /// The tag's check failed outright.
    SignedBad,
    /// The tag carries nothing to check.
    Unsigned,
    /// The tag could not be checked.
    Unverifiable,
}

impl ApprovalStatus {
    fn spelling(&self) -> &'static str {
        match self {
            Self::Approved(_) => "approved",
            Self::WrongRecord => "wrong_record",
            Self::UntrustedSigner => "untrusted_signer",
            Self::MeaningMissing => "meaning_missing",
            Self::SignedBad => "signed_bad",
            Self::Unsigned => "unsigned",
            Self::Unverifiable => "unverifiable",
        }
    }
}

/// The claim a decision payload carries, as a closed projection of the result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedApproval {
    /// The status, with the approved-only members.
    pub status: ApprovalStatus,
    /// The annotated tag's name.
    pub tag: NonEmptyText,
    /// The record the tag names.
    pub record_id: Identity,
    /// The record digest the tag names.
    pub record_digest: NonEmptyText,
    /// The trust input the writer used. Information only.
    pub trust: ClaimTrust,
}

impl VerifiedApproval {
    /// Read the projection, refusing an undeclared member, a member outside
    /// its status's row, and a protocol other than [`APPROVAL_PROTOCOL`].
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] for each of those and for a malformed
    /// member.
    pub fn from_json(value: &JsonValue, subject: Subject) -> Result<Self, ChangeAssuranceError> {
        let fields = Fields::read(value, subject, "verified_approval")?;
        let status = fields.text("status")?;
        let base = [
            "protocol",
            "status",
            "tag",
            "record_id",
            "record_digest",
            "trust",
        ];
        if status == "approved" {
            let mut allowed = base.to_vec();
            allowed.extend(["approver", "meaning", "signature_kind"]);
            fields.exact(&allowed)?;
        } else {
            fields.exact(&base)?;
        }
        fields.equals("protocol", &JsonValue::string(APPROVAL_PROTOCOL))?;
        let status = match status {
            "approved" => ApprovalStatus::Approved(read_approved(fields, subject)?),
            "wrong_record" => ApprovalStatus::WrongRecord,
            "untrusted_signer" => ApprovalStatus::UntrustedSigner,
            "meaning_missing" => ApprovalStatus::MeaningMissing,
            "signed_bad" => ApprovalStatus::SignedBad,
            "unsigned" => ApprovalStatus::Unsigned,
            "unverifiable" => ApprovalStatus::Unverifiable,
            _ => {
                return Err(ChangeAssuranceError::shape(
                    subject,
                    FieldFailure::malformed("verified_approval.status"),
                ));
            }
        };
        Ok(Self {
            status,
            tag: fields.non_empty("tag")?,
            record_id: fields.identity("record_id")?,
            record_digest: read_digest(fields, subject)?,
            trust: ClaimTrust::read(fields.nested("trust")?)?,
        })
    }

    /// The `approved` facts, when the status is `approved`.
    #[must_use]
    pub const fn approved(&self) -> Option<&ApprovedFacts> {
        match &self.status {
            ApprovalStatus::Approved(facts) => Some(facts),
            _ => None,
        }
    }

    /// The status's stored spelling.
    #[must_use]
    pub fn status_spelling(&self) -> &'static str {
        self.status.spelling()
    }
}

fn read_digest(fields: Fields<'_>, subject: Subject) -> Result<NonEmptyText, ChangeAssuranceError> {
    let digest = fields.non_empty("record_digest")?;
    if digest.as_str().len() > MAX_DIGEST_BYTES {
        return Err(ChangeAssuranceError::shape(
            subject,
            FieldFailure::malformed("record_digest"),
        ));
    }
    Ok(digest)
}

fn read_meaning(fields: Fields<'_>, subject: Subject) -> Result<Vec<String>, ChangeAssuranceError> {
    let mut meaning = Vec::new();
    for entry in fields.array("meaning", true)? {
        let text = entry.as_str().ok_or_else(|| {
            ChangeAssuranceError::shape(subject, FieldFailure::malformed("meaning"))
        })?;
        meaning.push(text.to_owned());
    }
    Ok(meaning)
}

fn read_kind(fields: Fields<'_>, subject: Subject) -> Result<SignatureKind, ChangeAssuranceError> {
    SignatureKind::parse(fields.text("signature_kind")?).ok_or_else(|| {
        ChangeAssuranceError::shape(subject, FieldFailure::malformed("signature_kind"))
    })
}

fn read_approved(
    fields: Fields<'_>,
    subject: Subject,
) -> Result<ApprovedFacts, ChangeAssuranceError> {
    Ok(ApprovedFacts {
        approver: fields.non_empty("approver")?,
        meaning: read_meaning(fields, subject)?,
        signature_kind: read_kind(fields, subject)?,
    })
}

/// The approval a receipt reports: the event writer's claim, filled only when
/// every rule of the review check held. It is a claim that Quoin has not
/// verified, to be re-verified by the relying party.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimedApproval {
    /// The annotated tag's name.
    pub tag: NonEmptyText,
    /// The digest the tag names, equal to the stored revision's digest.
    pub record_digest: NonEmptyText,
    /// The trust input the writer used. Information only.
    pub trust: ClaimTrust,
    /// The principal the writer's trust input named.
    pub approver: NonEmptyText,
    /// The meaning values, in order.
    pub meaning: Vec<String>,
    /// The kind of key behind the claim.
    pub signature_kind: SignatureKind,
}

impl ClaimedApproval {
    /// Project an `approved` claim, or `None` for any other status.
    #[must_use]
    pub fn from_claim(claim: &VerifiedApproval) -> Option<Self> {
        let facts = claim.approved()?;
        Some(Self {
            tag: claim.tag.clone(),
            record_digest: claim.record_digest.clone(),
            trust: claim.trust.clone(),
            approver: facts.approver.clone(),
            meaning: facts.meaning.clone(),
            signature_kind: facts.signature_kind,
        })
    }

    /// Read the receipt form.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] for an undeclared or absent member and a
    /// malformed one.
    pub fn from_json(fields: Fields<'_>, subject: Subject) -> Result<Self, ChangeAssuranceError> {
        fields.exact(&[
            "protocol",
            "tag",
            "record_digest",
            "trust",
            "approver",
            "meaning",
            "signature_kind",
        ])?;
        fields.equals("protocol", &JsonValue::string(APPROVAL_PROTOCOL))?;
        Ok(Self {
            tag: fields.non_empty("tag")?,
            record_digest: read_digest(fields, subject)?,
            trust: ClaimTrust::read(fields.nested("trust")?)?,
            approver: fields.non_empty("approver")?,
            meaning: read_meaning(fields, subject)?,
            signature_kind: read_kind(fields, subject)?,
        })
    }

    /// The JSON form.
    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        object(vec![
            ("protocol", JsonValue::string(APPROVAL_PROTOCOL)),
            ("tag", JsonValue::string(self.tag.as_str())),
            (
                "record_digest",
                JsonValue::string(self.record_digest.as_str()),
            ),
            ("trust", self.trust.to_json()),
            ("approver", JsonValue::string(self.approver.as_str())),
            ("meaning", string_values(&self.meaning)),
            (
                "signature_kind",
                JsonValue::string(self.signature_kind.as_str()),
            ),
        ])
    }
}
