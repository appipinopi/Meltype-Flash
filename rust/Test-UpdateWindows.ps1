# SPDX-License-Identifier: GPL-3.0-or-later
# Real selection, digest and ZIP validation over a controlled download boundary.
$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ('meltype-update-test-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$source = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'Update-Windows.ps1') -Raw
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseInput($source,[ref]$tokens,[ref]$errors)
if ($errors.Count) { throw $errors[0] }
$global:MeltypeUpdateTest = @{ releases = @(); archive = ''; downloads = 0 }
$replacements = @{
    'Get-Releases' = 'function Get-Releases { return $global:MeltypeUpdateTest.releases }'
    'Download-Asset' = 'function Download-Asset($Uri,$Destination,$ExpectedSize) { $global:MeltypeUpdateTest.downloads++; Copy-Item -LiteralPath $global:MeltypeUpdateTest.archive -Destination $Destination }'
}
$functions = @($ast.FindAll({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $replacements.ContainsKey($node.Name)},$true))
if ($functions.Count -ne 2) { throw 'Updater transport boundary changed' }
foreach ($function in $functions | Sort-Object {$_.Extent.StartOffset} -Descending) {
    $source = $source.Substring(0,$function.Extent.StartOffset)+$replacements[$function.Name]+$source.Substring($function.Extent.EndOffset)
}
$script = Join-Path $root 'Update-Windows.ps1'
[IO.File]::WriteAllText($script,$source)
$manager = Join-Path $root 'Manage-Windows.ps1'
[IO.File]::WriteAllText($manager, @'
param($Action,$Package)
if ($Action -ne 'Inspect') { throw 'Stage test tried to install an IME' }
if (-not (Test-Path -LiteralPath (Join-Path $Package 'payload.txt'))) { throw 'Missing extracted payload' }
Set-Content -LiteralPath (Join-Path $PSScriptRoot 'inspected') -Value $Package
'@)
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -AssemblyName System.IO.Compression
function Archive([string]$Name, [string[]]$Entries) {
    $path = Join-Path $root $Name
    $zip = [IO.Compression.ZipFile]::Open($path,[IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($entry in $Entries) {
            $stream = New-Object IO.StreamWriter(($zip.CreateEntry($entry)).Open())
            try { $stream.Write('controlled payload') } finally { $stream.Dispose() }
        }
    } finally { $zip.Dispose() }
    return $path
}
function Release([string]$Archive) {
    $asset = @{name='Meltype-flash-Windows-x64.zip';size=(Get-Item -LiteralPath $Archive).Length;digest=('sha256:'+(Get-FileHash -LiteralPath $Archive).Hash);browser_download_url='https://github.com/appipinopi/Meltype-Flash/releases/download/flash-v1.0.0/Meltype-flash-Windows-x64.zip'}
    return @{id=123;tag_name='flash-v1.0.0';draft=$false;prerelease=$false;assets=@($asset)}
}
function Reject([scriptblock]$Run,[string]$Pattern) {
    $failed = $false
    try { & $Run } catch { if ($_.Exception.Message -notlike $Pattern) { throw }; $failed=$true }
    if (-not $failed) { throw "Expected rejection: $Pattern" }
}
$good = Archive 'good.zip' @('payload.txt')
$state = $global:MeltypeUpdateTest
$state.archive = $good; $release = Release $good; $state.releases = @($release)
& $script -Action Check
if ($state.downloads -ne 0) { throw 'Check downloaded a ZIP' }
& $script -Action Stage -WhatIf
if ($state.downloads -ne 0) { throw 'WhatIf downloaded a ZIP' }
& $script -Action Stage
if (-not (Test-Path -LiteralPath (Join-Path $root 'inspected'))) { throw 'Downloaded package was not inspected' }
$release.assets[0].digest = 'sha256:'+('0'*64)
Reject { & $script -Action Stage } '*digest mismatch*'
foreach ($entry in @('../escape.txt','/absolute.txt','C:/drive.txt','folder\bad.txt')) {
    $archive = Archive ([Guid]::NewGuid().ToString('N')+'.zip') @($entry)
    $state.archive=$archive; $state.releases=@(Release $archive)
    Reject { & $script -Action Stage } '*Unsafe update archive path*'
}
$state.archive=Archive 'duplicates.zip' @('a.txt','A.txt'); $state.releases=@(Release $state.archive)
Reject { & $script -Action Stage } '*Duplicate or escaping*'
foreach ($invalid in @('missing-digest','foreign-host','old-fork','draft','pre-release','csharp')) {
    $release=Release $good
    switch ($invalid) {
        'missing-digest' { $release.assets[0].digest=$null }
        'foreign-host' { $release.assets[0].browser_download_url='https://example.org/update.zip' }
        'old-fork' { $release.assets[0].browser_download_url='https://github.com/appipinopi/Meltype/releases/download/flash-v1.0.0/Meltype-flash-Windows-x64.zip' }
        'draft' { $release.draft=$true }
        'pre-release' { $release.prerelease=$true }
        'csharp' { $release.assets[0].name='Meltype-Windows.zip' }
    }
    $state.releases=@($release)
    Reject { & $script -Action Stage } '*No stable Rust Windows ZIP*'
}
$release=Release $good; $state.releases=@($release)
@{releaseId=123;sha256=$release.assets[0].digest.Substring(7)} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'update-receipt.json') -Encoding UTF8
$before=$state.downloads
& $script -Action Stage
if ($state.downloads -ne $before) { throw 'Current release downloaded again' }
Write-Host "PASS: release selection, no-op/WhatIf, staging, SHA256, ZIP paths/duplicates, foreign/draft/C# filtering. Results: $root"
Remove-Variable -Name MeltypeUpdateTest -Scope Global
