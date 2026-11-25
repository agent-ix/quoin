// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Acceptance fixtures for SACM argument and evidence XMI.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use quoin_assurance::gsn::{GsnCase, GsnEdge, GsnNode};
use quoin_assurance::sacm::{Export, export};
use quoin_evidence::source::{DiskEvidence, MemoryEvidence};
use serde_json::json;

#[path = "sacm/metamodel.rs"]
mod metamodel;

const TIMESTAMP: &str = "2026-11-23T10:20:00Z";
const SECOND_TIMESTAMP: &str = "2026-11-23T11:20:00Z";
const FIRST_DIGEST: &str = "43525f0578f3747767a425cd3d03ebf400e56d3da6ce128e65d10f2632f8ca7e";
const SECOND_DIGEST: &str = "baa781a4e36db2254bd7be41fcdc5d169a8e338f5565ba4f986514077f886e20";
const RUN: &[u8] = b"{\r\n  \"result\": \"private-run-record\"\r\n}\r\n";
const RUN_PATH: &str = "spec/evidence/runs/SUITE-1/2026-11-23T10~3A20~3A00Z.json";

fn node(id: &str, kind: &str) -> GsnNode {
    GsnNode {
        id: id.to_owned(),
        kind: kind.to_owned(),
        source: format!("source-{id}"),
        statement: format!("statement for {id} <é> & \"proof\"\nnext line"),
        status: "supported".to_owned(),
        open_reason: None,
        undeveloped: None,
        evidence: None,
        source_data: None,
    }
}

