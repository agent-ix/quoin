// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Public npm registry transport and bounded package extraction (FR-187).

use std::fs;
use std::io::Read;
#[cfg(test)]
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use serde_json::Value;
use tempfile::Builder;
use url::Url;

mod archive;

use crate::error::ModulesError;
use crate::source::Source;
use archive::extract_package;
#[cfg(test)]
use archive::package_relative_path;

const REGISTRY: &str = "https://registry.npmjs.org/";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_METADATA_BYTES: u64 = 16 << 20;
const MAX_TARBALL_BYTES: u64 = 64 << 20;
const MAX_EXTRACTED_BYTES: u64 = 512 << 20;
const MAX_ARCHIVE_ENTRIES: u64 = 10_000;

/// A response from the public npm registry transport.
pub(crate) struct NpmResponse {
    pub(crate) final_url: Url,
    pub(crate) body: Vec<u8>,
}

/// Bounded GET seam for registry and tarball responses.
pub(crate) trait NpmTransport {
    /// Fetch one URL, returning at most `limit` bytes and its final URL.
    fn get(&self, url: &Url, limit: u64) -> Result<NpmResponse, ModulesError>;
}

/// The production HTTPS transport. Redirects stay on the public registry origin.
pub(crate) struct HttpNpmTransport;

impl NpmTransport for HttpNpmTransport {
    fn get(&self, url: &Url, limit: u64) -> Result<NpmResponse, ModulesError> {
        if !is_public_registry_url(url) {
            return Err(ModulesError::NpmRegistryUnsupported);
        }
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::custom(|attempt| {
                if is_public_registry_url(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|error| ModulesError::NpmTransport {
                detail: error.to_string(),
            })?;
        let response = client
            .get(url.clone())
            .send()
            .map_err(|error| map_transport_error(&error))?;
        if !response.status().is_success() {
            return Err(ModulesError::NpmTransport {
                detail: format!("registry returned HTTP {}", response.status().as_u16()),
            });
        }
        let final_url = response.url().clone();
        if !is_public_registry_url(&final_url) {
            return Err(ModulesError::NpmRegistryUnsupported);
        }
        let body = read_bounded_response(response, limit)?;
        Ok(NpmResponse { final_url, body })
    }
}

/// Resolve an npm source into an extracted package directory.
pub(crate) fn resolve_package(
    cache_root: &Path,
    source: &Source,
    transport: &dyn NpmTransport,
) -> Result<PathBuf, ModulesError> {
    let Source::Npm {
        package,
        version,
        registry,
    } = source
    else {
        return Err(ModulesError::UnsupportedSource {
            source_type: source.type_name(),
        });
    };

    if let Some(registry) = registry {
        let configured = Url::parse(registry).map_err(|_| ModulesError::NpmRegistryUnsupported)?;
        if configured.as_str() != REGISTRY {
            return Err(ModulesError::NpmRegistryUnsupported);
        }
    }
    if version.as_ref().is_some_and(String::is_empty) {
        return Err(ModulesError::InvalidSourceField {
            field: "version",
            rule: crate::error::SourceFieldRule::NonEmpty,
        });
    }

    let metadata_url = metadata_url(package, version.as_deref())?;
    let metadata_response = transport.get(&metadata_url, MAX_METADATA_BYTES)?;
    validate_response_url(&metadata_response.final_url)?;
    if u64::try_from(metadata_response.body.len()).unwrap_or(u64::MAX) > MAX_METADATA_BYTES {
        return Err(resource_bound(
            "npm metadata bytes",
            metadata_response.body.len(),
            MAX_METADATA_BYTES,
        ));
    }
    let metadata: Value = serde_json::from_slice(&metadata_response.body).map_err(|error| {
        ModulesError::NpmMetadata {
            detail: error.to_string(),
        }
    })?;
    let tarball = tarball_url(&metadata, version.as_deref())?;

    let archive_response = transport.get(&tarball, MAX_TARBALL_BYTES)?;
    validate_response_url(&archive_response.final_url)?;
    if u64::try_from(archive_response.body.len()).unwrap_or(u64::MAX) > MAX_TARBALL_BYTES {
        return Err(resource_bound(
            "npm compressed tarball bytes",
            archive_response.body.len(),
            MAX_TARBALL_BYTES,
        ));
    }

    let npm_cache = cache_root.join("npm");
    fs::create_dir_all(&npm_cache).map_err(|source| ModulesError::MaterializeFailed {
        target: npm_cache.clone(),
        source,
    })?;
    let extracted = Builder::new()
        .prefix("package-")
        .tempdir_in(&npm_cache)
        .map_err(|source| ModulesError::MaterializeFailed {
            target: npm_cache.clone(),
            source,
        })?;
    extract_package(&archive_response.body, extracted.path())?;
    Ok(extracted.keep())
}

fn metadata_url(package: &str, version: Option<&str>) -> Result<Url, ModulesError> {
    let mut url = Url::parse(REGISTRY).map_err(|error| ModulesError::NpmMetadata {
        detail: error.to_string(),
    })?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|()| ModulesError::NpmMetadata {
                detail: "registry URL cannot accept package path segments".to_owned(),
            })?;
        segments.pop_if_empty().push(package);
        if let Some(version) = version {
            segments.push(version);
        }
    }
    Ok(url)
}

