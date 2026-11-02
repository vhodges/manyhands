{ Executes the production registry adapter against isolated fixture subkeys. }
procedure RunInstallerStateTests;
var
  Entry, Value, Owned, Before: String;
  Kind: Cardinal;
  Rejected: Boolean;
begin
  CheckPolicy(CheckInstalledVersion('1.0.0') = '', 'Fresh installation rejected');
  CheckPolicy(RegWriteStringValue(HKCU64, InstallerKey, 'Version', '1.0.0-rc.2'), 'Fixture version write failed');
  CheckPolicy(CheckInstalledVersion('1.0.0-rc.2') = '', 'Reinstall rejected');
  CheckPolicy(CheckInstalledVersion('1.0.0') = '', 'Stable upgrade from preview rejected');
  CheckPolicy(CheckInstalledVersion('1.0.0-rc.1') <> '', 'Preview downgrade accepted');
  CheckPolicy(RegWriteStringValue(HKCU64, InstallerKey, 'Version', '1.0.0'), 'Stable fixture write failed');
  CheckPolicy(CheckInstalledVersion('1.0.0-rc.9') <> '', 'Stable-to-preview downgrade accepted');
  CheckPolicy(RegDeleteValue(HKCU64, InstallerKey, 'Version'), 'Fixture delete failed');
  CheckPolicy(CheckInstalledVersion('1.0.0') <> '', 'Missing installed-version state accepted');
  CheckPolicy(RegWriteDWordValue(HKCU64, InstallerKey, 'Version', 7), 'Fixture invalid-type write failed');
  CheckPolicy(CheckInstalledVersion('1.0.0') <> '', 'Unreadable installed-version state accepted');
  CheckPolicy(RegWriteStringValue(HKCU64, InstallerKey, 'Version', '1.0.0'), 'Recovery fixture version failed');
  CheckPolicy(RegWriteStringValue(HKCU64, InstallerKey, 'PendingVersion', '2.0.0-rc.2'), 'Pending version fixture failed');
  CheckPolicy(CheckInstalledVersion('1.9.9') <> '', 'Interrupted upgrade allowed a downgrade');
  CheckPolicy(CheckInstalledVersion('2.0.0-rc.2') = '', 'Interrupted upgrade could not be repaired');
  CheckPolicy(RegDeleteValue(HKCU64, InstallerKey, 'Version'), 'Pending-only fixture failed');
  CheckPolicy(CheckInstalledVersion('2.0.0') = '', 'Interrupted first install could not be repaired');
  CheckPolicy(RegWriteStringValue(HKCU64, InstallerKey, 'PendingVersion', 'invalid'), 'Invalid pending fixture failed');
  CheckPolicy(CheckInstalledVersion('2.0.0') <> '', 'Invalid pending version accepted');

  Entry := 'C:\Manyhands Policy Fixture With Spaces\bin';
  UpdateUserPath(False, Entry);
  CheckPolicy(not RegValueExists(HKCU, EnvironmentKey, 'Path'), 'Opt-out created PATH');
  UpdateUserPath(True, Entry);
  Kind := ReadUserPath(Value);
  CheckPolicy((Kind = 2) and (Value = Entry), 'Owned PATH addition or REG_EXPAND_SZ type failed');
  CheckPolicy(RegQueryStringValue(HKCU64, InstallerKey, 'OwnedPath', Owned) and (Owned = Entry), 'PATH ownership not recorded');
  UpdateUserPath(True, Entry);
  ReadUserPath(Value);
  CheckPolicy(Value = Entry, 'Reinstall duplicated PATH');
  UpdateUserPath(False, Entry);
  ReadUserPath(Value);
  CheckPolicy(Value = '', 'Owned PATH not removed');

  Before := 'C:\Other;"c:\manyhands policy fixture with spaces\BIN\";C:\Tail';
  { RegWriteStringValue deliberately preserves an existing REG_EXPAND_SZ. }
  CheckPolicy(RegDeleteValue(HKCU, EnvironmentKey, 'Path'), 'Fixture PATH reset failed');
  CheckPolicy(RegWriteStringValue(HKCU, EnvironmentKey, 'Path', Before), 'REG_SZ fixture write failed');
  UpdateUserPath(True, Entry);
  UpdateUserPath(False, Entry);
  Kind := ReadUserPath(Value);
  CheckPolicy((Kind = 1) and (Value = Before), 'Unowned PATH or REG_SZ type changed: type=' + IntToStr(Kind) + '; value=' + Value);

  Before := '%USERPROFILE%\Manyhands Policy Fixture\bin';
  Entry := ExpandPathEnvironment(Before);
  CheckPolicy(Entry <> Before, 'USERPROFILE fixture expansion failed');
  CheckPolicy(RegWriteExpandStringValue(HKCU, EnvironmentKey, 'Path', Before), 'Expanded fixture write failed');
  UpdateUserPath(True, Entry);
  UpdateUserPath(False, Entry);
  ReadUserPath(Value);
  CheckPolicy(Value = Before, 'Pre-existing environment-variable PATH entry changed');

  CheckPolicy(RegWriteStringValue(HKCU64, InstallerKey, 'OwnedPath', Entry), 'Duplicate ownership fixture failed');
  CheckPolicy(RegWriteStringValue(HKCU, EnvironmentKey, 'Path', Entry + ';' + Entry), 'Duplicate PATH fixture failed');
  UpdateUserPath(False, Entry);
  ReadUserPath(Value);
  CheckPolicy(Value = Entry, 'Owned removal deleted both duplicate entries');
  CheckPolicy(RegWriteStringValue(HKCU64, InstallerKey, 'OwnedPath', 'C:\Unrelated'), 'Unrelated ownership fixture failed');
  Rejected := False;
  try UpdateUserPath(False, Entry); except Rejected := True; end;
  CheckPolicy(Rejected, 'Unrelated PATH ownership was accepted');
  ReadUserPath(Value);
  CheckPolicy(Value = Entry, 'Unrelated PATH was removed');
  CheckPolicy(RegWriteDWordValue(HKCU, EnvironmentKey, 'Path', 123), 'Unsupported PATH type fixture failed');
  Rejected := False;
  try ReadUserPath(Value); except Rejected := True; end;
  CheckPolicy(Rejected, 'Unsupported PATH registry type accepted');
end;
