# Cross-Channel Disclosure Remediation Design

## Approved Boundary

The owner selected disclosure remediation before finishing Phase II Nix
desktop acceptance, then approved the data-first design on 2026-10-31.
Keep dependencies, features, accepted licence selections, persistent signing
identity, and existing tags unchanged. No commit, merge, push, workflow,
upstream contact, publication, dependency replacement, or special compliance
distribution package is authorized. Preserve all existing uncommitted work
in `phase-ii-packaging`. Editor feasibility is outside this work and no
feasibility binary ships.

## Approach And Alternatives

Prefer exact original notice inputs and a stricter shared generator over
updating/forking dependencies solely to change scanner output. The latter
would expand Rust/runtime verification and would not necessarily resolve
the underlying source licences. Deferring desktop work was offered; the
owner chose remediation instead.

Supplement published crates with unmodified upstream licence/grant files,
each bound to an exact crate version/checksum, immutable source linkage,
and original-text checksum. Preserve source-member attribution and original
AUTHORS/licence-directory files that filename-only harvesting missed.
Do not infer copyright holders from Cargo authors. Where upstream genuinely
supplies no named header, retain the exact grant/declaration and a narrowly
version-bound provenance explanation; no blanket canonical-text exemption.
Later licence changes without proved applicability remain unselected.

On 2026-11-01 the owner explicitly approved a narrower policy for the six
recovered declaration-only releases: `block 0.1.6`, `leak 0.1.2`,
`leaky-cow 0.1.1`, `mac 0.1.1`, `malloc_buf 0.0.6`, and `seahash 4.1.0`.
Treat those exact, checksum-verified original declarations as reviewed grants
by reference, retaining standard terms and original manifests/source
evidence and stating that no named original notice was supplied. This is
not evidence that the packages were unlicensed, not an invented holder,
and not authorization for a blanket exception or a different licence
selection. Unapproved declaration-only cases still fail closed. This
decision does not resolve KDE generated-binding or Apple SDK conditions.

The same inputs/generator serve all four native notice graphs and Nix.
Generation remains offline; only dependencies in the selected normal graph
enter the installed notices. CLI disclosures/inputs remain desktop-free.
Source filtering derives only safe, enumerated supplemental filenames,
never a broad private-directory inclusion.

## Data And Validation

Extend `packaging/licenses/supplemental.json` with exact source provenance
and reviewed resolution kinds. Store original bytes in
`packaging/licenses/supplemental/`, deduplicating identical texts when useful.
Bundled source members can instead be copied from checksum-pinned crates,
with exact paths/hashes and no network access. Include provenance, original
text, and any unresolved conditions in the installed inventory/HTML.

Validate package/version/source/checksum before applying an entry, reject
unsafe paths/symlinks/hash mismatches, and reject unknown resolution kinds.
Canonical SPDX fallback is acceptable only when actual matching original
notices exist or a reviewed exact upstream grant without a named header is
preserved. Aggregate unresolved package diagnostics; do not silently
manufacture attributions or claim complete clearance from scanner success.

## Protocol-Specific Hold

The original locked Plasma source archive includes separately licensed
KDE protocol XML. `blur.xml` is LGPL-2.1-or-later and GPUI uses its generated
Rust client bindings. Preserve the original XML/SPDX notices, full LGPL
text, exact source archive/checksum, crate MIT terms, and provenance.
Do not label every vendored definition MIT or conclude that system
`libwayland` makes the generated Rust code dynamically replaceable.

Smithay issue #796 contains a relevant API-definition interpretation, but
no explicit exception/clarification from blur's named rights holders was
found. The decision on generated-binding coverage and any source/relinking
compliance arrangement remains an owner/appropriate-review decision.
Local source builds are not binary-publication clearance. Do not change
dependencies, grant a waiver, or assemble a special compliance product
without separate approval. New objc2 originals also contain an Apple SDK
caveat; preserve it, not silently resolve it.

## Verification

Use failing regressions for source-member harvesting, exact supplements,
grant-only evidence, unsafe inputs, missing attributions, and no-output-on-
failure. Validate all committed input hashes and Cargo.lock associations.
Regenerate CLI/desktop disclosures for all four targets using pinned
cargo-about; inspect actual selected graphs and source/notice inventories.
Run the portable helper suite, Python syntax/whitespace and workflow lint.
Rebuild Nix CLI/full outputs and package checks only once disclosure inputs
stabilize; distinguish local build evidence from native runtime and legal/
publication acceptance. Use scoped read-only review before handoff.

Research evidence: `/tmp/opencode/task12-license-audit-2026-10-31/`,
`/tmp/opencode/task12-remediation-provenance/`, and
`/tmp/opencode/protocol-license-investigation/`. The execution record keeps
the actual verification and outstanding decision status.
