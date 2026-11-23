# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
[CmdletBinding()]
param([Parameter(Mandatory=$true)][ValidateSet('Install','Uninstall')][string]$Action)
$ErrorActionPreference = 'Stop'
try {
    # A launcher running under PowerShell 7 can pass its module path to 5.1.
    # Load the Windows PowerShell utility module from that host's own directory.
    Import-Module (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Utility/Microsoft.PowerShell.Utility.psd1') -Force -ErrorAction Stop
    & (Join-Path $PSScriptRoot 'Manage-Windows.ps1') -Action $Action -Package $PSScriptRoot
    exit 0
} catch {
    Write-Error $_ -ErrorAction Continue
    exit 1
}
