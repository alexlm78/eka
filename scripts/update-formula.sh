#!/usr/bin/env bash
# Fills the Homebrew formula template with the version and the checksums of a
# release. SHA256SUMS is the file the release workflow attaches to the release.
#
# usage: scripts/update-formula.sh VERSION SHA256SUMS [OUTPUT]
set -euo pipefail

version=${1:?version, for example 0.5.9}
sums=${2:?path to SHA256SUMS}
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
output=${3:-"$root/packaging/homebrew/eka-ls.rb"}

formula=$(cat "$root/packaging/homebrew/eka-ls.rb.in")
formula=${formula//@VERSION@/$version}

for target in aarch64-apple-darwin x86_64-apple-darwin \
  aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu; do
  sha=$(awk -v f="eka-${version}-${target}.tar.gz" '$2 == f { print $1 }' "$sums")
  if [ -z "$sha" ]; then
    echo "no checksum for $target in $sums" >&2
    exit 1
  fi
  field=$(printf '%s' "$target" | tr 'a-z-' 'A-Z_')
  formula=${formula//@SHA_${field}@/$sha}
done

printf '%s\n' "$formula" >"$output"
echo "$output: eka-ls v${version}"
