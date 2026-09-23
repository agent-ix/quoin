// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Splitting a Rust body into units for per-unit fan-out, and finding one
//! named item in a Rust source file (PLAT-1027).
//!
//! Not a parser. A lexical mask blanks comments, string literals and char
//! literals, so a brace inside `"{"` or `// }` cannot move the structure, and
//! everything else is brace matching on the masked text. Deterministic: the
//! same body always yields the same units in source order.
//!
//! [`split_rust_units`] applies one rule, first match wins:
//!
//! 1. Two or more `fn` items (not nested in another `fn`) → one
//!    [`UnitKind::Function`] unit per item.
//! 2. Otherwise, in the body of the single `fn` (or the whole text when there
//!    is none), every `match` arm and every `if` / `else if` / `else` block at
//!    the body's top nesting level → one [`UnitKind::Branch`] unit each, when
//!    that yields two or more.
//! 3. Otherwise one [`UnitKind::Whole`] unit holding the whole body.

/// What a unit is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnitKind {
    /// One `fn` item.
    Function,
    /// One `match` arm or one `if`/`else` block.
    Branch,
    /// The whole body: nothing smaller applied.
    Whole,
}

/// One unit of a code body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unit {
    /// What it is.
    pub(crate) kind: UnitKind,
    /// A short, human-readable name: `fn parse`, `match arm Some(x)`, `else`.
    pub(crate) label: String,
    /// The unit's source text, verbatim.
    pub(crate) text: String,
}

const fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `text`'s bytes with comments, string literals and char literals replaced
/// by spaces. Same length as `text`, so every index maps back unchanged, and
/// every structural byte left standing is ASCII, so every index this module
/// slices at is a char boundary.
pub(crate) fn mask(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = bytes.to_vec();
    let mut at = 0usize;
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in out.iter_mut().take(to).skip(from) {
            *byte = b' ';
        }
    };
    while at < bytes.len() {
        let rest = bytes.get(at..).unwrap_or_default();
        let prev_ident = at
            .checked_sub(1)
            .and_then(|before| bytes.get(before))
            .copied()
            .is_some_and(is_ident);
        if rest.starts_with(b"//") {
            let end = rest
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |offset| at + offset);
            blank(&mut out, at, end);
            at = end;
        } else if rest.starts_with(b"/*") {
            let end = block_comment_end(bytes, at);
            blank(&mut out, at, end);
            at = end;
        } else if let Some(end) = (!prev_ident).then(|| string_end(bytes, at)).flatten() {
            blank(&mut out, at, end);
            at = end;
        } else if rest.first() == Some(&b'\'') {
            if let Some(end) = char_literal_end(text, at) {
                blank(&mut out, at, end);
                at = end;
            } else {
                at += 1;
            }
        } else {
            at += 1;
        }
    }
    out
}

/// The index just past a (possibly nested) block comment opening at `at`.
fn block_comment_end(bytes: &[u8], at: usize) -> usize {
    let mut depth = 0usize;
    let mut cursor = at;
    while cursor < bytes.len() {
        let pair = bytes.get(cursor..cursor + 2);
        if pair == Some(b"/*") {
            depth += 1;
            cursor += 2;
        } else if pair == Some(b"*/") {
            depth = depth.saturating_sub(1);
            cursor += 2;
            if depth == 0 {
                return cursor;
            }
        } else {
            cursor += 1;
        }
    }
    bytes.len()
}

