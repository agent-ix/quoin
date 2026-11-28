// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for retained `quoin completeness`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use ix_trace_rs::implements;
use quoin_core::{ops::quire::DocumentSnapshotPayload, protocol::Response};

use crate::core_runtime::invoke;

pub(crate) fn command() -> Command {
    Command::new("completeness")
        .about("Report declared vocabulary values that no requirement owns")
        .arg(Arg::new("repo").long("repo").default_value("."))
        .arg(Arg::new("bundle").long("bundle"))
        .arg(Arg::new("module").long("module"))
        .arg(Arg::new("strict").long("strict").action(ArgAction::SetTrue))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

#[implements("NFR-011-M-2")]
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let bundle = arguments
        .get_one::<String>("bundle")
        .cloned()
        .unwrap_or_else(|| format!("{repo}/spec"));
    let snapshot = invoke(
        "quire.document_snapshot",
        &serde_json::json!({ "scope": repo }),
    )?;
    if !snapshot.outcome.carries_payload() {
        return Ok(snapshot);
    }
    let snapshot: DocumentSnapshotPayload = serde_json::from_value(snapshot.payload)
        .map_err(|error| format!("quire.document_snapshot returned an invalid payload: {error}"))?;
    let (documents, unreadable) =
        bundle_snapshot_with_selected(Path::new(&bundle), &snapshot.documents)?;
    let roots = arguments
        .get_one::<String>("module")
        .map_or_else(default_roots, |root| vec![root.clone()]);
    let modules = module_snapshot(&roots)?;
    let response = invoke(
        "completeness.assess_bundle",
        &serde_json::json!({ "bundle_root": bundle, "strict": arguments.get_flag("strict"), "documents": documents, "unreadable": unreadable, "modules": modules }),
    )?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let (rendered, warning) = if arguments.get_flag("json") {
        (
            serde_json::to_string_pretty(&response.payload).map_err(|error| error.to_string())?,
            None,
        )
    } else {
        render(&response.payload)
    };
    let outcome = if response
        .payload
        .get("verdict")
        .and_then(serde_json::Value::as_str)
        == Some("FAIL")
        || (arguments.get_flag("strict")
            && response
                .payload
                .get("verdict")
                .and_then(serde_json::Value::as_str)
                == Some("UNCHECKED"))
    {
        Outcome::Partial
    } else {
        response.outcome
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered, "warning": warning }),
        diagnostics: response.diagnostics,
        outcome,
    })
}

/// Read bundle frontmatter through the sole parser shared with assurance.
pub(crate) fn read_frontmatter(root: &Path) -> Result<Response, String> {
    let (documents, unreadable) = bundle_snapshot(root)?;
    invoke(
        "completeness.read_frontmatter",
        &serde_json::json!({ "documents": documents, "unreadable": unreadable }),
    )
}

fn bundle_snapshot(
    root: &Path,
) -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>), String> {
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    bundle_sources(root, files, &[], |path| std::fs::read_to_string(path))
}

#[implements("NFR-011-M-2")]
fn bundle_snapshot_with_selected(
    root: &Path,
    selected: &[quoin_core::ops::quire::DocumentSnapshotEntry],
) -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>), String> {
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    bundle_sources(root, files, selected, |path| std::fs::read_to_string(path))
}

#[implements("NFR-011-M-2")]
fn bundle_sources(
    root: &Path,
    files: Vec<PathBuf>,
    selected: &[quoin_core::ops::quire::DocumentSnapshotEntry],
    mut read: impl FnMut(&Path) -> std::io::Result<String>,
) -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>), String> {
    let selected = index_snapshot(selected)?;
    let mut documents = Vec::new();
    let mut unreadable = Vec::new();
    for path in files {
        let relative = path.strip_prefix(root).ok().map_or_else(
            || path.display().to_string(),
            |path| path.to_string_lossy().replace('\\', "/"),
        );
        let identity = path.canonicalize().unwrap_or_else(|_| path.clone());
        let contents = selected.get(&identity).map_or_else(
            || read(&path).map_err(|error| error.to_string()),
            |entry| match (&entry.content, &entry.error) {
                (Some(content), None) => Ok(content.clone()),
                (None, Some(error)) => Err(error.clone()),
                _ => Err("document snapshot entry must contain content or an error".to_owned()),
            },
        );
        match contents {
            Ok(raw) => documents.push(serde_json::json!({"path":relative,"raw":raw})),
            Err(reason) => {
                unreadable.push(serde_json::json!({"path":relative,"reason":reason}));
            }
        }
    }
    Ok((documents, unreadable))
}

#[implements("NFR-011-M-2")]
fn index_snapshot(
    selected: &[quoin_core::ops::quire::DocumentSnapshotEntry],
) -> Result<HashMap<PathBuf, &quoin_core::ops::quire::DocumentSnapshotEntry>, String> {
    let mut indexed = HashMap::with_capacity(selected.len());
    for entry in selected {
        if entry.content.is_some() == entry.error.is_some() {
            return Err(format!(
                "quire.document_snapshot entry `{}` must have exactly one read result",
                entry.path
            ));
        }
        let path = PathBuf::from(&entry.path);
        if indexed.insert(path.clone(), entry).is_some() {
            return Err(format!(
                "quire.document_snapshot returned duplicate selected path `{}`",
                path.display()
            ));
        }
    }
    Ok(indexed)
}

