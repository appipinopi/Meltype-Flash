; SPDX-License-Identifier: GPL-3.0-or-later
; Copyright (C) 2026 appipinopi
#ifndef AppVersion
  #define AppVersion "0.1.0-pre.4"
#endif
#ifndef SourceDir
  #error SourceDir is required (audited Rust package)
#endif
#ifndef OutputDir
  #define OutputDir "..\dist"
#endif
[Setup]
AppId=Meltype-flash
AppName=Meltype-flash
AppVersion={#AppVersion}
AppPublisher=appipinopi
AppPublisherURL=https://github.com/appipinopi/Meltype
AppSupportURL=https://github.com/appipinopi/Meltype/issues
DefaultDirName={autopf}\Meltype-flash\installer
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=Meltype-flash-{#AppVersion}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile={#SourceDir}\LICENSE
UninstallDisplayIcon={app}\meltype-flash.exe
CloseApplications=no
RestartApplications=no
SetupLogging=yes
[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "Invoke-FlashSetup.ps1"; DestDir: "{app}"; Flags: ignoreversion
[Icons]
Name: "{commonprograms}\Meltype-flash 設定"; Filename: "{app}\meltype-flash.exe"
[Code]
var InstallFailed: Boolean;
function Manage(Action: String): Boolean;
var Code: Integer;
begin
  Result := ExecAndLogOutput(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    '-NoProfile -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\Invoke-FlashSetup.ps1') +
    '" -Action ' + Action,
    ExpandConstant('{app}'), SW_HIDE, ewWaitUntilTerminated, Code, nil);
  Log('Meltype-flash ' + Action + ' result: ' + IntToStr(Code));
  Result := Result and (Code = 0);
end;
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    if not Manage('Install') then begin
      InstallFailed := True;
      RaiseException('Meltype-flash の IME 登録に失敗しました。セットアップログを確認してください。');
    end;
end;
function GetCustomSetupExitCode(): Integer;
begin
  if InstallFailed then Result := 1 else Result := 0;
end;
function InitializeUninstall(): Boolean;
begin
  Result := Manage('Uninstall');
  if not Result then
    MsgBox('IME の解除に失敗しました。アプリを閉じて再実行してください。設定・学習データは保持します。', mbError, MB_OK);
end;
