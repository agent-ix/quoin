#!/usr/bin/env python3
"""Apply a bounded set of design edits inside the configured design scope."""

import argparse
import json
import os
import sys
from pathlib import Path


MAX_INPUT = 1_048_576
MAX_EDITS = 64


def fail(code: str) -> int:
    print(json.dumps({"ok": False, "code": code}, separators=(",", ":")))
    return 1


def under(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scope", required=True)
    parser.add_argument("--module", action="append", default=[])
    args = parser.parse_args()

    scope = Path(args.scope)
    modules = [Path(path) for path in args.module]
    if not scope.is_absolute() or not modules or any(not path.is_absolute() for path in modules):
        return fail("rules_modified")
    try:
        scope = scope.resolve(strict=True)
        modules = [path.resolve(strict=True) for path in modules]
    except OSError:
        return fail("rules_modified")
    if not scope.is_dir() or any(not path.is_dir() for path in modules):
        return fail("rules_modified")

    raw = sys.stdin.buffer.read(MAX_INPUT + 1)
    if len(raw) > MAX_INPUT:
        return fail("rules_modified")
    try:
        request = json.loads(raw)
    except (json.JSONDecodeError, UnicodeDecodeError):
        return fail("rules_modified")
    if not isinstance(request, dict) or set(request) != {"edits"}:
        return fail("rules_modified")
    edits = request.get("edits")
    if not isinstance(edits, list) or not edits or len(edits) > MAX_EDITS:
        return fail("rules_modified")

    prepared: list[tuple[Path, str]] = []
    seen: set[Path] = set()
    for edit in edits:
        if not isinstance(edit, dict) or set(edit) != {"path", "content"}:
            return fail("rules_modified")
        relative = edit["path"]
        content = edit["content"]
        if not isinstance(relative, str) or not relative or not isinstance(content, str):
            return fail("rules_modified")
        relative_path = Path(relative)
        if relative_path.is_absolute() or ".." in relative_path.parts:
            return fail("rules_modified")
        try:
            target = (scope / relative_path).resolve(strict=False)
        except OSError:
            return fail("rules_modified")
        if not under(target, scope) or any(under(target, module) for module in modules):
            return fail("rules_modified")
        if target in seen:
            return fail("rules_modified")
        seen.add(target)
        prepared.append((target, content))

    try:
        for target, content in prepared:
            target.parent.mkdir(parents=True, exist_ok=True)
            temporary = target.with_name(f".{target.name}.design-edit-tmp")
            temporary.write_text(content, encoding="utf-8")
            os.replace(temporary, target)
    except OSError:
        return fail("rules_modified")
    print(json.dumps({"ok": True, "written": len(prepared)}, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
