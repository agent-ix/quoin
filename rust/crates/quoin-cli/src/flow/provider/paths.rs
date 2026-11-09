// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Repository-scoped path resolution and bounded reads for the provider.

use std::io::Read;
use std::path::{Path, PathBuf};

use super::{Code, Failure, MAX_DOCUMENT, fail};

pub(super) fn scoped_parent(root: &Path, path: &Path) -> Result<PathBuf, Failure> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let parent = path
        .parent()
        .ok_or_else(|| fail(Code::PathInvalid, "path has no parent"))?;
    let canonical = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    if !canonical.starts_with(root)
        || canonical
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(fail(Code::PathInvalid, "path escapes repository"));
    }
    Ok(canonical)
}

pub(super) fn scoped_path(root: &Path, path: &Path) -> Result<PathBuf, Failure> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canonical = path.canonicalize().map_err(|e| {
        fail(
            Code::DocumentUnavailable,
            format!("{}: {e}", path.display()),
        )
    })?;
    if !canonical.starts_with(root) {
        return Err(fail(Code::PathInvalid, "document escapes repository"));
    }
    Ok(canonical)
}

pub(super) fn read_bounded(path: &Path) -> Result<String, Failure> {
    let input = std::fs::File::open(path).map_err(|e| {
        fail(
            Code::DocumentUnavailable,
            format!("{}: {e}", path.display()),
        )
    })?;
    let mut bytes = Vec::new();
    input
        .take(MAX_DOCUMENT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| fail(Code::DocumentUnavailable, e.to_string()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_DOCUMENT {
        return Err(fail(Code::DocumentLimit, "document exceeds 4 MiB"));
    }
    String::from_utf8(bytes).map_err(|e| fail(Code::DocumentInvalid, e.to_string()))
}
