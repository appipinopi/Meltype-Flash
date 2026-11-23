# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
# AI用CPUランタイムとモデルのみをユーザー領域へ導入する。
param([Parameter(Mandatory=$true)][string]$Root, [string]$ModelSpec)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)
$tag = 'b12125'
$modelSha = '4de930c06bef8c263aa1aa40684af206db4ce1b96375b3b8ed0ea508e0b14f6c'
$modelUrl = 'https://huggingface.co/Miwa-Keita/zenz-v3.1-small-gguf/resolve/ddf6e44b2e05ab7ea9a3e31559c5e7948761365c/ggml-model-Q5_K_M.gguf'
function Child-Path([string]$Base,[string]$Relative) {
    $path = [IO.Path]::GetFullPath((Join-Path $Base $Relative))
    $prefix = [IO.Path]::GetFullPath($Base).TrimEnd('\')+'\'
    if (-not $path.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)) { throw 'AI path escapes its directory' }
    return $path
}
function Download([string]$Uri,[string]$Path,[string]$Sha) {
    if ($Sha -notmatch '^[0-9a-fA-F]{64}$') { throw 'Missing download SHA256' }
    Invoke-WebRequest -Uri $Uri -OutFile $Path -UseBasicParsing -TimeoutSec 180
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ne $Sha) { throw 'AI download SHA256 mismatch' }
}
function Installed([string]$Directory) {
    $receipt = Get-Content -LiteralPath (Join-Path $Directory 'receipt.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($file in $receipt.files.PSObject.Properties) {
        $path = Child-Path $Directory $file.Name
        if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $file.Value) { throw 'Existing AI environment failed verification' }
    }
    foreach ($key in @('server','model')) {
        if ($null -eq $receipt.files.PSObject.Properties[$receipt.$key]) { throw 'AI receipt is incomplete' }
    }
    return @{server=(Child-Path $Directory $receipt.server);model=(Child-Path $Directory $receipt.model)}
}
$spec = if ($ModelSpec) { Get-Content -LiteralPath $ModelSpec -Raw -Encoding UTF8 | ConvertFrom-Json } else { $null }
$directory = 'zenz-v3.1'
if ($spec) {
    if (-not $spec.available -or $spec.directory -notmatch '^[a-zA-Z0-9_.-]+$' -or $spec.files.Count -ne 1 -or $spec.files[0].name -cne 'model.gguf') { throw 'Invalid model specification' }
    $directory = $spec.directory; $modelSha = $spec.files[0].sha256; $modelUrl = $spec.files[0].url
}
$stage = $null
try {
    $architecture = [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString().ToLowerInvariant()
    if ($architecture -notin @('x64','arm64')) { throw 'Unsupported CPU architecture' }
    $parent = Join-Path ([IO.Path]::GetFullPath($Root)) 'ai'
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $target = Child-Path $parent "$directory-$tag-$architecture"
    if (Test-Path -LiteralPath $target) {
        $result = Installed $target
        if ($spec -and (Get-FileHash -LiteralPath $result.model -Algorithm SHA256).Hash -ne $modelSha) { throw 'Selected model mismatch' }
        $result | ConvertTo-Json -Compress; return
    }
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/ggml-org/llama.cpp/releases/tags/$tag" -Headers @{'User-Agent'='Meltype-AI-Setup'} -TimeoutSec 30
    $name = "llama-$tag-bin-win-cpu-$architecture.zip"
    $asset = @($release.assets | Where-Object name -EQ $name)[0]
    $url = "https://github.com/ggml-org/llama.cpp/releases/download/$tag/$name"
    if ($null -eq $asset -or $asset.browser_download_url -cne $url -or $asset.digest -notmatch '^sha256:([a-fA-F0-9]{64})$') { throw 'Invalid AI runtime release metadata' }
    $runtimeSha = $asset.digest.Substring(7)
    $stage = Child-Path $parent ('.setup-'+[Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $stage | Out-Null
    $archive = Join-Path $stage 'runtime.zip'
    Download $url $archive $runtimeSha
    $runtime = Join-Path $stage 'runtime'
    New-Item -ItemType Directory -Path $runtime | Out-Null
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($archive)
    try {
        $seen = @{}; $expandedBytes = 0L
        foreach ($entry in $zip.Entries) {
            $expandedBytes += $entry.Length
            if ($expandedBytes -gt 1073741824L -or $zip.Entries.Count -gt 10000) { throw 'AI archive exceeds bounds' }
            if ($entry.FullName -match '[:\\]' -or [IO.Path]::IsPathRooted($entry.FullName) -or $entry.FullName -match '(^|/)\.\.(/|$)') { throw 'Unsafe AI archive path' }
            $path = Child-Path $runtime $entry.FullName
            if ($seen.ContainsKey($path)) { throw 'Duplicate AI archive path' }
            $seen[$path] = $true
            if ((($entry.ExternalAttributes -shr 16) -band 0xF000) -notin @(0,0x8000,0x4000)) { throw 'Special file in AI archive' }
        }
    } finally { $zip.Dispose() }
    [IO.Compression.ZipFile]::ExtractToDirectory($archive,$runtime)
    Remove-Item -LiteralPath $archive
    $server = @(Get-ChildItem -LiteralPath $runtime -Recurse -File -Filter llama-server.exe)[0]
    if ($null -eq $server) { throw 'llama-server.exe is missing' }
    $probe = New-Object Diagnostics.Process
    try {
        $probe.StartInfo.FileName = $server.FullName
        $probe.StartInfo.Arguments = '--version'
        $probe.StartInfo.UseShellExecute = $false
        $probe.StartInfo.CreateNoWindow = $true
        $probe.StartInfo.RedirectStandardOutput = $true
        $probe.StartInfo.RedirectStandardError = $true
        $null = $probe.Start()
        if (-not $probe.WaitForExit(15000)) { $probe.Kill(); throw 'AI runtime check timed out' }
        if ($probe.ExitCode -ne 0) { throw ('AI runtime cannot start: '+$probe.StandardError.ReadToEnd()) }
    } finally { $probe.Dispose() }
    Download $modelUrl (Join-Path $stage 'model.gguf') $modelSha
    $files = @{}
    foreach ($file in Get-ChildItem -LiteralPath $stage -Recurse -File) {
        $relative = $file.FullName.Substring($stage.Length+1)
        $files[$relative] = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
    }
    @{server=$server.FullName.Substring($stage.Length+1);model='model.gguf';files=$files} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $stage 'receipt.json') -Encoding UTF8
    $null = Child-Path $parent ([IO.Path]::GetFileName($target))
    Move-Item -LiteralPath $stage -Destination $target
    Installed $target | ConvertTo-Json -Compress
} catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
} finally {
    if ($null -ne $stage -and (Test-Path -LiteralPath $stage)) {
        $checked = Child-Path $parent ([IO.Path]::GetFileName($stage))
        if ($checked -ne $stage -or [IO.Path]::GetFileName($stage) -notlike '.setup-*') { throw 'Invalid temporary AI directory' }
        Remove-Item -LiteralPath $checked -Recurse -Force
    }
}
