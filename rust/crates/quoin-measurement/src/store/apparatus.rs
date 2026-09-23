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

/// The most directories one plan's protected-apparatus walk may visit,
/// counting empty ones (PLAT-985, quoin#600 review).
///
/// [`MAX_PROTECTED_FILES`] bounds the *files* a `<directory>/**` entry
/// resolves to, but a directory that holds no file contributes nothing to
/// that count. A repository-relative tree of a million empty subdirectories
/// costs the walk a `readdir` each and resolves to zero files, so the file
/// cap alone never stops it. This bounds the walk itself, before counting
/// what it found.
pub(crate) const MAX_WALKED_DIRECTORIES: usize = 100_000;

/// The most files every plan's protected apparatus may resolve to, summed
/// across every plan one collection's write governs (PLAT-985, quoin#600
/// review).
///
/// [`MAX_PROTECTED_FILES`] bounds one plan alone; a write governed by many
/// plans, each just under that ceiling, has no bound on their sum without
/// this one.
pub(crate) const MAX_TOTAL_PROTECTED_FILES: usize = 200_000;

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
    let mut total_files = 0_usize;
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
        let mut walked_directories = 0_usize;
        for entry in protected.iter() {
            Walk {
                repo,
                plan: plan.id.as_str(),
                entry,
                files: &mut files,
                list: disk_listing,
                limit: MAX_PROTECTED_FILES,
                walked_directories: &mut walked_directories,
                directory_limit: MAX_WALKED_DIRECTORIES,
            }
            .resolve()?;
        }
        declared(collection, plan.id.as_str(), &files)?;
        total_files = total_files.saturating_add(files.len());
        enforce_total_cap(total_files)?;
        resolved.insert(plan.id.as_str().to_owned(), files);
    }
    Ok(resolved)
}

/// Refuse when `total_files`, the resolved files summed across every plan
/// this write governs so far, has passed [`MAX_TOTAL_PROTECTED_FILES`]
/// (PLAT-985, quoin#600 review).
///
/// A pure function of the running total, not the files themselves, so the
/// cap is tested against a number rather than against 200,001 real files on
/// disk.
fn enforce_total_cap(total_files: usize) -> Result<(), MeasurementError> {
    if total_files > MAX_TOTAL_PROTECTED_FILES {
        return Err(MeasurementError::new(
            MeasurementErrorCode::ApparatusTooLarge,
            format!(
                "protected apparatus resolves to more than {MAX_TOTAL_PROTECTED_FILES} files \
                 across every plan this write governs"
            ),
        ));
    }
    Ok(())
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
            // This runs before `verify_local_artifacts`, so for a protected
            // file this is the check that refuses a disagreeing digest, under
            // the same code PLAT-931's check uses for any other file.
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

/// What one directory entry is, by its own type: a symlink is reported as
/// one, never as what it points at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    File,
    Directory,
    Symlink,
    Other,
}

/// Lists one directory as exact names and their [`Kind`]s, sorted by name,
/// or says why it cannot. A directory that does not exist lists as empty.
///
/// A seam, so the exact-name match is tested against a listing that holds a
/// case-only variant whatever filesystem the tests run on.
type Lister = fn(&Path) -> Result<Vec<(String, Kind)>, String>;

/// One entry of one plan, being resolved into that plan's set.
struct Walk<'a> {
    repo: &'a Path,
    plan: &'a str,
    entry: &'a ApparatusPath,
    files: &'a mut ResolvedApparatus,
    list: Lister,
    /// The most files the plan's set may hold: [`MAX_PROTECTED_FILES`].
    limit: usize,
    /// Directories visited by this plan's walk so far, across every entry
    /// (PLAT-985, quoin#600 review). Shared across entries so a plan naming
    /// several large directory entries is bounded in total, not per entry.
    walked_directories: &'a mut usize,
    /// The most directories one plan's walk may visit, counting empty ones:
    /// [`MAX_WALKED_DIRECTORIES`].
    directory_limit: usize,
}

