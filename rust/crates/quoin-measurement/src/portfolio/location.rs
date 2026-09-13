// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Where a repository is, and whether anything is there.
//!
//! # Why this module names a filesystem
//!
//! Everything else in this crate takes a [`crate::source::MeasurementSource`],
//! and it stays that way: the reading is still a source's. But the portfolio
//! view is a walk over *locations a caller named* and its first three outputs
//! are facts about those locations rather than about their contents — the
//! resolved root, whether anything exists there, and whether what is there is a
//! directory (`portfolio.ts:79-82,228-243`). A source cannot answer those,
//! because a source is already inside one repository.
//!
//! So the probe is here, in one module, rather than as a fourth method on a
//! trait the other twelve callers would never use.

use std::path::{Component, Path, PathBuf};

/// What is at a location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LocationState {
    /// Nothing exists there. `portfolio.ts:228-234`.
    Missing,
    /// Something is there and it is not a directory. `portfolio.ts:236-243`.
    NotADirectory,
    /// A directory, which is the only state that is read further.
    Directory,
    /// Something is there and its metadata could not be read — the retained
    /// `statSync` throw that `portfolio.ts:271` catches.
    Unreadable(String),
}

/// What is at `root`.
pub(crate) fn probe(root: &Path) -> LocationState {
    if !root.exists() {
        return LocationState::Missing;
    }
    match root.metadata() {
        Ok(metadata) if metadata.is_dir() => LocationState::Directory,
        Ok(_) => LocationState::NotADirectory,
        Err(error) => LocationState::Unreadable(format!("{}: {error}", root.display())),
    }
}

/// `resolve(location)` (`node:path`), which `portfolio.ts:80` applies to every
/// location a caller names.
///
/// Absolute against the process's working directory, then normalised
/// lexically: `.` dropped, `..` popped, trailing separators removed. Symbolic
/// links are not followed, by either implementation — `resolve` is a string
/// operation in both.
pub(crate) fn resolve(location: &Path) -> PathBuf {
    let absolute = if location.is_absolute() {
        location.to_path_buf()
    } else {
        // A working directory that cannot be read is not a reason to panic in
        // a library; the root is what `resolve` would produce from an empty
        // one.
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(location)
    };
    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                out.push(component.as_os_str());
            }
        }
    }
    out
}

/// `basename(root) || root` (`portfolio.ts:296`): the directory's own name,
/// falling back to the whole root when it has none.
pub(crate) fn display_name(root: &Path) -> String {
    root.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| root.to_string_lossy().into_owned())
}
