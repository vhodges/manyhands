# Windows Signing Diagnostics Design

## Approved Scope

The owner approved progress logging, a three-minute native-tool timeout,
and a ten-minute workflow-step backstop after Release v0.1.4 was cancelled.
This is diagnostic containment, not a claim to have fixed the unknown stall.
No version/tag, publisher identity, secrets, timestamp policy, verification
requirements, workflow matrix, or release assets change. Integration and a
new native run need separate authorization.

## Evidence

[Release run 41233300191](https://github.com/vhodges/manyhands/actions/runs/41233300191)
used source `25a8174d` and tag `v0.1.4`. Both Linux jobs and macOS succeeded.
Windows remained in its signing step from 19:24:20 to 20:26:52 UTC before the
owner cancelled it. The supplied log contained only the step invocation and
cancellation; cleanup named an orphan `pwsh`, which does not identify the
stalled operation. Native-tool output was buffered and calls had no timeout.

## Chosen Design

- Add UTC progress messages before and after PFX loading/import, public
  certificate export, the runtime audit, each executable's signing and both
  verification APIs, metadata finalization, smoke testing, and cleanup.
  Diagnostics use PowerShell's information stream, not success output, so
  captured tool results and `Out-Null` do not swallow progress or corrupt data.
  Never log private certificate material, passwords, environment dumps, or
  complete argument lists.
- Replace the tools module's unbounded native invocation with .NET
  `ProcessStartInfo.ArgumentList`, without a shell. Drain stdout/stderr
  asynchronously to avoid pipe deadlocks. One 180-second deadline covers
  process execution and output collection. On expiry, kill the process tree
  if still running, allow a bounded five-second reap, and throw a timeout
  through the signing script's existing `finally`. Retain nonzero exit-code
  failures and captured diagnostics; always dispose the process handle.
- Set `timeout-minutes: 10` on both Windows signing and extracted-verification
  steps. This bounds in-process certificate cmdlets as well as cumulative
  work. A workflow hard cancellation cannot guarantee PowerShell `finally`;
  those jobs must remain on disposable hosted runners. Do not upload private
  diagnostic files or weaken cleanup/verification to obtain progress.

Only adding logs would still permit an hour-long stall. Only adding a
workflow limit would stop the run without identifying the boundary and could
skip normal cleanup. The combined approach is the approved narrow solution.

## Verification

Use real short-lived Python children from PowerShell on Linux to test
argument fidelity, simultaneous large stdout/stderr, nonzero exits, a hung
parent/child tree, timeout propagation into caller cleanup, and progress
stream separation. Check default/invalid deadlines. Exercise verification
progress with stubbed Windows APIs without touching certificate stores.
Check the two workflow step limits, run the affected Python tests, parse all
modified PowerShell, lint workflows, and check whitespace. Native Windows
signing and actual store cleanup remain pending a separately approved run.
All three PowerShell suites are registered in the Windows release-helper test
step before signing secrets are introduced, so the next authorized run also
exercises the process tests on Windows.
