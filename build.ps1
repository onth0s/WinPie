<#
.SYNOPSIS
    WinPie build and verification automation script.
.DESCRIPTION
    Builds the WinPie binary (Debug or Release) and optionally runs Clippy linter,
    unit/integration tests, and cleans the target cache.
.PARAMETER Release
    Compiles with optimized release profile (--release).
.PARAMETER Test
    Runs full test suite (cargo test --all-targets).
.PARAMETER Clippy
    Runs cargo clippy with -D warnings to enforce code quality.
.PARAMETER Clean
    Cleans target cache before building.
.PARAMETER All
    Runs full verification pipeline: Clean -> Clippy -> Build -> Test.
.EXAMPLE
    .\build.ps1
.PARAMETER NoStart
    Skips automatically starting the WinPie daemon after compilation.
.EXAMPLE
    .\build.ps1
    .\build.ps1 -Release
    .\build.ps1 -Release -Test -Clippy
    .\build.ps1 -All
    .\build.ps1 -NoStart
#>
param(
    [switch]$Release,
    [switch]$Test,
    [switch]$Clippy,
    [switch]$Clean,
    [switch]$All,
    [switch]$NoStart
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

if ($All) {
    $Clean = $true
    $Clippy = $true
    $Test = $true
}

$startTime = [System.Diagnostics.Stopwatch]::StartNew()
$mode = if ($Release) { "Release" } else { "Debug" }

# 0. Clean up stale binaries and kill any running WinPie process before building/copying
$staleOld = Join-Path $env:USERPROFILE ".cargo\bin\winpie.exe.old"
if (Test-Path $staleOld) {
    Remove-Item -Path $staleOld -Force -ErrorAction SilentlyContinue
}

$running = Get-Process -Name winpie -ErrorAction SilentlyContinue
if ($running) {
    Write-Host "[KILL] Terminating existing WinPie process(es)..." -ForegroundColor Yellow
    $cargoBinExe = Join-Path $env:USERPROFILE ".cargo\bin\winpie.exe"
    if (Test-Path $cargoBinExe) {
        & $cargoBinExe kill 2>&1 | Out-Null
    }
    Start-Sleep -Milliseconds 200
    Get-Process -Name winpie -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 200
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  WinPie Build Pipeline ($mode)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 0.5 Validate YAML configs for common escaping errors
if (Test-Path "config.yaml") {
    $cfgContent = Get-Content "config.yaml" -Raw
    $cfgLines = $cfgContent -split "`r?`n"
    for ($i = 0; $i -lt $cfgLines.Length; $i++) {
        $line = $cfgLines[$i]
        if ($line -match '"[^"]*:[\\/][^"]*"') {
            if ($line -match '":\\[^\\"]') {
                Write-Host "[WARN] config.yaml line $($i + 1): Unescaped backslash in path (Auto-repair enabled at runtime):" -ForegroundColor Yellow
                Write-Host "       $($line.Trim())" -ForegroundColor DarkGray
            }
        }
    }
}

# 1. Clean if requested
if ($Clean) {
    Write-Host "[CLEAN] Cleaning build target..." -ForegroundColor Yellow
    cargo clean
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Clean failed."
        exit $LASTEXITCODE
    }
}

# 2. Clippy lint check
if ($Clippy) {
    Write-Host "[CLIPPY] Running linter checks with -D warnings..." -ForegroundColor Yellow
    cargo clippy --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Clippy checks failed with warnings/errors."
        exit $LASTEXITCODE
    }
    Write-Host "[CLIPPY] All linter checks passed (0 warnings)." -ForegroundColor Green
}

# 3. Compile Binary
Write-Host "[BUILD] Compiling WinPie ($mode)..." -ForegroundColor Yellow
if ($Release) {
    cargo build --release
} else {
    cargo build
}

if ($LASTEXITCODE -ne 0) {
    Write-Error "Build compilation failed."
    exit $LASTEXITCODE
}
Write-Host "[BUILD] Compilation successful." -ForegroundColor Green

# 4. Run Tests if requested
if ($Test) {
    Write-Host "[TEST] Running unit and integration test suite..." -ForegroundColor Yellow
    if ($Release) {
        cargo test --release --all-targets
    } else {
        cargo test --all-targets
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Tests failed."
        exit $LASTEXITCODE
    }
    Write-Host "[TEST] All tests passed." -ForegroundColor Green
}

