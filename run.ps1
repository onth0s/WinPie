<#
.SYNOPSIS
    Convenience script to run WinPie (debug or release).
.DESCRIPTION
    Launches the WinPie radial interaction system.
.PARAMETER Release
    Runs the release build instead of debug.
.PARAMETER BuildOnly
    Delegates to .\build.ps1 to compile without launching.
.EXAMPLE
    .\run.ps1
    .\run.ps1 -Release
    .\run.ps1 -BuildOnly
#>
param(
    [switch]$Release,
    [switch]$BuildOnly
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

if ($BuildOnly) {
    if ($Release) {
        & "$PSScriptRoot\build.ps1" -Release
    } else {
        & "$PSScriptRoot\build.ps1"
    }
    exit $LASTEXITCODE
}

$mode = if ($Release) { "Release" } else { "Debug" }

Write-Host "Starting WinPie ($mode)..." -ForegroundColor Cyan
Write-Host "Press Win+Esc to open radial menu. Left-click/Win-up to commit. Right-click to cancel." -ForegroundColor DarkGray
Write-Host "Press Ctrl+C in this terminal to exit.`n" -ForegroundColor DarkGray

if ($Release) {
    cargo run --release -- --foreground
} else {
    cargo run -- --foreground
}
