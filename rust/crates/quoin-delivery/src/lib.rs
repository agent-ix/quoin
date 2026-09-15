// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native release delivery for the `quoin` executable.
//!
//! The crate owns the untrusted release-manifest, archive and filesystem
//! boundary. The CLI supplies the running version and renders the result; this
//! crate neither parses command-line arguments nor prints to an operator.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use tar::Archive;
use url::Url;

/// The stable release manifest published with each GitHub Release.
pub const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/agent-ix/quoin/releases/latest/download/quoin-update-manifest.json";
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_ARCHIVE_BYTES: usize = 128 * 1024 * 1024;
const MAX_BINARY_BYTES: usize = 128 * 1024 * 1024;

/// A bounded byte transport, injected so updater tests have no network dependency.
pub trait Transport {
    /// Retrieve `url`, refusing a body larger than `limit`.
    ///
    /// # Errors
    ///
    /// Returns a typed boundary failure when the transport cannot produce a
    /// bounded response.
    fn get(&self, url: &Url, limit: usize) -> Result<Vec<u8>, DeliveryError>;
}

/// The production HTTPS transport.
pub struct HttpTransport;

impl Transport for HttpTransport {
    fn get(&self, url: &Url, limit: usize) -> Result<Vec<u8>, DeliveryError> {
        let response = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.url().scheme() == "https" {
                    attempt.follow()
                } else {
                    // The manifest endpoint may be an explicit local HTTP
                    // override, but a release download must never be
                    // redirected from HTTPS onto an unencrypted transport.
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|error| DeliveryError::Transport(error.to_string()))?
            .get(url.as_str())
            .send()
            .map_err(|error| DeliveryError::Transport(error.to_string()))?;
        if !response.status().is_success() {
            return Err(DeliveryError::HttpStatus(response.status().as_u16()));
        }
        let declared = response.content_length();
        if declared.is_some_and(|size| size > u64::try_from(limit).unwrap_or(u64::MAX)) {
            return Err(DeliveryError::BodyTooLarge);
        }
        let mut reader = response;
        read_bounded(&mut reader, limit)
    }
}

/// A result from an update check or installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateResult {
    /// The manifest does not name a newer stable version.
    Current {
        /// The latest stable version named by the manifest.
        version: Version,
    },
    /// A newer release exists but the caller requested a check only.
    Available {
        /// The newer stable version that was not installed.
        version: Version,
    },
    /// Unix replacement completed atomically.
    Replaced {
        /// The version now installed at the executable path.
        version: Version,
    },
    /// Windows retained the running executable and staged its replacement.
    Staged {
        /// The verified stable version awaiting a post-exit replacement.
        version: Version,
        /// The sibling path holding the verified replacement executable.
        path: PathBuf,
    },
}

/// Native delivery errors with stable, operator-visible categories.
#[derive(Debug, thiserror::Error)]
pub enum DeliveryError {
    /// The supplied manifest endpoint is not a usable URL.
    #[error("update manifest URL is invalid")]
    InvalidManifestUrl,
    /// A default or artifact URL does not use HTTPS.
    #[error("update URL must use HTTPS")]
    InsecureUrl,
    /// A download transport failed before a response was received.
    #[error("update transport failed: {0}")]
    Transport(String),
    /// A server returned a non-success response.
    #[error("update server returned HTTP {0}")]
    HttpStatus(u16),
    /// A fetched response exceeded its declared safety bound.
    #[error("update download exceeds its safety bound")]
    BodyTooLarge,
    /// The manifest is not a valid v1 delivery manifest.
    #[error("release manifest is invalid: {0}")]
    Manifest(String),
    /// No release record can safely serve this host.
    #[error("release manifest has no supported artifact for {0}")]
    UnsupportedTarget(String),
    /// The archive does not hash to the manifest digest.
    #[error("downloaded archive checksum does not match the manifest")]
    DigestMismatch,
    /// The archive is not the single safe executable promised by the convention.
    #[error("release archive is unsafe: {0}")]
    Archive(String),
    /// A local filesystem operation did not complete.
    #[error("cannot install update: {0}")]
    Io(#[from] std::io::Error),
}

impl DeliveryError {
    /// The Quoin process outcome for this boundary failure.
    #[must_use]
    pub const fn outcome(&self) -> quoin_outcome::OutcomeClass {
        match self {
            Self::InvalidManifestUrl | Self::Manifest(_) => quoin_outcome::OutcomeClass::Invalid,
            Self::InsecureUrl
            | Self::UnsupportedTarget(_)
            | Self::DigestMismatch
            | Self::Archive(_) => quoin_outcome::OutcomeClass::Refused,
            Self::Transport(_) | Self::HttpStatus(_) | Self::BodyTooLarge | Self::Io(_) => {
                quoin_outcome::OutcomeClass::Internal
            }
        }
    }
}

/// A minimal dependency-free representation of the CLI's exit taxonomy.
pub mod quoin_outcome {
    /// Failure categories the adapter maps to `quoin_core::protocol::Outcome`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OutcomeClass {
        /// Understood but unsafe input or state.
        Refused,
        /// Malformed caller input.
        Invalid,
        /// Transport or local I/O boundary failure.
        Internal,
    }
}

