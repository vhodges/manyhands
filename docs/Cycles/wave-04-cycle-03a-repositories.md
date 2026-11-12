---
title: "Wave 04 Cycle 03a: Repositories Panel And Sharing Status"
date: 2026-11-12
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7888WSJYAMC7CDFS006D0E4"
---

# Wave 04 Cycle 03a: Repositories Panel And Sharing Status

Ticket `01M784Y8C73B39GR4JWENXE5D0` (`vh-3jpbf`).
[Design](../plans/2026-11-12-wave-04-cycle-03a-repositories-design.md) and
[implementation plan](../plans/2026-11-12-wave-04-cycle-03a-repositories-implementation.md).

Status: **draft**, not yet reviewed. The owner asked on 2026-11-12 to plan
Wave 04 Cycle 03 starting from its requirements, split it in two, and
answered decisions 1–7 in that session. Decisions 8–18 are the planner's
proposals, each with its reason, for the owner to confirm. Those marked
**(owner's eye)** change something people see today or depart from an
authority.

- Drafted on the ticket branch at `7034522f`, which holds origin/main
  `37be101d`. Cycle 01, Cycle 02 and Wave 03 D6 are merged there.

## Purpose And Authority

Make each repository's sharing state visible in plain language, where the
design puts it. After this Cycle a person sees at a glance which
repositories have work not shared yet, which are up to date, which only
live on this computer, and which need something from them, with one
obvious next step in each row's menu. The footer says when updates were
last checked and how much work waits to be shared. The repository details
no longer sit in the item area, below the item being read.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 04](../Waves/wave-04-workspace-design.md): Cycle 03 (lines
   354–397, with D6's amendment at 387–391), Owner Direction (65–101),
   Explicit Exclusions (169–183), Testing (505–514), Risks And Controls
   (532–544) and Owner Decisions 1–10 (546–576), with decisions 3 (no
   per-panel keys) and 7 (28 px rows).
3. [Desktop information architecture and editor RFC](../RFC/desktop-information-architecture-and-editor.md):
   the Wave 04 amendment (40–55), inaccessible entries with a reason and
   recovery action (58–60), navigation that never saves or synchronizes
   (82–84), Poll now distinct from Refresh (282–283), removal explaining
   that local files remain (318), no repeated background prompts
   (324–328), no secret-bearing values in models (340–342), accessibility
   (344–351).
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md): `MH-REPO-003` (remove without
   touching files), `MH-COLLAB-004` (polling never pushes),
   `MH-UX-001` (progress, outcome and recovery feedback),
   `MH-NFR-003`, `MH-NFR-004`, `MH-NFR-005`.
5. The design handoff: [`03-panels.md`](../design/03-panels.md)
   (Repositories, rows), [`02-layout-and-docking.md`](../design/02-layout-and-docking.md)
   (Footer items 3 and 4), [`06-language-and-states.md`](../design/06-language-and-states.md),
   [`07-design-tokens.md`](../design/07-design-tokens.md),
   [`08-gpui-kit-mapping.md`](../design/08-gpui-kit-mapping.md), frames
   `screenshots/1a`, `1b` and `1c`.