/// If a string literal (`"…"`, `b"…"`, `r"…"`, `r#"…"#`, `br#"…"#`) opens at
/// `at`, the index just past it.
fn string_end(bytes: &[u8], at: usize) -> Option<usize> {
    let mut cursor = at;
    if bytes.get(cursor) == Some(&b'b') {
        cursor += 1;
    }
    if bytes.get(cursor) == Some(&b'r') {
        cursor += 1;
        let mut hashes = 0usize;
        while bytes.get(cursor) == Some(&b'#') {
            hashes += 1;
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'"') {
            return None;
        }
        cursor += 1;
        while cursor < bytes.len() {
            if bytes.get(cursor) == Some(&b'"')
                && (1..=hashes).all(|offset| bytes.get(cursor + offset) == Some(&b'#'))
            {
                return Some(cursor + 1 + hashes);
            }
            cursor += 1;
        }
        return Some(bytes.len());
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    cursor += 1;
    while cursor < bytes.len() {
        match bytes.get(cursor) {
            Some(b'\\') => cursor += 2,
            Some(b'"') => return Some(cursor + 1),
            _ => cursor += 1,
        }
    }
    Some(bytes.len())
}

/// If a char literal opens at `at` (as opposed to a lifetime), the index just
/// past it.
fn char_literal_end(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(at + 1) == Some(&b'\\') {
        // The escaped char sits at `at + 2` (it may itself be a `'`), so the
        // closing quote is searched for from `at + 3`.
        let close = bytes
            .get(at + 3..)?
            .iter()
            .take(12)
            .position(|byte| *byte == b'\'')?;
        return Some(at + 3 + close + 1);
    }
    let next = text.get(at + 1..)?.chars().next()?;
    let after = at + 1 + next.len_utf8();
    (bytes.get(after) == Some(&b'\'')).then_some(after + 1)
}

/// The index of the delimiter closing the one at `open`, in masked bytes.
fn matching(masked: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i64;
    for (index, byte) in masked.iter().enumerate().skip(open) {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether `word` stands alone at `at` in `masked`.
fn word_at(masked: &[u8], at: usize, word: &str) -> bool {
    let end = at + word.len();
    masked.get(at..end) == Some(word.as_bytes())
        && !at
            .checked_sub(1)
            .and_then(|before| masked.get(before))
            .copied()
            .is_some_and(is_ident)
        && !masked.get(end).copied().is_some_and(is_ident)
}

fn skip_ws(masked: &[u8], mut at: usize, end: usize) -> usize {
    while at < end && masked.get(at).is_some_and(u8::is_ascii_whitespace) {
        at += 1;
    }
    at
}

/// The first `{` at bracket depth zero in `[from, end)`, stopping at a `;`
/// at depth zero (a declaration with no body).
fn body_open(masked: &[u8], from: usize, end: usize) -> Option<usize> {
    let mut depth = 0i64;
    for index in from..end {
        match masked.get(index)? {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'{' if depth == 0 => return Some(index),
            b';' if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// One `fn` item found in a text: where it starts (its `fn` keyword), its
/// name, and where its body's closing brace is.
struct FnItem {
    keyword: usize,
    name: String,
    close: usize,
}

/// The identifier starting at `at`.
fn ident_at(text: &str, masked: &[u8], at: usize) -> String {
    let end = (at..masked.len())
        .find(|index| !masked.get(*index).copied().is_some_and(is_ident))
        .unwrap_or(masked.len());
    text.get(at..end).unwrap_or_default().to_owned()
}

/// Every `fn` item with a body in `[from, end)`, skipping `fn`s nested inside
/// another `fn`'s body.
fn fn_items(text: &str, masked: &[u8], from: usize, end: usize) -> Vec<FnItem> {
    let mut items = Vec::new();
    let mut at = from;
    while at < end {
        if !word_at(masked, at, "fn") {
            at += 1;
            continue;
        }
        let name_at = skip_ws(masked, at + 2, end);
        let name = ident_at(text, masked, name_at);
        let Some(open) = body_open(masked, name_at, end) else {
            at += 2;
            continue;
        };
        let Some(close) = matching(masked, open).filter(|close| *close < end) else {
            at += 2;
            continue;
        };
        if !name.is_empty() {
            items.push(FnItem {
                keyword: at,
                name,
                close,
            });
        }
        at = close + 1;
    }
    items
}

/// The start of the line holding `at`, extended upward over contiguous
/// attribute (`#[`) and doc-comment (`///`) lines, so an extracted test keeps
/// its `#[test]` and its `/// Trace:` line.
fn item_start(text: &str, at: usize) -> usize {
    let line_start = |index: usize| {
        text.get(..index)
            .and_then(|head| head.rfind('\n'))
            .map_or(0, |newline| newline + 1)
    };
    let mut start = line_start(at);
    while start > 0 {
        let previous = line_start(start - 1);
        let line = text.get(previous..start).unwrap_or_default().trim();
        if line.starts_with("#[") || line.starts_with("///") {
            start = previous;
        } else {
            break;
        }
    }
    start
}

/// The source of the one `fn` named `name` in `source`, from its attributes
/// and doc comments through its closing brace. `name` may be a path
/// (`Type::method`); only its last segment is matched.
///
/// # Errors
/// When no `fn` has that name, or more than one does — an ambiguous symbol
/// is refused rather than resolved to whichever came first.
pub(crate) fn extract_rust_fn(source: &str, name: &str) -> Result<String, String> {
    let short = name.rsplit("::").next().unwrap_or(name);
    let masked = mask(source);
    let mut found = Vec::new();
    let mut at = 0usize;
    // Every `fn`, nested or not: a method lives inside an `impl` block.
    while at < masked.len() {
        if word_at(&masked, at, "fn") {
            let name_at = skip_ws(&masked, at + 2, masked.len());
            if ident_at(source, &masked, name_at) == short
                && let Some(open) = body_open(&masked, name_at, masked.len())
                && let Some(close) = matching(&masked, open)
            {
                found.push((item_start(source, at), close));
            }
        }
        at += 1;
    }
    match found.as_slice() {
        [(start, close)] => Ok(source.get(*start..=*close).unwrap_or_default().to_owned()),
        [] => Err(format!("no `fn {short}` with a body in the source")),
        many => Err(format!(
            "`fn {short}` is ambiguous: {} definitions in the source",
            many.len()
        )),
    }
}

/// Splits a Rust code body into units, by the rule in this module's doc.
pub(crate) fn split_rust_units(body: &str) -> Vec<Unit> {
    let masked = mask(body);
    let items = fn_items(body, &masked, 0, masked.len());
    if items.len() >= 2 {
        return items
            .iter()
            .map(|item| Unit {
                kind: UnitKind::Function,
                label: format!("fn {}", item.name),
                text: body
                    .get(item_start(body, item.keyword)..=item.close)
                    .unwrap_or_default()
                    .to_owned(),
            })
            .collect();
    }
    let (from, end) = items
        .first()
        .and_then(|item| {
            let open = body_open(&masked, item.keyword, item.close)?;
            Some((open + 1, item.close))
        })
        .unwrap_or((0, masked.len()));
    let branches = branches(body, &masked, from, end);
    if branches.len() >= 2 {
        return branches;
    }
    vec![Unit {
        kind: UnitKind::Whole,
        label: "whole body".to_owned(),
        text: body.to_owned(),
    }]
}

/// A one-line label: whitespace collapsed, cut at 60 chars.
fn short_label(prefix: &str, text: &str) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = collapsed.chars().take(60).collect();
    if cut.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix} {cut}")
    }
}

/// Every top-level `match` arm and `if`/`else` block in `[from, end)`.
fn branches(text: &str, masked: &[u8], from: usize, end: usize) -> Vec<Unit> {
    let mut units = Vec::new();
    let mut depth = 0i64;
    let mut at = from;
    while at < end {
        let Some(byte) = masked.get(at) else { break };
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0
            && word_at(masked, at, "match")
            && let Some(close) = match_arms(text, masked, at, end, &mut units)
        {
            at = close + 1;
            continue;
        }
        if depth == 0
            && word_at(masked, at, "if")
            && let Some(close) = if_chain(text, masked, at, end, &mut units)
        {
            at = close + 1;
            continue;
        }
        at += 1;
    }
    units
}

/// Pushes one unit per arm of the `match` at `at`; returns its closing brace.
fn match_arms(
    text: &str,
    masked: &[u8],
    at: usize,
    end: usize,
    units: &mut Vec<Unit>,
) -> Option<usize> {
    let open = body_open(masked, at + 5, end)?;
    let close = matching(masked, open).filter(|close| *close < end)?;
    let mut cursor = open + 1;
    loop {
        cursor = skip_ws(masked, cursor, close);
        if cursor >= close {
            break;
        }
        let arrow = find_arrow(masked, cursor, close)?;
        let pattern = text.get(cursor..arrow).unwrap_or_default();
        let value = skip_ws(masked, arrow + 2, close);
        let arm_end = if masked.get(value) == Some(&b'{') {
            matching(masked, value)? + 1
        } else {
            top_level_comma(masked, value, close).unwrap_or(close)
        };
        units.push(Unit {
            kind: UnitKind::Branch,
            label: short_label("match arm", pattern),
            text: text
                .get(cursor..arm_end)
                .unwrap_or_default()
                .trim()
                .to_owned(),
        });
        cursor = skip_ws(masked, arm_end, close);
        if masked.get(cursor) == Some(&b',') {
            cursor += 1;
        }
    }
    Some(close)
}

/// The first `=>` at bracket depth zero in `[from, end)`.
fn find_arrow(masked: &[u8], from: usize, end: usize) -> Option<usize> {
    let mut depth = 0i64;
    for index in from..end {
        match masked.get(index)? {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'=' if depth == 0 && masked.get(index + 1) == Some(&b'>') => return Some(index),
            _ => {}
        }
    }
    None
}

/// The first `,` at bracket depth zero in `[from, end)`.
fn top_level_comma(masked: &[u8], from: usize, end: usize) -> Option<usize> {
    let mut depth = 0i64;
    for index in from..end {
        match masked.get(index)? {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

/// Pushes one unit per block of the `if` chain at `at`; returns the last
/// block's closing brace.
fn if_chain(
    text: &str,
    masked: &[u8],
    at: usize,
    end: usize,
    units: &mut Vec<Unit>,
) -> Option<usize> {
    let mut start = at;
    let mut keyword_end = at + 2;
    let mut prefix = "if";
    loop {
        let open = body_open(masked, keyword_end, end)?;
        let close = matching(masked, open).filter(|close| *close < end)?;
        let condition = text.get(keyword_end..open).unwrap_or_default();
        units.push(Unit {
            kind: UnitKind::Branch,
            label: short_label(prefix, condition),
            text: text.get(start..=close).unwrap_or_default().to_owned(),
        });
        let next = skip_ws(masked, close + 1, end);
        if !word_at(masked, next, "else") {
            return Some(close);
        }
        let after_else = skip_ws(masked, next + 4, end);
        start = next;
        if word_at(masked, after_else, "if") {
            keyword_end = after_else + 2;
            prefix = "else if";
        } else {
            let open = (masked.get(after_else) == Some(&b'{')).then_some(after_else)?;
            let close = matching(masked, open).filter(|close| *close < end)?;
            units.push(Unit {
                kind: UnitKind::Branch,
                label: "else".to_owned(),
                text: text.get(start..=close).unwrap_or_default().to_owned(),
            });
            return Some(close);
        }
    }
}
