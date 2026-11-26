// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Real Mermaid parser integration, run explicitly by `make test-real`.

#[test]
#[ignore = "requires real mmdr; run with make test-real"]
fn exported_gsn_mermaid_is_accepted_by_mmdr() -> Result<(), Box<dyn std::error::Error>> {
    use quoin_assurance::{GsnCase, GsnNode, render_gsn_diagram};

    let mmdr = std::env::var("QUOIN_TEST_MMDR")?;
    let dir = std::env::temp_dir().join(format!("quoin-gsn-mmdr-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let input = dir.join("case.mmd");
    let output = dir.join("case.png");
    let diagram = render_gsn_diagram(&GsnCase {
        format: "quoin.gsn-case/v1".to_owned(),
        nodes: vec![GsnNode {
            id: "G1".to_owned(),
            kind: "goal".to_owned(),
            source: "StR-1".to_owned(),
            statement: "claim".to_owned(),
            status: "open".to_owned(),
            open_reason: None,
            undeveloped: Some(true),
            evidence: None,
            source_data: None,
        }],
        edges: vec![],
        roots: vec!["G1".to_owned()],
        unreachable: vec![],
        unmapped: vec![],
        signed_acceptance: None,
    })?;
    std::fs::write(&input, diagram)?;
    let result = std::process::Command::new(mmdr)
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-e")
        .arg("png")
        .output()?;
    assert!(
        result.status.success(),
        "mmdr failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(std::fs::metadata(&output).is_ok_and(|metadata| metadata.len() > 0));
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}
