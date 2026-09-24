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
//!
//! # One computed member, merged into the caller's own bytes
//!
//! [`write_measurement_collection`] writes the **caller's** JSON, canonicalised
//! — not a re-serialisation of the parsed collection — so that members this
//! crate does not model are never dropped on the way to disk. PLAT-969's
//! ruling adds one exception: `verificationStack.unverifiedArtifacts`,
//! the sorted list of `artifacts` names with no local filesystem entry, is
//! something no caller can state honestly for itself (only this repository
//! knows what it holds), so intake computes it and merges it into a clone of
//! the candidate before canonicalising, and refuses a candidate that states
//! it. PLAT-975 adds the second, under the same refusal:
//! `verificationStack.protectedApparatus`, each governing plan's resolved
//! protected apparatus with every file's digest, which intake resolves from
//! the repository itself (see [`super::apparatus`]). Nothing else about the
//! candidate is touched.

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
use crate::store::apparatus;
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
/// entry under `repo` that cannot be digested (PLAT-969), and the
/// `QM-APPARATUS-*` refusals when a governing plan's protected apparatus
/// cannot be resolved, or is not declared in `artifacts` (PLAT-975).
pub fn write_measurement_collection(
    repo: &Path,
    candidate: &JsonValue,
) -> Result<PathBuf, MeasurementError> {
    let source = DiskMeasurement::new(repo);
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())?;
    write_measurement_collection_with_plans(repo, candidate, &plans)
}

/// Publish a campaign collection against its exact source-bound selected plans.
/// The caller must have loaded and validated these plans from the verified
/// campaign source tree. Admission, apparatus, and artifact checks are shared
/// with ordinary collection publication.
pub(crate) fn write_measurement_collection_with_plans(
    repo: &Path,
    candidate: &JsonValue,
    plans: &[crate::types::plan::MeasurementPlan],
) -> Result<PathBuf, MeasurementError> {
    let collection = validate::measurement_collection(candidate, plans)?;
    // The protected apparatus first, so a protected file that is a symlink
    // or unreadable is refused under its own `QM-APPARATUS-*` code rather
    // than as an ordinary artifact.
    let protected = apparatus::resolve_protected(repo, &collection, plans)?;
    let unverified = verify_local_artifacts(repo, &collection)?;
    let id = CollectionId::parse(collection.collection_id.as_str())?;
    let path = measurement_path(repo, &id);
    // The **caller's** value is what is written, canonicalised — not a
    // re-serialisation of the parsed collection. `store.ts:40` does the same,
    // and it is what keeps members this crate does not model from being
    // dropped on the way to disk. `unverifiedArtifacts`
    // and `protectedApparatus` are the computed exceptions (PLAT-969,
    // PLAT-975): see this module's header.
    let written = with_computed_members(
        candidate,
        &unverified,
        apparatus::protected_member(&protected),
    );
    let bytes = canonical_json_bytes(&written)?;
    write_content_addressed(&path, &bytes)?;
    Ok(path)
}

/// Merge the computed `unverifiedArtifacts` list and `protectedApparatus`
/// record into a clone of the candidate's own `verificationStack`.
///
/// `validate::measurement_collection` has already refused a candidate that
/// states either member, so nothing the caller sent is overwritten. Each is
/// set only when it has something to say, so a collection with nothing
/// unverified and no protecting plan states neither rather than an empty
/// value — the same "absent, not empty" rule the read side
/// (`validate::stack`) applies back.
///
/// `write_measurement_collection` only reaches this after
/// `validate::measurement_collection` has already confirmed `verificationStack`
/// is an object (a schemaVersion-2 collection always carries one); if it is
/// somehow not, the candidate is returned unchanged rather than losing
/// whatever the caller actually sent.
fn with_computed_members(
    candidate: &JsonValue,
    names: &[String],
    protected: Option<JsonValue>,
) -> JsonValue {
    let mut written = candidate.clone();
    let JsonValue::Object(root) = &mut written else {
        return written;
    };
    let Some(JsonValue::Object(mut stack)) = root.get("verificationStack").cloned() else {
        return written;
    };
    if !names.is_empty() {
        let array = JsonValue::Array(names.iter().cloned().map(JsonValue::string).collect());
        stack.set("unverifiedArtifacts", array);
    }
    if let Some(record) = protected {
        stack.set("protectedApparatus", record);
    }
    root.set("verificationStack", JsonValue::Object(stack));
    written
}

/// Truth-check the digests intake can verify for itself, refusing a record
/// whose submitted digest disagrees with the bytes it names (PLAT-931), and
/// refusing one whose named artifact cannot be checked at all (PLAT-969).
/// Returns the sorted names that turned out to be labels — nothing under
/// `repo` at all — for [`write_measurement_collection`] to record honestly
/// rather than pass over in silence (PLAT-969's ruling: a label stays
/// admitted, but it is never unstated).
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
) -> Result<Vec<String>, MeasurementError> {
    let Some(stack) = collection.verification_stack.as_ref() else {
        return Ok(Vec::new());
    };
    // `stack.artifacts` is a `BTreeMap`, so iterating it already visits names
    // in sorted order — nothing further to sort before this is stored as
    // `unverifiedArtifacts`.
    let mut unverified = Vec::new();
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
            // shape by `validate::stack` and nothing here can check more, so
            // it is recorded as unverified rather than checked further.
            LocalArtifact::Label => unverified.push(name.clone()),
        }
    }
    Ok(unverified)
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
/// local thing whose bytes intake then cannot see. A symlink anywhere
/// *before* the final component is refused the same way, before the
/// NotFound-or-digest decision is even made (PLAT-969 F3): otherwise a
/// directory the name walks through, not just the name's last segment,
/// silently redirects the check — `dist -> /elsewhere` would digest a file
/// this repository does not hold, and `dist -> /missing` would `lstat` as
/// `NotFound` and pass as a label, both without the write ever refusing.
///
/// # Errors
///
/// [`MeasurementErrorCode::ArtifactNameUnsafe`] when the name is not a safe
/// relative path, and [`MeasurementErrorCode::ArtifactUnreadable`] when it
/// resolves to an entry that cannot be digested, or passes through a
/// symlinked component. Both name the artifact.
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
    if let Some(component) = first_symlink_ancestor(repo, relative.as_str()) {
        return Err(MeasurementError::new(
            MeasurementErrorCode::ArtifactUnreadable,
            format!(
                "verificationStack.artifacts.{name} names {} but `{component}` is a symlink, so \
                 the path cannot be checked against a local file",
                path.display()
            ),
        ));
    }
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

/// The first path component before `relative`'s final one that is a symlink
/// on disk under `repo`, checked left to right so the first one found is the
/// one a refusal names.
///
/// Walked against the real filesystem rather than decided lexically: a safe
/// *name* (no `..`, no leading `/`) says nothing about what the filesystem
/// actually put at each component, and only `symlink_metadata` can see that a
/// component the name calls a directory is a link elsewhere. The final
/// component's own symlink-ness is left to `digest_file_sha256`, which
/// already refuses it.
fn first_symlink_ancestor<'name>(repo: &Path, relative: &'name str) -> Option<&'name str> {
    let mut cursor = repo.to_path_buf();
    let mut segments = relative.split('/').peekable();
    while let Some(segment) = segments.next() {
        segments.peek()?;
        cursor.push(segment);
        let is_symlink = std::fs::symlink_metadata(&cursor)
            .is_ok_and(|metadata| metadata.file_type().is_symlink());
        if is_symlink {
            return Some(segment);
        }
    }
    None
}
