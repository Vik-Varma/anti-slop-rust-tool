#!/usr/bin/env bash
# Builds lib.rs with an old and a new compiler. Needs `rustup toolchain install 1.76`.
set -uo pipefail
cd "$(dirname "$0")"
out=$(mktemp -d)
for tc in 1.76 1.99; do
    if rustc +"$tc" --edition 2021 --crate-type lib -o "$out/lib-$tc.rlib" lib.rs 2> "$out/err-$tc"; then
        echo "rust $tc: builds"
    else
        echo "rust $tc: FAILS: $(grep -m1 '^error' "$out/err-$tc")"
    fi
done
