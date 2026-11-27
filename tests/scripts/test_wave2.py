#!/usr/bin/env python3
"""Focused tests for Wave 2 outcome and evidence-record handling."""

from __future__ import annotations

import contextlib
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import real_tool_test
import wave2_evidence as evidence
import behavioral_mutation_test


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
            patch.object(real_tool_test.shutil, "which", return_value=None),
            patch.object(evidence, "write_entries", side_effect=lambda _path, entries: captured.append(entries)),
            patch.object(evidence, "record", return_value=0),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(real_tool_test.run(), 1)

        self.assertEqual(len(captured), 1)
        entries = captured[0]
        self.assertEqual(len(entries), 6)
        for item in entries:
            self.assertEqual(item["outcome"], "error")
            self.assertEqual(item["config"]["execution_state"], "not-executed")
        trace_ids = {item["symbol"]: item.get("traceIds", []) for item in entries}
        self.assertEqual(
            trace_ids["case-approval-real-acceptance"],
            ["FR-225-AC-1", "FR-225-AC-4", "FR-226-AC-9"],
        )
        self.assertEqual(trace_ids["case-approval-real-refusal"], ["FR-225-AC-2"])
        self.assertEqual(trace_ids["sacm-published-metamodel"], ["FR-226-AC-11"])
        self.assertEqual(trace_ids["sacm-assurance-forge-interop"], ["FR-226-AC-12"])

    def test_unknown_mutation_outcomes_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "outcomes.json"
            path.write_text(
                '{"outcomes":[{"scenario":"Baseline","summary":"Success"},'
                '{"scenario":{"Mutant":{}},"summary":"NewOutcome"}]}',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "unrecognized summary"):
                behavioral_mutation_test.raw_counts(path)

    def test_empty_mutation_outcomes_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "outcomes.json"
            path.write_text('{"outcomes":[]}', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "no outcome rows"):
                behavioral_mutation_test.raw_counts(path)

    def test_operator_mutant_with_null_function_and_span_is_retained(self) -> None:
        outcome = {
            "scenario": {
                "Mutant": {
                    "name": "operator_mutant",
                    "file": "src/kernel.rs",
                    "function": None,
                    "span": {"start": None},
                }
            },
            "summary": "MissedMutant",
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "outcomes.json"
            path.write_text(json.dumps({"outcomes": [outcome]}), encoding="utf-8")
            self.assertEqual(
                behavioral_mutation_test.raw_counts(path),
                ({"killed": 0, "survived": 1, "unviable": 0}, False),
            )
            self.assertEqual(
                behavioral_mutation_test.raw_survivors(path),
                [{"mutation": "operator_mutant"}],
            )
        self.assertEqual(
            behavioral_mutation_test.mutant_details(outcome),
            {"file": "src/kernel.rs", "function": None, "line": None, "mutation": "operator_mutant"},
        )

    def test_non_object_mutation_document_fails_closed(self) -> None:
        for document in ("null", "[]"):
            with self.subTest(document=document), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "outcomes.json"
                path.write_text(document, encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "root is not an object"):
                    behavioral_mutation_test.raw_counts(path)

    def test_non_string_mutant_summary_fails_closed(self) -> None:
        for summary in ([], {}):
            with self.subTest(summary=summary), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "outcomes.json"
                path.write_text(
                    json.dumps({"outcomes": [{"scenario": {"Mutant": {}}, "summary": summary}]}),
                    encoding="utf-8",
                )
                with self.assertRaisesRegex(ValueError, "summary is not a string"):
                    behavioral_mutation_test.raw_counts(path)

    @pytest.mark.trace("NFR-042-M-1")
    def test_all_mutation_kernel_entries_bind_their_criterion(self) -> None:
        entries = [
            behavioral_mutation_test.mutation_entry(name, "pass", {})
            for name, _package, _source, _test_args in behavioral_mutation_test.KERNELS
        ]
        self.assertEqual(len(entries), 5)
        self.assertEqual(
            {entry["symbol"] for entry in entries},
            {kernel[0] for kernel in behavioral_mutation_test.KERNELS},
        )
        self.assertTrue(all(entry["outcome"] == "pass" for entry in entries))
        self.assertTrue(
            all(entry["traceIds"] == ["NFR-042-M-1"] for entry in entries),
        )

    def test_record_failure_keeps_the_real_tool_lane_failed(self) -> None:
        captured: list[list[dict]] = []
        with (
            patch.dict(os.environ, {}, clear=True),
            patch.object(evidence, "prepare"),
            patch.object(evidence, "version", return_value=("unavailable", None)),
            patch.object(real_tool_test.shutil, "which", return_value=None),
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
