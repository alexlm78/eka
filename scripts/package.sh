#!/usr/bin/env bash
# Packages a built eka binary into OUTDIR/eka-VERSION-TARGET.tar.gz plus its checksum.
#
# usage: scripts/package.sh VERSION TARGET BINARY [OUTDIR]
set -euo pipefail

version=${1:?version}
target=${2:?target triple}
binary=${3:?path to the eka binary}
outdir=${4:-dist}

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
name="eka-${version}-${target}"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

mkdir -p "$stage/$name" "$outdir"
install -m 755 "$binary" "$stage/$name/eka"
install -m 644 "$root/docs/eka.1" "$root/README.md" "$root/LICENSE" "$root/CHANGELOG.md" "$stage/$name/"
mkdir -p "$stage/$name/completions"
install -m 644 "$root"/completions/* "$stage/$name/completions/"

archive="$outdir/$name.tar.gz"
tar -C "$stage" -czf "$archive" "$name"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$outdir" && sha256sum "$name.tar.gz" >"$name.tar.gz.sha256")
else
  (cd "$outdir" && shasum -a 256 "$name.tar.gz" >"$name.tar.gz.sha256")
fi
echo "$archive"
