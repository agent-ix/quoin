// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Finding one named `def` in a Python source file, and splitting a Python
//! body into units for per-unit fan-out (PLAT-1035, parent PLAT-1024).
//!
//! The external corpus (PLAT-1026) dropped 17 sampled units because their
//! test or code was Python and the harness read Rust `fn` items only. This is
//! the Python half; [`super`] picks it by file extension.
//!
//! Not a parser, like the Rust half. A lexical mask blanks `#` comments and
//! turns every string literal, triple-quoted ones across lines included, into
//! a run of `"` with no newline left in it. The masked text is read as
//! logical lines: a physical line continues the one before while a bracket is
//! open or the one before ends in `\`, and blank or comment-only lines are no
//! lines at all. A logical line's indentation is its first physical line's
//! leading whitespace, a tab advancing to the next multiple of 8 as in
//! Python's own tokenizer. Deterministic: the same text always yields the same
//! result.
//!
//! A `def` / `async def` / `class` block is its header line plus every
//! following logical line indented deeper. Its source runs from its first
//! decorator (a `@` logical line at the same indentation directly above, so a
//! decorator spanning several lines comes whole), extended upward over
//! directly attached `#` comment lines (a blank line stops it), through its
//! last logical line. The docstring is the body's first statement, so it is
//! always inside.
//!
//! [`split_python_units`] mirrors the Rust splitter's rule exactly, so a
//! per-unit variant sees the same shapes in both languages and its results
//! are not skewed by language. First match wins:
//!
//! 1. Two or more `def`s not nested in another `def` (module-level functions
//!    and methods) → one [`UnitKind::Function`] unit per `def`.
//! 2. Otherwise, in the body of the single `def` (or the whole text when
//!    there is none), every clause of every `if` / `elif` / `else`,
//!    `try` / `except` / `else` / `finally` and `match` / `case` statement at
//!    the body's own indentation → one [`UnitKind::Branch`] unit each, when
//!    that yields two or more. A comment between two clauses belongs to the
//!    clause after it.
//! 3. Otherwise one [`UnitKind::Whole`] unit holding the whole body.
//!
//! Limits, all lexical: an f-string that nests its own quote character
//! (allowed from Python 3.12) is read as two adjacent strings, which masks
//! the same bytes unless a bracket or `#` sits between them; a non-ASCII
//! identifier character directly after a keyword is not a word boundary.

use super::{Unit, UnitKind, short_label, skip_ws, whole, word_at};

/// What every byte of a string literal becomes in the mask: not whitespace
/// (so a docstring-only line is still a line), not a bracket, not an
/// identifier byte, and not a newline.
const MASKED_STRING: u8 = b'"';

/// The UTF-8 byte-order mark.
const BOM: &str = "\u{feff}";

/// `text`'s bytes with every comment replaced by spaces and every string
/// literal, quotes and inner newlines included, replaced by
/// [`MASKED_STRING`]. A leading byte-order mark becomes form feeds, which
/// are whitespace of zero indentation width (as in Python's tokenizer), so
/// the first line keeps indentation 0 and its decorator stays attached.
/// Same length as `text`, so every index maps back unchanged; every `\n`
/// left is a real line break; and every masked range is whole characters,
/// so every index this module slices at is a char boundary.
fn mask(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = bytes.to_vec();
    let mut at = 0usize;
    if text.starts_with(BOM) {
        fill(&mut out, 0, BOM.len(), b'\x0c');
        at = BOM.len();
    }
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b'#' => {
                let end = line_end(bytes, at);
                fill(&mut out, at, end, b' ');
                at = end;
            }
            b'"' | b'\'' => {
                let end = string_end(bytes, at, byte);
                fill(&mut out, at, end, MASKED_STRING);
                at = end;
            }
            _ => at += 1,
        }
    }
    out
}

fn fill(out: &mut [u8], from: usize, to: usize, with: u8) {
    for byte in out.iter_mut().take(to).skip(from) {
        *byte = with;
    }
}

/// The index just past the string literal whose opening `quote` is at `at`.
/// A backslash always escapes the next byte, in raw strings too (`r"\""` is
/// one literal). An unterminated single-quoted string ends at its line's end;
/// an unterminated triple-quoted one at the end of the text.
fn string_end(bytes: &[u8], at: usize, quote: u8) -> usize {
    let triple = [quote; 3];
    let is_triple = bytes.get(at..at + 3) == Some(&triple[..]);
    let mut cursor = at + if is_triple { 3 } else { 1 };
    while let Some(&byte) = bytes.get(cursor) {
        if byte == b'\\' {
            cursor += 2;
            continue;
        }
        if is_triple {
            if bytes.get(cursor..cursor + 3) == Some(&triple[..]) {
                return cursor + 3;
            }
        } else if byte == quote {
            return cursor + 1;
        } else if byte == b'\n' {
            return cursor;
        }
        cursor += 1;
    }
    bytes.len()
}

