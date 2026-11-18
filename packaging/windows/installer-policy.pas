{ Pure policy shared by Inno's Pascal Script and executable local tests. }
function SplitPolicyString(Value, Separator: String): TArrayOfString;
var
  Position, Count: Integer;
begin
  SetLength(Result, 0);
  Count := 0;
  while True do begin
    Position := Pos(Separator, Value);
    SetLength(Result, Count + 1);
    if Position = 0 then begin
      Result[Count] := Value;
      Break;
    end;
    Result[Count] := Copy(Value, 1, Position - 1);
    Delete(Value, 1, Position + Length(Separator) - 1);
    Count := Count + 1;
  end;
end;

function PolicyDigits(Value: String): Boolean;
var
  I: Integer;
begin
  Result := False;
  if Value = '' then Exit;
  for I := 1 to Length(Value) do
    if (Value[I] < '0') or (Value[I] > '9') then Exit;
  Result := True;
end;

function ParsePolicyVersion(Value: String; var Core, Preview: TArrayOfString): Boolean;
var
  I, J, Dash: Integer;
  Suffix: String;
begin
  Result := False;
  SetLength(Preview, 0);
  Dash := Pos('-', Value);
  if Dash > 0 then begin
    Suffix := Copy(Value, Dash + 1, Length(Value));
    Value := Copy(Value, 1, Dash - 1);
    Preview := SplitPolicyString(Suffix, '.');
    for I := 0 to Length(Preview) - 1 do begin
      if Preview[I] = '' then Exit;
      for J := 1 to Length(Preview[I]) do
        if not (((Preview[I][J] >= '0') and (Preview[I][J] <= '9')) or
                ((Preview[I][J] >= 'A') and (Preview[I][J] <= 'Z')) or
                ((Preview[I][J] >= 'a') and (Preview[I][J] <= 'z')) or
                (Preview[I][J] = '-')) then Exit;
      if PolicyDigits(Preview[I]) and (Length(Preview[I]) > 1) and (Preview[I][1] = '0') then Exit;
    end;
  end;
  Core := SplitPolicyString(Value, '.');
  if Length(Core) <> 3 then Exit;
  for I := 0 to 2 do begin
    if not PolicyDigits(Core[I]) then Exit;
    if (Length(Core[I]) > 1) and (Core[I][1] = '0') then Exit;
  end;
  Result := True;
end;

function ComparePolicyNumber(Left, Right: String): Integer;
begin
  Result := 0;
  if Length(Left) < Length(Right) then Result := -1
  else if Length(Left) > Length(Right) then Result := 1
  else if Left < Right then Result := -1
  else if Left > Right then Result := 1;
end;

function CompareSemVersions(Left, Right: String; var Order: Integer): Boolean;
var
  LeftCore, RightCore, LeftPre, RightPre: TArrayOfString;
  I: Integer;
  LeftNumeric, RightNumeric: Boolean;
begin
  Result := False;
  Order := 0;
  if not ParsePolicyVersion(Left, LeftCore, LeftPre) then Exit;
  if not ParsePolicyVersion(Right, RightCore, RightPre) then Exit;
  Result := True;
  for I := 0 to 2 do begin
    Order := ComparePolicyNumber(LeftCore[I], RightCore[I]);
    if Order <> 0 then Exit;
  end;
  if (Length(LeftPre) = 0) and (Length(RightPre) = 0) then Exit;
  if Length(LeftPre) = 0 then begin Order := 1; Exit; end;
  if Length(RightPre) = 0 then begin Order := -1; Exit; end;
  I := 0;
  while (I < Length(LeftPre)) and (I < Length(RightPre)) do begin
    LeftNumeric := PolicyDigits(LeftPre[I]);
    RightNumeric := PolicyDigits(RightPre[I]);
    if LeftNumeric and RightNumeric then Order := ComparePolicyNumber(LeftPre[I], RightPre[I])
    else if LeftNumeric then Order := -1
    else if RightNumeric then Order := 1
    else if LeftPre[I] < RightPre[I] then Order := -1
    else if LeftPre[I] > RightPre[I] then Order := 1;
    if Order <> 0 then Exit;
    I := I + 1;
  end;
  if Length(LeftPre) < Length(RightPre) then Order := -1
  else if Length(LeftPre) > Length(RightPre) then Order := 1;
end;

function NormalizePolicyPath(Value: String): String;
begin
  Result := Trim(Value);
  if (Length(Result) >= 2) and (Result[1] = '"') and (Result[Length(Result)] = '"') then
    Result := Copy(Result, 2, Length(Result) - 2);
  while (Length(Result) > 3) and (Result[Length(Result)] = '\') do
    Delete(Result, Length(Result), 1);
  Result := LowerCase(Result);
end;

function PathContains(Value, Entry: String): Boolean;
var
  Parts: TArrayOfString;
  I: Integer;
begin
  Result := False;
  Parts := SplitPolicyString(Value, ';');
  for I := 0 to Length(Parts) - 1 do
    if NormalizePolicyPath(Parts[I]) = NormalizePolicyPath(Entry) then begin
      Result := True;
      Exit;
    end;
end;

function PlanPathUpdate(Current, ExpandedCurrent, Entry, PreviousOwned: String;
  Selected: Boolean; var Updated, NextOwned: String): Boolean;
var
  Parts: TArrayOfString;
  I, Count: Integer;
  Found: Boolean;
begin
  Result := False;
  Updated := Current;
  NextOwned := PreviousOwned;
  if (PreviousOwned <> '') and (NormalizePolicyPath(PreviousOwned) <> NormalizePolicyPath(Entry)) then Exit;
  if Selected and ((Pos(';', Entry) > 0) or (Pos('"', Entry) > 0)) then Exit;
  Parts := SplitPolicyString(Current, ';');
  Found := False;
  for I := 0 to Length(Parts) - 1 do
    if (PreviousOwned <> '') and (Parts[I] = PreviousOwned) then Found := True;
  if not Found then NextOwned := '';
  if Selected then begin
    if not PathContains(ExpandedCurrent, Entry) then begin
      if Current = '' then Updated := Entry else Updated := Current + ';' + Entry;
      NextOwned := Entry;
    end;
  end else if Found then begin
    Updated := '';
    Found := False;
    Count := 0;
    for I := 0 to Length(Parts) - 1 do begin
      if not Found and (Parts[I] = PreviousOwned) then Found := True
      else begin
        if Count > 0 then Updated := Updated + ';';
        Updated := Updated + Parts[I];
        Count := Count + 1;
      end;
    end;
    NextOwned := '';
  end;
  Result := True;
end;
