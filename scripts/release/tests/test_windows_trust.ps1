[CmdletBinding()]
param([switch]$Native)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Import-Module (Join-Path $PSScriptRoot '../windows-tools.psm1') -Force

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function New-TrustFixture {
    $directory = Join-Path ([IO.Path]::GetTempPath()) ('manyhands-trust-fixture-' + [Guid]::NewGuid())
    $null = [IO.Directory]::CreateDirectory($directory)
    $key = [Security.Cryptography.RSA]::Create(2048)
    $request = [Security.Cryptography.X509Certificates.CertificateRequest]::new(
        ('CN=Manyhands temporary trust test ' + [Guid]::NewGuid()), $key,
        [Security.Cryptography.HashAlgorithmName]::SHA256, [Security.Cryptography.RSASignaturePadding]::Pkcs1)
    $certificate = $request.CreateSelfSigned([DateTimeOffset]::UtcNow.AddMinutes(-5), [DateTimeOffset]::UtcNow.AddHours(1))
    $path = Join-Path $directory 'public fixture.cer'
    [IO.File]::WriteAllBytes($path, $certificate.Export([Security.Cryptography.X509Certificates.X509ContentType]::Cert))
    return [pscustomobject]@{ Directory = $directory; Key = $key; Certificate = $certificate; Path = $path }
}

function Remove-TrustFixture($Fixture) {
    $Fixture.Certificate.Dispose()
    $Fixture.Key.Dispose()
    Microsoft.PowerShell.Management\Remove-Item -LiteralPath $Fixture.Directory -Recurse -Force
}

# Run actual Windows store operations in a fresh process, before any portable
# stubs below are defined. Private fixture keys remain in memory, never PFXs.
if ($Native) {
    $store = Get-HostedVerificationTrustStore
    $fixture = New-TrustFixture
    $owned = $null
    try {
        $expected = "$store\$($fixture.Certificate.Thumbprint)"
        Assert-True (-not (Test-Path -LiteralPath $expected)) 'Fixture identity unexpectedly already exists'
        try {
            Add-VerificationTrust -CertificatePath $fixture.Path -Thumbprint $fixture.Certificate.Thumbprint -OwnedPath ([ref]$owned)
            Assert-True ($owned -ceq $expected -and (Test-Path -LiteralPath $expected)) 'Native public-root import failed'
            $secondOwner = $null
            Add-VerificationTrust -CertificatePath $fixture.Path -Thumbprint $fixture.Certificate.Thumbprint -OwnedPath ([ref]$secondOwner)
            Assert-True ($null -eq $secondOwner) 'Existing trust was claimed by a second caller'
            Remove-VerificationTrust -OwnedPath $secondOwner
            Assert-True (Test-Path -LiteralPath $expected) 'Unowned existing trust was removed'
            throw 'controlled fixture verification failure'
        } catch {
            if ($_.Exception.Message -cne 'controlled fixture verification failure') { throw }
        } finally {
            Remove-VerificationTrust -OwnedPath $owned
        }
        Assert-True (-not (Test-Path -LiteralPath $expected)) 'Owned native trust survived caller cleanup'
    } finally {
        try { Remove-VerificationTrust -OwnedPath $owned }
        finally { Remove-TrustFixture $fixture }
    }
    Write-Output 'Native hosted-Windows trust import/preservation/exception-cleanup fixture passed.'
    return
}

# Load the exact production helper bodies into this test scope so only the
# context and certificate-store boundaries are stubbed. The module itself is
# left intact for the isolated native child above.
$tokens = $null
$parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot '../windows-tools.psm1'), [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors | Out-String) }
foreach ($name in @('Add-VerificationTrust', 'Remove-VerificationTrust')) {
    $definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $true)
    if ($null -ne $definition) { . ([scriptblock]::Create($definition.Extent.Text)) }
}

