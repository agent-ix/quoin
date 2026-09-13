// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! An unprefixed install argument is a path, over the family rather than an
//! example (quoin#446).
//!
//! Carries `tests/props/fr-018.prop.test.ts` and the FR-018-AC-1..AC-4 half of
//! `tests/props/second-pass.prop.test.ts`, both `fast-check` properties over
//! `parseSourceArg`. The Rust tree has no property-testing dependency and this
//! cutover is not the place to add one, so the property is restated as an
//! exhaustive loop over a bounded domain: every two-letter scheme over the
//! lowercase alphabet — 676 of them, the `^[a-z]{2,10}$` generator's whole
//! shortest rank, minus the three known prefixes — plus a spread of argument
//! shapes that exercise the characters the original's `[a-z./-]` alphabet
//! allowed.
//!
//! What is deliberately NOT weakened: the rule under test is that an argument
//! with no known prefix becomes a path source carrying the argument
//! **verbatim**, colon and all. A test that stripped an unknown scheme would
//! turn `c:/specs` on a Windows path into a source named `/specs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_modules::{Source, parse_source_arg};

fn path_of(arg: &str) -> String {
    match parse_source_arg(arg).unwrap_or_else(|e| panic!("{arg:?}: {e}")) {
        Source::Path { path } => path,
        other => panic!("{arg:?} parsed as {other:?}, not a path source"),
    }
}

/// Trace: FR-018-AC-5
/// Provenance: quoin#446, carried from tests/props/fr-018.prop.test.ts
#[test]
fn tc_446_060_a_bare_argument_is_a_path_source_carrying_it_verbatim() {
    // The shapes the original's generator produced, plus the ones that would
    // break a naive "strip everything before a colon": a bare name, a relative
    // path, an absolute path, a dotted path and a trailing separator.
    for arg in [
        "m",
        "modules/spec-objects",
        "./local-module",
        "../sibling/module",
        "/srv/modules/spec-objects",
        ".hidden",
        "a-b-c.d/e-f",
        "with space/module",
        "trailing/",
    ] {
        assert_eq!(path_of(arg), arg, "{arg}");
    }
}

/// Trace: FR-018-AC-5
/// Provenance: quoin#446, carried from tests/props/fr-018.prop.test.ts
#[test]
fn tc_446_061_an_unknown_prefix_is_kept_rather_than_stripped() {
    const KNOWN: [&str; 3] = ["path", "github", "package"];
    let mut checked = 0_usize;
    for first in b'a'..=b'z' {
        for second in b'a'..=b'z' {
            let scheme = String::from_utf8(vec![first, second]).expect("ascii");
            if KNOWN.contains(&scheme.as_str()) {
                continue;
            }
            for rest in ["x", "a/b", "./rel", "/abs/path", "a-b.c"] {
                let arg = format!("{scheme}:{rest}");
                assert_eq!(path_of(&arg), arg, "{arg}");
                checked += 1;
            }
        }
    }
    // Not a check over an empty population: 676 two-letter schemes, none of
    // which is one of the three known prefixes, times five argument shapes.
    assert_eq!(checked, 26 * 26 * 5, "the enumeration did not run in full");
}

/// The `segment` arbitrary's alphabet, spread across its bounds.
const SEGMENTS: &[&str] = &["a", "spec-objects", "v1", "a-b-c", "abcdefghijkl"];

/// The optional-ref arbitrary: absent, then every segment.
fn refs() -> Vec<Option<&'static str>> {
    let mut all = vec![None];
    all.extend(SEGMENTS.iter().map(|s| Some(*s)));
    all
}

/// Trace: FR-018-AC-1
/// Provenance: quoin#446, carried from tests/props/second-pass.prop.test.ts
#[test]
fn tc_446_062_a_path_prefix_carries_the_remainder_verbatim() {
    // The original generated arbitrary strings up to 40 characters. The rule is
    // that the remainder is carried **verbatim**, so the cases that matter are
    // the ones another branch of the parser could have claimed: a second colon,
    // a `//` that means git-subdir under `github:`, a final `@` that means a
    // version under `package:`, and the empty remainder.
    for rest in [
        "modules/spec-objects",
        "./rel",
        "/abs/path",
        "github:owner/repo",
        "owner/repo//sub",
        "@scope/name@1.2.3",
        "a b\tc",
        "ünïcøde/módule",
        "trailing/",
    ] {
        assert_eq!(path_of(&format!("path:{rest}")), rest, "path:{rest}");
    }

    // ONE case in the original's domain that this port deliberately does not
    // match, stated here rather than quietly dropped from the loop above.
    // `src/plugins.ts` returned `{type:"path", path:""}` for a bare `path:`,
    // because it sliced the prefix off and asked nothing further; `quoin-modules`
    // refuses an empty path at parse time (QM001). The old behaviour was not a
    // feature — an empty path resolved to the process's working directory and
    // installed whatever happened to be there — so this is a refusal where there
    // used to be a confusing success, and the PR says so.
    let refusal = parse_source_arg("path:").expect_err("an empty path is refused");
    assert!(
        refusal.to_string().contains("QM001_INVALID_SOURCE"),
        "{refusal}"
    );
}

