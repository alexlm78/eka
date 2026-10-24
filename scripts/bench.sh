#!/usr/bin/env bash
# Compares eka with the system ls (and gls, eza when installed) using hyperfine.
# Builds two fixtures in a temporary directory: 20,000 files in one directory
# and 20,000 files spread over 2,200 directories.
#
# usage: scripts/bench.sh [path/to/eka]    (default: target/release/eka)
#        RUNS=30 scripts/bench.sh          (default: 15 runs per command)
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
eka=${1:-$root/target/release/eka}
command -v hyperfine >/dev/null || { echo "hyperfine is required" >&2; exit 1; }
[ -x "$eka" ] || { echo "build first: cargo build --release" >&2; exit 1; }

dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
python3 - "$dir" <<'PY'
import os, sys
base = sys.argv[1]
os.makedirs(base + "/flat")
for i in range(20000):
    open(f"{base}/flat/file_{i:05d}.txt", "w").close()
for d in range(200):
    for s in range(10):
        path = f"{base}/tree/d{d:03d}/s{s}"
        os.makedirs(path)
        for f in range(10):
            open(f"{path}/f{f}", "w").close()
PY

tools=("$eka" "/bin/ls")
for extra in gls eza; do
  if path=$(command -v "$extra" 2>/dev/null) && [ -x "$path" ]; then tools+=("$path"); fi
done

bench() {
  local label=$1 flags=$2 target=$3 cmds=()
  for tool in "${tools[@]}"; do cmds+=("$tool $flags $target"); done
  echo "## $label"
  hyperfine -N --warmup 3 --runs "${RUNS:-15}" "${cmds[@]}"
}

bench "20,000 files, names only (-1)" "-1" "$dir/flat"
bench "20,000 files, long format (-l)" "-l" "$dir/flat"
bench "2,200 directories, recursive (-R)" "-R" "$dir/tree"