$fixture = New-TrustFixture
$path = "Cert:\LocalMachine\Root\$($fixture.Certificate.Thumbprint)"
$state = @{
    Context = @{}
    Roots = @{}
    ImportCalls = 0
    RemoveCalls = [Collections.Generic.List[string]]::new()
    Owner = $null
    PartialFailure = $false
    WrongImportIdentity = $false
    RemovalFails = $false
    RemovalNoEffect = $false
}
function Reset-Store {
    $state.Context = @{ Windows = $true; Actions = 'true'; RunnerEnvironment = 'github-hosted'; Administrator = $true }
    $state.Roots = @{}
    $state.ImportCalls = 0
    $state.RemoveCalls.Clear()
    $state.Owner = $null
    $state.PartialFailure = $false
    $state.WrongImportIdentity = $false
    $state.RemovalFails = $false
    $state.RemovalNoEffect = $false
}
function Get-HostedVerificationTrustStore { $context = $state.Context; return Get-VerificationTrustStore @context }
function Test-Path {
    [CmdletBinding()]
    param([string]$LiteralPath)
    return $state.Roots.ContainsKey($LiteralPath)
}
function Import-Certificate {
    [CmdletBinding()]
    param([string]$FilePath, [string]$CertStoreLocation, [switch]$Confirm)
    Assert-True ($FilePath -ceq $fixture.Path -and $CertStoreLocation -ceq 'Cert:\LocalMachine\Root') 'Wrong public certificate or trust scope'
    Assert-True (-not $Confirm) 'PowerShell confirmation was not explicitly disabled'
    Assert-True ($state.Owner.Value -ceq $path) 'Ownership was not recorded before the import'
    $state.ImportCalls++
    $state.Roots[$path] = $true
    if ($state.PartialFailure) { throw 'fixture partial import failure' }
    $thumbprint = if ($state.WrongImportIdentity) { '0' * 40 } else { $fixture.Certificate.Thumbprint }
    return [pscustomobject]@{ Thumbprint = $thumbprint }
}
function Remove-Item {
    [CmdletBinding()]
    param([string]$LiteralPath, [switch]$Force)
    Assert-True ($LiteralPath -ceq $path -and $Force) 'Removal escaped the exact owned root identity'
    $state.RemoveCalls.Add($LiteralPath)
    if ($state.RemovalFails) { throw 'fixture trust removal failure' }
    if (-not $state.RemovalNoEffect) { $state.Roots.Remove($LiteralPath) }
}

$failures = [Collections.Generic.List[string]]::new()
$counts = @{ Total = 0 }
function Invoke-Test([string]$Name, [scriptblock]$Action) {
    $counts.Total++
    Reset-Store
    try { & $Action; Write-Output "Passed: $Name" }
    catch { $failures.Add("${Name}: $($_.Exception.Message)") }
}
function Assert-Rejected([scriptblock]$Action, [string]$Message) {
    $failure = $null
    try { & $Action | Out-Null } catch { $failure = $_.Exception.Message }
    Assert-True ($null -ne $failure -and $failure.Contains($Message)) 'Expected rejection diagnostic was missing'
}

