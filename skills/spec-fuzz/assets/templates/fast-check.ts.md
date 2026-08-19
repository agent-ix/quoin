# fast-check target template

TypeScript has no libFuzzer equivalent in ordinary use, so the harness is `fast-check`
running in the repo's existing test runner. That has one consequence worth stating: **this is
a bounded search, not a fuzzing campaign.** It runs for N cases and stops.

Say so in the report rather than implying parity with cargo-fuzz.

## Placement is the thing that bites

The `Trace:` carrier goes **immediately above `it(`** — never above `describe(`. A `describe`
block groups tests and registers no symbol of its own, so a tag above it binds to nothing
while reading correctly and matching a grep (`agent-ix/quoin#61`).

```ts
import fc from "fast-check";
import { describe, expect, it } from "vitest";

import { parseManifest } from "../src/manifest.js";

describe("the manifest parser under arbitrary input", () => {
  // The parser does not throw an unexpected error on any string.
  //
  // Trace: NFR-003-M-1
  //
  // spec-fuzz: obligation=NFR-003-M-1 harness=fast-check entry=parseManifest origin=advised
  it("NFR-003-M-1 never throws outside its declared rejection", () => {
    fc.assert(
      fc.property(fc.string(), (text) => {
        try {
          parseManifest(text);
        } catch (error) {
          // The declared rejection path. Any OTHER error is the bug this
          // target exists to find, so it is rethrown rather than swallowed —
          // a bare catch would make this target incapable of failing.
          if (!(error instanceof ManifestError)) throw error;
        }
      }),
      { numRuns: 10_000 },
    );
  });
});
```

## Generators that reach past the first parse

`fc.string()` finds shallow crashes and then plateaus — almost every case fails validation in
the first few bytes and the code behind it is never reached. Where the surface takes
structured text, build the structure and mutate it:

```ts
const manifestish = fc.record({
  name: fc.string(),
  version: fc.oneof(fc.string(), fc.constant("1.0.0")),
  entries: fc.array(fc.record({ id: fc.string(), ref: fc.string() }), { maxLength: 8 }),
});

fc.assert(
  fc.property(manifestish, (value) => {
    try {
      parseManifest(JSON.stringify(value));
    } catch (error) {
      if (!(error instanceof ManifestError)) throw error;
    }
  }),
);
```

Mixing `fc.constant("1.0.0")` into an otherwise arbitrary field is deliberate: it keeps a
fraction of cases valid enough to get past the field's own check, which is where the
interesting behaviour is.

## Seeds and reproduction

`fc.assert` prints the failing counterexample and its seed. Put the seed in the bug, not a
copy of the input — the seed reproduces the shrink as well as the failure.
