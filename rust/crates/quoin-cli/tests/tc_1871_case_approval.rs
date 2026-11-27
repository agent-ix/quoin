// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Signed case export through real Git SSH tags and Engineering Assurance.

#![cfg(target_os = "linux")]
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "fixture failures are reported as test panics"
)]

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const KIND: &str = "quoin.gsn-case/v1";
const MEANING: &str = "assurance-case-accepted";
const SIGNING_TIME: &str = "2026-01-01T02:03:04Z";
const PRINCIPAL: &str = "case-reviewer@example.invalid";
const DERIVED: &str = "---\nid: FR-001\ntype: FR\ntitle: Write the case\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|---|---|---|\n| FR-001-AC-1 | Retain a bounded case. | |\n";
const MODULE: &str = "
name: case-approval-fixture
version: 0.1.0
description: Closed local assurance corpus
artifact_types:
  - name: FR
traceability:
  trace_targets:
    - name: acceptance-criterion
      archetype: FR
      section: Acceptance Criteria
      id_column: ID
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
";
const AUTHORED: &str =
    include_str!("../../../../tests/fixtures/assurance-argument-cli/argument-no-refs.md");

fn verifier() -> PathBuf {
    let path = std::env::var_os("QUOIN_E2E_EA")
        .map(PathBuf::from)
        .expect("QUOIN_E2E_EA must name the built Engineering Assurance executable");
    assert!(path.is_absolute() && path.is_file(), "{}", path.display());
    path
}

#[derive(Clone, Copy)]
enum Case {
    Derived,
    Authored,
}

struct Fixture {
    home: tempfile::TempDir,
    repo: PathBuf,
    case: Case,
}

impl Fixture {
    fn new(case: Case) -> Self {
        let home = tempfile::tempdir().expect("scratch case");
        let repo = home.path().join("repo");
        fs::create_dir_all(repo.join("spec")).expect("local corpus");
        fs::write(repo.join("manifest.yaml"), MODULE).expect("local module");
        fs::write(repo.join("spec/derived.md"), DERIVED).expect("derived requirements");
        fs::write(repo.join("spec/authored.md"), AUTHORED).expect("authored argument");
        fs::write(repo.join("decisions.json"), "[]").expect("empty decisions");
        let fixture = Self { home, repo, case };
        fixture.git(&["init", "-q", "-b", "main"]);
        for (key, value) in [
            ("user.name", "Fixture Operator"),
            ("user.email", "operator@example.invalid"),
            ("gpg.format", "ssh"),
        ] {
            fixture.git(&["config", key, value]);
        }
        fixture.git(&["add", "."]);
        fixture.git(&["commit", "-qm", "Fixture corpus"]);
        success(
            &Command::new("ssh-keygen")
                .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                .arg(fixture.key())
                .output()
                .expect("ssh-keygen installed"),
        );
        fixture.trust("");
        fixture
    }

    fn key(&self) -> PathBuf {
        self.home.path().join("signer")
    }
    fn allowed(&self) -> PathBuf {
        self.home.path().join("allowed-signers")
    }

    fn trust(&self, option: &str) {
        let public = fs::read_to_string(self.key().with_extension("pub")).expect("public key");
        fs::write(
            self.allowed(),
            format!("{PRINCIPAL} {option} {}\n", public.trim()),
        )
        .expect("caller trust configuration");
    }

    fn git_command(repo: &Path) -> Command {
        let mut git = Command::new("git");
        git.current_dir(repo)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_COMMITTER_DATE", SIGNING_TIME);
        git
    }

    fn git(&self, args: &[&str]) -> String {
        success(
            &Self::git_command(&self.repo)
                .args(args)
                .output()
                .expect("git runs"),
        )
        .trim()
        .to_owned()
    }

    fn signed_tag(&self, name: &str, record: &str, digest: &str, meanings: &[&str]) {
        let meaning = if meanings.is_empty() {
            String::new()
        } else {
            format!(
                "Signed-off-meaning: {}\n",
                meanings.join("\nSigned-off-meaning: ")
            )
        };
        let message = format!(
            "Review the bounded case\n\nApproved-Record-Id: {record}\nApproved-Record-Digest: {digest}\n{meaning}"
        );
        self.git(&[
            "-c",
            &format!("user.signingkey={}", self.key().display()),
            "tag",
            "-s",
            name,
            "-m",
            &message,
        ]);
    }

