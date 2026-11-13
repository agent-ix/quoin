// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The licence-policy adapter (FR-150) over the SBOM reader (FR-041-AC-10).
//!
//! `licence-npm.syft.cdx.json` is the output of Syft 1.57.1 over the
//! engineering-assurance FR-039 licence fixture's `scan/` directory (FR-038
//! request shape), checked in unedited: five npm components, four declaring a
//! licence. A case Syft cannot produce from it (SPDX, an empty inventory, a
//! component with no purl) uses a minimal document written inline below.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::adapters::{
    AdapterSelection, FINDING_ADAPTERS, LicenceKind, parse_licence_policy, parse_sbom,
    read_components, select_finding_adapter,
};
use quoin_evidence::store::{gc, latest_scans, write_scan};
use quoin_evidence::types::{Finding, FindingRecord};
use quoin_evidence::{MemoryEvidence, STORE_SCHEMA_VERSION, SuiteId};

const SYFT: &str = include_str!("fixtures/licence-npm.syft.cdx.json");

const POLICY: &str = "\
format: licence-policy-v1
allow: [MIT]
deny: [GPL-3.0-only]
ban:
  - purl: pkg:npm/fx-vale
    reason: \"abandoned\"
";

fn run(sbom: &str, policy: &str) -> Vec<(String, String, String, String)> {
    parse_licence_policy(sbom, policy)
        .unwrap()
        .findings
        .into_iter()
        .map(|f| {
            (
                f.rule_id,
                f.severity.unwrap(),
                f.message.unwrap(),
                f.path.unwrap(),
            )
        })
        .collect()
}

fn finding(
    rule: &str,
    severity: &str,
    message: &str,
    path: &str,
) -> (String, String, String, String) {
    (rule.into(), severity.into(), message.into(), path.into())
}

fn refusal(sbom: &str, policy: &str) -> String {
    parse_licence_policy(sbom, policy).unwrap_err().to_string()
}

