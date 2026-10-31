$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Import-Module (Join-Path $PSScriptRoot '../windows-tools.psm1') -Force

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

# Execute only the real verification function, not the script's Windows-only
# certificate-store entry point. Its two platform APIs are stubbed below.
$tokens = $null
$parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot '../windows-sign.ps1'), [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors | Out-String) }
$definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Confirm-Signatures' }, $true)
. ([scriptblock]::Create($definition.Extent.Text))

$certificate = [pscustomobject]@{ Thumbprint = 'PUBLIC-FIXTURE-IDENTITY' }
$state = @{
    Signature = $null
    NativeFails = $false
    Calls = [Collections.Generic.List[object]]::new()
    AuthenticodeCalls = 0
}
function Invoke-ReleaseTool {
    param([string]$File, [string[]]$Arguments)
    Assert-True ($File -ceq 'signtool-fixture') 'Unexpected native verification tool'
    Assert-True (($Arguments[0..3] -join ' ') -ceq 'verify /pa /all /tw') 'Signature/timestamp verification flags changed'
    $state.Calls.Add($Arguments)
    if ($state.NativeFails) { throw 'fixture native verification failed' }
}
function Get-AuthenticodeSignature {
    param([string]$LiteralPath)
    $state.AuthenticodeCalls++
    return $state.Signature
}
function New-ValidSignature {
    return [pscustomobject]@{
        Status = 'Valid'
        SignerCertificate = $certificate
        TimeStamperCertificate = [pscustomobject]@{ Subject = 'Public timestamp fixture' }
    }
}

$failures = [Collections.Generic.List[string]]::new()
function Invoke-Test([string]$Name, [scriptblock]$Action) {
    try { & $Action; Write-Output "Passed: $Name" }
    catch { $failures.Add("${Name}: $($_.Exception.Message)") }
}

Invoke-Test 'both verification APIs have per-file progress without return-value pollution' {
    $state.Signature = New-ValidSignature
    $files = @('C:/fixture/bin/manyhands.exe', 'C:/fixture/bin/manyhands-cli.exe')
    $records = @(Confirm-Signatures $files $certificate 'signtool-fixture' 6>&1)
    Assert-True ($state.Calls.Count -eq 2 -and $state.AuthenticodeCalls -eq 2) 'Both files must pass both verifiers'
    Assert-True ($records.Count -eq 8) 'Expected start/completion progress around both verifiers for each file'
    foreach ($record in $records) {
        Assert-True ($record -is [Management.Automation.InformationRecord]) 'Progress polluted verification return values'
    }
    foreach ($name in @('manyhands.exe', 'manyhands-cli.exe')) {
        foreach ($message in @("Starting SignTool verification: $name", "Completed SignTool verification: $name",
            "Starting Windows Authenticode verification: $name", "Completed Windows Authenticode verification: $name")) {
            Assert-True (($records -join "`n").Contains($message)) "Missing progress: $message"
        }
    }
}

foreach ($reason in @('missing timestamp', 'unexpected identity', 'invalid signature')) {
    Invoke-Test "reject $reason without announcing successful verification" {
        $state.Signature = New-ValidSignature
        switch ($reason) {
            'missing timestamp' { $state.Signature.TimeStamperCertificate = $null }
            'unexpected identity' { $state.Signature.SignerCertificate = [pscustomobject]@{ Thumbprint = 'OTHER-PUBLIC-IDENTITY' } }
            'invalid signature' { $state.Signature.Status = 'HashMismatch' }
        }
        $records = [Collections.Generic.List[object]]::new()
        $failure = $null
        try { Confirm-Signatures @('C:/fixture/bin/manyhands.exe') $certificate 'signtool-fixture' 6>&1 | ForEach-Object { $records.Add($_) } }
        catch { $failure = $_.Exception.Message }
        Assert-True ($null -ne $failure -and $failure.Contains('Invalid, untimestamped, or unexpected Authenticode identity')) 'Unverified signature was accepted or its diagnostic changed'
        Assert-True (($records -join "`n").Contains('Starting Windows Authenticode verification: manyhands.exe')) 'Missing progress before the in-process verifier'
        Assert-True (-not ($records -join "`n").Contains('Completed Windows Authenticode verification: manyhands.exe')) 'Rejected signature was announced as verified'
    }
}

Invoke-Test 'native verification failures propagate before the Authenticode call' {
    $state.NativeFails = $true
    $before = $state.AuthenticodeCalls
    $records = [Collections.Generic.List[object]]::new()
    $failure = $null
    try { Confirm-Signatures @('C:/fixture/bin/manyhands.exe') $certificate 'signtool-fixture' 6>&1 | ForEach-Object { $records.Add($_) } }
    catch { $failure = $_.Exception.Message }
    Assert-True ($failure -ceq 'fixture native verification failed') 'Native verification failure did not propagate'
    Assert-True ($state.AuthenticodeCalls -eq $before) 'Verification continued after native failure'
    Assert-True (($records -join "`n").Contains('Starting SignTool verification: manyhands.exe')) 'Missing progress before native verification'
    Assert-True (-not ($records -join "`n").Contains('Completed SignTool verification: manyhands.exe')) 'Failed native verification was announced as complete'
}

if ($failures.Count) { throw ($failures -join "`n") }
Write-Output 'Windows signing-progress tests passed (5 cases; platform APIs stubbed).'
