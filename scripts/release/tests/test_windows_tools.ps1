$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Import-Module (Join-Path $PSScriptRoot '../windows-tools.psm1') -Force

function Assert-Throws([scriptblock]$Action) {
    $thrown = $false
    try { & $Action } catch { $thrown = $true }
    if (-not $thrown) { throw 'Expected an error' }
}

$output = @'
Dump of file C:\a path\manyhands.exe
  Image has the following dependencies:
    KERNEL32.dll
    WINSPOOL.DRV
    api-ms-win-core-synch-l1-2-0.dll
    KERNEL32.dll
  Summary
    1000 .data
'@
$dependencies = @(ConvertFrom-DumpbinDependencies $output)
if ($dependencies.Count -ne 3 -or $dependencies -notcontains 'KERNEL32.dll' -or $dependencies -notcontains 'WINSPOOL.DRV') { throw 'Dependency parsing failed' }
Assert-PeArchitecture '8664 machine (x64)'
Assert-Throws { Assert-PeArchitecture '14C machine (x86)' }

$temporary = Join-Path ([IO.Path]::GetTempPath()) ('manyhands-windows-policy-' + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
    [IO.File]::WriteAllText((Join-Path $temporary 'KERNEL32.dll'), 'OS fixture')
    Assert-SystemDependency -Name 'KERNEL32.dll' -SystemDirectory $temporary
    Assert-SystemDependency -Name 'api-ms-win-core-synch-l1-2-0.dll' -SystemDirectory $temporary
    Assert-SystemDependency -Name 'api-ms-win-shcore-scaling-l1-1-1.dll' -SystemDirectory $temporary
    $desktopLibraries = @('gdiplus.dll', 'icuuc.dll', 'uiautomationcore.dll')
    foreach ($library in $desktopLibraries) {
        [IO.File]::WriteAllText((Join-Path $temporary $library), 'Documented Windows system component fixture')
    }
    Assert-SystemDependencies -Names $desktopLibraries -SystemDirectory $temporary -BinaryName 'manyhands.exe'
    foreach ($library in $desktopLibraries) {
        Remove-Item -LiteralPath (Join-Path $temporary $library)
        Assert-Throws { Assert-SystemDependency -Name $library -SystemDirectory $temporary }
    }
    Assert-Throws { Assert-SystemDependency -Name 'unknown.dll' -SystemDirectory $temporary }
    [IO.File]::WriteAllText((Join-Path $temporary 'vendor.dll'), 'Runner-installed third-party fixture')
    Assert-Throws { Assert-SystemDependency -Name 'vendor.dll' -SystemDirectory $temporary }
    Assert-Throws { Assert-SystemDependency -Name 'api-ms-win-unreviewed-l1-1-0.dll' -SystemDirectory $temporary }
    Assert-Throws { Assert-SystemDependency -Name 'vcruntime140.dll' -SystemDirectory $temporary }
    Assert-Throws { Assert-SystemDependency -Name 'api-ms-win-crt-runtime-l1-1-0.dll' -SystemDirectory $temporary }
    Assert-Throws { Assert-SystemDependency -Name '../KERNEL32.dll' -SystemDirectory $temporary }
    $auditFailure = $null
    try {
        Assert-SystemDependencies -Names @('vendor.dll', 'vcruntime140.dll') -SystemDirectory $temporary -BinaryName 'manyhands.exe'
    } catch {
        $auditFailure = $_.Exception.Message
    }
    if (-not $auditFailure -or -not $auditFailure.Contains('vendor.dll') -or
        -not $auditFailure.Contains('vcruntime140.dll') -or -not $auditFailure.Contains('manyhands.exe')) {
        throw 'Dependency audit must report every rejected import and its executable'
    }
} finally {
    Remove-Item -LiteralPath $temporary -Recurse -Force
}
Write-Output 'Windows import-policy tests passed.'
