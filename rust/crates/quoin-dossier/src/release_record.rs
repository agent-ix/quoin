// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Release record document (FR-201): a view of the committed snapshot and the
//! Engineering Assurance release-verify result supplied by the caller.

use std::fmt::Write as _;
use std::fs;

use serde_json::Value;

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, line};
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};
use crate::release_snapshot::ReleaseSnapshot;

const EA_PROTOCOL: &str = "engineering-assurance.release-verify-result/v1";

/// The release record generator.
pub(crate) struct ReleaseRecordGenerator;

impl Generator for ReleaseRecordGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "release-record",
            id: "DOS-REL",
            title: "Release record",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let snapshot = inputs
            .snapshot
            .as_ref()
            .map(|path| read_snapshot(path))
            .transpose()?;
        let verify = inputs
            .release_verify
            .as_ref()
            .map(|path| read_verify(path))
            .transpose()?;
        Ok(render(snapshot.as_ref(), verify.as_ref()))
    }
}

fn read_snapshot(path: &std::path::Path) -> Result<ReleaseSnapshot, SourceFailure> {
    let source = path.display().to_string();
    let bytes = fs::read(path).map_err(|error| SourceFailure {
        source: source.clone(),
        detail: format!("cannot read snapshot: {error}"),
    })?;
    ReleaseSnapshot::parse(&bytes).map_err(|error| SourceFailure {
        source,
        detail: format!("invalid release snapshot: {error}"),
    })
}

fn read_verify(path: &std::path::Path) -> Result<Value, SourceFailure> {
    let source = path.display().to_string();
    let bytes = fs::read(path).map_err(|error| SourceFailure {
        source: source.clone(),
        detail: format!("cannot read release-verify result: {error}"),
    })?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| SourceFailure {
        source: source.clone(),
        detail: format!("invalid release-verify result: {error}"),
    })?;
    let object = value.as_object().ok_or_else(|| SourceFailure {
        source: source.clone(),
        detail: "release-verify result must be an object".to_owned(),
    })?;
    if object.get("protocol").and_then(Value::as_str) != Some(EA_PROTOCOL) {
        return Err(SourceFailure {
            source,
            detail: format!("protocol must be {EA_PROTOCOL}"),
        });
    }
    let valid_string = |key: &str| object.get(key).is_some_and(Value::is_string);
    let string_array = |value: &Value| {
        value
            .as_array()
            .is_some_and(|values| values.iter().all(Value::is_string))
    };
    let valid_strings = |key: &str| object.get(key).is_some_and(string_array);
    let valid_nested_strings = |key: &str, members: &[&str]| {
        object
            .get(key)
            .and_then(Value::as_object)
            .is_some_and(|nested| {
                members
                    .iter()
                    .all(|member| nested.get(*member).is_some_and(Value::is_string))
            })
    };
    let status = object.get("status").and_then(Value::as_str);
    let valid = match status {
        Some("released") => {
            valid_string("tag")
                && valid_strings("reasons")
                && object.get("approval").and_then(Value::as_object).is_some()
                && valid_nested_strings("release", &["id", "name", "snapshot_path"])
                && valid_nested_strings("baseline", &["name", "commit"])
                && object
                    .get("baseline")
                    .and_then(Value::as_object)
                    .and_then(|baseline| baseline.get("items"))
                    .and_then(Value::as_array)
                    .is_some()
                && valid_string("approver")
                && valid_strings("meaning")
                && valid_string("signed_at")
                && valid_string("signed_at_source")
                && object.get("accepted_anomalies").is_none_or(string_array)
        }
        Some("not_released") => valid_string("tag") && valid_strings("reasons"),
        _ => false,
    };
    if valid {
        Ok(value)
    } else {
        Err(SourceFailure {
            source,
            detail: "release-verify result is missing or has invalid fields for its status"
                .to_owned(),
        })
    }
}

