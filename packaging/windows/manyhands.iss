#ifndef AppVersion
  #error AppVersion is required
#endif
#ifndef NumericVersion
  #error NumericVersion is required
#endif
#ifndef PayloadDir
  #error PayloadDir is required
#endif

[Setup]
AppId=io.github.vhodges.manyhands
AppName=Manyhands
AppVersion={#AppVersion}
AppVerName=Manyhands {#AppVersion}
VersionInfoVersion={#NumericVersion}
VersionInfoProductVersion={#NumericVersion}
VersionInfoTextVersion={#AppVersion}
AppPublisher=Vince Hodges
AppPublisherURL=https://github.com/vhodges/manyhands
DefaultDirName={localappdata}\Programs\Manyhands
DisableDirPage=yes
DefaultGroupName=Manyhands
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
SetupArchitecture=x64
MinVersion=10.0.15063
SetupMutex=Local\io.github.vhodges.manyhands.installer
SetupIconFile=manyhands.ico
UninstallDisplayIcon={app}\manyhands.ico
OutputBaseFilename=manyhands-v{#AppVersion}-x86_64-pc-windows-msvc-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes
SignTool=manyhands
SignedUninstaller=yes
SignToolRetryCount=0
CloseApplications=yes
RestartApplications=no
LicenseFile={#PayloadDir}\LICENSE

[Tasks]
Name: "clipath"; Description: "Add Manyhands CLI to my user PATH (new terminals only)"; Flags: unchecked

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "INNO-LICENSE.txt"; DestDir: "{app}\licenses\installer"; Flags: ignoreversion
Source: "manyhands.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\Manyhands"; Filename: "{app}\bin\manyhands.exe"; WorkingDir: "{app}"; IconFilename: "{app}\manyhands.ico"

[Code]
#include "installer-policy.pas"
#include "installer-state.iss"

function InitializeSetup: Boolean;
var
  Failure: String;
begin
  Failure := CheckInstalledVersion('{#AppVersion}');
  Result := Failure = '';
  if not Result then SuppressibleMsgBox(Failure, mbError, MB_OK, IDOK);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  PreviousDir, PreviousOwned, Current, Updated, NextOwned, Entry: String;
  PathType: Cardinal;
begin
  Result := CheckInstalledVersion('{#AppVersion}');
  if Result <> '' then Exit;
  try
    if RegValueExists(HKCU64, InstallerKey, 'InstallDir') then begin
      if not RegQueryStringValue(HKCU64, InstallerKey, 'InstallDir', PreviousDir) then
        RaiseException('Cannot read the previous installer directory.');
      if NormalizePolicyPath(PreviousDir) <> NormalizePolicyPath(ExpandConstant('{app}')) then
        RaiseException('Uninstall Manyhands before changing its install directory.');
    end;
    PreviousOwned := '';
    if RegValueExists(HKCU64, InstallerKey, 'OwnedPath') and
       not RegQueryStringValue(HKCU64, InstallerKey, 'OwnedPath', PreviousOwned) then
      RaiseException('Cannot read installer PATH ownership.');
    if WizardIsTaskSelected('clipath') or (PreviousOwned <> '') then begin
      PathType := ReadUserPath(Current);
      Entry := ExpandConstant('{app}\bin');
      if not PlanPathUpdate(Current, ExpandPathEnvironment(Current), Entry, PreviousOwned,
        WizardIsTaskSelected('clipath'), Updated, NextOwned) then
        RaiseException('Invalid installer PATH ownership or a directory unsuitable for PATH.');
    end;
    { Establish a fail-closed version floor before changing installed files.
      Interrupted installs/upgrades can be repaired by the same or newer version. }
    if not RegWriteStringValue(HKCU64, InstallerKey, 'PendingVersion', '{#AppVersion}') or
       not RegWriteStringValue(HKCU64, InstallerKey, 'InstallDir', ExpandConstant('{app}')) then
      RaiseException('Cannot checkpoint installer recovery metadata.');
  except
    Result := GetExceptionMessage;
  end;
end;

procedure CompleteInstaller;
begin
  if not RegWriteStringValue(HKCU64, InstallerKey, 'InstallDir', ExpandConstant('{app}')) or
     not RegWriteStringValue(HKCU64, InstallerKey, 'Version', '{#AppVersion}') then
    RaiseException('Cannot record the installed Manyhands version.');
  UpdateUserPath(WizardIsTaskSelected('clipath'), ExpandConstant('{app}\bin'));
  if not RegDeleteValue(HKCU64, InstallerKey, 'PendingVersion') then
    RaiseException('Cannot complete installer recovery metadata.');
end;

#include "installer-completion.iss"

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then UpdateUserPath(False, ExpandConstant('{app}\bin'));
  if CurUninstallStep = usPostUninstall then
    if not RegDeleteKeyIncludingSubkeys(HKCU64, InstallerKey) then
      Log('Installer metadata could not be fully removed. No application data was touched.');
end;