/// Check a release manifest and optionally install its host artifact.
///
/// `custom_manifest` is true only for an explicit CLI override; it preserves
/// local development endpoints while the default and release asset URLs remain
/// HTTPS-only.
///
/// # Errors
///
/// Returns a classified failure for malformed release data, unsafe archives,
/// unsupported hosts, transport failures, or local installation failures.
pub fn update(
    transport: &dyn Transport,
    manifest_url: &str,
    custom_manifest: bool,
    current: &Version,
    executable: &Path,
    check_only: bool,
) -> Result<UpdateResult, DeliveryError> {
    let manifest_url = parse_manifest_url(manifest_url, custom_manifest)?;
    let manifest_bytes = transport.get(&manifest_url, MAX_MANIFEST_BYTES)?;
    let manifest = ReleaseManifest::parse(&manifest_bytes)?;
    let target = host_target()?;
    let artifact = manifest.select(target)?;
    if artifact.version <= *current {
        return Ok(UpdateResult::Current {
            version: artifact.version,
        });
    }
    if check_only {
        return Ok(UpdateResult::Available {
            version: artifact.version,
        });
    }
    let archive_url = Url::parse(&artifact.url)
        .map_err(|_| DeliveryError::Manifest("artifact URL is invalid".to_owned()))?;
    if archive_url.scheme() != "https" {
        return Err(DeliveryError::InsecureUrl);
    }
    let archive = transport.get(&archive_url, MAX_ARCHIVE_BYTES)?;
    if archive.len() != artifact.size {
        return Err(DeliveryError::DigestMismatch);
    }
    if sha256_hex(&archive) != artifact.sha256 {
        return Err(DeliveryError::DigestMismatch);
    }
    let binary = extract(&archive, artifact.archive, executable_name())?;
    install(executable, &binary, &artifact.version)
}

fn parse_manifest_url(value: &str, custom: bool) -> Result<Url, DeliveryError> {
    let url = Url::parse(value).map_err(|_| DeliveryError::InvalidManifestUrl)?;
    if !matches!(url.scheme(), "https" | "http") || url.host_str().is_none() {
        return Err(DeliveryError::InvalidManifestUrl);
    }
    if !custom && url.scheme() != "https" {
        return Err(DeliveryError::InsecureUrl);
    }
    Ok(url)
}

fn host_target() -> Result<&'static str, DeliveryError> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        (operating_system, architecture) => Err(DeliveryError::UnsupportedTarget(format!(
            "{architecture}-{operating_system}"
        ))),
    }
}

fn executable_name() -> &'static str {
    if cfg!(windows) { "quoin.exe" } else { "quoin" }
}

fn install(
    executable: &Path,
    bytes: &[u8],
    version: &Version,
) -> Result<UpdateResult, DeliveryError> {
    let executable = executable.canonicalize()?;
    let parent = executable.parent().ok_or_else(|| {
        DeliveryError::Archive("running executable has no parent directory".to_owned())
    })?;
    let parent = parent.canonicalize()?;
    let file_name = executable
        .file_name()
        .ok_or_else(|| DeliveryError::Archive("running executable has no file name".to_owned()))?;
    let stage = unique_stage(&parent, file_name)?;
    write_new(&stage, bytes)?;
    fs::set_permissions(&stage, fs::metadata(&executable)?.permissions())?;
    #[cfg(windows)]
    {
        let replacement = parent.join(format!("{}.new", file_name.to_string_lossy()));
        if replacement.exists() {
            return Err(DeliveryError::Archive(
                "a staged replacement already exists".to_owned(),
            ));
        }
        fs::rename(&stage, &replacement)?;
        Ok(UpdateResult::Staged {
            version: version.clone(),
            path: replacement,
        })
    }
    #[cfg(not(windows))]
    {
        fs::rename(&stage, &executable)?;
        Ok(UpdateResult::Replaced {
            version: version.clone(),
        })
    }
}

