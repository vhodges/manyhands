{ Inno handles ssPostInstall exceptions and otherwise can still exit 0.
  ssInstall exceptions are fatal in pinned 7.1.1, before any installation writes.
  BeginInstaller/CompleteInstaller are supplied by the installer or fixture. }
var
  InstallerCompleted: Boolean;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then BeginInstaller;
  if CurStep = ssPostInstall then begin
    try
      CompleteInstaller;
      InstallerCompleted := True;
    except
      Log('Manyhands completion failed: ' + GetExceptionMessage);
      RaiseException('Manyhands installation is incomplete. Repair with the same or newer installer. ' + GetExceptionMessage);
    end;
  end;
end;

function GetCustomSetupExitCode: Integer;
begin
  if InstallerCompleted then Result := 0 else Result := 42;
end;
