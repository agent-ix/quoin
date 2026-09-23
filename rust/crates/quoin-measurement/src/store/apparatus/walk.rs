// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The resolver's walk: one protected-apparatus entry of one plan, resolved
//! against the repository into that plan's set (PLAT-975, engineering-assurance
//! FR-024), under the write's [`Limits`] (PLAT-985).
//!
//! Split out of [`super`] when PLAT-985's limits and their tests took that
//! module past the 700-line ceiling (quoin#464): this is the part that touches
//! the filesystem, and [`super`] is the part that decides what a write records.

use std::error::Error as _;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use engineering_assurance::measurement::ApparatusPath;

use super::{Limits, Spent};
use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::types::collection::{ResolvedApparatus, unrecordable_apparatus_path};

/// What one directory entry is, by its own type: a symlink is reported as
/// one, never as what it points at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
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
pub(super) type Lister = fn(&Path) -> Result<Vec<(String, Kind)>, String>;

/// One entry of one plan, being resolved into that plan's set.
pub(super) struct Walk<'a> {
    pub(super) repo: &'a Path,
    pub(super) plan: &'a str,
    pub(super) entry: &'a ApparatusPath,
    pub(super) files: &'a mut ResolvedApparatus,
    pub(super) list: Lister,
    /// The ceilings: [`super::LIMITS`] outside tests.
    pub(super) limits: Limits,
    /// What the write has spent so far, shared by every entry of every plan
    /// (PLAT-985, quoin#600 review), so the write-wide limits bound the total
    /// rather than each entry or plan alone.
    pub(super) spent: &'a mut Spent,
}

