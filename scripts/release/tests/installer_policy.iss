#ifndef FixtureId
  #error A fresh fixture ID is required
#endif
#define FixtureRoot "Software\Manyhands\InstallerPolicyFixture-" + FixtureId
#define InstallerStateKey FixtureRoot + "\State"
#define InstallerEnvironmentKey FixtureRoot + "\Environment"

[Setup]
AppId=io.github.vhodges.manyhands.policy-test
AppName=Manyhands policy test
AppVersion=0.0.0
DefaultDirName={tmp}\manyhands-policy-unused
PrivilegesRequired=lowest
SetupArchitecture=x64
ArchitecturesInstallIn64BitMode=x64os
Uninstallable=no
CreateAppDir=no
OutputBaseFilename=installer-policy-test

[Code]
#include "..\..\..\packaging\windows\installer-policy.pas"
#include "..\..\..\packaging\windows\installer-managed-policy.pas"
#include "..\..\..\packaging\windows\installer-state.iss"
#include "installer_policy_cases.pas"
#include "installer_managed_cases.pas"
#include "installer_state_cases.pas"

function InitializeSetup: Boolean;
begin
  RunInstallerPolicyTests;
  RunInstallerManagedTests;
  if RegKeyExists(HKCU64, '{#FixtureRoot}') then RaiseException('Refusing pre-existing fixture state.');
  try
    RunInstallerStateTests;
  finally
    if RegKeyExists(HKCU64, '{#FixtureRoot}') then
      if not RegDeleteKeyIncludingSubkeys(HKCU64, '{#FixtureRoot}') then RaiseException('Fixture registry cleanup failed.');
  end;
  if not SaveStringToFile(ExpandConstant('{param:ResultFile}'), 'passed', False) then
    RaiseException('Could not record policy-test results.');
  Result := False; { Exit before installation: no files, shortcuts, or registry edits. }
end;
