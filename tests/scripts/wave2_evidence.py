#!/usr/bin/env python3
"""Shared result retention helpers for the Wave 2 maintenance lanes."""

from __future__ import annotations

import json
import os
import re
import secrets
import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "rust" / "target"))
ARTIFACTS_ROOT = Path(os.environ.get("WAVE2_EVIDENCE_DIR", TARGET / "wave2-evidence-mutation"))
RUN_ID = (datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
          + f"-{os.getpid()}-{secrets.token_hex(6)}")
ARTIFACTS = ARTIFACTS_ROOT / RUN_ID
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
        if env is not None:
            retained_env = {key: env[key] for key in
                            ("SPEC_OBJECTS_SAFETY_DIR", "SPEC_ARTIFACTS_PROCESS_DIR") if env.get(key)}
            stream.write(f"external_fixture_roots: {json.dumps(retained_env)}\n")
        try:
            result = subprocess.run(command, cwd=ROOT / "rust", env=env, stdout=stream, stderr=subprocess.STDOUT, check=False)
        except FileNotFoundError as error:
            stream.write(f"not executed: {error}\n")
            return str(log), None, str(error)
    return str(log), result.returncode, ""


def entry(symbol: str, outcome: str, config: dict[str, str], trace_ids: list[str] | None = None) -> dict:
    config.setdefault("artifact_id", RUN_ID)
    item: dict = {"symbol": symbol, "outcome": outcome, "config": config}
    if trace_ids:
        item["traceIds"] = trace_ids
    return item


def classify_execution(status: int | None, error: str = "") -> tuple[str, str]:
    """Return the recorded outcome and execution state for a tool invocation."""
    if error or status is None:
        return "error", "not-executed"
    return ("pass" if status == 0 else "fail"), "executed"


def write_entries(path: Path, entries: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"entries": entries}, indent=2) + "\n", encoding="utf-8")


def record(suite: str, tool: str, results: Path, name: str) -> int:
    log = ARTIFACTS / f"{name}-record.log"
    command = [str(QUOIN), "evidence", "record", "--repo", str(ROOT), "--suite", suite,
               "--kind", "Integration", "--adapter", "entries", "--tool", tool,
               "--results", str(results), "--timestamp", run_timestamp(suite)]
    with log.open("w", encoding="utf-8") as stream:
        stream.write(f"command: {json.dumps(command)}\ncwd: {ROOT}\n")
        try:
            result = subprocess.run(command, cwd=ROOT, text=True, stdout=stream, stderr=subprocess.STDOUT, check=False)
        except FileNotFoundError as error:
            stream.write(f"not executed: {error}\n")
            return 127
    return result.returncode


def run_timestamp(suite: str) -> str:
    """Reserve a unique canonical millisecond timestamp for an append-only run."""
    current = datetime.now(timezone.utc)
    current = current.replace(microsecond=(current.microsecond // 1000) * 1000)
    reservations = TARGET / "wave2-timestamp-reservations" / suite
    reservations.mkdir(parents=True, exist_ok=True)
    store_dir = ROOT / "spec" / "evidence" / "runs" / suite
    while True:
        stamp = current.isoformat(timespec="milliseconds").replace("+00:00", "Z")
        safe_stamp = re.sub(r"[^A-Za-z0-9.-]", lambda match: f"~{ord(match.group()):02X}", stamp)
        record_path = store_dir / f"{safe_stamp}.json"
        reservation = reservations / f"{safe_stamp}.reserve"
        if record_path.exists():
            current += timedelta(milliseconds=1)
            continue
        try:
            descriptor = os.open(reservation, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
        except FileExistsError:
            current += timedelta(milliseconds=1)
            continue
        os.close(descriptor)
        return stamp
