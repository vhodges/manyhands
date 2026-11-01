#ifndef FixtureId
  #error A fresh fixture ID is required
#endif
#define FixtureRoot "Software\Manyhands\InstallerLifecycleFixture-" + FixtureId
#define InstallerStateKey FixtureRoot + "\State"
#define InstallerEnvironmentKey FixtureRoot + "\Environment"
#define InstallerCheckpointWrite "FixtureCheckpointWrite"

[Setup]
AppId=io.github.vhodges.manyhands.lifecycle-test
AppName=Manyhands isolated lifecycle test
AppVersion=2.0.0
DefaultDirName={param:FixtureDir}\app
PrivilegesRequired=lowest
SetupArchitecture=x64
ArchitecturesInstallIn64BitMode=x64os
Uninstallable=no
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableWelcomePage=yes
OutputBaseFilename=installer-lifecycle-test

[Tasks]
Name: "clipath"; Description: "Unused isolated PATH fixture"; Flags: unchecked

[Files]
Source: "installer_lifecycle.iss"; DestDir: "{app}\bin"; DestName: "manyhands.exe"; Flags: ignoreversion; AfterInstall: CancelAfterCopy
Source: "installer_lifecycle.iss"; DestDir: "{app}\bin"; DestName: "manyhands-cli.exe"; Flags: ignoreversion
Source: "installer_lifecycle.iss"; DestDir: "{app}\licenses"; DestName: "README.md"; Flags: ignoreversion
Source: "..\..\..\packaging\windows\INNO-LICENSE.txt"; DestDir: "{app}\licenses\installer"; Flags: ignoreversion
Source: "{#ManagedFiles}"; DestDir: "{app}\licenses\installer"; DestName: "managed-files.txt"; Flags: ignoreversion
Source: "{#ManagedDigests}"; DestName: "managed-digests.txt"; Flags: dontcopy

[Code]
var
  FixtureInitialized: Boolean;
  PreparationChecked: Boolean;
  FixtureCancelSeen: Boolean;

#include "..\..\..\packaging\windows\installer-policy.pas"
#include "..\..\..\packaging\windows\installer-managed-policy.pas"

function FixtureCheckpointWrite(RootKey: HKEY; SubKey, Name, Value: String): Boolean;
begin
  if (ExpandConstant('{param:Mode}') = 'startfail') or
     ((ExpandConstant('{param:Mode}') = 'startpartial') and (Name = 'InstallDir')) then Result := False
  else Result := RegWriteStringValue(RootKey, SubKey, Name, Value);
end;

#include "..\..\..\packaging\windows\installer-state.iss"
#include "..\..\..\packaging\windows\installer-managed.iss"
#include "..\..\..\packaging\windows\installer-lifecycle.iss"

function SetFixtureFileAttributes(Name: String; Attributes: Cardinal): Boolean;
  external 'SetFileAttributesW@kernel32.dll stdcall';
function CreateFixtureDirectoryLink(Name, Target: String; Flags: Cardinal): Byte;
  external 'CreateSymbolicLinkW@kernel32.dll stdcall';

procedure AddFixtureReparsePoint;
begin
  if CreateFixtureDirectoryLink(ExpandConstant('{param:FixtureDir}\app\licenses\junction'),
    ExpandConstant('{param:FixtureDir}\outside'), 1) = 0 then
    RaiseException('Could not create the isolated reparse-point fixture.');
end;

procedure CancelAfterCopy;
begin
  { Exercise the actual cancel path, not Abort in an entry callback (Inno
    handles callback exceptions and may continue copying). }
  if ExpandConstant('{param:Mode}') = 'midcancel' then WizardForm.Close;
end;

function InitializeSetup: Boolean;
var
  App: String;
begin
  App := ExpandConstant('{param:FixtureDir}\app');
  if (ExpandConstant('{param:FixtureDir}') = '') or DirExists(App) or
     RegKeyExists(HKCU64, '{#FixtureRoot}') then RaiseException('Refusing unowned fixture state.');
  if ExpandConstant('{param:Mode}') <> 'fresh' then begin
    ForceDirectories(App + '\bin');
    ForceDirectories(App + '\licenses\old-crate');
    SaveStringToFile(App + '\bin\manyhands.exe', 'old executable', False);
    SaveStringToFile(App + '\bin\stale.dll', 'old DLL', False);
    SaveStringToFile(App + '\licenses\old-crate\COPYING', 'old licence', False);
    SaveStringToFile(App + '\user-state.txt', 'preserve user state', False);
    RegWriteStringValue(HKCU64, InstallerKey, 'Version', '1.0.0');
    RegWriteStringValue(HKCU64, InstallerKey, 'InstallDir', App);
  end;
  ForceDirectories(ExpandConstant('{param:FixtureDir}\outside'));
  SaveStringToFile(ExpandConstant('{param:FixtureDir}\outside\keep.txt'), 'outside managed trees', False);
  FixtureInitialized := True;
  LoadIncomingManagedFiles;
  Result := True;
