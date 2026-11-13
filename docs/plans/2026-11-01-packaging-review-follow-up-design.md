# Packaging Review Follow-Up Design

## Approval And Boundaries

The owner approved fixing the four follow-up findings against `0871a0e0` on
`phase-ii-packaging`. For recovered panics, the owner explicitly selected
**Document limitation**, not a Rust behavior change. Continue in the existing
worktree. No merge, release, workflow dispatch, credentials, version/tag,
signing identity, licence-original or owner-approval changes are included.

## Installer Reparse Boundary

Check the normalized application root and descendants, not directories above
the root. Reject a caller outside that boundary. Preserve rejection of a
reparse point at the application root, in owned trees, or along an expected
file's parent path. A profile/AppData junction above a regular application
directory is allowed, including on fresh installation. Keep fresh checks at
preparation, install start, and mutation boundaries; this is not a promise of
transactional protection against concurrent same-user filesystem changes.

Actual pinned-engine tests must distinguish ordinary directories, junctions
above the application, a junction at the application root, and junctions
below it. Cover an actual successful fresh/upgrade path through the redirected
ancestor, not only a helper call.

## Bounded Path Validation

The pinned Inno 7.1.1/Wine baseline is 13.877 seconds for one 1,000-entry
validation; the actual 920-path Windows manifest's seven equivalent validation
calls take 63.804 seconds. Evidence is in
`/tmp/opencode/manyhands-packaging-feedback-0871a0e0/results.json`.

Replace pairwise path splitting/comparison with a sorted prefix-node index.
Validate each file path once, emit each directory prefix and final file node,
then merge-sort nodes case-insensitively by their full prefix. Adjacent equal
keys must use identical spelling, and may repeat only for directories. This
detects directory-case aliases, duplicate files and file/directory collisions
without the naive whole-file-string neighbour trap (`bin\foo`, `bin\foo-bar`,
`bin\foo\child`). Sorting is O(M log M) for M path components, with bounded
linear auxiliary storage, and does not reorder the path/digest binding.

Validate compiler-embedded file/digest tables only when loaded. Retain an
independent sorted lookup of validated incoming files, so pruning membership
checks use binary search rather than repeated linear scans. Filesystem
snapshots remain freshly enumerated and validated on every required boundary;
do not cache mutable destination state. Keep all installed SHA-256 checks
before the first stale deletion, mandatory repair-floor behavior and exit 42.

Retain malformed/colliding-input coverage at the trusted input-loading
boundary, and wrong-content/forged-metadata coverage at completion. Benchmark
the actual-size and 1,000-entry cases under the pinned engine, with a generous
regression budget that rejects the observed multi-second quadratic behavior.

## Attribution Placeholders

Extend only the existing placeholder-label vocabulary with `authors?` across
angle, square, single-curly and double-curly delimiters. Add direct,
complete-grant, canonical, scoped and supplemental regressions, including
multiline/case variants. Ordinary named authors remain accepted. Do not change
upstream bytes, licence selections or the six scoped owner approvals.

## Recovered-Panic Documentation

Document that the Windows release desktop panic hook runs before unwinding,
including for a progress-sink panic later caught by `catch_unwind`. Its modal
dialog blocks the panicking thread until dismissed; locks held by that thread
remain held during the callback. The restart guidance is conservative, not a
claim that the app has terminated. Existing stderr/default-hook behavior and
fixed-text privacy remain unchanged. Native dialog compilation/visibility and
Windows/Homebrew acceptance remain unverified, not inferred from Wine.

## Verification

Use red/green Python/FPC/pinned-engine tests, all release helper and PowerShell
suites, workflow lint, grammar and exact-original checks. Regenerate all four
target disclosure bundles offline after the placeholder change. Rust source
and dependencies are unchanged, so historical Rust results remain historical
and no new full Rust run or native acceptance claim is needed for this pass.
Record measurements, test results and remaining limits in a follow-up execution
record and refresh current operational/PR guidance.
