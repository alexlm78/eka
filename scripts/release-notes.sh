#!/usr/bin/env bash
# Prints the CHANGELOG.md section of one version, without its heading.
# Exits with an error when the version has no section or the section is empty.
#
# usage: scripts/release-notes.sh VERSION
set -euo pipefail

version=${1:?version}
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

notes=$(awk -v v="$version" '
  /^## \[/ {
    if (found) exit
    if (index($0, "## [" v "]") == 1) { found = 1; next }
  }
  # link reference definitions at the end of the file are not release notes
  /^\[[^]]+\]: / { if (found) exit }
  found { print }
' "$root/CHANGELOG.md")

if [ -z "$(printf '%s' "$notes" | tr -d '[:space:]')" ]; then
  echo "no release notes for $version in CHANGELOG.md" >&2
  exit 1
fi
printf '%s\n' "$notes"