    fn command(&self, repo: &Path, flag: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
        command
            .current_dir(repo)
            .env("IX_HOME", self.home.path().join("ix"))
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args([
                "assurance",
                "--repo",
                ".",
                "--as-of",
                "2026-01-02T00:00:00Z",
            ])
            .arg(flag);
        match self.case {
            Case::Derived => {
                command.args(["--module", ".", "--claim-type", "FR"]);
            }
            Case::Authored => {
                command.args(["--argument", "AA-10180", "--decisions", "decisions.json"]);
            }
        }
        command
    }

    fn output(&self, flag: &str, args: &[&str]) -> Output {
        self.command(&self.repo, flag)
            .args(args)
            .output()
            .expect("quoin runs")
    }

    fn verified(&self, flag: &str, tag: &str, ea: &Path, trust: &str) -> Output {
        self.output(
            flag,
            &[
                "--case-approval-tag",
                tag,
                "--case-approval-verifier",
                ea.to_str().unwrap(),
                "--case-approval-allowed-signers",
                trust,
            ],
        )
    }

    fn trust_path(&self) -> String {
        self.allowed().to_str().unwrap().to_owned()
    }

    fn unsigned(&self) -> Value {
        serde_json::from_str(&success(&self.output("--gsn", &[]))).expect("underlying GSN is JSON")
    }
}

fn success(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}

fn without_approval(output: &Output, unsigned: &Value, diagnostic: &str) {
    let json: Value = serde_json::from_str(&success(output)).expect("underlying case is present");
    assert_eq!(&json, unsigned);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(diagnostic),
        "wanted {diagnostic:?}: {stderr}"
    );
}

fn independent_digest(unsigned: &Value) -> String {
    let bytes = serde_json_canonicalizer::to_vec(unsigned).expect("independent RFC 8785 encoder");
    let mut checksum = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("sha256sum installed");
    checksum.stdin.take().unwrap().write_all(&bytes).unwrap();
    let hex = success(&checksum.wait_with_output().unwrap());
    format!("sha256:{}", hex.split_whitespace().next().unwrap())
}

fn strip_acceptance_artifact(xmi: &str) -> &str {
    let start =
        "    <artifactElement xmi:type=\"sacm:Artifact\" xmi:id=\"case-signed-acceptance\">";
    let (_, artifact) = xmi.split_once(start).expect("one acceptance artifact");
    artifact
        .split_once("    </artifactElement>\n")
        .expect("acceptance artifact ends")
        .0
}

fn assert_acceptance_artifact(xmi: &str, accepted: &Value, unsigned_xmi: &str) {
    assert_eq!(xmi.matches("xmi:id=\"case-signed-acceptance\"").count(), 1);
    let marker =
        "    <artifactElement xmi:type=\"sacm:Artifact\" xmi:id=\"case-signed-acceptance\">";
    let (before, after) = xmi.split_once(marker).expect("acceptance Artifact");
    assert!(
        before
            .rfind("<artifactPackage ")
            .is_some_and(|position| position > before.find("</argumentPackage>").unwrap())
    );
    let (_, after) = after
        .split_once("    </artifactElement>\n")
        .expect("acceptance Artifact ends");
    assert_eq!(format!("{before}{after}"), unsigned_xmi);
    let artifact = strip_acceptance_artifact(xmi);
    assert!(artifact.contains("<name content=\"Human acceptance of case content\"/>"));
    for key in [
        "record_id",
        "record_digest",
        "tag",
        "approver",
        "signer_source",
        "signed_at",
        "signed_at_source",
        "signed_at_missing",
        "trusted_at_missing",
        "meaning",
    ] {
        let value = if key == "meaning" {
            accepted[key][0].as_str()
        } else {
            accepted[key].as_str()
        };
        let prefix = format!("<key>\n          <value content=\"{key}\"/>\n        </key>");
        if let Some(value) = value {
            let (_, tagged_value) = artifact
                .split_once(&prefix)
                .unwrap_or_else(|| panic!("missing {key} in {artifact}"));
            assert!(tagged_value.starts_with(&format!("\n        <content>\n          <value content=\"{value}\"/>\n        </content>")), "{key}: {tagged_value}");
        } else {
            assert!(!artifact.contains(&prefix), "{key} appeared unexpectedly");
        }
    }
}

