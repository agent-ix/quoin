// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain identity newtypes.
//!
//! The TypeScript passes `string` for every one of these, and
//! `typeIdentity(packageIdentity, name)` is two strings in the order a caller
//! has to remember. These types make the swap a compile error and give the
//! `<org>/<repo>` split one implementation instead of four `split("/")` calls.
//!
//! None of them validate on construction: the schema does that, and a type that
//! refused a malformed value here would remove the diagnostic the caller is
//! supposed to report.

use std::fmt;

macro_rules! string_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wrap an already-read value.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// The underlying text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume into the underlying text.
            #[must_use]
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

string_newtype! {
    /// A semantic package's IR identity, `<org>/<repo>` — never a URL and never
    /// an `ix://` identity (FR-070).
    PackageIdentity
}

string_newtype! {
    /// An object type's name, as `object_types[].name` declares it.
    ObjectTypeName
}

string_newtype! {
    /// A module's `name`.
    ModuleName
}

string_newtype! {
    /// A module's `version`.
    ModuleVersion
}

string_newtype! {
    /// The `semantic.contract_version` a manifest declares.
    ContractVersion
}

string_newtype! {
    /// The `@agent-ix/semantic-core` version a manifest compiles against.
    SemanticCoreVersion
}

string_newtype! {
    /// A named representation mapping (FR-071..073).
    MappingName
}

impl PackageIdentity {
    /// The `<org>` and `<repo>` halves.
    ///
    /// Returns `None` when the value carries no `/`. The TypeScript destructures
    /// unconditionally and produces `undefined` for the missing half, which
    /// reaches the user as the literal text `ix://agent-ix/undefined/type/x`;
    /// modelling the absence keeps that out of an emitted identity.
    #[must_use]
    pub fn split(&self) -> Option<(&str, &str)> {
        self.as_str().split_once('/')
    }

    /// The `<org>` half, or the whole value when there is no `/`.
    #[must_use]
    pub fn org(&self) -> &str {
        self.split().map_or(self.as_str(), |(org, _)| org)
    }

    /// The `<repo>` half, or `""` when there is no `/`.
    ///
    /// Matches the TypeScript's `packageIdentity.split("/")` result for a
    /// two-segment value; for a three-segment value both take the first two.
    #[must_use]
    pub fn repo(&self) -> &str {
        self.split().map_or("", |(_, rest)| {
            rest.split_once('/').map_or(rest, |(repo, _)| repo)
        })
    }
}

#[cfg(test)]
// Indexing and `unreachable!` are a test-only convenience: an out-of-range
// index in a test is a failing test, not a downed worker.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Trace: FR-075
    #[test]
    fn tc_378_010_package_identity_splits() {
        let id = PackageIdentity::from("agent-ix/spec-objects");
        assert_eq!(id.org(), "agent-ix");
        assert_eq!(id.repo(), "spec-objects");
    }

    /// Trace: FR-075
    #[test]
    fn tc_378_011_package_identity_without_slash_has_no_repo() {
        let id = PackageIdentity::from("agent-ix");
        assert_eq!(id.split(), None);
        assert_eq!(id.org(), "agent-ix");
        assert_eq!(id.repo(), "");
    }

    /// Trace: FR-075
    #[test]
    fn tc_378_012_package_identity_takes_the_first_two_segments() {
        let id = PackageIdentity::from("agent-ix/spec/objects");
        assert_eq!(id.org(), "agent-ix");
        assert_eq!(id.repo(), "spec");
    }
}
