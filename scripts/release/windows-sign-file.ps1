[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$FilePath,
    [Parameter(Mandatory)][ValidatePattern('^[0-9A-Fa-f]{40}$')][string]$Thumbprint,
    [Parameter(Mandatory)][string]$SignTool
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'windows-tools.psm1') -Force
$null = Get-HostedVerificationTrustStore
# The caller owns the private-key/trust lifetime. No PFX, password, or secret
# is put on the compiler's signing command line or imported by this callback.
$certificate = Get-Item -LiteralPath "Cert:\CurrentUser\My\$Thumbprint"
try {
    if (-not $certificate.HasPrivateKey -or $certificate.Thumbprint -ne $Thumbprint) {
        throw 'The compiler callback could not select the expected private signing identity'
    }
    $file = (Resolve-Path -LiteralPath $FilePath).Path
    Set-ReleaseSignatures @($file) $certificate $SignTool
    Confirm-Signatures @($file) $certificate $SignTool
} finally { $certificate.Dispose() }
