# Windows release signing

The release uses a persistent self-signed Authenticode code-signing
certificate supplied by the maintainer. Both executables are signed and
RFC 3161-timestamped with SHA-256. `signing.json` contains the timestamp
endpoint. There is no unsigned fallback and no per-run certificate creation.

The reviewed RFC 3161 endpoint is **`http://timestamp.digicert.com/`**, as
shown in [Microsoft's SignTool examples](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool#examples).
The v0.1.3 native trial using Windows SDK 10.0.26100.0 rejected the previous
HTTPS URL with `SignTool Error: Invalid Timestamp URL`. The helper validates
and normalizes only this reviewed HTTP endpoint; changing the provider,
transport, or URL path requires an explicit policy update. The root slash
added by URI normalization is expected.

HTTP transports the timestamp request/response, but the RFC 3161 timestamp
token itself is cryptographically signed. Both `/fd SHA256` and `/td SHA256`
remain required, as do native verification of the signatures, expected
publisher identity, and presence of a verified timestamp. There is no
untimestamped fallback. This correction does not change the persistent
publisher certificate or signing secrets.

Self-signing preserves a stable publisher identity but does **not** make
Windows trust it or eliminate SmartScreen warnings. The package contains
only the public `.cer`, its fingerprints, and an explanation. It never
installs a trust certificate on a user's machine.

## One-time maintainer provisioning

Run these commands in **PowerShell 7 on your own Windows machine**, with
GitHub CLI authenticated to the repository. They create a real persistent
signing identity; choose the subject you want users to see. Keep the PFX
and its password private and retain a protected backup for future releases.

```powershell
$certificate = New-SelfSignedCertificate `
    -Type CodeSigningCert `
    -Subject 'CN=Vince Hodges (Manyhands)' `
    -FriendlyName 'Manyhands release signing' `
    -CertStoreLocation 'Cert:\CurrentUser\My' `
    -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
    -KeyExportPolicy Exportable `
    -NotAfter (Get-Date).AddYears(3)
$password = Read-Host 'Choose a PFX password' -AsSecureString
$pfxPath = Join-Path $HOME 'manyhands-signing.pfx'
Export-PfxCertificate -Cert $certificate -FilePath $pfxPath -Password $password | Out-Null
[Convert]::ToBase64String([IO.File]::ReadAllBytes($pfxPath)) | `
    gh secret set WINDOWS_SIGNING_PFX_BASE64 --repo vhodges/manyhands
gh secret set WINDOWS_SIGNING_PFX_PASSWORD --repo vhodges/manyhands
```

Enter the same PFX password at the final command's secret prompt. Do not
paste the PFX, base64 text, or password into repository files, release
notes, logs, or chat. Base64 is an encoding, not encryption.

Before expiry, replace both secrets with a renewed/new valid identity.
If the key changes, the public fingerprint changes; disclose that in the
release notes. Timestamped old signatures are verified against their
signing time; the verification path does not simply reject an expired
certificate that has an otherwise valid timestamped signature.

## Native build requirements

Use the release's pinned Rust and build both binaries with:

```powershell
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --locked --release --no-default-features --bin manyhands-cli --target x86_64-pc-windows-msvc
cargo build --locked --release --no-default-features --features desktop --bin manyhands --target x86_64-pc-windows-msvc
```

The existing Windows dependency configuration vendors OpenSSL. MSVC
builds require native Windows Perl with `IPC::Cmd`, not Git's MSYS Perl.
The native workflow must verify that prerequisite before building. The
release import audit uses MSVC `dumpbin`, checks both x86_64 headers,
normal/delay imports, and rejects dynamic CRT or non-system DLL imports.
`system-libraries.txt` is the exact reviewed OS/API-set baseline. Ordinary
modules must also exist in the native OS System directory; finding an
arbitrary vendor DLL there does not make it an approved OS dependency.
Unreviewed DLLs are not silently collected from the runner's PATH.
Audit failures list every rejected import for the executable being checked,
so additional missing contracts can be reviewed together. The DPI contract
`api-ms-win-shcore-scaling-l1-1-1.dll` is an explicitly reviewed Windows
Shcore forwarding alias, documented by Microsoft for `GetDpiForMonitor`.
The desktop also imports OS-provided GDI+ (`gdiplus.dll`), ICU common C APIs
(`icuuc.dll`), and UI Automation (`uiautomationcore.dll`). These exact names
are reviewed with Microsoft documentation linked in the policy file; their
presence in the native OS System directory is still required. System ICU
was introduced in Windows 10 version 1703. The package records that runtime
requirement and does not collect replacement DLLs from developer-tool PATHs.

## Signing and extracted verification

With the two secrets available as environment variables in the signing
step, on a disposable Windows runner:

```powershell
pwsh -NoProfile -File scripts/release/windows-sign.ps1 -Mode Sign -Staging $staging
```

The helper validates the certificate's private key, code-signing EKU,
validity dates, and self-signed identity, imports it into the runner's
current-user My store, signs and timestamps both executables, and verifies
the exact expected identity with SignTool and Authenticode APIs. It adds
temporary public trust only for runner verification. Owned private files,
the imported private key, and newly added trust are removed in `finally`;
existing unrelated certificates are preserved. Cleanup failure fails the job.

After archive creation and extraction to a fresh directory, repeat:

```powershell
pwsh -NoProfile -File scripts/release/windows-sign.ps1 -Mode Verify -Staging $extractedPayload
```

Verification uses the public certificate and recorded fingerprints; it
does not need signing secrets. It repeats import/signature/timestamp
checks and runs the packaged CLI from an isolated directory with only
Windows system directories on its PATH. Linux-hosted parser/metadata
tests do not establish that these native signing operations succeeded.
