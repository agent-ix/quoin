// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Resolving and digesting a plan's protected apparatus when a collection is
//! written (PLAT-975, engineering-assurance FR-024).
//!
//! # Why intake resolves the set itself
//!
//! `verificationStack.artifacts` is the producer's own map. A producer can
//! leave a file out of it, and a comparison over that map alone would then
//! never see the file change. So intake walks the repository itself, resolves
//! every `protected_apparatus` entry of every plan governing an observation,
//! digests each file, and refuses the write when the producer's map omits a
//! resolved file ([`MeasurementErrorCode::ApparatusUndeclared`]) or disagrees
//! with its bytes. The resolved (path, digest) set is what intake records, in
//! `verificationStack.protectedApparatus`, so a later comparison reads the
//! apparatus as it was when the collection was written and never today's
//! disk.
//!
//! # The resolver (engineering-assurance FR-024)
//!
//! - A file entry names one regular file. Nothing there, or a directory
//!   there, names no file ([`MeasurementErrorCode::ApparatusUnresolved`]).
//! - A `<directory>/**` entry names every regular file under the directory,
//!   recursively, dotfiles included, and must name at least one.
//! - Every name is matched **exactly** against the directory listing, so the
//!   match is case-sensitive even on a case-insensitive filesystem, where a
//!   bare `stat` of `Answers.json` would find `answers.json`.
//! - A symlink — named by an entry, passed through on the way to it, or found
//!   under a directory entry — is refused and never followed
//!   ([`MeasurementErrorCode::ApparatusSymlink`]). Following one would digest
//!   bytes the repository does not hold, which is PLAT-969's F3 in another
//!   place.
//! - A file that cannot be digested, or anything under a directory entry that
//!   is neither a file, a directory nor a symlink, is
//!   [`MeasurementErrorCode::ApparatusUnreadable`].
//! - One plan's set holds at most [`MAX_PROTECTED_FILES`] files; past that
//!   the write is [`MeasurementErrorCode::ApparatusTooLarge`], because every
//!   resolved file becomes a member of the stored collection.

use std::collections::BTreeMap;
use std::error::Error as _;
use std::fs::FileType;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use engineering_assurance::measurement::ApparatusPath;
use quoin_store::{JsonObject, JsonValue};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::types::collection::{MeasurementCollection, ResolvedApparatus};
use crate::types::plan::MeasurementPlan;

/// The most files one plan's protected apparatus may resolve to.
///
/// Each resolved file is one `path: digest` member of the stored collection,
/// about a hundred bytes; this bounds that member near 5 MB.
pub const MAX_PROTECTED_FILES: usize = 50_000;

/// Resolve every governing plan's protected apparatus under `repo`, keyed by
/// plan id, and require the candidate's `verificationStack.artifacts` to
/// declare every resolved file at its digest.
///
/// A plan governs the collection when an observation's metric names it, the
/// same lookup [`crate::validate::measurement_collection`] admitted the
/// observation under. A plan with no `protected_apparatus` contributes
/// nothing, so a collection no protecting plan governs resolves to an empty
/// map and is written exactly as before PLAT-975.
///
/// # Errors
///
/// [`MeasurementErrorCode::ApparatusUnresolved`],
/// [`MeasurementErrorCode::ApparatusSymlink`],
/// [`MeasurementErrorCode::ApparatusUnreadable`] and
/// [`MeasurementErrorCode::ApparatusTooLarge`] from resolution, each naming
/// the plan and the entry;
/// [`MeasurementErrorCode::ApparatusUndeclared`] naming every resolved file
/// the artifacts map omits; and
/// [`MeasurementErrorCode::CollectionInvalid`] when it states a resolved
/// file at another digest.
pub(super) fn resolve_protected(
    repo: &Path,
    collection: &MeasurementCollection,
    plans: &[MeasurementPlan],
) -> Result<BTreeMap<String, ResolvedApparatus>, MeasurementError> {
    let by_metric: BTreeMap<&str, &MeasurementPlan> = plans
        .iter()
        .map(|plan| (plan.metric.as_str(), plan))
        .collect();
    let mut resolved = BTreeMap::new();
    for observation in &collection.observations {
        let Some(plan) = by_metric.get(observation.metric.as_str()) else {
            continue;
        };
        let Some(protected) = plan.protected_apparatus.as_ref() else {
            continue;
        };
        if resolved.contains_key(plan.id.as_str()) {
            continue;
        }
        let mut files = ResolvedApparatus::new();
        for entry in protected.iter() {
            Walk {
                repo,
                plan: plan.id.as_str(),
                entry,
                files: &mut files,
            }
            .resolve()?;
        }
        declared(collection, plan.id.as_str(), &files)?;
        resolved.insert(plan.id.as_str().to_owned(), files);
    }
    Ok(resolved)
}

