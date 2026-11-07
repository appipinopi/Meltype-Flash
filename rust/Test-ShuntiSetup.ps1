# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
# ダウンロードは固定のテストデータへ差し替え、実際の展開・検証・後始末を確かめる。
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Setup-Shunti.ps1') -FunctionsOnly
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -AssemblyName System.IO.Compression
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('Meltype-shunti-test-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot | Out-Null
function Assert([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Reject([scriptblock]$Action, [string]$Message) {
    $failed = $false
    try { & $Action | Out-Null } catch { $failed = $true }
    Assert $failed $Message
}
function Fixture([string]$Path, [string]$Extra = '', [bool]$BadHash = $false) {
    $payload = @{}
    foreach ($name in @('shunti-helper.exe','LICENSE','MELTYPE-LICENSE.txt','THIRD_PARTY_NOTICES.md','DATA_SOURCES.md','UPSTREAM.txt')) { $payload[$name]='fixture' }
    $files = @{}
    # Compute the fixture hash rather than relying on a handwritten digest.
    $hash = [Security.Cryptography.SHA256]::Create()
    try { $sha = ([BitConverter]::ToString($hash.ComputeHash([Text.Encoding]::UTF8.GetBytes('fixture')))).Replace('-','').ToLowerInvariant() } finally { $hash.Dispose() }
    foreach ($name in @($payload.Keys)) { $files[$name]=$sha }
    if ($BadHash) { $files['shunti-helper.exe']='0'*64 }
    $payload['runtime-receipt.json']=(@{commit='14801bffb90c9ea5f2dd0f594943a97ea35c8a1a';files=$files} | ConvertTo-Json -Depth 5 -Compress)
    $zip = [IO.Compression.ZipFile]::Open($Path,[IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in $payload.Keys) {
            $writer = New-Object IO.StreamWriter(($zip.CreateEntry($name)).Open(),(New-Object Text.UTF8Encoding($false)))
            try { $writer.Write($payload[$name]) } finally { $writer.Dispose() }
        }
        if ($Extra) {
            $writer = New-Object IO.StreamWriter(($zip.CreateEntry($Extra)).Open())
            try { $writer.Write('bad') } finally { $writer.Dispose() }
        }
    } finally { $zip.Dispose() }
}
try {
    $script:fixture = Join-Path $testRoot 'valid.zip'
    Fixture $script:fixture
    foreach ($name in @('../escape','SHUNTI-helper.exe','dir/file.exe','C:escape','file\escape')) {
        $archive = Join-Path $testRoot ([Guid]::NewGuid().ToString('N')+'.zip')
        Fixture $archive $name
        $stage = Join-Path $testRoot ([Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $stage | Out-Null
        Reject { Shunti-Extract $archive $stage } ('Unsafe archive accepted: '+$name)
    }
    $bad = Join-Path $testRoot 'bad.zip'
    Fixture $bad '' $true
    $stage = Join-Path $testRoot 'bad-stage'
    New-Item -ItemType Directory -Path $stage | Out-Null
    Reject { Shunti-Extract $bad $stage } 'Modified runtime accepted'

    # Exercise the real download hash check before replacing the transport.
    function Invoke-WebRequest { param($Uri,$OutFile,[switch]$UseBasicParsing,$TimeoutSec) [IO.File]::WriteAllText($OutFile,'modified') }
    Reject { Shunti-Download 'https://example.invalid/' (Join-Path $testRoot 'download') ('0'*64) } 'Wrong download SHA accepted'
    function Invoke-RestMethod {
        param($Uri,$Headers,$TimeoutSec)
        return @{tag_name='flash-v1.0.7';draft=$false;prerelease=$false;assets=@(@{name='Meltype-flash-Shunti-Windows-x64.zip';browser_download_url='https://github.com/appipinopi/Meltype-Flash/releases/download/flash-v1.0.7/Meltype-flash-Shunti-Windows-x64.zip';digest=('sha256:'+(Shunti-Hash $script:fixture))})}
    }
    $script:downloads = 0
    function Shunti-Download {
        param($Url,$Path,$Sha)
        $script:downloads++
        if ($Url.EndsWith('.zip')) {
            Assert ($Sha -eq (Shunti-Hash $script:fixture)) 'Runtime SHA not passed to transport'
            Copy-Item -LiteralPath $script:fixture -Destination $Path
        } else {
            Assert ($Sha -match '^[0-9a-f]{64}$') 'Model SHA missing'
            [IO.File]::WriteAllText($Path,'model fixture')
        }
    }
    $script:probes = 0
    function Shunti-Probe {
        param($Directory)
        $script:probes++
        Assert (Test-Path -LiteralPath (Join-Path $Directory 'kkc_model.bin')) 'Probe ran before model download'
    }
    $data = Join-Path $testRoot '日本語 データ'
    $result = Install-Shunti $data
    Assert ($result.engine -eq 'shunti' -and (Test-Path -LiteralPath $result.helper)) 'Setup did not return the executable path'
    Assert ($script:downloads -eq 3 -and $script:probes -eq 1) 'Missing downloads or runtime probe'
    $again = Install-Shunti $data
    Assert ($again.helper -eq $result.helper -and $script:downloads -eq 3) 'Verified environment was downloaded again'
    [IO.File]::WriteAllText((Join-Path (Split-Path $result.helper) 'kkc_model.bin'),'tampered')
    Reject { Install-Shunti $data } 'Modified model was reused'

    function Shunti-Probe { param($Directory) throw 'simulated runtime failure' }
    $rollback = Join-Path $testRoot 'rollback'
    Reject { Install-Shunti $rollback } 'Failed runtime was published'
    Assert (@(Get-ChildItem -LiteralPath (Join-Path $rollback 'ai') -Force).Count -eq 0) 'Failed setup left a partial environment'
    'Shunti setup: verified reuse, tampering, unsafe archives, SHA checks and rollback passed'
} finally {
    $resolved = [IO.Path]::GetFullPath($testRoot)
    $prefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\Meltype-shunti-test-'
    if (-not $resolved.StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe test cleanup path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
