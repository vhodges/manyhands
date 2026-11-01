{ Shared by the actual Inno adapter and FPC/engine fixtures. Only these two
  trees belong to the installer. Root files and user data are never pruned. }
function ValidManagedPath(Value: String): Boolean;
var
  Parts: TArrayOfString;
  Part, Base: String;
  I, J, Dot: Integer;
begin
  Result := False;
  Parts := SplitPolicyString(Value, '\');
  if Length(Parts) < 2 then Exit;
  if (Parts[0] <> 'bin') and (Parts[0] <> 'licenses') then Exit;
  for I := 0 to Length(Parts) - 1 do begin
    Part := Parts[I];
    if (Part = '') or (Part = '.') or (Part = '..') then Exit;
    if (Part[Length(Part)] = '.') or (Part[Length(Part)] = ' ') then Exit;
    for J := 1 to Length(Part) do
      if (Ord(Part[J]) < 32) or (Ord(Part[J]) > 126) or
         (Pos(Part[J], '<>:"|?*/') <> 0) then Exit;
    Base := Uppercase(Part);
    Dot := Pos('.', Base);
    if Dot <> 0 then Base := Copy(Base, 1, Dot - 1);
    while (Length(Base) > 0) and (Base[Length(Base)] = ' ') do Delete(Base, Length(Base), 1);
    if (Base = 'CON') or (Base = 'CONIN$') or (Base = 'CONOUT$') or
       (Base = 'PRN') or (Base = 'AUX') or (Base = 'NUL') then Exit;
    if Length(Base) = 4 then
      if ((Copy(Base, 1, 3) = 'COM') or (Copy(Base, 1, 3) = 'LPT')) and
         (Base[4] >= '1') and (Base[4] <= '9') then Exit;
  end;
  Result := True;
end;

function ManagedListContains(Files: TArrayOfString; Name: String): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 0 to Length(Files) - 1 do
    if CompareText(Files[I], Name) = 0 then begin Result := True; Exit; end;
end;

function ValidManagedPaths(Files: TArrayOfString): Boolean;
var
  I, J, K, Count: Integer;
  LeftParts, RightParts: TArrayOfString;
begin
  Result := False;
  for I := 0 to Length(Files) - 1 do begin
    if not ValidManagedPath(Files[I]) then Exit;
    LeftParts := SplitPolicyString(Files[I], '\');
    for J := 0 to I - 1 do begin
      RightParts := SplitPolicyString(Files[J], '\');
      Count := Length(LeftParts);
      if Length(RightParts) < Count then Count := Length(RightParts);
      for K := 0 to Count - 1 do begin
        if CompareText(LeftParts[K], RightParts[K]) <> 0 then Break;
        { Equal components must use identical case, including directories. }
        if LeftParts[K] <> RightParts[K] then Exit;
        if K = Count - 1 then Exit; { Duplicate or file/directory collision. }
      end;
    end;
  end;
  Result := True;
end;

function ValidManagedFileList(Files: TArrayOfString): Boolean;
begin
  Result := ValidManagedPaths(Files) and
    ManagedListContains(Files, 'bin\manyhands.exe') and
    ManagedListContains(Files, 'bin\manyhands-cli.exe') and
    ManagedListContains(Files, 'licenses\README.md') and
    ManagedListContains(Files, 'licenses\installer\INNO-LICENSE.txt') and
    ManagedListContains(Files, 'licenses\installer\managed-files.txt');
end;

function ValidManagedDigestList(Files, Digests: TArrayOfString): Boolean;
var
  I, J: Integer;
  Line: String;
begin
  Result := False;
  if not ValidManagedFileList(Files) or (Length(Files) <> Length(Digests)) then Exit;
  for I := 0 to Length(Files) - 1 do begin
    Line := Digests[I];
    if Length(Line) <> 66 + Length(Files[I]) then Exit;
    if (Copy(Line, 65, 2) <> '  ') or (Copy(Line, 67, Length(Line)) <> Files[I]) then Exit;
    for J := 1 to 64 do
      if not (((Line[J] >= '0') and (Line[J] <= '9')) or
              ((Line[J] >= 'a') and (Line[J] <= 'f'))) then Exit;
  end;
  Result := True;
end;
