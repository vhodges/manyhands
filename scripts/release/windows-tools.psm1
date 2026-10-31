Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:SystemLibraries = @(Get-Content -LiteralPath (Join-Path $PSScriptRoot '../../packaging/windows/system-libraries.txt') |
    ForEach-Object { $_.Trim() } | Where-Object { $_ -and -not $_.StartsWith('#') })

function Invoke-ReleaseTool {
    param([string]$File, [string[]]$Arguments)
    $output = & $File @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Native release tool failed ($LASTEXITCODE): $File`n$($output -join "`n")"
    }
    return ($output -join "`n")
}

function Find-VisualStudio {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) { throw 'Visual Studio discovery tool is unavailable' }
    $root = Invoke-ReleaseTool $vswhere @('-latest', '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath')
    if (-not $root -or -not (Test-Path -LiteralPath $root -PathType Container)) { throw 'MSVC installation is unavailable' }
    return $root.Trim()
}

function Find-Dumpbin {
    $root = Find-VisualStudio
    $versions = Get-ChildItem -LiteralPath (Join-Path $root 'VC/Tools/MSVC') -Directory |
        Where-Object Name -Match '^\d+\.\d+\.\d+$' | Sort-Object { [Version]$_.Name } -Descending
    foreach ($version in $versions) {
        $tool = Join-Path $version.FullName 'bin/Hostx64/x64/dumpbin.exe'
        if (Test-Path -LiteralPath $tool -PathType Leaf) { return $tool }
    }
    throw 'MSVC x64 dumpbin.exe is unavailable'
}

function Find-SignTool {
    $sdk = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
    $versions = Get-ChildItem -LiteralPath $sdk -Directory |
        Where-Object Name -Match '^\d+\.\d+\.\d+\.\d+$' | Sort-Object { [Version]$_.Name } -Descending
    foreach ($version in $versions) {
        $tool = Join-Path $version.FullName 'x64/signtool.exe'
        if (Test-Path -LiteralPath $tool -PathType Leaf) { return $tool }
    }
    throw 'Windows SDK x64 SignTool is unavailable'
}

function ConvertFrom-DumpbinDependencies {
    param([string]$Output)
    return @([regex]::Matches($Output, '(?im)^\s*([A-Z0-9_-][A-Z0-9_.-]*\.[A-Z0-9]+)\s*$') |
        ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique)
}

function Assert-PeArchitecture {
    param([string]$Headers)
    if ($Headers -notmatch '(?im)^\s*8664 machine \(x64\)\s*$') { throw 'PE architecture is not x86_64' }
}

function Assert-SystemDependency {
    param([string]$Name, [string]$SystemDirectory = [Environment]::GetFolderPath('System'))
    if ($Name -notmatch '^[A-Za-z0-9_-][A-Za-z0-9_.-]*\.[A-Za-z0-9]+$') { throw "Invalid PE dependency name: $Name" }
    if ($Name -match '^(vcruntime|msvcp|msvcr|concrt|ucrtbase|api-ms-win-crt-)') {
        throw "Dynamic MSVC CRT dependency $Name; build all release objects with -C target-feature=+crt-static"
    }
    if ($script:SystemLibraries -notcontains $Name) { throw "Unreviewed Windows OS/API-set dependency: $Name" }
    if ($Name -match '^(api-ms-win-|ext-ms-win-)') { return }
    if (-not (Test-Path -LiteralPath (Join-Path $SystemDirectory $Name) -PathType Leaf)) {
        throw "Unreviewed non-system Windows dependency: $Name"
    }
}

function Assert-SystemDependencies {
    param([string[]]$Names, [string]$SystemDirectory = [Environment]::GetFolderPath('System'), [string]$BinaryName = 'executable')
    $failures = [Collections.Generic.List[string]]::new()
    foreach ($name in $Names) {
        try { Assert-SystemDependency -Name $name -SystemDirectory $SystemDirectory }
        catch { $failures.Add($_.Exception.Message) }
    }
    if ($failures.Count) {
        throw "Windows runtime audit failed for ${BinaryName}:`n$($failures -join "`n")"
    }
}

function Get-WindowsRuntimeReport {
    param([string]$Staging)
    $dumpbin = Find-Dumpbin
    $binaries = [ordered]@{}
    foreach ($name in @('manyhands.exe', 'manyhands-cli.exe')) {
        $binary = Join-Path $Staging "bin/$name"
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Missing Windows executable: $name" }
        Assert-PeArchitecture (Invoke-ReleaseTool $dumpbin @('/HEADERS', $binary))
        $dependencies = @(ConvertFrom-DumpbinDependencies (Invoke-ReleaseTool $dumpbin @('/DEPENDENTS', $binary)))
        if ($dependencies.Count -eq 0) { throw "No imports found in Windows executable: $name" }
        Assert-SystemDependencies -Names $dependencies -BinaryName $name
        $binaries[$name] = $dependencies
    }
    $extraLibraries = @(Get-ChildItem -LiteralPath (Join-Path $Staging 'bin') -Filter '*.dll' -File)
    if ($extraLibraries.Count) { throw 'Unexpected bundled DLLs require an explicit redistribution policy' }
    return [ordered]@{ system = 'windows'; architecture = 'x86_64'; crt = 'static'; binaries = $binaries }
}

Export-ModuleMember -Function Invoke-ReleaseTool, Find-SignTool, ConvertFrom-DumpbinDependencies, Assert-PeArchitecture, Assert-SystemDependency, Assert-SystemDependencies, Get-WindowsRuntimeReport
