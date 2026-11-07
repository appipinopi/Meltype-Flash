# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
# Windows用shuntiの実行環境・辞書・モデルだけを個人領域へ導入する。
param([string]$Root, [switch]$FunctionsOnly)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)

function Shunti-Child([string]$Base, [string]$Name) {
    if ($Name -notmatch '^[a-zA-Z0-9][a-zA-Z0-9_.-]*$') { throw 'Unsafe shunti file name' }
    return [IO.Path]::Combine([IO.Path]::GetFullPath($Base), $Name)
}
function Shunti-Hash([string]$Path) {
    if ((Get-Item -LiteralPath $Path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Shunti links are not allowed' }
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}
function Shunti-Download([string]$Url, [string]$Path, [string]$Sha) {
    if ($Sha -notmatch '^[0-9a-fA-F]{64}$') { throw 'Missing shunti SHA256' }
    Invoke-WebRequest -Uri $Url -OutFile $Path -UseBasicParsing -TimeoutSec 180
    if ((Shunti-Hash $Path) -ne $Sha) { throw 'Shunti download SHA256 mismatch' }
}
function Shunti-Installed([string]$Directory) {
    if ((Get-Item -LiteralPath $Directory -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Shunti directory cannot be a link' }
    $receipt = Get-Content -LiteralPath (Join-Path $Directory 'receipt.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($name in @('shunti-helper.exe','kkc_lex.bin','kkc_model.bin','runtime-receipt.json','LICENSE','MELTYPE-LICENSE.txt','THIRD_PARTY_NOTICES.md','DATA_SOURCES.md','UPSTREAM.txt')) {
        if ($null -eq $receipt.files.PSObject.Properties[$name]) { throw 'Incomplete shunti environment receipt' }
    }
    foreach ($file in $receipt.files.PSObject.Properties) {
        if ((Shunti-Hash (Shunti-Child $Directory $file.Name)) -ne $file.Value) { throw 'Existing shunti environment failed verification' }
    }
    return @{engine='shunti';helper=(Shunti-Child $Directory 'shunti-helper.exe')}
}
function Shunti-Extract([string]$Archive, [string]$Stage) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($Archive)
    try {
        $seen = @{}; $bytes = 0L
        foreach ($entry in $zip.Entries) {
            $bytes += $entry.Length
            if ($bytes -gt 134217728L -or $zip.Entries.Count -gt 100) { throw 'Shunti archive exceeds bounds' }
            $path = Shunti-Child $Stage $entry.FullName
            if ($seen.ContainsKey($path)) { throw 'Duplicate shunti archive file' }
            if ((($entry.ExternalAttributes -shr 16) -band 0xF000) -notin @(0,0x8000)) { throw 'Special file in shunti archive' }
            $seen[$path] = $true
        }
        foreach ($entry in $zip.Entries) {
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry,(Shunti-Child $Stage $entry.FullName),$false)
        }
    } finally { $zip.Dispose() }
    $receipt = Get-Content -LiteralPath (Join-Path $Stage 'runtime-receipt.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($receipt.commit -cne '14801bffb90c9ea5f2dd0f594943a97ea35c8a1a') { throw 'Unexpected shunti source revision' }
    foreach ($name in @('shunti-helper.exe','LICENSE','MELTYPE-LICENSE.txt','THIRD_PARTY_NOTICES.md','DATA_SOURCES.md','UPSTREAM.txt')) {
        if ($null -eq $receipt.files.PSObject.Properties[$name]) { throw 'Incomplete shunti runtime receipt' }
    }
    foreach ($file in $receipt.files.PSObject.Properties) {
        if ((Shunti-Hash (Shunti-Child $Stage $file.Name)) -ne $file.Value) { throw 'Shunti runtime verification failed' }
    }
}
function Shunti-Probe([string]$Directory) {
    $probe = New-Object Diagnostics.Process
    try {
        $probe.StartInfo.FileName = Shunti-Child $Directory 'shunti-helper.exe'
        $probe.StartInfo.UseShellExecute = $false
        $probe.StartInfo.CreateNoWindow = $true
        $probe.StartInfo.RedirectStandardInput = $true
        $probe.StartInfo.RedirectStandardOutput = $true
        $probe.StartInfo.RedirectStandardError = $true
        $probe.StartInfo.StandardOutputEncoding = New-Object Text.UTF8Encoding($false)
        $null = $probe.Start()
        $output = $probe.StandardOutput.ReadToEndAsync()
        $errors = $probe.StandardError.ReadToEndAsync()
        $request = [Text.Encoding]::UTF8.GetBytes("C`t`tきょうはいいてんきです`n")
        $probe.StandardInput.BaseStream.Write($request,0,$request.Length)
        $probe.StandardInput.BaseStream.Flush()
        $probe.StandardInput.Close()
        if (-not $probe.WaitForExit(30000)) { $probe.Kill(); throw 'Shunti conversion check timed out' }
        if ($probe.ExitCode -ne 0 -or $output.Result -notmatch '^READY\r?\n[^\r\n]+\x1f') {
            throw ('Shunti conversion check failed: '+$errors.Result)
        }
    } finally { $probe.Dispose() }
}
function Install-Shunti([string]$DataRoot) {
    if ([Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString() -ne 'X64') { throw 'Shunti currently supports Windows x64' }
    $parent = Join-Path ([IO.Path]::GetFullPath($DataRoot)) 'ai'
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    if ((Get-Item -LiteralPath $parent -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Shunti parent cannot be a link' }
    $target = Shunti-Child $parent 'shunti-14801bff-s8-windows-x64'
    if (Test-Path -LiteralPath $target) { return Shunti-Installed $target }
    $stage = Shunti-Child $parent ('setup-shunti-'+[Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $stage | Out-Null
    try {
        $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/appipinopi/Meltype-Flash/releases/latest' -Headers @{'User-Agent'='Meltype-Shunti-Setup'} -TimeoutSec 30
        if ($release.draft -or $release.prerelease -or $release.tag_name -notmatch '^flash-v[0-9]+\.[0-9]+\.[0-9]+$') { throw 'Invalid shunti release metadata' }
        $name = 'Meltype-flash-Shunti-Windows-x64.zip'
        $assets = @($release.assets | Where-Object name -CEQ $name)
        if ($assets.Count -ne 1) { throw 'Windows shunti package is not published in this release yet' }
        $asset = $assets[0]
        $url = 'https://github.com/appipinopi/Meltype-Flash/releases/download/'+$release.tag_name+'/'+$name
        if ($asset.browser_download_url -cne $url -or $asset.digest -notmatch '^sha256:([a-fA-F0-9]{64})$') { throw 'Invalid shunti runtime asset metadata' }
        $archive = Join-Path $stage 'runtime.zip'
        Shunti-Download $url $archive $asset.digest.Substring(7)
        Shunti-Extract $archive $stage
        Remove-Item -LiteralPath $archive
        $base = 'https://github.com/shuntilettuce/Japanese-IME-for-HarmonyOS-next/releases/download/shuntelligence-v7/'
        Shunti-Download ($base+'kkc_lex_v5.bin') (Join-Path $stage 'kkc_lex.bin') '209a151603339a533810a5692037a02dc89725cd4a461b7ecc730e7ade207c75'
        Shunti-Download ($base+'kkc_model_s8pv5.bin') (Join-Path $stage 'kkc_model.bin') '8f802b7c1659a56d00c5566dd03ea1c280fda58eb139cbf55393575e8cc84a41'
        Shunti-Probe $stage
        $files = @{}
        foreach ($file in Get-ChildItem -LiteralPath $stage -File) { $files[$file.Name] = Shunti-Hash $file.FullName }
        @{files=$files} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $stage 'receipt.json') -Encoding UTF8
        Move-Item -LiteralPath $stage -Destination $target
        return Shunti-Installed $target
    } finally {
        if (Test-Path -LiteralPath $stage) {
            # Only remove our checked, freshly created child directory.
            $checked = Shunti-Child $parent ([IO.Path]::GetFileName($stage))
            if ($checked -ne $stage -or [IO.Path]::GetFileName($stage) -notlike 'setup-shunti-*') { throw 'Invalid shunti temporary directory' }
            Remove-Item -LiteralPath $checked -Recurse -Force
        }
    }
}
if (-not $FunctionsOnly) {
    try {
        if (-not $Root) { throw 'Missing AI data directory' }
        Install-Shunti $Root | ConvertTo-Json -Compress
    } catch {
        [Console]::Error.WriteLine($_.Exception.Message)
        exit 1
    }
}