fn tarball_url(metadata: &Value, requested_version: Option<&str>) -> Result<Url, ModulesError> {
    let selected = if let Some(version) = requested_version {
        metadata
            .get("version")
            .and_then(Value::as_str)
            .filter(|actual| *actual == version)
            .ok_or_else(|| ModulesError::NpmMetadata {
                detail: format!("registry did not return requested version {version:?}"),
            })?;
        metadata
    } else {
        let version = metadata
            .get("dist-tags")
            .and_then(|tags| tags.get("latest"))
            .and_then(Value::as_str)
            .ok_or_else(|| ModulesError::NpmMetadata {
                detail: "registry metadata has no latest dist-tag".to_owned(),
            })?;
        metadata
            .get("versions")
            .and_then(|versions| versions.get(version))
            .ok_or_else(|| ModulesError::NpmMetadata {
                detail: format!("latest release {version:?} is absent from versions"),
            })?
    };
    let raw_url = selected
        .get("dist")
        .and_then(|dist| dist.get("tarball"))
        .and_then(Value::as_str)
        .ok_or_else(|| ModulesError::NpmMetadata {
            detail: "selected release has no dist.tarball URL".to_owned(),
        })?;
    let url = Url::parse(raw_url).map_err(|error| ModulesError::NpmMetadata {
        detail: format!("dist.tarball URL is invalid: {error}"),
    })?;
    if !is_public_registry_url(&url) {
        return Err(ModulesError::NpmRegistryUnsupported);
    }
    Ok(url)
}

fn validate_response_url(url: &Url) -> Result<(), ModulesError> {
    if is_public_registry_url(url) {
        Ok(())
    } else {
        Err(ModulesError::NpmRegistryUnsupported)
    }
}

fn is_public_registry_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("registry.npmjs.org")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}

fn read_bounded_response(response: Response, limit: u64) -> Result<Vec<u8>, ModulesError> {
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        let observed =
            usize::try_from(response.content_length().unwrap_or(u64::MAX)).unwrap_or(usize::MAX);
        return Err(resource_bound("npm response bytes", observed, limit));
    }
    let read_limit = limit.saturating_add(1);
    let mut body = Vec::new();
    response
        .take(read_limit)
        .read_to_end(&mut body)
        .map_err(|error| map_body_read_error(&error))?;
    if u64::try_from(body.len()).unwrap_or(u64::MAX) > limit {
        return Err(resource_bound("npm response bytes", body.len(), limit));
    }
    Ok(body)
}

fn map_transport_error(error: &reqwest::Error) -> ModulesError {
    if error.is_timeout() {
        ModulesError::NpmTimeout
    } else {
        ModulesError::NpmTransport {
            detail: error.to_string(),
        }
    }
}

fn map_body_read_error(error: &std::io::Error) -> ModulesError {
    if error.kind() == std::io::ErrorKind::TimedOut {
        ModulesError::NpmTimeout
    } else {
        ModulesError::NpmTransport {
            detail: error.to_string(),
        }
    }
}