/// The index of the `\n` ending the physical line holding `from`, or the end
/// of `bytes`.
fn line_end(bytes: &[u8], from: usize) -> usize {
    bytes
        .get(from..)
        .and_then(|rest| rest.iter().position(|byte| *byte == b'\n'))
        .map_or(bytes.len(), |offset| from + offset)
}

/// The start of the physical line holding `at`, in masked bytes.
fn line_start(masked: &[u8], at: usize) -> usize {
    masked
        .get(..at)
        .and_then(|head| head.iter().rposition(|byte| *byte == b'\n'))
        .map_or(0, |newline| newline + 1)
}

/// One logical line.
#[derive(Debug, Clone, Copy)]
struct Line {
    /// Where its first physical line starts.
    start: usize,
    /// Where its first token starts.
    head: usize,
    /// Its last physical line's `\n`, or the end of the text.
    end: usize,
    /// Its first physical line's indentation, tabs expanded.
    indent: usize,
}

/// The indentation of the physical line `[start, end)` and where its first
/// token is; the token index is `end` for a blank line.
fn indentation(masked: &[u8], start: usize, end: usize) -> (usize, usize) {
    let mut width = 0usize;
    for index in start..end {
        match masked.get(index) {
            Some(b' ') => width += 1,
            Some(b'\t') => width = (width / 8 + 1) * 8,
            Some(b'\r' | b'\x0c') => {}
            _ => return (width, index),
        }
    }
    (width, end)
}

/// Whether the physical line `[from, end)` ends in a `\` continuation.
fn ends_with_backslash(masked: &[u8], from: usize, end: usize) -> bool {
    masked
        .get(from..end)
        .and_then(|line| line.iter().rev().find(|byte| **byte != b'\r'))
        == Some(&b'\\')
}

/// Every logical line of the masked text, in order.
fn logical_lines(masked: &[u8]) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut at = 0usize;
    while at < masked.len() {
        let first_end = line_end(masked, at);
        let (indent, head) = indentation(masked, at, first_end);
        if head == first_end {
            at = first_end + 1;
            continue;
        }
        let mut depth = 0i64;
        let mut from = head;
        let mut end = first_end;
        loop {
            for byte in masked.get(from..end).unwrap_or_default() {
                match byte {
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => depth -= 1,
                    _ => {}
                }
            }
            let continued = depth > 0 || ends_with_backslash(masked, from, end);
            if !continued || end >= masked.len() {
                break;
            }
            from = end + 1;
            end = line_end(masked, from);
        }
        lines.push(Line {
            start: at,
            head,
            end,
            indent,
        });
        at = end + 1;
    }
    lines
}

/// What a block is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    /// `def` or `async def`.
    Def,
    /// `class`.
    Class,
}

/// One `def` or `class` block.
#[derive(Debug, Clone)]
struct Block {
    kind: BlockKind,
    name: String,
    /// Its header's index in the logical lines.
    line: usize,
    /// One past its last logical line.
    end: usize,
    /// The innermost block holding it, as an index into the blocks.
    parent: Option<usize>,
}

/// The block a logical line whose first token is at `head` opens, if any:
/// `def name`, `async def name` or `class name`.
fn header(text: &str, masked: &[u8], head: usize) -> Option<(BlockKind, String)> {
    let is_async = word_at(masked, head, "async");
    let at = if is_async {
        skip_ws(masked, head + 5, masked.len())
    } else {
        head
    };
    let (kind, after) = if word_at(masked, at, "def") {
        (BlockKind::Def, at + 3)
    } else if !is_async && word_at(masked, at, "class") {
        (BlockKind::Class, at + 5)
    } else {
        return None;
    };
    let name_at = skip_ws(masked, after, masked.len());
    let name: String = text
        .get(name_at..)?
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some((kind, name))
}

/// Every `def` and `class` block, nested or not, in source order.
fn blocks(text: &str, masked: &[u8], lines: &[Line]) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((kind, name)) = header(text, masked, line.head) else {
            continue;
        };
        let end = lines
            .iter()
            .skip(index + 1)
            .position(|next| next.indent <= line.indent)
            .map_or(lines.len(), |offset| index + 1 + offset);
        while open
            .last()
            .and_then(|outer| blocks.get(*outer))
            .is_some_and(|outer| outer.end <= index)
        {
            open.pop();
        }
        blocks.push(Block {
            kind,
            name,
            line: index,
            end,
            parent: open.last().copied(),
        });
        open.push(blocks.len() - 1);
    }
    blocks
}

