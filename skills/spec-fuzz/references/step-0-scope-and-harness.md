# Step 0: Scope and harness

**Goal**: know what the repository can actually run before generating anything for it.

## Detect the harness from the manifest, never from the requirement

The requirement says *"the parser SHALL NOT panic on arbitrary input"*. It does not say Rust.
The **repository's manifest** says Rust, and that is the only thing that may choose a harness
(FR-038-AC-7).

| Manifest | Harness | Present when |
|---|---|---|
| `Cargo.toml` | cargo-fuzz / libFuzzer | a `fuzz/Cargo.toml` exists with `cargo-fuzz = true`, or `cargo fuzz --version` resolves |
| `pyproject.toml` / `setup.py` | atheris | `atheris` is a declared dev-dependency |
| `package.json` | fast-check | `fast-check` is a declared dependency |

A repository with several manifests has several harnesses; scope each obligation to the one
its entry point lives in.

## Absent tooling stops the run

If the manifest is present but the harness is not, **stop here**. Emit one finding per
selected obligation and write no files (FR-038-AC-4).

Do not:

- run `cargo install cargo-fuzz`
- add `atheris` or `fast-check` to a dependency list
- create `fuzz/Cargo.toml`, a `[[bin]]` block, or a nightly toolchain file

A fuzz workspace is not a generated artifact. It is a dev-dependency, a nightly toolchain
requirement and a CI job, and adding all three to somebody's repository because their spec
mentioned a parser is a decision that belongs to them (ADR-0011: generated artifacts are
consumer-owned).

The finding is the deliverable in this case, and it is a good one — it says *"your spec
declares N fuzzable surfaces and this repository cannot fuzz"*, which is a fact nobody had
written down.

## Record what you detected

The provenance line carries it, so a re-run and a reviewer can both see which harness was
chosen and why:

```
spec-fuzz: obligation=NFR-012-M-2 harness=cargo-fuzz entry=quire_rs::parse_document origin=advised
```

`origin ∈ {advised, authored}` — whether the method came from `quoin advise`'s
recommendation or from the requirement's own `Verification` cell. They are different claims
and the distinction survives into the report.
