// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The native executable's self-contained semantic-contract data (quoin#527).

use std::fs;
use std::path::Path;

use crate::error::SemanticError;

include!(concat!(env!("OUT_DIR"), "/embedded_contract.rs"));

/// Materialize the exact semantic-contract bytes compiled into this binary.
///
/// The paths are generated from Quoin-owned build inputs, never caller input.
/// A stable cache root makes the work idempotent across native invocations.
///
/// # Errors
///
/// Returns [`SemanticError::EmbeddedContractWrite`] when the selected cache
/// root cannot be created or updated.
pub fn materialize_embedded_contract(root: &Path) -> Result<(), SemanticError> {
    for (relative, bytes) in EMBEDDED_FILES {
        let target = root.join(relative);
        if fs::read(&target).is_ok_and(|existing| existing == *bytes) {
            continue;
        }
        let parent = target
            .parent()
            .ok_or_else(|| SemanticError::EmbeddedContractWrite {
                path: target.clone(),
                source: std::io::Error::other("embedded contract path has no parent"),
            })?;
        fs::create_dir_all(parent).map_err(|source| SemanticError::EmbeddedContractWrite {
            path: parent.to_path_buf(),
            source,
        })?;
        let staging = target.with_extension(format!("partial.{}", std::process::id()));
        fs::write(&staging, bytes).map_err(|source| SemanticError::EmbeddedContractWrite {
            path: staging.clone(),
            source,
        })?;
        fs::rename(&staging, &target).map_err(|source| SemanticError::EmbeddedContractWrite {
            path: target,
            source,
        })?;
    }
    Ok(())
}
