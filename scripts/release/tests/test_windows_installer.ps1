$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Import-Module (Join-Path $PSScriptRoot '../windows-tools.psm1') -Force

$tokens = $null
$errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot '../windows-installer.ps1'), [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw ($errors | Out-String) }
$definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-InnoCompilerArguments' }, $true)
. ([scriptblock]::Create($definition.Extent.Text))
$definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Invoke-InnoCompiler' }, $true)
if ($null -eq $definition) { throw 'Compiler build needs an explicit larger native timeout' }
. ([scriptblock]::Create($definition.Extent.Text))
& {
    function Invoke-ReleaseTool {
        param([string]$File, [string[]]$Arguments, [int]$TimeoutSeconds)
        if ($File -cne 'fixture-iscc' -or ($Arguments -join ' ') -cne '/Q fixture.iss' -or $TimeoutSeconds -ne 1800) {
            throw 'Compiler must receive its full argument vector and explicit 1800-second budget'
        }
        return 'compiled'
    }
    if ((Invoke-InnoCompiler 'fixture-iscc' @('/Q', 'fixture.iss')) -cne 'compiled') { throw 'Compiler result was lost' }
}
$definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Assert-InnoCompiler' }, $true)
if ($null -eq $definition) { throw 'Compiler version must be checked through its actual engine version command' }
. ([scriptblock]::Create($definition.Extent.Text))
& {
    $state = @{ Version = '7.1.1' }
    function Invoke-ReleaseTool {
        param([string]$File, [string[]]$Arguments)
        if ($File -cne 'fixture-iscc' -or ($Arguments -join ' ') -cne '--version') { throw 'Wrong compiler identity probe' }
        return $state.Version
    }
    Assert-InnoCompiler 'fixture-iscc' '7.1.1'
    $state.Version = '7.0.0'
    $rejected = $false
    try { Assert-InnoCompiler 'fixture-iscc' '7.1.1' } catch { $rejected = $_.Exception.Message.Contains('Wrong Inno compiler version') }
    if (-not $rejected) { throw 'Unpinned compiler was accepted' }
}
$arguments = Get-InnoCompilerArguments -Payload 'C:\payload with spaces' -Output 'C:\output with spaces' `
    -Version '1.2.3-rc.1' -NumericVersion '1.2.3.0' -Identity ('A' * 40) `
    -Signer 'C:\SDK tools\signtool.exe' -PowerShell 'C:\Program Files\PowerShell\pwsh.exe' `
    -Callback 'C:\source files\windows-sign-file.ps1' -Definition 'C:\source files\manyhands.iss' `
    -ManagedFiles 'C:\output with spaces\managed-files.txt' -ManagedDigests 'C:\output with spaces\managed-digests.txt'
foreach ($expected in @('/DPayloadDir=C:\payload with spaces', '/OC:\output with spaces', '/DAppVersion=1.2.3-rc.1', '/DNumericVersion=1.2.3.0', '/DManagedFiles=C:\output with spaces\managed-files.txt', '/DManagedDigests=C:\output with spaces\managed-digests.txt')) {
    if ($arguments -cnotcontains $expected) { throw "Missing argument-vector element: $expected" }
}
$signing = @($arguments | Where-Object { $_.StartsWith('/Smanyhands=') })
if ($signing.Count -ne 1 -or -not $signing[0].Contains('-FilePath $f') -or -not $signing[0].Contains('$qC:\SDK tools\signtool.exe$q')) {
    throw 'Compiler callback does not safely quote files and select the mandatory signer'
}
if (@($arguments | Where-Object { $_ -match '^[-/](NS|no-signing)$' }).Count) { throw 'Unsigned compiler fallback is forbidden' }
Write-Output 'Installer compiler argument-vector/signing callback cases passed.'

if ($IsWindows) {
    $null = Get-HostedVerificationTrustStore
    if (-not $env:INNO_COMPILER) { throw 'Native policy tests require the pinned Inno compiler' }
    $owned = Join-Path ([IO.Path]::GetTempPath()) ('manyhands-inno-policy-' + [Guid]::NewGuid())
    $null = New-Item -ItemType Directory -Path $owned
    try {
        Invoke-ReleaseTool $env:INNO_COMPILER @('/Q', "/O$owned", ('/DFixtureId=' + [Guid]::NewGuid().ToString('N')), (Join-Path $PSScriptRoot 'installer_policy.iss')) | Out-Null
        $report = Join-Path $owned 'passed.txt'
        $expectedExit = $false
        try {
            Invoke-ReleaseTool (Join-Path $owned 'installer-policy-test.exe') @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', "/ResultFile=$report") | Out-Null
        } catch {
            if ($_.Exception.Message -notlike 'Native release tool failed (1):*') { throw }
            $expectedExit = $true
        }
        if (-not $expectedExit -or -not (Test-Path -LiteralPath $report) -or (Get-Content -Raw -LiteralPath $report) -cne 'passed') {
            throw 'Inno Pascal policy fixture did not pass and exit before installation'
        }
        Invoke-ReleaseTool $env:INNO_COMPILER @('/Q', "/O$owned", (Join-Path $PSScriptRoot 'installer_completion.iss')) | Out-Null
        foreach ($expectedCode in @(42, 0)) {
            $report = Join-Path $owned "completion-$expectedCode.txt"
            $failure = if ($expectedCode -eq 42) { '1' } else { '0' }
            try {
                Invoke-ReleaseTool (Join-Path $owned 'installer-completion-test.exe') @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', "/ResultFile=$report", "/Failure=$failure") | Out-Null
                if ($expectedCode -ne 0) { throw 'Post-install failure was silently reported as success' }
            } catch {
                if ($expectedCode -eq 0 -or $_.Exception.Message -notlike "Native release tool failed (${expectedCode}):*") { throw }
            }
            if (-not (Test-Path -LiteralPath $report) -or (Get-Content -Raw -LiteralPath $report) -cne 'attempted') {
                throw 'Post-install fixture did not reach its controlled completion operation'
            }
        }
    } finally { Remove-Item -LiteralPath $owned -Recurse -Force }
    Write-Output 'Native Inno Pascal Script SemVer/PATH fixture passed; no installation performed.'
    Write-Output 'Native Inno completion fixture returned 42 on post-install failure and 0 on success; no app files/registry changed.'
    # Real-engine lifecycle/pruning and mandatory-signing retry fixtures use
    # fresh owned directories/registry keys, never the user's app or PATH.
    Invoke-ReleaseTool (Get-Command python -ErrorAction Stop).Source @('-B', (Join-Path $PSScriptRoot 'test_installer_engine.py')) -TimeoutSeconds 300 | Out-Null
} else {
    Write-Output 'Native Inno policy fixture skipped outside Windows; local FPC executes the same policy/cases.'
}
