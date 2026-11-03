# Nix Packaging Layout Design

The owner approved moving the supporting root `nix/` directory directly to
`packaging/nix/` on 2026-11-01, keeping `flake.nix` and `flake.lock` at the
repository root. This supersedes the earlier root-level supporting-directory
decision. Do not leave forwarding imports or symlinks at the old location.

Update the root flake imports, relative licence/runtime-manifest paths,
filtered README path and installation command, source-policy tests and
current documentation links. Preserve historical execution evidence and
record the superseding decision rather than rewriting earlier outcomes.

This is a layout-only refactor: preserve public outputs, dependency/tool
pins, source-isolation boundaries, build commands, runtime wrappers and
Devenv. Verify both architectures evaluate, source-policy/loader checks
build, and production derivations/filtered sources differ only by the
approved guide relocation, matching installation path and root README link.
Existing binary
build/runtime results remain historical evidence, not new relocated builds.

Continue in `phase-ii-packaging`; commit and push the follow-up as authorized.
No merge, native workflow, release, tag, external tap or credential changes.
