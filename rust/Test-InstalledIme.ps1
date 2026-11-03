# SPDX-License-Identifier: GPL-3.0-or-later
# Exercise a real machine registration, then restore the original IME.
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$Package,
    [Parameter(Mandatory=$true)][string]$ReportDirectory,
    [switch]$ExerciseRegistration,
    [switch]$ProbeOnly
)
$ErrorActionPreference = 'Stop'
$clsid = '{C540BAF6-88B1-5B30-B524-2C8702B034FC}'
$profile = '{6D01DE9C-8AB3-5CFC-A5C6-10893CDF8762}'
$windows = [Environment]::GetFolderPath('Windows')
if ($ProbeOnly) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class InstalledImeProbe {
    [DllImport("ole32.dll")] static extern int CoInitializeEx(IntPtr p, uint flags);
    [DllImport("ole32.dll")] static extern void CoUninitialize();
    [DllImport("ole32.dll")] static extern int CoCreateInstance(ref Guid clsid, IntPtr outer, uint context, ref Guid iid, out IntPtr value);
    public static void Run() {
        int init = CoInitializeEx(IntPtr.Zero, 2);
        Marshal.ThrowExceptionForHR(init);
        try {
            Guid clsid = new Guid("C540BAF6-88B1-5B30-B524-2C8702B034FC");
            Guid iid = new Guid("aa80e7f7-2021-11d2-93e0-0060b067b86e");
            IntPtr value;
            Marshal.ThrowExceptionForHR(CoCreateInstance(ref clsid, IntPtr.Zero, 1, ref iid, out value));
            if (value == IntPtr.Zero) throw new Exception("Null ITfTextInputProcessor");
            Marshal.Release(value);
        } finally { CoUninitialize(); }
    }
}
'@
    [InstalledImeProbe]::Run()
    Write-Host "PASS: registered ITfTextInputProcessor COM activation ($([IntPtr]::Size*8)-bit)"
    return
}
# The flash registration is independent. This exercise never replaces upstream keys.
$packageRoot = (Resolve-Path -LiteralPath $Package).Path
& (Join-Path $packageRoot 'Manage-Windows.ps1') -Action Inspect -Package $packageRoot
if (-not $?) { throw 'Invalid package' }
function Server([string]$Id, [Microsoft.Win32.RegistryView]$View) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine,$View)
    try {
        $key = $base.OpenSubKey("SOFTWARE\Classes\CLSID\$Id\InprocServer32")
        if ($null -eq $key) { return $null }
        try { return [string]$key.GetValue('') } finally { $key.Dispose() }
    } finally { $base.Dispose() }
}
$views = @([Microsoft.Win32.RegistryView]::Registry64,[Microsoft.Win32.RegistryView]::Registry32)
$flash = @($views | ForEach-Object { Server $clsid $_ })
if (-not $ExerciseRegistration) {
    @{ product='Meltype-FLASH'; servers=$flash; readOnly=$true } | ConvertTo-Json
    return
}
if (@($flash | Where-Object { $_ }).Count) { throw 'Existing Meltype-FLASH registration: exercise refused to preserve it.' }
$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not [Environment]::Is64BitProcess -or -not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'The explicit registration exercise requires an administrator 64-bit PowerShell.'
}
$upstreamClsid = '{417D801B-A9BD-4C26-BD16-356A825A6998}'
$before = @($views | ForEach-Object { Server $upstreamClsid $_ })
New-Item -ItemType Directory -Path $ReportDirectory -ErrorAction Stop | Out-Null
$report = (Resolve-Path -LiteralPath $ReportDirectory).Path
$manager = Join-Path $packageRoot 'Manage-Windows.ps1'
$attempted=$false; $passed=$false; $unregistered=$false; $upstreamPreserved=$false
try {
    $attempted=$true
    & $manager -Action Install -Package $packageRoot -SkipCurrentUserStartup
    if (-not $?) { throw 'Flash installation failed' }
    foreach ($shell in @("$windows/System32/WindowsPowerShell/v1.0/powershell.exe","$windows/SysWOW64/WindowsPowerShell/v1.0/powershell.exe")) {
        & $shell -NoProfile -ExecutionPolicy Bypass -File $PSCommandPath -Package $packageRoot -ReportDirectory $report -ProbeOnly
        if ($LASTEXITCODE -ne 0) { throw 'Flash COM activation failed' }
    }
    $passed=$true
} finally {
    try {
        if ($attempted) {
            & $manager -Action Uninstall -SkipCurrentUserStartup
            if (-not $?) { throw 'Flash test uninstallation failed' }
        }
        $remaining=@($views | ForEach-Object { Server $clsid $_ } | Where-Object { $_ })
        $unregistered=$remaining.Count -eq 0
        $after=@($views | ForEach-Object { Server $upstreamClsid $_ })
        $upstreamPreserved=($before | ConvertTo-Json -Compress) -ceq ($after | ConvertTo-Json -Compress)
        if (-not $upstreamPreserved -or -not $unregistered) { throw 'Registration cleanup or upstream preservation check failed' }
    } finally {
        @{product='Meltype-FLASH';registeredComPassed=$passed;flashUnregistered=$unregistered;upstreamPreserved=$upstreamPreserved;externalApplicationInputTested=$false} |
            ConvertTo-Json | Set-Content -LiteralPath (Join-Path $report 'result.json') -Encoding UTF8
    }
}
Write-Host "PASS: independent flash x64/x86 registration and activation; upstream preserved. Report: $report"