fn edge(kind: &str, from: &str, to: &str) -> GsnEdge {
    GsnEdge {
        kind: kind.to_owned(),
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

fn solution(id: &str, at: Option<&str>, verdict: &str) -> GsnNode {
    let mut sn = node(id, "solution");
    let mut suite = json!({ "suite": "SUITE-1" });
    if let Some(at) = at {
        suite["at"] = json!(at);
    }
    sn.evidence = Some(json!({ "suites": [suite], "verdict": verdict }));
    sn
}

fn model(nodes: Vec<GsnNode>, edges: Vec<GsnEdge>, roots: &[&str]) -> GsnCase {
    GsnCase {
        format: "quoin.gsn-case/v1".to_owned(),
        nodes,
        edges,
        roots: roots.iter().map(|id| (*id).to_owned()).collect(),
        unreachable: Vec::new(),
        unmapped: Vec::new(),
        signed_acceptance: None,
    }
}

fn shape_fixture() -> GsnCase {
    let mut open = node("G2", "goal");
    open.status = "open".to_owned();
    open.open_reason = Some("Review still open".to_owned());
    open.undeveloped = Some(true);
    let mut undeveloped = node("G3", "goal");
    undeveloped.undeveloped = Some(true);
    model(
        vec![
            node("G1", "goal"),
            open,
            node("S1", "strategy"),
            undeveloped,
            node("G4", "goal"),
            solution("Sn1", Some(TIMESTAMP), "bound"),
            node("C1", "context"),
            node("A1", "assumption"),
            node("J1", "justification"),
            node("CC1", "counter_claim"),
        ],
        vec![
            edge("supported_by", "G1", "G2"),
            edge("supported_by", "G1", "S1"),
            edge("supported_by", "S1", "G3"),
            edge("supported_by", "S1", "G4"),
            edge("supported_by", "G4", "Sn1"),
            edge("in_context_of", "G1", "C1"),
            edge("in_context_of", "G1", "A1"),
            edge("in_context_of", "S1", "J1"),
            edge("challenges", "CC1", "G3"),
        ],
        &["G1"],
    )
}

fn disk_repo() -> tempfile::TempDir {
    let repo = tempfile::tempdir().expect("scratch evidence store");
    let run = repo.path().join(RUN_PATH);
    std::fs::create_dir_all(run.parent().expect("run parent")).expect("create evidence store");
    std::fs::write(run, RUN).expect("write exact run bytes");
    repo
}

fn shape_export() -> Export {
    let repo = disk_repo();
    export(&shape_fixture(), &DiskEvidence::new(repo.path())).expect("fixture exports")
}

/// Provenance: PLAT-1649
#[trace("FR-226-AC-8")]
#[test]
#[ignore = "external SACM importer fixture; run via make test-sacm-interop"]
fn export_external_interop_fixture() {
    let output = std::env::var_os("QUOIN_SACM_INTEROP_FIXTURE")
        .expect("QUOIN_SACM_INTEROP_FIXTURE must name the generated XMI path");
    let mut fixture = shape_fixture();
    fixture.nodes.push(solution("Sn2", Some(TIMESTAMP), "stale"));
    fixture.edges.push(edge("supported_by", "G4", "Sn2"));
    let repo = disk_repo();
    let result = export(&fixture, &DiskEvidence::new(repo.path())).expect("fixture exports");
    let path = std::path::PathBuf::from(output);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture output directory");
    }
    std::fs::write(path, result.xmi).expect("write external interop fixture");
}

fn signed_shape_export() -> Export {
    let mut fixture = shape_fixture();
    fixture.signed_acceptance = Some(
        serde_json::from_value(json!({
            "label": "Human acceptance of case content", "record_id": "quoin.gsn-case/v1",
            "record_digest": format!("sha256:{SECOND_DIGEST}"),
            "tag": "case/review", "approver": "case-reviewer@example.invalid",
            "meaning": ["assurance-case-accepted"], "signer_source": "allowed_signers",
            "signed_at_missing": "tagger_date_unreadable", "trusted_at_missing": "ssh_signature"
        }))
        .expect("signed acceptance shape"),
    );
    let repo = disk_repo();
    export(&fixture, &DiskEvidence::new(repo.path())).expect("signed fixture exports")
}

#[trace("FR-226-AC-1", "FR-226-AC-2", "FR-226-AC-3", "FR-226-AC-4")]
#[test]
fn sacm_maps_every_node_kind_and_retains_counter_claims() {
    let first = shape_export();
    assert_eq!(first.xmi, shape_export().xmi);
    let parsed = parse_xml(&first.xmi);
    assert_eq!(parsed.role, "sacm:AssuranceCasePackage");
    assert_eq!(
        parsed.attr("xmlns:xmi"),
        "http://www.omg.org/spec/XMI/20131001"
    );
    assert_eq!(
        parsed.attr("xmlns:sacm"),
        "http://www.omg.org/spec/SACM/20220301"
    );
    assert_eq!(parsed.attr("xmi:version"), "2.5.1");
    assert_eq!(parsed.attr("xmi:type"), "sacm:AssuranceCasePackage");
    assert_eq!(parsed.children.len(), 3);
    let argument = parsed.child("argumentPackage");
    assert_eq!(argument.attr("xmi:type"), "sacm:ArgumentPackage");
    assert_eq!(
        parsed.child("artifactPackage").attr("xmi:type"),
        "sacm:ArtifactPackage"
    );
    let argument: Vec<_> = argument
        .children
        .iter()
        .filter(|item| item.role == "argumentElement")
        .collect();
    let expected = [
        ("G1", "Claim", "needsSupport"),
        ("G2", "Claim", "needsSupport"),
        ("S1", "ArgumentReasoning", ""),
        ("G3", "Claim", "needsSupport"),
        ("G4", "Claim", "asserted"),
        ("Sn1", "ArtifactReference", ""),
        ("C1", "ArtifactReference", ""),
        ("A1", "Claim", "assumed"),
        ("J1", "Claim", "asserted"),
        ("CC1", "Claim", "needsSupport"),
    ];
    assert_eq!(argument.len(), 18);
    for (element, (id, kind, declaration)) in argument.iter().zip(expected) {
        assert_eq!(element.attr("xmi:id"), id);
        assert_eq!(element.attr("xmi:type"), format!("sacm:{kind}"));
        assert_eq!(element.attr("assertionDeclaration"), declaration);
        assert_eq!(element.children[0].attr("content"), format!("source-{id}"));
    }
    assert_eq!(
        argument[1].description(),
        "open: Review still open\nstatement for G2 <é> & \"proof\"\nnext line"
    );
    let relations: Vec<_> = argument
        .iter()
        .skip(expected.len())
        .map(|relation| {
            (
                relation.attr("xmi:type"),
                relation.attr("source"),
                relation.attr("target"),
                relation.attr("reasoning"),
                relation.attr("isCounter"),
            )
        })
        .collect();
    assert_eq!(
        relations,
        [
            ("sacm:AssertedInference", "G2", "G1", "", "false"),
            ("sacm:AssertedInference", "G3", "G1", "S1", "false"),
            ("sacm:AssertedInference", "G4", "G1", "S1", "false"),
            ("sacm:AssertedEvidence", "Sn1", "G4", "", "false"),
            ("sacm:AssertedContext", "C1", "G1", "", "false"),
            ("sacm:AssertedContext", "A1", "G1", "", "false"),
            ("sacm:AssertedContext", "J1", "S1", "", "false"),
            ("sacm:AssertedInference", "CC1", "G3", "", "true"),
        ]
    );
    assert_eq!(argument[10].attr("xmi:id"), "G2-G1-AssertedInference-1");
    assert_eq!(argument[11].attr("xmi:id"), "G3-G1-AssertedInference-3");
    assert_eq!(resolve_references(&parsed), Ok(()));
    let invalid = String::from_utf8(first.xmi).expect("UTF-8").replacen(
        "reasoning=\"S1\"",
        "reasoning=\"invalid-reference\"",
        1,
    );
    assert_eq!(
        resolve_references(&parse_xml(invalid.as_bytes())),
        Err("invalid-reference".to_owned())
    );
}

#[trace(
    "FR-228-AC-1",
    "FR-228-AC-2",
    "FR-228-AC-3",
    "FR-228-AC-4",
    "FR-228-AC-5",
    "FR-228-AC-6",
    "FR-228-AC-7"
)]
#[test]
fn sacm_content_addresses_exact_bytes_and_retains_non_supporting_evidence() {
    let repo = disk_repo();
    let second_run = repo
        .path()
        .join("spec/evidence/runs/SUITE-1/2026-11-23T11~3A20~3A00Z.json");
    let mut different_bytes = RUN.to_vec();
    different_bytes.push(b' ');
    std::fs::write(&second_run, different_bytes).expect("write the second run");
    let disk = DiskEvidence::new(repo.path());
    let fixture = model(
        vec![
            node("G1", "goal"),
            solution("Sn1", Some(TIMESTAMP), "bound"),
            node("G2", "goal"),
            solution("Sn2", Some(TIMESTAMP), "stale"),
            node("G3", "goal"),
            solution("Sn3", Some(SECOND_TIMESTAMP), "suspect"),
            node("G4", "goal"),
            solution("Sn4", Some(TIMESTAMP), "undischarged"),
            node("G5", "goal"),
            solution("Sn5", None, "undischarged"),
            node("G6", "goal"),
            solution("Sn6", Some("absent"), "bound"),
        ],
        (1..=6)
            .map(|n| edge("supported_by", &format!("G{n}"), &format!("Sn{n}")))
            .collect(),
        &["G1", "G2", "G3", "G4", "G5", "G6"],
    );
    let first = export(&fixture, &disk).expect("evidence export");
    assert_eq!(
        first.xmi,
        export(&fixture, &disk).expect("repeat evidence export").xmi
    );
    let parsed = parse_xml(&first.xmi);
    assert_eq!(resolve_references(&parsed), Ok(()));
    let leaves: Vec<_> = parsed
        .child("artifactPackage")
        .children
        .iter()
        .filter(|item| item.attr("xmi:type") == "sacm:Artifact")
        .collect();
    assert_eq!(leaves.len(), 2);
    for (artifact, (digest, stamp, verdict)) in leaves.iter().zip([
        (FIRST_DIGEST, TIMESTAMP, "bound"),
        (SECOND_DIGEST, SECOND_TIMESTAMP, "suspect"),
    ]) {
        assert_eq!(artifact.attr("xmi:id"), format!("artifact-{digest}"));
        assert_eq!(artifact.attr("gid"), format!("sha256:{digest}"));
        assert_eq!(artifact.attr("version"), stamp);
        assert_eq!(artifact.children[0].attr("content"), "SUITE-1");
        assert_eq!(artifact.description(), format!("verdict: {verdict}"));
    }
    let locations: BTreeSet<_> = parsed
        .child("artifactPackage")
        .children
        .iter()
        .filter(|item| item.attr("xmi:type") == "sacm:Resource")
        .map(|item| item.children[0].attr("content"))
        .collect();
    assert_eq!(
        locations,
        BTreeSet::from([
            RUN_PATH,
            "spec/evidence/runs/SUITE-1/2026-11-23T11~3A20~3A00Z.json",
        ])
    );
    assert!(
        locations
            .iter()
            .all(|path| repo.path().join(path).is_file())
    );
    let package = parsed.child("artifactPackage");
    for digest in [FIRST_DIGEST, SECOND_DIGEST] {
        let group = package.by_id(&format!("artifact-{digest}-record"));
        assert_eq!(group.attr("xmi:type"), "sacm:ArtifactGroup");
        assert_eq!(
            group.attr("artifactElement"),
            format!("artifact-{digest} artifact-{digest}-location-1")
        );
        let resource = package.by_id(&format!("artifact-{digest}-location-1"));
        assert_eq!(resource.attr("location"), "");
        assert!(
            resource
                .children
                .iter()
                .all(|child| child.role != "location")
        );
    }
    let argument = parsed.child("argumentPackage");
    let evidence: Vec<_> = argument
        .children
        .iter()
        .filter(|child| child.attr("xmi:type") == "sacm:AssertedEvidence")
        .collect();
    assert_eq!(evidence.len(), 6);
    for (index, (digest, verdict)) in [
        (FIRST_DIGEST, ""),
        (FIRST_DIGEST, "stale"),
        (SECOND_DIGEST, "suspect"),
        (FIRST_DIGEST, "undischarged"),
        (
            "",
            "undischarged, missing run for SUITE-1 (no observation time)",
        ),
        (
            "",
            "recorded verdict: bound, missing run for SUITE-1 at absent",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let id = index + 1;
        let solution = argument.by_id(&format!("Sn{id}"));
        let claim = argument.by_id(&format!("G{id}"));
        let relation = evidence[index];
        let expected_ref = if digest.is_empty() {
            String::new()
        } else {
            format!("artifact-{digest}")
        };
        assert_eq!(solution.attr("referencedArtifactElement"), expected_ref);
        assert_eq!(relation.attr("source"), format!("Sn{id}"));
        assert_eq!(relation.attr("target"), format!("G{id}"));
        assert_eq!(relation.attr("isCounter"), "false");
        if verdict.is_empty() {
            assert_eq!(claim.attr("assertionDeclaration"), "asserted");
            assert_eq!(relation.attr("assertionDeclaration"), "asserted");
        } else {
            assert_eq!(claim.attr("assertionDeclaration"), "needsSupport");
            assert_eq!(relation.description(), format!("not supporting: {verdict}"));
            assert_eq!(relation.attr("assertionDeclaration"), "needsSupport");
        }
    }
    let xml = String::from_utf8(first.xmi).expect("UTF-8");
    assert!(
        !xml.contains("private-run-record")
            && !xml.contains(repo.path().to_str().expect("temp path is UTF-8"))
    );
    let digests: BTreeSet<_> = xml
        .split(|ch: char| !ch.is_ascii_hexdigit())
        .filter(|value| matches!(value.len(), 40 | 64))
        .collect();
    assert_eq!(digests, BTreeSet::from([FIRST_DIGEST, SECOND_DIGEST]));
    std::fs::write(&second_run, RUN).expect("copy identical bytes to the second retained path");
    let shared = parse_xml(&export(&fixture, &disk).expect("deduplicated bytes").xmi);
    assert_eq!(
        shared
            .child("artifactPackage")
            .children
            .iter()
            .filter(|item| item.attr("xmi:type") == "sacm:Artifact")
            .count(),
        1
    );
    assert_eq!(
        shared
            .child("argumentPackage")
            .by_id("Sn3")
            .attr("referencedArtifactElement"),
        format!("artifact-{FIRST_DIGEST}")
    );
    assert_eq!(resolve_references(&shared), Ok(()));
}

#[trace("FR-227-AC-2")]
#[test]
fn sacm_does_not_approximate_unmapped_nested_reasoning() {
    let authored = json!({
        "topClaim": { "id": "G-root", "statement": "Top Claim", "status": "open" },
        "reasoning": [
            { "id": "S-outer", "statement": "Outer reasoning", "supports": "G-root" },
            { "id": "S-inner", "statement": "Inner reasoning", "supports": "S-outer",
              "criteria": [{ "criterion": "Subclaim", "status": "open" }] },
        ],
    });
    let fixture = quoin_assurance::gsn::from_authored(&authored).expect("valid nested GSN");
    let exported = export(&fixture, &MemoryEvidence::default()).expect("export with omissions");
    let parsed = parse_xml(&exported.xmi);
    let argument = parsed.child("argumentPackage");
    for (id, source) in [("S1", "S-outer"), ("S2", "S-inner")] {
        let strategy = argument.by_id(id);
        assert_eq!(strategy.attr("xmi:type"), "sacm:ArgumentReasoning");
        assert_eq!(strategy.child("name").attr("content"), source);
    }
    assert!(
        argument
            .children
            .iter()
            .all(|element| element.attr("xmi:type") != "sacm:AssertedInference"),
        "a reported strategy path must produce no inference"
    );
    assert_eq!(
        exported.report.diagnostics().collect::<Vec<_>>(),
        [
            "unmapped: G-root: supported_by S-outer: strategy has no child for an asserted relationship",
            "unmapped: S-inner: supported_by S-inner:Subclaim: strategy has no direct parent Claim",
            "unmapped: S-outer: supported_by S-inner: nested reasoning has no SACM relationship between strategies",
        ]
    );
}

#[trace("FR-226-AC-6")]
#[test]
fn sacm_rejects_dangling_gsn_before_returning_xmi() {
    let mut fixture = shape_fixture();
    fixture
        .edges
        .push(edge("supported_by", "missing-parent", "missing-child"));
    let error = export(&fixture, &MemoryEvidence::default())
        .expect_err("invalid model is refused")
        .to_string();
    assert!(
        error.contains("WF-6")
            && error.contains("missing-parent")
            && error.contains("missing-child"),
        "{error}"
    );
}

#[derive(Debug)]
struct Element {
    role: String,
    attrs: BTreeMap<String, String>,
    children: Vec<Self>,
}

impl Element {
    fn attr(&self, name: &str) -> &str {
        self.attrs.get(name).map_or("", String::as_str)
    }

    fn child(&self, role: &str) -> &Self {
        self.children
            .iter()
            .find(|child| child.role == role)
            .expect("child role present")
    }

    fn by_id(&self, id: &str) -> &Self {
        self.children
            .iter()
            .find(|child| child.attr("xmi:id") == id)
            .expect("id present")
    }

    fn description(&self) -> &str {
        self.children
            .iter()
            .find(|item| item.role == "description")
            .expect("description exists")
            .children[0]
            .children[0]
            .attr("content")
    }

    fn walk<'a>(&'a self, output: &mut Vec<&'a Self>) {
        output.push(self);
        for child in &self.children {
            child.walk(output);
        }
    }
}

