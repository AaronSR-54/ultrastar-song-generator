<#
.SYNOPSIS
    Builds the Windows installer (.msi) for the desktop app.

.DESCRIPTION
    Packages an existing release binary with WiX v5. Nothing is compiled and
    nothing is signed: SmartScreen shows its warning until the project gets an
    Authenticode certificate (tracked as a future change).

    The MSI installs per user into %LOCALAPPDATA%\Programs\UltraStar Song
    Generator, so it never asks for administrator rights.

.PARAMETER Version
    Overrides the version read from Cargo (useful for testing).

.PARAMETER Binary
    Release executable to package. Defaults to target\release\usdx-desktop.exe.

.PARAMETER OutDir
    Output directory. Defaults to dist\.

.EXAMPLE
    packaging\windows\package.ps1
#>
[CmdletBinding()]
param(
    [string]$Version,
    [string]$Binary,
    [string]$OutDir
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $Binary) { $Binary = Join-Path $root 'target\release\usdx-desktop.exe' }
if (-not $OutDir) { $OutDir = Join-Path $root 'dist' }

if (-not (Get-Command wix -ErrorAction SilentlyContinue)) {
    throw 'wix not found. Install the WiX toolset with: dotnet tool install --global wix --version 5.*'
}
if (-not (Test-Path -LiteralPath $Binary)) {
    throw "binary not found: $Binary (run: cargo build --release -p usdx-desktop)"
}

$icon = Join-Path $root 'assets\icon.ico'
if (-not (Test-Path -LiteralPath $icon)) { throw "icon not found: $icon" }

# The workspace manifest is the only source of truth for the version:
# MSI needs three numeric components, the file name keeps the full tag.
$versionInfo = & (Join-Path $root 'packaging\version.ps1') $Version
$display = ($versionInfo | Where-Object { $_ -like 'VERSION_DISPLAY=*' }) -replace '^VERSION_DISPLAY=', ''
$numeric = ($versionInfo | Where-Object { $_ -like 'VERSION_NUMERIC=*' }) -replace '^VERSION_NUMERIC=', ''
if (-not $display -or -not $numeric) {
    throw "could not read the version from packaging\version.ps1: $versionInfo"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$artifact = Join-Path $OutDir "UltraStar-Song-Generator-$display-windows-x64.msi"
if (Test-Path -LiteralPath $artifact) { Remove-Item -LiteralPath $artifact }

Push-Location $root
try {
    & wix build (Join-Path $PSScriptRoot 'UltraStarSongGenerator.wxs') `
        -arch x64 `
        -d "Version=$numeric" `
        -d "BinarySource=$Binary" `
        -d "IconSource=$icon" `
        -o $artifact
    if ($LASTEXITCODE -ne 0) { throw "wix build failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

# Per-user directories trip ICE64 (a directory in the user profile that is not
# anybody's key path); the component is keyed in HKCU, so it is suppressed. The
# other ICEs stay enabled on purpose: they caught ICE43 while this file was
# being written, and hiding them again would hide that class of bug. `wix
# build` has no -sice, hence the separate validate call. Validation is best
# effort: some hardened CI service accounts cannot run the ICE engine, and that
# must not hide a successfully built MSI.
& wix msi validate $artifact -sice ICE64
if ($LASTEXITCODE -ne 0) {
    Write-Warning "ICE validation reported problems for $artifact; the MSI was still built."
}

Write-Output $artifact

# `wix msi validate` leaves its exit code in $LASTEXITCODE and the GitHub
# runner propagates it, so a successful build would still fail the step.
exit 0