/// Trace: FR-018-AC-2
/// Provenance: quoin#446, carried from tests/props/second-pass.prop.test.ts
#[test]
fn tc_446_063_a_github_prefix_yields_owner_repo_and_a_ref_only_when_given() {
    let mut checked = 0_usize;
    for owner in SEGMENTS {
        for repo in SEGMENTS {
            for r in refs() {
                let arg = match r {
                    Some(r) => format!("github:{owner}/{repo}@{r}"),
                    None => format!("github:{owner}/{repo}"),
                };
                match parse_source_arg(&arg).unwrap_or_else(|e| panic!("{arg:?}: {e}")) {
                    Source::Github {
                        repo: parsed,
                        r#ref,
                        sha,
                    } => {
                        assert_eq!(parsed, format!("{owner}/{repo}"), "{arg}");
                        assert_eq!(r#ref.as_deref(), r, "{arg}");
                        assert_eq!(sha, None, "{arg}");
                    }
                    other => panic!("{arg:?} parsed as {other:?}"),
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 5 * 5 * 6, "the enumeration did not run in full");
}

/// Trace: FR-018-AC-3
/// Provenance: quoin#446, carried from tests/props/second-pass.prop.test.ts
#[test]
fn tc_446_064_a_double_slash_yields_git_subdir_split_at_the_first_one() {
    let mut checked = 0_usize;
    for owner in SEGMENTS {
        for repo in SEGMENTS {
            for sub in SEGMENTS {
                for r in refs() {
                    let arg = match r {
                        Some(r) => format!("github:{owner}/{repo}//{sub}@{r}"),
                        None => format!("github:{owner}/{repo}//{sub}"),
                    };
                    match parse_source_arg(&arg).unwrap_or_else(|e| panic!("{arg:?}: {e}")) {
                        Source::GitSubdir {
                            url,
                            path,
                            r#ref,
                            sha,
                        } => {
                            assert_eq!(url, format!("{owner}/{repo}"), "{arg}");
                            assert_eq!(path, *sub, "{arg}");
                            assert_eq!(r#ref.as_deref(), r, "{arg}");
                            assert_eq!(sha, None, "{arg}");
                        }
                        other => panic!("{arg:?} parsed as {other:?}"),
                    }
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(
        checked,
        5 * 5 * 5 * 6,
        "the enumeration did not run in full"
    );
}

/// Trace: FR-018-AC-4
/// Provenance: quoin#446, carried from tests/props/second-pass.prop.test.ts
#[test]
fn tc_446_065_a_package_prefix_splits_on_the_final_at_so_a_scope_survives() {
    // The whole point of the criterion: `@scope/name` starts with an `@`, so a
    // split on the FIRST one would leave the package named `scope/name` and
    // every scoped module would install under the wrong name.
    let mut checked = 0_usize;
    for scope in SEGMENTS {
        for name in SEGMENTS {
            for version in refs() {
                let package = format!("@{scope}/{name}");
                let arg = match version {
                    Some(v) => format!("package:{package}@{v}"),
                    None => format!("package:{package}"),
                };
                match parse_source_arg(&arg).unwrap_or_else(|e| panic!("{arg:?}: {e}")) {
                    Source::Npm {
                        package: parsed,
                        version: parsed_version,
                        registry,
                    } => {
                        assert_eq!(parsed, package, "{arg}");
                        assert_eq!(parsed_version.as_deref(), version, "{arg}");
                        assert_eq!(registry, None, "{arg}");
                    }
                    other => panic!("{arg:?} parsed as {other:?}"),
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 5 * 5 * 6, "the enumeration did not run in full");
}
