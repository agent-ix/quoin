// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Closed string unions become `Copy` enums, once, with one shape.
//!
//! `src/measurement/` declares fifteen closed string unions across the four
//! files this module serves (quoin#469). Ported one at a time they would be
//! fifteen hand-written `as_str`/`from_code` pairs, and the plan's §13.3 finding
//! is that a chain over `&str` is exactly the shape that silently skips a new
//! variant. So the spelling table is declared once per union and the accessors
//! are generated from it: the wire spelling and the variant cannot drift apart
//! because there is only one place they are written down.
//!
//! The shape is `quoin_core::error::CoreErrorCode`'s, restated for a value
//! rather than an error code: `as_str`, `all`, `from_code`. The round-trip and
//! uniqueness assertions live in `tests/tc_469_wire_enums.rs`, over
//! [`WireEnum`], so a union added later is covered the moment it is declared.

/// A closed string union with a stable wire spelling per variant.
///
/// Implemented only by [`wire_enum!`]. The spellings are the API: renaming one
/// is a wire break, exactly as renaming an error code is.
pub trait WireEnum: Copy + Sized + PartialEq + 'static {
    /// Every variant, in declaration order.
    fn all() -> &'static [Self];

    /// The wire spelling of this variant.
    fn as_str(self) -> &'static str;

    /// The variant a wire spelling names, or `None` if it names none.
    fn from_code(code: &str) -> Option<Self>;

    /// The union's own name, for diagnostics and for the census test.
    fn union_name() -> &'static str;
}

/// Declares a closed string union as a `Copy` enum implementing [`WireEnum`].
macro_rules! wire_enum {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $wire:literal,
            )+
        }
    ) => {
        $(#[$enum_meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
            ::serde::Serialize, ::serde::Deserialize,
        )]
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                #[serde(rename = $wire)]
                $variant,
            )+
        }

        impl $crate::common::wire_enum::WireEnum for $name {
            fn all() -> &'static [Self] {
                &[$(Self::$variant),+]
            }

            fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire,)+
                }
            }

            fn from_code(code: &str) -> Option<Self> {
                match code {
                    $($wire => Some(Self::$variant),)+
                    _ => None,
                }
            }

            fn union_name() -> &'static str {
                stringify!($name)
            }
        }

        impl $name {
            /// Every variant, in declaration order.
            #[must_use]
            pub fn all() -> &'static [Self] {
                <Self as $crate::common::wire_enum::WireEnum>::all()
            }

            /// The wire spelling of this variant.
            #[must_use]
            pub fn as_str(self) -> &'static str {
                <Self as $crate::common::wire_enum::WireEnum>::as_str(self)
            }

            /// The variant a wire spelling names, or `None` if it names none.
            #[must_use]
            pub fn from_code(code: &str) -> Option<Self> {
                <Self as $crate::common::wire_enum::WireEnum>::from_code(code)
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str(
                    <Self as $crate::common::wire_enum::WireEnum>::as_str(*self),
                )
            }
        }
    };
}

pub(crate) use wire_enum;
