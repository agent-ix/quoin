"""Live instruction interpretation controls, separate from installed-binary smoke.

Uses synthetic command results to exercise only matrix interpretation and fallback
policy. Full healthy/hollow repository audits are separate behavioral evidence.
Requires an authenticated Claude CLI. --skill-root permits a regression control
against an older instruction tree without modifying the author tree.
"""

import argparse
import json
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument(
    "--skill-root",
    type=Path,
    default=Path(__file__).resolve().parents[2] / "skills/gap-analysis",
)
args = parser.parse_args()
instructions = "\n\n".join(
    p.read_text()
    for p in [
        args.skill_root / "SKILL.md",
        args.skill_root / "references/step-3-matrix-verification.md",
        args.skill_root / "references/step-6-specreview-artifact.md",
    ]
)
cases = [
    {
        "id": "statuses",
        "statuses": [
            "qualified-record",
            "retired",
            "tagged",
            "untagged",
            "tagged-by-ignored-test",
            "method-without-symbol",
        ],
        "diagnostics": [],
    },
    {"id": "healthy", "statuses": ["tagged"], "diagnostics": []},
    {"id": "retained", "statuses": ["qualified-record", "retired"], "diagnostics": []},
    {"id": "zero", "statuses": [], "diagnostics": []},
    {
        "id": "command-failure",
        "matrix_exit": 2,
        "static_fallback_exit": 2,
        "stderr": "source extraction failed; usable traceability model is installed",
    },
    {
        "id": "partial-extraction",
        "statuses": ["tagged"],
        "diagnostics": [
            {
                "reason": "source-language-unavailable",
                "guidance": "The build could not extract one required language; extraction is incomplete.",
            }
        ],
    },
]
prompt = (
    """Execute the matrix interpretation, fallback and verdict policy in the supplied skill instructions against these SYNTHETIC command-result fixtures. This is a bounded instruction behavior test, not a repository audit. No tools, file writes, or actual audit verdicts. Assume reverse-gap/stub inspection clean and no run evidence. Do not infer policy for an unknown status from its English name: if the supplied instructions do not describe it, list it as unsupported and do not claim PASS. For an absent optional diagnostic field assume none. Set grep_authorized only if the supplied instructions authorize fallback under that case. Return ONLY JSON: {"results":[{"id":str,"verdict":"PASS"|"CONDITIONAL"|"FAIL","unsupported":[str],"source_test_count":int,"retained_count":int,"retired_count":int,"grep_authorized":bool}]}. Count only tagged criteria as source tests, keep ignored-only separate. Do not silently repair or modernize the supplied instructions.\nINSTRUCTIONS:\n"""
    + instructions
    + "\nCASES:\n"
    + json.dumps(cases)
)
result = subprocess.run(
    ["claude", "-p", "--max-turns", "1", prompt],
    capture_output=True,
    text=True,
    check=False,
    timeout=180,
)
if result.returncode:
    raise RuntimeError(result.stderr or result.stdout)
output = result.stdout.strip()
if output.startswith("```"):
    output = "\n".join(output.splitlines()[1:-1])
rows = {r["id"]: r for r in json.loads(output)["results"]}
assert set(rows) == {c["id"] for c in cases}, rows
for name in ["statuses", "retained"]:
    assert rows[name]["unsupported"] == [], rows[name]
    assert rows[name]["retained_count"] == 1 and rows[name]["retired_count"] == 1, rows[
        name
    ]
assert (
    rows["statuses"]["verdict"] == "FAIL" and rows["statuses"]["source_test_count"] == 1
), rows["statuses"]
assert rows["healthy"]["verdict"] == "PASS", rows["healthy"]
assert rows["retained"]["source_test_count"] == 0, rows["retained"]
for name in ["zero", "command-failure", "partial-extraction"]:
    assert rows[name]["verdict"] != "PASS" and not rows[name]["grep_authorized"], rows[
        name
    ]
print(json.dumps(rows, indent=2))
print("Six instruction behavior scenarios passed")
