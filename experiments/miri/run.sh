#!/usr/bin/env bash
# Runs every program here under Miri. `bad_*` must be reported as undefined behaviour or a
# data race; `good_*` must run clean. Needs a nightly toolchain with the `miri` component.
set -uo pipefail
cd "$(dirname "$0")"
status=0
for src in src/bin/*.rs; do
    bin=$(basename "$src" .rs)
    out=$(cargo +nightly miri run --quiet --bin "$bin" < /dev/null 2>&1)
    if grep -qE "Undefined Behavior|Data race" <<< "$out"; then verdict=ub; else verdict=clean; fi
    case "$bin:$verdict" in
        bad_*:ub | good_*:clean) echo "ok    $bin: $verdict" ;;
        *) echo "FAIL  $bin: $verdict"; echo "$out" | tail -20; status=1 ;;
    esac
    grep -m1 -E "^error: (Undefined Behavior|Data race).*" <<< "$out" | sed 's/^/      /'
done
exit $status
