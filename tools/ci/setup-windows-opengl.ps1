# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
# Software OpenGL for actual GUI tests on GPU-less GitHub Windows runners.
# These DLLs stay beside test executables and are never package inputs.
param([Parameter(Mandatory=$true)][string[]]$Directories)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows') {
    throw 'This helper is only for GitHub Windows CI.'
}
$stage = Join-Path $env:RUNNER_TEMP 'meltype-mesa-26.2.4'
New-Item -ItemType Directory -Path $stage -Force | Out-Null
$archive = Join-Path $stage 'mesa.7z'
if (-not (Test-Path -LiteralPath $archive)) {
    Invoke-WebRequest 'https://github.com/pal1000/mesa-dist-win/releases/download/26.2.4/mesa3d-26.2.4-release-msvc.7z' -OutFile $archive
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne '351fc8c8b695878ffb3eaa044b3ead08672a48b1a045e3c3e3975811df0f6695') {
    throw 'Mesa download checksum mismatch'
}
& 7z x $archive "-o$stage" 'x64/opengl32.dll' 'x64/libgallium_wgl.dll' -y | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Mesa extraction failed' }
foreach ($directory in $Directories) {
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
    foreach ($name in @('opengl32.dll','libgallium_wgl.dll')) {
        Copy-Item -LiteralPath (Join-Path $stage "x64/$name") -Destination (Join-Path $directory $name) -Force
    }
}
'GALLIUM_DRIVER=llvmpipe' | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
