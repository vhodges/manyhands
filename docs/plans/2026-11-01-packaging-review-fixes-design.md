# Packaging Review Fixes Design

The owner approved this scoped review-fix pass on 2026-11-01, including
regressions and follow-up commits pushed to `phase-ii-packaging`.

## Installer Safety

Give ISCC an explicit bounded timeout and allow bounded signing retries,
with a larger documented workflow budget leaving time for verification and
cleanup. Keep exact-identity signature verification and RFC 3161 timestamps
mandatory; never add an unsigned/untimestamped fallback.

Keep preparation read-only. Establish the pending-version floor immediately
before actual installation can modify files, and verify with the real Inno
engine that checkpoint failure prevents installation. Preserve the floor
after files may have changed: Inno 7.1.1 does not generally restore overwritten
files during cancellation rollback. Do not clear it merely on cancellation.

Remove stale files only from installer-owned `bin` and `licenses` trees
after successful payload installation, before marking completion. Derive the
incoming managed-file list from the verified ZIP payload, include installer
licence/manifest additions, validate the complete list before deleting
anything, and reject traversal/reparse points. Cleanup failure remains an
incomplete installation requiring same/newer-version repair. Do not use
wholesale pre-install `[InstallDelete]`, touch user state, or redesign the
installer into a transactional update system.

## Remaining Review Fixes

- Windows desktop startup errors and panics get a native, fixed-text error
  dialog; preserve existing stderr behavior and avoid persisting sensitive
  panic payloads. Keep the CLI console behavior unchanged.
- Reject single/double curly-brace attribution placeholders with regressions
  for direct named-attribution and complete-grant validation paths. Do not
  change licence inputs, accepted selections, grants or owner decisions.
- Derive versioned disclosure-check paths from locked package metadata.
- Remove the unsupported runtime Git requirement from Nix, Homebrew and
  generated native metadata/guidance. Keep build/test Git dependencies and
  SSH-server-side Git requirements distinct. Test headless operation without
  a local Git executable and preserve a graphics-free CLI closure.
- Remove the dead installer argument guard and duplicate signing-config
  read, retaining equivalent pre/post-compile payload and timestamp checks.

Use local tests/fixtures, Devenv Rust gates and source-built Nix verification.
Wine/structural tests do not establish native signing/install acceptance.
No merge, tags, publication, workflow dispatch, tap repository or credentials.
