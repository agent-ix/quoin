// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The record index of the SOUP joins (FR-142): which `soup_record` a finding's
//! path selects, by the one purl equality rule.

use std::collections::BTreeMap;

use quoin_evidence::purl::PurlIdentity;

use super::UnmatchedReason;
use crate::soup::plan::Existing;

/// A position in the `records` slice.
#[derive(Clone, Copy)]
pub(in crate::soup) struct RecordRef(pub(in crate::soup) usize);

/// Where one advisory finding went.
pub(in crate::soup) enum Hit {
    /// To exactly one record.
    One(RecordRef),
    /// To none; the candidates are the records the finding could belong to.
    Not(UnmatchedReason, Vec<usize>),
}

/// Records by the purl identity a finding's path names (FR-142: type compared
/// case-insensitively, namespace, name and version percent-decoded, qualifiers
/// and subpath ignored). One rule, `PurlIdentity`, for every join.
pub(in crate::soup) struct RecordIndex {
    by_identity: BTreeMap<PurlIdentity, Vec<usize>>,
    /// Every parseable record purl, with or without a version.
    identities: Vec<(usize, PurlIdentity)>,
    count: usize,
}

impl RecordIndex {
    pub(in crate::soup) fn new(records: &[Existing]) -> Self {
        let identities: Vec<(usize, PurlIdentity)> = records
            .iter()
            .enumerate()
            .filter_map(|(position, record)| Some((position, PurlIdentity::parse(&record.purl)?)))
            .collect();
        let mut by_identity: BTreeMap<PurlIdentity, Vec<usize>> = BTreeMap::new();
        for (position, identity) in &identities {
            by_identity
                .entry(identity.clone())
                .or_default()
                .push(*position);
        }
        Self {
            by_identity,
            identities,
            count: records.len(),
        }
    }

    /// The records of the same package, whatever their version.
    fn same_package(&self, identity: &PurlIdentity) -> Vec<usize> {
        self.identities
            .iter()
            .filter(|(_, other)| other.same_package(identity))
            .map(|(position, _)| *position)
            .collect()
    }

    fn lookup(&self, identity: &PurlIdentity) -> Hit {
        match self.by_identity.get(identity).map(Vec::as_slice) {
            None | Some([]) => Hit::Not(UnmatchedReason::NoRecord, Vec::new()),
            Some([only]) => Hit::One(RecordRef(*only)),
            Some(many) => Hit::Not(UnmatchedReason::Ambiguous, many.to_vec()),
        }
    }

    /// The record a `cargo-audit` or `cargo-deny` finding path selects:
    /// `<name>@<version>` as the cargo crate it names, or a purl. A finding
    /// that names no version, or no path, is unmatched and could belong to
    /// every record of its crate, or to every record.
    pub(in crate::soup) fn find(&self, path: Option<&str>) -> Hit {
        let Some(path) = path else {
            return Hit::Not(UnmatchedReason::NoVersion, (0..self.count).collect());
        };
        let identity = if path.starts_with("pkg:") {
            PurlIdentity::parse(path)
        } else {
            let (name, version) = path.rsplit_once('@').unwrap_or((path, ""));
            PurlIdentity::parse(&format!("pkg:cargo/{name}@{version}"))
        };
        match identity {
            Some(identity) if identity.version.is_some() => self.lookup(&identity),
            Some(identity) => Hit::Not(UnmatchedReason::NoVersion, self.same_package(&identity)),
            None => Hit::Not(UnmatchedReason::NoVersion, Vec::new()),
        }
    }

    /// The record a finding whose path is a component purl selects (the
    /// `licence-policy` tool). A path that is not a purl names no record and
    /// could belong to any.
    pub(in crate::soup) fn find_purl(&self, path: Option<&str>) -> Hit {
        match path.and_then(PurlIdentity::parse) {
            None => Hit::Not(UnmatchedReason::NoPurl, (0..self.count).collect()),
            Some(identity) if identity.version.is_some() => self.lookup(&identity),
            Some(identity) => Hit::Not(UnmatchedReason::NoVersion, self.same_package(&identity)),
        }
    }

    /// The record an OSV finding's purl selects. Its name/version fallback is
    /// intentionally unmatched because it does not identify an ecosystem.
    pub(in crate::soup) fn find_osv(&self, path: Option<&str>) -> Hit {
        let applicable: Vec<usize> = self
            .identities
            .iter()
            .filter(|(_, identity)| assessed_by_osv(&identity.kind))
            .map(|(position, _)| *position)
            .collect();
        let Some(path) = path.filter(|path| path.starts_with("pkg:")) else {
            return Hit::Not(UnmatchedReason::NoPurl, applicable);
        };
        let Some(identity) = PurlIdentity::parse(path) else {
            return Hit::Not(UnmatchedReason::NoPurl, applicable);
        };
        if !assessed_by_osv(&identity.kind) {
            return Hit::Not(UnmatchedReason::NoRecord, Vec::new());
        }
        match identity.version {
            Some(_) => self.lookup(&identity),
            None => Hit::Not(UnmatchedReason::NoVersion, self.same_package(&identity)),
        }
    }
}

pub(in crate::soup) fn purl_type(purl: &str) -> Option<&str> {
    let kind = purl.strip_prefix("pkg:")?.split_once('/')?.0;
    Some(if kind.eq_ignore_ascii_case("cargo") {
        "cargo"
    } else if kind.eq_ignore_ascii_case("npm") {
        "npm"
    } else if kind.eq_ignore_ascii_case("pypi") {
        "pypi"
    } else if kind.eq_ignore_ascii_case("golang") {
        "golang"
    } else if kind.eq_ignore_ascii_case("deb") {
        "deb"
    } else {
        return None;
    })
}

fn assessed_by_osv(kind: &str) -> bool {
    matches!(kind, "npm" | "pypi" | "golang" | "deb")
}
