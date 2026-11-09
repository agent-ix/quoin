# TypeScript / fast-check templates

File: `tests/props/fr-NNN.prop.test.ts`.
Runner: whichever the repo already uses — read `vitest.config.*`, `jest.config.*`, or
`package.json#scripts.test`. The imports below assume vitest, the only runner the `trace` helper
(`@agent-ix/ix-trace-ts/vitest`) supports; a jest repo has no explicit TS trace marker.

## Anatomy

```ts
// tests/props/fr-012.prop.test.ts
import fc from "fast-check";
import { describe, it, expect } from "vitest";
import { detectDuplicates, type Entry } from "../../src/catalog.js";
import { trace } from "@agent-ix/ix-trace-ts/vitest";

// grounded in step 2 from `## Inputs` + the signature at src/catalog.ts:88
const entryArb: fc.Arbitrary<Entry> = fc.record({
  kind: fc.stringMatching(/^[a-z]{1,12}$/),
  name: fc.stringMatching(/^[a-z]{1,12}$/),
});

describe("FR-012-AC-1 duplicate modules are sorted", () => {
  /**
   * Declaring modules are listed in sorted order.
   * spec-correctness: row=FR-012-AC-1 property=ordering extraction=extractable origin=regex review=none
   */
  it("holds for any catalog", () => {
    trace("FR-012-AC-1", "TC-345");
    fc.assert(
      fc.property(fc.array(entryArb, { maxLength: 32 }), (entries) => {
        for (const d of detectDuplicates(entries).duplicates) {
          expect(d.modules).toEqual([...d.modules].sort());
        }
      }),
    );
  });
});
```

Both tag carriers: the `trace("FR-012-AC-1", "TC-345")` call (the criterion id plus the
`spec/matrix.md` row's own `TC-` id; an id in a comment binds nothing), and `FR-012-AC-1` inside
the `describe` title. `trace(...ids: string[]): void` records the ids on the running
test when `QUIRE_TRACE_EMIT=1` (and does nothing otherwise), so it must be called inside a test
callback; `quire coverage` binds the quoted ids statically.

**The `trace(...)` call goes inside the `it(` / `test(` callback — never in the enclosing
`describe`.** A tag binds to the symbol whose source span encloses it, and
`describe(…)` groups tests but *registers none itself* (quire-rs
`src/symbols/typescript.rs`, `registration`). A tag in a `describe` body therefore attaches
to nothing: it reads fine, greps fine, and is invisible to `quire coverage`. That is the
whole of agent-ix/quoin#61 — measured, one tag moved from its `describe` to its `test(`
took `FR-005` from 0/3 to 1/3 with no other change.

Python and Rust are unaffected: a symbol's span covers its annotation block, declaration
**and body**, so a decorator above the function and an attribute above `#[test] fn` both
land inside a real test symbol. Only the TS placement can fall outside one.

## Family bodies

```ts
// round-trip
expect(parse(render(x))).toEqual(x);

// round-trip, lossy
expect(normalize(parse(render(x)))).toEqual(normalize(x));

// idempotence — seed with f's own outputs
const seeded = fc.oneof(rawConfigArb, rawConfigArb.map(normalize));
expect(normalize(normalize(x))).toEqual(normalize(x));

// invariant
expect(resolve(x).length).toBeLessThanOrEqual(MAX_MODULES);

// ordering, order-independence
expect(summarize(xs)).toEqual(summarize([...xs].reverse()));
```

## error-case — negative domain

```ts
describe("FR-005-AC-1 unknown commands error", () => {
  /**
   * An unknown command raises an error that names the usage.
   * spec-correctness: row=FR-005-AC-1 property=error-case extraction=extractable origin=regex review=none
   */
  it("rejects any command outside the known set", () => {
    trace("FR-005-AC-1", "TC-345");
    fc.assert(
      fc.property(fc.stringMatching(/^[a-z]{1,12}$/), (cmd) => {
        fc.pre(!KNOWN.has(cmd));
        expect(() => run([cmd])).toThrow(UnknownCommandError); // class, from src
        expect(() => run([cmd])).toThrow(/Usage:/);            // payload, from ## Outputs
      }),
    );
  });
});
```

## lifecycle — fc.commands / modelRun

```ts
describe("FR-040-AC-2 session lifecycle", () => {
  /**
   * A session accepts exactly the ops its state allows.
   * spec-correctness: row=FR-040-AC-2 property=lifecycle extraction=extractable origin=regex review=none
   */
  it("matches the model over any op sequence", () => {
    trace("FR-040-AC-2", "TC-345");
    fc.assert(
      fc.property(fc.commands(allCommands, { maxCommands: 24 }), (cmds) =>
        fc.modelRun(() => ({ model: newModel(), real: new Session() }), cmds),
      ),
    );
  });
});
```

Each command's `check(model)` is the precondition; its `run(model, real)` carries the
assertion. That split *is* the precondition/oracle split from step 2.

## concurrency — deterministic scheduler

```ts
describe("FR-050-AC-1 concurrent writes", () => {
  /**
   * Concurrent writers never lose an entry.
   * spec-correctness: row=FR-050-AC-1 property=concurrency extraction=extractable origin=regex review=none
   */
  it("linearizes under any interleaving", async () => {
    trace("FR-050-AC-1", "TC-345");
    await fc.assert(
      fc.asyncProperty(fc.scheduler(), async (s) => {
        const store = new Store();
        s.scheduleSequence([
          { label: "a", builder: () => store.put("a", 1) },
          { label: "b", builder: () => store.put("b", 2) },
        ]);
        await s.waitAll();
        expect(store.size).toBe(2);
      }),
    );
  });
});
```

`fc.scheduler()` is deterministic and shrinkable — never use real timers or `Promise.all`
races for a concurrency property.

## A `candidate` record

Same file, same path, and it **runs**. `extraction: candidate` means the metamorphic label
was not corroborated structurally — something for a reviewer to read, not a reason to
disable a test:

```ts
describe("FR-018-AC-3 source maps to one root", () => {
  /**
   * A plugin source maps to exactly one resolved root.
   * spec-correctness: row=FR-018-AC-3 property=invariant extraction=candidate origin=regex-candidate
   */
  it("holds for any plugin source", () => {
    trace("FR-018-AC-3", "TC-345");
    …
  });
});
```

What marks it for review is a `medium` finding in the review artifact (step 6), not an
`it.skip` in the tree.

## Notes

- `fc.assert(fc.property(…))` synchronous, `await fc.assert(fc.asyncProperty(…))` async.
  A forgotten `await` makes the test pass vacuously.
- Bound every `fc.array` with `maxLength`.
- `fc.pre(...)` inside the property, not around it.
- Return `void` or `boolean` from a property predicate — an assertion that throws is fine;
  a returned truthy non-boolean is not.
- Reuse the repo's existing arbitraries and factories before writing new ones.