#[trace("FR-225-AC-1", "FR-225-AC-4", "FR-226-AC-9")]
#[test]
#[ignore = "real case-approval lane requires QUOIN_E2E_EA, Git, ssh-keygen and sha256sum"]
fn sacm_and_gsn_share_the_real_verified_approval_for_both_case_modes() {
    let ea = verifier();
    for case in [Case::Derived, Case::Authored] {
        let fixture = Fixture::new(case);
        let unsigned = fixture.unsigned();
        assert!(
            unsigned["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|node| node["status"] == "open")
        );
        let digest = independent_digest(&unsigned);
        let presentation = serde_json::to_string_pretty(&unsigned).unwrap();
        assert_eq!(
            independent_digest(&serde_json::from_str(&presentation).unwrap()),
            digest
        );
        fixture.signed_tag("case/approved", KIND, &digest, &[MEANING]);
        let approved = fixture.verified(
            "--gsn",
            "refs/tags/case/approved",
            &ea,
            &fixture.trust_path(),
        );
        let mut gsn: Value = serde_json::from_str(&success(&approved)).expect("signed GSN");
        let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../quoin-assurance/schemas/gsn-case-v1.schema.json");
        let schema: Value = serde_json::from_slice(&fs::read(&schema_path).unwrap()).unwrap();
        let validator =
            quoin_jsonschema::SchemaValidator::compile_vendored(&schema_path, &schema, &[])
                .expect("GSN schema compiles");
        assert!(validator.is_valid(&gsn), "{:?}", validator.errors(&gsn));
        let warning = gsn
            .get("warning")
            .and_then(Value::as_str)
            .unwrap_or("<none>")
            .to_owned();
        let accepted = gsn.as_object_mut().unwrap().remove("signed_acceptance");
        assert!(
            accepted.is_some(),
            "signed acceptance missing; warning={warning}; stderr={}; payload={gsn}",
            String::from_utf8_lossy(&approved.stderr)
        );
        let accepted = accepted.unwrap();
        assert_eq!(gsn, unsigned);
        assert_eq!(
            accepted,
            json!({
                "label": "Human acceptance of case content", "record_id": KIND, "record_digest": digest,
                "tag": "case/approved", "approver": PRINCIPAL, "meaning": [MEANING],
                "signer_source": "allowed_signers", "signed_at": SIGNING_TIME,
                "signed_at_source": "tagger_date", "trusted_at_missing": "ssh_signature",
            })
        );
        let unsigned_xmi = success(&fixture.output("--sacm", &[]));
        let xmi = success(&fixture.verified("--sacm", "case/approved", &ea, &fixture.trust_path()));
        assert_acceptance_artifact(&xmi, &accepted, &unsigned_xmi);

        let (file, original, sentence) = match case {
            Case::Derived => ("spec/derived.md", DERIVED, "Retain a bounded case."),
            Case::Authored => (
                "spec/authored.md",
                AUTHORED,
                "The bounded synthetic widget change is acceptable.",
            ),
        };
        fs::write(
            fixture.repo.join(file),
            original.replace(sentence, "The changed case needs review."),
        )
        .unwrap();
        let changed = fixture.unsigned();
        assert_ne!(independent_digest(&changed), digest);
        without_approval(
            &fixture.verified("--gsn", "case/approved", &ea, &fixture.trust_path()),
            &changed,
            "EA status wrong_record",
        );
    }
}

#[trace("FR-225-AC-2")]
#[test]
#[ignore = "real refusal lane requires QUOIN_E2E_EA and Git SSH verification"]
fn sacm_case_approval_refuses_wrong_bindings_expired_keys_and_non_acceptance_meanings() {
    let ea = verifier();
    let fixture = Fixture::new(Case::Derived);
    let unsigned = fixture.unsigned();
    let baseline = success(&fixture.output("--sacm", &[]));
    let digest = independent_digest(&unsigned);
    fixture.git(&["tag", "-a", "unsigned", "-m", "Unsigned review"]);
    fixture.signed_tag(
        "other-digest",
        KIND,
        &format!("sha256:{}", "0".repeat(64)),
        &[MEANING],
    );
    fixture.signed_tag("other-kind", "other-record", &digest, &[MEANING]);
    fixture.signed_tag("other-meaning", KIND, &digest, &["test-review"]);
    fixture.signed_tag("repeat-meaning", KIND, &digest, &[MEANING, MEANING]);
    fixture.signed_tag("expired", KIND, &digest, &[MEANING]);
    for (tag, expected) in [
        ("absent", "EA status unverifiable"),
        ("unsigned", "EA status unsigned"),
        ("other-digest", "EA status wrong_record"),
        ("other-kind", "EA status wrong_record"),
        ("other-meaning", "exactly one assurance-case-accepted"),
        ("repeat-meaning", "exactly one assurance-case-accepted"),
        ("expired", "EA status untrusted_signer"),
    ] {
        if tag == "expired" {
            fixture.trust("valid-before=\"20000101\"");
        }
        let output = fixture.verified("--sacm", tag, &ea, &fixture.trust_path());
        assert_eq!(success(&output), baseline, "{tag}");
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(
            diagnostic.contains(tag) && diagnostic.contains(expected),
            "{tag}: {diagnostic}"
        );
    }
}

fn executable(path: &Path, script: &str) {
    fs::write(path, script).expect("fixture executable");
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[trace("FR-225-AC-3", "FR-225-AC-5")]
#[test]
fn sacm_case_approval_uses_dedicated_inputs_and_preserves_missing_claimed_time() {
    let fixture = Fixture::new(Case::Authored);
    let unsigned = fixture.unsigned();
    let digest = independent_digest(&unsigned);
    let trust = fixture.trust_path();
    let exe = fixture.home.path().join("fixture-verifier");
    let calls = fixture.home.path().join("called");
    executable(
        &exe,
        &format!(
            "#!/bin/sh\nprintf called > '{}'\ncat >/dev/null\nprintf bad-result\n",
            calls.display()
        ),
    );
    let args = [
        "--case-approval-verifier",
        exe.to_str().unwrap(),
        "--case-approval-allowed-signers",
        &trust,
    ];
    let no_tag = fixture.output("--gsn", &args);
    assert_eq!(
        serde_json::from_str::<Value>(&success(&no_tag)).unwrap(),
        unsigned
    );
    assert!(no_tag.stderr.is_empty());
    assert!(!calls.exists());
    without_approval(
        &fixture.output(
            "--gsn",
            &[
                "--case-approval-tag",
                "selected",
                "--case-approval-verifier",
                exe.to_str().unwrap(),
                "--engineering-assurance",
                exe.to_str().unwrap(),
                "--allowed-signers",
                &trust,
            ],
        ),
        &unsigned,
        "--case-approval-allowed-signers",
    );
    assert!(!calls.exists());
    without_approval(
        &fixture.verified(
            "--gsn",
            "selected",
            &fixture.home.path().join("absent-executable"),
            &trust,
        ),
        &unsigned,
        "engineering-assurance is unavailable",
    );
    without_approval(
        &fixture.verified("--gsn", "selected", &exe, &trust),
        &unsigned,
        "malformed approval result",
    );
    assert!(calls.exists());

    let result = json!({
        "protocol": "engineering-assurance.approval-verify-result/v1", "status": "approved",
        "tag": "selected", "record_id": KIND, "record_digest": digest, "approver": PRINCIPAL,
        "meaning": [MEANING], "signature_kind": "ssh",
        "binding": {"record_id": "match", "record_digest": "match"},
        "trust": {"allowed_signers": trust},
        "signature": {"status": "signed_valid", "kind": "tag", "signature_kind": "ssh",
            "signer": {"source": "allowed_signers", "mapping": "mapped", "principal": PRINCIPAL}},
        "signed_at_missing": "tagger_date_unreadable",
    });
    executable(
        &exe,
        &format!("#!/bin/sh\ncat >/dev/null\ncat <<'APPROVAL'\n{result}\nAPPROVAL\n"),
    );
    let missing_trusted_time = fixture.verified("--gsn", "selected", &exe, &fixture.trust_path());
    assert_eq!(
        serde_json::from_str::<Value>(&success(&missing_trusted_time)).unwrap(),
        unsigned,
        "an EA approval without trusted_at_missing is malformed and must not be accepted"
    );

    let mut result = result;
    result["trusted_at_missing"] = json!("ssh_signature");
    executable(
        &exe,
        &format!("#!/bin/sh\ncat >/dev/null\ncat <<'APPROVAL'\n{result}\nAPPROVAL\n"),
    );
    let output = fixture.verified("--gsn", "selected", &exe, &fixture.trust_path());
    let approved: Value = serde_json::from_str(&success(&output)).unwrap();
    let accepted = &approved["signed_acceptance"];
    assert_eq!(accepted["signed_at_missing"], "tagger_date_unreadable");
    assert_eq!(accepted["trusted_at_missing"], "ssh_signature");
    assert!(accepted.get("signed_at").is_none());
    let xmi = success(&fixture.verified("--sacm", "selected", &exe, &fixture.trust_path()));
    assert_acceptance_artifact(&xmi, accepted, &success(&fixture.output("--sacm", &[])));
}

#[trace("FR-225-AC-5")]
#[test]
#[ignore = "offline verifier lane requires QUOIN_E2E_EA, Git, strace and ssh-keygen"]
fn sacm_case_approval_does_not_start_gitsign_or_fetch_a_missing_promisor_blob() {
    let ea = verifier();
    let fixture = Fixture::new(Case::Derived);
    let unsigned = fixture.unsigned();
    let digest = independent_digest(&unsigned);
    let tip = fixture.git(&["rev-parse", "HEAD"]);
    let tag = format!(
        "object {tip}\ntype commit\ntag cms\ntagger Operator <operator@example.invalid> 1767225784 +0000\n\nReview\n\nApproved-Record-Id: {KIND}\nApproved-Record-Digest: {digest}\nSigned-off-meaning: {MEANING}\n\n-----BEGIN SIGNED MESSAGE-----\nc2lnbmF0dXJl\n-----END SIGNED MESSAGE-----\n"
    );
    let mut hash = Fixture::git_command(&fixture.repo)
        .args(["hash-object", "-t", "tag", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    hash.stdin
        .take()
        .unwrap()
        .write_all(tag.as_bytes())
        .unwrap();
    let object = success(&hash.wait_with_output().unwrap());
    fixture.git(&["update-ref", "refs/tags/cms", object.trim()]);

    let trace_path = fixture.home.path().join("offline.trace");

    let (cms, executed) = trace_case_approval_offline(
        &fixture,
        &ea,
        &trace_path,
        &fixture.repo,
        "cms",
        &fixture.trust_path(),
    );
    without_approval(&cms, &unsigned, "gitsign_identity_missing");
    assert!(executed.contains("approval-verify"), "{executed}");
    assert!(
        !executed
            .lines()
            .any(|line| line.contains("execve(") && line.contains("gitsign\"")),
        "{executed}"
    );

    fs::copy(fixture.allowed(), fixture.repo.join("trust-file")).unwrap();
    fixture.git(&["add", "trust-file"]);
    fixture.git(&["commit", "-qm", "Retain the trust blob"]);
    fixture.git(&["config", "uploadpack.allowFilter", "true"]);
    fixture.git(&["config", "uploadpack.allowAnySHA1InWant", "true"]);
    fixture.signed_tag("promisor", KIND, &digest, &[MEANING]);
    let shallow = fixture.home.path().join("partial");
    let origin = format!("file://{}", fixture.repo.display());
    success(
        &Fixture::git_command(fixture.home.path())
            .args([
                "clone",
                "-q",
                "--no-checkout",
                "--filter=blob:none",
                &origin,
            ])
            .arg(&shallow)
            .output()
            .unwrap(),
    );
    let absent_blob = fixture.git(&["rev-parse", "HEAD:trust-file"]);
    let missing = Fixture::git_command(&shallow)
        .env("GIT_ALLOW_PROTOCOL", "")
        .args(["cat-file", "-e", &absent_blob])
        .output()
        .unwrap();
    assert!(!missing.status.success(), "trust blob really is missing");
    fs::create_dir(shallow.join("spec")).unwrap();
    fs::write(shallow.join("spec/derived.md"), DERIVED).unwrap();
    fs::write(shallow.join("spec/authored.md"), AUTHORED).unwrap();
    fs::write(shallow.join("manifest.yaml"), MODULE).unwrap();
    let expected = serde_json::from_str(&success(
        &fixture.command(&shallow, "--gsn").output().unwrap(),
    ))
    .unwrap();
    let (partial, executed) = trace_case_approval_offline(
        &fixture,
        &ea,
        &trace_path,
        &shallow,
        "promisor",
        "HEAD:trust-file",
    );
    without_approval(&partial, &expected, "EA status unverifiable");
    assert!(executed.contains("approval-verify"), "{executed}");
    assert!(
        !executed.lines().any(|line| line.contains("git-upload-pack")
            || line.contains("git-remote-")
            || line.contains("socket(")
            || line.contains("connect(")),
        "{executed}"
    );
}

fn trace_case_approval_offline(
    fixture: &Fixture,
    verifier: &Path,
    trace_path: &Path,
    repo: &Path,
    tag: &str,
    trust: &str,
) -> (Output, String) {
    let command = fixture.command(repo, "--gsn");
    let output = Command::new("strace")
        .args([
            "-f",
            "-qq",
            "-e",
            "trace=execve,%network",
            "-e",
            "inject=%network:error=ENETDOWN",
            "-o",
        ])
        .arg(trace_path)
        .arg("--")
        .arg(command.get_program())
        .args(command.get_args())
        .current_dir(repo)
        .envs(
            command
                .get_envs()
                .filter_map(|(key, val)| val.map(|val| (key, val))),
        )
        .args([
            "--case-approval-tag",
            tag,
            "--case-approval-verifier",
            verifier.to_str().unwrap(),
            "--case-approval-allowed-signers",
            trust,
        ])
        .output()
        .unwrap();
    (output, fs::read_to_string(trace_path).unwrap())
}
