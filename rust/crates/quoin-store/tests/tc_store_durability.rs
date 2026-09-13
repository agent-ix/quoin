// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What an interrupted write and a concurrent write leave behind.
//!
//! The store's atomicity claim is not "the writer finishes". It is that a
//! reader never observes a state the writer did not intend to publish: either
//! the previous record or the complete new one, never a prefix of either, and
//! never a temporary mistaken for a record.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use quoin_store::parse_strict_json;
use quoin_store::store::{
    recover_orphaned_temporaries, store_root, write_atomic, write_canonical,
    write_content_addressed,
};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "quoin-store-durability-{}-{name}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn temporaries_in(directory: &Path) -> Vec<String> {
    std::fs::read_dir(directory)
        .map(|read| {
            read.flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with('.') && name.contains(".tmp-"))
                .collect()
        })
        .unwrap_or_default()
}

/// A write interrupted before its rename publishes nothing, and the temporary
/// it left is removed by the next recovery run.
///
/// The temporary carries a *torn* body on purpose. If a reader could ever
/// reach it, this is the byte sequence that would be read as a record; the
/// assertion is that the record path still holds the previous complete record
/// and that the torn bytes are never reachable under any record name.
///
/// Trace: FR-100-AC-9
#[test]
fn tc_380_an_interrupted_write_leaves_no_torn_record_and_no_orphaned_temporary() {
    let scratch = scratch("interrupted");
    let records = store_root(&scratch).join("change-assurance/records");
    std::fs::create_dir_all(&records).unwrap();
    let record = records.join("record.json");

    let complete = br#"{"schemaVersion":1,"value":"first"}"#;
    write_atomic(&record, complete).unwrap();

    // Exactly what a killed process leaves: a dot-prefixed sibling holding a
    // prefix of the bytes that were never renamed into place.
    let orphan = records.join(".record.json.tmp-999999-0");
    std::fs::write(&orphan, br#"{"schemaVersion":1,"value":"seco"#).unwrap();

    // The record path is untouched: the previous complete record, not a torn
    // one, and the torn bytes are under no name a reader addresses.
    assert_eq!(std::fs::read(&record).unwrap(), complete);
    parse_strict_json(&std::fs::read(&record).unwrap()).expect("a complete record");
    let readable: Vec<PathBuf> = std::fs::read_dir(&records)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    assert_eq!(readable, vec![record.clone()]);

    // The next run recovers the orphan, and recovery is idempotent.
    assert_eq!(recover_orphaned_temporaries(&records).unwrap(), 1);
    assert!(temporaries_in(&records).is_empty());
    assert_eq!(recover_orphaned_temporaries(&records).unwrap(), 0);
    assert_eq!(std::fs::read(&record).unwrap(), complete);

    // A recovery run over a directory that never existed is zero, not an error.
    assert_eq!(
        recover_orphaned_temporaries(&scratch.join("absent")).unwrap(),
        0
    );
    std::fs::remove_dir_all(&scratch).ok();
}

/// A reader racing a writer sees one complete record or the other, never a
/// prefix of either, and the writer leaves no temporary behind.
///
/// Trace: FR-100-AC-9
#[test]
fn tc_380_a_reader_racing_a_writer_never_observes_a_torn_record() {
    let scratch = scratch("torn");
    let records = store_root(&scratch).join("change-assurance/records");
    std::fs::create_dir_all(&records).unwrap();
    let record = records.join("record.json");

    let short = br#"{"schemaVersion":1,"value":"a"}"#.to_vec();
    let long = format!(r#"{{"schemaVersion":1,"value":"{}"}}"#, "b".repeat(200_000)).into_bytes();
    write_atomic(&record, &short).unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let reads = Arc::new(AtomicUsize::new(0));
    let reader = {
        let record = record.clone();
        let stop = Arc::clone(&stop);
        let reads = Arc::clone(&reads);
        let short = short.clone();
        let long = long.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let Ok(bytes) = std::fs::read(&record) else {
                    continue;
                };
                assert!(
                    bytes == short || bytes == long,
                    "observed a record that is neither version: {} bytes",
                    bytes.len()
                );
                parse_strict_json(&bytes).expect("every observable state parses");
                reads.fetch_add(1, Ordering::Relaxed);
            }
        })
    };

    for round in 0..200 {
        let bytes = if round % 2 == 0 { &long } else { &short };
        write_atomic(&record, bytes).unwrap();
    }
    stop.store(true, Ordering::Relaxed);
    reader.join().expect("the reader saw only complete records");

    assert!(reads.load(Ordering::Relaxed) > 0, "the reader never ran");
    assert!(temporaries_in(&records).is_empty());
    std::fs::remove_dir_all(&scratch).ok();
}

