#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX
#
# PLAT-1025 mutation runner. For every entry of mutations.json with no
# `outcome` yet, apply its scratch edit (an exact, single-occurrence
# substitution of `find` by `replace` in `file`), record the unified diff as
# `patch`, rerun exactly the one owning test where the kind needs it, record
# the outcome, and revert the file with `git checkout`. Nothing mutated is
# ever committed: the script refuses to start, and stops, if a tracked file
# outside this directory is modified.
#
# Outcomes: caught (the owning test failed), survived (it passed),
# compile_error / not_run / find_not_unique (the entry is then `dropped`, with
# the reason), n/a (no rerun needed: requirement, criterion and trace edits,
# and code mutants of a row whose code no test cites).
#
# A caught code mutant also records `failure`: `assertion` when the first
# panic of the run is raised inside the owning test's own body (an assert, a
# panic, or an unwrap on the result it checks), `harness_reject` when proptest
# gave up rejecting inputs ("Too many global rejects"), and `panic_elsewhere`
# when the panic is raised in library code or a helper. Only an `assertion`
# failure is evidence about what the test checks (PR #628 review).
#
# A violating mutant with no `modes` is a pair-only mutant: it makes no corpus
# row, and exists so a test-weakening mutant of an RT row has a pair.
#
# Order matters: violating_code entries run first, because a test_weakening
# entry reruns its weakened test against its `pair` (a violating_code id) and
# reads the pair's outcome on the original test.
#
# Usage, from anywhere inside the worktree:
#   CARGO_TARGET_DIR=<dir> run-mutants.sh [id ...]
#   run-mutants.sh classify <logdir> id ...   (re-read `failure` from kept logs)
# Every cargo run takes /tmp/quire-heavy-check.lock with CARGO_BUILD_JOBS=1. To take the
# lock once for a batch instead, run the script under `flock <lock>` with
# QUOIN_EVAL_V2_LOCK=held.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(git -C "$here" rev-parse --show-toplevel)"
mutations="$here/mutations.json"
sample="$here/sample.json"
lock="${QUOIN_EVAL_V2_LOCK:-/tmp/quire-heavy-check.lock}"
: "${CARGO_TARGET_DIR:?set CARGO_TARGET_DIR to the named target dir of this worktree}"
logdir="${QUOIN_EVAL_V2_LOGDIR:-$(mktemp -d)}"

dirty() {
  git -C "$root" status --porcelain --untracked-files=no -- . ":(exclude)rust/crates/quoin-jev/tests/fixtures/eval-v2"
}
if [ -n "$(dirty)" ]; then
  echo "refusing: tracked files are modified:" >&2
  dirty >&2
  exit 2
fi

field() { jq -r --arg id "$1" ".mutations[] | select(.id == \$id) | $2" "$mutations"; }
set_field() { # id jq-path json-value
  local tmp
  tmp="$(mktemp)"
  jq --arg id "$1" --argjson v "$3" "(.mutations[] | select(.id == \$id) | $2) = \$v" "$mutations" >"$tmp"
  mv "$tmp" "$mutations"
}

applied=()

# apply ID: substitute find by replace in file, exactly once.
apply() {
  local id="$1" file find replace
  file="$root/$(field "$id" .file)"
  applied+=("$(field "$id" .file)")
  find="$(mktemp)"
  replace="$(mktemp)"
  field "$id" .find | head -c -1 >"$find"
  field "$id" .replace | head -c -1 >"$replace"
  FIND="$find" REPL="$replace" perl -0777 -i -pe '
    BEGIN { local $/; open my $f, "<", $ENV{FIND} or die; $F = <$f>; open my $r, "<", $ENV{REPL} or die; $R = <$r>; }
    my $n = () = /\Q$F\E/g;
    die "find occurs $n times\n" unless $n == 1;
    s/\Q$F\E/$R/;
  ' "$file"
  rm -f "$find" "$replace"
}