end;

procedure InitializeWizard;
begin
  if ExpandConstant('{param:Mode}') = 'pre-reparse' then AddFixtureReparsePoint;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := CheckInstallPreparation('2.0.0');
  if ExpandConstant('{param:Mode}') = 'pre-reparse' then begin
    if Pos('reparse point', Result) = 0 then RaiseException('Pre-copy reparse traversal was not rejected.');
    Exit;
  end;
  if Result <> '' then RaiseException(Result);
  if RegValueExists(HKCU64, InstallerKey, 'PendingVersion') then
    RaiseException('Preparation wrote a pending floor.');
  PreparationChecked := True;
  if ExpandConstant('{param:Mode}') = 'preparecancel' then begin
    Result := 'Controlled cancellation before install start.';
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  { Preparation temporarily disables Cancel. Cancel once the engine has
    returned to the preparing page with its normal enabled Cancel button. }
  if (CurPageID = wpPreparing) and PreparationChecked and
     (ExpandConstant('{param:Mode}') = 'preparecancel') then WizardForm.Close;
end;

procedure BeginInstaller;
begin
  StartInstaller('2.0.0');
end;

procedure WriteFixtureForgedDigests(Name: String);
var
  I: Integer;
  Records: String;
begin
  { A complete, path-bound forged table matching all current installed bytes
    must not replace the compiler-embedded expectations cached at startup. }
  Records := '';
  for I := 0 to Length(IncomingManagedFiles) - 1 do
    Records := Records + GetSHA256OfFile(ExpandConstant('{app}\') + IncomingManagedFiles[I]) +
      '  ' + IncomingManagedFiles[I] + #13 + #10;
  if not SaveStringToFile(Name, Records, False) then RaiseException('Cannot inject forged digest metadata.');
end;

procedure CompleteInstaller;
var
  Mode: String;
begin
  Mode := ExpandConstant('{param:Mode}');
  { Simulate Inno's successful Keep existing/Skip outcomes at the exact
    ssPostInstall boundary, before the shared production completion gate. }
  if (Mode = 'old-binary') or (Mode = 'old-readonly') or (Mode = 'forged-installed-digests') or
     (Mode = 'forged-temporary-digests') then
    if not SaveStringToFile(ExpandConstant('{app}\bin\manyhands.exe'), 'old executable', False) then
      RaiseException('Cannot inject retained old executable bytes.');
  if Mode = 'old-readonly' then
    if not SetFixtureFileAttributes(ExpandConstant('{app}\bin\manyhands.exe'), FILE_ATTRIBUTE_READONLY) then
      RaiseException('Cannot inject retained read-only executable.');
  if Mode = 'old-cli' then
    if not SaveStringToFile(ExpandConstant('{app}\bin\manyhands-cli.exe'), 'old skipped executable', False) then
      RaiseException('Cannot inject skipped CLI bytes.');
  if Mode = 'old-license' then
    if not SaveStringToFile(ExpandConstant('{app}\licenses\installer\INNO-LICENSE.txt'), 'old licence bytes', False) then
      RaiseException('Cannot inject retained licence bytes.');
  if Mode = 'old-manifest' then
    if not SaveStringToFile(ExpandConstant('{app}\licenses\installer\managed-files.txt'), 'old managed manifest', False) then
      RaiseException('Cannot inject retained managed manifest bytes.');
  if Mode = 'forged-installed-digests' then
    WriteFixtureForgedDigests(ExpandConstant('{app}\licenses\installer\managed-digests.txt'));
  if Mode = 'forged-temporary-digests' then
    WriteFixtureForgedDigests(ExpandConstant('{tmp}\managed-digests.txt'));
  if Mode = 'missing' then DeleteFile(ExpandConstant('{app}\bin\manyhands-cli.exe'));
  if Mode = 'malformed' then IncomingManagedFiles[0] := 'bin\..\user-state.txt';
  if Mode = 'collision' then IncomingManagedFiles[0] := 'bin\MANYHANDS-CLI.exe';
  if Mode = 'post-reparse' then AddFixtureReparsePoint;
  if Mode = 'prunefail' then
    if not SetFixtureFileAttributes(ExpandConstant('{app}\bin\stale.dll'), FILE_ATTRIBUTE_READONLY) then
      RaiseException('Cannot set controlled stale-file failure.');
  PruneInstallerManagedFiles;
  Log('Completion gate returned; advancing Version and clearing PendingVersion.');
  RegWriteStringValue(HKCU64, InstallerKey, 'Version', '2.0.0');
  RegDeleteValue(HKCU64, InstallerKey, 'PendingVersion');
end;

#include "..\..\..\packaging\windows\installer-completion.iss"

procedure CancelButtonClick(CurPageID: Integer; var Cancel, Confirm: Boolean);
begin
  FixtureCancelSeen := True;
  Cancel := True;
  Confirm := False;
end;

procedure DeinitializeSetup;
var
  Mode, Value: String;
  Contents: AnsiString;
  Untouched, Completed: Boolean;
begin
  if not FixtureInitialized then Exit;
  Mode := ExpandConstant('{param:Mode}');
  Untouched := (Mode = 'preparecancel') or (Mode = 'startfail') or (Mode = 'pre-reparse');
  Completed := (Mode = 'success') or (Mode = 'fresh');
  try
    if ((Mode = 'preparecancel') or (Mode = 'midcancel')) and not FixtureCancelSeen then
      RaiseException('The actual engine cancellation path was not reached.');
    if not RegQueryStringValue(HKCU64, InstallerKey, 'Version', Value) then RaiseException('Missing fixture version.');
    if Completed then begin
      if Value <> '2.0.0' then RaiseException('Success did not advance version.');
    end else if Value <> '1.0.0' then RaiseException('Incomplete install advanced version.');
    if RegValueExists(HKCU64, InstallerKey, 'PendingVersion') <> (not Untouched and not Completed) then
      RaiseException('Wrong cancellation/failure repair floor.');
    if not Untouched and not Completed then begin
      if CheckInstalledVersion('1.5.0') = '' then RaiseException('Partial install allowed downgrade.');
      if CheckInstalledVersion('2.0.0') <> '' then RaiseException('Same-version repair rejected.');
      if CheckInstalledVersion('2.1.0') <> '' then RaiseException('Newer repair rejected.');
    end;
    if Untouched then begin
      if not LoadStringFromFile(ExpandConstant('{app}\bin\manyhands.exe'), Contents) or (Contents <> 'old executable') then
        RaiseException('Cancelled/failed start modified the executable.');
      if CheckInstalledVersion('1.5.0') <> '' then RaiseException('Read-only preparation left a floor.');
    end;
    if Mode = 'midcancel' then
      if not LoadStringFromFile(ExpandConstant('{app}\bin\manyhands.exe'), Contents) or (Contents = 'old executable') then
        RaiseException('Mid-copy cancellation fixture did not overwrite its executable.');
    if FileExists(ExpandConstant('{app}\bin\stale.dll')) = Completed then RaiseException('Wrong stale executable state.');
    if FileExists(ExpandConstant('{app}\licenses\old-crate\COPYING')) = Completed then RaiseException('Wrong stale licence state.');
    if Mode <> 'fresh' then
      if not LoadStringFromFile(ExpandConstant('{app}\user-state.txt'), Contents) or (Contents <> 'preserve user state') then
        RaiseException('User state changed.');
    if not LoadStringFromFile(ExpandConstant('{param:FixtureDir}\outside\keep.txt'), Contents) or (Contents <> 'outside managed trees') then
      RaiseException('Reparse point traversal modified an outside file.');
    if not SaveStringToFile(ExpandConstant('{param:ResultFile}'), 'passed', False) then RaiseException('Cannot record fixture result.');
  finally
    SetFixtureFileAttributes(ExpandConstant('{param:FixtureDir}\app\bin\manyhands.exe'), FILE_ATTRIBUTE_NORMAL);
    SetFixtureFileAttributes(ExpandConstant('{param:FixtureDir}\app\bin\stale.dll'), FILE_ATTRIBUTE_NORMAL);
    if not RegDeleteKeyIncludingSubkeys(HKCU64, '{#FixtureRoot}') then RaiseException('Fixture registry cleanup failed.');
  end;
end;
