// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Preflight machine identities required by `MeasurementCollection` intake.

use std::collections::BTreeMap;

pub(super) fn valid_toolchains(toolchains: &BTreeMap<String, String>) -> bool {
    !toolchains.is_empty()
        && toolchains.iter().all(|(name, identity)| {
            matches!(name.as_str(), "node" | "rust" | "python") && !identity.trim().is_empty()
        })
}

#[cfg(test)]
mod tests {
    use super::valid_toolchains;
    use std::collections::BTreeMap;

    /// Trace: FR-114-AC-2, TC-1942. Provenance: PLAT-1043.
    #[test]
    fn campaign_preflight_requires_supported_toolchain_identity() {
        assert!(!valid_toolchains(&BTreeMap::new()));
        assert!(!valid_toolchains(&BTreeMap::from([(
            "container".to_owned(),
            "image@sha256:abc".to_owned(),
        )])));
        assert!(!valid_toolchains(&BTreeMap::from([(
            "rust".to_owned(),
            "  ".to_owned(),
        )])));
        assert!(valid_toolchains(&BTreeMap::from([(
            "rust".to_owned(),
            "rustc 1.98.1 x86_64-unknown-linux-gnu".to_owned(),
        )])));
    }
}
