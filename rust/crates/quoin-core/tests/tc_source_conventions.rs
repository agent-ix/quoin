// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The conventions `.claude/skills/rust-style/SKILL.md` states, checked.
//!
//! A source-inspection test is a last resort (rust-review §2), and it is the
//! right resort here: no runtime path can observe whether a file carries an
//! SPDX header, and an idiom doc nobody enforces is a doc that stops being
//! true. Every rule below is one the style doc states in prose.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // crates/quoin-core -> crates -> <workspace root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Provenance: quoin#375
#[test]
fn tc_375_every_rust_file_carries_the_agpl_spdx_header() {
    let root = workspace_root();
    let mut files = Vec::new();
    rust_sources(&root.join("crates"), &mut files);

    let mut missing = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        let head: String = text.lines().take(2).collect::<Vec<_>>().join("\n");
        if !head.contains("SPDX-License-Identifier: AGPL-3.0-or-later")
            || !head.contains("Copyright (C) 2026 Agent-IX")
        {
            missing.push(file.strip_prefix(&root).unwrap().display().to_string());
        }
    }
    assert_eq!(missing, Vec::<String>::new());
}

/// Provenance: quoin#375
#[test]
fn tc_375_every_crate_opts_into_the_workspace_lint_policy() {
    // The style doc says there is no exception. An exception added without
    // editing this test — and the reason in the root Cargo.toml — fails here.
    let root = workspace_root();
    let mut without = Vec::new();
    for entry in std::fs::read_dir(root.join("crates")).unwrap() {
        let manifest = entry.unwrap().path().join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).unwrap();
        if !text.contains("[lints]\nworkspace = true") {
            without.push(manifest.strip_prefix(&root).unwrap().display().to_string());
        }
    }
    assert_eq!(without, Vec::<String>::new());
}

// ---------------------------------------------------------------------------
// FR-036-AC-8 / CON-1: the executables quoin starts.
//
// quoin transcribes; the consumer's CI executes (ADR-0011 invariant 1). The
// set is asserted whole, so a new binary fails by default. The scan fails
// CLOSED: a file it cannot lex, or a process construction it cannot resolve to
// a known executable, is an error naming the file and line, never a skip.
// ---------------------------------------------------------------------------

/// Developer tools that ship no engine behaviour.
const DEVELOPER_TOOLS: &[&str] = &["quoin-suite-trace", "quoin-workflow-check"];

/// Executables quoin starts.
const EXPECTED_EXECUTABLES: &[&str] = &["$EDITOR", "git", "ix-flow"];

/// Non-literal executor arguments, each keyed by the file that owns it:
/// `runner_program()` names `ix-flow`; `config edit` starts the operator's own
/// editor.
const KNOWN_ARGUMENTS: &[(&str, &str, &str)] = &[
    ("quoin-cli/src/flow.rs", "runner_program()", "ix-flow"),
    ("quoin-cli/src/config.rs", "&editor", "$EDITOR"),
];

/// Fillers keep each character's UTF-8 width so byte offsets agree with the
/// original text.
fn fill(c: char, with: char) -> String {
    if c == '\n' {
        "\n".to_owned()
    } else {
        with.to_string().repeat(c.len_utf8())
    }
}

fn char_line(chars: &[char], at: usize) -> usize {
    chars[..at].iter().filter(|c| **c == '\n').count() + 1
}

/// Blank the (nested) block comment at `start`; returns the index after it.
fn mask_block_comment(chars: &[char], start: usize, out: &mut String) -> Result<usize, String> {
    let mut i = start;
    let mut depth = 0usize;
    loop {
        match (chars.get(i), chars.get(i + 1)) {
            (Some('/'), Some('*')) => {
                depth += 1;
                out.push_str("  ");
                i += 2;
            }
            (Some('*'), Some('/')) => {
                depth -= 1;
                out.push_str("  ");
                i += 2;
                if depth == 0 {
                    return Ok(i);
                }
            }
            (Some(c), _) => {
                out.push_str(&fill(*c, ' '));
                i += 1;
            }
            (None, _) => {
                return Err(format!(
                    "line {}: unterminated block comment",
                    char_line(chars, start)
                ));
            }
        }
    }
}