fn walk(path: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            walk(&entry_path, files);
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "md")
            && (kind.is_file() || entry_path.is_file())
        {
            files.push(entry_path);
        }
    }
}

fn module_snapshot(roots: &[String]) -> Result<Vec<serde_json::Value>, String> {
    let mut modules = Vec::new();
    let mut seen = BTreeSet::new();
    for root in roots {
        let root = locate(Path::new(root));
        let Some(root) = root else { continue };
        if !seen.insert(root.clone()) {
            continue;
        }
        let Ok(manifest) = std::fs::read_to_string(root.join("manifest.yaml")) else {
            continue;
        };
        let refs = invoke(
            "completeness.schema_refs",
            &serde_json::json!({"manifest":manifest}),
        )?;
        if !refs.outcome.carries_payload() {
            continue;
        }
        let mut schemas = BTreeMap::new();
        for reference in refs
            .payload
            .get("refs")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            let source = root.join(reference);
            let value = std::fs::read_to_string(&source).map_or_else(
                |error| serde_json::json!({"unreadable":error.to_string()}),
                |text| serde_json::json!({"text":text}),
            );
            schemas.insert(reference.to_owned(), value);
        }
        modules.push(serde_json::json!({"label":root,"manifest":manifest,"schemas":schemas}));
    }
    Ok(modules)
}

fn locate(candidate: &Path) -> Option<PathBuf> {
    if candidate.join("manifest.yaml").is_file() {
        return Some(candidate.to_path_buf());
    }
    let entries = std::fs::read_dir(candidate).ok()?;
    entries
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.join("manifest.yaml").is_file())
}
fn default_roots() -> Vec<String> {
    let mut roots = std::env::var("QUOIN_MODULE_PATHS")
        .ok()
        .map_or_else(Vec::new, |paths| {
            paths
                .split(':')
                .filter(|path| !path.is_empty())
                .map(str::to_owned)
                .collect()
        });
    let home = std::env::var_os("IX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".ix")));
    if let Some(home) = home
        && let Ok(entries) = std::fs::read_dir(home.join("filament/modules"))
    {
        roots.extend(
            entries
                .flatten()
                .map(|entry| entry.path().display().to_string()),
        );
    }
    roots
}
fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}
fn render(payload: &serde_json::Value) -> (String, Option<String>) {
    let mut lines = Vec::new();
    let mut warnings = Vec::new();
    if payload
        .get("vocabularies")
        .and_then(serde_json::Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        warnings.push(
            "no active module declares `traceability.vocabulary_coverage`, so nothing was checked. \
             Install one (e.g. spec-artifacts-iso) or pass --module."
                .replace("\n             ", ""),
        );
    }
    for unresolved in array(payload, "unresolved") {
        let name = string(unresolved, "name");
        let reason = string(unresolved, "reason");
        warnings.push(format!("vocabulary '{name}' not resolved: {reason}"));
    }
    for unreadable in array(payload, "unreadable") {
        let path = string(unreadable, "path");
        let reason = string(unreadable, "reason");
        warnings.push(format!("frontmatter unreadable in {path}: {reason}"));
    }
    for rollup in array(payload, "rollups") {
        lines.push(format!(
            "{}: {}/{} owned, {} excused, {} unowned",
            string(rollup, "vocabulary"),
            number(rollup, "owned"),
            number(rollup, "declared"),
            number(rollup, "excused"),
            number(rollup, "unowned"),
        ));
    }
    let findings = array(payload, "findings");
    for finding in findings {
        let document = finding
            .get("document")
            .and_then(serde_json::Value::as_str)
            .map_or_else(String::new, |document| format!(" {document}:"));
        lines.push(format!(
            "  [{}]{} {}",
            string(finding, "severity"),
            document,
            string(finding, "message"),
        ));
    }
    let high = findings
        .iter()
        .filter(|finding| string(finding, "severity") == "high")
        .count();
    let medium = findings
        .iter()
        .filter(|finding| string(finding, "severity") == "medium")
        .count();
    lines.push(format!(
        "\n{} — {high} high, {medium} medium (bundle {})",
        string(payload, "verdict"),
        string(payload, "bundleRoot"),
    ));
    (
        lines.join("\n"),
        (!warnings.is_empty()).then(|| warnings.join("\n")),
    )
}

fn array<'a>(payload: &'a serde_json::Value, key: &str) -> &'a [serde_json::Value] {
    payload
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn string<'a>(payload: &'a serde_json::Value, key: &str) -> &'a str {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}

fn number(payload: &serde_json::Value, key: &str) -> u64 {
    payload
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test fixtures use expect to report malformed local test data"
)]
mod tests {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use super::{bundle_sources, command, index_snapshot, render};
    use ix_trace_rs::trace;
    use quoin_core::ops::quire::DocumentSnapshotEntry;
    use tempfile::tempdir;

