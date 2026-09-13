// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The identity-bearing strings of a measurement record.
//!
//! # Which strings get a newtype and which do not
//!
//! Not every `string` in `intervention-types.ts` and `operational-types.ts` is
//! an identity. A newtype earns its place when two values of the same Rust type
//! could be swapped at a call site and the compiler would not care — a
//! `record_id` handed where a `control_id` belongs, a digest handed where a path
//! belongs. Those are the ones below.
//!
//! Free prose is left as `String`: `owner`, `unit`, `surface`, `coverage`,
//! `actor`, `trigger`, `description`, `statement`, `population`, the gap and
//! action lists. Wrapping those buys no confusion-safety and only adds a
//! conversion at every use.
//!
//! # These newtypes hold, and do not validate
//!
//! The four files this wave ports are type declarations. They validate nothing —
//! `intervention.ts` and `operational.ts` do, and those are quoin#471/#472. A
//! constructor that refused here would be behaviour this port invented, so
//! `parse_stored`-style refusals (the `quoin_store::RawFileSha256Digest` shape)
//! are deliberately absent, and named as deferred rather than quietly skipped.
//!
//! [`WireInstant`] is the sharpest case. quoin already carries six hand-rolled
//! instant validators across four crates, accepting three different languages
//! (the plan's §13.1 census). This crate has exactly one grammar, in
//! [`crate::date_time::Rfc3339DateTime`] (quoin#468), and intake parses these
//! through it. Adding a seventh here — even a small one — is the defect that
//! census exists to stop; `tc_468_boundary` asserts `date_time` is the only
//! module here that reads an instant.

/// Declares an identity newtype over an immutable string.
macro_rules! identity_newtype {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident;
    ) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
            ::serde::Serialize, ::serde::Deserialize,
        )]
        #[serde(transparent)]
        $vis struct $name(Box<str>);

        impl $name {
            /// Holds the stored spelling as-is.
            ///
            /// This wave does not validate; see the module header for why, and
            /// for where the validating constructor lands.
            #[must_use]
            pub fn from_stored(value: impl Into<Box<str>>) -> Self {
                Self(value.into())
            }

            /// The stored spelling.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

identity_newtype! {
    /// A record's own identity — `record_id` on both record families.
    pub struct RecordId;
}

identity_newtype! {
    /// The thing measured — `subject.id`.
    pub struct SubjectId;
}

identity_newtype! {
    /// A source revision — `subject.revision`, `producer.source_revision` and a
    /// version pin's `revision`.
    pub struct Revision;
}

identity_newtype! {
    /// An experiment arm — `baseline.id`, `treatments[].id` and the
    /// `treatment_id` that refers to one.
    pub struct ArmId;
}

identity_newtype! {
    /// A measured quantity's name — `measured_effects[].metric`.
    pub struct MetricName;
}

identity_newtype! {
    /// An operational control — `capability.control_id`, `exercise.control_id`.
    pub struct ControlId;
}

identity_newtype! {
    /// A path to a retained evidence file, relative to the store.
    pub struct EvidencePath;
}

identity_newtype! {
    /// A digest as stored, algorithm prefix included.
    ///
    /// Deliberately not `quoin_store::RawFileSha256Digest`: this wave declares
    /// record shapes and reaches outside the crate for nothing. Verification —
    /// where that type is the right one — is quoin#471/#472.
    pub struct Digest;
}

identity_newtype! {
    /// An IANA media type as recorded.
    pub struct MediaType;
}

identity_newtype! {
    /// An instant as it appears on the wire, unparsed.
    ///
    /// **Validation happens in [`crate::date_time::Rfc3339DateTime`], not
    /// here.** That is this crate's one RFC 3339 grammar (quoin#468) and
    /// `tc_468_boundary` asserts no second module grows one. It is not applied
    /// at this type because the four files this wave ports are type
    /// declarations that validate nothing: `intervention.ts` and
    /// `operational.ts` do, and they are quoin#471/#472, which parse these
    /// strings through `Rfc3339DateTime` at intake. Refusing here would be a
    /// refusal this port invented, and would make a record unreadable that the
    /// retained TypeScript reads.
    pub struct WireInstant;
}
