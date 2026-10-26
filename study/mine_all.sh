#!/usr/bin/env bash
# Mines every repository in a list, cloned under CORPUS, into an output directory.
# Usage: study/mine_all.sh CORPUS [LIST] [OUT_DIR] [ANTI_SLOP_BINARY]
#   discovery:   study/mine_all.sh ~/corpus study/repos.txt study/data
#   replication: study/mine_all.sh ~/corpus-rep study/repos-replication.txt study/data-replication
set -uo pipefail
cd "$(dirname "$0")/.."
corpus=$1
list=${2:-study/repos.txt}
out=${3:-study/data}
bin=${4:-target/release/anti-slop}
mkdir -p "$out"
while read -r repo; do
    echo "${repo#*/}"
done < "$list" | xargs -P 4 -I{} sh -c "python3 study/mine.py '$corpus/{}' '$bin' '$out/{}.csv' 2>&1 | tail -1"
