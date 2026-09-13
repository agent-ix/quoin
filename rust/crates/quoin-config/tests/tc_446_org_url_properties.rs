// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Which remote urls name an organization, over families rather than examples
//! (quoin#446).
//!
//! These carry `tests/props/fr-025.prop.test.ts` and the FR-025 half of
//! `tests/props/second-pass.prop.test.ts`, both `fast-check` properties over
//! `originOrg`. The Rust tree has no property-testing dependency and this
//! cutover is not the place to add one, so each property is restated as an
//! **exhaustive loop over its bounded domain** — every casing of a six-letter
//! word is 64 cases, every nesting depth the original generated is five, and
//! the scp/https × port × `.git` matrix is sixteen. Enumerating the domain is
//! strictly stronger than sampling it where the domain is this small; where it
//! is not (the segment alphabet), a fixed spread of representative segments
//! stands in, which is the one place these are weaker than the originals and is
//! said here rather than left to be discovered.
//!
//! The oracle is `resolve_org_from_documents` with only a git config supplied —
//! no flag, no environment, no config layer — which is exactly what
//! `originOrg` became on the TypeScript side (`src/core/org.ts`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_config::paths::FixedEnvironment;
use quoin_config::{OrgOptions, resolve_org_from_documents};

/// A git config declaring one remote, spelled as git writes it.
fn git_config(url: &str, section: &str) -> String {
    format!(
        "[core]\n\trepositoryformatversion = 0\n[{section}]\n\turl = {url}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n"
    )
}

/// `originOrg(config)`: the git remote as the only source in play.
fn origin_org(config: &str) -> Option<String> {
    let (resolved, report) = resolve_org_from_documents(
        &OrgOptions::default(),
        &FixedEnvironment::new(),
        None,
        None,
        Some(config),
    );
    assert!(
        !report.degraded && report.issues.is_empty(),
        "no config layer was supplied: {report:?}"
    );
    resolved.org.map(|org| org.as_str().to_owned())
}

/// The `segment` arbitrary's alphabet, spread across its bounds: shortest
/// legal, longest legal, digits, and an internal hyphen.
const SEGMENTS: &[&str] = &["a", "agent-ix", "acme", "x9", "a-b-c", "abcdefghijkl"];

/// Trace: FR-025-AC-9
/// Provenance: quoin#446, carried from tests/props/fr-025.prop.test.ts
#[test]
fn tc_446_050_a_remote_that_names_no_organization_yields_none() {
    // The four families `orgFromRemoteUrl` documented as carrying no org. A
    // path segment or a host name substituted here would be an org an author
    // never chose, attached to every spec the repository produces.
    for repo in SEGMENTS {
        for url in [
            format!("/srv/git/{repo}.git"),
            format!("../{repo}"),
            format!("file:///srv/git/{repo}.git"),
            format!("https://git.example.com/{repo}.git"),
        ] {
            assert_eq!(
                origin_org(&git_config(&url, r#"remote "origin""#)),
                None,
                "{url}"
            );
        }
    }
}

/// Trace: FR-025-AC-10
/// Provenance: quoin#446, carried from tests/props/fr-025.prop.test.ts
#[test]
fn tc_446_051_a_nested_namespace_qualifies_by_the_segment_before_the_repository() {
    // Depths 2..=6, the range the original generated, in both url forms.
    for depth in 2..=6_usize {
        let path: Vec<&str> = (0..depth).map(|i| SEGMENTS[i % SEGMENTS.len()]).collect();
        let owner = path[depth - 2];
        let joined = path.join("/");
        for url in [
            format!("https://git.example.com/{joined}.git"),
            format!("git@git.example.com:{joined}.git"),
        ] {
            assert_eq!(
                origin_org(&git_config(&url, r#"remote "origin""#)).as_deref(),
                Some(owner),
                "{url}"
            );
        }
    }
}

/// Trace: FR-025-AC-2, FR-025-AC-3
/// Provenance: quoin#446, carried from tests/props/second-pass.prop.test.ts
#[test]
fn tc_446_052_both_url_forms_yield_the_owner_across_their_whole_matrix() {
    for owner in SEGMENTS {
        for repo in SEGMENTS {
            for dot_git in [true, false] {
                let suffix = if dot_git { ".git" } else { "" };

                // AC-2: the scp-style form, for any host.
                for host in SEGMENTS {
                    let url = format!("git@{host}.example.com:{owner}/{repo}{suffix}");
                    assert_eq!(
                        origin_org(&git_config(&url, r#"remote "origin""#)).as_deref(),
                        Some(*owner),
                        "{url}"
                    );
                }

                // AC-3: the https form, with and without a port. The port is a
                // family of one representative rather than all 65535, because
                // the rule is "a port is not a path segment" and one port that
                // is not confused for one proves it.
                for port in ["", ":8443"] {
                    let url = format!("https://git.example.com{port}/{owner}/{repo}{suffix}");
                    assert_eq!(
                        origin_org(&git_config(&url, r#"remote "origin""#)).as_deref(),
                        Some(*owner),
                        "{url}"
                    );
                }
            }
        }
    }
}

/// Every casing of `word`, all `2^len` of them.
fn casings(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    let len = u32::try_from(chars.len()).expect("short word");
    (0..1_u32 << len)
        .map(|mask| {
            chars
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    if mask >> i & 1 == 1 {
                        c.to_ascii_uppercase()
                    } else {
                        c.to_ascii_lowercase()
                    }
                })
                .collect()
        })
        .collect()
}

/// Trace: FR-025-AC-11
/// Provenance: quoin#446, carried from tests/props/second-pass.prop.test.ts
#[test]
fn tc_446_053_the_section_name_matches_case_insensitively_and_the_remote_name_does_not() {
    // The asymmetry is git's own: a section name is case-insensitive, the
    // quoted subsection name is not. All 64 casings of each word, not a sample.
    let sections = casings("remote");
    assert_eq!(sections.len(), 64, "the casing enumeration is incomplete");
    for section in &sections {
        let config = git_config(
            "https://git.example.com/agent-ix/repo.git",
            &format!("{section} \"origin\""),
        );
        assert_eq!(
            origin_org(&config).as_deref(),
            Some("agent-ix"),
            "section {section}"
        );
    }

    for name in casings("origin") {
        if name == "origin" {
            continue;
        }
        let config = git_config(
            "https://git.example.com/agent-ix/repo.git",
            &format!("remote \"{name}\""),
        );
        assert_eq!(origin_org(&config), None, "remote {name}");
    }
}
