[CmdletBinding()]
param(
    [ValidateSet('Provision', 'Build')][string]$Mode = 'Build',
    [string]$Staging,
    [string]$Archive,
    [string]$Compiler,
    [string]$OutputDirectory,
    [ValidatePattern('^[0-9A-Fa-f]{40}$')][string]$Thumbprint,
    [string]$SignTool
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'windows-tools.psm1') -Force

function Get-InnoCompilerArguments {
    param([string]$Payload, [string]$Output, [string]$Version, [string]$NumericVersion,
        [string]$Identity, [string]$Signer, [string]$PowerShell, [string]$Callback, [string]$Definition)
    # $q/$f are Inno's documented quoting/file substitutions. No password,
    # PFX, shell evaluation, /NS unsigned mode, or generic $p callback is used.
    $command = '$q' + $PowerShell + '$q -NoLogo -NoProfile -File $q' + $Callback +
        '$q -FilePath $f -Thumbprint ' + $Identity + ' -SignTool $q' + $Signer + '$q'
    return @('/Q', '/NI', "/DPayloadDir=$Payload", "/DAppVersion=$Version", "/DNumericVersion=$NumericVersion",
        "/O$Output", "/Smanyhands=$command", $Definition)
}

function Assert-InnoCompiler {
    param([string]$Path, [string]$ExpectedVersion)
    $version = Invoke-ReleaseTool $Path @('--version')
    if ($version.Trim() -cne $ExpectedVersion) { throw 'Wrong Inno compiler version' }
}

if (-not $IsWindows) { throw 'Inno compiler provisioning/building requires Windows' }
$null = Get-HostedVerificationTrustStore
$config = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot '../../packaging/windows/inno-setup.json') | ConvertFrom-Json
if ($Mode -eq 'Provision') {
    if (-not $env:RUNNER_TEMP -or -not $env:GITHUB_ENV) { throw 'Compiler provisioning requires runner temporary/environment paths' }
    $owned = Join-Path $env:RUNNER_TEMP ('manyhands-inno-' + [Guid]::NewGuid())
    $null = New-Item -ItemType Directory -Path $owned
    $download = Join-Path $owned 'compiler-installer.exe'
    try {
        Invoke-WebRequest -Uri $config.url -OutFile $download -TimeoutSec 120
        if ((Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash -ine $config.sha256) {
            throw 'Pinned Inno Setup download checksum mismatch'
        }
        $destination = Join-Path $owned 'compiler'
        Invoke-ReleaseTool $download @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', '/CURRENTUSER', "/DIR=$destination") | Out-Null
        $Compiler = Join-Path $destination 'ISCC.exe'
        if (-not (Test-Path -LiteralPath $Compiler -PathType Leaf)) { throw 'Pinned Inno compiler was not installed' }
        Assert-InnoCompiler $Compiler $config.version
        "INNO_COMPILER=$Compiler" | Add-Content -LiteralPath $env:GITHUB_ENV -Encoding utf8
    } finally {
        if (Test-Path -LiteralPath $download) { Remove-Item -LiteralPath $download -Force }
    }
    return
}

if (-not $Staging -or -not $Archive -or -not $Compiler -or -not $OutputDirectory -or -not $Thumbprint -or -not $SignTool) {
    throw 'Installer build requires staging, pinned compiler, output, identity, and SignTool'
}
Assert-InnoCompiler $Compiler $config.version
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Installer compiler output must be a fresh directory' }
$null = New-Item -ItemType Directory -Path $OutputDirectory
$output = (Resolve-Path -LiteralPath $OutputDirectory).Path
$parametersPath = Join-Path $output 'parameters.json'
$python = (Get-Command python -ErrorAction Stop).Source
Invoke-ReleaseTool $python @('-B', (Join-Path $PSScriptRoot 'installer.py'), 'parameters',
    '--staging', $Staging, '--archive', $Archive, '--output', $parametersPath) | Out-Null
$parameters = Get-Content -Raw -LiteralPath $parametersPath | ConvertFrom-Json
$arguments = Get-InnoCompilerArguments -Payload (Resolve-Path -LiteralPath $Staging).Path -Output $output `
    -Version $parameters.version -NumericVersion $parameters.numeric_version -Identity $Thumbprint -Signer $SignTool `
    -PowerShell (Join-Path $PSHOME 'pwsh.exe') -Callback (Join-Path $PSScriptRoot 'windows-sign-file.ps1') `
    -Definition (Join-Path $PSScriptRoot '../../packaging/windows/manyhands.iss')
Invoke-ReleaseTool $Compiler $arguments | Out-Null
$installer = Join-Path $output $parameters.name
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) { throw 'Compiler did not produce the expected installer' }
return $installer
