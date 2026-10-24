// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The store adapts its value model; the shared owner encodes identities.

use ix_trace_rs::trace;

#[test]
fn canonical_encoding_has_no_retained_local_implementations() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/json");
    for retired in ["jcs.rs", "order.rs", "number.rs"] {
        assert!(
            !source.join(retired).exists(),
            "retired local canonical implementation remains: {retired}"
        );
    }
}

#[trace("FR-100-AC-10")]
#[test]
fn canonical_digests_match_shared_owner_and_literal_vectors()
-> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (
            r#"{"\ue000":1,"\ud800\udc00":2}"#,
            "{\"𐀀\":2,\"\u{e000}\":1}",
        ),
        (r#"{"2":2,"10":10}"#, r#"{"10":10,"2":2}"#),
        (r#"{"z":-0.0,"n":1e21}"#, r#"{"n":1e+21,"z":0}"#),
    ];
    for (input, expected) in cases {
        let store = quoin_store::json::parse::parse_strict_json_str(input)?;
        let boundary: serde_json::Value = serde_json::from_str(input)?;
        let owner = quire_canonical::to_vec(&boundary, quire_canonical::Limits::new(4096))?;
        assert_eq!(owner, expected.as_bytes());
        assert_eq!(quoin_store::canonical_bytes(&store)?, owner);
        let expected_value = quoin_store::json::parse::parse_strict_json_str(expected)?;
        assert_eq!(
            quoin_store::digest_canonical_value(&store)?,
            quoin_store::digest_canonical_value(&expected_value)?
        );
    }
    Ok(())
}
