---
id: 01M6PSTN6J43YGN95THV1X3D8N
item_id: 01M4CC0VN2K8DBHNMZ46NTX5JJ
created_at: 2026-11-05T21:37:43.189280421Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
## Part B checkpoint (Task 13)

**Part B is done.** Tasks 10 to 13 added conflict resolution to the desktop. Each task had an independent review, and every finding was fixed or recorded as a ruling or known limit. See the execution ledger.

- **Task 10:** conflict reads and the resolve intents.
- **Task 11a:** kept resolution sets that survive a restart.
- **Task 11b:** the resolution sheet.
- **Task 12:** guidance for conflicts that need fixing outside Manyhands.
- **Task 13:** the entry points. Review changes is on the conflict banner, the operation area and pending rows after a restart, and Repair locally… and Resolve conflict… are in the lifecycle dialog.

**Main.** `origin/main` is unchanged at `c69789d9` and already merged.

**Gates at `5f5b418e`:** fmt, check, and both clippy lines (`-D warnings`) are clean. The whole desktop bin suite gives 1498 passed, 0 failed. The changes are desktop-only, so the SSH tier does not apply.

**Owner rulings, 2026-11-05:**
- K-14 is widened to remote conflicts.
- **R-25:** a remote kept set outlives its record while its attempt stands.
- **R-26:** a local set goes outside on `external_repair`, or on a read that can't start a resolution.
- **R-27:** draw every handoff 2g element; the side columns are plain tinted lines.
- **R-28:** pending rows offer Review changes for resolve, retry_resolution and external_repair.
- **K-24:** a Share main conflict offers no Review changes and is resolved outside Manyhands.

**Orchestrator rulings:**
- **R-24:** a resolve can't be given up.
- **R-29:** entry points don't wait for the lifecycle lock; Repair locally… waits for in-flight work; the dialog reopens with its way on after a local resolution.

**Known limits added:** K-19 to K-24.

**Next.** Part C, Task 14 (Republish… and Resume), now in progress.