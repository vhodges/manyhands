{ Registry adapter shared with the isolated native fixture. These overrides
  are used only by tests; the release compiler uses the defaults below. }
#ifndef InstallerStateKey
  #define InstallerStateKey "Software\Manyhands\Installer"
#endif
#ifndef InstallerEnvironmentKey
  #define InstallerEnvironmentKey "Environment"
#endif

const
  InstallerKey = '{#InstallerStateKey}';
  EnvironmentKey = '{#InstallerEnvironmentKey}';

function QueryValueType(RootKey: HKEY; SubKey, ValueName: String; Flags: Cardinal;
  var ValueType: Cardinal; Data: NativeUInt; var DataSize: Cardinal): Longint;
  external 'RegGetValueW@advapi32.dll stdcall';

function ReadUserPath(var Value: String): Cardinal;
var
  Size: Cardinal;
  Status: Longint;
begin
  Value := '';
  Size := 0;
  Result := 0;
  Status := QueryValueType(HKCU, EnvironmentKey, 'Path', $10000006, Result, 0, Size);
  if Status = 2 then begin Result := 2; Exit; end;
  if (Status <> 0) or ((Result <> 1) and (Result <> 2)) then
    RaiseException('Cannot safely read the user PATH registry value.');
  if not RegQueryStringValue(HKCU, EnvironmentKey, 'Path', Value) then
    RaiseException('Cannot read the user PATH. No PATH change was made.');
end;

function ExpandPathEnvironment(Value: String): String;
var
  Start, Finish: Integer;
  Rest, Name, Replacement: String;
begin
  Result := '';
  while True do begin
    Start := Pos('%', Value);
    if Start = 0 then begin Result := Result + Value; Exit; end;
    Result := Result + Copy(Value, 1, Start - 1);
    Rest := Copy(Value, Start + 1, Length(Value));
    Finish := Pos('%', Rest);
    if Finish = 0 then begin Result := Result + Copy(Value, Start, Length(Value)); Exit; end;
    Name := Copy(Rest, 1, Finish - 1);
    Replacement := GetEnv(Name);
    if Replacement = '' then Replacement := '%' + Name + '%';
    Result := Result + Replacement;
    Value := Copy(Rest, Finish + 1, Length(Rest));
  end;
end;

function CheckInstalledVersion(Incoming: String): String;
var
  Installed: String;
  Names: TArrayOfString;
  Order, I: Integer;
  Known: Boolean;
begin
  Result := '';
  if not CompareSemVersions(Incoming, Incoming, Order) then begin
    Result := 'The incoming Manyhands version is invalid.';
    Exit;
  end;
  Known := False;
  Names := SplitPolicyString('Version;PendingVersion', ';');
  for I := 0 to Length(Names) - 1 do begin
    if RegValueExists(HKCU64, InstallerKey, Names[I]) then begin
      Known := True;
      if not RegQueryStringValue(HKCU64, InstallerKey, Names[I], Installed) then
        Result := 'Cannot read installed or unfinished Manyhands version metadata.'
      else if not CompareSemVersions(Incoming, Installed, Order) then
        Result := 'Installed or unfinished Manyhands version metadata is invalid. Uninstall it before recovery.'
      else if Order < 0 then
        Result := 'Manyhands ' + Installed + ' is installed or unfinished. Downgrades to ' + Incoming + ' are not allowed.';
      if Result <> '' then Exit;
    end;
  end;
  if not Known and RegKeyExists(HKCU64, InstallerKey) then
    Result := 'Cannot read installed Manyhands version metadata. Uninstall it before recovery.';
end;

procedure UpdateUserPath(Selected: Boolean; Entry: String);
var
  Current, Rechecked, Updated, PreviousOwned, NextOwned: String;
  ValueType: Cardinal;
  Written: Boolean;
begin
  PreviousOwned := '';
  if RegValueExists(HKCU64, InstallerKey, 'OwnedPath') and
     not RegQueryStringValue(HKCU64, InstallerKey, 'OwnedPath', PreviousOwned) then
    RaiseException('Cannot read installer PATH ownership.');
  if not Selected and (PreviousOwned = '') then Exit;
  ValueType := ReadUserPath(Current);
  if not PlanPathUpdate(Current, ExpandPathEnvironment(Current), Entry, PreviousOwned,
    Selected, Updated, NextOwned) then RaiseException('Refusing unrelated PATH ownership.');
  if Updated <> Current then begin
    ReadUserPath(Rechecked);
    if Rechecked <> Current then RaiseException('User PATH changed concurrently. Retry setup.');
    { Claim a new addition before mutation, so partial setup can remove it. }
    if (NextOwned <> '') and (NextOwned <> PreviousOwned) then
      if not RegWriteStringValue(HKCU64, InstallerKey, 'OwnedPath', NextOwned) then
        RaiseException('Cannot record PATH ownership; PATH was not changed.');
    if ValueType = 1 then Written := RegWriteStringValue(HKCU, EnvironmentKey, 'Path', Updated)
    else Written := RegWriteExpandStringValue(HKCU, EnvironmentKey, 'Path', Updated);
    if not Written then begin
      RegWriteStringValue(HKCU64, InstallerKey, 'OwnedPath', PreviousOwned);
      RaiseException('Cannot write the user PATH.');
    end;
    ReadUserPath(Rechecked);
    if Rechecked <> Updated then RaiseException('Cannot confirm the user PATH update.');
  end;
  if not RegWriteStringValue(HKCU64, InstallerKey, 'OwnedPath', NextOwned) then
    RaiseException('Cannot checkpoint installer PATH ownership.');
end;