/// Replace the body of the string literal (plain or raw) at `start` with `_`;
/// returns the index after it.
fn mask_string(chars: &[char], start: usize, out: &mut String) -> Result<usize, String> {
    let mut i = start;
    let c = chars[i];
    let (hashes, raw) = if c == 'r' {
        let hashes = raw_open(chars, i).unwrap_or(0);
        out.push('r');
        for _ in 0..hashes {
            out.push('#');
        }
        out.push('"');
        i += hashes + 2;
        (hashes, true)
    } else {
        out.push('"');
        i += 1;
        (0, false)
    };
    loop {
        let Some(&c) = chars.get(i) else {
            return Err(format!(
                "line {}: unterminated string",
                char_line(chars, start)
            ));
        };
        if !raw && c == '\\' {
            out.push_str(&fill(c, '_'));
            if let Some(&escaped) = chars.get(i + 1) {
                out.push_str(&fill(escaped, '_'));
            }
            i += 2;
        } else if c == '"' && (0..hashes).all(|k| chars.get(i + 1 + k) == Some(&'#')) {
            out.push('"');
            for _ in 0..hashes {
                out.push('#');
            }
            i += 1 + hashes;
            break;
        } else {
            out.push_str(&fill(c, '_'));
            i += 1;
        }
    }
    Ok(i)
}

/// `(masked, original)`: comments blanked to spaces and string/char bodies
/// replaced by `_`, newlines and offsets preserved, so structure can be read
/// without a comment or a string fooling it.
fn mask(source: &str) -> Result<String, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push_str(&fill(chars[i], ' '));
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i = mask_block_comment(&chars, i, &mut out)?;
        } else if c == '"'
            || (c == 'r' && matches!(next, Some('"' | '#')) && raw_open(&chars, i).is_some())
        {
            i = mask_string(&chars, i, &mut out)?;
        } else if c == '\'' {
            // A char literal is `'x'` or `'\..'`; anything else is a lifetime.
            let closes = if next == Some('\\') {
                chars[i + 2..]
                    .iter()
                    .take(10)
                    .position(|c| *c == '\'')
                    .map(|p| i + 3 + p)
            } else if chars.get(i + 2) == Some(&'\'') {
                Some(i + 3)
            } else {
                None
            };
            if let Some(end) = closes {
                for c in &chars[i..end] {
                    out.push_str(&fill(*c, '_'));
                }
                i = end;
            } else {
                out.push('\'');
                i += 1;
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    Ok(out)
}

/// Hash count of a raw-string opener at `at` (`r"`, `r#"`), if it is one.
fn raw_open(chars: &[char], at: usize) -> Option<usize> {
    let hashes = chars[at + 1..].iter().take_while(|c| **c == '#').count();
    (chars.get(at + 1 + hashes) == Some(&'"')).then_some(hashes)
}

fn line_at(text: &str, offset: usize) -> usize {
    text[..offset].matches('\n').count() + 1
}

/// Offset of the `}` closing the `{` at `open`, in masked text.
fn matching_brace(masked: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (at, c) in masked.char_indices().skip_while(|(at, _)| *at < open) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// Blank every `#[cfg(test)] mod name { .. }` (attributes may sit between).
fn without_test_modules(masked: &str) -> Result<String, String> {
    let mut text = masked.to_owned();
    let mut from = 0;
    while let Some(found) = text[from..].find("#[cfg(test)]") {
        let at = from + found;
        let mut rest = at + "#[cfg(test)]".len();
        // Skip further attributes and whitespace.
        loop {
            let trimmed = text[rest..].trim_start();
            rest = text.len() - trimmed.len();
            if trimmed.starts_with("#[") {
                let close = trimmed.find(']').ok_or("unclosed attribute")?;
                rest += close + 1;
            } else {
                break;
            }
        }
        let item = &text[rest..];
        let item = item.strip_prefix("pub ").unwrap_or(item);
        if item.starts_with("mod ") {
            let brace_or_semi = text[rest..].find(['{', ';']).map(|p| rest + p);
            match brace_or_semi {
                Some(open) if text.as_bytes()[open] == b'{' => {
                    let close = matching_brace(&text, open).ok_or_else(|| {
                        format!(
                            "line {}: unbalanced braces in a test module",
                            line_at(&text, open)
                        )
                    })?;
                    let blanked: String = text[at..=close]
                        .chars()
                        .map(|c| {
                            if c == '\n' {
                                "\n".to_owned()
                            } else {
                                " ".repeat(c.len_utf8())
                            }
                        })
                        .collect();
                    text.replace_range(at..=close, &blanked);
                }
                // `#[cfg(test)] mod tests;` lives in its own `tests.rs`.
                _ => {}
            }
        }
        from = at + 1;
    }
    Ok(text)
}

/// Names bound to a process `Command` by the file's `use` items.
fn command_names(masked: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut from = 0;
    while let Some(found) = masked[from..].find("use ") {
        let start = from + found;
        from = start + 4;
        let before = masked[..start].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let Some(end) = masked[start..].find(';') else {
            continue;
        };
        let statement = &masked[start + 4..start + end];
        if !statement.contains("process") {
            continue;
        }
        let tokens: Vec<&str> = statement
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|t| !t.is_empty())
            .collect();
        for (index, token) in tokens.iter().enumerate() {
            if *token == "Command" {
                match (tokens.get(index + 1), tokens.get(index + 2)) {
                    (Some(&"as"), Some(alias)) => names.insert((*alias).to_owned()),
                    _ => names.insert("Command".to_owned()),
                };
            }
        }
    }
    names
}

/// The text of the call's argument list starting at the `(` at `open`.
fn call_argument(text: &str, open: usize) -> Option<&str> {
    let mut depth = 0usize;
    for (at, c) in text.char_indices().skip_while(|(at, _)| *at < open) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[open + 1..at]);
                }
            }
            _ => {}
        }
    }
    None
}

