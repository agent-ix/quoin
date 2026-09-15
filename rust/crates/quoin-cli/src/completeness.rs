// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for retained `quoin completeness`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::{Outcome, Response};

use crate::core_bridge::invoke;

pub(crate) fn command() -> Command {
    Command::new("completeness")
        .about("Report declared vocabulary values that no requirement owns")
        .arg(Arg::new("repo").long("repo").default_value("."))
        .arg(Arg::new("bundle").long("bundle"))
        .arg(Arg::new("module").long("module"))
        .arg(Arg::new("strict").long("strict").action(ArgAction::SetTrue))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let bundle = arguments
        .get_one::<String>("bundle")
        .cloned()
        .unwrap_or_else(|| format!("{repo}/spec"));
    let (documents, unreadable) = bundle_snapshot(Path::new(&bundle));
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
    let (documents, unreadable) = bundle_snapshot(root);
    invoke(
        "completeness.read_frontmatter",
        &serde_json::json!({ "documents": documents, "unreadable": unreadable }),
    )
}

fn bundle_snapshot(root: &Path) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let mut documents = Vec::new();
    let mut unreadable = Vec::new();
    for path in files {
        let relative = path.strip_prefix(root).ok().map_or_else(
            || path.display().to_string(),
            |path| path.to_string_lossy().replace('\\', "/"),
        );
        match std::fs::read_to_string(&path) {
            Ok(raw) => documents.push(serde_json::json!({"path":relative,"raw":raw})),
            Err(error) => {
                unreadable.push(serde_json::json!({"path":relative,"reason":error.to_string()}));
            }
        }
    }
    (documents, unreadable)
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
    use super::{command, render};

    /// Trace: FR-037, FR-062
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

    /// Trace: FR-037, FR-062
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
}