6. Predecessors: Wave 03 D1 (the sidebar, the details block, Retry and
   Remove of an unavailable repository), D4 (Share main), D6 (the polling
   model, Settings › Checking for updates, its W-5: "Cycle 03 reads them
   instead of deriving its own"), Cycle 01 (panels, footer, dock menu,
   regions) and Cycle 02 (`ListRow`, list states, header toolbars,
   decision 18).

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`7034522f`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M784Y8C73B39GR4JWENXE5D0`, open, created 2026-11-12 from main and published. |
| Latest main merged into the ticket branch | Created from origin/main `37be101d`; a merge on 2026-11-12 found nothing new. |
| Cycle 01 merged | Yes: main `eb0798a2`. |
| Cycle 02 merged | Yes: main `37be101d`, ticket `01M757DHFAVPEVY4TWQTVKQAMJ` closed 2026-11-12. |
| Wave 03 D6 merged | Yes: main `48653d01`. Its checking states are in scope. |
| GPUI Kit 0.6.6 names verified | `PopupMenu`, `ContextMenuExt`, `ListState`, `StatusBar` read from the locked source during planning (design facts 10–15). |
| Required checks at baseline | Not run during planning. Task 0 runs them. |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge. |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents in the design's Document Amendments. It changes
nothing in `src/lib.rs`, `src/repository.rs`, `src/repository/`,
`src/results.rs`, `src/drafts/`, `src/runtime.rs` or the CLI, and adds no
Cargo dependency. Every value it shows comes from reads the desktop makes
today (`list_repositories`, `resolve_repository`, `polling_status`,
`list_remotes_redacted` through the scheduler, `show_item`), and from D6's
model.

**In flight.** Wave 04 Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`, Activity
and Needs attention) is planned after this Cycle and touches the footer
and the operations area. Open follow-up tickets from D1
(`01M5CS4MH25QQ769HV61ZRKHXG`, `01M5F3GVDYTH1Q5V5FCS9YF7V3`), D4
(`01M6H7VXJJHRYHTW9S8TJ88921`) and D5 (`01M6RAS63EWVHWWVBSZ836RXG2`) have
no code on their branches.

## Scope

1. **Sharing facts in D6's model** (design Part A): the stored last check
   outcome after a relaunch, the shared copy's redacted address, the items
   not shared yet (ahead and never shared, main included), and a finished
   share leaving the count at once.
2. **One state per repository** (Part B): Can't open, Needs fixing,
   Locked, Needs attention, Checking…, Can't reach, {n} to share, Paused,
   Up to date, This computer, or none, each with its icon, its one action
   and its sentence; the tooltip with the folder, main branch and shared
   copy.
3. **Rows and list states** (Part C): Cycle 02's row, one meaning of
   selected, no update during render, titles of what is not shared, and
   the loading, empty, busy and failed states of the repositories read.
4. **The repository menu** (Part D), by right click and by keyboard: the
   state's action, Check now, Share main, Open folder, Settings…, Remove
   from Manyhands….
5. **Toolbar** (Part E): Add or create a repository…, Check all for
   updates now, More with Refresh, Rebuild the search index and Remove
   from Manyhands….
6. **The details block leaves the item area** (Part F; vh-d78zh).
7. **Footer** (Part G): "Checked for updates {ago}" and "{n} to share".
8. **Keyboard and privacy** (Parts H, I).

## Explicit Exclusions And Downstream Obligations

- Activity, the Needs attention popover, the three operation-area rows of
  Cycle 01's K-19, and whether an unreachable repository counts in Needs
  attention: Cycle 03b.
- The Add repository sheet (Cycle 05); Add opens today's Settings page.
- Removing the panel's Settings button and the Settings sheet (Cycle 04).
- Sharing every unshared item at once (decision 7).
- A library read of unshared checkpoint commits (decision 2).
- Project and Team scopes, title-bar menus, search (Wave exclusions).
- Any library, repository-format or CLI change. A gap found in the
  headless API goes to a separate library ticket.
- **For Cycle 03b:** read the repository state from Part B's function, not
  a second derivation; the footer's right group now holds the two status
  items before Needs attention.

## Required Contract

1. **Same rows as the library.** One row per repository
   `list_repositories` gives; every state comes from service state or
   D6's model, never a guess (Wave Cycle 03 exit evidence).
2. **Navigation sends nothing.** Choosing a repository, moving the cursor
   and opening a menu save, synchronize and send nothing beyond the reads
   a choice makes today (RFC 82–84).
3. **Sharing stays deliberate.** Nothing here shares except Share main
   chosen by the person; Check now and Check all only check for updates
   (PRD `MH-COLLAB-004`).
4. **Removal keeps the files.** Remove from Manyhands… confirms first and
   says local files remain (RFC 318; `MH-REPO-003`).
5. **Keyboard.** Every new control is reachable by Tab or the menu keys,
   works by Enter or Space, has an accessible name and shows focus (RFC
   344–351; `MH-NFR-005`).