# Revert exactly the files this script edited, and nothing else.
revert_all() {
  if [ "${#applied[@]}" -gt 0 ]; then
    git -C "$root" checkout -- "${applied[@]}"
    applied=()
  fi
  if [ -n "$(dirty)" ]; then
    echo "revert failed; tracked files still modified" >&2
    dirty >&2
    exit 3
  fi
}
trap revert_all EXIT

owning() { jq -c --arg id "$1" '.natural[] | select(.id == $id) | .owning_test' "$sample"; }

# run_test SOURCE-ID LOG: prints caught | survived | compile_error | not_run
run_test() {
  local ot pkg name
  ot="$(owning "$1")"
  pkg="$(jq -r .package <<<"$ot")"
  name="$(jq -r .name <<<"$ot")"
  mapfile -t selector < <(jq -r '.selector[]' <<<"$ot")
  if [ "$lock" = held ]; then
    # The caller already holds the heavy-check lock for a whole batch.
    (cd "$root/rust" && CARGO_BUILD_JOBS=1 \
      cargo test -p "$pkg" --locked "${selector[@]}" -- --exact "$name") >"$2" 2>&1 || true
  else
    (cd "$root/rust" && CARGO_BUILD_JOBS=1 flock "$lock" \
      cargo test -p "$pkg" --locked "${selector[@]}" -- --exact "$name") >"$2" 2>&1 || true
  fi
  if grep -q "could not compile" "$2"; then
    echo compile_error
  elif grep -qE "test result: FAILED\. 0 passed; 1 failed" "$2"; then
    echo caught
  elif grep -qE "test result: ok\. 1 passed; 0 failed" "$2"; then
    echo survived
  else
    echo not_run
  fi
}

# line_shift ID TEST-PATH TEST-LEAD: how many lines mutant ID moves a test that
# starts at TEST-LEAD, when ID edits the test's own file above it.
line_shift() {
  local find replace
  if [ "$(field "$1" .file)" != "$2" ]; then
    echo 0
    return
  fi
  find="$(mktemp)"
  replace="$(mktemp)"
  field "$1" .find | head -c -1 >"$find"
  field "$1" .replace | head -c -1 >"$replace"
  git -C "$root" show "HEAD:$2" | FIND="$find" REPL="$replace" LEAD="$3" perl -0777 -ne '
    open my $f, "<", $ENV{FIND} or die; local $/; my $F = <$f>;
    open my $r, "<", $ENV{REPL} or die; my $R = <$r>;
    my $at = index($_, $F);
    my $line = 1 + (() = substr($_, 0, $at) =~ /\n/g);
    print $line < $ENV{LEAD} ? (() = $R =~ /\n/g) - (() = $F =~ /\n/g) : 0;
  '
  rm -f "$find" "$replace"
}

# failure_kind ID LOG: assertion | harness_reject | panic_elsewhere, for the
# caught code mutant ID whose owning-test run wrote LOG.
failure_kind() {
  local test path lead end loc file line shift
  if grep -qE "Too many (global|local) rejects" "$2"; then
    echo harness_reject
    return
  fi
  test="$(jq -c --arg id "$(field "$1" .source)" '.natural[] | select(.id == $id) | .source.test' "$sample")"
  path="$(jq -r .path <<<"$test")"
  lead="$(jq -r .leading_line <<<"$test")"
  end="$(jq -r .end_line <<<"$test")"
  shift="$(line_shift "$1" "$path" "$lead")"
  lead=$((lead + shift))
  end=$((end + shift))
  path="${path#rust/}"
  # A proptest failure names the failing prop_assert after "Test failed:".
  loc="$(grep -oE "Test failed: .* at [^ ]+:[0-9]+" "$2" | head -n 1 | grep -oE "[^ ]+:[0-9]+$" || true)"
  if [ -z "$loc" ]; then
    loc="$(grep -oE "panicked at [^ ]+:[0-9]+:[0-9]+:" "$2" | head -n 1 | sed -E 's/^panicked at //; s/:[0-9]+:$//' || true)"
  fi
  file="${loc%:*}"
  line="${loc##*:}"
  if [ -n "$loc" ] && [ "$file" = "$path" ] && [ "$line" -ge "$lead" ] && [ "$line" -le "$end" ]; then
    echo assertion
  else
    echo panic_elsewhere
  fi
}

