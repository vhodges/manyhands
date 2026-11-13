[Setup]
AppId=io.github.vhodges.manyhands.validation-scale-test
AppName=Manyhands isolated validation scale test
AppVersion=0.0.0
DefaultDirName={tmp}\manyhands-scale-unused
PrivilegesRequired=lowest
SetupArchitecture=x64
Uninstallable=no
CreateAppDir=no
OutputBaseFilename=installer-scale-test

[Code]
#include "..\..\..\packaging\windows\installer-policy.pas"
#include "..\..\..\packaging\windows\installer-managed-policy.pas"

function ScaleTicks: Cardinal;
  external 'GetTickCount@kernel32.dll stdcall';

function InitializeSetup: Boolean;
var
  Files, Digests: TArrayOfString;
  Pass, I: Integer;
  Started: Cardinal;
  Report: String;
begin
  if FileExists(ExpandConstant('{param:ResultFile}')) then RaiseException('Refusing existing scale report.');
  if not LoadStringsFromFile(ExpandConstant('{param:ManagedFiles}'), Files) or
     not LoadStringsFromFile(ExpandConstant('{param:ManagedDigests}'), Digests) then
    RaiseException('Cannot load scale fixture inputs.');
  Report := 'file_count=' + IntToStr(Length(Files)) + #13#10;
  { The old normal-upgrade path made four embedded-table validations and
    three destination validations. Keep the same work as a scale regression,
    even though production now validates the embedded tables only once. }
  for Pass := 0 to 6 do begin
    Started := ScaleTicks;
    if (Pass = 0) or (Pass = 1) or (Pass = 3) or (Pass = 5) then begin
      if not ValidManagedDigestList(Files, Digests) then RaiseException('Invalid scale digest inputs.');
    end else if not ValidManagedPaths(Files) then RaiseException('Invalid scale file inputs.');
    Report := Report + 'pass_' + IntToStr(Pass) + '_ms=' + IntToStr(ScaleTicks - Started) + #13#10;
  end;
  SortManagedStrings(Files, False);
  Started := ScaleTicks;
  for Pass := 0 to 1 do
    for I := 0 to Length(Files) - 1 do
      if not SortedManagedListContains(Files, Files[I]) then RaiseException('Scale membership lookup failed.');
  Report := Report + 'membership_ms=' + IntToStr(ScaleTicks - Started) + #13#10;
  if not SaveStringToFile(ExpandConstant('{param:ResultFile}'), Report, False) then
    RaiseException('Cannot record scale fixture result.');
  Result := False; { No files, registry keys or installation operations. }
end;
