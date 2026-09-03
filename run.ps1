<#
.SYNOPSIS
    Convenience script to run WinPie (debug or release).
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
        cargo build --release
    } else {
        cargo build
    }
    exit $LASTEXITCODE
}

Write-Host "Starting WinPie..." -ForegroundColor Cyan
Write-Host "Press Win+Esc to open radial menu. Left-click to select. Right-click to cancel." -ForegroundColor DarkGray
Write-Host "Press Ctrl+C in this terminal to exit.`n" -ForegroundColor DarkGray

if ($Release) {
    cargo run --release
} else {
    cargo run
}