fn cdx(components: &str) -> String {
    format!(r#"{{"bomFormat":"CycloneDX","components":[{components}]}}"#)
}

/// The real Syft capture: an allowed, a denied, a not-listed, an unlicensed
/// and a banned component produce the four finding kinds on their own purls,
/// none for the allowed one, and the message, severity word and path of each.
#[trace("FR-150-AC-1", "FR-150-AC-11", "FR-150-AC-12", "FR-150-AC-16")]
#[test]
fn tc_1671_001_the_syft_licence_fixture_yields_each_finding_kind_on_its_purl() {
    let document: serde_json::Value = serde_json::from_str(SYFT).unwrap();
    assert_eq!(
        document["metadata"]["tools"]["components"][0]["name"],
        "syft"
    );
    assert_eq!(
        document["metadata"]["tools"]["components"][0]["version"],
        "1.57.1"
    );
    assert_eq!(document["components"].as_array().unwrap().len(), 5);

    assert_eq!(
        run(SYFT, POLICY),
        [
            finding(
                "licence-denied",
                "denied",
                "GPL-3.0-only",
                "pkg:npm/fx-ember@0.4.1"
            ),
            finding(
                "licence-unknown",
                "unknown",
                "no licence declared",
                "pkg:npm/fx-moss@0.9.0"
            ),
            finding(
                "licence-not-allowed",
                "not-allowed",
                "Apache-2.0",
                "pkg:npm/fx-quill@2.0.0"
            ),
            finding(
                "licence-unknown",
                "unknown",
                "MIT OR BSD-3-Clause",
                "pkg:npm/fx-vale@3.1.0"
            ),
            finding(
                "package-banned",
                "banned",
                "banned by pkg:npm/fx-vale: abandoned",
                "pkg:npm/fx-vale@3.1.0"
            ),
        ]
    );
}

/// The reader exposes what Syft recorded, kept as written, with a kind.
#[trace("FR-041-AC-10")]
#[test]
fn tc_1671_002_the_reader_exposes_declared_licences_by_kind() {
    let components = read_components(SYFT).unwrap();
    let by_purl = |purl: &str| components.iter().find(|c| c.symbol == purl).unwrap();
    let ember = by_purl("pkg:npm/fx-ember@0.4.1");
    assert_eq!(ember.licences.len(), 1);
    assert_eq!(ember.licences[0].kind, LicenceKind::Id);
    assert_eq!(ember.licences[0].text, "GPL-3.0-only");
    let vale = by_purl("pkg:npm/fx-vale@3.1.0");
    assert_eq!(vale.licences[0].kind, LicenceKind::Expression);
    assert!(by_purl("pkg:npm/fx-moss@0.9.0").licences.is_empty());

    let name = read_components(&cdx(
        r#"{"name":"n","version":"1","licenses":[{"license":{"name":"Custom Terms"}}]}"#,
    ))
    .unwrap();
    assert_eq!(name[0].licences[0].kind, LicenceKind::Name);
    assert_eq!(name[0].licences[0].text, "Custom Terms");
}

/// SPDX `MIT` is an id, `MIT OR Apache-2.0` an expression, `NOASSERTION` and
/// `NONE` declare nothing, and the run entries do not change when the licences
/// are exposed.
#[trace("FR-041-AC-10", "FR-150-AC-2", "FR-150-AC-8")]
#[test]
fn tc_1671_003_spdx_values_are_read_by_the_same_reader() {
    let spdx = r#"{"spdxVersion":"SPDX-2.3","packages":[
        {"name":"a","versionInfo":"1","licenseDeclared":"MIT"},
        {"name":"b","versionInfo":"2","licenseDeclared":"MIT OR Apache-2.0"},
        {"name":"c","versionInfo":"3","licenseDeclared":"NOASSERTION"},
        {"name":"d","versionInfo":"4","licenseDeclared":"NONE"}]}"#;
    let components = read_components(spdx).unwrap();
    assert_eq!(components[0].licences[0].kind, LicenceKind::Id);
    assert_eq!(components[1].licences[0].kind, LicenceKind::Expression);
    assert!(components[2].licences.is_empty() && components[3].licences.is_empty());

    let entries: Vec<String> = parse_sbom(spdx)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();
    let stripped = spdx.replace("licenseDeclared", "ignored");
    let before: Vec<String> = parse_sbom(&stripped)
        .unwrap()
        .entries
        .iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();
    assert_eq!(entries, before);
    assert_eq!(entries[0], r#"{"symbol":"a@1","outcome":"pass"}"#);

    // The licence-policy adapter reads SPDX through the same reader.
    let policy = "format: licence-policy-v1\nallow: [MIT]\n";
    assert_eq!(
        run(spdx, policy),
        [
            finding("licence-unknown", "unknown", "MIT OR Apache-2.0", "b@2"),
            finding("licence-unknown", "unknown", "no licence declared", "c@3"),
            finding("licence-unknown", "unknown", "no licence declared", "d@4"),
        ]
    );
    // A document that is neither format is refused as the reader refuses it.
    let error = refusal(r#"{"hello":1}"#, policy);
    assert!(error.contains("neither a CycloneDX"), "{error}");
}

/// A component with an empty or absent `licenses` array is unknown, never
/// omitted and never allowed.
#[trace("FR-150-AC-2")]
#[test]
fn tc_1671_004_no_declared_licence_is_unknown_not_allowed() {
    let sbom = cdx(r#"{"name":"a","version":"1","licenses":[]},{"name":"b","version":"2"}"#);
    assert_eq!(
        run(&sbom, "format: licence-policy-v1\nallow: [MIT]\n"),
        [
            finding("licence-unknown", "unknown", "no licence declared", "a@1"),
            finding("licence-unknown", "unknown", "no licence declared", "b@2"),
        ]
    );
}

/// An expression is unknown until its exact text is in `allow`; an expression or
/// name equal to a `deny` entry is denied. Nothing is parsed, and matching is
/// ASCII case-insensitive.
#[trace("FR-150-AC-3")]
#[test]
fn tc_1671_005_an_expression_is_matched_verbatim_and_never_parsed() {
    let sbom = cdx(
        r#"{"name":"a","version":"1","licenses":[{"expression":"MIT OR Apache-2.0"}]},
           {"name":"b","version":"1","licenses":[{"license":{"name":"Evil Terms"}}]}"#,
    );
    let unlisted = "format: licence-policy-v1\nallow: [MIT, Apache-2.0]\n";
    assert_eq!(
        run(&sbom, unlisted),
        [
            finding("licence-unknown", "unknown", "MIT OR Apache-2.0", "a@1"),
            finding("licence-unknown", "unknown", "Evil Terms", "b@1"),
        ]
    );
    let listed = "format: licence-policy-v1\nallow: [\"mit or apache-2.0\"]\ndeny: [Evil Terms]\n";
    assert_eq!(
        run(&sbom, listed),
        [finding("licence-denied", "denied", "Evil Terms", "b@1")]
    );
}

/// A version-less entry bans every version, a versioned one only its own, and a
/// banned component with a denied licence yields both findings. A purl differing
/// in type case or percent-encoding still matches.
#[trace("FR-150-AC-4")]
#[test]
fn tc_1671_006_bans_match_by_purl_equality() {
    let sbom = cdx(
        r#"{"name":"p","version":"1","purl":"pkg:npm/p@1","licenses":[{"license":{"id":"GPL-3.0-only"}}]},
           {"name":"p","version":"2","purl":"pkg:npm/p@2","licenses":[{"license":{"id":"MIT"}}]},
           {"name":"s","version":"1","purl":"pkg:NPM/%40scope/s@1?x=y","licenses":[{"license":{"id":"MIT"}}]}"#,
    );
    let policy = "format: licence-policy-v1\nallow: [MIT]\ndeny: [GPL-3.0-only]\nban:\n  - purl: pkg:npm/p@2\n  - purl: pkg:npm/@scope/s\n    reason: why\n";
    assert_eq!(
        run(&sbom, policy),
        [
            finding("licence-denied", "denied", "GPL-3.0-only", "pkg:npm/p@1"),
            finding(
                "package-banned",
                "banned",
                "banned by pkg:npm/p@2",
                "pkg:npm/p@2"
            ),
            finding(
                "package-banned",
                "banned",
                "banned by pkg:npm/@scope/s: why",
                "pkg:NPM/%40scope/s@1?x=y"
            ),
        ]
    );
    let everything = "format: licence-policy-v1\nallow: [MIT]\nban:\n  - purl: pkg:npm/p\n";
    let banned: Vec<_> = run(&sbom, everything)
        .into_iter()
        .filter(|f| f.0 == "package-banned")
        .collect();
    assert_eq!(banned.len(), 2);
}

/// All-allowed is a scan with no findings that evaluated its components; an
/// empty SBOM evaluates none, which is vacuous.
#[trace("FR-150-AC-5", "FR-150-AC-15")]
#[test]
fn tc_1671_007_clean_and_vacuous_scans_are_distinct() {
    let sbom = cdx(r#"{"name":"a","version":"1","licenses":[{"license":{"id":"MIT"}}]}"#);
    let policy = "format: licence-policy-v1\nallow: [MIT]\ndeny: [GPL-3.0-only]\n";
    let clean = parse_licence_policy(&sbom, policy).unwrap();
    assert!(clean.findings.is_empty());
    assert_eq!(clean.rules_evaluated, Some(1));
    assert_eq!(clean.evaluated, Some(vec!["a@1".to_owned()]));
    assert_eq!(clean.tool.as_deref(), Some("licence-policy"));
    let ruleset: serde_json::Value =
        serde_json::from_str(clean.ruleset.as_deref().unwrap()).unwrap();
    assert_eq!(ruleset["allow"], serde_json::json!(["MIT"]));
    assert_eq!(ruleset["deny"], serde_json::json!(["GPL-3.0-only"]));

    let vacuous = parse_licence_policy(&cdx(""), policy).unwrap();
    assert!(vacuous.findings.is_empty());
    assert_eq!(vacuous.rules_evaluated, Some(0));

    // The real capture: every component is listed.
    let real = parse_licence_policy(SYFT, POLICY).unwrap();
    assert_eq!(real.evaluated.unwrap().len(), 5);
    assert!(real.ruleset.unwrap().contains("fx-vale"));
}

/// Each way a policy can be wrong is refused, naming the reason.
#[trace("FR-150-AC-7")]
#[test]
fn tc_1671_008_a_bad_policy_is_refused_with_its_reason() {
    let sbom = cdx(r#"{"name":"a","version":"1"}"#);
    for (policy, reason) in [
        ("allow: [MIT\n  bad: : :", "not valid YAML"),
        ("allow: [MIT]\n", "format: licence-policy-v1"),
        (
            "format: licence-policy-v2\nallow: [MIT]\n",
            "format: licence-policy-v1",
        ),
        ("format: licence-policy-v1\n", "`allow` is required"),
        (
            "format: licence-policy-v1\nallow: []\n",
            "`allow` is required",
        ),
        (
            "format: licence-policy-v1\nallow: [MIT]\nextra: 1\n",
            "unknown key `extra`",
        ),
        (
            "format: licence-policy-v1\nallow: [MIT]\nban:\n  - purl: left-pad\n",
            "not a parseable purl",
        ),
        (
            "format: licence-policy-v1\nallow: [MIT]\nban:\n  - reason: x\n",
            "no `purl`",
        ),
        (
            "format: licence-policy-v1\nallow: [MIT]\nban:\n  - purl: pkg:npm/a\n    why: x\n",
            "unknown key `why`",
        ),
    ] {
        let error = refusal(&sbom, policy);
        assert!(error.starts_with("licence-policy: "), "{error}");
        assert!(error.contains(reason), "{policy:?} -> {error}");
    }
}

/// A component with no purl is recorded under `name@version`, which no ban
/// entry can match.
#[trace("FR-150-AC-9")]
#[test]
fn tc_1671_009_a_component_without_a_purl_is_pathed_by_name_and_never_banned() {
    let sbom =
        cdx(r#"{"name":"left-pad","version":"1.0.0","licenses":[{"license":{"id":"MIT"}}]}"#);
    let policy = "format: licence-policy-v1\nallow: [MIT]\nban:\n  - purl: pkg:npm/left-pad\n";
    assert!(run(&sbom, policy).is_empty());
    let unlicensed = cdx(r#"{"name":"left-pad","version":"1.0.0"}"#);
    assert_eq!(
        run(&unlicensed, policy),
        [finding(
            "licence-unknown",
            "unknown",
            "no licence declared",
            "left-pad@1.0.0"
        )]
    );
}

/// One id in `allow` with a free-text name outside it is unknown; with an id in
/// neither list not-allowed; with an id in `deny` and anything else denied.
/// Each component has at most one licence finding.
#[trace("FR-150-AC-14")]
#[test]
fn tc_1671_010_precedence_gives_one_licence_finding_per_component() {
    let sbom = cdx(
        r#"{"name":"a","version":"1","licenses":[{"license":{"id":"MIT"}},{"license":{"name":"Free Text"}}]},
           {"name":"b","version":"1","licenses":[{"license":{"id":"MIT"}},{"license":{"id":"Zlib"}}]},
           {"name":"c","version":"1","licenses":[{"license":{"id":"GPL-3.0-only"}},{"license":{"id":"Zlib"}},{"license":{"name":"Free Text"}}]},
           {"name":"d","version":"1","licenses":[{"license":{"name":"Free Text"}},{"license":{"id":"Zlib"}}]}"#,
    );
    let policy = "format: licence-policy-v1\nallow: [MIT]\ndeny: [GPL-3.0-only]\n";
    assert_eq!(
        run(&sbom, policy),
        [
            finding("licence-unknown", "unknown", "Free Text", "a@1"),
            finding("licence-not-allowed", "not-allowed", "Zlib", "b@1"),
            finding("licence-denied", "denied", "GPL-3.0-only", "c@1"),
            finding("licence-not-allowed", "not-allowed", "Zlib", "d@1"),
        ]
    );
}

/// `--policy` is the licence-policy adapter's alone, and it is required there.
#[trace("FR-150-AC-6", "FR-150-AC-13")]
#[test]
fn tc_1671_011_the_policy_input_belongs_to_one_adapter_only() {
    let adapter = |name: &str| {
        select_finding_adapter(AdapterSelection {
            adapter: Some(name),
            tool: None,
        })
        .unwrap()
    };
    let sbom = cdx(r#"{"name":"a","version":"1"}"#);
    let missing = adapter("licence-policy")
        .run(&sbom, None)
        .unwrap_err()
        .to_string();
    assert!(
        missing.starts_with("licence-policy: usage: --policy"),
        "{missing}"
    );

    for name in ["cargo-deny", "cargo-audit", "sarif", "audit-script"] {
        let error = adapter(name)
            .run("{}", Some(POLICY))
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with(&format!("{name}: --policy applies only")),
            "{error}"
        );
    }
    assert!(FINDING_ADAPTERS.iter().any(|a| a.name == "licence-policy"));
    // Selection by the declared tool, as every other finding adapter.
    let by_tool = select_finding_adapter(AdapterSelection {
        adapter: None,
        tool: Some("licence-policy 1"),
    });
    assert_eq!(by_tool.unwrap().name, "licence-policy");
}

/// The adapter source names no licence identifier and ships no policy.
#[trace("FR-150-AC-10", "FR-150-CON-1")]
#[test]
fn tc_1671_012_the_adapter_source_carries_no_licence_list() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/adapters/licence_policy.rs"),
    )
    .unwrap();
    // The file's own SPDX header names a licence; no other line may.
    let source: String = source
        .lines()
        .filter(|line| !line.starts_with("// SPDX-License-Identifier"))
        .collect();
    for identifier in [
        "MIT",
        "Apache",
        "GPL",
        "BSD",
        "ISC",
        "MPL",
        "Unlicense",
        "allow: [",
    ] {
        assert!(
            !source.contains(identifier),
            "licence_policy.rs names {identifier}"
        );
    }
    // The SBOM is read by the FR-041 reader alone: this module names neither
    // document format.
    for format_key in [
        "bomFormat",
        "spdxVersion",
        "licenseDeclared",
        "\"licenses\"",
    ] {
        assert!(
            !source.contains(format_key),
            "licence_policy.rs reads SBOM key {format_key}"
        );
    }
}

fn scan(evaluated: Option<Vec<String>>) -> FindingRecord {
    FindingRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new("SUITE-1"),
        tool: "licence-policy".to_owned(),
        evidence_kind: None,
        timestamp: "2026-11-01T00:00:00Z".to_owned(),
        ruleset: None,
        rules_evaluated: Some(1),
        findings: vec![Finding::new("licence-unknown")],
        evaluated,
    }
}

/// An `evaluated` list survives write, read and gc; a scan without one reads
/// back without the field and claims no coverage.
#[trace("FR-034-AC-27")]
#[test]
fn tc_1671_013_an_evaluated_list_is_kept_and_its_absence_claims_nothing() {
    let mut store = MemoryEvidence::new();
    write_scan(&mut store, &scan(Some(vec!["pkg:npm/a@1".to_owned()]))).unwrap();
    let mut other = scan(None);
    other.suite = SuiteId::new("SUITE-2");
    let path = write_scan(&mut store, &other).unwrap();

    gc(&mut store, false).unwrap();
    let mut skipped = Vec::new();
    let scans = latest_scans(&store, &mut skipped).unwrap();
    let with = scans
        .iter()
        .find(|s| s.suite.as_str() == "SUITE-1")
        .unwrap();
    let without = scans
        .iter()
        .find(|s| s.suite.as_str() == "SUITE-2")
        .unwrap();
    assert_eq!(
        with.evaluated.as_deref(),
        Some(&["pkg:npm/a@1".to_owned()][..])
    );
    assert_eq!(without.evaluated, None);
    let stored = quoin_evidence::EvidenceSource::read(&store, &path)
        .unwrap()
        .unwrap();
    assert!(!stored.contains("evaluated"), "{stored}");
}

/// A licence entry the reader cannot read is still a declared licence: the
/// component is unknown, never all-allowed because the entry vanished.
#[trace("FR-150-AC-2", "FR-150-AC-3")]
#[test]
fn tc_1671_014_an_unreadable_licence_entry_is_unknown_never_allowed() {
    let sbom = cdx(
        r#"{"name":"a","version":"1","licenses":[{"license":{"id":"MIT"}},{"license":{"url":"https://x"}}]},
           {"name":"b","version":"1","licenses":[{"license":{"id":"MIT"}},{"expression":42}]},
           {"name":"c","version":"1","licenses":[{"license":{"id":"MIT"}},{"license":{"id":""}}]}"#,
    );
    let findings = run(&sbom, "format: licence-policy-v1\nallow: [MIT]\n");
    let names: Vec<(&str, &str)> = findings
        .iter()
        .map(|f| (f.0.as_str(), f.3.as_str()))
        .collect();
    assert_eq!(
        names,
        [
            ("licence-unknown", "a@1"),
            ("licence-unknown", "b@1"),
            ("licence-unknown", "c@1")
        ]
    );
    assert!(findings[0].2.contains("https://x"), "{}", findings[0].2);
}

/// An SPDX value with the `+` operator is an expression, not an id; several
/// offending texts are joined with `, `.
#[trace("FR-041-AC-10", "FR-150-AC-16")]
#[test]
fn tc_1671_015_operators_make_an_expression_and_messages_join_their_texts() {
    let spdx = r#"{"spdxVersion":"SPDX-2.3","packages":[
        {"name":"a","versionInfo":"1","licenseDeclared":"GPL-2.0+"}]}"#;
    assert_eq!(
        read_components(spdx).unwrap()[0].licences[0].kind,
        LicenceKind::Expression
    );
    let sbom = cdx(
        r#"{"name":"m","version":"1","licenses":[{"license":{"id":"Zlib"}},{"license":{"id":"ISC"}}]}"#,
    );
    assert_eq!(
        run(&sbom, "format: licence-policy-v1\nallow: [MIT]\n"),
        [finding(
            "licence-not-allowed",
            "not-allowed",
            "Zlib, ISC",
            "m@1"
        )]
    );
}