fn render(snapshot: Option<&ReleaseSnapshot>, verify: Option<&Value>) -> Generated {
    let mut body = String::from("# Release record\n\n## Release\n\n");
    let mut gaps = Vec::new();
    if let Some(verify) = verify {
        render_release(&mut body, verify);
    } else {
        body.push_str("not stated\n\n");
    }
    body.push_str("## Baseline\n\n");
    if let Some(verify) = verify {
        render_baseline(&mut body, snapshot, verify);
    } else {
        body.push_str("Baseline commit: not stated\nBaseline name: not stated\nBaseline items: not stated\n\n");
    }
    body.push_str("## Decision\n\n");
    if let Some(verify) = verify {
        render_decision(&mut body, verify, &mut gaps);
    } else {
        body.push_str("Status: not stated\n\n");
        gaps.push(Gap::new(
            GapKind::NoReleaseVerifyInput,
            "DOS-REL",
            "no --release-verify result was supplied",
        ));
    }
    render_approval_disclosure(&mut body);

    body.push_str("## Evidence at release\n\n");
    if let Some(snapshot) = snapshot {
        render_evidence(&mut body, snapshot);
    } else {
        gaps.push(Gap::new(
            GapKind::NoSnapshot,
            "DOS-REL",
            "no --snapshot file was supplied",
        ));
    }
    body.push_str("## Version history\n\n");
    if let Some(snapshot) = snapshot {
        render_history(&mut body, snapshot);
    }
    let document_version = verify
        .filter(|value| value.get("status").and_then(Value::as_str) == Some("released"))
        .and_then(|value| value.get("release"))
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    Generated {
        body,
        document_version,
        gaps,
    }
}

fn render_release(body: &mut String, verify: &Value) {
    let release = verify.get("release").and_then(Value::as_object);
    let approval = verify.get("approval").and_then(Value::as_object);
    field(
        body,
        "Release id",
        release.and_then(|value| value.get("id")),
    );
    field(
        body,
        "Release name",
        release.and_then(|value| value.get("name")),
    );
    field(
        body,
        "Snapshot path",
        release.and_then(|value| value.get("snapshot_path")),
    );
    field(body, "Tag", approval.and_then(|value| value.get("tag")));
    body.push('\n');
}

fn render_baseline(body: &mut String, snapshot: Option<&ReleaseSnapshot>, verify: &Value) {
    let baseline = verify.get("baseline").and_then(Value::as_object);
    text_field(
        body,
        "Baseline commit",
        snapshot.map(|value| value.baseline_commit.as_str()),
    );
    field(
        body,
        "Baseline name",
        baseline.and_then(|value| value.get("name")),
    );
    field(
        body,
        "Baseline items",
        baseline.and_then(|value| value.get("items")),
    );
    body.push('\n');
}

fn render_decision(body: &mut String, verify: &Value, gaps: &mut Vec<Gap>) {
    let status = verify
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("not stated");
    let _ = writeln!(body, "Status: {status}\n");
    field(body, "Approver", verify.get("approver"));
    field(body, "Meaning", verify.get("meaning"));
    field(body, "Signed at", verify.get("signed_at"));
    field(body, "Signed at source", verify.get("signed_at_source"));
    field(body, "Accepted anomalies", verify.get("accepted_anomalies"));
    if status == "not_released" {
        let reasons = verify.get("reasons").and_then(Value::as_array);
        body.push_str("Reasons:\n");
        if let Some(reasons) = reasons {
            for reason in reasons {
                let _ = writeln!(body, "- {}", scalar(reason));
            }
        }
        body.push('\n');
        gaps.push(Gap::new(
            GapKind::ReleaseNotVerified,
            "DOS-REL",
            format!(
                "EA status is not_released: {}",
                reasons
                    .map(|values| values.iter().map(scalar).collect::<Vec<_>>().join("; "))
                    .unwrap_or_default()
            ),
        ));
    }
}

fn render_approval_disclosure(body: &mut String) {
    body.push_str("Approval entries in this export are treated as unbound to the exact bytes of this DOS-REL document because Quoin has not compared their source-record digests to those bytes. They report only the source records and verdicts in their inputs.\n\nVerifier identity: not provided as a distinct field by the source approval contracts.\n\n");
}