6. **Colour is never the only signal.** Every state has a label and an
   icon; every footer item has text.
7. **No secret** in a row, tooltip, description, menu or footer item:
   only names, paths, branches, remote names, the library's redacted
   addresses and titles (`MH-NFR-003`; RFC 340–342).
8. **R-60 holds.** No menu item, toolbar button, footer item or key acts
   behind a dialog or sheet.

## Acceptance And Exit Evidence

Planned proof. The Status column is filled at the final task from the
branch's own tests (by exact name) and the execution ledger. "On screen"
rows await the owner's checks; Claude records only what the owner reports.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Each repository state comes from real service state, with no fabricated rows (contract 1) | `each_state_and_its_precedence` (P1–P3); `the_stored_outcome_and_shared_copy_are_kept`; `rows_states_and_titles` | Planned |
| A2 | "{n} to share" counts what is not shared, follows this app's shares, and agrees with Settings | `to_share_counts_and_finished_shares` (P4, P5); `status_text_table` (E-9) | Planned |
| A3 | Every action works by keyboard (contract 5) | `the_menu_by_pointer_and_keyboard`; `panel_toolbars_work_by_tab_and_enter` (E-11); `status_items_say_and_choose`; the edited keyboard journeys (E-5) | Planned |
| A4 | No secret appears in a row, tooltip, menu or footer item (contract 7) | `polling_privacy` (E-15); `tooltips_say_the_state_and_the_facts` | Planned |
| A5 | Every existing repository journey still passes: select, Retry, Remove, Share main, Add | The edited tests E-1 to E-8, and the fast tier | Planned |
| A6 | The details block leaves the item area and nothing it offered is lost | `the_item_area_holds_nothing_of_the_repository` (P14); design Part F's table; E-1 to E-8 | Planned |
| A7 | Removal confirms and keeps the files (contract 4) | `the_menu_by_pointer_and_keyboard` (P11) | Planned |
| A8 | Nothing acts behind a dialog or sheet (contract 8) | P12 in the menu, toolbar and footer tests | Planned |
| A9 | One meaning of selected; no update during render (decision 3) | `a_repository_rows_selected_state_has_one_meaning` (E-3); `rows_states_and_titles` (P16) | Planned |
| A10 | The panel and footer match the design in the dark theme | On screen: the owner, against frames 1a and 1b | Planned |

## Product-Owner Decisions (2026-11-12)

Wave 04 decisions 1–10 (2026-11-07) and the Cycle 01 and Cycle 02
decisions govern and are not repeated.

**Answered by the owner on 2026-11-12:**

1. **Cycle 03 is split.** 03a (this Cycle): the Repositories panel and the
   footer's "to share" and "Checked for updates". 03b
   (`01M784Y94Q3AXT6XJX38F5VK1V`): Activity and the Needs attention
   popover. *Alternative:* one Cycle of about 13 tasks.
2. **"N to share" comes from the latest check.** It counts the items D6's
   latest check found ahead of, or not yet on, the shared copy, so "2 to
   share" means two items, not two commits. It appears once a check has
   run. No library change. *Alternatives:* a library ticket first, or
   leaving it out.
3. **Folded in:** `01M654G5VGQMXTH3BX9GKE7JEY` (vh-d78zh, the repository
   details out of the item tab) and, from `01M5CS4MH25QQ769HV61ZRKHXG`,
   the repository row's double selected signal and
   `RepositoriesPane::set` notifying during render. (03b takes the
   operations split and "Finishing".)
4. **Four more state labels:** Can't open, Needs fixing, Locked and Needs
   attention, beside the design's six. *Alternative:* one "Needs
   attention" for all.
5. **The details go to the row and tooltip.** An invalid configuration is
   Needs fixing with its guidance; the index state stays with the Tickets
   and Documents lists; the item area shows its empty state.
   *Alternative:* a Details… sheet.
6. **The footer's "N to share" is the total across repositories, and
   each activation chooses the next repository with work to share.**
