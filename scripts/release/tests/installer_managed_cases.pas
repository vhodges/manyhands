procedure RunInstallerManagedTests;
var
  Files, Bad, Digests: TArrayOfString;
  Hash: String;
  I: Integer;
begin
  Files := SplitPolicyString('bin\manyhands-cli.exe|bin\manyhands.exe|licenses\README.md|licenses\installer\INNO-LICENSE.txt|licenses\installer\managed-files.txt', '|');
  if not ValidManagedFileList(Files) then RaiseException('Valid managed list rejected.');
  Hash := '';
  for I := 1 to 64 do Hash := Hash + 'a';
  SetArrayLength(Digests, Length(Files));
  for I := 0 to Length(Files) - 1 do Digests[I] := Hash + '  ' + Files[I];
  if not ValidManagedDigestList(Files, Digests) then RaiseException('Valid bound digest list rejected.');
  Digests[0] := 'A' + Copy(Hash, 2, 63) + '  ' + Files[0];
  if ValidManagedDigestList(Files, Digests) then RaiseException('Noncanonical digest accepted.');
  Digests[0] := Hash + ' ' + Files[0];
  if ValidManagedDigestList(Files, Digests) then RaiseException('Malformed digest separator accepted.');
  Digests[0] := Hash + '  bin\..\outside';
  if ValidManagedDigestList(Files, Digests) then RaiseException('Unbound digest path accepted.');
  Digests[0] := Digests[1];
  if ValidManagedDigestList(Files, Digests) then RaiseException('Duplicate digest binding accepted.');
  Digests[0] := Hash + '  ' + Files[0];
  SetArrayLength(Digests, Length(Digests) - 1);
  if ValidManagedDigestList(Files, Digests) then RaiseException('Missing digest entry accepted.');
  SetArrayLength(Digests, Length(Files) + 1);
  if ValidManagedDigestList(Files, Digests) then RaiseException('Extra digest entry accepted.');
  Bad := SplitPolicyString('..|bin\..\outside|bin\\file|bin\a:stream|bin\CON.txt|bin\COM1|bin\CONIN$|bin\con .txt|bin\file.|bin\file |BIN\a|licenses\<file>|user\state|bin\a/b|bin\a' + #10 + 'b', '|');
  for I := 0 to Length(Bad) - 1 do
    if ValidManagedPath(Bad[I]) then RaiseException('Unsafe managed path accepted: ' + Bad[I]);
  SetArrayLength(Files, Length(Files) + 1);
  Files[Length(Files) - 1] := 'bin\Manyhands.exe';
  if ValidManagedFileList(Files) then RaiseException('Case collision accepted.');
  Files[Length(Files) - 1] := 'licenses\INSTALLER\other.txt';
  if ValidManagedFileList(Files) then RaiseException('Directory case alias accepted.');
  Files[Length(Files) - 1] := 'licenses\README.md\child';
  if ValidManagedFileList(Files) then RaiseException('File/directory collision accepted.');
  Bad := SplitPolicyString('bin\foo|bin\foo-bar|bin\foo\child', '|');
  if ValidManagedPaths(Bad) then RaiseException('Non-neighbour file/directory collision accepted.');
  Bad := SplitPolicyString('licenses\Pkg\z|licenses\Pkg-other\mid|licenses\pkg\a', '|');
  if ValidManagedPaths(Bad) then RaiseException('Non-neighbour directory case alias accepted.');
  Bad := SplitPolicyString('licenses\package\z|licenses\package-other\mid|licenses\package\a', '|');
  if not ValidManagedPaths(Bad) then RaiseException('Valid unordered directory siblings rejected.');
  Bad := SplitPolicyString('bin\A|licenses\b|bin\a', '|');
  if ValidManagedPaths(Bad) then RaiseException('Separated duplicate/case-alias file accepted.');
  Bad := SplitPolicyString('licenses\z|bin\b|licenses\a|bin\a', '|');
  SortManagedStrings(Bad, False);
  if (Bad[0] <> 'bin\a') or (Bad[3] <> 'licenses\z') then
    RaiseException('Managed lookup sorting failed.');
  if not SortedManagedListContains(Bad, 'BIN\B') or
     not SortedManagedListContains(Bad, 'licenses\z') then
    RaiseException('Managed binary lookup missed an existing file.');
  if SortedManagedListContains(Bad, 'bin\0') or
     SortedManagedListContains(Bad, 'licenses\zz') or
     SortedManagedListContains(Bad, 'bin\a\child') then
    RaiseException('Managed binary lookup accepted a missing file.');
  SetArrayLength(Bad, 0);
  SortManagedStrings(Bad, False);
  if SortedManagedListContains(Bad, 'bin\a') then RaiseException('Empty managed lookup matched a file.');
  SetArrayLength(Files, 1);
  Files[0] := 'bin\manyhands.exe';
  if ValidManagedFileList(Files) then RaiseException('Incomplete managed list accepted.');
end;
