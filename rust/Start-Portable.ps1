# SPDX-License-Identifier: GPL-3.0-or-later
# Registration-free buffered input. Ctrl+Alt+J toggles input; settings-only remains available.
[CmdletBinding()]
param([switch]$Tray, [switch]$SettingsOnly, [switch]$InstallShortcut)
$ErrorActionPreference = 'Stop'
$executable = Join-Path $PSScriptRoot 'meltype-flash.exe'
if (-not (Test-Path -LiteralPath $executable)) { throw 'Run from an extracted package.' }
if ($InstallShortcut) {
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Programs')) 'Meltype-FLASH.lnk'))
    $link.TargetPath = $executable
    $link.WorkingDirectory = $PSScriptRoot
    $link.IconLocation = "$executable,0"
    $link.Description = 'Meltype-FLASH'
    $link.Save()
    Write-Host 'Registered Meltype-FLASH in the Start menu.'
}
$previous = $env:MELTYPE_FLASH_DATA_DIR
try {
    $env:MELTYPE_FLASH_DATA_DIR = Join-Path $PSScriptRoot 'data'
    New-Item -ItemType Directory -Path $env:MELTYPE_FLASH_DATA_DIR -Force | Out-Null
    $arguments = @()
    if (-not $SettingsOnly) { $arguments += '--portable-keyboard' }
    if ($Tray) { $arguments += '--tray' }
    & $executable @arguments
} finally {
    $env:MELTYPE_FLASH_DATA_DIR = $previous
}
