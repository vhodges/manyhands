## Summary

- Generate a version/checksum-bound Apple Silicon Homebrew cask from the verified macOS archive, without modifying an external tap or bypassing platform security.
- Add a pinned Inno Setup 7.1.1 per-user Windows installer using the verified portable payload, existing signing identity, mandatory signatures/timestamps, opt-in owned PATH integration, and interrupted-upgrade/downgrade protection.
- Add a root source-building Linux Nix flake for x86_64 and ARM64: desktop plus a separately built featureless CLI, or CLI alone. Include runtime wrappers, icons/desktop entry, private-source isolation, and package checks; leave Devenv unchanged.
- Repair shared disclosure harvesting/verification, retaining exact upstream originals and source archives with scoped evidence rather than invented copyright holders. Preserve notices in the macOS app as well as the archive root.

## Verification

- [x] 213 portable packaging tests passed, including Ruby syntax and compiled shared Pascal policy tests; no skips in the full run.
- [x] Both workflows passed actionlint/ShellCheck; all 26 release Python files parsed with Python 3.11 grammar; whitespace checks passed without normalizing pinned originals.
- [x] Devenv Rust formatting, all-feature check, and all-target/all-feature Clippy with warnings denied passed. Rust test targets and 15 doc-tests were covered across the original run and scoped continuations; the original broad run's capture-directory setup failure remains recorded rather than relabelled a clean single run.
- [x] Linux x86_64 and ARM64 flake package/app/check outputs evaluate.
- [x] Current x86_64 CLI package, installed help, graphics-free closure, source-policy and desktop-library-loader checks passed. Isolated CLI help created no user state.
- [x] All four target disclosure bundles generated offline in Nix sandboxes; original notice/source hashes, complete-original HTML, scoped evidence and applicable records were independently checked.
- [x] Current x86_64 desktop package built; all current-system flake checks passed. Installed full-package binaries/disclosures/source hashes were checked. The owner confirmed the isolated NixOS/COSMIC Wayland window opened and text rendered correctly, then closed it (normal exit 0; no stderr).
- [x] Scoped Homebrew/Windows/Nix/disclosure reviews and final integration review completed; no remaining P1/P2 findings in those reviewed boundaries.

## Remaining acceptance and scope

- Native Homebrew style/audit/install/uninstall and Windows signing/install/upgrade/repair/uninstall acceptance remain; Linux fixtures/Wine checks are not native acceptance.
- ARM64 executable builds/runtime and future published-tag consumer examples remain. Existing `v0.1.6` predates these changes and must not be repurposed.
- KDE protocol/generated-binding applicability and the objc2 upstream Apple SDK caveat are documented uncertainties, not established violations or claims of legal clearance. The Apple caveat is repeated across 18 dependencies, not 18 distinct restrictions. No new licensing investigation or dependency change is part of this handoff.
- The owner's dated declaration acceptance is limited to six exact package/version/checksum-bound original releases; unknown cases remain fail-closed.
- No feasibility binaries ship. No merge, version/tag change, workflow dispatch, release publication, external tap change, or signing-identity change is included.
- Native Build CI remains a separately authorized **post-merge, on-main** step, not a pre-merge gate.

Detailed execution evidence and limitations: `docs/plans/2026-10-30-release-workflow-execution.md`.
