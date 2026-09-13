// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The `bytesBase64` attachment encoding, and the one declared divergence.
//!
//! # Encoding is not the interesting half
//!
//! `graph-adapters.ts:564` writes `Buffer.from(bytes).toString("base64")` —
//! RFC 4648 §4, padded, no line breaks. [`encode`] produces the same bytes for
//! every input, and `tests/tc_475_parity.rs` measures that against the
//! TypeScript rather than asserting it.
//!
//! This is **not** a second base64 implementation of
//! [`quoin_measurement::intervention::ids::base64url_no_pad`]: that one is RFC
//! 4648 §5 (`-` and `_`, no padding), which is Node's `"base64url"`, and this
//! one is §4 (`+` and `/`, padded), which is Node's `"base64"`. They are
//! different alphabets producing different bytes, and the two call sites read
//! back through different decoders. Merging them would mean one of the two
//! stopped round-tripping.
//!
//! # Decoding is the interesting half, and it is quoin#465
//!
//! Node's `Buffer.from(text, "base64")` is lenient in three ways at once: it
//! **silently discards** every character outside the alphabet, it does not
//! require padding, and it stops at the first `=`. So
//! `Buffer.from("aG!!VsbG8=", "base64")` is `"hello"` — the `!!` simply
//! vanishes, and a corrupted attachment decodes to something plausible.
//!
//! [`decode`] refuses all three. That is a **declared divergence**: input Node
//! accepted is newly refused. It is declared rather than fixed because
//! silently discarding bytes from a digest-bearing attachment is the failure
//! the refusal exists to prevent — the whole point of `bytesBase64` is that its
//! digest is checkable, and a decoder that invents a decoding for corrupt input
//! makes the check pass over the wrong bytes. See `DECLARED_DIVERGENCES` in
//! `tests/tc_475_parity.rs`.

use crate::error::{GraphAdapterError, GraphAdapterErrorCode, Result};

/// The RFC 4648 §4 alphabet, in value order.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The value of one alphabet character, or `None` for anything else.
fn value_of(byte: u8) -> Option<u8> {
    // `u8::try_from` on the found index cannot fail: the alphabet is 64 long.
    ALPHABET
        .iter()
        .position(|candidate| *candidate == byte)
        .and_then(|index| u8::try_from(index).ok())
}

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

/// Decode strict RFC 4648 §4 base64.
///
/// Refuses what Node's decoder accepts: a character outside the alphabet, a
/// length that is not a multiple of four, and `=` anywhere but as the last one
/// or two characters. See the module header — this is the declared divergence.
///
/// # Errors
///
/// [`GraphAdapterErrorCode::AttachmentEncodingInvalid`] for every refusal, with
/// the offending offset named.
pub fn decode(text: &str) -> Result<Vec<u8>> {
    let refuse = |detail: String| {
        Err(GraphAdapterError::new(
            GraphAdapterErrorCode::AttachmentEncodingInvalid,
            detail,
        ))
    };
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return refuse(format!(
            "base64 length {} is not a multiple of 4; Node's decoder pads silently, this one \
             refuses (quoin#465)",
            bytes.len()
        ));
    }
    let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if padding > 2 {
        return refuse(format!("{padding} padding characters; at most 2 are legal"));
    }
    let body = bytes.len() - padding;
    let mut accumulator: u32 = 0;
    let mut held: u32 = 0;
    let mut out = Vec::with_capacity(body / 4 * 3);
    for (offset, byte) in bytes.iter().take(body).enumerate() {
        let Some(value) = value_of(*byte) else {
            return refuse(format!(
                "byte {offset} (`{}`) is outside the base64 alphabet; Node's decoder discards it \
                 silently, this one refuses (quoin#465)",
                char::from(*byte).escape_debug()
            ));
        };
        accumulator = (accumulator << 6) | u32::from(value);
        held += 6;
        if held >= 8 {
            held -= 8;
            out.push(u8::try_from((accumulator >> held) & 0xff).unwrap_or(0));
        }
    }
    Ok(out)
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
    use super::{decode, encode};
    use crate::error::GraphAdapterErrorCode;

    /// The RFC 4648 §10 vectors, which are the encoding's published oracle.
    #[test]
    fn tc_475_010_the_rfc_4648_vectors_encode_and_decode() {
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
            assert_eq!(decode(encoded).expect("a vector decodes"), plain.as_bytes());
        }
    }

    /// Every byte value round-trips, and the `+` and `/` symbols are reached.
    #[test]
    fn tc_475_011_every_byte_round_trips_through_the_full_alphabet() {
        let all: Vec<u8> = (0..=255u8).collect();
        let encoded = encode(&all);
        assert!(encoded.contains('+') && encoded.contains('/'), "{encoded}");
        assert_eq!(decode(&encoded).expect("round trip"), all);
    }

    /// The three lenient forms Node accepts are refused here (quoin#465).
    #[test]
    fn tc_475_012_the_lenient_forms_node_accepts_are_refused() {
        for lenient in ["aG!!VsbG8=", "aGVsbG8", "aGVs bG8=", "aGVsbG8=\n"] {
            let error = decode(lenient).expect_err("strict decoding refuses");
            assert_eq!(
                error.code(),
                GraphAdapterErrorCode::AttachmentEncodingInvalid,
                "{lenient:?}"
            );
        }
    }
}
