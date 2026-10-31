Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:SystemLibraries = @(Get-Content -LiteralPath (Join-Path $PSScriptRoot '../../packaging/windows/system-libraries.txt') |
    ForEach-Object { $_.Trim() } | Where-Object { $_ -and -not $_.StartsWith('#') })

function Write-ReleaseProgress {
    param([string]$Message)
    # The information stream remains visible through assignments/Out-Null,
    # without becoming part of parsed native-tool output.
    Write-Host ('[{0}] {1}' -f [DateTime]::UtcNow.ToString('o'), $Message)
}

function Invoke-ReleaseTool {
    [CmdletBinding()]
    param([string]$File, [string[]]$Arguments, [ValidateRange(1, 3600)][int]$TimeoutSeconds = 180)
    $name = [IO.Path]::GetFileName($File)
    $process = [Diagnostics.Process]::new()
    $process.StartInfo.FileName = $File
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.CreateNoWindow = $true
    $process.StartInfo.WorkingDirectory = $PWD.ProviderPath
    $process.StartInfo.RedirectStandardOutput = $true
    $process.StartInfo.RedirectStandardError = $true
    $process.StartInfo.StandardOutputEncoding = [Text.Encoding]::UTF8
    $process.StartInfo.StandardErrorEncoding = [Text.Encoding]::UTF8
    foreach ($argument in $Arguments) { $process.StartInfo.ArgumentList.Add($argument) }
    Write-ReleaseProgress "Starting native tool: $name (timeout ${TimeoutSeconds}s)"
    $clock = [Diagnostics.Stopwatch]::StartNew()
    try {
        if (-not $process.Start()) { throw "Could not start native release tool: $name" }
        # Drain both pipes before waiting; sequential reads deadlock on full
        # stderr/stdout. The same deadline also bounds inherited open pipes.
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $completed = $process.WaitForExit([int][Math]::Max(0, $TimeoutSeconds * 1000 - $clock.ElapsedMilliseconds))
        if ($completed) {
            $remaining = [int][Math]::Max(0, $TimeoutSeconds * 1000 - $clock.ElapsedMilliseconds)
            $completed = [Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($stdout, $stderr), $remaining)
        }
        if (-not $completed) {
            $terminationError = ''
            try {
                if (-not $process.HasExited) {
                    $process.Kill($true)
                    if (-not $process.WaitForExit(5000)) { $terminationError = '; process did not exit after termination' }
                } else {
                    $terminationError = '; tool exited but redirected output did not close'
                }
            } catch { $terminationError = '; could not terminate the native process tree' }
            throw [TimeoutException]::new("Native release tool timed out after $TimeoutSeconds seconds: $name$terminationError")
        }
        $output = ((@($stdout.Result, $stderr.Result) | Where-Object { $_ }) -join "`n").TrimEnd([char[]]"`r`n")
        if ($process.ExitCode -ne 0) {
            throw "Native release tool failed ($($process.ExitCode)): $name`n$output"
        }
        Write-ReleaseProgress ('Completed native tool: {0} ({1:F1}s)' -f $name, $clock.Elapsed.TotalSeconds)
        return $output
    } catch {
        Write-ReleaseProgress ('Failed native tool: {0} ({1:F1}s)' -f $name, $clock.Elapsed.TotalSeconds)
        throw
    } finally {
        $process.Dispose()
    }
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

function Get-SignToolTimestampUrl {
    param([string]$Url)
    # Use Microsoft's documented RFC 3161 endpoint; native verification
    # authenticates the signed timestamp token despite HTTP transport.
    $endpoint = $null
    if (-not [Uri]::TryCreate($Url, [UriKind]::Absolute, [ref]$endpoint) -or
        $endpoint.AbsoluteUri -cne 'http://timestamp.digicert.com/') {
        throw 'Timestamp endpoint must be the reviewed SignTool RFC 3161 URL: http://timestamp.digicert.com/'
    }
    return $endpoint.AbsoluteUri
}

function Get-VerificationTrustStore {
    param([bool]$Windows, [string]$Actions, [string]$RunnerEnvironment, [bool]$Administrator)
    if (-not $Windows -or $Actions -cne 'true' -or $RunnerEnvironment -cne 'github-hosted' -or -not $Administrator) {
        throw 'Temporary verification trust requires an elevated GitHub-hosted Windows runner'
    }
    return 'Cert:\LocalMachine\Root'
}

function Get-HostedVerificationTrustStore {
    $administrator = $false
    if ($IsWindows) {
        $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
        try {
            $principal = [Security.Principal.WindowsPrincipal]::new($identity)
            $administrator = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
        } finally { $identity.Dispose() }
    }
    return Get-VerificationTrustStore -Windows $IsWindows -Actions $env:GITHUB_ACTIONS `
        -RunnerEnvironment $env:RUNNER_ENVIRONMENT -Administrator $administrator
}

function Add-VerificationTrust {
    param([string]$CertificatePath, [string]$Thumbprint, [ref]$OwnedPath)
    $store = Get-HostedVerificationTrustStore
    if ($Thumbprint -notmatch '\A[0-9A-Fa-f]{40}\z') { throw 'Invalid verification trust thumbprint' }
    if ($null -eq $OwnedPath -or $null -ne $OwnedPath.Value) { throw 'Expected an empty verification trust ownership slot' }
    if ([IO.Path]::GetExtension($CertificatePath) -ine '.cer') { throw 'Verification trust requires a public .cer file' }
    $public = [Security.Cryptography.X509Certificates.X509Certificate2]::new($CertificatePath)
    try {
        if ($public.HasPrivateKey -or $public.Thumbprint -ne $Thumbprint) {
            throw 'Verification trust public certificate does not match the expected identity'
        }
    } finally { $public.Dispose() }
    $path = "$store\$($Thumbprint.ToUpperInvariant())"
    if (Test-Path -LiteralPath $path) {
        Write-ReleaseProgress 'Verification trust already present; existing certificate will be preserved'
        return
    }
    # Record ownership before import so a partial import is cleaned up too.
    $OwnedPath.Value = $path
    Write-ReleaseProgress 'Starting temporary verification trust import into LocalMachine/Root'
    $imported = @(Import-Certificate -FilePath $CertificatePath -CertStoreLocation $store -Confirm:$false -ErrorAction Stop)
    if ($imported.Count -ne 1 -or $imported[0].Thumbprint -ne $Thumbprint) {
        throw 'Imported verification trust identity changed'
    }
    if (-not (Test-Path -LiteralPath $path)) { throw 'Verification trust was not found after import' }
    Write-ReleaseProgress 'Completed temporary verification trust import into LocalMachine/Root'
}

function Remove-VerificationTrust {
    param([string]$OwnedPath)
    if ([string]::IsNullOrEmpty($OwnedPath)) { return }
    if ($OwnedPath -notmatch '\ACert:\\LocalMachine\\Root\\[0-9A-Fa-f]{40}\z') {
        throw 'Refusing to remove anything except an exact owned machine-root certificate'
    }
    if (Test-Path -LiteralPath $OwnedPath) {
        Write-ReleaseProgress 'Removing owned temporary machine-root verification trust'
        Remove-Item -LiteralPath $OwnedPath -Force -ErrorAction Stop
        if (Test-Path -LiteralPath $OwnedPath) { throw 'Owned verification trust is still present after removal' }
        Write-ReleaseProgress 'Removed owned temporary machine-root verification trust'
    }
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

Export-ModuleMember -Function Write-ReleaseProgress, Invoke-ReleaseTool, Find-SignTool, Get-SignToolTimestampUrl, Get-VerificationTrustStore, Get-HostedVerificationTrustStore, Add-VerificationTrust, Remove-VerificationTrust, ConvertFrom-DumpbinDependencies, Assert-PeArchitecture, Assert-SystemDependency, Assert-SystemDependencies, Get-WindowsRuntimeReport