/// Require `artifacts` to state every resolved file at its digest.
fn declared(
    collection: &MeasurementCollection,
    plan: &str,
    files: &ResolvedApparatus,
) -> Result<(), MeasurementError> {
    let artifacts = collection
        .verification_stack
        .as_ref()
        .map(|stack| &stack.artifacts);
    let mut undeclared = Vec::new();
    for (path, digest) in files {
        match artifacts.and_then(|artifacts| artifacts.get(path)) {
            None => undeclared.push(format!(
                "verificationStack.artifacts does not declare `{path}`, which plan {plan} \
                 protects; it digests to {}",
                digest.to_stored()
            )),
            // `verify_local_artifacts` has already refused a disagreeing
            // digest for every name that is a file; this is the file changing
            // between that read and this one.
            Some(stated) if stated != digest => {
                return Err(MeasurementError::new(
                    MeasurementErrorCode::CollectionInvalid,
                    format!(
                        "verificationStack.artifacts.{path} does not match the protected file \
                         plan {plan} names: the record says {}, the file digests to {}",
                        stated.to_stored(),
                        digest.to_stored()
                    ),
                ));
            }
            Some(_) => {}
        }
    }
    if undeclared.is_empty() {
        Ok(())
    } else {
        Err(MeasurementError::with_findings(
            MeasurementErrorCode::ApparatusUndeclared,
            format!("plan {plan}'s protected apparatus is not declared in the collection"),
            undeclared,
        ))
    }
}

/// One entry of one plan, being resolved into that plan's set.
struct Walk<'a> {
    repo: &'a Path,
    plan: &'a str,
    entry: &'a ApparatusPath,
    files: &'a mut ResolvedApparatus,
}