fn unique_stage(parent: &Path, file_name: &OsStr) -> Result<PathBuf, DeliveryError> {
    let stage = parent.join(format!(
        ".{}.quoin-update-{}",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    if stage.exists() {
        return Err(DeliveryError::Archive(
            "an update staging file already exists".to_owned(),
        ));
    }
    Ok(stage)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), DeliveryError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn extract(
    archive: &[u8],
    format: ArchiveFormat,
    executable: &str,
) -> Result<Vec<u8>, DeliveryError> {
    match format {
        ArchiveFormat::TarGz => extract_tar(archive, executable),
        ArchiveFormat::Zip => extract_zip(archive, executable),
    }
}

fn extract_tar(archive: &[u8], executable: &str) -> Result<Vec<u8>, DeliveryError> {
    let decoder = GzDecoder::new(Cursor::new(archive));
    let mut archive = Archive::new(decoder);
    let mut entries = archive
        .entries()
        .map_err(|error| DeliveryError::Archive(error.to_string()))?;
    let Some(entry) = entries.next() else {
        return Err(DeliveryError::Archive("archive is empty".to_owned()));
    };
    let mut entry = entry.map_err(|error| DeliveryError::Archive(error.to_string()))?;
    validate_archive_path(
        &entry
            .path()
            .map_err(|error| DeliveryError::Archive(error.to_string()))?,
        executable,
    )?;
    if !entry.header().entry_type().is_file() {
        return Err(DeliveryError::Archive(
            "archive member is not a regular file".to_owned(),
        ));
    }
    let bytes = read_bounded(&mut entry, MAX_BINARY_BYTES)?;
    if entries.next().is_some() {
        return Err(DeliveryError::Archive(
            "archive contains more than one member".to_owned(),
        ));
    }
    Ok(bytes)
}

fn extract_zip(archive: &[u8], executable: &str) -> Result<Vec<u8>, DeliveryError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(archive))
        .map_err(|error| DeliveryError::Archive(error.to_string()))?;
    if archive.len() != 1 {
        return Err(DeliveryError::Archive(
            "archive must contain exactly one member".to_owned(),
        ));
    }
    let mut entry = archive
        .by_index(0)
        .map_err(|error| DeliveryError::Archive(error.to_string()))?;
    validate_archive_path(Path::new(entry.name()), executable)?;
    if !entry.is_file() || entry.is_symlink() {
        return Err(DeliveryError::Archive(
            "archive member is not a regular file".to_owned(),
        ));
    }
    read_bounded(&mut entry, MAX_BINARY_BYTES)
}

fn validate_archive_path(path: &Path, executable: &str) -> Result<(), DeliveryError> {
    if path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(name)) if name == OsStr::new(executable))
    {
        return Err(DeliveryError::Archive(
            "archive member path is not the expected executable".to_owned(),
        ));
    }
    Ok(())
}