7. **"N to share" is a reminder.** The tooltip names the items; sharing
   stays where it is today: each item's Share, D6's Share… in Activity,
   Share main for main. *Alternatives:* Share all…, or opening the first
   item.

**Proposed by the planner, for the owner to confirm:**

8. **Checking… comes before Can't reach (owner's eye).** The order shown
   on 2026-11-12 put Can't reach first; a person who chooses Retry now
   would then see no sign that it is working. Every other position is as
   shown. *Alternative:* keep Can't reach first.
9. **A stored outcome shows after a relaunch.** Until this process has
   checked a repository, the library's stored last outcome sets its state:
   Can't reach, Locked or Needs attention (design Part A). A stored Locked
   offers Check now, which asks for the passphrase, because the stored
   outcome names no key (R-5).
10. **Settings agrees with the panel (owner's eye).** D6's Settings line
    reads the same stored outcome and the same count: after a relaunch it
    can say "Can't reach {repo}. Your work is safe." where it said
    "Checked 2 hours ago", and "N not shared yet" also counts items never
    shared.
11. **A finished share leaves the count at once.** A successful share of an
    item, or of main, removes it from "to share" without waiting for the
    next check. A failed share removes nothing.
12. **No label while the state is not known** (reading, not checked yet,
    checking soon, waiting). The tooltip says why. Nothing is shown that
    the service did not say.
13. **The empty item area.** With no tab: "Select a repository." when none
    is chosen, as today; otherwise "Open a ticket or document from the
    lists", the design's empty-pane text, without its icon until Cycle 05.
14. **Remove from Manyhands… works for every repository (owner's eye).**
    Today the panel offers Remove only for an unavailable one; Settings
    offers it for any. The design's menu offers it for every row. The
    dialog is today's, with its "Local files remain" note; Cycle 05
    restyles it.
15. **The panel's Settings button stays** until Cycle 04 removes it with
    the sheet (Wave Cycle 04).
16. **Check all includes paused repositories**, as Settings' Check now
    does: it is an explicit request. *Alternative:* skip paused ones.
17. **Existing-test edits.** The named edits E-1 to E-15 are part of
    approving the design; those marked **Removes** are named there. Any
    other edit this Cycle's changes need is pre-authorized, recorded in
    the ledger as E-16 onward with its reason, checked by the task's
    reviewer, and called out in the review-ready summary when it removes
    an assertion.
18. **Test runs follow `AGENTS.md`.** Between tasks, only the tests a task
    wrote or changed run, by exact name. The fast tier runs once, at
    review-ready, with `--no-fail-fast`.

## Known Limits

These are visible to users; the design's Known Limits give the detail.

- "N to share" appears after the first check of a session, and another
  process's shares show at the next check (K-1).
- After a relaunch a stored outcome stands until this app checks (K-2).
- An unshared item whose title cannot be read is named by its ID (K-3).
- After a relaunch a locked key is offered Check now rather than
  Unlock… (K-4).
- A publication remote that is not SSH reads "This computer" (K-5).
- Tooltips show on hover only; their text is also each row's accessible
  description (K-6).
- The panel's Settings button stays until Cycle 04 (K-8).

## Open Questions

Decisions 8–18 await confirmation. The design's U-1 to U-6 are checked by
the tasks that first rely on them.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-12 | "start planning Wave 4 Cycle 3, start with the cycle doc and gathering requirements" | Owner direction | Research, then questions | Session, 2026-11-12 |
| Planning research | Activity is several stores with no times; no read counts unshared saves; "Share main" shares main only; the stored outcome is dropped; the details block is drawn in two places | Facts; owner questions | Decisions 1–7; design facts 5, 17, 23, 27–29 | Session, 2026-11-12 |
| Owner, 2026-11-12 | Split, "to share" source, Activity's shape, fold-ins, labels, details, footer, reminder | Owner decisions | Decisions 1–7; tickets `vh-3jpbf` and `vh-dt0aq` filed | Session, 2026-11-12 |
