{ Preparation is read-only. In pinned Inno 7.1.1 ssInstall is fatal on an
  exception and precedes PerformInstall (Setup.MainForm.pas:220-225). Unlike
  ssPostInstall, it cannot swallow a failed checkpoint and proceed to copy. }
function CheckInstallPreparation(Incoming: String): String;
var
  PreviousDir, PreviousOwned, Current, Updated, NextOwned, Entry: String;
  PathType: Cardinal;
begin
  Result := CheckInstalledVersion(Incoming);
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
    ValidateInstallerDestination;
  except
    Result := GetExceptionMessage;
  end;
end;

procedure StartInstaller(Incoming: String);
var
  Failure: String;
begin
  Failure := CheckInstallPreparation(Incoming);
  if Failure <> '' then RaiseException(Failure);
  CheckpointInstaller(Incoming, ExpandConstant('{app}'));
  { Never clear this merely because setup is cancelled: Inno does not restore
    overwritten files on rollback. Same/newer repair must remain mandatory. }
end;
