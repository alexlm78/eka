#!/usr/bin/env bash
# Points the Homebrew formula at the source tarball of a release tag and fills
# in its checksum. Run it once the tag for VERSION exists on GitHub.
#
# usage: scripts/update-formula.sh VERSION [FORMULA]
set -euo pipefail

version=${1:?version, for example 0.4.0}
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
formula=${2:-"$root/packaging/homebrew/eka.rb"}

url="https://github.com/alexlm78/eka/archive/refs/tags/v${version}.tar.gz"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

curl -fsSL -o "$work/source.tar.gz" "$url"
if command -v sha256sum >/dev/null 2>&1; then
  sum=$(sha256sum "$work/source.tar.gz")
else
  sum=$(shasum -a 256 "$work/source.tar.gz")
fi
sha=${sum%% *}

sed -E \
  -e "s|^( *url \").*(\")\$|\\1${url}\\2|" \
  -e "s|^( *sha256 \").*(\")\$|\\1${sha}\\2|" \
  "$formula" >"$work/formula.rb"
cat "$work/formula.rb" >"$formula"
echo "$formula: v${version} sha256 ${sha}"
