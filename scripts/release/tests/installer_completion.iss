[Setup]
AppId=io.github.vhodges.manyhands.completion-test
AppName=Manyhands completion test
AppVersion=0.0.0
DefaultDirName={tmp}\manyhands-completion-unused
PrivilegesRequired=lowest
SetupArchitecture=x64
Uninstallable=no
CreateAppDir=no
DisableWelcomePage=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
OutputBaseFilename=installer-completion-test

[Code]
procedure CompleteInstaller;
begin
  if not SaveStringToFile(ExpandConstant('{param:ResultFile}'), 'attempted', False) then
    RaiseException('Cannot write completion fixture result.');
  if ExpandConstant('{param:Failure|1}') = '1' then
    RaiseException('Controlled post-install failure: no files or registry changed.');
end;

#include "..\..\..\packaging\windows\installer-completion.iss"
