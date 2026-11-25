#!/usr/bin/env python3
# Trace: FR-226-AC-12
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX
"""Exercise a generated Quoin SACM export with an installed external importer."""

from __future__ import annotations

import collections
import json
import os
import pathlib
import select
import subprocess
import sys
import tempfile
import time
import xml.etree.ElementTree as ET


XMI = "http://www.omg.org/spec/XMI/20131001"
TOOL_RELEASE = "0.3.0-alpha.3"
MCP_VERSION = "assurance-forge 0.1.0 (MCP 2025-06-18)"
MCP_RESPONSE_TIMEOUT_SECONDS = 10
TOOL_VERSION_TIMEOUT_SECONDS = 5
RUN_PATH = "spec/evidence/runs/SUITE-1/2026-11-23T10~3A20~3A00Z.json"
ARTIFACT_ID = "artifact-43525f0578f3747767a425cd3d03ebf400e56d3da6ce128e65d10f2632f8ca7e"


def fail(message: str) -> None:
    raise RuntimeError(message)


def import_case(tool: pathlib.Path, fixture: pathlib.Path) -> tuple[subprocess.Popen[str], tempfile.TemporaryDirectory[str]]:
    settings_dir = tempfile.TemporaryDirectory(prefix="quoin-sacm-settings-")
    settings = pathlib.Path(settings_dir.name) / "settings.json"
    settings.write_text('{"mcp":{"enabled":true}}', encoding="utf-8")
    proc = subprocess.Popen(
        [str(tool), "--offline-project", str(fixture), "--settings", str(settings)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    return proc, settings_dir


class Mcp:
    def __init__(self, proc: subprocess.Popen[str]) -> None:
        self.proc = proc
        self.request_id = 0
        self.stdout_buffer = bytearray()

    def request(self, method: str, params: dict[str, object]) -> dict[str, object]:
        if self.proc.stdin is None or self.proc.stdout is None:
            fail("Assurance Forge MCP process has no JSON-RPC pipes")
        self.request_id += 1
        request_id = self.request_id
        request = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
        self.proc.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
        self.proc.stdin.flush()
        deadline = time.monotonic() + MCP_RESPONSE_TIMEOUT_SECONDS
        while True:
            newline = self.stdout_buffer.find(b"\n")
            if newline < 0:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    fail(
                        f"Assurance Forge MCP timed out after {MCP_RESPONSE_TIMEOUT_SECONDS}s "
                        f"waiting for {method} response"
                    )
                ready, _, _ = select.select([self.proc.stdout], [], [], remaining)
                if not ready:
                    fail(
                        f"Assurance Forge MCP timed out after {MCP_RESPONSE_TIMEOUT_SECONDS}s "
                        f"waiting for {method} response"
                    )
                chunk = os.read(self.proc.stdout.fileno(), 4096)
                if not chunk:
                    fail(f"Assurance Forge ended before JSON-RPC response to {method}")
                self.stdout_buffer.extend(chunk)
                continue
            line = bytes(self.stdout_buffer[:newline])
            del self.stdout_buffer[: newline + 1]
            response = json.loads(line.decode("utf-8"))
            if response.get("id") == request_id:
                if "error" in response:
                    fail(f"Assurance Forge JSON-RPC error: {response['error']}")
                result = response.get("result")
                if not isinstance(result, dict):
                    fail(f"Assurance Forge returned a non-object result for {method}")
                return result

    def start(self) -> None:
        self.request(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "quoin-sacm-interop", "version": "1"},
            },
        )
        if self.proc.stdin is None:
            fail("Assurance Forge MCP process has no stdin")
        self.proc.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n')
        self.proc.stdin.flush()

    def call(self, name: str, arguments: dict[str, object]) -> dict[str, object]:
        result = self.request("tools/call", {"name": name, "arguments": arguments})
        if result.get("isError"):
            fail(f"Assurance Forge tool {name} failed: {result}")
        content = result.get("content")
        if not isinstance(content, list) or not content or not isinstance(content[0], dict):
            fail(f"Assurance Forge tool {name} returned no text content")
        text = content[0].get("text")
        if not isinstance(text, str):
            fail(f"Assurance Forge tool {name} returned no text payload")
        payload = json.loads(text)
        if not isinstance(payload, dict):
            fail(f"Assurance Forge tool {name} returned a non-object payload")
        return payload

    def overview(self) -> dict[str, object]:
        return self.call("get_case_overview", {})

    def element(self, element_id: str) -> dict[str, object]:
        payload = self.call("get_element", {"id": element_id})
        element = payload.get("element")
        if not isinstance(element, dict):
            fail(f"Imported model has no element {element_id}")
        return payload