fn render_evidence(body: &mut String, snapshot: &ReleaseSnapshot) {
    field(body, "Coverage", json(&snapshot.coverage).as_ref());
    field(
        body,
        "Evidence freshness",
        json(&snapshot.evidence_freshness).as_ref(),
    );
    field(
        body,
        "Open findings",
        json(&snapshot.open_findings).as_ref(),
    );
    field(body, "SOUP", json(&snapshot.soup).as_ref());
    field(body, "Anomalies", json(&snapshot.anomalies).as_ref());
    body.push('\n');
}

fn render_history(body: &mut String, snapshot: &ReleaseSnapshot) {
    body.push_str("| Record | Revision | Commit |\n| --- | ---: | --- |\n");
    for record in &snapshot.change_records {
        let _ = writeln!(
            body,
            "| {} | {} | {} |",
            cell(&record.record_id),
            record.revision,
            cell(&record.commit)
        );
    }
    body.push('\n');
}

fn field(body: &mut String, label: &str, value: Option<&Value>) {
    let value = value.map_or_else(|| "not stated".to_owned(), scalar);
    let _ = writeln!(body, "{label}: {}", line(&value));
}

fn text_field(body: &mut String, label: &str, value: Option<&str>) {
    let _ = writeln!(body, "{label}: {}", line(value.unwrap_or("not stated")));
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "not stated".to_owned()),
    }
}

