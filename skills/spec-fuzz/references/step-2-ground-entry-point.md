# Step 2: Ground the entry point

**Goal**: the symbol the target will call. It must exist.

## Why this step cannot be skipped

A property test that grounds nothing is a weak test. **A fuzz target that calls nothing is a
build error in somebody's CI**, and it arrives in a harness most reviewers do not run
locally, so it fails in the one place nobody is watching.

The entry point is therefore *found in the source*, never inferred from the requirement's
wording (FR-038-AC-2).

## How to find it

In order, stopping at the first that resolves:

1. **The requirement's `## Inputs` section**, if it names a symbol or a module path. This is
   the author telling you directly.
2. **An existing target in the same suite** covering a neighbouring surface — its imports
   show the shape of a callable entry point in this repository.
3. **The public surface**: a function taking `&[u8]`, `&str`, `bytes` or `string` and
   returning a `Result`/`Option`, exported from the module the requirement names.

Read the function. Confirm it takes the input the requirement describes and that calling it
has no side effect outside the process — a fuzz target that writes files or opens sockets
will be killed by the fuzzer's own sanitizers and produce nothing but noise.

## Ungrounded is a finding, not a guess

If none of the three resolves, **write no file for that obligation** and record a finding
naming it and what you looked for (FR-038-AC-3).

This is the honest outcome and it is often the useful one. *"NFR-003 declares the config
loader must not panic on arbitrary input, and there is no function in `src/` that takes
arbitrary bytes"* means either the surface does not exist yet or the requirement is about
something else. Both are worth a reviewer's attention; a generated target guessing at
`load_config()` is not.

## Do not fuzz through a wrapper

Where the requirement names a surface and the obvious public function wraps it in file I/O,
ground on the **inner** function that takes the bytes. Fuzzing the wrapper spends the whole
run on the filesystem and reaches the parser a few thousand times instead of a few hundred
million.

Record the choice in the provenance line, because it is a judgement:

```
spec-fuzz: obligation=NFR-003-M-1 harness=cargo-fuzz entry=loader::manifest::parse_manifest origin=advised
```

If you grounded on an inner function while the requirement named the outer one, say so in
the report. It is the kind of thing that is obviously right when explained and looks like an
error when not.
