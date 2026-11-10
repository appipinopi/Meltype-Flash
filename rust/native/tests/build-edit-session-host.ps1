# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
param([string]$NativeX64, [string]$NativeX86, [string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
# Compatibility-owner SDK hosts need these objects; the production DLL is Rust.
& (Join-Path $root 'native/tip/build.ps1') -RustCore -CompatibilityObjects
& (Join-Path $PSScriptRoot 'prepare-reference-json.ps1')
if (-not $NativeX64) { $NativeX64 = Join-Path $root 'target/x86_64-pc-windows-msvc/release/meltype_native.dll' }
if (-not $NativeX86) { $NativeX86 = Join-Path $root 'target/i686-pc-windows-msvc/release/meltype_native.dll' }
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw 'MSVC tools are required for the shell edit-session test.' }
$vcvars = Join-Path $vs 'VC/Auxiliary/Build/vcvarsall.bat'
foreach ($arch in 'x64','x86') {
    $out = if ($OutputDirectory) { Join-Path $OutputDirectory $arch } else { Join-Path $root "dist/edit-session-host/$arch" }
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    $native = if ($arch -eq 'x64') { $NativeX64 } else { $NativeX86 }
    $linkMap = Get-Content -LiteralPath (Join-Path $root "native/tip/obj/rust-$arch/MeltypeTip.map") -Raw
    if ($linkMap -notmatch 'meltype_native' -or $linkMap -match '(TextService|RustPipe|Json|RustDll|Dll)\.obj') {
        throw "Production DLL must not link C++ bridge objects: $arch"
    }
    Copy-Item -LiteralPath (Join-Path $root "native/tip/bin/rust-$arch/MeltypeTip.dll") -Destination $out
    Copy-Item -LiteralPath $native -Destination (Join-Path $out 'meltype_native.dll')
    Copy-Item -LiteralPath (Join-Path (Split-Path $native) 'meltype-tip-worker.exe') -Destination $out
    $objects = @('Dll','TextService','RustPipe','Json','ModeIndicator') | ForEach-Object { '"' + (Join-Path $root "native/tip/obj/rust-$arch/$_.obj") + '"' }
    $vcarch = if ($arch -eq 'x64') { 'x64' } else { 'x64_x86' }
    foreach ($test in 'service_state_host','json_host','edit_session_host','commit_batch_host','candidate_view_host','fields_host','edit_events_host','keys_host','mode_host','transport_host','key_edit_host','native_key_owner_host','selection_host','apply_reply_host') {
        $source = Join-Path $PSScriptRoot "$test.cpp"
        $cmd = "`"$vcvars`" $vcarch >nul && cl /nologo /utf-8 /std:c++17 /EHsc /DUNICODE /D_UNICODE /DMELTYPE_RUST_CORE `"$source`" /Fo`"$out/test.obj`" /Fe`"$out/test.exe`" $($objects -join ' ') /link ole32.lib oleaut32.lib uuid.lib user32.lib gdi32.lib advapi32.lib d2d1.lib dwrite.lib dwmapi.lib"
        cmd /c $cmd
        if ($LASTEXITCODE -ne 0) { throw "$test failed to build: $arch" }
        & (Join-Path $out 'test.exe')
        if ($LASTEXITCODE -ne 0) { throw "$test failed: $arch (exit $LASTEXITCODE)" }
        if ($test -eq 'native_key_owner_host') {
            Copy-Item -LiteralPath (Join-Path $out 'test.exe') -Destination (Join-Path $out 'native_key_owner_host.exe')
        }
    }
}