fn json(value: &impl serde::Serialize) -> Option<Value> {
    serde_json::to_value(value).ok()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic is the failure report"
)]
mod tests {
    use super::*;
    use crate::release_snapshot::ReleaseSnapshot;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn snapshot() -> ReleaseSnapshot {
        ReleaseSnapshot::parse(
            br#"{
              "protocol":"quoin.release-snapshot/v1",
              "as_of":"2026-11-01T00:00:00Z",
              "baseline_commit":"snapshot-commit",
              "previous_release":null,
              "coverage":[{"id":"FR-001","criteria":[{"id":"FR-001-AC-1","static_status":"tagged","evidence_status":"bound"}]}],
              "evidence_freshness":{"bound":1,"stale":0,"suspect":0,"undischarged":0,"no run evidence":0},
              "open_findings":[],
              "soup":{"rows":[],"not_assessed":[]},
              "anomalies":[],
              "change_records":[{"record_id":"REC-2","revision":2,"path":"records/2.md","commit":"commit-2"},{"record_id":"REC-1","revision":1,"path":"records/1.md","commit":"commit-1"}]
            }"#,
        )
        .unwrap()
    }

    #[trace("FR-201-AC-1", "FR-201-AC-4")]
    #[test]
    fn released_record_uses_the_snapshot_and_ea_fields_in_source_order() {
        let verify = json!({
            "protocol": EA_PROTOCOL,
            "status": "released",
            "tag": "v1.2.3",
            "reasons": [],
            "approval": {"tag":"v1.2.3"},
            "release": {"id":"REL-12","name":"Release 12","snapshot_path":"release/snapshot.json"},
            "baseline": {"name":"Baseline 12","commit":"snapshot-commit","items":[{"path":"spec.md","kind":"specification"}]},
            "approver": "Ada Example",
            "meaning": ["release authorization"],
            "signed_at": "2026-11-02T03:04:05Z",
            "signed_at_source": "tagger_date",
            "accepted_anomalies": ["ANOM-12"]
        });
        let generated = render(Some(&snapshot()), Some(&verify));
        assert!(generated.gaps.is_empty());
        assert_eq!(generated.document_version.as_deref(), Some("Release 12"));
        let first = generated.body.find("REC-2").unwrap();
        let second = generated.body.find("REC-1").unwrap();
        assert!(first < second);
        for source in [
            "Ada Example",
            "[\"release authorization\"]",
            "2026-11-02T03:04:05Z",
            "REL-12",
            "Release 12",
            "release/snapshot.json",
            "Baseline 12",
            "snapshot-commit",
            "ANOM-12",
        ] {
            assert!(generated.body.contains(source), "missing {source}");
        }
        assert!(generated.body.contains("Baseline commit: snapshot-commit"));
    }

    #[trace("FR-201-AC-2", "FR-201-AC-3")]
    #[test]
    fn not_released_is_a_gap_and_never_a_success_status() {
        let verify = json!({
            "protocol": EA_PROTOCOL,
            "status": "not_released",
            "tag": "v1.2.3",
            "reasons": ["approval_not_found", "snapshot_not_found"]
        });
        let generated = render(None, Some(&verify));
        assert!(generated.body.contains("Status: not_released"));
        assert!(generated.body.contains("approval_not_found"));
        assert!(generated.body.contains("snapshot_not_found"));
        assert!(!generated.body.contains("Status: released"));
        assert!(
            generated
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::NoSnapshot)
        );
        assert!(
            generated
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::ReleaseNotVerified)
        );
        let missing = render(None, None);
        assert!(
            missing
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::NoSnapshot)
        );
        assert!(
            missing
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::NoReleaseVerifyInput)
        );
    }

    #[trace("FR-201-AC-5")]
    #[test]
    fn evidence_store_changes_do_not_change_snapshot_counts_or_history() {
        let temp = tempfile::tempdir().unwrap();
        let snapshot_path = temp.path().join("snapshot.json");
        fs::write(&snapshot_path, serde_json::to_vec(&snapshot()).unwrap()).unwrap();
        let verify_path = temp.path().join("verify.json");
        fs::write(
            &verify_path,
            serde_json::to_vec(&json!({
                "protocol": EA_PROTOCOL,
                "status": "released",
                "tag": "v1",
                "reasons": [],
                "approval": {"status":"approved","tag":"v1"},
                "release": {"id":"REL-1","name":"Release one","snapshot_path":"release/snapshot.json"},
                "baseline": {"name":"Baseline one","commit":"baseline-commit","items":[]},
                "approver": "Ada Example",
                "meaning": ["release authorization"],
                "signed_at": "2026-11-02T03:04:05Z",
                "signed_at_source": "tagger_date",
                "accepted_anomalies": []
            }))
            .unwrap(),
        )
        .unwrap();
        let inputs = Inputs {
            scope: temp.path().to_owned(),
            modules: Vec::new(),
            clause_bindings: Vec::new(),
            pack: None,
            soup_binding: None,
            soup_records: None,
            soup_support: None,
            snapshot: Some(snapshot_path),
            release_verify: Some(verify_path),
            as_of: None,
            version: "1.0".to_owned(),
        };
        let generator = ReleaseRecordGenerator;
        let before = generator.generate(&inputs).unwrap();
        fs::write(temp.path().join("evidence-store.json"), br#"{"bound":99}"#).unwrap();
        let after = generator.generate(&inputs).unwrap();
        assert_eq!(after, before);
        assert!(after.body.contains("Evidence freshness: {\"bound\":1"));
        assert!(after.body.contains("REC-2"));
        assert!(after.body.contains("REC-1"));
    }

    #[trace("FR-201-AC-6")]
    #[test]
    fn snapshot_parser_rejects_unknown_members_and_wrong_protocols() {
        let mut bytes = br#"{"protocol":"quoin.release-snapshot/v1"}"#.to_vec();
        assert!(ReleaseSnapshot::parse(&bytes).is_err());
        bytes = serde_json::to_vec(&json!({
            "protocol":"other/v1",
            "as_of":"x",
            "baseline_commit":"x",
            "previous_release":null,
            "coverage":[],
            "evidence_freshness":{"bound":0,"stale":0,"suspect":0,"undischarged":0,"no run evidence":0},
            "open_findings":[],
            "soup":{"rows":[],"not_assessed":[]},
            "anomalies":[],
            "change_records":[],
            "unexpected":true
        })).unwrap();
        assert!(ReleaseSnapshot::parse(&bytes).is_err());
    }
}
