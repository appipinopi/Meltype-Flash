# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 lnkiai
#
# Meltype IME (TSF の DLL) をビルドする。Visual Studio Build Tools (C++) が要る。
#   powershell -ExecutionPolicy Bypass -File native\tip\build.ps1 [-Configuration Release|Debug]
# 出力: native\tip\bin\x64\MeltypeTip.dll と native\tip\bin\x86\MeltypeTip.dll (32bit のアプリ用)
# -RustCore: Rust単体DLLを生成。MSVCリンカー／Windows SDKでリソースを結合する。
# -CompatibilityObjects: 比較試験用C++を追加コンパイルする（配布DLLにはリンクしない）。

param([ValidateSet('Release', 'Debug')][string]$Configuration = 'Release', [switch]$RustCore, [switch]$CompatibilityObjects)

$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) { throw 'Visual Studio (Build Tools) の C++ ビルドツールが見つかりません。Meltype IME をビルドするには、Visual Studio Build Tools の「C++ によるデスクトップ開発」を入れてください。' }
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw 'Visual Studio の C++ ビルドツールが見つかりません。' }
$vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvarsall.bat'
# vcvarsall.bat は vswhere.exe を PATH から探すので、見つからないという表示が出ないように足しておく
$env:PATH = (Split-Path $vswhere) + ';' + $env:PATH

# ファイルの SHA-256 (Get-FileHash は、ほかのシェルから起動した PowerShell では読み込めないことがあるので .NET で計算する)
function Get-Sha256([string]$path) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($sha.ComputeHash([IO.File]::ReadAllBytes($path))).Replace('-', '') }
    finally { $sha.Dispose() }
}

$sources = 'Dll.cpp', 'TextService.cpp', 'CandidateWindow.cpp', 'DisplayAttributes.cpp', 'LangBar.cpp', 'Pipe.cpp', 'Json.cpp', 'ModeIndicator.cpp'
if ($CompatibilityObjects -and -not $RustCore) { throw '-CompatibilityObjects requires -RustCore' }
$coreDefine = if ($RustCore) { '/DMELTYPE_RUST_CORE' } else { '' }
# /Brepro: 同じソースからは同じ DLL にする (日時を埋め込まない)。入れ直したときに、変わっていない DLL を登録し直さない (UAC を出さない) ため
$optimize = if ($Configuration -eq 'Release') { '/O2 /MT /DNDEBUG' } else { '/Od /MTd /Zi' }
$libs = 'ole32.lib oleaut32.lib uuid.lib user32.lib gdi32.lib advapi32.lib d2d1.lib dwrite.lib dwmapi.lib'

if ($RustCore) {
    $root = [IO.Path]::GetFullPath((Join-Path $here '../..'))
    $serverTarget = if ($env:MELTYPE_TSF_TARGET_DIR) { $env:MELTYPE_TSF_TARGET_DIR } else { Join-Path $root 'target/tsf-server' }
    $profile = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }
    $release = if ($Configuration -eq 'Release') { '--release' } else { '' }
    foreach ($arch in 'x64','x86') {
        $target = if ($arch -eq 'x64') { 'x86_64-pc-windows-msvc' } else { 'i686-pc-windows-msvc' }
        $vcarch = if ($arch -eq 'x64') { 'x64' } else { 'x64_x86' }
        $out = Join-Path $here "bin/rust-$arch"; $obj = Join-Path $here "obj/rust-$arch"
        New-Item -ItemType Directory -Force -Path $out,$obj | Out-Null
        $compat = if ($CompatibilityObjects) {
            "cl /nologo /Brepro /std:c++17 /EHsc /W4 /wd4100 /utf-8 /DUNICODE /D_UNICODE /DMELTYPE_RUST_CORE $optimize /Fo`"$obj\\`" /Fd`"$obj\\`" /c Dll.cpp TextService.cpp RustPipe.cpp Json.cpp ModeIndicator.cpp"
        } else { 'ver >nul' }
        $cmd = @(
            "`"$vcvars`" $vcarch >nul",
            "cd /d `"$here`"", $compat,
            "rc /nologo /I `"$here`" /I `"$root`" /fo `"$obj/MeltypeTip.res`" `"$root/rust/FlashTip.rc`"",
            "cargo rustc --manifest-path `"$root/Cargo.toml`" -p meltype-native --lib --locked --features tsf-server --target $target --target-dir `"$serverTarget`" $release -- -C link-arg=`"$obj/MeltypeTip.res`" -C link-arg=/Brepro -C link-arg=/MAP:`"$obj/MeltypeTip.map`""
        ) -join ' && '
        Write-Host "== Rust TSF $arch =="
        cmd /c $cmd
        if ($LASTEXITCODE -ne 0) { throw "Rust TSF build failed: $arch" }
        $dll = Join-Path $out 'MeltypeTip.dll'
        Copy-Item -LiteralPath (Join-Path $serverTarget "$target/$profile/meltype_native.dll") -Destination $dll
        [IO.File]::WriteAllText("$dll.sha256", (Get-Sha256 $dll))
    }
    Write-Host 'できました:' (Join-Path $here 'bin')
    return
}

foreach ($arch in 'x64', 'x86') {
    $out = Join-Path $here "bin\$arch"
    $obj = Join-Path $here "obj\$arch"
    New-Item -ItemType Directory -Force -Path $out, $obj | Out-Null
    $vcarch = if ($arch -eq 'x64') { 'x64' } else { 'x64_x86' }
    $cmd = @(
        "`"$vcvars`" $vcarch >nul",
        "cd /d `"$here`"",
        "rc /nologo /fo `"$obj\MeltypeTip.res`" MeltypeTip.rc",
        "cl /nologo /Brepro /std:c++17 /EHsc /W4 /wd4100 /utf-8 /DUNICODE /D_UNICODE $coreDefine $optimize /Fo`"$obj\\`" /Fd`"$obj\\`" /LD $($sources -join ' ') `"$obj\MeltypeTip.res`" /link /Brepro /DEF:MeltypeTip.def /MAP:`"$obj\MeltypeTip.map`" /IMPLIB:`"$obj\MeltypeTip.lib`" /OUT:`"$out\MeltypeTip.dll`" $libs"
    ) -join ' && '
    Write-Host "== $arch =="
    cmd /c $cmd
    if ($LASTEXITCODE -ne 0) { throw "$arch のビルドに失敗しました。" }
    # 署名する前の中身のハッシュ。インストールのとき、登録済みの DLL と同じかを比べるのに使う (署名には毎回違う時刻が入るため)
    $dll = Join-Path $out 'MeltypeTip.dll'
    [IO.File]::WriteAllText("$dll.sha256", (Get-Sha256 $dll))
}
Write-Host 'できました:' (Join-Path $here 'bin')
