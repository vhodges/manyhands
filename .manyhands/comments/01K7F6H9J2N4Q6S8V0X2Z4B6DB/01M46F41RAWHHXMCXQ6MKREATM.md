---
manyhands_managed: true
manyhands_kind: comment
id: "01M46F41RAWHHXMCXQ6MKREATM"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T16:42:30Z"
---

User-requested plan review found risks and unresolved decisions that the initial
handoff should have surfaced. No implementation approval has been given.

The [design review table](../../../docs/plans/2026-10-05-wave-02-cycle-03-authenticated-ssh-transport-design.md)
now records three questions sent to the user: username-less remote handling,
one ambiguous unlock prompt versus explicit retry, and fresh host approval
after corrupt-registry replacement loses stored pins. Answers remain pending;
recommendations are not recorded as approved decisions.

Corrected two mechanisms after inspecting the locked backend source:

- Anonymous remote creation still applies configured URL rewrites. Compare raw
  configured endpoints with effective direction-specific URLs before connecting.
- Download/push replace connection options even on connected transports. Use a
  scoped adapter owning every transfer's callbacks, with no raw Remote escape;
  inspect per-ref push rejection independently of the top-level return code.

Also corrected the unconditional rebuild/pin-preservation claim and clarified
that connect_auth includes Git service/advertisement work. Failed connection
setup does not always mean invalid credentials; cache only positively confirmed
success and classify ambiguous failures conservatively.

An engineering gate remains: production network stall bounds and safe timeout
initialization. The locked backend's connect/I/O defaults are zero, and exposed
setters modify global C state before threads. A fixture watchdog is not production
cancellation. Characterize handshake/auth/advertisement/transfer stalls early,
then specify the host/library initialization contract before driver execution.

The Cycle, design, plan, and ticket now reflect these findings. Native Windows
OpenSSL/Ed25519 and server-fixture portability are explicitly feasibility targets,
not established evidence. Added tests/checkpoints cover the identified gaps.
This review continues the same planning work on HEAD `305c0f0`; main and fetched
origin/main refs are unchanged from the recorded preflight. Only planning and
ticket/comment files changed. No Rust implementation or runtime verification
was performed for this review.

A future reviewing-cycle-plans skill could capture this audit: distinguish
approved requirements from proposals, identify decisions needing user input,
verify dependency behavior, review failures/recovery and scope boundaries, and
map each risk to evidence. The user suggested that as future work; no new skill
was created in this Cycle review.
