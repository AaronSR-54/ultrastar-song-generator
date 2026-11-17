# Prints the product version derived from the workspace manifest.
#
# Mirrors packaging/version.sh for Windows jobs. The workspace version is the
# only source of truth: no installer manifest repeats it.
#
# Usage:
#   packaging\version.ps1              # read the version from Cargo
#   packaging\version.ps1 0.2.0-rc.1   # normalize an explicit version (testing)
#
# Output (one per line):
#   VERSION_DISPLAY=0.2.0-rc.1   # full semver, used in file names
#   VERSION_NUMERIC=0.2.0        # three numeric components, used by MSI and plist
param([string]$Version)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot

if (-not $Version) {
    $packageId = & cargo pkgid --manifest-path (Join-Path $root 'Cargo.toml') -p usdx-desktop
    if ($LASTEXITCODE -ne 0) { throw 'cargo pkgid failed' }
    $Version = ($packageId -split '@')[-1].Trim()
}

if ($Version -notmatch '^(\d+)\.(\d+)\.(\d+)') {
    throw "cannot parse version '$Version'"
}

$numeric = '{0}.{1}.{2}' -f $Matches[1], $Matches[2], $Matches[3]

Write-Output "VERSION_DISPLAY=$Version"
Write-Output "VERSION_NUMERIC=$numeric"
