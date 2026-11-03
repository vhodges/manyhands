{ Shared executable cases: compiled locally with FPC and on Windows by Inno. }
procedure CheckPolicy(Condition: Boolean; MessageText: String);
begin
  if not Condition then RaiseException(MessageText);
end;

procedure CheckVersion(Left, Right: String; Expected: Integer);
var
  Order: Integer;
begin
  CheckPolicy(CompareSemVersions(Left, Right, Order), 'Valid SemVer rejected: ' + Left);
  CheckPolicy(Order = Expected, 'Wrong SemVer precedence: ' + Left + ' / ' + Right);
end;

procedure RunInstallerPolicyTests;
var
  Order, I: Integer;
  Values, Invalid: TArrayOfString;
  Entry, Updated, Owned: String;
begin
  Values := SplitPolicyString('1.0.0-alpha;1.0.0-alpha.1;1.0.0-alpha.beta;1.0.0-beta;1.0.0-beta.2;1.0.0-beta.11;1.0.0-rc.1;1.0.0', ';');
  for I := 0 to Length(Values) - 1 do begin
    CheckVersion(Values[I], Values[I], 0);
    if I > 0 then begin
      CheckVersion(Values[I-1], Values[I], -1);
      CheckVersion(Values[I], Values[I-1], 1);
    end;
  end;
  CheckVersion('1.9.9', '2.0.0-alpha', -1);
  CheckVersion('1.0.0-9999999999999999999999999999999', '1.0.0-10000000000000000000000000000000', -1);
  CheckVersion('1.0.0-1', '1.0.0-A', -1);
  CheckVersion('1.0.0-A', '1.0.0-a', -1);
  CheckVersion('9999999999999999999999999999999.0.0', '2.0.0', 1);
  Invalid := SplitPolicyString(';1;v1.0.0;01.0.0;1.01.0;1.0.01;1.0.0-;1.0.0-a..b;1.0.0-01;1.0.0+a;1.0.0-a_1;1.0.0.0; 1.0.0', ';');
  for I := 0 to Length(Invalid) - 1 do begin
    CheckPolicy(not CompareSemVersions(Invalid[I], '1.0.0', Order), 'Invalid installed SemVer accepted');
    CheckPolicy(not CompareSemVersions('1.0.0', Invalid[I], Order), 'Invalid incoming SemVer accepted');
  end;

  Entry := 'C:\Programs With Spaces\Manyhands\bin';
  CheckPolicy(PathContains('C:\Tools;"c:\programs with spaces\manyhands\BIN\";C:\Other', Entry), 'Quoted/case/trailing slash duplicate missed');
  CheckPolicy(not PathContains(Entry + '-other', Entry), 'PATH substring treated as an entry');
  CheckPolicy(PlanPathUpdate('C:\Tools;', 'C:\Tools;', Entry, '', True, Updated, Owned), 'Append rejected');
  CheckPolicy((Updated = 'C:\Tools;;' + Entry) and (Owned = Entry), 'Append did not preserve original PATH bytes');
  CheckPolicy(PlanPathUpdate(Updated, Updated, Entry, Owned, False, Updated, Owned), 'Owned removal rejected');
  CheckPolicy((Updated = 'C:\Tools;') and (Owned = ''), 'Owned removal did not restore trailing separator');
  CheckPolicy(PlanPathUpdate(Entry, Entry, Entry, '', True, Updated, Owned), 'Existing entry rejected');
  CheckPolicy((Updated = Entry) and (Owned = ''), 'Existing entry incorrectly claimed');
  CheckPolicy(PlanPathUpdate('%LOCALAPPDATA%\Programs\Manyhands\bin', Entry, Entry, '', True, Updated, Owned), 'Expanded entry rejected');
  CheckPolicy((Updated = '%LOCALAPPDATA%\Programs\Manyhands\bin') and (Owned = ''), 'Expanded pre-existing entry changed');
  CheckPolicy(PlanPathUpdate('', '', Entry, '', False, Updated, Owned), 'PATH opt-out rejected');
  CheckPolicy((Updated = '') and (Owned = ''), 'Opt-out modified PATH');
  CheckPolicy(PlanPathUpdate(Entry, Entry, Entry, Entry, True, Updated, Owned), 'Reinstall rejected');
  CheckPolicy((Updated = Entry) and (Owned = Entry), 'Reinstall duplicated or lost ownership');
  CheckPolicy(PlanPathUpdate('C:\Other', 'C:\Other', Entry, Entry, False, Updated, Owned), 'Missing owned entry rejected');
  CheckPolicy((Updated = 'C:\Other') and (Owned = ''), 'Missing owned entry changed unrelated PATH');
  CheckPolicy(PlanPathUpdate(Entry + ';' + Entry, Entry + ';' + Entry, Entry, Entry, False, Updated, Owned), 'Duplicate removal rejected');
  CheckPolicy((Updated = Entry) and (Owned = ''), 'Removal deleted more than one entry');
  CheckPolicy(not PlanPathUpdate('C:\Other', 'C:\Other', Entry, 'C:\Other', False, Updated, Owned), 'Unrelated PATH ownership accepted');
  CheckPolicy(not PlanPathUpdate('', '', 'C:\Semi;Colon\bin', '', True, Updated, Owned), 'Unsafe PATH separator accepted');
end;
