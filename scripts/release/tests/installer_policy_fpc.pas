program InstallerPolicyTests;
{$mode delphi}{$H+}
uses SysUtils;
type TArrayOfString = array of String;
procedure RaiseException(MessageText: String);
begin
  raise Exception.Create(MessageText);
end;
{$I installer-policy.pas}
{$I installer_policy_cases.pas}
begin
  RunInstallerPolicyTests;
  WriteLn('Installer SemVer/PATH policy cases passed (shared production Pascal).');
end.