if [ "${1:-}" = classify ]; then
  shift
  kept="$1"
  shift
  for id in "$@"; do
    if [ "$(field "$id" .outcome)" = caught ]; then
      set_field "$id" .failure "\"$(failure_kind "$id" "$kept/$id.log")\""
      echo "$id $(field "$id" .failure)"
    fi
  done
  exit 0
fi

patch_of() { git -C "$root" diff -- "$(field "$1" .file)"; }

process() {
  local id="$1" kind source outcome p
  kind="$(field "$id" .kind)"
  source="$(field "$id" .source)"
  set_field "$id" .owning_test "$(owning "$source")"
  if [ "$(field "$id" .file)" = null ]; then
    # A trace swap shown without a test edits no file; its patch is given.
    set_field "$id" .outcome '"n/a"'
    echo "$id $kind n/a"
    return
  fi
  if ! apply "$id" 2>"$logdir/$id.apply"; then
    set_field "$id" .outcome '"dropped"'
    set_field "$id" .drop_reason "$(jq -Rs . <"$logdir/$id.apply")"
    return
  fi
  p="$(patch_of "$id")"
  set_field "$id" .patch "$(jq -Rs . <<<"$p")"
  if [ "$(owning "$source")" = null ] && [ "$kind" != test_weakening ]; then
    # No test cites this code: the mutant is true by construction only.
    revert_all
    set_field "$id" .outcome '"n/a"'
    echo "$id $kind n/a (no owning test)"
    return
  fi
  case "$kind" in
    violating_code)
      outcome="$(run_test "$source" "$logdir/$id.log")"
      revert_all
      case "$outcome" in
        caught)
          set_field "$id" .outcome '"caught"'
          set_field "$id" .failure "\"$(failure_kind "$id" "$logdir/$id.log")\""
          ;;
        survived) set_field "$id" .outcome '"survived"' ;;
        *) set_field "$id" .outcome '"dropped"'; set_field "$id" .drop_reason "\"$outcome\"" ;;
      esac
      ;;
    additive_code)
      outcome="$(run_test "$source" "$logdir/$id.log")"
      revert_all
      if [ "$outcome" = survived ]; then
        set_field "$id" .outcome '"survived"'
      else
        set_field "$id" .outcome '"dropped"'
        set_field "$id" .drop_reason "\"owning test outcome: $outcome; an additive mutant must still pass\""
      fi
      ;;
    test_weakening)
      local pair pair_outcome
      outcome="$(run_test "$source" "$logdir/$id.log")"
      if [ "$outcome" != survived ]; then
        revert_all
        set_field "$id" .outcome '"dropped"'
        set_field "$id" .drop_reason "\"weakened test against the shipped code: $outcome; it must still pass\""
        return
      fi
      pair="$(field "$id" .pair)"
      pair_outcome="$(field "$pair" .outcome)"
      set_field "$id" .pair_outcome_original_test "\"$pair_outcome\""
      if [ "$pair_outcome" = caught ] && apply "$pair" 2>"$logdir/$id.pair.apply"; then
        outcome="$(run_test "$source" "$logdir/$id.pair.log")"
        set_field "$id" .pair_outcome_weakened_test "\"$outcome\""
      else
        set_field "$id" .pair_outcome_weakened_test null
      fi
      revert_all
      set_field "$id" .outcome '"survived"'
      ;;
    requirement_text | criterion | trace_swap | fr_statement)
      revert_all
      set_field "$id" .outcome '"n/a"'
      ;;
    *)
      echo "unknown kind $kind for $id" >&2
      exit 4
      ;;
  esac
  echo "$id $kind $(field "$id" .outcome)"
}

if [ "$#" -gt 0 ]; then
  ids=("$@")
else
  mapfile -t ids < <(jq -r '
    [.mutations[] | select(.outcome == null)]
    | sort_by({"violating_code": 0, "additive_code": 1, "test_weakening": 2}[.kind] // 3)
    | .[].id' "$mutations")
fi
for id in "${ids[@]}"; do
  process "$id"
done
echo "logs in $logdir"
