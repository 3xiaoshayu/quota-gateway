[CmdletBinding()]
param(
    [int]$BuildJobs
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RootDir = $PSScriptRoot
$AppBin = Join-Path $RootDir 'src-tauri\target\release\quota-gateway.exe'
$CopyScript = Join-Path $RootDir 'copy.ps1'

Set-Location -LiteralPath $RootDir

if (-not $PSBoundParameters.ContainsKey('BuildJobs')) {
    $BuildJobs = if ($env:CARGO_BUILD_JOBS) {
        [int]$env:CARGO_BUILD_JOBS
    }
    else {
        16
    }
}
if ($BuildJobs -lt 1 -or $BuildJobs -gt 256) {
    throw 'BuildJobs must be between 1 and 256.'
}

if (-not (Get-Command bun -ErrorAction SilentlyContinue)) {
    throw 'bun is not installed or not in PATH.'
}

Write-Host "Cargo build jobs: $BuildJobs"

& bun install
if ($LASTEXITCODE -ne 0) {
    throw "bun install failed with exit code $LASTEXITCODE."
}

$PreviousBuildJobs = $env:CARGO_BUILD_JOBS
try {
    $env:CARGO_BUILD_JOBS = [string]$BuildJobs
    & bun tauri build --no-bundle
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri build failed with exit code $LASTEXITCODE."
    }
}
finally {
    if ($null -eq $PreviousBuildJobs) {
        Remove-Item Env:CARGO_BUILD_JOBS -ErrorAction SilentlyContinue
    }
    else {
        $env:CARGO_BUILD_JOBS = $PreviousBuildJobs
    }
}

if (-not (Test-Path -LiteralPath $AppBin -PathType Leaf)) {
    throw "Build finished, but executable not found: $AppBin"
}

Write-Host "Built: $AppBin"
& $CopyScript
