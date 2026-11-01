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

function ManagedSortCompare(Left, Right: String; Tagged: Boolean): Integer;
begin
  if Tagged then begin
    Delete(Left, 1, 1);
    Delete(Right, 1, 1);
  end;
  Result := CompareText(Left, Right);
end;

procedure SortManagedStrings(var Values: TArrayOfString; Tagged: Boolean);
var
  Work: TArrayOfString;
  Count, Width, Start, Middle, Finish, Left, Right, Output, I: Integer;
begin
  { Bottom-up merge sort: no recursion or quadratic worst case. Tagged
    prefix nodes sort by their path, never by their directory/file marker. }
  Count := Length(Values);
  if Count < 2 then Exit;
  SetArrayLength(Work, Count);
  Width := 1;
  while Width < Count do begin
    Start := 0;
    while Start < Count do begin
      Middle := Start + Width;
      if Middle > Count then Middle := Count;
      Finish := Middle + Width;
      if Finish > Count then Finish := Count;
      Left := Start;
      Right := Middle;
      Output := Start;
      while (Left < Middle) and (Right < Finish) do begin
        if ManagedSortCompare(Values[Left], Values[Right], Tagged) <= 0 then begin
          Work[Output] := Values[Left];
          Left := Left + 1;
        end else begin
          Work[Output] := Values[Right];
          Right := Right + 1;
        end;
        Output := Output + 1;
      end;
      while Left < Middle do begin
        Work[Output] := Values[Left];
        Left := Left + 1;
        Output := Output + 1;
      end;
      while Right < Finish do begin
        Work[Output] := Values[Right];
        Right := Right + 1;
        Output := Output + 1;
      end;
      Start := Finish;
    end;
    for I := 0 to Count - 1 do Values[I] := Work[I];
    if Width > Count div 2 then Width := Count else Width := Width * 2;
  end;
end;

function SortedManagedListContains(Files: TArrayOfString; Name: String): Boolean;
var
  First, Last, Middle, Order: Integer;
begin
  Result := False;
  First := 0;
  Last := Length(Files) - 1;
  while First <= Last do begin
    Middle := First + (Last - First) div 2;
    Order := CompareText(Name, Files[Middle]);
    if Order = 0 then begin Result := True; Exit; end;
    if Order < 0 then Last := Middle - 1 else First := Middle + 1;
  end;
end;

function ValidManagedPaths(Files: TArrayOfString): Boolean;
var
  I, J, Count, Next: Integer;
  Nodes: TArrayOfString;
  Name, Previous: String;
begin
  Result := False;
  Count := 0;
  for I := 0 to Length(Files) - 1 do begin
    if not ValidManagedPath(Files[I]) then Exit;
    Count := Count + 1;
    for J := 1 to Length(Files[I]) do
      if Files[I][J] = '\' then Count := Count + 1;
  end;
  SetArrayLength(Nodes, Count);
  Next := 0;
  for I := 0 to Length(Files) - 1 do begin
    Name := Files[I];
    for J := 1 to Length(Name) do
      if Name[J] = '\' then begin
        Nodes[Next] := 'D' + Copy(Name, 1, J - 1);
        Next := Next + 1;
      end;
    Nodes[Next] := 'F' + Name;
    Next := Next + 1;
  end;
  SortManagedStrings(Nodes, True);
  for I := 1 to Length(Nodes) - 1 do begin
    Name := Copy(Nodes[I], 2, Length(Nodes[I]));
    Previous := Copy(Nodes[I - 1], 2, Length(Nodes[I - 1]));
    if CompareText(Previous, Name) = 0 then begin
      { Every prefix is indexed, so foo/foo-bar/foo\child cannot hide a
        collision. Only identically spelled directory nodes may repeat. }
      if (Previous <> Name) or (Nodes[I][1] <> 'D') or (Nodes[I - 1][1] <> 'D') then Exit;
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
