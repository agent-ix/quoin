# Step 1: Select the obligations

**Goal**: the set of obligations that want a fuzz target, decided by module data.

## The selector

An obligation is in scope when its verification method carries **`evidence_kind: Fuzz`** in
the active `verification_catalog`.

```
quoin advise --repo <repo> --json
quoin catalog methods --json          # the catalog, with each method's evidence_kind
```

Take the method from the obligation's authored `Verification` cell if it has one, and from
the top recommendation otherwise. Record which in `origin=` (step 0).

## Do not carry a list of fuzz methods here

Today the catalog declares two — `fuzzing` and `grammar-based-fuzzing`. **Do not hardcode
those two names.** A module adding a third, or renaming one, must change this skill's
behaviour without changing this skill (FR-038-CON-1).

The test for "is this a fuzz method" is `evidence_kind === "Fuzz"`, every time.

## Two facts about today's catalog worth knowing

Neither changes the selector; both change what you should expect to see.

**`grammar-based-fuzzing` is never advised.** It is keyed on `structured-input` and
`grammar-declared`, and `characteristicsOf` mints neither, so no statement can reach it
(`agent-ix/quoin#128`). It arrives only as an authored method. If you were expecting the
advisor to suggest it for a grammar, that is why it did not.

**`fuzzing` is advised from two of its three keys.** `untrusted-input` and `parser` are
mintable; `deserializer` is not. An obligation about deserialization that says neither
"untrusted" nor "pars…" will not be advised, and is a legitimate authored-method case.

## What a good selection looks like

Obligations that survive this step describe an **input surface**: something that consumes
bytes or text from outside and must not crash. If a selected obligation is about a return
value, a threshold or an ordering, the method is wrong — say so in the report rather than
generating a target that fuzzes the wrong thing.

That is a finding about the method, not about the requirement. Do not reword the
requirement, and do not silently drop it (FR-038-AC-9).
