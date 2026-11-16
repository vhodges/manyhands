---
slug: vh-0tcrp
id: 01M7JJ02N7XR3T2D6XT7BMV7TE
title: Repository add requires an SSH publication remote and key; support read-only use and more transports
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A repository can only be added to Manyhands when its publication remote
(`publication_remote` in `.manyhands/config.toml`) has SSH-form fetch and push
URLs. Anything else, including an anonymous `https://` clone, is rejected as an
unavailable or ineligible publication remote (`publication_remote_eligible` in
`src/repository.rs`, over `configured_remote_endpoint` in
`src/repository/transport/endpoint.rs`). Transport support is SSH only, and
the SSH client needs a key.

Found 2026-11-16 while demoing the v0.2.0-rc.1 Homebrew install on a work
Mac. The work environment has no GitHub and no SSH keys, so the owner cloned
the Manyhands repository over anonymous HTTPS and could not add it, although
nothing in the demo needed to publish. Workaround: set `origin` to an
SSH-shaped URL and keep the HTTPS URL as a second remote for fetching. The
check only parses the URL, and it rejects `insteadOf` rewriting by design.

Two requests from the owner:

1. **A read-only or local-only way in.** Adding a repository should not need a
   usable publication remote or a key. Reading, browsing and local authoring
   have no need for one. Operations that publish (sync, comment publication,
   ticket close and promotion) should refuse with a clear answer that names
   the missing remote or key, not block the add. The same applies to a
   repository with no publication remote configured at all.
2. **More transports and authentication methods.** Support at least anonymous
   and token-authenticated HTTPS, `file://` and local paths, and SSH
   with an agent or no managed key, instead of SSH with a Manyhands-managed key
   only. Decide how credentials are stored, how host and endpoint trust work
   per transport, and what the privacy rules for output are for each. The SSH
   harness and its host-trust model are the starting point, not the limit.

To settle in planning: whether (1) ships alone before (2); how the committed
`publication_remote` name stays portable when one machine reaches the remote
over HTTPS and another over SSH; whether the eligibility check keeps rejecting
rewritten URLs; and which transports the SSH-harness tests must cover. Per the
Testing Strategy, only transport-dependent behavior gets an SSH case.