$startTime.Stop()
$elapsedSec = [math]::Round($startTime.Elapsed.TotalSeconds, 2)

# Output artifact information
$targetDir = if ($Release) { "target\release" } else { "target\debug" }
$exePath = Join-Path $PSScriptRoot "$targetDir\winpie.exe"

Write-Host "`n----------------------------------------" -ForegroundColor DarkGray
if (Test-Path $exePath) {
    $item = Get-Item $exePath
    $sizeKb = [math]::Round($item.Length / 1KB, 1)
    $sizeMb = [math]::Round($item.Length / 1MB, 2)
    Write-Host "[SUCCESS] Artifact: $exePath" -ForegroundColor Green
    Write-Host "[INFO]    Binary Size: $sizeMb MB ($sizeKb KB)" -ForegroundColor DarkCyan

    # 5. Sync to PATH (.cargo\bin) for all builds; maintain Windows Startup Shortcut
    $appDataWinPie = Join-Path $env:APPDATA "WinPie"
    if (-not (Test-Path $appDataWinPie)) {
        New-Item -ItemType Directory -Path $appDataWinPie -Force | Out-Null
    }
    if (Test-Path "config.yaml") {
        Copy-Item -Path "config.yaml" -Destination (Join-Path $appDataWinPie "config.yaml") -Force
        Write-Host "[CONFIG]  Synced config.yaml -> $appDataWinPie\config.yaml" -ForegroundColor Green
    }

    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    $canonicalExe = $exePath
    if (Test-Path $cargoBin) {
        $destPath = Join-Path $cargoBin "winpie.exe"
        try {
            Copy-Item -Path $exePath -Destination $destPath -Force -ErrorAction Stop
        } catch {
            Write-Host "[PATH]    Destination locked by running process; renaming and replacing..." -ForegroundColor Yellow
            $destOld = "$destPath.old"
            if (Test-Path $destOld) {
                Remove-Item -Path $destOld -Force -ErrorAction SilentlyContinue
            }
            Move-Item -Path $destPath -Destination $destOld -Force -ErrorAction SilentlyContinue
            Copy-Item -Path $exePath -Destination $destPath -Force
        }
        Write-Host "[PATH]    Synced to $destPath (Run 'winpie' from any terminal)" -ForegroundColor Green
        $canonicalExe = $destPath
    }

    $startupFolder = [System.IO.Path]::Combine($env:APPDATA, "Microsoft\Windows\Start Menu\Programs\Startup")
    if (Test-Path $startupFolder) {
        $shortcutPath = Join-Path $startupFolder "WinPie.lnk"
        if ($Release -or (Test-Path $shortcutPath)) {
            $wscript = New-Object -ComObject WScript.Shell
            $shortcut = $wscript.CreateShortcut($shortcutPath)
            $shortcut.TargetPath = $canonicalExe
            $shortcut.WorkingDirectory = $PSScriptRoot
            $shortcut.Description = "WinPie Radial Menu Daemon"
            $shortcut.Save()
            Write-Host "[STARTUP] Updated Startup Shortcut: $shortcutPath -> $canonicalExe" -ForegroundColor Green
        }
    }

    # 6. Automatically start the WinPie background daemon unless -NoStart was requested
    if (-not $NoStart) {
        Write-Host "[START]   Launching WinPie daemon in background..." -ForegroundColor Green
        Start-Process -FilePath $exePath -WorkingDirectory $PSScriptRoot
        Start-Sleep -Milliseconds 300
        $runningDaemon = Get-Process -Name winpie -ErrorAction SilentlyContinue
        if ($runningDaemon) {
            Write-Host "[RUNNING] WinPie daemon is active (PID $($runningDaemon[0].Id)). Press Win+Esc to activate!" -ForegroundColor Green
        } else {
            Write-Host "[WARN]    WinPie did not report as running after launch." -ForegroundColor Yellow
        }
    } else {
        Write-Host "[SKIP]    Daemon startup skipped (-NoStart specified)." -ForegroundColor DarkGray
    }
}
Write-Host "[INFO]    Elapsed Time: $elapsedSec s" -ForegroundColor DarkCyan
Write-Host "----------------------------------------`n" -ForegroundColor DarkGray
