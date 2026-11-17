{ Filesystem adapter. The incoming list is embedded by the compiler from the
  verified ZIP, never read from an older installation or mutable installed list. }
var
  IncomingManagedFiles, IncomingManagedDigests, IncomingManagedLookup: TArrayOfString;
  IncomingManagedReady: Boolean;

function ManagedFileAttributes(Name: String): Cardinal;
  external 'GetFileAttributesW@kernel32.dll stdcall';

procedure AppendManagedFile(var Files: TArrayOfString; Name: String);
var
  Count: Integer;
begin
  Count := Length(Files);
  SetArrayLength(Files, Count + 1);
  Files[Count] := Name;
end;

function CheckedManagedAttributes(Name: String; AllowMissing: Boolean): Cardinal;
var
  ErrorCode: Cardinal;
begin
  Result := ManagedFileAttributes(Name);
  if Result = $FFFFFFFF then begin
    { Use Inno's captured error: interpreter API calls may clobber the thread's
      live GetLastError value between script instructions. }
    ErrorCode := DLLGetLastError;
    if AllowMissing and ((ErrorCode = 2) or (ErrorCode = 3)) then Exit;
    RaiseException('Cannot inspect installer-owned path: ' + Name);
  end;
  if (Result and $400) <> 0 then RaiseException('Refusing installer reparse point: ' + Name);
end;

procedure CheckManagedAncestors(Name: String; AllowMissing: Boolean);
var
  Parent, Root, Normalized: String;
  Attributes: Cardinal;