def stop(proc: subprocess.Popen[str], settings: tempfile.TemporaryDirectory[str]) -> None:
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)
    settings.cleanup()


def overview_for(tool: pathlib.Path, fixture: pathlib.Path) -> dict[str, object]:
    proc, settings = import_case(tool, fixture)
    try:
        client = Mcp(proc)
        client.start()
        return client.overview()
    finally:
        stop(proc, settings)


def expected_counts(fixture: pathlib.Path) -> collections.Counter[str]:
    root = ET.parse(fixture).getroot()
    return collections.Counter(
        value.split(":", 1)[-1].lower()
        for element in root.iter()
        if (value := element.attrib.get(f"{{{XMI}}}type")) is not None
        and value.split(":", 1)[-1] not in {"AssuranceCasePackage", "ArgumentPackage", "ArtifactPackage"}
    )


def validate_good_import(tool: pathlib.Path, fixture: pathlib.Path) -> dict[str, object]:
    overview = overview_for(tool, fixture)
    warnings = overview.get("load_warnings", [])
    if warnings:
        fail(f"Assurance Forge reported import warnings: {warnings}")
    actual = overview.get("element_counts_by_type")
    if not isinstance(actual, dict):
        fail("Assurance Forge returned no SACM element counts")
    expected = expected_counts(fixture)
    try:
        actual_counts = collections.Counter({kind: int(count) for kind, count in actual.items()})
    except (TypeError, ValueError):
        fail(f"Assurance Forge returned invalid SACM element counts: {actual}")
    if actual_counts != expected:
        fail(f"SACM element counts differ: XMI={dict(expected)}, Assurance Forge={dict(actual_counts)}")
    if overview.get("element_count") != sum(expected.values()):
        fail(f"total SACM element count differs: XMI={sum(expected.values())}, Assurance Forge={overview.get('element_count')}")
    for kind, count in expected.items():
        if actual.get(kind) != count:
            fail(f"{kind} count differs: XMI={count}, Assurance Forge={actual.get(kind)}")
    relationships = sum(count for kind, count in expected.items() if kind.startswith("asserted"))
    if relationships != sum(
        int(count) for kind, count in actual.items() if kind.startswith("asserted")
    ):
        fail("AssertedRelationships count differs from XMI")
    if expected["claim"] != int(actual.get("claim", -1)):
        fail("Claim count differs from XMI")
    if expected["argumentreasoning"] != int(actual.get("argumentreasoning", -1)):
        fail("ArgumentReasoning count differs from XMI")
    if expected["artifact"] != int(actual.get("artifact", -1)):
        fail("Artifact count differs from XMI")

    proc, settings = import_case(tool, fixture)
    try:
        client = Mcp(proc)
        client.start()
        for element_id in ("G2", "G3"):
            element = client.element(element_id)["element"]
            if element.get("assertion_declaration") != "needsSupport":
                fail(f"{element_id} was imported with an asserting declaration: {element}")
        g3 = client.element("G3")["element"]
        if g3.get("undeveloped") is not True:
            fail(f"undeveloped Claim G3 lost its state: {g3}")
        stale = client.element("Sn2")
        relation = next(
            (
                candidate
                for candidate in stale.get("relationships_from_here", [])
                if isinstance(candidate, dict) and candidate.get("type") == "assertedevidence"
            ),
            None,
        )
        if not isinstance(relation, dict) or relation.get("assertion_declaration") != "needsSupport":
            fail(f"stale evidence relationship lost its needsSupport declaration: {stale}")
        if relation.get("source_refs") != ["Sn2"] or relation.get("target_refs") != ["G4"]:
            fail(f"stale evidence relationship endpoints changed: {relation}")
        group = client.element(f"{ARTIFACT_ID}-record")["element"]
        if group.get("type") != "artifactgroup":
            fail(f"ArtifactGroup was dropped or changed type: {group}")
        resource = client.element(f"{ARTIFACT_ID}-location-1")["element"]
        if resource.get("type") != "resource" or resource.get("name") != RUN_PATH:
            fail(f"Resource name/path was changed: {resource}")
    finally:
        stop(proc, settings)
    return overview


