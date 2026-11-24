#!/usr/bin/env python3
"""Shared result retention helpers for the Wave 2 maintenance lanes."""

from __future__ import annotations

import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "rust" / "target"))
ARTIFACTS = Path(os.environ.get("WAVE2_EVIDENCE_DIR", TARGET / "wave2-evidence-mutation"))
QUOIN = TARGET / "debug" / ("quoin.exe" if os.name == "nt" else "quoin")


def prepare() -> None:
    ARTIFACTS.mkdir(parents=True, exist_ok=True)


def version(command: list[str]) -> tuple[str, str | None]:
    try:
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    except FileNotFoundError:
        return "unavailable", None
    output = (result.stdout + result.stderr).strip()
    return output or f"exit {result.returncode}", output


def execute(name: str, command: list[str], env: dict[str, str] | None = None) -> tuple[str, int | None, str]:
    log = ARTIFACTS / f"{name}.log"
    with log.open("w", encoding="utf-8") as stream:
        stream.write(f"command: {json.dumps(command)}\ncwd: {ROOT / 'rust'}\n")
        try:
            result = subprocess.run(command, cwd=ROOT / "rust", env=env, stdout=stream, stderr=subprocess.STDOUT, check=False)
        except FileNotFoundError as error:
            stream.write(f"not executed: {error}\n")
            return str(log), None, str(error)
    return str(log), result.returncode, ""


def entry(symbol: str, outcome: str, config: dict[str, str], trace_ids: list[str] | None = None) -> dict:
    item: dict = {"symbol": symbol, "outcome": outcome, "config": config}
    if trace_ids:
        item["traceIds"] = trace_ids
    return item


def write_entries(path: Path, entries: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"entries": entries}, indent=2) + "\n", encoding="utf-8")


def record(suite: str, tool: str, results: Path, name: str) -> int:
    log = ARTIFACTS / f"{name}-record.log"
    command = [str(QUOIN), "evidence", "record", "--repo", str(ROOT), "--suite", suite,
               "--kind", "Integration", "--adapter", "entries", "--tool", tool,
               "--results", str(results), "--timestamp", run_timestamp()]
    with log.open("w", encoding="utf-8") as stream:
        stream.write(f"command: {json.dumps(command)}\ncwd: {ROOT}\n")
        try:
            result = subprocess.run(command, cwd=ROOT, text=True, stdout=stream, stderr=subprocess.STDOUT, check=False)
        except FileNotFoundError as error:
            stream.write(f"not executed: {error}\n")
            return 127
    return result.returncode


def run_timestamp() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