try {
    Invoke-Test 'only elevated GitHub-hosted Windows selects machine-root trust' {
        $context = $state.Context
        Assert-True ((Get-VerificationTrustStore @context) -ceq 'Cert:\LocalMachine\Root') 'Wrong verification trust store'
    }
    Invoke-Test 'unsupported contexts fail before any store mutation' {
        foreach ($change in @(@{ Windows = $false }, @{ Actions = '' }, @{ RunnerEnvironment = 'self-hosted' }, @{ Administrator = $false })) {
            Reset-Store
            foreach ($key in $change.Keys) { $state.Context[$key] = $change[$key] }
            $owned = $null
            Assert-Rejected { Add-VerificationTrust $fixture.Path $fixture.Certificate.Thumbprint ([ref]$owned) } 'elevated GitHub-hosted Windows runner'
            Assert-True ($null -eq $owned -and $state.ImportCalls -eq 0) 'Rejected context mutated verification trust'
        }
    }
    Invoke-Test 'pre-existing trust is preserved and never owned or removed' {
        $state.Roots[$path] = $true
        $owned = $null
        Add-VerificationTrust $fixture.Path $fixture.Certificate.Thumbprint ([ref]$owned)
        Remove-VerificationTrust $owned
        Assert-True ($null -eq $owned -and $state.ImportCalls -eq 0 -and $state.RemoveCalls.Count -eq 0) 'Existing trust was modified'
    }
    Invoke-Test 'public identity import records ownership first and exact cleanup removes it' {
        $owned = $null
        $state.Owner = [ref]$owned
        $records = @(Add-VerificationTrust $fixture.Path $fixture.Certificate.Thumbprint ([ref]$owned) 6>&1)
        Assert-True ($owned -ceq $path -and $state.ImportCalls -eq 1) 'New trust was not owned'
        Assert-True ($records.Count -eq 2 -and @($records | Where-Object { $_ -isnot [Management.Automation.InformationRecord] }).Count -eq 0) 'Progress polluted trust helper return values'
        Remove-VerificationTrust $owned
        Assert-True (-not $state.Roots.ContainsKey($path) -and $state.RemoveCalls.Count -eq 1) 'Owned trust was not removed'
    }
    Invoke-Test 'partially successful import is still owned and cleaned by the caller' {
        $owned = $null
        $state.Owner = [ref]$owned
        $state.PartialFailure = $true
        try { Assert-Rejected { Add-VerificationTrust $fixture.Path $fixture.Certificate.Thumbprint ([ref]$owned) } 'fixture partial import failure' }
        finally { Remove-VerificationTrust $owned }
        Assert-True ($owned -ceq $path -and -not $state.Roots.ContainsKey($path)) 'Partial import escaped cleanup'
    }
    Invoke-Test 'mismatched public identity is rejected before mutation' {
        $owned = $null
        Assert-Rejected { Add-VerificationTrust $fixture.Path ('0' * 40) ([ref]$owned) } 'public certificate'
        Assert-True ($null -eq $owned -and $state.ImportCalls -eq 0) 'Mismatched identity mutated trust'
    }
    Invoke-Test 'invalid thumbprint is rejected before mutation' {
        $owned = $null
        Assert-Rejected { Add-VerificationTrust $fixture.Path '*' ([ref]$owned) } 'thumbprint'
        Assert-True ($null -eq $owned -and $state.ImportCalls -eq 0) 'Invalid identity mutated trust'
    }
    Invoke-Test 'private-container file names are not accepted as public certificate inputs' {
        $owned = $null
        Assert-Rejected { Add-VerificationTrust ($fixture.Path + '.pfx') $fixture.Certificate.Thumbprint ([ref]$owned) } 'public .cer'
        Assert-True ($null -eq $owned -and $state.ImportCalls -eq 0) 'Private-container input mutated trust'
    }
    Invoke-Test 'unexpected imported identity fails and still cleans the owned path' {
        $owned = $null
        $state.Owner = [ref]$owned
        $state.WrongImportIdentity = $true
        try { Assert-Rejected { Add-VerificationTrust $fixture.Path $fixture.Certificate.Thumbprint ([ref]$owned) } 'Imported verification trust identity changed' }
        finally { Remove-VerificationTrust $owned }
        Assert-True (-not $state.Roots.ContainsKey($path)) 'Failed import identity check escaped cleanup'
    }
    Invoke-Test 'cleanup refuses broad or unrelated certificate paths' {
        foreach ($unsafe in @('Cert:\LocalMachine\Root', "Cert:\CurrentUser\Root\$($fixture.Certificate.Thumbprint)",
            "Cert:\LocalMachine\My\$($fixture.Certificate.Thumbprint)", 'Cert:\LocalMachine\Root\*')) {
            Assert-Rejected { Remove-VerificationTrust $unsafe } 'owned machine-root'
        }
        Assert-True ($state.RemoveCalls.Count -eq 0) 'Unsafe cleanup touched the store'
    }
    Invoke-Test 'removal errors propagate instead of announcing successful cleanup' {
        $state.Roots[$path] = $true
        $state.RemovalFails = $true
        Assert-Rejected { Remove-VerificationTrust $path } 'fixture trust removal failure'
        Assert-True ($state.Roots.ContainsKey($path)) 'Removal failure was hidden'
    }
    Invoke-Test 'cleanup verifies that the owned certificate actually disappeared' {
        $state.Roots[$path] = $true
        $state.RemovalNoEffect = $true
        Assert-Rejected { Remove-VerificationTrust $path } 'still present'
    }
    Invoke-Test 'both modes share guarded trust and preflight before private-key import' {
        $source = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '../windows-sign.ps1'))
        Assert-True (-not $source.Contains('Cert:\CurrentUser\Root')) 'Signing script still imports current-user roots'
        $preflight = $source.IndexOf('Get-HostedVerificationTrustStore')
        Assert-True ($preflight -ge 0 -and $preflight -lt $source.IndexOf('Import-PfxCertificate')) 'Hosted/admin preflight is not before private-key import'
        $tokens = $null
        $errors = $null
        $scriptAst = [Management.Automation.Language.Parser]::ParseInput($source, [ref]$tokens, [ref]$errors)
        $imports = @($scriptAst.FindAll({ param($node) $node -is [Management.Automation.Language.CommandAst] -and $node.GetCommandName() -eq 'Add-VerificationTrust' }, $true))
        Assert-True ($imports.Count -eq 1) 'Both modes must share one guarded trust call'
        for ($node = $imports[0].Parent; $null -ne $node; $node = $node.Parent) {
            Assert-True ($node -isnot [Management.Automation.Language.IfStatementAst]) 'Verification trust is conditional on one mode'
        }
    }
    Invoke-Test 'trust cleanup failure still removes owned private identity and files' {
        $tokens = $null
        $errors = $null
        $scriptAst = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot '../windows-sign.ps1'), [ref]$tokens, [ref]$errors)
        $mainTry = @($scriptAst.EndBlock.Statements | Where-Object { $_ -is [Management.Automation.Language.TryStatementAst] })[0]
        $body = $mainTry.Finally.Extent.Text
        $cleanup = [scriptblock]::Create($body.Substring(1, $body.Length - 2))
        $storedCertificate = $null
        $collection = $null
        $certificate = $null
        $password = $null
        $trustCertificate = $path
        $myCertificate = "Cert:\CurrentUser\My\$($fixture.Certificate.Thumbprint)"
        $temporary = 'C:\owned-signing-temp-fixture'
        $cleanupState = @{ Removed = [Collections.Generic.List[string]]::new() }
        function Remove-VerificationTrust { param([string]$OwnedPath) throw 'fixture root cleanup failure' }
        function Test-Path { [CmdletBinding()]param([string]$LiteralPath) return $true }
        function Remove-Item {
            [CmdletBinding()]
            param([string]$LiteralPath, [switch]$Force, [switch]$Recurse, [switch]$DeleteKey)
            $cleanupState.Removed.Add($LiteralPath)
        }
        Assert-Rejected { & $cleanup } 'Could not fully clean the temporary Windows signing context'
        Assert-True ($cleanupState.Removed.Count -eq 2 -and $cleanupState.Removed.Contains($myCertificate) -and $cleanupState.Removed.Contains($temporary)) 'Trust cleanup failure prevented private-state cleanup'
    }
} finally { Remove-TrustFixture $fixture }

if ($failures.Count) { throw ($failures -join "`n") }
Write-Output "Windows verification-trust policy/ownership tests passed ($($counts.Total) cases; store APIs stubbed)."
if ($IsWindows -and $env:GITHUB_ACTIONS -ceq 'true' -and $env:RUNNER_ENVIRONMENT -ceq 'github-hosted') {
    # A fresh child has no store stubs. The native wrapper bounds its lifetime;
    # the workflow helper-step timeout is an additional cancellation backstop.
    $pwsh = Join-Path $PSHOME 'pwsh.exe'
    Invoke-ReleaseTool $pwsh @('-NoLogo', '-NoProfile', '-File', $PSCommandPath, '-Native') | Write-Output
} else {
    Write-Output 'Native machine-root fixture skipped outside GitHub-hosted Windows; no real trust store changed.'
}