    #[trace("FR-037", "FR-062")]
    #[test]
    fn tc_445_742_renders_the_complete_assessment_and_warns_separately() {
        let payload = serde_json::json!({
            "bundleRoot": "spec",
            "vocabularies": [],
            "unresolved": [{"name": "quality", "reason": "schema absent"}],
            "unreadable": [{"path": "FR-001.md", "reason": "invalid frontmatter"}],
            "rollups": [{"vocabulary": "security", "owned": 1, "declared": 3, "excused": 1, "unowned": 1}],
            "findings": [
                {"severity": "high", "document": "FR-001.md", "message": "missing reason"},
                {"severity": "medium", "message": "unowned reliability"}
            ],
            "verdict": "FAIL"
        });

        let (rendered, warning) = render(&payload);

        assert_eq!(
            rendered,
            "security: 1/3 owned, 1 excused, 1 unowned\n  [high] FR-001.md: missing reason\n  [medium] unowned reliability\n\nFAIL — 1 high, 1 medium (bundle spec)"
        );
        assert_eq!(
            warning.as_deref(),
            Some(
                "no active module declares `traceability.vocabulary_coverage`, so nothing was checked. Install one (e.g. spec-artifacts-iso) or pass --module.\nvocabulary 'quality' not resolved: schema absent\nfrontmatter unreadable in FR-001.md: invalid frontmatter"
            )
        );
    }

    #[trace("FR-037", "FR-062")]
    #[test]
    fn tc_445_743_preserves_the_retained_completeness_grammar() {
        let matches = command()
            .try_get_matches_from([
                "completeness",
                "--repo",
                "other",
                "--bundle",
                "other/spec",
                "--module",
                "module",
                "--strict",
                "--json",
            ])
            .expect("the retained completeness grammar parses");

        assert_eq!(
            matches.get_one::<String>("repo").map(String::as_str),
            Some("other")
        );
        assert_eq!(
            matches.get_one::<String>("bundle").map(String::as_str),
            Some("other/spec")
        );
        assert_eq!(
            matches.get_one::<String>("module").map(String::as_str),
            Some("module")
        );
        assert!(matches.get_flag("strict"));
        assert!(matches.get_flag("json"));
    }

    #[trace("NFR-011-M-2")]
    #[test]
    fn tc_nfr011_document_snapshot_is_reused_without_reopening_selected_documents() {
        let root = tempdir().expect("temporary bundle root");
        let selected_path = root.path().join("selected.md");
        let bundle_only_path = root.path().join("bundle-only.md");
        let unreadable_path = root.path().join("unreadable.md");
        std::fs::write(&selected_path, "filesystem value must not be read")
            .expect("selected file exists for canonical path lookup");
        std::fs::write(&bundle_only_path, "bundle-only placeholder")
            .expect("bundle-only file exists");
        std::fs::write(&unreadable_path, "unreadable placeholder")
            .expect("unreadable file exists for canonical path lookup");
        let selected_path = selected_path
            .canonicalize()
            .expect("selected canonical path");
        let unreadable_path = unreadable_path
            .canonicalize()
            .expect("unreadable canonical path");
        let snapshot = vec![
            DocumentSnapshotEntry {
                path: selected_path.display().to_string(),
                content: Some("snapshot bytes".to_owned()),
                error: None,
            },
            DocumentSnapshotEntry {
                path: unreadable_path.display().to_string(),
                content: None,
                error: Some("permission denied".to_owned()),
            },
        ];
        let selected_paths = snapshot
            .iter()
            .map(|entry| PathBuf::from(&entry.path))
            .collect::<Vec<_>>();
        let mut expected_selected = vec![selected_path.clone(), unreadable_path.clone()];
        expected_selected.sort();
        assert_eq!(selected_paths, expected_selected);
        assert!(
            snapshot
                .iter()
                .all(|entry| entry.content.is_some() ^ entry.error.is_some())
        );
        let mut files = vec![
            selected_path.clone(),
            bundle_only_path
                .canonicalize()
                .expect("bundle canonical path"),
            unreadable_path,
        ];
        files.sort();
        let reads = RefCell::new(Vec::new());

        let (documents, unreadable) = bundle_sources(root.path(), files, &snapshot, |path| {
            reads.borrow_mut().push(path.to_path_buf());
            Ok("bundle-only bytes".to_owned())
        })
        .expect("valid snapshot entries");

        assert_eq!(
            reads.into_inner(),
            vec![bundle_only_path],
            "selected and selected-unreadable paths must not be reopened"
        );
        assert_eq!(
            documents,
            vec![
                serde_json::json!({"path":"bundle-only.md","raw":"bundle-only bytes"}),
                serde_json::json!({"path":"selected.md","raw":"snapshot bytes"}),
            ]
        );
        assert_eq!(
            unreadable,
            vec![serde_json::json!({"path":"unreadable.md","reason":"permission denied"})]
        );
        assert!(
            index_snapshot(&[snapshot[0].clone(), snapshot[0].clone()]).is_err(),
            "duplicate selected paths must be rejected"
        );
    }
}