/// The executables `source` (at `path`, relative to `crates/`) starts through
/// a process `Command`, or every reason the scan cannot say.
fn executables_in(path: &str, source: &str) -> Result<BTreeSet<String>, Vec<String>> {
    let masked = mask(source).map_err(|e| vec![format!("{path}: {e}")])?;
    let code = without_test_modules(&masked).map_err(|e| vec![format!("{path}: {e}")])?;
    let names = command_names(&code);
    let mut found = BTreeSet::new();
    let mut errors = Vec::new();
    let mut from = 0;
    while let Some(found_at) = code[from..].find("::new(") {
        let new_at = from + found_at;
        from = new_at + 1;
        let receiver: String = code[..new_at]
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let last = receiver.rsplit("::").next().unwrap_or("");
        let qualified = receiver.ends_with("process::Command");
        if !(qualified || names.contains(last)) {
            continue;
        }
        let open = new_at + "::new".len();
        let line = line_at(&code, new_at);
        let Some(argument) = call_argument(&code, open) else {
            errors.push(format!(
                "{path}:{line}: unbalanced process Command::new call"
            ));
            continue;
        };
        let original = call_argument(source, open).unwrap_or(argument).trim();
        if original.starts_with('"') && original.ends_with('"') && original.len() >= 2 {
            found.insert(original[1..original.len() - 1].to_owned());
        } else if let Some((_, _, name)) = KNOWN_ARGUMENTS
            .iter()
            .find(|(file, arg, _)| path.ends_with(file) && *arg == original)
        {
            found.insert((*name).to_owned());
        } else {
            errors.push(format!(
                "{path}:{line}: cannot resolve the executable `{original}` to a literal or a known argument"
            ));
        }
    }
    if errors.is_empty() {
        Ok(found)
    } else {
        Err(errors)
    }
}

