#!/usr/bin/env python3
"""Run behavioral cargo-mutants checks over the five Wave 2 kernels."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import wave2_evidence as evidence


KERNELS = [
    ("approval-decision", "quoin-change-assurance", "crates/quoin-change-assurance/src/verify/approval.rs"),
    ("discharge", "quoin-core", "crates/quoin-core/src/ops/evidence/discharge.rs"),
    ("evidence-binding", "quoin-evidence", "crates/quoin-evidence/src/store/graph.rs"),
    ("risk-completeness", "quoin-risk-advice", "crates/quoin-risk-advice/src/summary.rs"),
    ("class-floor", "quoin-auditor", "crates/quoin-auditor/src/audit/class.rs"),
]


def raw_counts(path: Path) -> tuple[dict[str, int], bool]:
    document = json.loads(path.read_text(encoding="utf-8"))
    counts = {"killed": 0, "survived": 0, "unviable": 0}
    baseline_failed = False
    for outcome in document.get("outcomes", []):
        summary = outcome.get("summary")
        if summary == "CaughtMutant":
            counts["killed"] += 1
        elif summary in {"MissedMutant", "Timeout"}:
            counts["survived"] += 1
        elif summary == "Unviable":
            counts["unviable"] += 1
        elif outcome.get("scenario") == "Baseline" and summary != "Success":
            baseline_failed = True
    return counts, baseline_failed


def raw_survivors(path: Path) -> list[dict]:
    document = json.loads(path.read_text(encoding="utf-8"))
    survivors = []
    for outcome in document.get("outcomes", []):
        if outcome.get("summary") not in {"MissedMutant", "Timeout"}:
            continue
        mutant = outcome.get("scenario", {}).get("Mutant", {})
        span = mutant.get("span", {}).get("start", {})
        survivor = {"mutation": mutant.get("name") or outcome.get("summary")}
        if span.get("line") is not None:
            survivor["line"] = span["line"]
        survivors.append(survivor)
    return survivors


def main() -> int:
    evidence.prepare()
    mutants_version, version_output = evidence.version(["cargo", "mutants", "--version"])
    entries: list[dict] = []
    disposition: list[dict] = []
    failed = version_output is None
    summaries: dict[str, dict] = {}

    for name, package, source in KERNELS:
        output = evidence.ARTIFACTS / "mutants" / name
        output.mkdir(parents=True, exist_ok=True)
        command = ["cargo", "mutants", "--manifest-path", str(evidence.ROOT / "rust" / "Cargo.toml"),
                   "--package", package, "--file", source, "--output", str(output),
                   "--in-place", "--cargo-arg", "--locked"]
        env = os.environ.copy()
        env.pop("CARGO_MUTANTS_JOBS", None)
        env["SPEC_OBJECTS_SAFETY_DIR"] = "/home/peter/dev/spec-objects-safety"
        log, status, error = evidence.execute(f"mutation-{name}", command, env)
        outcomes = output / "mutants.out" / "outcomes.json"
        config = {"kernel": name, "source": source, "tool_version": mutants_version,
                  "command_log": log, "raw_outcomes": str(outcomes)}
        if error:
            config.update({"execution_state": "not-executed", "reason": error})
            entries.append(evidence.entry(name, "error", config))
            failed = True
            continue
        if not outcomes.is_file():
            config.update({"execution_state": "not-executed", "reason": "cargo-mutants did not retain outcomes.json"})
            entries.append(evidence.entry(name, "error", config))
            failed = True
            continue
        counts, baseline_failed = raw_counts(outcomes)
        summaries[name] = {"package": package, "source": source, **counts,
                           "cargo_mutants_exit": status, "baseline_failed": baseline_failed,
                           "raw_outcomes": str(outcomes)}
        config.update({"execution_state": "executed", **{key: str(value) for key, value in counts.items()},
                       "baseline_failed": str(baseline_failed)})
        run_failed = status != 0 or baseline_failed or not any(counts.values())
        survivors = raw_survivors(outcomes)
        record_entry = evidence.entry(name, "fail" if run_failed or survivors else "pass", config)
        viable = counts["killed"] + counts["survived"]
        if viable:
            record_entry["score"] = counts["killed"] / viable
            record_entry["metric"] = "mutation-score"
        if survivors:
            record_entry["survivors"] = survivors[:25]
        entries.append(record_entry)
        failed |= run_failed
        raw = json.loads(outcomes.read_text(encoding="utf-8"))
        for item in raw.get("outcomes", []):
            if item.get("summary") not in {"MissedMutant", "Timeout"}:
                continue
            mutant = item.get("scenario", {}).get("Mutant", {})
            disposition.append({"kernel": name, "file": mutant.get("file"),
                                "function": mutant.get("function", {}).get("function_name"),
                                "line": mutant.get("span", {}).get("start", {}).get("line"),
                                "mutation": mutant.get("name"), "raw_outcome": item.get("summary"),
                                "human_review_disposition": None})

    summary_path = evidence.ARTIFACTS / "mutation-summary.json"
    summary_path.write_text(json.dumps({"tool": mutants_version, "kernels": summaries}, indent=2) + "\n", encoding="utf-8")
    disposition_path = evidence.ARTIFACTS / "survivor-dispositions.json"
    disposition_path.write_text(json.dumps({"status": "pending-human-review", "survivors": disposition}, indent=2) + "\n", encoding="utf-8")
    results = evidence.ARTIFACTS / "mutation-results.json"
    evidence.write_entries(results, entries)
    record_status = evidence.record("SUITE-005", mutants_version if mutants_version != "unavailable" else "cargo-mutants unavailable",
                                   results, "mutation")
    if record_status:
        print(f"SUITE-005 evidence recording failed; see {evidence.ARTIFACTS / 'mutation-record.log'}", file=sys.stderr)
        failed = True
    print(f"Raw mutation outcomes: {evidence.ARTIFACTS / 'mutants'}")
    print(f"Human survivor dispositions required: {len(disposition)}; see {disposition_path}")
    print(f"Mutation summary: {summary_path}")
    return 1 if failed or disposition else 0


if __name__ == "__main__":
    raise SystemExit(main())
