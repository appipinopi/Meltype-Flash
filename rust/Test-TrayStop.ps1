# SPDX-License-Identifier: GPL-3.0-or-later
# Isolated real tray and cooperative shutdown. Never registers an IME or changes startup.
param([Parameter(Mandatory=$true)][string]$Binary, [switch]$Portable)
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path -LiteralPath $Binary).Path
$root = Join-Path ([IO.Path]::GetTempPath()) ('meltype tray stop 日本語-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$configPath = Join-Path $root 'config.json'
$settingsVersion = (Get-Content -LiteralPath (Join-Path $PSScriptRoot 'core/data/settings-defaults.json') -Raw -Encoding UTF8 | ConvertFrom-Json).SettingsVersion
$config = [ordered]@{SettingsVersion=$settingsVersion;WelcomeShown=$true;TsfIntroduced=$true;AutoUpdate=$false;Enabled=$true} | ConvertTo-Json -Compress
[IO.File]::WriteAllText($configPath, $config)
$previousDataDirectory = $env:MELTYPE_RUST_DATA_DIR
$previousFlashDirectory = $env:MELTYPE_FLASH_DATA_DIR
$tray = $null; $helper = $null
try {
    $env:MELTYPE_RUST_DATA_DIR = $root
    $env:MELTYPE_FLASH_DATA_DIR = $root
    if ($Portable) {
        # --portable-keyboard intentionally selects Keyboard mode at startup.
        # Prepare its canonical configuration first; shutdown must preserve these
        # exact bytes, including Enabled, rather than compare against TSF mode.
        $cli = Join-Path (Split-Path -Parent $exe) 'meltype-rs.exe'
        & $cli --control '{"op":"settings_set","name":"Mode","value":"Keyboard"}' | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Could not prepare portable configuration' }
        $config = [IO.File]::ReadAllText($configPath)
        $prepared = $config | ConvertFrom-Json
        if ($prepared.Mode -ne 'Keyboard' -or $prepared.Enabled -ne $true) { throw 'Invalid portable fixture' }
    }
    $trayArguments = if ($Portable) { @('--tray','--portable-keyboard') } else { @('--tray') }
    $tray = Start-Process -FilePath $exe -ArgumentList $trayArguments -WindowStyle Hidden -PassThru -RedirectStandardError (Join-Path $root 'tray.log')
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    $ready = $false
    while ([DateTime]::UtcNow -lt $deadline) {
        if ($tray.HasExited) {
            $diagnostic = Get-Content -LiteralPath (Join-Path $root 'tray.log') -Raw -ErrorAction SilentlyContinue
            throw "Tray exited early: $($tray.ExitCode). Stderr: $diagnostic"
        }
        $lockPath = Join-Path $root '.tray.lock'
        if (Test-Path -LiteralPath $lockPath) {
            $lock = [IO.File]::Open($lockPath, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::ReadWrite)
            try {
                try { $lock.Lock(0,1); $lock.Unlock(0,1) } catch [IO.IOException] { $ready = $true }
            } finally { $lock.Dispose() }
        }
        if ($ready) { break }
        Start-Sleep -Milliseconds 100
    }
    if (-not $ready) { throw 'Tray never acquired its singleton lock' }
    Start-Sleep -Milliseconds 1000
    $report = Join-Path $root 'stop-result.json'
    $helper = Start-Process -FilePath $exe -ArgumentList @('--startup-action','stop-tray','--startup-result',('"'+$report+'"')) -WindowStyle Hidden -PassThru -RedirectStandardError (Join-Path $root 'helper.log')
    $null = $helper.Handle # Retain a process handle before a fast helper exits.
    if (-not $helper.WaitForExit(15000)) { throw 'Stop helper timed out' }
    if ($helper.ExitCode -ne 0) { throw "Stop helper failed: $(Get-Content -LiteralPath $report -Raw)" }
    if (-not $tray.WaitForExit(5000)) { throw 'Tray lock released but process did not exit' }
    $result = Get-Content -LiteralPath $report -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($result.ok -ne $true -or $result.changed -ne $true) { throw 'Tray stop was not acknowledged' }
    if ((Get-Content -LiteralPath $configPath -Raw) -ne $config) { throw 'Tray shutdown modified Enabled or other settings' }
    Write-Host "PASS: real tray shutdown acknowledged, process exited, config preserved. Results: $root"
} finally {
    foreach ($child in @($helper,$tray)) {
        if ($null -ne $child) {
            if (-not $child.HasExited) { $child.Kill(); $child.WaitForExit() }
            $child.Dispose()
        }
    }
    $env:MELTYPE_RUST_DATA_DIR = $previousDataDirectory
    $env:MELTYPE_FLASH_DATA_DIR = $previousFlashDirectory
}
