// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The content address of a retained binary attachment.
//!
//! An attachment is opaque bytes (a profile, a trace, sanitizer output) kept
//! under `spec/evidence/attachments/<sha256-hex>`. The address is the SHA-256 of
//! the bytes, so the name binds the file to exactly what it held when it was
//! retained: byte-distinct inputs cannot share an address, and the same bytes
//! always reproduce it. This is the only digest this crate computes; records
//! stay addressed by assigned identity.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::store::store_root;

/// Hex characters in a SHA-256 address.
pub const SHA256_HEX_LEN: usize = 64;

/// The lowercase hex SHA-256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(SHA256_HEX_LEN);
    for byte in Sha256::digest(bytes) {
        // Writing to a `String` cannot fail.
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Whether `text` is a well-formed address: 64 lowercase hex digits.
#[must_use]
pub fn is_sha256_hex(text: &str) -> bool {
    text.len() == SHA256_HEX_LEN && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The directory attachments are retained in: `<store>/attachments`.
#[must_use]
pub fn attachments_root(repo: &Path) -> PathBuf {
    store_root(repo).join("attachments")
}

/// Where the attachment with address `hex` lives.
///
/// `hex` must satisfy [`is_sha256_hex`]; a caller holding anything else would
/// be naming a path, not an address, so it is not joined.
#[must_use]
pub fn attachment_path(repo: &Path, hex: &str) -> Option<PathBuf> {
    is_sha256_hex(hex).then(|| attachments_root(repo).join(hex))
}

#[cfg(test)]
mod tests {
    use super::{attachment_path, is_sha256_hex, sha256_hex};

    #[test]
    fn the_address_is_the_sha256_of_the_bytes() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(is_sha256_hex(&sha256_hex(b"")));
    }

    #[test]
    fn only_a_well_formed_address_names_a_path() {
        let repo = std::path::Path::new("/r");
        assert!(attachment_path(repo, "../x").is_none());
        assert!(attachment_path(repo, &"A".repeat(64)).is_none());
        assert!(attachment_path(repo, &"a".repeat(64)).is_some());
    }
}
