# atheris target template

One file per obligation, at `fuzz/fuzz_<obligation_slug>_<surface>.py`, executable.

Atheris instruments at import time, so **`atheris.instrument_imports()` must wrap the import
of the module under test**. An import outside that block is uninstrumented: the fuzzer runs,
reports coverage, finds nothing, and looks like it worked.

## Bytes in, no interpretation

```python
#!/usr/bin/env python3
"""The config loader does not raise on arbitrary input.

Trace: NFR-003-M-1

spec-fuzz: obligation=NFR-003-M-1 harness=atheris entry=myapp.config.parse_config origin=advised
"""

import sys

import atheris

with atheris.instrument_imports():
    from myapp.config import parse_config


def test_one_input(data: bytes) -> None:
    fdp = atheris.FuzzedDataProvider(data)
    text = fdp.ConsumeUnicodeNoSurrogates(fdp.remaining_bytes())
    try:
        parse_config(text)
    except (ValueError, KeyError):
        # The declared rejection path. Raising a *declared* error on garbage is
        # the requirement being met, so it must not read as a crash. Catch the
        # exception types the requirement names — never bare `except Exception`,
        # which would swallow the AttributeError and IndexError this exists to find.
        pass


atheris.Setup(sys.argv, test_one_input)
atheris.Fuzz()
```

The narrow `except` is the whole design of the file. `except Exception` turns a target that
finds real bugs into one that can never fail, and it looks more careful.

## Which exceptions to catch

From the requirement, not from the code. If the requirement says *"rejects malformed input"*
and the code raises `ConfigError`, catch `ConfigError`. If the code also raises `TypeError`
on some path, that is a **finding** — the requirement says reject, and `TypeError` is a
crash, not a rejection.

Do not widen the catch to make the target quiet. That is the same move as widening a rule to
lower a finding count.

## Structured input

`FuzzedDataProvider` builds typed values without a hand-rolled decoder:

```python
def test_one_input(data: bytes) -> None:
    fdp = atheris.FuzzedDataProvider(data)
    document = fdp.ConsumeUnicodeNoSurrogates(fdp.ConsumeIntInRange(0, 4096))
    selector = fdp.ConsumeUnicodeNoSurrogates(fdp.remaining_bytes())
    try:
        run_query(selector, document)
    except QueryError:
        pass
```

Consume the bounded field first and the unbounded one last, or the second field is almost
always empty and half the input surface is never reached.