impl Walk<'_> {
    /// Resolve the entry, adding every file it names to the set.
    fn resolve(&mut self) -> Result<(), MeasurementError> {
        let Some(directory) = self.entry.directory() else {
            let (on_disk, found) = self.descend(self.entry.as_str(), false)?;
            let path = self.entry.as_str().to_owned();
            return match found {
                Found::File => self.digest(&on_disk, path),
                Found::Directory => Err(self.refuse(
                    MeasurementErrorCode::ApparatusUnresolved,
                    format!("names a directory, not a file; protect it as `{path}/**`"),
                )),
                Found::Other => Err(self.refuse(
                    MeasurementErrorCode::ApparatusUnreadable,
                    "names something that is not a regular file",
                )),
            };
        };
        let (at, found) = self.descend(directory, true)?;
        if found != Found::Directory {
            return Err(self.refuse(
                MeasurementErrorCode::ApparatusUnresolved,
                format!("`{directory}` is not a directory"),
            ));
        }
        let mut pending = vec![(at, directory.to_owned())];
        let mut seen = 0_usize;
        while let Some((dir, relative)) = pending.pop() {
            for (name, kind) in self.listing(&dir)? {
                let child = format!("{relative}/{name}");
                if kind.is_symlink() {
                    return Err(self.refuse(
                        MeasurementErrorCode::ApparatusSymlink,
                        format!("`{child}` is a symlink, which is refused and not followed"),
                    ));
                }
                if kind.is_dir() {
                    pending.push((dir.join(&name), child));
                } else if kind.is_file() {
                    seen += 1;
                    self.digest(&dir.join(&name), child)?;
                } else {
                    return Err(self.refuse(
                        MeasurementErrorCode::ApparatusUnreadable,
                        format!("`{child}` is not a regular file"),
                    ));
                }
            }
        }
        if seen == 0 {
            return Err(self.refuse(
                MeasurementErrorCode::ApparatusUnresolved,
                format!("`{directory}` holds no file"),
            ));
        }
        Ok(())
    }

    /// Walk `relative`'s segments from the repository root, matching each
    /// name exactly and refusing a symlink at any of them. Every segment but
    /// the last must be a directory. Returns the last segment's path and what
    /// it is.
    fn descend(
        &self,
        relative: &str,
        expect_directory: bool,
    ) -> Result<(PathBuf, Found), MeasurementError> {
        let mut cursor = self.repo.to_path_buf();
        let mut segments = relative.split('/').peekable();
        let mut walked = String::new();
        while let Some(segment) = segments.next() {
            if !walked.is_empty() {
                walked.push('/');
            }
            walked.push_str(segment);
            let Some(kind) = self
                .listing(&cursor)?
                .into_iter()
                .find_map(|(name, kind)| (name == segment).then_some(kind))
            else {
                let what = if expect_directory {
                    "directory"
                } else {
                    "file"
                };
                return Err(self.refuse(
                    MeasurementErrorCode::ApparatusUnresolved,
                    format!("names no {what}: `{walked}` does not exist"),
                ));
            };
            if kind.is_symlink() {
                return Err(self.refuse(
                    MeasurementErrorCode::ApparatusSymlink,
                    format!("`{walked}` is a symlink, which is refused and not followed"),
                ));
            }
            cursor.push(segment);
            if segments.peek().is_none() {
                let found = if kind.is_dir() {
                    Found::Directory
                } else if kind.is_file() {
                    Found::File
                } else {
                    Found::Other
                };
                return Ok((cursor, found));
            }
            if !kind.is_dir() {
                return Err(self.refuse(
                    MeasurementErrorCode::ApparatusUnresolved,
                    format!("`{walked}` is not a directory"),
                ));
            }
        }
        // `ApparatusPath` refuses an empty entry and an empty segment, so
        // `split` yields at least one segment and the loop returns from it.
        Err(self.refuse(MeasurementErrorCode::ApparatusUnresolved, "names nothing"))
    }

    /// Every entry of `dir` as its exact name and its own type (a symlink is
    /// reported as one, never as what it points at), sorted by name. A
    /// directory that does not exist lists as empty.
    fn listing(&self, dir: &Path) -> Result<Vec<(String, FileType)>, MeasurementError> {
        let unreadable = |error: std::io::Error| {
            self.refuse(
                MeasurementErrorCode::ApparatusUnreadable,
                format!("`{}` cannot be listed: {error}", dir.display()),
            )
        };
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(unreadable(error)),
        };
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(unreadable)?;
            let kind = entry.file_type().map_err(unreadable)?;
            let Ok(name) = entry.file_name().into_string() else {
                return Err(self.refuse(
                    MeasurementErrorCode::ApparatusUnreadable,
                    format!(
                        "`{}` holds a name that is not UTF-8, which a collection cannot record",
                        dir.display()
                    ),
                ));
            };
            out.push((name, kind));
        }
        out.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(out)
    }

    /// Digest one resolved file into the set under its repository-relative
    /// path.
    fn digest(&mut self, path: &Path, relative: String) -> Result<(), MeasurementError> {
        let digest = quoin_store::digest_file_sha256(path).map_err(|error| {
            let cause = error
                .source()
                .map_or_else(String::new, |source| format!(": {source}"));
            let code = match error {
                quoin_store::StoreError::DigestSourceIsSymlink { .. } => {
                    MeasurementErrorCode::ApparatusSymlink
                }
                _ => MeasurementErrorCode::ApparatusUnreadable,
            };
            self.refuse(
                code,
                format!("`{relative}` cannot be digested: {error}{cause}"),
            )
        })?;
        self.files.insert(relative, digest);
        if self.files.len() > MAX_PROTECTED_FILES {
            return Err(self.too_large());
        }
        Ok(())
    }

    fn too_large(&self) -> MeasurementError {
        self.refuse(
            MeasurementErrorCode::ApparatusTooLarge,
            format!("resolves to more than {MAX_PROTECTED_FILES} files"),
        )
    }

    /// A refusal naming the plan and the entry.
    fn refuse(
        &self,
        code: MeasurementErrorCode,
        detail: impl std::fmt::Display,
    ) -> MeasurementError {
        MeasurementError::new(
            code,
            format!(
                "plan {} protected_apparatus entry `{}` {detail}",
                self.plan, self.entry
            ),
        )
    }
}

/// What the last segment of an entry is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Found {
    File,
    Directory,
    Other,
}

/// The `verificationStack.protectedApparatus` member intake writes, or `None`
/// when no plan protects anything, so the member is absent rather than empty.
pub(super) fn protected_member(
    resolved: &BTreeMap<String, ResolvedApparatus>,
) -> Option<JsonValue> {
    if resolved.is_empty() {
        return None;
    }
    let mut plans = JsonObject::new();
    for (plan, files) in resolved {
        let mut digests = JsonObject::new();
        for (path, digest) in files {
            digests.set(path.clone(), JsonValue::string(digest.to_stored()));
        }
        plans.set(plan.clone(), JsonValue::Object(digests));
    }
    Some(JsonValue::Object(plans))
}
