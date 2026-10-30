// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The bounded, no-follow read of one regular file.
//!
//! This crate computes no digest of anything: change-assurance records,
//! attestations and receipts are addressed by `(record_id, revision)` or an
//! assigned identity, and experiment and operational records carry an opaque
//! UUID.

use crate::error::StoreError;

/// Read one regular file through a no-follow, nonblocking descriptor, bounded
/// by a caller-owned byte limit.
///
/// # The open is race-safe (PLAT-985, quoin#600 review)
///
/// The path is opened exactly once, with `O_NOFOLLOW` (refuse a symlink at the
/// final component, atomically with the open) and `O_NONBLOCK` (a FIFO opens
/// immediately rather than blocking for a writer). The file-type check that
/// follows inspects the *open handle* (`fstat`, not `stat`), and every read is
/// against that same handle, bounded one byte past the size `fstat` reported,
/// so a file that grows after the check cannot make the read allocate past
/// `limit`. A `symlink_metadata`-then-`read` pair would name the path twice and
/// let a swap land between them.
///
/// On Windows - a release target, where neither flag exists - the open passes
/// `FILE_FLAG_OPEN_REPARSE_POINT` instead, so a symlink is opened as itself
/// and the handle's metadata reports it as a symlink, which the type check
/// refuses.
///
/// # Errors
///
/// Refuses, in that order: a symbolic link, anything that is not a regular
/// file, a file larger than `limit`, and a file whose length changed between
/// the open and the read. Also refuses on any I/O failure opening or reading
/// the file.
pub fn read_regular_file_bounded(
    path: &std::path::Path,
    limit: u64,
) -> Result<Vec<u8>, StoreError> {
    read_opened_file_bounded(&open_no_follow(path)?, path, limit)
}

/// Open `path` for reading, once, without following a symlink
/// at the final component and without blocking on a FIFO.
fn open_no_follow(path: &std::path::Path) -> Result<std::fs::File, StoreError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // From `winbase.h`; std names neither constant, and two flags do not
        // justify a `windows-sys` pin. `BACKUP_SEMANTICS` lets a directory
        // open, so it is refused by the type check as on unix rather than as
        // an I/O failure.
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
    }
    options.open(path).map_err(|source| {
        // `O_NOFOLLOW` turns a symlink at the final component into ELOOP
        // rather than a followed open, so that failure is reported under the
        // same code a pre-open `symlink_metadata` check would have used, not
        // folded into the generic `Io` variant.
        if refused_as_symlink(&source) {
            StoreError::ReadSourceIsSymlink {
                path: path.to_path_buf(),
            }
        } else {
            StoreError::Io {
                operation: "open",
                path: path.to_path_buf(),
                source,
            }
        }
    })
}

/// Whether an open failed because `O_NOFOLLOW` met a symlink.
#[cfg(unix)]
fn refused_as_symlink(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(libc::ELOOP)
}

/// Whether an open failed because it met a symlink: never, off unix, where the
/// open does not refuse one and the handle's type check does.
#[cfg(not(unix))]
fn refused_as_symlink(_error: &std::io::Error) -> bool {
    false
}

