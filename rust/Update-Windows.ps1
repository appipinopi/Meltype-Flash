# SPDX-License-Identifier: GPL-3.0-or-later
# Download only a published Rust Windows package from this fork. No arbitrary URLs.
[CmdletBinding(SupportsShouldProcess=$true)]
param([ValidateSet('Check','Stage','Apply')][string]$Action = 'Check')
$ErrorActionPreference = 'Stop'
$repository = 'appipinopi/Meltype-Flash'
$limit = 268435456L
function Installed-Root {
    $machine = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
    try {
        $key = $machine.OpenSubKey('SOFTWARE\Microsoft\Windows\CurrentVersion')
        try { return (Join-Path ([string]$key.GetValue('ProgramFilesDir')) 'Meltype-FLASH') } finally { $key.Dispose() }
    } finally { $machine.Dispose() }
}
if ($Action -eq 'Apply' -and -not $WhatIfPreference) {
    $installedRoot = Installed-Root
    if (-not (Test-Path -LiteralPath (Join-Path $installedRoot 'installed.json'))) { throw 'No managed Rust IME installation. Use Manage-Windows.ps1 -Action Install first; portable use supports Stage only.' }
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        $shell = Join-Path ([Environment]::GetFolderPath('Windows')) 'System32/WindowsPowerShell/v1.0/powershell.exe'
        $process = Start-Process -FilePath $shell -Verb RunAs -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',('"'+$PSCommandPath+'"'),'-Action','Apply') -WindowStyle Hidden -Wait -PassThru
        try { if ($process.ExitCode -ne 0) { throw "Elevated updater failed: $($process.ExitCode)" } } finally { $process.Dispose() }
        $installed = Get-Content -LiteralPath (Join-Path $installedRoot 'installed.json') -Raw | ConvertFrom-Json
        $newExe = [IO.Path]::GetFullPath((Join-Path ([string]$installed.package) 'meltype-flash.exe'))
        if (-not $newExe.StartsWith($installedRoot+'\versions\',[StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $newExe -PathType Leaf)) { throw 'Invalid updated tray path' }
        Start-Process -FilePath $newExe -ArgumentList '--tray' -WindowStyle Hidden | Out-Null
        Write-Host "Updated tray started. Installation logs: $installedRoot\update-logs"
        return
    }
    $logRoot = Join-Path $installedRoot 'update-logs'
    foreach ($path in @($installedRoot,$logRoot)) {
        if ((Test-Path -LiteralPath $path) -and ((Get-Item -LiteralPath $path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Reparse point in update log path' }
    }
    New-Item -ItemType Directory -Path $logRoot -Force | Out-Null
    Start-Transcript -LiteralPath (Join-Path $logRoot (([Guid]::NewGuid().ToString('N'))+'.log')) | Out-Null
}
function Get-Releases {
    Invoke-RestMethod -Uri "https://api.github.com/repos/$repository/releases?per_page=20" -Headers @{ 'User-Agent' = 'Meltype-flash-Updater'; Accept = 'application/vnd.github+json' } -TimeoutSec 20
}
function Select-Release($Releases) {
    foreach ($release in $Releases) {
        if ($release.draft -or $release.prerelease) { continue }
        foreach ($asset in $release.assets) {
            if ($asset.name -notmatch '(?i)^Meltype[-_.]flash[-_.]Windows[-_.a-z0-9]*\.zip$') { continue }
            if ($asset.size -le 0 -or $asset.size -gt $limit) { continue }
            if ($asset.digest -notmatch '^sha256:[a-fA-F0-9]{64}$') { continue }
            $expected = "https://github.com/$repository/releases/download/$([Uri]::EscapeDataString([string]$release.tag_name))/$([Uri]::EscapeDataString([string]$asset.name))"
            if ($asset.browser_download_url -cne $expected) { continue }
            return @{ release = $release; asset = $asset; uri = $expected; sha256 = $asset.digest.Substring(7) }
        }
    }
    return $null
}
function Download-Asset([string]$Uri, [string]$Destination, [long]$ExpectedSize) {
    Add-Type -AssemblyName System.Net.Http
    $handler = New-Object Net.Http.HttpClientHandler
    $handler.AllowAutoRedirect = $false
    $client = New-Object Net.Http.HttpClient($handler)
    $client.Timeout = [TimeSpan]::FromSeconds(120)
    $client.DefaultRequestHeaders.UserAgent.ParseAdd('Meltype-flash-Updater')
    $response = $null; $input = $null; $output = $null
    $cancellation = New-Object Threading.CancellationTokenSource
    $cancellation.CancelAfter(120000)
    try {
        for ($redirect = 0; $redirect -le 5; $redirect++) {
            $target = [Uri]$Uri
            if ($target.Scheme -ne 'https' -or $target.UserInfo -or -not $target.IsDefaultPort -or $target.Host -notin @('github.com','release-assets.githubusercontent.com','objects.githubusercontent.com')) {
                throw 'Untrusted update download host'
            }
            $response = $client.GetAsync($target, [Net.Http.HttpCompletionOption]::ResponseHeadersRead).GetAwaiter().GetResult()
            if ([int]$response.StatusCode -in @(301,302,303,307,308)) {
                if ($redirect -eq 5 -or $null -eq $response.Headers.Location) { throw 'Too many update redirects' }
                $Uri = ([Uri]::new($target, $response.Headers.Location)).AbsoluteUri
                $response.Dispose(); $response = $null
                continue
            }
            $null = $response.EnsureSuccessStatusCode()
            break
        }
        if ($response.Content.Headers.ContentLength -and $response.Content.Headers.ContentLength -ne $ExpectedSize) { throw 'Update length mismatch' }
        $input = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
        $output = [IO.File]::Open($Destination, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
        $buffer = New-Object byte[] 65536
        $total = 0L
        $deadline = [DateTime]::UtcNow.AddMinutes(2)
        while (($count = $input.ReadAsync($buffer,0,$buffer.Length,$cancellation.Token).GetAwaiter().GetResult()) -gt 0) {
            $total += $count
            if ($total -gt $ExpectedSize -or $total -gt $limit -or [DateTime]::UtcNow -gt $deadline) { throw 'Update download exceeds bounds' }
            $output.Write($buffer,0,$count)
        }
        if ($total -ne $ExpectedSize) { throw 'Incomplete update download' }
    } finally {
        if ($null -ne $output) { $output.Dispose() }
        if ($null -ne $input) { $input.Dispose() }
        if ($null -ne $response) { $response.Dispose() }
        $client.Dispose(); $handler.Dispose(); $cancellation.Dispose()
    }
}
function Expand-Checked([string]$Archive, [string]$Destination) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $prefix = [IO.Path]::GetFullPath($Destination).TrimEnd('\')+'\'
        $paths = @{}; $expanded = 0L
        foreach ($entry in $zip.Entries) {
            $expanded += $entry.Length
            if ($expanded -gt 1073741824L -or $zip.Entries.Count -gt 4096) { throw 'Update archive exceeds extraction bounds' }
            if ($entry.FullName -match '[:\\]' -or [IO.Path]::IsPathRooted($entry.FullName) -or $entry.FullName -match '(^|/)\.\.(/|$)') { throw 'Unsafe update archive path' }
            $path = [IO.Path]::GetFullPath((Join-Path $Destination $entry.FullName))
            if (-not $path.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase) -or $paths.ContainsKey($path)) { throw 'Duplicate or escaping update archive path' }
            $paths[$path] = $true
            # Unix symlinks and special files must not enter a privileged package.
            $kind = ($entry.ExternalAttributes -shr 16) -band 0xF000
            if ($kind -notin @(0,0x8000,0x4000)) { throw 'Update archive contains special files' }
        }
        New-Item -ItemType Directory -Path $Destination | Out-Null
        [IO.Compression.ZipFile]::ExtractToDirectory($Archive,$Destination)
    } finally { $zip.Dispose() }
}
$chosen = Select-Release @(Get-Releases)
if ($null -eq $chosen) { throw 'No stable Rust Windows ZIP with a GitHub SHA256 digest is published in this fork.' }
$receiptPath = Join-Path $PSScriptRoot 'update-receipt.json'
$receipt = if (Test-Path -LiteralPath $receiptPath) { Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json } else { $null }
if ($receipt.releaseId -eq $chosen.release.id -and $receipt.sha256 -eq $chosen.sha256) {
    Write-Host 'This package is already current.'
    return
}
Write-Host "Rust Windows update: $($chosen.release.tag_name) / $($chosen.asset.name) ($($chosen.asset.size) bytes)"
if ($Action -eq 'Check') { return }
if (-not $PSCmdlet.ShouldProcess($chosen.uri, "$Action verified Rust Windows update")) { return }
$stage = Join-Path ([IO.Path]::GetTempPath()) ('Meltype-flash-update-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
$archive = Join-Path $stage 'update.zip'
Download-Asset $chosen.uri $archive $chosen.asset.size
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $chosen.sha256) { throw 'GitHub release digest mismatch. Update was not applied.' }
$package = Join-Path $stage 'package'
Expand-Checked $archive $package
$manager = Join-Path $PSScriptRoot 'Manage-Windows.ps1'
& $manager -Action Inspect -Package $package
if (-not $?) { throw 'Downloaded package verification failed' }
Write-Host "Verified staged update: $package"
if ($Action -eq 'Stage') { return }
& $manager -Action Update -Package $package
if (-not $?) { throw 'Update failed. Previous version files were retained.' }
# Record in the newly installed version, not the old source folder.
$machine = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
try {
    $key = $machine.OpenSubKey('SOFTWARE\Microsoft\Windows\CurrentVersion')
    try { $programFiles = [string]$key.GetValue('ProgramFilesDir') } finally { $key.Dispose() }
} finally { $machine.Dispose() }
$installRoot = Join-Path $programFiles 'Meltype-FLASH'
$state = Get-Content -LiteralPath (Join-Path $installRoot 'installed.json') -Raw | ConvertFrom-Json
$expected = Join-Path (Join-Path $installRoot 'versions') ((Get-FileHash -LiteralPath (Join-Path $package 'SHA256SUMS.json')).Hash.Substring(0,16))
if ([IO.Path]::GetFullPath([string]$state.package) -ne [IO.Path]::GetFullPath($expected)) { throw 'Updated installation receipt does not match downloaded package' }
@{ releaseId = $chosen.release.id; sha256 = $chosen.sha256; tag = $chosen.release.tag_name } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $expected 'update-receipt.json') -Encoding UTF8
Write-Host 'Update installed. Launch the tray from the new version folder.'