impl Walk<'_> {
    /// Resolve the entry, adding every file it names to the set.
    fn resolve(&mut self) -> Result<(), MeasurementError> {
        let Some(directory) = self.entry.directory() else {
            let (on_disk, found) = self.descend(self.entry.as_str(), false)?;
            let path = self.entry.as_str().to_owned();
            return match found {
                Kind::File => self.digest(&on_disk, path),
                Kind::Directory => Err(self.refuse(
                    MeasurementErrorCode::ApparatusUnresolved,
                    format!("names a directory, not a file; protect it as `{path}/**`"),
                )),
                Kind::Symlink | Kind::Other => Err(self.refuse(
                    MeasurementErrorCode::ApparatusUnreadable,
                    "names something that is not a regular file",
                )),
            };
        };
        let (at, found) = self.descend(directory, true)?;
        if found != Kind::Directory {
            return Err(self.refuse(
                MeasurementErrorCode::ApparatusUnresolved,
                format!("`{directory}` is not a directory"),
            ));
        }
        let mut pending = vec![(at, directory.to_owned())];
        let mut seen = 0_usize;
        while let Some((dir, relative)) = pending.pop() {
            *self.walked_directories += 1;
            if *self.walked_directories > self.directory_limit {
                return Err(self.refuse(
                    MeasurementErrorCode::ApparatusTooLarge,
                    format!(
                        "walks more than {} directories, counting empty ones",
                        self.directory_limit
                    ),
                ));
            }
            for (name, kind) in self.listing(&dir)? {
                let child = format!("{relative}/{name}");
                match kind {
                    Kind::Symlink => {
                        return Err(self.refuse(
                            MeasurementErrorCode::ApparatusSymlink,
                            format!("`{child}` is a symlink, which is refused and not followed"),
                        ));
                    }
                    Kind::Directory => pending.push((dir.join(&name), child)),
                    Kind::File => {
                        seen += 1;
                        self.digest(&dir.join(&name), child)?;
                    }
                    Kind::Other => {
                        return Err(self.refuse(
                            MeasurementErrorCode::ApparatusUnreadable,
                            format!("`{child}` is not a regular file"),
                        ));
                    }
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
    ) -> Result<(PathBuf, Kind), MeasurementError> {
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
            if kind == Kind::Symlink {
                return Err(self.refuse(
                    MeasurementErrorCode::ApparatusSymlink,
                    format!("`{walked}` is a symlink, which is refused and not followed"),
                ));
            }
            cursor.push(segment);
            if segments.peek().is_none() {
                return Ok((cursor, kind));
            }
            if kind != Kind::Directory {
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

    /// `dir`'s listing through [`Walk::list`], a failure refused as
    /// [`MeasurementErrorCode::ApparatusUnreadable`].
    fn listing(&self, dir: &Path) -> Result<Vec<(String, Kind)>, MeasurementError> {
        (self.list)(dir)
            .map_err(|detail| self.refuse(MeasurementErrorCode::ApparatusUnreadable, detail))
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
        if self.files.len() > self.limit {
            return Err(self.too_large());
        }
        Ok(())
    }

    fn too_large(&self) -> MeasurementError {
        self.refuse(
            MeasurementErrorCode::ApparatusTooLarge,
            format!("resolves to more than {} files", self.limit),
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

/// The repository's own listing of `dir`: see [`Lister`].
fn disk_listing(dir: &Path) -> Result<Vec<(String, Kind)>, String> {
    let unreadable =
        |error: std::io::Error| format!("`{}` cannot be listed: {error}", dir.display());
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(unreadable(error)),
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(unreadable)?;
        let file_type = entry.file_type().map_err(unreadable)?;
        let kind = if file_type.is_symlink() {
            Kind::Symlink
        } else if file_type.is_dir() {
            Kind::Directory
        } else if file_type.is_file() {
            Kind::File
        } else {
            Kind::Other
        };
        let Ok(name) = entry.file_name().into_string() else {
            return Err(format!(
                "`{}` holds a name that is not UTF-8, which a collection cannot record",
                dir.display()
            ));
        };
        out.push((name, kind));
    }
    out.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(out)
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

#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use std::path::Path;

    use engineering_assurance::measurement::ApparatusPath;

    use super::{
        Kind, MAX_PROTECTED_FILES, MAX_TOTAL_PROTECTED_FILES, MAX_WALKED_DIRECTORIES, Walk,
        disk_listing, enforce_total_cap,
    };
    use crate::error::MeasurementErrorCode;
    use crate::types::collection::ResolvedApparatus;

    /// A repository whose `harness` directory holds `Answers.json` and
    /// nothing else, whatever filesystem the test runs on.
    #[allow(
        clippy::unnecessary_wraps,
        reason = "must match the `Lister` fn-pointer signature, which can fail on a real disk"
    )]
    fn case_listing(dir: &Path) -> Result<Vec<(String, Kind)>, String> {
        Ok(match dir.to_str() {
            Some("/repo") => vec![("harness".to_owned(), Kind::Directory)],
            Some("/repo/harness") => vec![("Answers.json".to_owned(), Kind::File)],
            _ => Vec::new(),
        })
    }

    fn walk<'a>(
        repo: &'a Path,
        entry: &'a ApparatusPath,
        files: &'a mut ResolvedApparatus,
        list: super::Lister,
        limit: usize,
        walked_directories: &'a mut usize,
    ) -> Walk<'a> {
        Walk {
            repo,
            plan: "MP-1",
            entry,
            files,
            list,
            limit,
            walked_directories,
            directory_limit: MAX_WALKED_DIRECTORIES,
        }
    }

    #[test]
    fn descend_matches_each_name_exactly() {
        let entry = ApparatusPath::new("harness/answers.json").unwrap();
        let mut files = ResolvedApparatus::new();
        let repo = Path::new("/repo");
        let mut walked_directories = 0_usize;
        let walker = walk(
            repo,
            &entry,
            &mut files,
            case_listing,
            MAX_PROTECTED_FILES,
            &mut walked_directories,
        );
        let error = walker.descend("harness/answers.json", false).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ApparatusUnresolved);
        assert!(
            error
                .subject()
                .contains("`harness/answers.json` does not exist")
        );
        let (found, kind) = walker.descend("harness/Answers.json", false).unwrap();
        assert_eq!(found, Path::new("/repo/harness/Answers.json"));
        assert_eq!(kind, Kind::File);
    }

    #[test]
    fn a_set_past_the_limit_is_too_large() {
        assert_eq!(MAX_PROTECTED_FILES, 50_000);
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        std::fs::create_dir_all(root.join("labels")).unwrap();
        for name in ["a", "b", "c"] {
            std::fs::write(root.join("labels").join(name), name).unwrap();
        }
        let entry = ApparatusPath::new("labels/**").unwrap();
        let mut files = ResolvedApparatus::new();
        let mut walked_directories = 0_usize;
        let error = walk(
            root,
            &entry,
            &mut files,
            disk_listing,
            2,
            &mut walked_directories,
        )
        .resolve()
        .unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ApparatusTooLarge);
        assert!(error.subject().contains("more than 2 files"));

        let mut files = ResolvedApparatus::new();
        let mut walked_directories = 0_usize;
        walk(
            root,
            &entry,
            &mut files,
            disk_listing,
            3,
            &mut walked_directories,
        )
        .resolve()
        .unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn a_walk_past_the_directory_limit_is_too_large_even_with_no_files() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        // Ten empty subdirectories: zero files resolved, so
        // `MAX_PROTECTED_FILES` alone never stops this walk.
        for index in 0..10 {
            std::fs::create_dir_all(root.join("empty").join(index.to_string())).unwrap();
        }
        let entry = ApparatusPath::new("empty/**").unwrap();
        let mut files = ResolvedApparatus::new();
        let mut walked_directories = 0_usize;
        let error = Walk {
            repo: root,
            plan: "MP-1",
            entry: &entry,
            files: &mut files,
            list: disk_listing,
            limit: MAX_PROTECTED_FILES,
            walked_directories: &mut walked_directories,
            directory_limit: 5,
        }
        .resolve()
        .unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ApparatusTooLarge);
        assert!(error.subject().contains("more than 5 directories"));
    }

    #[test]
    fn the_cross_plan_total_is_capped_independent_of_any_one_plan() {
        assert_eq!(MAX_TOTAL_PROTECTED_FILES, 200_000);
        enforce_total_cap(MAX_TOTAL_PROTECTED_FILES).unwrap();
        let error = enforce_total_cap(MAX_TOTAL_PROTECTED_FILES + 1).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ApparatusTooLarge);
        assert!(error.subject().contains("more than 200000 files"));
    }
}
