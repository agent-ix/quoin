// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Real Mermaid parser integration, run explicitly by `make test-real`.

#[test]
#[ignore = "requires real mmdr; run with make test-real"]
fn exported_gsn_mermaid_is_accepted_by_mmdr() {
    let mmdr =
        std::env::var("QUOIN_TEST_MMDR").expect("make test-real requires QUOIN_TEST_MMDR=<path>");
    let dir = std::env::temp_dir().join(format!("quoin-gsn-mmdr-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("case.mmd");
    let output = dir.join("case.png");
    std::fs::write(
        &input,
        "flowchart TB\n  G1[\"claim\"]\n  S1[/\"strategy\"/]\n  G1 --> S1\n",
    )
    .expect("write Mermaid fixture");
    let result = std::process::Command::new(mmdr)
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-e")
        .arg("png")
        .output()
        .expect("run mmdr");
    assert!(
        result.status.success(),
        "mmdr failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(std::fs::metadata(&output).is_ok_and(|metadata| metadata.len() > 0));
    let _ = std::fs::remove_dir_all(&dir);
}
