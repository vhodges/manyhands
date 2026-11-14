## Summary

- Generate a version/checksum-bound Apple Silicon Homebrew cask from the verified macOS archive, without modifying an external tap or bypassing platform security.
- Add a pinned Inno Setup 7.1.1 per-user Windows installer using the verified portable payload, existing signing identity, mandatory signatures/timestamps, opt-in owned PATH integration, and interrupted-upgrade/downgrade protection. Keep preparation read-only, checkpoint at actual install start, and retain the repair floor after possible writes. Use bounded compiler/signing retries and post-copy stale-file pruning only inside owned `bin`/`licenses` trees. Allow redirected profile ancestors while rejecting reparse points at/below the application root; replace quadratic path validation with a sorted prefix index and once-validated embedded tables.
- Add fixed-text native Windows release desktop startup/panic dialogs, retaining stderr/default panic behavior and leaving CLI/debug/non-Windows behavior unchanged. Document the owner-accepted limitation that even recovered panics show the modal dialog and block their reporting thread until dismissal.
- Add a root source-building Linux Nix flake for x86_64 and ARM64: desktop plus a separately built featureless CLI, or CLI alone. Keep its supporting expressions and guide in `packaging/nix/`. Include a desktop graphics wrapper, icons/desktop entry, private-source isolation, and lock-derived disclosure checks; leave Devenv unchanged. Remove the unsupported local Git runtime dependency across Nix, Homebrew and native metadata/guidance.
- Repair shared disclosure harvesting/verification, retaining exact upstream originals and source archives with scoped evidence rather than invented copyright holders. Reject curly-brace attribution templates as well as angle/square placeholders, including `author`/`authors`. Preserve notices in the macOS app as well as the archive root.

## Verification

- [x] Final follow-up helper run passed **264 tests with no skips** in 238.263 seconds, including Ruby/FPC and all ten pinned Inno-engine tests under Wine. The preceding fixture teardown failure and correction remain recorded. All five PowerShell helper suites pass; actual Windows certificate-store APIs remain stubbed/guarded on Linux.
- [x] Both workflows passed actionlint/ShellCheck; all 29 Python files parsed with Python 3.11 grammar; all 79 supplemental lock bindings / 17 exact originals verified. Whitespace checks passed without normalizing pinned originals.
- [x] At `0871a0e0`, Devenv formatting, all-feature check and all-target/all-feature Clippy with warnings denied passed. This follow-up changes no Rust source or dependencies, so those results are historical rather than new runs.
- [x] At `0871a0e0`, the full locked/all-feature Rust rerun with two test threads passed: **4,901 standard tests/doc-tests and 857 SSH cases (5,758 passes)**, with one existing on-request characterization ignored. The initial desktop harness timeouts and successful reruns remain recorded. CLI no-argument smoke also passed then.
- [x] Linux x86_64 and ARM64 flake package/app/check outputs evaluate.
- [x] At `0871a0e0`, x86_64 desktop and CLI packages built and all current-system flake checks passed, including empty-PATH repository creation and Git-/graphics-free CLI closure. Application binaries were not rebuilt for this follow-up; current source-policy/transitive-isolation and desktop-loader checks pass.
- [x] All four target disclosure bundles generated offline in Nix sandboxes; original notice/source hashes, complete-original HTML, scoped evidence and applicable records were independently checked.
- [x] At `0871a0e0`, installed-output audit verified both raw CLIs create/commit repositories with `PATH=""`, neither runtime closure contains Git, and CLI-only has no graphics/fonts. Fresh follow-up disclosure audits verify every notice/source and applicable supplemental original/scope/full-original HTML; complete licence trees remain byte-identical to the audited originals.
- [x] On the earlier pre-review build, the owner confirmed an isolated NixOS/COSMIC Wayland window opened and text rendered correctly, then closed it (normal exit 0; no stderr). That manual window check was not repeated on the review-fix outputs.
- [x] Independent scoped review accepted review findings 4–7 (Windows reporting, attribution templates, lock-derived checks and no local Git requirement), with no actionable findings.
- [x] Installer review's P2 destination-content gap is resolved. All incoming managed-file SHA-256s are checked before pruning using separately embedded/cached expectations; wrong-byte/forged-metadata regressions remain green, and independent re-review accepted the correction.
- [x] Follow-up scoped review accepted the reparse boundary, prefix-node sort/lookup, immutable tables, author placeholders and recovered-panic documentation with no actionable findings. Pinned-engine benchmarks reduced seven equivalent validations of the actual 920-path list from 63.804 seconds to 4.944 seconds; these are Wine validation timings, not native whole-install timings.

## Remaining acceptance and scope

- Native Homebrew style/audit/install/uninstall and Windows signing/install/upgrade/repair/uninstall/dialog acceptance remain; Linux fixtures/Wine and injected-dialog tests are not native acceptance.
- ARM64 executable builds/runtime and future published-tag consumer examples remain. Existing `v0.1.6` predates these changes and must not be repurposed.
- KDE protocol/generated-binding applicability and the objc2 upstream Apple SDK caveat are documented uncertainties, not established violations or claims of legal clearance. The Apple caveat is repeated across 18 dependencies, not 18 distinct restrictions. No new licensing investigation or dependency change is part of this handoff.
- The owner's dated declaration acceptance is limited to six exact package/version/checksum-bound original releases; unknown cases remain fail-closed.
- No feasibility binaries ship. No merge, version/tag change, workflow dispatch, release publication, external tap change, or signing-identity change is included.
- Native Build CI remains a separately authorized **post-merge, on-main** step, not a pre-merge gate.

Current follow-up evidence: `docs/plans/2026-11-01-packaging-review-follow-up-execution.md`. Prior review-fix evidence: `docs/plans/2026-11-01-packaging-review-fixes-execution.md`. Historical channel/build evidence: `docs/plans/2026-10-30-release-workflow-execution.md`.
