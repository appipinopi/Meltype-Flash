# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
#
# Removes only the Rust build cache. Sources, settings and dist/ are kept.
# When CARGO_TARGET_DIR is set, that directory is the target.
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    if ($env:CARGO_TARGET_DIR) {
        Write-Host "CARGO_TARGET_DIR = $env:CARGO_TARGET_DIR"
    }
    cargo clean
    Write-Host "Removed the Rust build cache."
} finally {
    Pop-Location
}
