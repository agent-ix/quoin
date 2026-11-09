// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Read-only workflow receipt and document inventory; no retrospective approval.

use crate::Result;
use serde_json::{Value, json};
use std::fs;
use std::io::Read;
use std::path::Path;

fn files(root: &Path, result: &mut Vec<std::path::PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            files(&entry.path(), result)?;
        } else if kind.is_file() {
            result.push(entry.path());
        }
        if result.len() > 100_000 {
            return Err("inventory exceeds 100000 files".to_owned());
        }
    }
    Ok(())
}

pub(super) fn run(state: &str, repos: &[String]) -> Result<()> {
    let mut paths = Vec::new();
    files(Path::new(state), &mut paths)?;
    paths.sort();
    let mut runs = Vec::new();
    let mut documents = Vec::new();
    let mut unreadable = Vec::new();
    for path in paths {
        if path.extension().is_none_or(|s| s != "json") {
            continue;
        }
        let parsed = read(&path)
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).map_err(|e| e.to_string()));
        match parsed {
            Ok(run)
                if matches!(
                    run.get("defName").and_then(Value::as_str),
                    Some("review" | "to-plan")
                ) =>
            {
                let created = run.pointer("/events/0/ts").and_then(Value::as_str);
                if created.is_some_and(|s| s < "2026-09-21") {
                    continue;
                }
                runs.push(json!({"id":run.get("id"),"workflow":run.get("defName"),"phase":run.get("phase"),"created_at":created,"classification":"enforcement-unverified-inventory-only","recorded_reviews":run.pointer("/items/review_doc"),"human_acknowledgements":run.get("satisfiedGates").and_then(Value::as_array).map(Vec::len)}));
            }
            Ok(_) => {}
            Err(error) => unreadable.push(json!({"path":path,"reason":error})),
        }
    }
    for repo in repos {
        for dir in ["spec/reviews", "reviews"] {
            let mut paths = Vec::new();
            files(&Path::new(repo).join(dir), &mut paths)?;
            paths.sort();
            for path in paths {
                if path.extension().is_none_or(|s| s != "md") {
                    continue;
                }
                let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
                if metadata.len() > 4 * 1024 * 1024 {
                    unreadable.push(json!({"path":path,"reason":"over 4 MiB inventory limit"}));
                    continue;
                }
                let source = String::from_utf8(read(&path)?).map_err(|e| e.to_string())?;
                let frontmatter = source
                    .strip_prefix("---\n")
                    .and_then(|s| s.split_once("\n---"))
                    .map(|(s, _)| s);
                if frontmatter
                    .is_some_and(|s| s.lines().any(|line| line.trim() == "type: SpecReview"))
                {
                    documents.push(json!({"path":path,"classification":"document-present-validation-and-acceptance-not-established"}));
                }
            }
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"since":"2026-09-21","runs":runs,"documents":documents,"unreadable":unreadable,"limitations":"Read-only inventory, not validation or retrospective acceptance. Date filter applies to receipts; document inventory is undated and is not a count of affected reviews. Missing receipts do not imply invalid analyses."})).map_err(|e|e.to_string())?);
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("over 4 MiB inventory limit".to_owned());
    }
    Ok(bytes)
}
