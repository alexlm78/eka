#!/usr/bin/env bash
# Regenerates the checked-in shell completions from the built binary.
# Run it after changing the command-line options; `cargo test` fails when the
# files are out of date.
#
# usage: scripts/gen-completions.sh [PATH-TO-EKA]
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
eka=${1:-"$root/target/release/eka"}

mkdir -p "$root/completions"
"$eka" --completions bash >"$root/completions/eka.bash"
"$eka" --completions zsh >"$root/completions/_eka"
"$eka" --completions fish >"$root/completions/eka.fish"
echo "completions written to $root/completions"