begin
  Root := NormalizePolicyPath(ExpandFileName(ExpandConstant('{app}')));
  Name := ExpandFileName(Name);
  Normalized := NormalizePolicyPath(Name);
  if (Normalized <> Root) and (Copy(Normalized, 1, Length(Root) + 1) <> Root + '\') then
    RaiseException('Installer path is outside the application directory: ' + Name);
  while True do begin
    Attributes := CheckedManagedAttributes(Name, AllowMissing);
    if (Attributes <> $FFFFFFFF) and ((Attributes and FILE_ATTRIBUTE_DIRECTORY) = 0) then
      RaiseException('Installer directory is not a directory: ' + Name);
    { The app and its descendants are owned. A relocated profile/AppData
      ancestor is not: allow its junction without following it for pruning. }
    if NormalizePolicyPath(Name) = Root then Exit;
    Parent := ExtractFileDir(Name);
    if (Parent = '') or (Parent = Name) then
      RaiseException('Installer path does not reach the application directory.');
    Name := Parent;
  end;
end;

procedure CollectManagedFiles(Relative: String; AllowMissing: Boolean; var Files: TArrayOfString);
var
  Name, Child: String;
  Attributes: Cardinal;
  Found: TFindRec;
begin
  if not ValidManagedPath(Relative + '\__directory_check__') then
    RaiseException('Invalid installer-owned directory.');
  Name := ExpandConstant('{app}\') + Relative;
  Attributes := CheckedManagedAttributes(Name, AllowMissing);
  if Attributes = $FFFFFFFF then Exit;
  if (Attributes and FILE_ATTRIBUTE_DIRECTORY) = 0 then RaiseException('Invalid installer-owned tree.');
  if not FindFirst(Name + '\*', Found) then RaiseException('Cannot enumerate installer-owned tree: ' + Name);
  try
    repeat
      if (Found.Name <> '.') and (Found.Name <> '..') then begin
        Child := Relative + '\' + Found.Name;
        Attributes := CheckedManagedAttributes(ExpandConstant('{app}\') + Child, False);
        if (Attributes and FILE_ATTRIBUTE_DIRECTORY) <> 0 then CollectManagedFiles(Child, False, Files)
        else begin
          if not ValidManagedPath(Child) then RaiseException('Invalid installed managed file path.');
          AppendManagedFile(Files, Child);
        end;
      end;
    until not FindNext(Found);
  finally
    FindClose(Found);
  end;
end;

procedure LoadIncomingManagedFiles;
var
  I: Integer;
begin
  if IncomingManagedReady then RaiseException('Installer expectations are already loaded.');
  ExtractTemporaryFile('managed-files.txt');
  ExtractTemporaryFile('managed-digests.txt');
  if not LoadStringsFromFile(ExpandConstant('{tmp}\managed-files.txt'), IncomingManagedFiles) or
     not LoadStringsFromFile(ExpandConstant('{tmp}\managed-digests.txt'), IncomingManagedDigests) or
     not ValidManagedDigestList(IncomingManagedFiles, IncomingManagedDigests) then
    RaiseException('The embedded installer managed-file or digest list is invalid.');
  { Cache embedded expectations once. Never consult mutable installed trust
    metadata (or reread temporary trust inputs) during completion. }
  SetArrayLength(IncomingManagedLookup, Length(IncomingManagedFiles));
  for I := 0 to Length(IncomingManagedFiles) - 1 do
    IncomingManagedLookup[I] := IncomingManagedFiles[I];
  SortManagedStrings(IncomingManagedLookup, False);
  IncomingManagedReady := True;
end;

procedure ValidateInstallerDestination;
var
  Existing: TArrayOfString;
begin
  if not IncomingManagedReady then RaiseException('Installer expectations have not been loaded.');
  CheckManagedAncestors(ExpandConstant('{app}'), True);
  SetArrayLength(Existing, 0);
  CollectManagedFiles('bin', True, Existing);
  CollectManagedFiles('licenses', True, Existing);
  if not ValidManagedPaths(Existing) then RaiseException('Colliding installed managed-file paths.');
end;

procedure PruneInstallerManagedFiles;
var
  Existing: TArrayOfString;
  I: Integer;
  Name: String;
  Attributes: Cardinal;
begin
  { The embedded expectations were validated once at loading. Validate every
    expected file and fresh complete trees before the first deletion. Missing/
    corrupt/reparsed destination inputs never authorize deletion. }
  if not IncomingManagedReady then RaiseException('Installer expectations have not been loaded.');
  CheckManagedAncestors(ExpandConstant('{app}'), False);
  SetArrayLength(Existing, 0);
  CollectManagedFiles('bin', False, Existing);
  CollectManagedFiles('licenses', False, Existing);
  if not ValidManagedPaths(Existing) then RaiseException('Colliding installed managed-file paths.');
  SortManagedStrings(Existing, False);
  for I := 0 to Length(IncomingManagedFiles) - 1 do begin
    Name := ExpandConstant('{app}\') + IncomingManagedFiles[I];
    CheckManagedAncestors(ExtractFileDir(Name), False);
    Attributes := CheckedManagedAttributes(Name, False);
    if ((Attributes and FILE_ATTRIBUTE_DIRECTORY) <> 0) or
        not SortedManagedListContains(Existing, IncomingManagedFiles[I]) then
      RaiseException('Missing expected installed managed file: ' + IncomingManagedFiles[I]);
    { Inno Keep existing/Skip may reach ssPostInstall with old bytes at a valid
      path. Verify ALL bytes, including this manifest and the Inno licence,
      before any stale file can be removed or completion can be recorded. }
    if GetSHA256OfFile(Name) <> Copy(IncomingManagedDigests[I], 1, 64) then
      RaiseException('Installed managed file contents do not match the incoming installer: ' + IncomingManagedFiles[I]);
  end;
  for I := 0 to Length(Existing) - 1 do
    if not SortedManagedListContains(IncomingManagedLookup, Existing[I]) then begin
      Name := ExpandConstant('{app}\') + Existing[I];
      CheckManagedAncestors(ExtractFileDir(Name), False);
      Attributes := CheckedManagedAttributes(Name, False);
      if ((Attributes and FILE_ATTRIBUTE_DIRECTORY) <> 0) or not DeleteFile(Name) then
        RaiseException('Cannot remove stale installer-owned file: ' + Existing[I]);
    end;
  { Empty obsolete directories may remain. Never recursively delete a tree. }
end;
