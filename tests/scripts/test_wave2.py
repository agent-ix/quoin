#!/usr/bin/env python3
"""Focused tests for Wave 2 outcome and evidence-record handling."""

from __future__ import annotations

import contextlib
import io
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

import real_tool_test
import wave2_evidence as evidence


class Wave2EvidenceTests(unittest.TestCase):
    def test_execution_outcome_classification_preserves_failure_and_not_executed(self) -> None:
        self.assertEqual(evidence.classify_execution(0), ("pass", "executed"))
        self.assertEqual(evidence.classify_execution(17), ("fail", "executed"))
        self.assertEqual(evidence.classify_execution(None, "missing tool"), ("error", "not-executed"))

    def test_only_ignored_rust_test_trace_annotations_are_selected(self) -> None:
        source = """\
#[trace("FR-001-AC-1")]
#[test]
fn ordinary_test() {}

#[trace("FR-002-AC-1", "FR-002-AC-2")]
#[ignore = "external tool"]
#[test]
fn ignored_test() {}

#[trace(
    "FR-003-AC-1",
)]
#[ignore]
#[test]
fn another_ignored_test() {}
"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.rs"
            path.write_text(source, encoding="utf-8")
            self.assertEqual(
                real_tool_test.ignored_test_trace_ids(path),
                ["FR-002-AC-1", "FR-002-AC-2", "FR-003-AC-1"],
            )
            self.assertEqual(
                real_tool_test.ignored_test_trace_ids(path, {"ignored_test"}),
                ["FR-002-AC-1", "FR-002-AC-2"],
            )

    def test_unavailable_tools_are_error_not_executed_without_empty_osv_date(self) -> None:
        captured: list[list[dict]] = []
        with (
            patch.dict(os.environ, {}, clear=True),
            patch.object(evidence, "prepare"),
            patch.object(evidence, "version", return_value=("unavailable", None)),
            patch.object(evidence, "write_entries", side_effect=lambda _path, entries: captured.append(entries)),
            patch.object(evidence, "record", return_value=0),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(real_tool_test.run(), 1)

        self.assertEqual(len(captured), 1)
        entries = captured[0]
        self.assertEqual(len(entries), 3)
        for item in entries:
            self.assertEqual(item["outcome"], "error")
            self.assertEqual(item["config"]["execution_state"], "not-executed")
        osv = next(item for item in entries if item["symbol"] == "osv-authentic-report")
        self.assertNotIn("osv_mirror_date", osv["config"])
        self.assertEqual(osv["config"]["artifact_id"], evidence.RUN_ID)

    def test_record_failure_keeps_the_real_tool_lane_failed(self) -> None:
        captured: list[list[dict]] = []
        with (
            patch.dict(os.environ, {}, clear=True),
            patch.object(evidence, "prepare"),
            patch.object(evidence, "version", return_value=("unavailable", None)),
            patch.object(evidence, "write_entries", side_effect=lambda _path, entries: captured.append(entries)),
            patch.object(evidence, "record", return_value=9),
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            self.assertEqual(real_tool_test.run(), 1)
        self.assertEqual(len(captured), 1)

    def test_run_timestamp_reservations_prevent_same_millisecond_collision(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            original_target, original_root = evidence.TARGET, evidence.ROOT
            evidence.TARGET = Path(directory) / "target"
            evidence.ROOT = Path(directory) / "repo"
            try:
                first = evidence.run_timestamp("SUITE-004")
                second = evidence.run_timestamp("SUITE-004")
            finally:
                evidence.TARGET, evidence.ROOT = original_target, original_root
        self.assertNotEqual(first, second)
        self.assertRegex(first, r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$")


if __name__ == "__main__":
    unittest.main()
