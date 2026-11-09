# Python / hypothesis templates

File: `tests/props/test_fr_NNN.py`.
Every emitted test runs — there is no inert form.

## Anatomy

```python
# tests/props/test_fr_012.py
import pytest
from hypothesis import given, strategies as st

from quoin.catalog import Entry, detect_duplicates

# grounded in step 2 from `## Inputs` + the signature at src/quoin/catalog.py:88
entries = st.builds(
    Entry,
    kind=st.from_regex(r"\A[a-z]{1,12}\Z"),
    name=st.from_regex(r"\A[a-z]{1,12}\Z"),
)


@pytest.mark.trace("FR-012-AC-1", "TC-345")
@given(catalog=st.lists(entries, max_size=32))
def test_fr_012_ac_1_duplicate_modules_sorted(catalog):
    """Declaring modules are listed in sorted order.

    spec-correctness: row=FR-012-AC-1 property=ordering extraction=extractable origin=regex review=none
    """
    for dup in detect_duplicates(catalog).duplicates:
        assert dup.modules == sorted(dup.modules)
```

Both tag carriers: the `@pytest.mark.trace("FR-012-AC-1", "TC-345")` marker (the criterion id
plus the `spec/matrix.md` row's own `TC-` id; a docstring id binds nothing; the marker is
registered by the `ix_trace_py` pytest plugin, or by a `markers = trace(*ids)` entry in the pytest
config, otherwise `--strict-markers` rejects it), and `fr_012_ac_1_`
in the function name.

## Family bodies

```python
# round-trip
assert parse(render(x)) == x

# round-trip, lossy
assert normalize(parse(render(x))) == normalize(x)

# idempotence — seed with f's own outputs
seeded = st.one_of(raw_configs, raw_configs.map(normalize))
assert normalize(normalize(x)) == normalize(x)

# invariant
assert len(resolve(x)) <= MAX_MODULES

# ordering, permutation preservation
assert Counter(reorder(xs)) == Counter(xs)
```

## error-case — negative domain

```python
from hypothesis import assume
import pytest


@pytest.mark.trace("FR-005-AC-1", "TC-345")
@given(cmd=st.from_regex(r"\A[a-z]{1,12}\Z"))
def test_fr_005_ac_1_unknown_command_errors(cmd):
    """An unknown command raises an error that names the usage.

    spec-correctness: row=FR-005-AC-1 property=error-case extraction=extractable origin=regex review=none
    """
    assume(cmd not in KNOWN)
    with pytest.raises(UnknownCommandError, match="Usage:"):
        run([cmd])
```

`pytest.raises(Class, match=...)` carries both required assertions — the error class from
the code, the payload from `## Outputs` — in one line.

Prefer `st.sampled_from` over a broad regex plus `assume` when the negative domain is small;
too many `assume` rejections trip hypothesis's health checks.

## lifecycle — RuleBasedStateMachine

```python
from hypothesis.stateful import RuleBasedStateMachine, invariant, precondition, rule


@pytest.mark.trace("FR-040-AC-2", "TC-345")
class SessionMachine(RuleBasedStateMachine):
    """A session accepts exactly the ops its state allows.

    spec-correctness: row=FR-040-AC-2 property=lifecycle extraction=extractable origin=regex review=none
    """

    def __init__(self):
        super().__init__()
        self.sut = Session()
        self.model = []

    @rule()
    def open(self):
        self.sut.open()

    @rule(data=st.text(max_size=64))
    @precondition(lambda self: self.sut.is_open)
    def write(self, data):
        self.sut.write(data)
        self.model.append(data)

    @invariant()
    def buffer_matches_model(self):
        assert self.sut.buffer == "".join(self.model)


TestSessionLifecycle = SessionMachine.TestCase
```

`@precondition` is the step-2 precondition; `@invariant` is the step-2 oracle.

## concurrency — always a downgrade

hypothesis ships no interleaving scheduler, so a Python concurrency criterion never gets
real interleaving coverage. Emit a `RuleBasedStateMachine` over the async model, and record
the downgrade as a finding:

```python
@pytest.mark.trace("FR-050-AC-1", "TC-345")
class TestConcurrentWrites(RuleBasedStateMachine):
    """Concurrent writers never lose an entry.

    spec-correctness: row=FR-050-AC-1 property=concurrency extraction=not-extractable origin=llm-second-pass confidence=low
    """
```

The finding says the interleaving is not exhaustively explored. That is what needs a
person — the test itself is fine to run.

## A `candidate` record

Same file, same path, and it runs:

```python
@pytest.mark.trace("FR-018-AC-3", "TC-345")
@given(src=plugin_sources)
def test_fr_018_ac_3_source_maps_to_one_root(src):
    """A plugin source maps to exactly one resolved root.

    spec-correctness: row=FR-018-AC-3 property=invariant extraction=candidate origin=regex-candidate
    """
```

What marks it for review is a `medium` finding in the review artifact (step 6), not a skip
marker in the tree.

## Notes

- `@given` outermost, `@pytest.mark.*` above it; hypothesis and pytest fixtures do not mix
  freely — pass state through the strategy, not through a function-scoped fixture.
- Bound every `st.lists` with `max_size`, every `st.text` with `max_size`.
- Never `@seed(...)` a generated test, and never `@settings(derandomize=True)` — both hide
  the failures the property exists to find.
- Raise `max_examples` only with a reason recorded as a finding.
