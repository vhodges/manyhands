program InstallerPolicyTests;
{$mode delphi}{$H+}
uses SysUtils;
type TArrayOfString = array of String;
procedure SetArrayLength(var Values: TArrayOfString; Count: Integer);
begin
  SetLength(Values, Count);
end;
procedure RaiseException(MessageText: String);
begin
  raise Exception.Create(MessageText);
end;
{$I installer-policy.pas}
{$I installer-managed-policy.pas}
{$I installer_policy_cases.pas}
{$I installer_managed_cases.pas}
begin
  RunInstallerPolicyTests;
  RunInstallerManagedTests;
  WriteLn('Installer SemVer/PATH policy cases passed (shared production Pascal).');
end.
