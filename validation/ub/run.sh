#!/usr/bin/env bash
# Runs each demo under Miri and checks that the flagged pattern is reported as
# undefined behaviour while the fixed control is clean.
set -u
cd "$(dirname "$0")"
fail=0
check() {
  local bin=$1 expect=$2
  out=$(cargo +nightly miri run -q --bin "$bin" 2>&1)
  if echo "$out" | grep -qE "Undefined Behavior|data race"; then got=ub; else got=clean; fi
  if [ "$got" = "$expect" ]; then status=ok; else status=MISMATCH; fail=1; fi
  printf '%-22s expected %-5s got %-5s %s\n' "$bin" "$expect" "$got" "$status"
  if [ "$got" = ub ]; then echo "$out" | grep -E "^error: (Undefined Behavior|.*data race)" | head -1 | sed 's/^/    /'; fi
}
check set_len_uninit ub
check set_len_fixed clean
check uninit_assume_init ub
check uninit_fixed clean
check send_race ub
check send_bounded clean
exit $fail
