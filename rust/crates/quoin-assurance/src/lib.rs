// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The assurance-case view (quoin#384, Stage 5 of the burn-down).
//!
//! Ported from `src/assurance/`, which its own header describes as "strictly a
//! view": it collects nothing, computes no coverage and writes nothing to the
//! store. That property is why this is the first real capability across the
//! boundary — there is no IO to model, so a difference between the two
//! implementations is a difference in the logic and nowhere else.
//!
//! # Why this crate has no dependencies yet
//!
//! The retained module's five cross-directory imports are all `import type`,
//! which is what makes it cycle-free. It does **not** make it dependency-free
//! here: `build_case` needs `Obligation`, `BundleDocument`, `Finding`,
//! `TrustAssessment` and `IndependenceAssessment` to exist as Rust types
//! before it can be written, and four of those five belong to crates that do
//! not exist yet.
//!
//! So the port starts with the part of the retained surface that names no
//! imported type at all. That is not a shortcut: it puts a real retained
//! capability through the whole pattern — request shape, exit taxonomy,
//! differential comparison against the retained implementation — on a surface
//! where a failure is legible.

/// The requirement an obligation belongs to.
///
/// `FR-001-AC-3` → `FR-001`; `NFR-010-M-2` → `NFR-010`. Derived from the id
/// rather than from an edge because obligations are minted from a document's
/// own tables — the ownership is not in question, and an edge for it would be
/// a second thing to keep in agreement.
///
/// # The suffix is deliberately unconstrained
///
/// The retained implementation is `/^([A-Za-z]+-\d+)/` and matches a *prefix*:
/// everything after the requirement id is ignored, whatever it spells. That is
/// load-bearing. An earlier check in this repository enumerated the suffixes it
/// expected — `AC|CON|VC|EX|SC` — and silently skipped every `-M-` obligation,
/// which is the whole `nfr-metric` source. A suffix list is a standing bet that
/// nobody mints a new kind, and the engine took no such bet.
///
/// # Non-matching input is returned unchanged
///
/// Not an error, and not empty. An id the pattern does not recognise is still
/// an id the caller holds, and returning it unchanged keeps the view total —
/// a projection that drops rows it cannot classify reads as a clean case over
/// a narrower population, which is the failure the retained module's header
/// names.
#[must_use]
pub fn requirement_of(obligation_id: &str) -> &str {
    let mut letters = 0usize;
    let mut digits = 0usize;
    let mut seen_dash = false;
    let mut end = 0usize;

    // A byte scan rather than an index walk: the workspace forbids
    // `indexing_slicing`, and an id off an untrusted stream is exactly the
    // input that lint exists for. Every branch below reads a byte the iterator
    // handed over, so there is no position to get wrong.
    for (offset, byte) in obligation_id.bytes().enumerate() {
        if seen_dash {
            // `\d+`
            if byte.is_ascii_digit() {
                digits += 1;
                end = offset + 1;
                continue;
            }
            // The match is a PREFIX: a suffix of any spelling ends the scan
            // rather than failing it.
            break;
        }
        // `[A-Za-z]+`
        if byte.is_ascii_alphabetic() {
            letters += 1;
            continue;
        }
        if byte == b'-' && letters > 0 {
            seen_dash = true;
            continue;
        }
        return obligation_id;
    }

    if letters == 0 || !seen_dash || digits == 0 {
        return obligation_id;
    }
    // `get` rather than a slice expression: `end` is an ASCII boundary by
    // construction, and saying so with a fallible call means a future edit
    // that breaks the invariant degrades to "unchanged" instead of panicking
    // inside a view whose whole contract is that it is total.
    obligation_id.get(..end).unwrap_or(obligation_id)
}

#[cfg(test)]
mod tests {
    use super::requirement_of;

    #[test]
    fn takes_the_requirement_prefix() {
        assert_eq!(requirement_of("FR-001-AC-3"), "FR-001");
        assert_eq!(requirement_of("NFR-010-M-2"), "NFR-010");
        assert_eq!(requirement_of("StR-009-VC-1"), "StR-009");
    }

    #[test]
    fn ignores_a_suffix_of_any_spelling() {
        // The defect a suffix allow-list produces, asserted directly.
        assert_eq!(requirement_of("NFR-013-M-1"), "NFR-013");
        assert_eq!(requirement_of("FR-042-QQQ-9"), "FR-042");
        assert_eq!(requirement_of("US-024-EX-5"), "US-024");
    }

    #[test]
    fn returns_an_unrecognised_id_unchanged() {
        assert_eq!(requirement_of("not-an-id"), "not-an-id");
        assert_eq!(requirement_of(""), "");
        assert_eq!(requirement_of("123-456"), "123-456");
        assert_eq!(requirement_of("FR"), "FR");
        assert_eq!(requirement_of("FR-"), "FR-");
    }

    #[test]
    fn a_bare_requirement_id_is_its_own_requirement() {
        assert_eq!(requirement_of("FR-001"), "FR-001");
    }

    #[test]
    fn handles_non_ascii_without_slicing_inside_a_character() {
        // The slice above is only safe because every consumed byte is ASCII.
        // A multi-byte character before the pattern must therefore fall to the
        // unchanged path rather than panic.
        assert_eq!(requirement_of("é-001-AC-1"), "é-001-AC-1");
    }
}