/// Whether `block` sits anywhere inside a `def`'s body.
fn nested_in_def(blocks: &[Block], block: &Block) -> bool {
    let mut parent = block.parent;
    while let Some(outer) = parent.and_then(|index| blocks.get(index)) {
        if outer.kind == BlockKind::Def {
            return true;
        }
        parent = outer.parent;
    }
    false
}

/// The class directly holding `block`, if any.
fn owning_class<'a>(blocks: &'a [Block], block: &Block) -> Option<&'a Block> {
    block
        .parent
        .and_then(|index| blocks.get(index))
        .filter(|outer| outer.kind == BlockKind::Class)
}

/// `block`'s source: its decorators and attached comments through its last
/// logical line, trailing whitespace dropped.
fn block_text(text: &str, masked: &[u8], lines: &[Line], block: &Block) -> String {
    let Some(header) = lines.get(block.line) else {
        return String::new();
    };
    let mut first = block.line;
    while let Some(above) = first.checked_sub(1).and_then(|index| lines.get(index)) {
        if above.indent != header.indent || masked.get(above.head) != Some(&b'@') {
            break;
        }
        first -= 1;
    }
    let mut start = lines.get(first).map_or(header.start, |line| line.start);
    while start > 0 {
        let previous = line_start(masked, start - 1);
        let comment_only = text
            .get(previous..start)
            .is_some_and(|line| line.trim_start_matches(BOM).trim_start().starts_with('#'))
            && masked
                .get(previous..start)
                .is_some_and(|line| line.iter().all(u8::is_ascii_whitespace));
        if !comment_only {
            break;
        }
        start = previous;
    }
    let end = block
        .end
        .checked_sub(1)
        .and_then(|last| lines.get(last))
        .map_or(header.end, |last| last.end);
    text.get(start..end)
        .unwrap_or_default()
        .trim_start_matches(BOM)
        .trim_end()
        .to_owned()
}

/// The source of the one `def` `name` names in `source`, from its decorators
/// and attached comments through the end of its body.
///
/// `name` is `function`, or a dotted path (`Class.method`; `::` is read as
/// `.` too). A `def` nested inside another `def`'s body is never a match; a
/// method is. For a path, the segment before the name is the owning class:
/// when `source` has a `class` of that name, only `def`s directly in such a
/// class body match; when it has none, the qualifier is read as a module
/// (`mymod.run`) and only defs outside any class match, so `Foo.run` never
/// resolves to `Bar.run`.
///
/// # Errors
/// When no `def` matches, or more than one does: an ambiguous symbol is
/// refused rather than resolved to whichever came first.
pub(crate) fn extract_python_def(source: &str, name: &str) -> Result<String, String> {
    let segments: Vec<&str> = name
        .split("::")
        .flat_map(|segment| segment.split('.'))
        .collect();
    let short = segments.last().copied().unwrap_or(name);
    let qualifier = segments
        .len()
        .checked_sub(2)
        .and_then(|index| segments.get(index))
        .copied();
    let masked = mask(source);
    let lines = logical_lines(&masked);
    let blocks = blocks(source, &masked, &lines);
    let mut candidates: Vec<&Block> = blocks
        .iter()
        .filter(|block| {
            block.kind == BlockKind::Def && block.name == short && !nested_in_def(&blocks, block)
        })
        .collect();
    if let Some(qualifier) = qualifier {
        let is_class = blocks
            .iter()
            .any(|block| block.kind == BlockKind::Class && block.name == qualifier);
        candidates.retain(|block| match owning_class(&blocks, block) {
            Some(class) => is_class && class.name == qualifier,
            None => !is_class,
        });
    }
    match candidates.as_slice() {
        [block] => Ok(block_text(source, &masked, &lines, block)),
        [] => Err(format!(
            "no `def {name}` outside another function's body in the source"
        )),
        many => Err(format!(
            "`def {name}` is ambiguous: {} definitions in the source",
            many.len()
        )),
    }
}

/// Splits a Python code body into units, by the rule in this module's doc.
pub(crate) fn split_python_units(body: &str) -> Vec<Unit> {
    let masked = mask(body);
    let lines = logical_lines(&masked);
    let blocks = blocks(body, &masked, &lines);
    let functions: Vec<&Block> = blocks
        .iter()
        .filter(|block| block.kind == BlockKind::Def && !nested_in_def(&blocks, block))
        .collect();
    if functions.len() >= 2 {
        return functions
            .iter()
            .map(|block| {
                let label = owning_class(&blocks, block).map_or_else(
                    || format!("def {}", block.name),
                    |class| format!("def {}.{}", class.name, block.name),
                );
                Unit {
                    kind: UnitKind::Function,
                    label,
                    text: block_text(body, &masked, &lines, block),
                }
            })
            .collect();
    }
    let range = functions
        .first()
        .map_or(0..lines.len(), |block| block.line + 1..block.end);
    let branches = branches(body, &masked, lines.get(range).unwrap_or_default());
    if branches.len() >= 2 {
        return branches;
    }
    vec![whole(body)]
}

