"""Exercise real matrix binding mutations; requires quoin, quire, and installed modules."""
import json
from pathlib import Path
import subprocess
import tempfile


def run(*args):
    result = subprocess.run(args, capture_output=True, text=True, check=False)
    if result.returncode:
        raise RuntimeError(f'{args}: exit {result.returncode}: {result.stderr}')
    return result.stdout


with tempfile.TemporaryDirectory(prefix='gap-analysis-contract-') as directory:
    root = Path(directory)
    (root / 'spec').mkdir()
    (root / 'tests').mkdir()
    (root / 'spec' / 'FR-001.md').write_text('''---
id: FR-001
title: Double an integer
type: FR
---
# FR-001: Double an integer

## Description

The system SHALL return twice the input integer.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-001-AC-1 | For every integer n, double(n) returns n * 2. | Test |
''')
    run('git', '-C', directory, 'init', '-q')
    run('git', '-C', directory, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'add', 'spec')
    run('git', '-C', directory, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture')
    cases = {
        'healthy': ('@pytest.mark.trace("FR-001-AC-1")', 'assert 2 * 3 == 6', 'tagged'),
        'missing': ('', 'assert 2 * 3 == 6', 'untagged'),
        'name-only': ('# FR-001-AC-1', 'assert 2 * 3 == 6', 'untagged'),
        'ignored': ('@pytest.mark.skip(reason="fixture")\n@pytest.mark.trace("FR-001-AC-1")', 'assert 2 * 3 == 6', 'tagged-by-ignored-test'),
        'hollow': ('@pytest.mark.trace("FR-001-AC-1")', 'pass', 'tagged'),
        'stale': ('@pytest.mark.trace("FR-999-AC-1")', 'assert 2 * 3 == 6', 'untagged'),
    }
    for name, (marker, body, expected) in cases.items():
        (root / 'tests' / 'test_double.py').write_text(f'import pytest\n\n{marker}\ndef test_fr_001_ac_1_double():\n    {body}\n')
        static = json.loads(run('quire', 'matrix', '--scope', directory, '--format', 'json'))
        evidence = json.loads(run('quoin', 'matrix', '--repo', directory, '--json'))
        a = static['coverage_matrix'][0]['criteria'][0]['status']
        b = evidence['requirements'][0]['criteria'][0]['static_status']
        assert a == b == expected, (name, a, b, expected)
        if name == 'stale':
            coverage = json.loads(run('quire', 'coverage', '--scope', directory, '--json'))
            assert 'FR-999-AC-1' in json.dumps(coverage['untracked_symbols']), coverage
        print(f'{name}: {a}; both commands agree')
    # A tagged hollow test is deliberately still tagged: the skill must inspect its body.
    review = root / 'reviews' / 'gap.md'
    review.parent.mkdir()
    review.write_text('''---
id: SR-001
title: Gap analysis fixture
type: SpecReview
analysis: gap-analysis
scope: "spec/, tests/"
review_set: subset
---
## Summary

Controlled fixture audit; plan completion was not assessed.

## Verdict

**FAIL** — the test body is hollow.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Tagged test has no behavioral assertion | tests/test_double.py |

## Coverage

Plan completion: not assessed
Semantic review: skipped
''')
    run('quire', 'validate', '--scope', directory, 'reviews/gap.md')
    print('review artifact: validates')
