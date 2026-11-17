#!/usr/bin/env bash
# Prints the product version derived from the workspace manifest.
#
# The workspace version is the only source of truth: every installer reads it
# from here instead of repeating a version in its own manifest.
#
# Usage:
#   packaging/version.sh            # read the version from Cargo
#   packaging/version.sh 0.2.0-rc.1 # normalize an explicit version (testing)
#
# Output (one per line):
#   VERSION_DISPLAY=0.2.0-rc.1   # full semver, used in file names
#   VERSION_NUMERIC=0.2.0        # three numeric components, used by MSI and plist
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ $# -ge 1 ]]; then
  version="$1"
else
  package_id="$(cargo pkgid --manifest-path "$root/Cargo.toml" -p usdx-desktop)"
  version="${package_id##*@}"
fi

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]]; then
  echo "error: cannot parse version '$version'" >&2
  exit 1
fi

core="${version%%[-+]*}"
IFS='.' read -r major minor patch <<< "$core"
patch="${patch%%[^0-9]*}"

printf 'VERSION_DISPLAY=%s\n' "$version"
printf 'VERSION_NUMERIC=%s.%s.%s\n' "$major" "$minor" "$patch"