fn read_bounded(reader: &mut impl Read, limit: usize) -> Result<Vec<u8>, DeliveryError> {
    let mut bytes = Vec::new();
    reader
        .take(u64::try_from(limit.saturating_add(1)).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(DeliveryError::BodyTooLarge);
    }
    Ok(bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseManifest {
    schema_version: u8,
    version: String,
    artifacts: Vec<ArtifactRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactRecord {
    target: String,
    asset: String,
    archive: ArchiveFormat,
    size: usize,
    sha256: String,
    url: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum ArchiveFormat {
    TarGz,
    Zip,
}

struct SelectedArtifact {
    version: Version,
    archive: ArchiveFormat,
    size: usize,
    sha256: String,
    url: String,
}

impl ReleaseManifest {
    fn parse(bytes: &[u8]) -> Result<Self, DeliveryError> {
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|error| DeliveryError::Manifest(error.to_string()))?;
        if manifest.schema_version != 1 {
            return Err(DeliveryError::Manifest(
                "unsupported schema version".to_owned(),
            ));
        }
        let version = Version::parse(&manifest.version)
            .map_err(|_| DeliveryError::Manifest("version is not SemVer".to_owned()))?;
        if !version.pre.is_empty() {
            return Err(DeliveryError::Manifest(
                "prerelease manifests are not update candidates".to_owned(),
            ));
        }
        if manifest.artifacts.is_empty() {
            return Err(DeliveryError::Manifest(
                "manifest has no artifacts".to_owned(),
            ));
        }
        for artifact in &manifest.artifacts {
            validate_record(artifact)?;
        }
        Ok(manifest)
    }

    fn select(&self, target: &str) -> Result<SelectedArtifact, DeliveryError> {
        let version = Version::parse(&self.version)
            .map_err(|_| DeliveryError::Manifest("version is not SemVer".to_owned()))?;
        let records = self
            .artifacts
            .iter()
            .filter(|record| record.target == target)
            .collect::<Vec<_>>();
        if records.len() != 1 {
            return Err(DeliveryError::UnsupportedTarget(target.to_owned()));
        }
        let record = records
            .first()
            .copied()
            .ok_or_else(|| DeliveryError::UnsupportedTarget(target.to_owned()))?;
        Ok(SelectedArtifact {
            version,
            archive: record.archive,
            size: record.size,
            sha256: record.sha256.clone(),
            url: record.url.clone(),
        })
    }
}

fn validate_record(record: &ArtifactRecord) -> Result<(), DeliveryError> {
    if record.target.is_empty()
        || record.asset.is_empty()
        || record.size == 0
        || record.size > MAX_ARCHIVE_BYTES
    {
        return Err(DeliveryError::Manifest(
            "artifact fields are invalid".to_owned(),
        ));
    }
    if record.asset.contains('/') || record.asset.contains('\\') || record.asset.contains("..") {
        return Err(DeliveryError::Manifest("asset name is unsafe".to_owned()));
    }
    if record.sha256.len() != 64
        || !record
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(DeliveryError::Manifest(
            "sha256 must be 64 lowercase hexadecimal characters".to_owned(),
        ));
    }
    let url = Url::parse(&record.url)
        .map_err(|_| DeliveryError::Manifest("artifact URL is invalid".to_owned()))?;
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err(DeliveryError::Manifest(
            "artifact URL must be HTTPS".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    struct FixtureTransport {
        bodies: BTreeMap<String, Vec<u8>>,
    }
    impl Transport for FixtureTransport {
        fn get(&self, url: &Url, limit: usize) -> Result<Vec<u8>, DeliveryError> {
            let body = self
                .bodies
                .get(url.as_str())
                .ok_or_else(|| DeliveryError::Transport("fixture URL absent".to_owned()))?
                .clone();
            if body.len() > limit {
                return Err(DeliveryError::BodyTooLarge);
            }
            Ok(body)
        }
    }

    fn manifest(archive: &[u8], url: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"schema_version": 1, "version": "9.9.9", "artifacts": [{"target": host_target().unwrap(), "asset": "quoin-v9.9.9-test.tar.gz", "archive": "tar-gz", "size": archive.len(), "sha256": sha256_hex(archive), "url": url}]})).unwrap()
    }

    fn tar(bytes: &[u8], path: &str) -> Vec<u8> {
        let buffer = Vec::new();
        let encoder = flate2::write::GzEncoder::new(buffer, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(u64::try_from(bytes.len()).unwrap());
        header.set_mode(0o755);
        header.set_cksum();
        archive.append_data(&mut header, path, bytes).unwrap();
        archive.into_inner().unwrap().finish().unwrap()
    }

    fn zip(bytes: &[u8], path: &str) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut archive = zip::ZipWriter::new(cursor);
        archive
            .start_file(path, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(bytes).unwrap();
        archive.finish().unwrap().into_inner()
    }

    /// Trace: FR-022
    #[test]
    fn tc_373_746_manifest_rejects_prereleases_and_duplicate_targets() {
        let prerelease = br#"{"schema_version":1,"version":"1.0.0-rc.1","artifacts":[]}"#;
        assert!(matches!(
            ReleaseManifest::parse(prerelease),
            Err(DeliveryError::Manifest(_))
        ));
        let duplicate = br#"{"schema_version":1,"version":"1.0.0","artifacts":[{"target":"x","asset":"a.tar.gz","archive":"tar-gz","size":1,"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","url":"https://example.test/a"},{"target":"x","asset":"b.tar.gz","archive":"tar-gz","size":1,"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","url":"https://example.test/b"}]}"#;
        let parsed = ReleaseManifest::parse(duplicate).expect("fixture parses");
        assert!(matches!(
            parsed.select("x"),
            Err(DeliveryError::UnsupportedTarget(_))
        ));
    }

    /// Trace: FR-022
    #[test]
    fn tc_373_747_check_only_never_fetches_or_mutates_the_archive() {
        let archive_url = "https://example.test/quoin.tar.gz";
        let archive = tar(b"new executable", executable_name());
        let manifest_url = "https://example.test/manifest.json";
        let mut bodies = BTreeMap::new();
        bodies.insert(manifest_url.to_owned(), manifest(&archive, archive_url));
        let scratch = tempfile::tempdir().unwrap();
        let executable = scratch.path().join(executable_name());
        fs::write(&executable, b"old executable").unwrap();
        let result = update(
            &FixtureTransport { bodies },
            manifest_url,
            false,
            &Version::new(1, 0, 0),
            &executable,
            true,
        )
        .unwrap();
        assert_eq!(
            result,
            UpdateResult::Available {
                version: Version::new(9, 9, 9)
            }
        );
        assert_eq!(fs::read(executable).unwrap(), b"old executable");
    }

    /// Trace: FR-022
    #[test]
    fn tc_373_748_checksum_mismatch_leaves_the_installed_binary_unchanged() {
        let archive_url = "https://example.test/quoin.tar.gz";
        let archive = tar(b"new executable", executable_name());
        let manifest_url = "https://example.test/manifest.json";
        let digest = sha256_hex(&archive);
        let document = String::from_utf8(manifest(&archive, archive_url))
            .unwrap()
            .replacen(&digest, &"b".repeat(64), 1);
        let mut bodies = BTreeMap::new();
        bodies.insert(manifest_url.to_owned(), document.into_bytes());
        bodies.insert(archive_url.to_owned(), archive);
        let scratch = tempfile::tempdir().unwrap();
        let executable = scratch.path().join(executable_name());
        fs::write(&executable, b"old executable").unwrap();
        let result = update(
            &FixtureTransport { bodies },
            manifest_url,
            false,
            &Version::new(1, 0, 0),
            &executable,
            false,
        );
        assert!(matches!(result, Err(DeliveryError::DigestMismatch)));
        assert_eq!(fs::read(executable).unwrap(), b"old executable");
    }

    /// Trace: FR-022
    #[test]
    fn tc_373_749_archive_paths_and_traversal_are_refused_before_installation() {
        let archive = tar(b"malicious", "nested/quoin");
        assert!(matches!(
            extract(&archive, ArchiveFormat::TarGz, executable_name()),
            Err(DeliveryError::Archive(_))
        ));
        assert!(matches!(
            validate_archive_path(Path::new("../quoin"), executable_name()),
            Err(DeliveryError::Archive(_))
        ));
        let zip = zip(b"malicious", "nested/quoin");
        assert!(matches!(
            extract(&zip, ArchiveFormat::Zip, executable_name()),
            Err(DeliveryError::Archive(_))
        ));
    }

    /// Trace: FR-022
    #[test]
    fn tc_373_750_verified_archive_replaces_the_real_staged_binary() {
        let archive_url = "https://example.test/quoin.tar.gz";
        let archive = tar(b"new executable", executable_name());
        let manifest_url = "https://example.test/manifest.json";
        let mut bodies = BTreeMap::new();
        bodies.insert(manifest_url.to_owned(), manifest(&archive, archive_url));
        bodies.insert(archive_url.to_owned(), archive);
        let scratch = tempfile::tempdir().unwrap();
        let executable = scratch.path().join(executable_name());
        fs::write(&executable, b"old executable").unwrap();
        let result = update(
            &FixtureTransport { bodies },
            manifest_url,
            false,
            &Version::new(1, 0, 0),
            &executable,
            false,
        )
        .unwrap();
        #[cfg(not(windows))]
        assert_eq!(
            result,
            UpdateResult::Replaced {
                version: Version::new(9, 9, 9)
            }
        );
        #[cfg(windows)]
        assert!(matches!(result, UpdateResult::Staged { .. }));
        #[cfg(not(windows))]
        assert_eq!(fs::read(executable).unwrap(), b"new executable");
    }
}
