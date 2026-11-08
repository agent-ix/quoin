// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `all_variants!`: an `ALL` array that cannot go stale (PLAT-1036).
//!
//! A hand-written `pub const ALL: [Self; N]` compiles when an enum variant is
//! added and the array is not updated, so `from_wire` would then refuse a code
//! quoin itself emits. This macro writes the array *and* an exhaustive `match`
//! over the same variant list: a variant missing from the list makes that
//! `match` non-exhaustive, which is a compile error, and a variant listed twice
//! is an unreachable pattern, which this workspace's `-D warnings` gate
//! rejects. The length is counted from the list, so it is never a literal.

/// Expand to `pub const ALL: [Self; N]` for the enum `$ty`, listing every
/// variant in the order given (declaration order, by convention).
macro_rules! all_variants {
    ($(#[$meta:meta])* $ty:ident: $($variant:ident),+ $(,)?) => {
        $(#[$meta])*
        pub const ALL: [Self; 0 $(+ { let _ = stringify!($variant); 1 })+] = {
            fn exhaustive(reason: $ty) {
                match reason {
                    $($ty::$variant => {}),+
                }
            }
            let _: fn($ty) = exhaustive;
            [$(Self::$variant),+]
        };
    };
}

pub(crate) use all_variants;
