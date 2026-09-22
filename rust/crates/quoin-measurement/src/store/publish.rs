// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Publishing a measurement collection, write-once.
//!
//! Ports `writeMeasurementCollection` (`store.ts:31-53`).
//!
//! # What the retained code does, and what this does instead
//!
//! `store.ts` writes a `wx` temporary beside the destination and then
//! `renameSync`s it into place after an `existsSync` test. Rename **replaces**,
//! so a concurrent writer's differing bytes can be clobbered in the window
//! between the test and the rename — the exact collision the function exists to
//! refuse, turned into a silent overwrite. `atomic-file.ts`, the newer of the
//! two copies, already knows this and uses `linkSync`.
//!
//! Both unify on [`quoin_store::store::write_content_addressed`], which links
//! rather than renames, `fsync`s the file and the directory, and refuses with
//! `ContentCollision`. Two declared divergences follow, both strengthenings:
//! `store.ts`'s rename becomes a link, and durability is added.

use std::error::Error as _;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use quoin_store::{
    JsonValue, RawFileSha256Digest, canonical_json_bytes, store::write_content_addressed,
};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::plans::{PlanLoadOptions, load_measurement_plans};
use crate::raw_evidence::RawEvidencePath;
use crate::source::DiskMeasurement;
use crate::store::paths::measurement_path;
use crate::types::collection::MeasurementCollection;
use crate::types::ids::CollectionId;
use crate::validate;

/// Publish a complete collection, refusing to replace a retained one.
///
/// Returns the path written, whether or not this call was the writer: an
/// identical republication is idempotent, as `store.ts:38` is.
///
/// # Errors
///
/// [`crate::error::MeasurementErrorCode::CollectionInvalid`] when the candidate
/// is not admissible, [`crate::error::MeasurementErrorCode::CollectionIdUnsafe`]
/// when its id does not name a file,
/// [`crate::error::MeasurementErrorCode::CollectionIdCollision`] when the id is
/// already retained holding different bytes, and
/// [`crate::error::MeasurementErrorCode::PlanInvalid`] from the plan load the
/// admission check needs, [`crate::error::MeasurementErrorCode::CollectionInvalid`]
/// when a named `verificationStack.artifacts` entry is reachable under `repo`
/// and its bytes do not match the digest the candidate submitted (PLAT-931),
/// [`crate::error::MeasurementErrorCode::ArtifactNameUnsafe`] when an artifact
/// name is not a safe relative path, and
/// [`crate::error::MeasurementErrorCode::ArtifactUnreadable`] when it names an
/// entry under `repo` that cannot be digested (PLAT-969).
pub fn write_measurement_collection(
    repo: &Path,
    candidate: &JsonValue,
) -> Result<PathBuf, MeasurementError> {
    let source = DiskMeasurement::new(repo);
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())?;
    let collection = validate::measurement_collection(candidate, &plans)?;
    verify_local_artifacts(repo, &collection)?;
    let id = CollectionId::parse(collection.collection_id.as_str())?;
    let path = measurement_path(repo, &id);
    // The **caller's** value is what is written, canonicalised — not a
    // re-serialisation of the parsed collection. `store.ts:40` does the same,
    // and it is what keeps members this crate does not model from being
    // dropped on the way to disk.
    let bytes = canonical_json_bytes(candidate)?;
    write_content_addressed(&path, &bytes)?;
    Ok(path)
}

/// Truth-check the digests intake can verify for itself, refusing a record
/// whose submitted digest disagrees with the bytes it names (PLAT-931), and
/// refusing one whose named artifact cannot be checked at all (PLAT-969).
///
/// `verificationStack.lockDigest`, `.executableDigest` and `.configDigest`
/// carry no name or path in the candidate at all — only `quoin measurement
/// record --digest-from-file` (or the producer's own care) can ever check
/// those. `artifacts`, though, is a name-keyed map, and every name is tried
/// here as a `repo`-relative path under the same safety rule
/// [`crate::raw_evidence::RawEvidencePath`] applies to raw evidence. Each name
/// lands in exactly one [`LocalArtifact`] or refuses the write (FR-044-AC-6).
///
/// Until PLAT-969 an unsafe name and an entry that could not be digested were
/// both skipped with a bare `continue`, so a record naming `../outside` or a
/// directory was admitted with its digest never checked.
fn verify_local_artifacts(
    repo: &Path,
    collection: &MeasurementCollection,
) -> Result<(), MeasurementError> {
    let Some(stack) = collection.verification_stack.as_ref() else {
        return Ok(());
    };
    for (name, submitted) in &stack.artifacts {
        match reach_local_artifact(repo, name)? {
            LocalArtifact::Digested(computed) if computed == *submitted => {}
            LocalArtifact::Digested(computed) => {
                return Err(MeasurementError::new(
                    MeasurementErrorCode::CollectionInvalid,
                    format!(
                        "verificationStack.artifacts.{name} does not match the local file: the \
                         record says {}, the file digests to {}",
                        submitted.to_stored(),
                        computed.to_stored(),
                    ),
                ));
            }
            // A label, not a file this repository holds: the fixture
            // collections' own `config` is one. Its digest was checked for
            // shape by `validate::stack` and nothing here can check more.
            LocalArtifact::Label => {}
        }
    }
    Ok(())
}

/// What one `verificationStack.artifacts` name is, locally.
#[derive(Debug)]
enum LocalArtifact {
    /// The name is a regular file under the repository, digested here.
    Digested(RawFileSha256Digest),
    /// Nothing exists at the name under the repository: it labels an artifact
    /// the repository does not hold, and its digest is admitted on shape.
    Label,
}

/// Resolve one artifact name under `repo`.
///
/// Only a name with **no filesystem entry at all** is a [`LocalArtifact::Label`].
/// An entry that exists and cannot be digested — a directory, a symlink, an
/// unreadable or oversized file — is refused, because the record names a
/// local thing whose bytes intake then cannot see.
///
/// # Errors
///
/// [`MeasurementErrorCode::ArtifactNameUnsafe`] when the name is not a safe
/// relative path, and [`MeasurementErrorCode::ArtifactUnreadable`] when it
/// resolves to an entry that cannot be digested. Both name the artifact.
fn reach_local_artifact(repo: &Path, name: &str) -> Result<LocalArtifact, MeasurementError> {
    let relative = RawEvidencePath::parse(name).map_err(|error| {
        MeasurementError::new(
            MeasurementErrorCode::ArtifactNameUnsafe,
            format!(
                "verificationStack.artifacts.{name} is not a safe repository-relative path, so \
                 it cannot be checked against a local file: {}",
                error.subject()
            ),
        )
    })?;
    let path = repo.join(relative.as_str());
    // Only absence makes a label. Any other answer — an entry, or a stat that
    // failed for another reason — goes to the digest, which refuses what it
    // cannot read and says why.
    if std::fs::symlink_metadata(&path).is_err_and(|error| error.kind() == ErrorKind::NotFound) {
        return Ok(LocalArtifact::Label);
    }
    quoin_store::digest_file_sha256(&path)
        .map(LocalArtifact::Digested)
        .map_err(|error| {
            let cause = error
                .source()
                .map_or_else(String::new, |source| format!(": {source}"));
            MeasurementError::new(
                MeasurementErrorCode::ArtifactUnreadable,
                format!(
                    "verificationStack.artifacts.{name} names {} but it cannot be digested: \
                     {error}{cause}",
                    path.display()
                ),
            )
        })
}
