$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Import-Module (Join-Path $PSScriptRoot '../windows-tools.psm1') -Force

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$failures = [Collections.Generic.List[string]]::new()
function Invoke-Test([string]$Name, [scriptblock]$Action) {
    try { & $Action; Write-Output "Passed: $Name" }
    catch { $failures.Add("${Name}: $($_.Exception.Message)") }
}

$python = (Get-Command python -ErrorAction Stop).Source
$fixture = Join-Path $PSScriptRoot 'windows_process_fixture.py'
$temporary = Join-Path ([IO.Path]::GetTempPath()) ('manyhands-process-tests-' + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
    Invoke-Test 'UTC progress uses the information stream, not return values' {
        $records = @(Write-ReleaseProgress 'fixture phase' 6>&1)
        Assert-True ($records.Count -eq 1) 'Expected one progress record'
        Assert-True ($records[0] -is [Management.Automation.InformationRecord]) 'Progress polluted success output'
        Assert-True ($records[0].ToString() -match '^\[\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+Z\] fixture phase$') 'Progress lacks a UTC timestamp'
    }

    Invoke-Test 'default native deadline is 180 seconds' {
        $command = Get-Command Invoke-ReleaseTool
        Assert-True ($command.Parameters.ContainsKey('TimeoutSeconds')) 'Missing native-tool timeout'
        $parameter = @($command.ScriptBlock.Ast.FindAll({
            param($node)
            $node -is [Management.Automation.Language.ParameterAst] -and $node.Name.VariablePath.UserPath -eq 'TimeoutSeconds'
        }, $true))
        Assert-True ($parameter.Count -eq 1 -and $parameter[0].DefaultValue.SafeGetValue() -eq 180) 'Expected a three-minute default deadline'
    }

    Invoke-Test 'argument vectors preserve spaces, quotes, empty strings and Unicode' {
        $arguments = @('space value', '', 'quote"value', '日本語', 'SECRET-argument-sentinel')
        $records = @(Invoke-ReleaseTool $python (@('-B', $fixture, 'arguments') + $arguments) 6>&1)
        $output = @($records | Where-Object { $_ -is [string] })
        Assert-True ($output.Count -eq 1) 'Progress or diagnostics polluted the returned tool output'
        $actual = @($output[0] | ConvertFrom-Json)
        Assert-True ($actual.Count -eq $arguments.Count) 'Arguments were split or dropped'
        for ($index = 0; $index -lt $arguments.Count; $index++) {
            Assert-True ($actual[$index] -ceq $arguments[$index]) 'Argument fidelity changed'
        }
        $progress = @($records | Where-Object { $_ -is [Management.Automation.InformationRecord] })
        Assert-True ($progress.Count -eq 2) 'Missing native start/completion progress'
        Assert-True (($progress -join "`n") -notmatch 'SECRET-argument-sentinel') 'Progress leaked tool arguments'
    }

    Invoke-Test 'start progress reaches callers before the child runs' {
        $gate = Join-Path $temporary 'progress gate'
        $records = @(Invoke-ReleaseTool $python @('-B', $fixture, 'progress', $gate) 6>&1 | ForEach-Object {
            if ($_ -is [Management.Automation.InformationRecord] -and $_.ToString() -match 'Starting native tool:') {
                [IO.File]::WriteAllText($gate, 'start progress delivered')
            }
            $_
        })
        Assert-True ($records -contains 'progress reached caller') 'Native progress was buffered until after child completion'
    }

    Invoke-Test 'stdout and stderr drain concurrently without deadlock' {
        $output = Invoke-ReleaseTool $python @('-B', $fixture, 'streams')
        Assert-True ($output.Contains('O' * 200000)) 'stdout was lost or truncated'
        Assert-True ($output.Contains('E' * 200000)) 'stderr was lost or truncated'
    }

    Invoke-Test 'nonzero exit retains code and both diagnostic streams' {
        $failure = $null
        try { Invoke-ReleaseTool $python @('-B', $fixture, 'failure') | Out-Null }
        catch { $failure = $_.Exception.Message }
        Assert-True ($null -ne $failure) 'Nonzero exit was accepted'
        Assert-True ($failure.Contains('(7)') -and $failure.Contains('stdout failure detail') -and $failure.Contains('stderr failure detail')) 'Failure diagnostics were lost'
    }

    Invoke-Test 'empty output remains a string' {
        $output = Invoke-ReleaseTool $python @('-c', 'pass')
        Assert-True ($output -is [string] -and $output -ceq '') 'Empty tool output changed type'
    }

    Invoke-Test 'native tools inherit the PowerShell working directory' {
        Push-Location -LiteralPath $temporary
        try {
            $output = Invoke-ReleaseTool $python @('-c', 'import os; print(os.getcwd())')
            Assert-True ($output -ceq $PWD.ProviderPath) 'Native working directory changed'
        } finally { Pop-Location }
    }

    Invoke-Test 'invalid deadlines fail before launching the tool' {
        foreach ($seconds in @(0, -1, 3601)) {
            $rejected = $false
            try { Invoke-ReleaseTool $python @('-B', $fixture, 'arguments') -TimeoutSeconds $seconds | Out-Null }
            catch [Management.Automation.ParameterBindingException] { $rejected = $true }
            Assert-True $rejected 'Invalid native-tool deadline was accepted'
        }
    }

    Invoke-Test 'timeout kills a live child tree and allows caller cleanup' {
        Assert-True ((Get-Command Invoke-ReleaseTool).Parameters.ContainsKey('TimeoutSeconds')) 'Missing native-tool timeout'
        $pidFile = Join-Path $temporary 'owned process IDs.json'
        $failure = $null
        $callerCleaned = $false
        $clock = [Diagnostics.Stopwatch]::StartNew()
        try {
            try { Invoke-ReleaseTool $python @('-B', $fixture, 'tree', $pidFile) -TimeoutSeconds 3 | Out-Null }
            catch [TimeoutException] { $failure = $_.Exception.Message }
            finally { $callerCleaned = $true }
            Assert-True ($null -ne $failure -and $failure.Contains('timed out after 3 seconds')) 'Expected a native-tool timeout failure'
            Assert-True ($clock.Elapsed.TotalSeconds -lt 15) 'Timeout did not bound the native invocation'
            Assert-True $callerCleaned 'Caller cleanup did not run after timeout'
            Assert-True (Test-Path -LiteralPath $pidFile) 'Process tree never reached the fixture handshake'
            foreach ($processId in (Get-Content -Raw -LiteralPath $pidFile | ConvertFrom-Json)) {
                $owned = Get-Process -Id $processId -ErrorAction SilentlyContinue
                if ($null -ne $owned) {
                    try { Assert-True ($owned.WaitForExit(3000)) 'Timed-out parent or child is still running' }
                    finally { $owned.Dispose() }
                }
            }
        } finally {
            # Only these test-owned PIDs may need cleanup after a regression.
            if (Test-Path -LiteralPath $pidFile) {
                foreach ($processId in (Get-Content -Raw -LiteralPath $pidFile | ConvertFrom-Json)) {
                    $owned = Get-Process -Id $processId -ErrorAction SilentlyContinue
                    if ($null -ne $owned) {
                        try { if (-not $owned.HasExited) { $owned.Kill($true); $null = $owned.WaitForExit(3000) } }
                        finally { $owned.Dispose() }
                    }
                }
            }
        }
    }

    Invoke-Test 'the deadline also bounds output held open after the tool exits' {
        $pidFile = Join-Path $temporary 'held-pipe child.pid'
        $failure = $null
        $clock = [Diagnostics.Stopwatch]::StartNew()
        try {
            try { Invoke-ReleaseTool $python @('-B', $fixture, 'held-pipe', $pidFile) -TimeoutSeconds 3 | Out-Null }
            catch [TimeoutException] { $failure = $_.Exception.Message }
            Assert-True ($null -ne $failure -and $failure.Contains('tool exited but redirected output did not close')) 'Missing output-drain timeout diagnostic'
            Assert-True ($clock.Elapsed.TotalSeconds -lt 15) 'Inherited output pipes bypassed the deadline'
        } finally {
            # A parent that has already exited cannot be tree-killed through
            # its old Process handle. Dispose of this deliberately detached,
            # test-owned child; production jobs use disposable runners.
            if (Test-Path -LiteralPath $pidFile) {
                $owned = Get-Process -Id ([int](Get-Content -Raw -LiteralPath $pidFile)) -ErrorAction SilentlyContinue
                if ($null -ne $owned) {
                    try { if (-not $owned.HasExited) { $owned.Kill($true); $null = $owned.WaitForExit(3000) } }
                    finally { $owned.Dispose() }
                }
            }
        }
    }
} finally {
    Remove-Item -LiteralPath $temporary -Recurse -Force
}

if ($failures.Count) { throw ($failures -join "`n") }
Write-Output 'Windows native-process tests passed (11 cases).'