pub(super) fn resource_bound(what: &'static str, observed: usize, limit: u64) -> ModulesError {
    ModulesError::ResourceBoundExceeded {
        what,
        observed: u64::try_from(observed).unwrap_or(u64::MAX),
        limit,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]

    use std::collections::VecDeque;
    use std::io::Cursor;
    use std::sync::Mutex;

    use flate2::Compression;
    use flate2::write::GzEncoder;
    use ix_trace_rs::trace;

    use super::*;

    struct FixtureTransport {
        responses: Mutex<VecDeque<NpmResponse>>,
        requests: Mutex<Vec<Url>>,
    }

    impl FixtureTransport {
        fn new(responses: Vec<NpmResponse>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn requests(&self) -> Vec<Url> {
            self.requests.lock().expect("requests lock").clone()
        }
    }

    impl NpmTransport for FixtureTransport {
        fn get(&self, url: &Url, _limit: u64) -> Result<NpmResponse, ModulesError> {
            self.requests
                .lock()
                .expect("requests lock")
                .push(url.clone());
            self.responses
                .lock()
                .expect("responses lock")
                .pop_front()
                .ok_or_else(|| ModulesError::NpmTransport {
                    detail: "unexpected extra request".to_owned(),
                })
        }
    }

    fn response(url: &str, body: Vec<u8>) -> NpmResponse {
        NpmResponse {
            final_url: Url::parse(url).expect("response URL"),
            body,
        }
    }

    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tarball = tar::Builder::new(encoder);
        for (path, contents) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(u64::try_from(contents.len()).expect("test file size"));
            header.set_mode(0o644);
            header.set_cksum();
            tarball
                .append_data(&mut header, path, Cursor::new(*contents))
                .expect("append archive file");
        }
        tarball
            .into_inner()
            .expect("finish tar")
            .finish()
            .expect("finish gzip")
    }

    fn raw_tarball(path: &str, size: u64, contents: &[u8]) -> Vec<u8> {
        let mut header = tar::Header::new_gnu();
        let path_bytes = path.as_bytes();
        assert!(path_bytes.len() < 100, "fixture path fits USTAR name field");
        header.as_mut_bytes()[..path_bytes.len()].copy_from_slice(path_bytes);
        header.set_size(size);
        header.set_mode(0o644);
        header.set_cksum();
        let mut raw = Vec::new();
        raw.extend_from_slice(header.as_bytes());
        raw.extend_from_slice(contents);
        let padding = (512 - (contents.len() % 512)) % 512;
        raw.resize(raw.len() + padding, 0);
        raw.resize(raw.len() + 1024, 0);
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw).expect("write raw tar bytes");
        encoder.finish().expect("finish gzip")
    }

    fn version_metadata(version: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "version": version,
            "dist": { "tarball": format!("https://registry.npmjs.org/demo/-/demo-{version}.tgz") }
        }))
        .expect("metadata JSON")
    }

    #[trace("FR-187-AC-1")]
    #[test]
    fn exact_version_uses_requested_endpoint_and_installs_package_tree() {
        let transport = FixtureTransport::new(vec![
            response(
                "https://registry.npmjs.org/demo/1.2.3",
                version_metadata("1.2.3"),
            ),
            response(
                "https://registry.npmjs.org/demo/-/demo-1.2.3.tgz",
                archive(&[("package/manifest.yaml", b"id: demo\n")]),
            ),
        ]);
        let temp = tempfile::tempdir().expect("temp cache");
        let extracted = resolve_package(
            temp.path(),
            &Source::Npm {
                package: "demo".to_owned(),
                version: Some("1.2.3".to_owned()),
                registry: None,
            },
            &transport,
        )
        .expect("package resolved");
        assert_eq!(
            transport.requests()[0].as_str(),
            "https://registry.npmjs.org/demo/1.2.3"
        );
        assert_eq!(
            fs::read(extracted.join("manifest.yaml")).expect("extracted manifest"),
            b"id: demo\n"
        );
    }

    #[trace("FR-187-AC-2")]
    #[test]
    fn unversioned_source_selects_latest_dist_tag() {
        let latest_tarball = "https://registry.npmjs.org/demo/-/demo-2.0.0.tgz";
        let transport = FixtureTransport::new(vec![
            response(
                "https://registry.npmjs.org/demo",
                serde_json::to_vec(&serde_json::json!({
                    "dist-tags": { "latest": "2.0.0" },
                    "versions": { "2.0.0": { "dist": { "tarball": latest_tarball } } }
                }))
                .expect("packument"),
            ),
            response(
                latest_tarball,
                archive(&[("package/manifest.yaml", b"id: demo\n")]),
            ),
        ]);
        let temp = tempfile::tempdir().expect("temp cache");
        resolve_package(
            temp.path(),
            &Source::Npm {
                package: "demo".to_owned(),
                version: None,
                registry: None,
            },
            &transport,
        )
        .expect("latest package resolved");
        assert_eq!(transport.requests()[1].as_str(), latest_tarball);
    }

    #[trace("FR-187-AC-3")]
    #[test]
    fn scoped_name_is_encoded_as_one_registry_segment() {
        let url = metadata_url("@agent-ix/spec-objects-security", Some("0.13.0"))
            .expect("registry metadata URL");
        assert_eq!(
            url.as_str(),
            "https://registry.npmjs.org/@agent-ix%2Fspec-objects-security/0.13.0"
        );
    }

    #[trace("FR-187-AC-4")]
    #[test]
    fn registry_override_outside_public_origin_is_refused_before_request() {
        let temp = tempfile::tempdir().expect("temp cache");
        for override_url in [
            "http://registry.npmjs.org/",
            "https://example.org/",
            "https://registry.npmjs.org/custom/",
        ] {
            let transport = FixtureTransport::new(Vec::new());
            let error = resolve_package(
                temp.path(),
                &Source::Npm {
                    package: "demo".to_owned(),
                    version: None,
                    registry: Some(override_url.to_owned()),
                },
                &transport,
            )
            .expect_err("override must be refused");
            assert!(matches!(error, ModulesError::NpmRegistryUnsupported));
            assert!(transport.requests().is_empty());
        }
    }

    #[trace("FR-187-AC-5")]
    #[test]
    fn redirected_response_outside_public_origin_is_refused() {
        let transport = FixtureTransport::new(vec![response(
            "http://example.org/demo",
            version_metadata("1.0.0"),
        )]);
        let temp = tempfile::tempdir().expect("temp cache");
        let error = resolve_package(
            temp.path(),
            &Source::Npm {
                package: "demo".to_owned(),
                version: Some("1.0.0".to_owned()),
                registry: None,
            },
            &transport,
        )
        .expect_err("redirect must be refused");
        assert!(matches!(error, ModulesError::NpmRegistryUnsupported));
    }

    #[trace("FR-187-AC-6")]
    #[test]
    fn metadata_body_over_limit_is_rejected() {
        let transport = FixtureTransport::new(vec![response(
            "https://registry.npmjs.org/demo/1.0.0",
            vec![b'x'; usize::try_from(MAX_METADATA_BYTES + 1).expect("test bound")],
        )]);
        let temp = tempfile::tempdir().expect("temp cache");
        let error = resolve_package(
            temp.path(),
            &Source::Npm {
                package: "demo".to_owned(),
                version: Some("1.0.0".to_owned()),
                registry: None,
            },
            &transport,
        )
        .expect_err("oversized metadata must be refused");
        assert!(matches!(error, ModulesError::ResourceBoundExceeded { .. }));
    }

    #[trace("FR-187-AC-7")]
    #[test]
    fn compressed_tarball_over_limit_is_rejected() {
        let tarball = "https://registry.npmjs.org/demo/-/demo-1.0.0.tgz";
        let transport = FixtureTransport::new(vec![
            response(
                "https://registry.npmjs.org/demo/1.0.0",
                version_metadata("1.0.0"),
            ),
            response(
                tarball,
                vec![b'x'; usize::try_from(MAX_TARBALL_BYTES + 1).expect("test bound")],
            ),
        ]);
        let temp = tempfile::tempdir().expect("temp cache");
        let error = resolve_package(
            temp.path(),
            &Source::Npm {
                package: "demo".to_owned(),
                version: Some("1.0.0".to_owned()),
                registry: None,
            },
            &transport,
        )
        .expect_err("oversized tarball must be refused");
        assert!(matches!(error, ModulesError::ResourceBoundExceeded { .. }));
    }

    #[trace("FR-187-AC-8")]
    #[test]
    fn declared_extracted_bytes_over_limit_are_rejected_before_copy() {
        let temp = tempfile::tempdir().expect("temp extraction");
        let path = Path::new("oversized.bin");
        let bytes = raw_tarball("package/oversized.bin", MAX_EXTRACTED_BYTES + 1, &[]);
        let error = extract_package(&bytes, temp.path()).expect_err("size bound");
        assert!(
            matches!(error, ModulesError::ResourceBoundExceeded { .. }),
            "{error:?}"
        );
        assert!(!temp.path().join(path).exists());
    }

    #[trace("FR-187-AC-9")]
    #[test]
    fn archive_entry_count_over_limit_is_rejected() {
        let encoder = GzEncoder::new(Vec::new(), Compression::fast());
        let mut tarball = tar::Builder::new(encoder);
        for index in 0..=MAX_ARCHIVE_ENTRIES {
            let mut header = tar::Header::new_gnu();
            header.set_size(0);
            header.set_mode(0o644);
            header.set_cksum();
            tarball
                .append_data(
                    &mut header,
                    format!("package/file-{index}"),
                    Cursor::new([]),
                )
                .expect("append empty file");
        }
        let bytes = tarball
            .into_inner()
            .expect("finish tar")
            .finish()
            .expect("finish gzip");
        let temp = tempfile::tempdir().expect("temp extraction");
        let error = extract_package(&bytes, temp.path()).expect_err("entry count bound");
        assert!(matches!(error, ModulesError::ResourceBoundExceeded { .. }));
    }

    #[trace("FR-187-AC-10")]
    #[test]
    fn archive_path_that_escapes_package_root_is_rejected() {
        let temp = tempfile::tempdir().expect("temp extraction");
        let bytes = raw_tarball("package/../../outside", 6, b"unsafe");
        assert!(extract_package(&bytes, temp.path()).is_err());
        assert!(
            !temp
                .path()
                .parent()
                .expect("parent")
                .join("outside")
                .exists()
        );
    }

    #[trace("FR-187-AC-11")]
    #[test]
    fn package_script_is_only_extracted_and_never_executed() {
        let temp = tempfile::tempdir().expect("temp extraction");
        let bytes = archive(&[(
            "package/package.json",
            br#"{"scripts":{"install":"touch marker"}}"#,
        )]);
        extract_package(&bytes, temp.path()).expect("archive extraction");
        assert!(!temp.path().join("marker").exists());
    }

    #[trace("FR-187-AC-13")]
    #[test]
    fn timed_out_transport_error_keeps_timeout_code() {
        let error = map_body_read_error(&std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "fixture timeout",
        ));
        assert!(matches!(error, ModulesError::NpmTimeout));
    }

    #[trace("FR-187-AC-3")]
    #[test]
    fn metadata_urls_encode_scoped_package_as_one_segment() {
        let url = metadata_url("@agent-ix/spec-objects-security", Some("0.13.0"))
            .expect("registry metadata URL");
        assert_eq!(
            url.as_str(),
            "https://registry.npmjs.org/@agent-ix%2Fspec-objects-security/0.13.0"
        );
    }

    #[trace("FR-187-AC-5")]
    #[test]
    fn response_origin_accepts_only_public_https_registry() {
        assert!(is_public_registry_url(
            &Url::parse("https://registry.npmjs.org/a").expect("URL")
        ));
        for invalid in [
            "http://registry.npmjs.org/a",
            "https://example.org/a",
            "https://registry.npmjs.org:444/a",
            "https://user@registry.npmjs.org/a",
        ] {
            assert!(
                !is_public_registry_url(&Url::parse(invalid).expect("URL")),
                "unexpectedly accepted {invalid}"
            );
        }
    }

    #[trace("FR-187-AC-10")]
    #[test]
    fn package_path_rejects_traversal_and_symlink_target_separators() {
        for invalid in [
            "../outside",
            "package/../outside",
            "package/dir/../../outside",
            "package\\outside",
            "/package/outside",
            "other/manifest.yaml",
        ] {
            assert!(
                package_relative_path(Path::new(invalid)).is_err(),
                "{invalid}"
            );
        }
        assert_eq!(
            package_relative_path(Path::new("package/manifest.yaml")).expect("safe path"),
            PathBuf::from("manifest.yaml")
        );
    }
}