/// Concurrent writers against one store root produce a readable store with
/// every completed record intact.
///
/// Trace: FR-100-AC-9
#[test]
fn tc_380_concurrent_writes_to_one_store_root_keep_every_completed_record() {
    let scratch = scratch("concurrent");
    let root = store_root(&scratch).join("change-assurance/records");
    std::fs::create_dir_all(&root).unwrap();

    let writers = 8_usize;
    let per_writer = 25_usize;
    let mut handles = Vec::new();
    for writer in 0..writers {
        let root = root.clone();
        handles.push(std::thread::spawn(move || {
            for index in 0..per_writer {
                let value = quoin_store::parse_strict_json_str(&format!(
                    r#"{{"schemaVersion":1,"writer":{writer},"index":{index}}}"#
                ))
                .unwrap();
                write_canonical(&root.join(format!("r-{writer}-{index}.json")), &value).unwrap();
            }
        }));
    }
    for handle in handles {
        handle.join().expect("no writer failed");
    }

    for writer in 0..writers {
        for index in 0..per_writer {
            let bytes = std::fs::read(root.join(format!("r-{writer}-{index}.json"))).unwrap();
            // The whole record, byte for byte: a record that is merely
            // parseable is not a record that survived intact.
            let expected = quoin_store::canonical_json_bytes(
                &quoin_store::parse_strict_json_str(&format!(
                    r#"{{"schemaVersion":1,"writer":{writer},"index":{index}}}"#
                ))
                .unwrap(),
            )
            .unwrap();
            assert_eq!(bytes, expected, "writer {writer} record {index}");
            parse_strict_json(&bytes).expect("every completed record parses");
        }
    }
    assert!(temporaries_in(&root).is_empty());
    std::fs::remove_dir_all(&scratch).ok();
}

/// Concurrent content-addressed writers of *differing* bytes: one publishes,
/// the rest are refused. No writer's bytes are clobbered by another's.
///
/// Trace: FR-100-AC-9
#[test]
fn tc_380_concurrent_content_addressed_writers_refuse_rather_than_clobber() {
    let scratch = scratch("content-addressed");
    let root = store_root(&scratch).join("change-assurance/outputs");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("output.bin");

    let written = Arc::new(AtomicUsize::new(0));
    let refused = Arc::new(AtomicUsize::new(0));
    let matched = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for writer in 0_usize..8 {
        let path = path.clone();
        let written = Arc::clone(&written);
        let refused = Arc::clone(&refused);
        let matched = Arc::clone(&matched);
        handles.push(std::thread::spawn(move || {
            let bytes = vec![u8::try_from(writer).unwrap(); 4096];
            match write_content_addressed(&path, &bytes) {
                Ok(true) => written.fetch_add(1, Ordering::Relaxed),
                // `Ok(false)` means "already present with exactly these
                // bytes", which cannot happen when every writer's bytes
                // differ. Counted rather than panicked so the assertion is
                // reported by the test thread, not lost in a worker.
                Ok(false) => matched.fetch_add(1, Ordering::Relaxed),
                Err(_) => refused.fetch_add(1, Ordering::Relaxed),
            };
        }));
    }
    for handle in handles {
        handle.join().expect("no writer panicked");
    }

    assert_eq!(
        matched.load(Ordering::Relaxed),
        0,
        "differing bytes must never compare equal"
    );
    assert_eq!(written.load(Ordering::Relaxed), 1, "exactly one publisher");
    assert_eq!(refused.load(Ordering::Relaxed), 7, "the rest are refused");
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 4096);
    assert!(
        bytes.iter().all(|byte| *byte == bytes[0]),
        "the published bytes are one writer's, not a mixture"
    );
    assert!(temporaries_in(&root).is_empty());
    std::fs::remove_dir_all(&scratch).ok();
}