fn element(start: &BytesStart<'_>) -> Element {
    let name = start.name();
    Element {
        role: name.into_inner().to_owned(),
        attrs: start
            .attributes()
            .map(|attribute| {
                let attr = attribute.expect("valid attribute");
                (
                    attr.key.into_inner().to_owned(),
                    attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .expect("decoded attribute")
                        .into_owned(),
                )
            })
            .collect(),
        children: Vec::new(),
    }
}

fn parse_xml(bytes: &[u8]) -> Element {
    let mut reader = Reader::from_reader(bytes);
    let mut stack = Vec::<Element>::new();
    let mut root = None;
    loop {
        let complete = match reader.read_event().expect("valid XML") {
            Event::Start(start) => {
                stack.push(element(&start));
                None
            }
            Event::Empty(start) => Some(element(&start)),
            Event::End(_) => Some(stack.pop().expect("matching start element")),
            Event::Eof => break,
            _ => None,
        };
        if let Some(complete) = complete {
            if let Some(parent) = stack.last_mut() {
                parent.children.push(complete);
            } else {
                assert!(root.replace(complete).is_none(), "one document root");
            }
        }
    }
    root.expect("document root")
}

fn resolve_references(root: &Element) -> Result<(), String> {
    let mut elements = Vec::new();
    root.walk(&mut elements);
    let mut ids = BTreeSet::new();
    for id in elements
        .iter()
        .map(|item| item.attr("xmi:id"))
        .filter(|id| !id.is_empty())
    {
        if !ids.insert(id) {
            return Err(id.to_owned());
        }
    }
    for element in elements {
        for property in [
            "source",
            "target",
            "reasoning",
            "referencedArtifactElement",
            "artifactElement",
        ] {
            for id in element.attr(property).split_ascii_whitespace() {
                if !ids.contains(id) {
                    return Err(id.to_owned());
                }
            }
        }
    }
    Ok(())
}