/// FR-036-AC-8, CON-1.
#[trace("FR-036-AC-8", "FR-036-CON-1", "TC-203")]
#[test]
fn quoin_executes_exactly_git_and_ix_flow_and_the_operators_editor() {
    let crates = workspace_root().join("crates");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&crates).unwrap() {
        let krate = entry.unwrap().path();
        let name = krate.file_name().unwrap().to_string_lossy().into_owned();
        if !DEVELOPER_TOOLS.contains(&name.as_str()) && krate.join("src").is_dir() {
            rust_sources(&krate.join("src"), &mut files);
        }
    }
    files.retain(|f| f.file_name().is_none_or(|n| n != "tests.rs"));
    assert!(
        files.len() > 50,
        "the walk must reach the production sources"
    );
    let mut found = BTreeSet::new();
    let mut errors = Vec::new();
    for file in files {
        let relative = file.strip_prefix(&crates).unwrap().display().to_string();
        match executables_in(&relative, &std::fs::read_to_string(&file).unwrap()) {
            Ok(set) => found.extend(set),
            Err(mut e) => errors.append(&mut e),
        }
    }
    assert_eq!(errors, Vec::<String>::new());
    let expected: BTreeSet<String> = EXPECTED_EXECUTABLES
        .iter()
        .map(|n| (*n).to_owned())
        .collect();
    assert_eq!(found, expected);
}

fn injected(source: &str) -> Result<BTreeSet<String>, Vec<String>> {
    executables_in("quoin-x/src/lib.rs", source)
}

/// The check must be able to fail: each way the first version silently
/// dropped an executor now reports it.
#[trace("FR-036-AC-8", "FR-036-CON-1", "TC-203")]
#[test]
fn the_executable_check_sees_an_injected_binary() {
    let one = |s: &str| BTreeSet::from([s.to_owned()]);
    // A plain literal.
    let plain = "use std::process::Command;\nfn f() { Command::new(\"curl\"); }\n";
    assert_eq!(injected(plain), Ok(one("curl")));
    // (a) a `#[cfg(test)]` in a doc comment or string does not hide the rest.
    let hidden = "/// see #[cfg(test)]\nuse std::process::Command;\nconst S: &str = \"#[cfg(test)]\";\nfn f() { Command::new(\"curl\"); }\n";
    assert_eq!(injected(hidden), Ok(one("curl")));
    // ... while a real test module is excluded, and code after it is read.
    let real = "use std::process::Command;\n#[cfg(test)]\nmod tests { fn t() { Command::new(\"mkfifo\"); } }\nfn f() { Command::new(\"curl\"); }\n";
    assert_eq!(injected(real), Ok(one("curl")));
    // (b) grouped import in any order.
    let grouped = "use std::process::{Stdio, Command};\nfn f() { Command::new(\"curl\"); }\n";
    assert_eq!(injected(grouped), Ok(one("curl")));
    // (c) any alias.
    let aliased = "use std::process::{Command as Cmd, Stdio};\nfn f() { Cmd::new(\"curl\"); }\n";
    assert_eq!(injected(aliased), Ok(one("curl")));
    // Fully qualified, with no import.
    let qualified = "fn f() { std::process::Command::new(\"curl\"); }\n";
    assert_eq!(injected(qualified), Ok(one("curl")));
    // An executable it cannot resolve fails with file and line.
    let opaque = "use std::process::Command;\nfn f() {\n    Command::new(tool);\n}\n";
    let errors = injected(opaque).unwrap_err();
    assert!(errors[0].starts_with("quoin-x/src/lib.rs:3:"), "{errors:?}");
    // A file it cannot lex fails rather than being skipped.
    assert!(injected("fn f() { \"open\n").is_err());
    // clap's `Command` is not a process.
    assert_eq!(
        injected("use clap::Command;\nfn f() { Command::new(\"config\"); }\n"),
        Ok(BTreeSet::new())
    );
}