def verify_dangling_reference_is_rejected(tool: pathlib.Path, fixture: pathlib.Path) -> None:
    contents = fixture.read_text(encoding="utf-8")
    original = 'source="Sn1" target="G4"'
    replacement = 'source="Sn1" target="MISSING-GOAL"'
    if contents.count(original) != 1:
        fail("could not create the deterministic dangling-reference fixture")
    with tempfile.TemporaryDirectory(prefix="quoin-sacm-broken-") as directory:
        broken = pathlib.Path(directory) / "broken.sacm.xml"
        broken.write_text(contents.replace(original, replacement, 1), encoding="utf-8")
        overview = overview_for(tool, broken)
    warnings = overview.get("load_warnings")
    if not isinstance(warnings, list) or not any(
        isinstance(warning, str) and "MISSING-GOAL" in warning for warning in warnings
    ):
        fail(f"the importer did not report the dangling target: {warnings}")
    try:
        if warnings:
            fail(f"Assurance Forge reported import warnings: {warnings}")
    except RuntimeError:
        return
    fail("the interop acceptance predicate accepted a dangling reference")


def main() -> int:
    tool_name = os.environ.get("QUOIN_SACM_TOOL")
    if not tool_name:
        fail("test-sacm-interop: QUOIN_SACM_TOOL must name the installed Assurance Forge MCP executable")
    fixture_name = os.environ.get("QUOIN_SACM_INTEROP_FIXTURE")
    if not fixture_name:
        fail("test-sacm-interop: QUOIN_SACM_INTEROP_FIXTURE must name the generated Quoin XMI")
    tool = pathlib.Path(tool_name)
    fixture = pathlib.Path(fixture_name)
    if not tool.is_file() or not os.access(tool, os.X_OK):
        fail(f"test-sacm-interop: QUOIN_SACM_TOOL is not an executable file: {tool}")
    if not fixture.is_file():
        fail(f"test-sacm-interop: generated fixture is missing: {fixture}")
    try:
        version_result = subprocess.run(
            [tool, "--version"],
            check=True,
            capture_output=True,
            text=True,
            timeout=TOOL_VERSION_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        fail(f"Assurance Forge --version timed out after {TOOL_VERSION_TIMEOUT_SECONDS}s")
    version = (version_result.stdout + version_result.stderr).strip()
    if version != MCP_VERSION:
        fail(f"expected Assurance Forge MCP {MCP_VERSION}, got {version!r}")
    overview = validate_good_import(tool, fixture)
    verify_dangling_reference_is_rejected(tool, fixture)
    print(f"Assurance Forge {TOOL_RELEASE} (MIT), MCP {version}")
    print(f"Imported unchanged SACM 2.3 XMI without warnings: {overview.get('element_counts_by_type')}")
    print("Verified open/undeveloped Claims, stale evidence endpoints/declaration, ArtifactGroup presence, and Resource path.")
    print("Confirmed a dangling target is reported as a load warning and rejected.")
    print("ArtifactGroup member ids are not exposed by MCP; Assurance Forge release test covers group-reference round-trip.")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, RuntimeError, json.JSONDecodeError, ET.ParseError) as error:
        print(f"test-sacm-interop: {error}", file=sys.stderr)
        sys.exit(1)
