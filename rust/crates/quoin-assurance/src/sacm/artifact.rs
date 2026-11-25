// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! SACM evidence artifacts from the exact run records selected by GSN.

use std::collections::{BTreeMap, btree_map::Entry};

use quoin_evidence::ids::SuiteId;
use quoin_evidence::paths::{RUNS_DIR, record_path};
use quoin_evidence::source::EvidenceSource;
use quoin_store::attachment::sha256_hex;

use crate::case::LeafEvidence;
use crate::gsn::{GsnCase, GsnNode};
use crate::matrix::EvidenceStatus;

use super::ExportError;
use super::xml::Xml;

pub(super) struct Artifacts<'a> {
    records: Vec<Artifact>,
    solutions: BTreeMap<&'a str, Solution>,
}

struct Artifact {
    id: String,
    identity: String,
    selections: Vec<Selection>,
}

struct Selection {
    suite: String,
    at: String,
    verdict: Option<EvidenceStatus>,
    path: String,
}

pub(super) struct Solution {
    pub(super) artifact_ids: String,
    non_supporting: Option<EvidenceStatus>,
}

impl Solution {
    pub(super) fn non_supporting(&self) -> Option<EvidenceStatus> {
        self.non_supporting
    }
}

impl<'a> Artifacts<'a> {
    pub(super) fn collect(
        model: &'a GsnCase,
        source: &dyn EvidenceSource,
    ) -> Result<Self, ExportError> {
        let mut records = Vec::new();
        let mut solutions = BTreeMap::new();
        let mut by_identity = BTreeMap::new();
        let mut by_path = BTreeMap::new();
        for node in model.nodes.iter().filter(|node| node.kind == "solution") {
            let evidence = leaf_evidence(node)?;
            let verdict = evidence.as_ref().and_then(|evidence| evidence.verdict);
            let mut ids = Vec::new();
            let mut missing = true;
            if let Some(evidence) = evidence {
                missing = evidence.suites.is_empty();
                for suite in evidence.suites {
                    let Some(at) = suite.at else {
                        missing = true;
                        continue;
                    };
                    let path = selected_path(&suite.suite, &at)?;
                    let digest = match by_path.entry(path.clone()) {
                        Entry::Occupied(entry) => entry.into_mut(),
                        Entry::Vacant(entry) => entry.insert(
                            source
                                .read(&path)?
                                .map(|contents| sha256_hex(contents.as_bytes())),
                        ),
                    };
                    let Some(digest) = digest else {
                        missing = true;
                        continue;
                    };
                    let selection = Selection {
                        suite: suite.suite,
                        at,
                        verdict,
                        path: format!("spec/evidence/{path}"),
                    };
                    let id = remember(&mut by_identity, &mut records, digest, selection)?;
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
            let non_supporting = match (missing, verdict, node.status.as_str()) {
                (true, _, _) => Some(EvidenceStatus::Undischarged),
                (false, Some(EvidenceStatus::Bound), "supported") => None,
                (false, status, _) => Some(
                    status
                        .filter(|status| *status != EvidenceStatus::Bound)
                        .unwrap_or(EvidenceStatus::Undischarged),
                ),
            };
            solutions.insert(
                node.id.as_str(),
                Solution {
                    artifact_ids: ids.join(" "),
                    non_supporting,
                },
            );
        }
        Ok(Self { records, solutions })
    }

    pub(super) fn solution(&self, id: &str) -> Option<&Solution> {
        self.solutions.get(id)
    }

    pub(super) fn write(&self, xml: &mut Xml) -> Result<(), ExportError> {
        for artifact in &self.records {
            artifact.write(xml)?;
        }
        Ok(())
    }
}

fn leaf_evidence(node: &GsnNode) -> Result<Option<LeafEvidence>, ExportError> {
    // Authored solutions carry evidence references. Only a suite selection
    // identifies an exact retained run record.
    node.evidence
        .as_ref()
        .filter(|evidence| evidence.get("suites").is_some())
        .map(|evidence| {
            serde_json::from_value(evidence.clone()).map_err(|source| {
                ExportError::InvalidEvidence {
                    node: node.id.clone(),
                    source,
                }
            })
        })
        .transpose()
}

fn selected_path(suite: &str, at: &str) -> Result<String, ExportError> {
    if suite.is_empty() || matches!(suite, "." | "..") || suite.contains(['/', '\\', ':', '\0']) {
        return Err(ExportError::InvalidSuite {
            suite: suite.to_owned(),
        });
    }
    Ok(record_path(RUNS_DIR, &SuiteId::new(suite), at))
}

fn remember(
    by_identity: &mut BTreeMap<String, usize>,
    records: &mut Vec<Artifact>,
    digest: &str,
    selection: Selection,
) -> Result<String, ExportError> {
    let id = format!("artifact-{digest}");
    match by_identity.entry(digest.to_owned()) {
        Entry::Vacant(entry) => {
            entry.insert(records.len());
            records.push(Artifact {
                id: id.clone(),
                identity: format!("sha256:{digest}"),
                selections: vec![selection],
            });
        }
        Entry::Occupied(entry) => {
            let artifact = records
                .get_mut(*entry.get())
                .ok_or(ExportError::MissingArtifact)?;
            if !artifact
                .selections
                .iter()
                .any(|existing| existing.path == selection.path)
            {
                artifact.selections.push(selection);
            }
        }
    }
    Ok(id)
}

impl Artifact {
    fn write(&self, xml: &mut Xml) -> Result<(), ExportError> {
        let first = self
            .selections
            .first()
            .ok_or(ExportError::MissingArtifact)?;
        xml.start(
            "artifactElement",
            &[
                ("xmi:type", "sacm:Artifact"),
                ("xmi:id", &self.id),
                ("gid", &self.identity),
                ("version", &first.at),
            ],
        )?;
        xml.name(&first.suite)?;
        xml.description(&format!("verdict: {}", verdict_name(first.verdict)))?;
        xml.end("artifactElement")?;

        // A group associates the run artifact with its location Resources.
        // Resource is a peer of Artifact in an ArtifactPackage.
        let mut members = vec![self.id.clone()];
        for (index, selection) in self.selections.iter().enumerate() {
            let id = format!("{}-location-{}", self.id, index + 1);
            xml.start(
                "artifactElement",
                &[("xmi:type", "sacm:Resource"), ("xmi:id", &id)],
            )?;
            xml.name(&selection.path)?;
            xml.description(&format!(
                "suite: {}\nversion: {}\nverdict: {}",
                selection.suite,
                selection.at,
                verdict_name(selection.verdict),
            ))?;
            xml.end("artifactElement")?;
            members.push(id);
        }
        xml.start(
            "artifactElement",
            &[
                ("xmi:type", "sacm:ArtifactGroup"),
                ("xmi:id", &format!("{}-record", self.id)),
                ("artifactElement", &members.join(" ")),
            ],
        )?;
        xml.name("Retained record locations")?;
        xml.end("artifactElement")
    }
}

fn verdict_name(verdict: Option<EvidenceStatus>) -> &'static str {
    verdict.map_or("not recorded", EvidenceStatus::as_str)
}