impl Walk<'_> {
    /// Resolve the entry, adding every file it names to the set.
    pub(super) fn resolve(&mut self) -> Result<(), MeasurementError> {
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
            self.spent.directories += 1;
            if self.spent.directories > self.limits.directories {
                return Err(self.refuse(
                    MeasurementErrorCode::ApparatusTooLarge,
                    format!(
                        "walks more than {} directories, counting empty ones, across every plan \
                         this write governs",
                        self.limits.directories
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
    ///
    /// A path a stored record could not hold is refused before the file is
    /// opened: writing it would store a record the reader refuses forever.
    fn digest(&mut self, path: &Path, relative: String) -> Result<(), MeasurementError> {
        if let Some(reason) = unrecordable_apparatus_path(&relative) {
            return Err(self.refuse(
                MeasurementErrorCode::ApparatusUnreadable,
                format!("`{relative}` cannot be recorded in a collection: {reason}"),
            ));
        }
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
        if self.files.insert(relative, digest).is_none() {
            self.spent.files += 1;
        }
        if self.files.len() > self.limits.plan_files {
            return Err(self.refuse(
                MeasurementErrorCode::ApparatusTooLarge,
                format!("resolves to more than {} files", self.limits.plan_files),
            ));
        }
        if self.spent.files > self.limits.total_files {
            return Err(self.refuse(
                MeasurementErrorCode::ApparatusTooLarge,
                format!(
                    "resolves to more than {} files across every plan this write governs",
                    self.limits.total_files
                ),
            ));
        }
        Ok(())
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
pub(super) fn disk_listing(dir: &Path) -> Result<Vec<(String, Kind)>, String> {
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

#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use std::path::Path;

    use engineering_assurance::measurement::ApparatusPath;

    use super::super::{
        LIMITS, Limits, MAX_PROTECTED_FILES, MAX_TOTAL_PROTECTED_FILES, MAX_WALKED_DIRECTORIES,
        Spent,
    };
    use super::{Kind, Walk, disk_listing};
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

    /// Resolve `entry` under `root` as plan `plan`, against `limits`, into a
    /// fresh set, charging `spent`.
    fn resolve(
        root: &Path,
        plan: &str,
        entry: &str,
        limits: Limits,
        spent: &mut Spent,
    ) -> Result<ResolvedApparatus, crate::error::MeasurementError> {
        let entry = ApparatusPath::new(entry).unwrap();
        let mut files = ResolvedApparatus::new();
        Walk {
            repo: root,
            plan,
            entry: &entry,
            files: &mut files,
            list: disk_listing,
            limits,
            spent,
        }
        .resolve()?;
        Ok(files)
    }

    fn limits(plan_files: usize, directories: usize, total_files: usize) -> Limits {
        Limits {
            plan_files,
            directories,
            total_files,
        }
    }

    #[test]
    fn descend_matches_each_name_exactly() {
        let entry = ApparatusPath::new("harness/answers.json").unwrap();
        let mut files = ResolvedApparatus::new();
        let repo = Path::new("/repo");
        let mut spent = Spent::default();
        let walker = Walk {
            repo,
            plan: "MP-1",
            entry: &entry,
            files: &mut files,
            list: case_listing,
            limits: LIMITS,
            spent: &mut spent,
        };
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
        let error = resolve(
            root,
            "MP-1",
            "labels/**",
            limits(2, MAX_WALKED_DIRECTORIES, MAX_TOTAL_PROTECTED_FILES),
            &mut Spent::default(),
        )
        .unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ApparatusTooLarge);
        assert!(error.subject().contains("more than 2 files"));

        let files = resolve(
            root,
            "MP-1",
            "labels/**",
            limits(3, MAX_WALKED_DIRECTORIES, MAX_TOTAL_PROTECTED_FILES),
            &mut Spent::default(),
        )
        .unwrap();
        assert_eq!(files.len(), 3);
    }

    /// `empty/` holds ten empty subdirectories, so the walk visits eleven
    /// directories and finds no file: [`MAX_PROTECTED_FILES`] alone never
    /// stops it. Past the directory limit it is too large before it can be
    /// "holds no file"; at the limit the walk completes, and the same limit is
    /// shared by a second entry of the same write.
    #[test]
    fn a_walk_past_the_directory_limit_is_too_large_even_with_no_files() {
        assert_eq!(MAX_WALKED_DIRECTORIES, 100_000);
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        for index in 0..10 {
            std::fs::create_dir_all(root.join("empty").join(index.to_string())).unwrap();
        }
        std::fs::create_dir_all(root.join("other")).unwrap();
        std::fs::write(root.join("other").join("f"), "f").unwrap();

        let error = resolve(
            root,
            "MP-1",
            "empty/**",
            limits(MAX_PROTECTED_FILES, 10, MAX_TOTAL_PROTECTED_FILES),
            &mut Spent::default(),
        )
        .unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ApparatusTooLarge);
        assert!(error.subject().contains("more than 10 directories"));

        // At the limit, all eleven are walked; with no file the refusal is
        // the ordinary empty-directory one, not a size refusal.
        let mut spent = Spent::default();
        let error = resolve(
            root,
            "MP-1",
            "empty/**",
            limits(MAX_PROTECTED_FILES, 11, MAX_TOTAL_PROTECTED_FILES),
            &mut spent,
        )
        .unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ApparatusUnresolved);
        assert_eq!(spent.directories, 11);

        // The count is the write's, not the entry's or the plan's: a second
        // plan's one-directory walk after those eleven passes the limit.
        let error = resolve(
            root,
            "MP-2",
            "other/**",
            limits(MAX_PROTECTED_FILES, 11, MAX_TOTAL_PROTECTED_FILES),
            &mut spent,
        )
        .unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ApparatusTooLarge);
        assert!(error.subject().contains("plan MP-2"));
    }

    /// Two plans each resolving two files, each well inside its own limit:
    /// the write-wide total refuses the second plan past it and not at it.
    #[test]
    fn the_cross_plan_total_is_capped_independent_of_any_one_plan() {
        assert_eq!(MAX_TOTAL_PROTECTED_FILES, 200_000);
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        for (dir, name) in [
            ("first", "a"),
            ("first", "b"),
            ("second", "c"),
            ("second", "d"),
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(name), name).unwrap();
        }

        let mut spent = Spent::default();
        let under = limits(MAX_PROTECTED_FILES, MAX_WALKED_DIRECTORIES, 3);
        resolve(root, "MP-1", "first/**", under, &mut spent).unwrap();
        let error = resolve(root, "MP-2", "second/**", under, &mut spent).unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::ApparatusTooLarge);
        assert!(
            error
                .subject()
                .contains("more than 3 files across every plan this write governs"),
            "{error}"
        );

        let mut spent = Spent::default();
        let at = limits(MAX_PROTECTED_FILES, MAX_WALKED_DIRECTORIES, 4);
        resolve(root, "MP-1", "first/**", at, &mut spent).unwrap();
        resolve(root, "MP-2", "second/**", at, &mut spent).unwrap();
        assert_eq!(spent.files, 4);
    }

    /// A protected file whose name a stored record cannot hold is refused at
    /// intake, rather than written into a record the reader then refuses as
    /// `QM-COLLECTION-INVALID` forever.
    #[test]
    fn a_file_name_a_record_cannot_hold_is_refused_at_intake() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        std::fs::create_dir_all(root.join("labels")).unwrap();
        std::fs::write(root.join("labels").join("ok.json"), "ok").unwrap();
        for name in ["a:b.json", "**", "x?.json"] {
            let bad = root.join("labels").join(name);
            std::fs::write(&bad, "bad").unwrap();
            let error =
                resolve(root, "MP-1", "labels/**", LIMITS, &mut Spent::default()).unwrap_err();
            assert_eq!(
                error.code(),
                MeasurementErrorCode::ApparatusUnreadable,
                "{name}"
            );
            assert!(
                error
                    .subject()
                    .contains("cannot be recorded in a collection"),
                "{error}"
            );
            std::fs::remove_file(bad).unwrap();
        }
        let files = resolve(root, "MP-1", "labels/**", LIMITS, &mut Spent::default()).unwrap();
        assert_eq!(files.len(), 1);
    }
}
