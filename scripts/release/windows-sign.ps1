[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Staging,
    [ValidateSet('Sign', 'Verify')][string]$Mode = 'Sign'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Native Windows signing/verification requires Windows and PowerShell 7' }
Import-Module (Join-Path $PSScriptRoot 'windows-tools.psm1') -Force

function Confirm-CodeSigningUsage($Certificate) {
    $eku = @($Certificate.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' })
    if ($eku.Count -ne 1 -or -not @($eku[0].EnhancedKeyUsages | Where-Object { $_.Value -eq '1.3.6.1.5.5.7.3.3' }).Count) {
        throw 'Certificate does not permit code signing'
    }
    if ($Certificate.Subject -cne $Certificate.Issuer) { throw 'Expected one self-signed publisher certificate' }
}

function Confirm-Signatures([string[]]$Files, $Certificate, [string]$SignTool) {
    foreach ($file in $Files) {
        $name = [IO.Path]::GetFileName($file)
        Write-ReleaseProgress "Starting SignTool verification: $name"
        Invoke-ReleaseTool $SignTool @('verify', '/pa', '/all', '/tw', $file) | Out-Null
        Write-ReleaseProgress "Completed SignTool verification: $name"
        Write-ReleaseProgress "Starting Windows Authenticode verification: $name"
        $signature = Get-AuthenticodeSignature -LiteralPath $file
        if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate -or
            $signature.SignerCertificate.Thumbprint -ne $Certificate.Thumbprint -or
            $null -eq $signature.TimeStamperCertificate) {
            throw "Invalid, untimestamped, or unexpected Authenticode identity: $file"
        }
        Write-ReleaseProgress "Completed Windows Authenticode verification: $name"
    }
}

Write-ReleaseProgress "Starting Windows signing/verification (mode $Mode)"
$root = (Resolve-Path -LiteralPath $Staging).Path
$python = (Get-Command python -ErrorAction Stop).Source
$helper = Join-Path $PSScriptRoot 'windows.py'
Write-ReleaseProgress 'Starting Windows SDK SignTool discovery'
$signTool = Find-SignTool
Write-ReleaseProgress 'Completed Windows SDK SignTool discovery'
$files = @((Join-Path $root 'bin/manyhands.exe'), (Join-Path $root 'bin/manyhands-cli.exe'))
$temporary = Join-Path ([IO.Path]::GetTempPath()) ('manyhands-signing-' + [Guid]::NewGuid())
$myCertificate = $null
$trustCertificate = $null
$identity = $null
$collection = $null
$password = $null
$storedCertificate = $null
$certificate = $null

try {
    New-Item -ItemType Directory -Path $temporary | Out-Null
    if ($Mode -eq 'Sign') {
        if (-not $env:WINDOWS_SIGNING_PFX_BASE64 -or -not $env:WINDOWS_SIGNING_PFX_PASSWORD) {
            throw 'WINDOWS_SIGNING_PFX_BASE64 and WINDOWS_SIGNING_PFX_PASSWORD are required'
        }
        Write-ReleaseProgress 'Starting in-memory PFX loading and identity validation'
        $bytes = [Convert]::FromBase64String($env:WINDOWS_SIGNING_PFX_BASE64)
        $password = ConvertTo-SecureString $env:WINDOWS_SIGNING_PFX_PASSWORD -AsPlainText -Force
        $collection = [Security.Cryptography.X509Certificates.X509Certificate2Collection]::new()
        $collection.Import($bytes, $env:WINDOWS_SIGNING_PFX_PASSWORD, [Security.Cryptography.X509Certificates.X509KeyStorageFlags]::EphemeralKeySet)
        if ($collection.Count -ne 1 -or -not $collection[0].HasPrivateKey) { throw 'PFX must contain exactly one signing identity with its private key' }
        $identity = $collection[0]
        Confirm-CodeSigningUsage $identity
        if ($identity.NotBefore.ToUniversalTime() -gt [DateTime]::UtcNow -or $identity.NotAfter.ToUniversalTime() -le [DateTime]::UtcNow) {
            throw 'Signing certificate is not currently valid'
        }
        Write-ReleaseProgress 'Completed in-memory PFX loading and identity validation'
        $myPath = "Cert:\CurrentUser\My\$($identity.Thumbprint)"
        if (Test-Path -LiteralPath $myPath) { throw 'Refusing to replace an existing runner signing identity' }
        $pfx = Join-Path $temporary 'identity.pfx'
        [IO.File]::WriteAllBytes($pfx, $bytes)
        [Array]::Clear($bytes, 0, $bytes.Length)
        Remove-Item Env:\WINDOWS_SIGNING_PFX_BASE64, Env:\WINDOWS_SIGNING_PFX_PASSWORD
        # Record ownership before import so a partial import is cleaned up.
        $myCertificate = $myPath
        Write-ReleaseProgress 'Starting signing identity import into CurrentUser/My'
        $storedCertificate = Import-PfxCertificate -FilePath $pfx -CertStoreLocation 'Cert:\CurrentUser\My' -Password $password
        Remove-Item -LiteralPath $pfx -Force
        if ($storedCertificate.Thumbprint -ne $identity.Thumbprint) { throw 'Imported signing identity changed' }
        $certificate = $storedCertificate
        Write-ReleaseProgress 'Completed signing identity import into CurrentUser/My'
        Write-ReleaseProgress 'Starting public certificate export'
        New-Item -ItemType Directory -Path (Join-Path $root 'signing') -Force | Out-Null
        Export-Certificate -Cert $certificate -FilePath (Join-Path $root 'signing/manyhands.cer') -Type CERT | Out-Null
        Write-ReleaseProgress 'Completed public certificate export'
    } else {
        Write-ReleaseProgress 'Starting packaged public identity validation'
        Invoke-ReleaseTool $python @('-B', $helper, 'verify', '--staging', $root) | Out-Null
        $certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new((Join-Path $root 'signing/manyhands.cer'))
        Confirm-CodeSigningUsage $certificate
        Write-ReleaseProgress 'Completed packaged public identity validation'
    }

    Write-ReleaseProgress 'Starting Windows runtime import audit'
    $runtime = Get-WindowsRuntimeReport $root
    $runtimePath = Join-Path $temporary 'runtime.json'
    $runtime | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $runtimePath -Encoding utf8NoBOM
    Write-ReleaseProgress 'Completed Windows runtime import audit'
    if ($Mode -eq 'Sign') {
        $configuration = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot '../../packaging/windows/signing.json') | ConvertFrom-Json
        $timestamp = Get-SignToolTimestampUrl -Url $configuration.timestamp_url
        foreach ($file in $files) {
            $name = [IO.Path]::GetFileName($file)
            Write-ReleaseProgress "Starting signing and RFC 3161 timestamping: $name"
            Invoke-ReleaseTool $signTool @('sign', '/s', 'My', '/sha1', $certificate.Thumbprint,
                '/fd', 'SHA256', '/tr', $timestamp, '/td', 'SHA256', $file) | Out-Null
            Write-ReleaseProgress "Completed signing and RFC 3161 timestamping: $name"
        }
    }

    $rootPath = "Cert:\CurrentUser\Root\$($certificate.Thumbprint)"
    if (-not (Test-Path -LiteralPath $rootPath)) {
        $trustCertificate = $rootPath
        Write-ReleaseProgress 'Starting temporary verification trust import'
        Import-Certificate -FilePath (Join-Path $root 'signing/manyhands.cer') -CertStoreLocation 'Cert:\CurrentUser\Root' | Out-Null
        Write-ReleaseProgress 'Completed temporary verification trust import'
    } else {
        Write-ReleaseProgress 'Verification trust already present; existing certificate will be preserved'
    }
    Confirm-Signatures $files $certificate $signTool

    if ($Mode -eq 'Sign') {
        Write-ReleaseProgress 'Starting verified signing metadata finalization'
        $fingerprint = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($certificate.RawData))
        $signer = [ordered]@{
            subject = $certificate.Subject
            thumbprint_sha1 = $certificate.Thumbprint
            fingerprint_sha256 = $fingerprint
            expires_utc = $certificate.NotAfter.ToUniversalTime().ToString('o')
            timestamp_url = $timestamp
        }
        $signerPath = Join-Path $temporary 'signer.json'
        $signer | ConvertTo-Json | Set-Content -LiteralPath $signerPath -Encoding utf8NoBOM
        Invoke-ReleaseTool $python @('-B', $helper, 'finalize', '--staging', $root, '--runtime', $runtimePath, '--signer', $signerPath) | Out-Null
        Write-ReleaseProgress 'Completed verified signing metadata finalization'
    } else {
        Write-ReleaseProgress 'Starting packaged runtime inventory comparison'
        $manifest = Get-Content -LiteralPath (Join-Path $root 'release.json') -Raw | ConvertFrom-Json
        foreach ($name in @('manyhands.exe', 'manyhands-cli.exe')) {
            $recorded = @($manifest.runtime_audit.binaries.PSObject.Properties[$name].Value)
            if (Compare-Object $runtime.binaries[$name] $recorded) { throw "Packaged PE imports changed: $name" }
        }
        Write-ReleaseProgress 'Completed packaged runtime inventory comparison'
    }
    # Python clears the build-tool PATH and isolates HOME/current directory.
    Write-ReleaseProgress 'Starting packaged CLI smoke test'
    Invoke-ReleaseTool $python @('-B', $helper, 'smoke', '--staging', $root) | Out-Null
    Write-ReleaseProgress 'Completed packaged CLI smoke test'
    Write-Output "Verified Windows Authenticode signatures and runtime imports ($Mode)."
} catch {
    Write-ReleaseProgress "Failed Windows signing/verification (mode $Mode); entering cleanup"
    throw
} finally {
    Write-ReleaseProgress 'Starting signing cleanup: disposing certificate handles'
    if ($null -ne $storedCertificate) { $storedCertificate.Dispose() }
    if ($null -ne $collection) { foreach ($item in $collection) { $item.Dispose() } }
    if ($null -ne $certificate -and $Mode -eq 'Verify') { $certificate.Dispose() }
    if ($null -ne $password) { $password.Dispose() }
    Write-ReleaseProgress 'Completed signing certificate handle disposal'
    $cleanupErrors = @()
    if ($null -ne $trustCertificate -and (Test-Path -LiteralPath $trustCertificate)) {
        Write-ReleaseProgress 'Removing owned temporary verification trust'
        Remove-Item -LiteralPath $trustCertificate -Force -ErrorAction SilentlyContinue -ErrorVariable +cleanupErrors
    }
    if ($null -ne $myCertificate -and (Test-Path -LiteralPath $myCertificate)) {
        Write-ReleaseProgress 'Removing owned signing identity and private key'
        Remove-Item -LiteralPath $myCertificate -DeleteKey -Force -ErrorAction SilentlyContinue -ErrorVariable +cleanupErrors
    }
    if (Test-Path -LiteralPath $temporary) {
        Write-ReleaseProgress 'Removing owned temporary signing files'
        Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue -ErrorVariable +cleanupErrors
    }
    if ($cleanupErrors.Count) {
        Write-ReleaseProgress 'Failed signing cleanup'
        throw 'Could not fully clean the temporary Windows signing context'
    }
    Write-ReleaseProgress 'Completed signing cleanup'
}
