// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The `bytesBase64` attachment encoding.
//!
//! `graph-adapters.ts:564` writes `Buffer.from(bytes).toString("base64")` --
//! RFC 4648 §4, padded, no line breaks. [`encode`] produces the same bytes for
//! every input.
//!
//! This is **not** a second base64 implementation of
//! [`quoin_measurement::intervention::ids::base64url_no_pad`]: that one is RFC
//! 4648 §5 (`-` and `_`, no padding); this one is §4 (`+` and `/`, padded).
//! Nothing here decodes: the retained scorer bytes are carried, not checked.

/// The RFC 4648 §4 alphabet, in value order.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode bytes as `Buffer.from(bytes).toString("base64")` does.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        // `chunks(3)` yields 1, 2 or 3 bytes; `get` keeps the indexing lint
        // satisfied without asserting a length the iterator already promises.
        let first = u32::from(chunk.first().copied().unwrap_or(0));
        let second = u32::from(chunk.get(1).copied().unwrap_or(0));
        let third = u32::from(chunk.get(2).copied().unwrap_or(0));
        let packed = (first << 16) | (second << 8) | third;
        let symbol = |shift: u32| {
            let index = usize::try_from((packed >> shift) & 0x3f).unwrap_or(0);
            char::from(ALPHABET.get(index).copied().unwrap_or(b'A'))
        };
        out.push(symbol(18));
        out.push(symbol(12));
        out.push(if chunk.len() > 1 { symbol(6) } else { '=' });
        out.push(if chunk.len() > 2 { symbol(0) } else { '=' });
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::encode;

    /// The RFC 4648 §10 vectors, which are the encoding's published oracle.
    #[test]
    fn tc_475_010_the_rfc_4648_vectors_encode() {
        for (plain, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(plain.as_bytes()), encoded, "encode({plain:?})");
        }
    }

    /// Every byte value is encoded, and the `+` and `/` symbols are reached.
    #[test]
    fn tc_475_011_every_byte_value_reaches_the_full_alphabet() {
        let all: Vec<u8> = (0..=255u8).collect();
        let encoded = encode(&all);
        assert!(encoded.contains('+') && encoded.contains('/'), "{encoded}");
    }
}
