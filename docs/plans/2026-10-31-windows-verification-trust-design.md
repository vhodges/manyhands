# Windows Verification Trust Correction

## Approved Scope And Evidence

The owner approved temporary `LocalMachine\Root` verification trust only on
an elevated, disposable GitHub-hosted Windows runner. The persistent signing
identity remains in `CurrentUser\My`; publisher identity, secrets, SHA-256,
RFC 3161 timestamps, both signature verifiers, and existing deadlines remain.
No consumer certificate installation or public publisher trust is introduced.
Commit/push/merge, version/tag changes, and another run need separate approval.

[Release v0.1.5 run 41243103652](https://github.com/vhodges/manyhands/actions/runs/41243103652)
used `b411e208`. The supplied log shows successful PFX loading/store import,
runtime auditing, and both signing/timestamp calls (0.6 and 0.2 seconds).
It then stopped inside `Import-Certificate` into `CurrentUser\Root` until
the ten-minute step limit. The call is proven; an unseen Windows root-trust
confirmation is the leading explanation, not a directly observed dialog.

## Design

1. Separate pure runner-policy selection from actual context discovery.
   Require Windows, `GITHUB_ACTIONS=true`, `RUNNER_ENVIRONMENT=github-hosted`,
   and an elevated administrator token; otherwise fail before private-key
   import. Environment checks prevent accidental use, not malicious code
   already running as administrator. There is no current-user fallback.
2. Encapsulate temporary trust import/removal in the existing tools module.
   Recheck the real context at import. Validate the public certificate against
   the expected thumbprint before mutation. Use only `Cert:\LocalMachine\Root`.
   Preserve an existing matching certificate; record the exact owned path by
   reference before import so partial failures are cleaned. Validate the
   imported identity and store presence. Cleanup accepts only an exact owned
   machine-root thumbprint path, removes that certificate, and verifies removal.
3. Both Sign and Verify modes share this policy and lifecycle. Cleanup failure
   still fails the job and does not prevent subsequent private-key/file cleanup.
   Keep progress and the 180-second native/ten-minute step deadlines unchanged.
4. Register portable policy/ownership tests in the Windows helper step. Also
   include a Windows-only real import/preserve/remove fixture using an ephemeral
   in-memory test certificate, never the publisher identity. Gate it to the
   same hosted-runner/admin context, before signing secrets. Bound the helper
   step to ten minutes because its fixture now exercises certificate cmdlets.

## Verification And Alternatives

Portable PowerShell tests cover allowed/rejected contexts, preservation,
partial-import failure, wrong public identity, exact cleanup, and removal
failure. Existing signature/deadline tests must remain green. Native fixture
execution, actual signing verification, and real-store cleanup remain pending
the next authorized Windows run; Linux tests use stubbed store APIs only.

Keeping current-user roots retains the suspected confirmation problem;
`-Confirm:$false` does not establish suppression of Windows' own root-store UI.
A custom trust verifier avoids store mutation but is a larger change to the
Authenticode verification contract. The owner selected the narrower CI fix.

References: [Import-Certificate](https://learn.microsoft.com/en-us/powershell/module/pki/import-certificate),
[GitHub Windows runner privileges](https://docs.github.com/en/actions/reference/runners/github-hosted-runners#administrative-privileges),
[PowerShell current-user root warning report](https://github.com/PowerShell/PowerShell/issues/7748).