/// The clause keywords that may follow an `if` at its own indentation.
const IF_CLAUSES: &[&str] = &["elif", "else"];
/// The clause keywords that may follow a `try` at its own indentation.
const TRY_CLAUSES: &[&str] = &["except", "else", "finally"];

/// Whether `line` opens a `match` statement: `match` is a soft keyword, so
/// the line must also end in the `:` that opens its block.
fn opens_match(masked: &[u8], line: &Line) -> bool {
    word_at(masked, line.head, "match")
        && masked
            .get(line.head..line.end)
            .and_then(|bytes| bytes.iter().rev().find(|byte| !byte.is_ascii_whitespace()))
            == Some(&b':')
}

/// The clause header lines of the compound statement opened at `lines[at]`,
/// and one past its last line; `None` when that line opens no `if`, `try` or
/// `match`. For `if`/`try` the clauses are the opener and every `elif` /
/// `else` / `except` / `finally` at its indentation; for `match` they are
/// the `case` lines (the `match` line itself is no clause).
fn clauses(masked: &[u8], lines: &[Line], at: usize) -> Option<(Vec<usize>, usize)> {
    let line = lines.get(at)?;
    let base = line.indent;
    let deeper = |index: usize| lines.get(index).is_some_and(|next| next.indent > base);
    let mut next = at + 1;
    if opens_match(masked, line) {
        let case_indent = lines.get(next).filter(|_| deeper(next))?.indent;
        let mut starts = Vec::new();
        while deeper(next) {
            if lines.get(next).is_some_and(|case| {
                case.indent == case_indent && word_at(masked, case.head, "case")
            }) {
                starts.push(next);
            }
            next += 1;
        }
        return Some((starts, next));
    }
    let continuations = if word_at(masked, line.head, "if") {
        IF_CLAUSES
    } else if word_at(masked, line.head, "try") {
        TRY_CLAUSES
    } else {
        return None;
    };
    let mut starts = vec![at];
    loop {
        while deeper(next) {
            next += 1;
        }
        let continues = lines.get(next).is_some_and(|clause| {
            clause.indent == base
                && continuations
                    .iter()
                    .any(|keyword| word_at(masked, clause.head, keyword))
        });
        if !continues {
            return Some((starts, next));
        }
        starts.push(next);
        next += 1;
    }
}

/// The index of the `:` ending the clause header whose first token is at
/// `head`: the first at bracket depth zero that is not a walrus `:=`.
fn header_colon(masked: &[u8], head: usize, end: usize) -> usize {
    let mut depth = 0i64;
    for index in head..end {
        match masked.get(index) {
            Some(b'(' | b'[' | b'{') => depth += 1,
            Some(b')' | b']' | b'}') => depth -= 1,
            Some(b':') if depth == 0 && masked.get(index + 1) != Some(&b'=') => return index,
            _ => {}
        }
    }
    end
}

/// One [`UnitKind::Branch`] unit per clause of every `if` / `try` / `match`
/// statement at the indentation of `lines`' first line, as the Rust half
/// takes every top-level `if` chain and `match`. A clause's text runs from
/// the end of the clause before it, so a comment between two clauses stays
/// with the clause it precedes rather than falling between units.
fn branches(text: &str, masked: &[u8], lines: &[Line]) -> Vec<Unit> {
    let Some(base) = lines.first().map(|line| line.indent) else {
        return Vec::new();
    };
    let mut units = Vec::new();
    let mut index = 0usize;
    while let Some(line) = lines.get(index) {
        let found = (line.indent == base)
            .then(|| clauses(masked, lines, index))
            .flatten();
        let Some((starts, end)) = found else {
            index += 1;
            continue;
        };
        for (position, start) in starts.iter().enumerate() {
            let Some(header) = lines.get(*start) else {
                continue;
            };
            let last = starts.get(position + 1).map_or(end, |next| *next) - 1;
            let from = if position == 0 {
                header.head
            } else {
                lines
                    .get(*start - 1)
                    .map_or(header.head, |before| before.end + 1)
            };
            let to = lines
                .get(last)
                .map_or(header.end, |last_line| last_line.end);
            let keyword: String = text
                .get(header.head..)
                .unwrap_or_default()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let colon = header_colon(masked, header.head, header.end);
            let condition = text
                .get(header.head + keyword.len()..colon)
                .unwrap_or_default();
            units.push(Unit {
                kind: UnitKind::Branch,
                label: short_label(&keyword, condition),
                text: text.get(from..to).unwrap_or_default().trim().to_owned(),
            });
        }
        index = end.max(index + 1);
    }
    units
}
