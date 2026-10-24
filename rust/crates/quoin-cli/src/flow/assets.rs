// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Materialize first-party workflow assets embedded in the published executable.
//! Existing snapshots are immutable so active ix-flow runs survive upgrades.

use std::fs;
use std::path::{Path, PathBuf};

use super::provider::LENSES;

pub(super) fn materialize(name: &str, home: &Path, executable: &Path) -> Result<PathBuf, String> {
    let (skill, definition) = match name {
        "review" => (
            include_str!("../../../../../workflows/review/SKILL.md"),
            include_str!("../../../../../workflows/review/workflows/review/def.yaml"),
        ),
        "to-plan" => (
            include_str!("../../../../../workflows/to-plan/SKILL.md"),
            include_str!("../../../../../workflows/to-plan/workflows/to-plan/def.yaml"),
        ),
        _ => return Err("an unknown flow command reached dispatch".to_owned()),
    };
    let executable = executable
        .to_str()
        .ok_or("workflow provider executable path is not UTF-8")?;
    let skill = skill
        .replace(
            "__QUOIN_EXECUTABLE__",
            &serde_json::to_string(executable).map_err(|e| e.to_string())?,
        )
        .replace("__QUOIN_LENSES__", &LENSES.join(", "));
    let version = crate::build_version();
    if !version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err("unsafe workflow runtime version".to_owned());
    }
    let root = home.join("workflows/quoin").join(version);
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let definition_path = PathBuf::from("workflows").join(name).join("def.yaml");
    let mut existing = fs::read_dir(&root)
        .map_err(|e| e.to_string())?
        .take(1025)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if existing.len() > 1024 {
        return Err("too many workflow snapshots for one release; retain active-run snapshots and remove obsolete snapshots explicitly".to_owned());
    }
    existing.sort_by_key(std::fs::DirEntry::file_name);
    for entry in existing {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(&format!("{name}-"))
            && entry.file_type().map_err(|e| e.to_string())?.is_dir()
            && fs::read(entry.path().join("SKILL.md")).ok().as_deref() == Some(skill.as_bytes())
            && fs::read(entry.path().join(&definition_path))
                .ok()
                .as_deref()
                == Some(definition.as_bytes())
        {
            return Ok(entry.path());
        }
    }
    let stage = tempfile::Builder::new()
        .prefix(".staging-")
        .tempdir_in(&root)
        .map_err(|e| e.to_string())?;
    let target = root.join(format!(
        "{name}-{}",
        stage
            .path()
            .file_name()
            .ok_or("missing staging name")?
            .to_string_lossy()
    ));
    fs::create_dir_all(stage.path().join("workflows").join(name)).map_err(|e| e.to_string())?;
    fs::write(stage.path().join("SKILL.md"), skill).map_err(|e| e.to_string())?;
    fs::write(stage.path().join(definition_path), definition).map_err(|e| e.to_string())?;
    fs::rename(stage.path(), &target).map_err(|e| e.to_string())?;
    Ok(target)
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions may panic")]
mod tests {
    use super::materialize;

    /// Trace: FR-020-AC-4
    #[test]
    fn snapshots_reuse_identical_bytes_and_preserve_old_or_incomplete_runs() {
        let root = tempfile::tempdir().expect("home");
        let executable = std::env::current_exe().expect("executable");
        let first = materialize("review", root.path(), &executable).expect("first");
        assert_eq!(
            first,
            materialize("review", root.path(), &executable).expect("reuse")
        );
        let definition = first.join("workflows/review/def.yaml");
        std::fs::write(&definition, "damaged definition").expect("simulate incomplete snapshot");
        let repaired = materialize("review", root.path(), &executable).expect("repair");
        assert_ne!(first, repaired);
        assert_eq!(
            std::fs::read_to_string(definition).expect("preserved old run"),
            "damaged definition"
        );
        assert!(
            std::fs::read_to_string(repaired.join("workflows/review/def.yaml"))
                .expect("owned definition")
                .contains("selected_analyses_covered")
        );
    }

    /// Trace: FR-020-AC-2
    #[test]
    fn unknown_workflows_are_refused_before_materialization() {
        let root = tempfile::tempdir().expect("home");
        assert!(materialize("matrix", root.path(), root.path()).is_err());
        assert_eq!(std::fs::read_dir(root.path()).expect("read").count(), 0);
    }
}
