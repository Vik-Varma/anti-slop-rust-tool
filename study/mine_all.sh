#!/usr/bin/env bash
# Mines every repository listed in study/repos.txt, cloned under CORPUS, into study/data/.
# Usage: study/mine_all.sh CORPUS [ANTI_SLOP_BINARY]
set -uo pipefail
cd "$(dirname "$0")/.."
corpus=$1
bin=${2:-target/release/anti-slop}
mkdir -p study/data
while read -r repo; do
    name=${repo#*/}
    echo "$name"
done < study/repos.txt | xargs -P 4 -I{} sh -c "python3 study/mine.py '$corpus/{}' '$bin' 'study/data/{}.csv' 2>&1 | tail -1"