fn read_opened_file_bounded(
    file: &std::fs::File,
    path: &std::path::Path,
    limit: u64,
) -> Result<Vec<u8>, StoreError> {
    let metadata = file.metadata().map_err(|source| StoreError::Io {
        operation: "fstat",
        path: path.to_path_buf(),
        source,
    })?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        // `O_NOFOLLOW` already refuses a symlink at open on unix; on Windows
        // the reparse point is opened as itself and refused here.
        return Err(StoreError::ReadSourceIsSymlink {
            path: path.to_path_buf(),
        });
    }
    if !file_type.is_file() {
        return Err(StoreError::ReadSourceNotRegularFile {
            path: path.to_path_buf(),
            kind: if file_type.is_dir() {
                "directory"
            } else {
                "not a regular file"
            },
        });
    }
    let declared = metadata.len();
    if declared > limit {
        return Err(StoreError::ReadSourceTooLarge {
            path: path.to_path_buf(),
            size: declared,
            limit,
        });
    }
    let mut bytes = Vec::new();
    // One byte past the declared size is enough to see that it grew.
    std::io::Read::read_to_end(
        &mut std::io::Read::take(file, declared.saturating_add(1)),
        &mut bytes,
    )
    .map_err(|source| StoreError::Io {
        operation: "read",
        path: path.to_path_buf(),
        source,
    })?;
    let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if actual != declared {
        return Err(StoreError::ReadSourceChangedWhileReading {
            path: path.to_path_buf(),
            expected: declared,
            actual,
        });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use super::{open_no_follow, read_opened_file_bounded, read_regular_file_bounded};
    use crate::error::StoreErrorCode;

    const LIMIT: u64 = 1024;

    /// Trace: FR-098-CON-1
    #[test]
    fn tc_380_bounded_read_refuses_symlinks_directories_and_oversize_files() {
        let dir = std::env::temp_dir().join(format!("quoin-store-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("payload.bin");
        std::fs::write(&file, b"abc").expect("write");
        assert_eq!(read_regular_file_bounded(&file, LIMIT).expect("reads"), b"abc");

        let error = read_regular_file_bounded(&dir, LIMIT).expect_err("a directory has no bytes");
        assert_eq!(error.code(), StoreErrorCode::ReadSourceNotRegularFile);

        let error = read_regular_file_bounded(&file, 2).expect_err("over the limit");
        assert_eq!(error.code(), StoreErrorCode::ReadSourceTooLarge);

        let link = dir.join("payload.link");
        let _ = std::fs::remove_file(&link);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&file, &link).expect("symlink");
            let error = read_regular_file_bounded(&link, LIMIT).expect_err("a link is not its target");
            assert_eq!(error.code(), StoreErrorCode::ReadSourceIsSymlink);
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A FIFO with no writer is refused as not a regular file, promptly:
    /// `O_NONBLOCK` makes the open itself return instead of waiting for a
    /// writer, and the type check on the open handle refuses it before any
    /// read. Without `O_NONBLOCK` the open blocks until a writer appears, so
    /// the call runs on a thread with a bounded wait and a regression reports
    /// as a failed assertion, not a hung test process.
    ///
    /// Trace: FR-110-AC-10
    /// Provenance: PLAT-985, quoin#600
    #[cfg(unix)]
    #[test]
    fn tc_985_a_fifo_with_no_writer_fails_fast_rather_than_blocking() {
        let dir = tempdir("fifo");
        let fifo = dir.join("blocking.pipe");
        make_fifo(&fifo);

        let (sender, receiver) = std::sync::mpsc::channel();
        let path = fifo.clone();
        std::thread::spawn(move || {
            let _ = sender.send(read_regular_file_bounded(&path, LIMIT));
        });
        let outcome = receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect(
                "read_regular_file_bounded blocked past the timeout instead of failing fast on a \
                 FIFO with no writer",
            );
        let error = outcome.expect_err("a FIFO is not a regular file and must be refused");
        assert_eq!(error.code(), StoreErrorCode::ReadSourceNotRegularFile);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The type check and the read observe the file that was opened, not
    /// whatever the path names afterwards. The file is opened, then the path
    /// is swapped - first for a FIFO, then for a symlink to other bytes -
    /// before the handle is read; the bytes are still the opened file's.
    ///
    /// Trace: FR-110-AC-10
    /// Provenance: PLAT-985, quoin#600
    #[cfg(unix)]
    #[test]
    fn tc_985_the_read_uses_the_opened_handle_not_the_path() {
        let dir = tempdir("swap");
        let path = dir.join("answers.json");
        let other = dir.join("other.json");
        std::fs::write(&other, b"xyz").expect("write");

        std::fs::write(&path, b"abc").expect("write");
        let file = open_no_follow(&path).expect("opens");
        std::fs::remove_file(&path).expect("unlink");
        make_fifo(&path);
        assert_eq!(
            read_opened_file_bounded(&file, &path, LIMIT).expect("reads the opened file"),
            b"abc"
        );

        std::fs::remove_file(&path).expect("unlink");
        std::fs::write(&path, b"abc").expect("write");
        let file = open_no_follow(&path).expect("opens");
        std::fs::remove_file(&path).expect("unlink");
        std::os::unix::fs::symlink(&other, &path).expect("symlink");
        assert_eq!(
            read_opened_file_bounded(&file, &path, LIMIT).expect("reads the opened file"),
            b"abc"
        );

        // The path as it now reads is refused, so the two answers above came
        // from the handle and not from a second look at the path.
        let error = read_regular_file_bounded(&path, LIMIT).expect_err("a link is not its target");
        assert_eq!(error.code(), StoreErrorCode::ReadSourceIsSymlink);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A fresh directory under the system temporary directory, unique to this
    /// process and `label`.
    #[cfg(unix)]
    fn tempdir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("quoin-store-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    #[cfg(unix)]
    fn make_fifo(path: &std::path::Path) {
        let made = std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("mkfifo is on PATH in this test environment");
        assert!(made.success(), "mkfifo {path:?} failed");
    }
}
