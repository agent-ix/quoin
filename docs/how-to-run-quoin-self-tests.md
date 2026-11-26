---
type: how-to
description: List Quoin's embedded self-test cases and retain a run record.
---

# Run Quoin's embedded self-tests

Use `quoin selftest` to inspect the cases shipped with an installed Quoin binary and record their outcomes. The runner executes the installed binary's embedded scripts and keeps their results in a JSON run record.

## Before you start

Install Quoin on a supported Linux system and make sure `quoin` is on `PATH`. The examples use `jq` to inspect the retained JSON file.

## Steps

1. **Check the installed version and list its cases.**

   ```sh
   quoin --version
   quoin selftest --list
   ```

   The list contains the case ids embedded in that binary. The installed release includes `selftest-list`.

2. **Run one case and save its record.**

   ```sh
   run_dir=$(mktemp -d)
   cd "$run_dir"
   quoin selftest --run --case selftest-list --out "$run_dir/selftest.json"
   ```

   `--out` retains the run record at the named path. From a repository that has `spec/evidence/`, Quoin also retains a copy in that repository's run store. From this temporary directory there is no repository store, so the explicit `--out` path is the retained destination. The runner writes no run-result JSON to stdout.

3. **Run the complete embedded case set when you want a broader check.**

   ```sh
   quoin selftest --run --out "$run_dir/all-selftests.json"
   ```

   Without `--case`, the runner selects every embedded case. A failed, errored, or timed-out case makes the command return a non-zero exit code, while the record retains the outcome details.

## Expected output

The `--list` command prints `selftest-list`. The selected-case run writes `selftest.json` under the temporary directory. Its record names the installed Quoin version and includes a `selftest-list` result with status `pass` and criterion `FR-235-AC-1`.

## Verify the record

Inspect the retained version and case outcome:

```sh
jq '{toolVersion, suite, results: [.results[] | {id, status, criteria}]}' \
  "$run_dir/selftest.json"
```

The output shows the installed version and the selected case result. Remove the temporary files when you no longer need them:

```sh
rm -rf "$run_dir"
```
