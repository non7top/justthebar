#!/bin/sh
# Stages the release files and builds the installer: build.sh <version> <dir with jtb-app.exe and jtb_hook.dll> <output exe>
# PROVENANCE_REPO (owner/repo), when set, is embedded as the installer's provenance claim.
set -eu
version=$1
dist=$2
out=$3
root=$(cd "$(dirname "$0")/.." && pwd)
numeric=$(printf '%s' "$version" | sed -n 's/^\([0-9][0-9]*\)\.\([0-9][0-9]*\)\.\([0-9][0-9]*\).*/\1.\2.\3.0/p')
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cp "$dist/jtb-app.exe" "$dist/jtb_hook.dll" "$stage/"
cp "$root"/scripts/*.cmd "$root/LICENSE" "$root/NOTICE" "$root/config/jtb.example.json" "$stage/"
set -- -V2 "-DVERSION=$version" "-DVERSION_NUMERIC=${numeric:-0.0.0.0}" "-DSTAGE=$stage" "-DOUTFILE=$(realpath "$out")"
if [ -n "${PROVENANCE_REPO:-}" ]; then
  set -- "$@" "-DPROVENANCE_REPO=$PROVENANCE_REPO"
fi
makensis "$@" "$root/installer/jtb.nsi"
