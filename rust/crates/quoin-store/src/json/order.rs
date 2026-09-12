// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The two member orderings the canonical serializers use.
//!
//! They are different orderings and the difference is load-bearing:
//!
//! * [`cmp_utf16`] — RFC 8785 §3.2.3 sorts member names by **UTF-16 code
//!   unit**. The oracle gets this from JavaScript `Array.prototype.sort`,
//!   whose default comparator is UTF-16 lexicographic. Rust's `str: Ord` is
//!   *not* the same order: it compares Unicode scalar values, so `U+10000`
//!   sorts after `U+FFFD` in Rust and before it in UTF-16.
//! * [`ecmascript_own_property_order`] — the evidence store's pretty form is
//!   `JSON.stringify(sortKeys(value), null, 2)`, and `sortKeys` re-inserts the
//!   sorted names into a fresh JavaScript object. Insertion does not preserve
//!   the sorted order: ECMAScript `OrdinaryOwnPropertyKeys` emits **array
//!   index** names first, in ascending numeric order, and only then the
//!   remaining names in insertion order. So `{"10":_, "2":_, "a":_}`
//!   serializes as `2, 10, a`, not as the sorted `10, 2, a`.

use std::cmp::Ordering;

/// Compare two strings by UTF-16 code unit, as RFC 8785 §3.2.3 requires.
#[must_use]
pub fn cmp_utf16(left: &str, right: &str) -> Ordering {
    // Fast path: while both strings stay inside the BMP-with-no-surrogate-pair
    // region, byte order and UTF-16 order agree. Rather than special-case that,
    // compare code units directly — member names are short and this is the one
    // place a subtle difference is unrecoverable.
    let mut left_units = left.encode_utf16();
    let mut right_units = right.encode_utf16();
    loop {
        match (left_units.next(), right_units.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) => match a.cmp(&b) {
                Ordering::Equal => {}
                other => return other,
            },
        }
    }
}

/// The largest value an ECMAScript array index may take: `2^32 - 2`.
///
/// `"4294967295"` (`2^32 - 1`) is *not* an array index — it is an ordinary
/// string key — and that off-by-one decides its position in the pretty form.
pub const MAX_ARRAY_INDEX: u32 = u32::MAX - 1;

/// The array index a member name denotes, if it denotes one.
///
/// A name is an array index only when it is the canonical decimal spelling of
/// an integer in `0..=MAX_ARRAY_INDEX`. `"01"`, `"-1"`, `"1.0"`, `"+1"`, `""`
/// and `"4294967295"` are all ordinary string keys.
#[must_use]
pub fn array_index(name: &str) -> Option<u32> {
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if name.len() > 1 && name.starts_with('0') {
        return None;
    }
    // `len() > 10` cannot fit in u32; `parse` rejects the rest.
    let value: u32 = name.parse().ok()?;
    (value <= MAX_ARRAY_INDEX).then_some(value)
}

/// Member names in the order `JSON.stringify` walks them after `sortKeys`
/// has re-inserted them into a fresh object.
///
/// Array-index names first, ascending numerically; then every other name in
/// UTF-16 sorted order, which is the order `sortKeys` inserted them in.
pub fn ecmascript_own_property_order<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut indexed: Vec<(u32, &'a str)> = Vec::new();
    let mut strings: Vec<&'a str> = Vec::new();
    for name in names {
        match array_index(name) {
            Some(index) => indexed.push((index, name)),
            None => strings.push(name),
        }
    }
    indexed.sort_unstable_by_key(|(index, _)| *index);
    strings.sort_unstable_by(|a, b| cmp_utf16(a, b));
    indexed
        .into_iter()
        .map(|(_, name)| name)
        .chain(strings)
        .collect()
}

/// Member names in RFC 8785 order: UTF-16 code unit, with no index exception.
pub fn jcs_order<'a>(names: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut names: Vec<&'a str> = names.collect();
    names.sort_unstable_by(|a, b| cmp_utf16(a, b));
    names
}

#[cfg(test)]
mod tests {
    use super::{array_index, cmp_utf16, ecmascript_own_property_order, jcs_order};
    use std::cmp::Ordering;

    #[test]
    fn tc_380_utf16_order_puts_astral_before_high_bmp() {
        // U+10000 encodes as D800 DC00; U+FFFD is a single FFFD unit.
        // D800 < FFFD, so UTF-16 order places the astral character first —
        // the reverse of Rust's scalar-value `str: Ord`.
        assert_eq!(cmp_utf16("\u{10000}", "\u{FFFD}"), Ordering::Less);
        assert_eq!("\u{10000}".cmp("\u{FFFD}"), Ordering::Greater);
    }

    #[test]
    fn tc_380_array_index_recognises_only_canonical_spellings() {
        assert_eq!(array_index("0"), Some(0));
        assert_eq!(array_index("4294967294"), Some(u32::MAX - 1));
        assert_eq!(array_index("4294967295"), None);
        assert_eq!(array_index("01"), None);
        assert_eq!(array_index("-1"), None);
        assert_eq!(array_index("1.0"), None);
        assert_eq!(array_index(""), None);
        assert_eq!(array_index("9007199254740993"), None);
    }

    #[test]
    fn tc_380_own_property_order_hoists_indices_ahead_of_sorted_names() {
        let names = ["10", "2", "a", "1"];
        assert_eq!(
            ecmascript_own_property_order(names.into_iter()),
            vec!["1", "2", "10", "a"]
        );
        assert_eq!(jcs_order(names.into_iter()), vec!["1", "10", "2", "a"]);
    }
}
