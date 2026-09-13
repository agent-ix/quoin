// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain identity newtypes for the completeness check.
//!
//! `assessVocabulary(declaration, documents)` juggles four strings that all read
//! the same at a call site: the vocabulary's name, one of its values, the
//! artifact type it projects from, and the frontmatter field carrying the claim.
//! Three of those are swappable in the TypeScript without a type error.

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
    /// A declaration's own name, e.g. `quality-characteristics`.
    VocabularyName
}

string_newtype! {
    /// One value inside a declared vocabulary, e.g. `safety`.
    VocabularyValue
}

string_newtype! {
    /// The artifact type a projection reads, e.g. `NFR`.
    ArtifactTypeName
}

string_newtype! {
    /// A frontmatter field, e.g. `quality_attribute`.
    FrontmatterField
}
