// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The committed golden corpus, as `golden_parity.rs` and
//! `tc_445_snapshot_equivalence.rs` both read it: load a golden, project its
//! fields, and rebuild the file tree a case describes.

use std::path::Path;

use serde_json::Value;

pub(crate) fn golden(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(name);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("golden {name} must be readable: {error}"),
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => panic!("golden {name} must be JSON: {error}"),
    }
}

pub(crate) fn array<'a>(value: &'a Value, key: &str) -> &'a Vec<Value> {
    match value.get(key).and_then(Value::as_array) {
        Some(items) => items,
        None => panic!("golden has no array at {key}"),
    }
}

pub(crate) fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// Rebuild a file tree from a golden's `{ path: contents }` map.
pub(crate) fn materialize(root: &Path, files: &Value) {
    let Some(map) = files.as_object() else {
        return;
    };
    for (relative, body) in map {
        let path = root.join(relative);
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            panic!("mkdir {}: {error}", parent.display());
        }
        if let Err(error) = std::fs::write(&path, body.as_str().unwrap_or_default()) {
            panic!("write {}: {error}", path.display());
        }
    }
}
