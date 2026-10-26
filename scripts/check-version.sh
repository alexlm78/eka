#!/usr/bin/env bash
# Checks that the version in Cargo.toml is ready to be tagged:
#   - the newest CHANGELOG.md section is this version, and it has notes
#     (the release workflow stops without them);
#   - the patch number is odd (see docs/ROADMAP.md, "Versioning").
# Cargo.lock matching Cargo.toml is checked by `cargo build --locked`.
#
# usage: scripts/check-version.sh
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)
newest=$(sed -n 's/^## \[\([0-9][^]]*\)\].*/\1/p' "$root/CHANGELOG.md" | head -n 1)

if [ "$newest" != "$version" ]; then
  echo "error: Cargo.toml is at $version but the newest CHANGELOG.md section is ${newest:-missing}" >&2
  echo "       add a '## [$version] - YYYY-MM-DD' section (not '[Unreleased]')" >&2
  exit 1
fi
"$root/scripts/release-notes.sh" "$version" >/dev/null

patch=${version##*.}
if [ $((patch % 2)) -ne 1 ]; then
  echo "error: version $version has an even patch number; commits use odd ones" >&2
  exit 1
fi
echo "version $version: changelog section and patch number ok"
