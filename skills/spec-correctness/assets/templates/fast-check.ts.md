# TypeScript / fast-check templates

File: `tests/props/fr-NNN.prop.test.ts` (queued: `tests/props/_review/…`).
Inert marker: `it.skip` / `describe.skip`.
Runner: whichever the repo already uses — read `vitest.config.*`, `jest.config.*`, or
`package.json#scripts.test`. The imports below assume vitest; swap for `@jest/globals`
where the repo uses jest.

## Anatomy

```ts
// tests/props/fr-012.prop.test.ts
import fc from "fast-check";
import { describe, it, expect } from "vitest";
import { detectDuplicates, type Entry } from "../../src/catalog.js";

// grounded in step 2 from `## Inputs` + the signature at src/catalog.ts:88
const entryArb: fc.Arbitrary<Entry> = fc.record({
  kind: fc.stringMatching(/^[a-z]{1,12}$/),
  name: fc.stringMatching(/^[a-z]{1,12}$/),
});

describe("FR-012-AC-1 duplicate modules are sorted", () => {
  /**
   * Trace: FR-012-AC-1 — declaring modules are listed in sorted order.
   * spec-correctness: row=FR-012-AC-1 property=ordering extraction=extractable origin=regex review=none
   */
  it("holds for any catalog", () => {
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

Both tag carriers: `Trace:` on its own JSDoc line, and `FR-012-AC-1` inside the `describe`
title.

**The `Trace:` block goes immediately above the `it(` / `test(` registration — never above
the enclosing `describe`.** A tag binds to the symbol whose source span encloses it, and
`describe(…)` groups tests but *registers none itself* (quire-rs
`src/symbols/typescript.rs`, `registration`). A tag above a `describe` therefore attaches
to nothing: it reads fine, greps fine, and is invisible to `quire coverage`. That is the
whole of agent-ix/quoin#61 — measured, one tag moved from its `describe` to its `test(`
took `FR-005` from 0/3 to 1/3 with no other change.

Python and Rust are unaffected: a symbol's span covers its annotation block, declaration
**and body**, so a docstring inside the function and an attribute above `#[test] fn` both
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
   * Trace: FR-005-AC-1 — an unknown command raises an error that names the usage.
   * spec-correctness: row=FR-005-AC-1 property=error-case extraction=extractable origin=regex review=none
   */
  it("rejects any command outside the known set", () => {
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
   * Trace: FR-040-AC-2 — a session accepts exactly the ops its state allows.
   * spec-correctness: row=FR-040-AC-2 property=lifecycle extraction=extractable origin=regex review=none
   */
  it("matches the model over any op sequence", () => {
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
   * Trace: FR-050-AC-1 — concurrent writers never lose an entry.
   * spec-correctness: row=FR-050-AC-1 property=concurrency extraction=extractable origin=regex review=none
   */
  it("linearizes under any interleaving", async () => {
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

## Queued form

```ts
describe("FR-018-AC-3 source maps to one root", () => {
  /**
   * Trace: FR-018-AC-3 — a plugin source maps to exactly one resolved root.
   * spec-correctness: row=FR-018-AC-3 property=invariant extraction=candidate origin=regex-candidate review=required
   */
  it.skip("spec-correctness review pending", () => { … });
});
```

Acceptance turns `it.skip` into `it` and moves the file. The tag carriers are untouched.

## Notes

- `fc.assert(fc.property(…))` synchronous, `await fc.assert(fc.asyncProperty(…))` async.
  A forgotten `await` makes the test pass vacuously.
- Bound every `fc.array` with `maxLength`.
- `fc.pre(...)` inside the property, not around it.
- Return `void` or `boolean` from a property predicate — an assertion that throws is fine;
  a returned truthy non-boolean is not.
- Reuse the repo's existing arbitraries and factories before writing new ones.
