#ifndef AppVersion
  #error AppVersion is required
#endif
#ifndef NumericVersion
  #error NumericVersion is required
#endif
#ifndef PayloadDir
  #error PayloadDir is required
#endif
#ifndef ManagedFiles
  #error ManagedFiles is required (generated from the verified portable ZIP)
#endif
#ifndef ManagedDigests
  #error ManagedDigests is required (generated from verified payload and installer additions)
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
SignToolRetryCount=2
SignToolRetryDelay=2000
CloseApplications=yes
RestartApplications=no
LicenseFile={#PayloadDir}\LICENSE

[Tasks]
Name: "clipath"; Description: "Add Manyhands CLI to my user PATH (new terminals only)"; Flags: unchecked

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "INNO-LICENSE.txt"; DestDir: "{app}\licenses\installer"; Flags: ignoreversion
Source: "{#ManagedFiles}"; DestDir: "{app}\licenses\installer"; DestName: "managed-files.txt"; Flags: ignoreversion
Source: "{#ManagedDigests}"; DestName: "managed-digests.txt"; Flags: dontcopy
Source: "manyhands.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\Manyhands"; Filename: "{app}\bin\manyhands.exe"; WorkingDir: "{app}"; IconFilename: "{app}\manyhands.ico"

[Code]
#include "installer-policy.pas"
#include "installer-managed-policy.pas"
#include "installer-state.iss"
#include "installer-managed.iss"
#include "installer-lifecycle.iss"

function InitializeSetup: Boolean;
var
  Failure: String;
begin
  Failure := CheckInstalledVersion('{#AppVersion}');
  Result := Failure = '';
  if not Result then SuppressibleMsgBox(Failure, mbError, MB_OK, IDOK);
  if Result then begin
    try
      LoadIncomingManagedFiles;
    except
      SuppressibleMsgBox(GetExceptionMessage, mbError, MB_OK, IDOK);
      Result := False;
    end;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := CheckInstallPreparation('{#AppVersion}');
end;

procedure BeginInstaller;
begin
  StartInstaller('{#AppVersion}');
end;

procedure CompleteInstaller;
begin
  PruneInstallerManagedFiles;
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
