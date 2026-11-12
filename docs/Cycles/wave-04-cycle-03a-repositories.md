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

Status: **draft, revision 3.** Revision 3 applies review round 2 and
decision 24; it is not yet re-reviewed. The owner asked on 2026-11-12 to
plan Wave 04 Cycle 03 starting from its requirements, split it in two,
answered decisions 1–7 in that session, decisions 19–22 after review
round 1 and decision 24 after review round 2. Decisions 8–18 and 23 were
the planner's proposals, each with its reason; the owner confirmed them
all on 2026-11-12 ("They all look good"). Those marked
**(owner's eye)** change something people see today or depart from an
authority.

- Drafted on the ticket branch at `7034522f`, which holds origin/main
  `37be101d`. Cycle 01, Cycle 02 and Wave 03 D6 are merged there.
- Revision 1 (`f6d57bd4`) was reviewed by two independent reviewers
  (source and feasibility; contract and consistency): 3 Blocking, 30
  Important and 24 Minor findings, each checked against the source and
  applied in revision 2 or raised with the owner (Review Record).
- Revision 2 (`2a0f202e`) had a scoped re-review and an adversarial review
  (round 2): 1 Blocking, 16 Important and 21 Minor findings, each checked
  against the source and applied in revision 3 or raised with the owner.

## Purpose And Authority

Make each repository's sharing state visible in plain language, where the
design puts it. After this Cycle a person sees at a glance which
repositories have work not shared yet, which are up to date, which only
live on this computer, and which need something from them, with the
reason and one obvious next step at the top of each row's menu. The
footer says when updates were last checked and how much work waits to be
shared. The repository details no longer sit in the item area, below the
item being read.

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
   regions) and Cycle 02 (list states, header toolbars, decision 18).

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`7034522f`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M784Y8C73B39GR4JWENXE5D0`, open, created 2026-11-12 from main and published. |
| Latest main merged into the ticket branch | Created from origin/main `37be101d`; a merge on 2026-11-12 found nothing new. |
| Cycle 01 merged | Yes: main `eb0798a2`. |
| Cycle 02 merged | Yes: main `37be101d`, ticket `01M757DHFAVPEVY4TWQTVKQAMJ` closed 2026-11-12. |
| Wave 03 D6 merged | Yes: main `48653d01`. Its checking states are in scope. |
| GPUI Kit 0.6.6 names verified | The Kit `List` and its wrapper, `PopupMenu` and `MenuItemElement`, `ContextMenuExt` and `StatusBar`, read from the locked source during planning and review (design facts 14, 15, 31, 34, 43). |
| Required checks at baseline | Not run during planning. Task 0 runs them. |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge. |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents in the design's Document Amendments. It changes
nothing in `src/lib.rs`, `src/repository.rs`, `src/repository/`,
`src/results.rs`, `src/drafts/`, `src/runtime.rs`, `tests/` or the CLI,
and adds no Cargo dependency. Every value it shows comes from reads the
desktop makes today (`list_repositories`, `resolve_repository`,
`polling_status`, `list_remotes_redacted` through the scheduler,
`show_item`), and from D6's model.

**In flight.** Wave 04 Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`, Activity
and Needs attention) is planned after this Cycle and touches the footer
and the operations area. Open follow-up tickets from D1
(`01M5CS4MH25QQ769HV61ZRKHXG`, `01M5F3GVDYTH1Q5V5FCS9YF7V3`), D4
(`01M6H7VXJJHRYHTW9S8TJ88921`) and D5 (`01M6RAS63EWVHWWVBSZ836RXG2`) have
no code on their branches.

## Scope

1. **Sharing facts in D6's model** (design Part A): the stored last check
   outcome after a relaunch, the shared copy's redacted address, the
   unshared list (items ahead, never shared or diverged, main included,
   and the problems a check lists), kept across failed checks and
   followed at once by this app's saves and shares, and reading one
   repository again on Retry.
2. **One state per repository** (Part B): Can't open, Needs fixing,
   Locked, Needs attention, Checking…, Can't reach, {n} to share, Paused,
   Up to date, This computer, or no label (a busy opening, a non-SSH
   remote, anything not yet known), each with its icon, its one action
   and its sentence; the tooltip with the folder, main branch and shared
   copy.
3. **Rows and list states** (Part C): the panel's own row with one meaning
   of selected, the chosen repository the list no longer gives kept as a
   row, rows applied after the frame, and the loading, empty, incomplete,
   busy and failed states of the repositories read.
4. **Titles of what is not shared** (Part C), read once per item.
5. **The repository menu** (Part D), by right click and by keyboard: the
   state's sentence, its action, Check now, Share main, Open folder,
   Settings…, Remove from Manyhands….
6. **Toolbar** (Part E): Add or create a repository…, Check all for
   updates now, More with Refresh, Rebuild the search index and Remove
   from Manyhands….
7. **The details block leaves the item area** (Part F; vh-d78zh).
8. **Footer** (Part G): "Checked for updates {ago}" and "{n} to share".
9. **Keyboard, focus and privacy** (Parts H, I).

## Explicit Exclusions And Downstream Obligations

- Activity, the Needs attention popover, the three operation-area rows of
  Cycle 01's K-19: Cycle 03b.
- Which repository states the footer's Needs attention counts (an
  unreachable repository, Can't open, Needs fixing, a stored Locked or
  Needs attention, a check's problems, Can't check, a pending conflict):
  Cycle 03b decides (design K-9).
- The Add repository sheet (Cycle 05); Add opens today's Settings page.
- Removing the panel's Settings button and the Settings sheet (Cycle 04).
- Sharing every unshared item at once (decision 7).
- A library read of unshared checkpoint commits (decision 2).
- Project and Team scopes, title-bar menus, search (Wave exclusions).
- Any library, repository-format or CLI change. A gap found in the
  headless API goes to a separate library ticket.
- **For Cycle 03b:** read the repository state from Part B's function, not
  a second derivation; the footer's right group now holds the two status
  items before Needs attention; decide the Needs attention count above.

## Required Contract

1. **Same rows as the library.** One row per repository
   `list_repositories` gives, plus the chosen one it no longer gives
   (decision 22); every state comes from service state or D6's model,
   never a guess (Wave Cycle 03 exit evidence).
2. **Navigation sends nothing.** Choosing a repository, moving the cursor
   and opening a menu save, synchronize and send nothing beyond the reads
   a choice makes today (RFC 82–84).
3. **Sharing stays deliberate.** Nothing here shares except Share main
   chosen by the person; Check now and Check all only check for updates
   (PRD `MH-COLLAB-004`).
4. **Removal keeps the files.** Remove from Manyhands… confirms first and
   says local files remain (RFC 318; `MH-REPO-003`).
5. **Keyboard.** Every new control is reachable by Tab or the menu keys,
   works by Enter or Space, has an accessible name and shows focus; the
   reason for a state is readable without hovering (RFC 344–351;
   `MH-NFR-005`; decision 21).
6. **Colour is never the only signal.** Every state with a colour has a
   label and an icon; every footer item has text.
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
| A1 | Each repository state comes from real service state, with no fabricated rows (contract 1) | `each_state_and_its_precedence` (P1–P3, P18–P20, P34, P35); `the_stored_outcome_and_shared_copy_are_kept`; `rows_and_list_states` (P38) | Planned |
| A2 | "{n} to share" counts what is not shared, survives a failed check, follows this app's saves and shares, and agrees with Settings | `to_share_counts_and_finished_shares` (P4, P5); `unshared_follows_attempts_shares_and_saves` (P30–P33); `finished_shares_and_saves_are_heard`; `status_text_table` (E-9) | Planned |
| A3 | Every action works by keyboard, and a state's reason is readable without hovering (contract 5) | `the_menu_by_pointer_and_keyboard` (P10, P21); `panel_toolbars_work_by_tab_and_enter` (E-11); `status_items_say_and_choose`; the edited keyboard journeys (E-5) | Planned |
| A4 | No secret appears in a row, tooltip, menu or footer item (contract 7) | `polling_privacy` (E-15, P17); `tooltips_say_the_state_and_the_facts` | Planned |
| A5 | Every existing repository journey still passes: select, Retry, Remove, Share main, Add | The edited tests E-1 to E-8, and the fast tier | Planned |
| A6 | The details block leaves the item area and nothing it offered is lost | `the_item_area_holds_nothing_of_the_repository` (P14); the kept row (P18); design Part F's table; E-1 to E-8 | Planned |
| A7 | Removal confirms and keeps the files (contract 4) | `the_menu_by_pointer_and_keyboard` (P11) | Planned |
| A8 | Nothing acts behind a dialog or sheet (contract 8) | P12 in the menu, toolbar and footer tests, each with a dialog and a sheet | Planned |
| A9 | One meaning of selected; rows applied after the frame (decision 3) | `a_repository_rows_selected_state_has_one_meaning`, unchanged; `rows_and_list_states` (P9, P16); that nothing notifies during render is checked in code review | Planned |
| A10 | Navigation sends nothing (contract 2) | `the_menu_by_pointer_and_keyboard` (P22) | Planned |
| A11 | Check now and Check all only check (contract 3); Check all checks each repository (D6 W-5), skipping those waiting on a key or host, live or stored | `check_all_checks_each_repository` (P23, P24); `the_menu_by_pointer_and_keyboard` | Planned |
| A12 | Colour is never the only signal (contract 6) | `each_state_and_its_precedence` (every state with a tone has a label); `status_items_say_and_choose` | Planned |
| A13 | The panel and footer match the design in the dark theme | On screen: the owner, against frames 1a and 1b | Planned |

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
   operations split and "Finishing".) Review found the first already met
   at `7034522f`: the row sets no `aria_selected`, and a test pins it
   (design fact 34); this Cycle keeps it so.
4. **Four more state labels:** Can't open, Needs fixing, Locked and Needs
   attention, beside the design's six. *Alternative:* one "Needs
   attention" for all.
5. **The details go to the row and tooltip.** An invalid configuration is
   Needs fixing with its guidance; the index state stays with the Tickets
   and Documents lists; the item area shows its empty state.
   *Alternative:* a Details… sheet.
6. **The footer's "N to share" is the total across repositories, and
   each activation chooses the next repository with work to share.**
7. **"N to share" is a reminder.** The row's tooltip names the items;
   sharing stays where it is today: each item's Share, D6's Share… in
   Activity, Share main for main. *Alternatives:* Share all…, or opening
   the first item.

**Proposed by the planner and confirmed by the owner on 2026-11-12:**

8. **Checking… comes before Locked, Needs attention and Can't reach
   (owner's eye).** The order shown on 2026-11-12 put those first; a
   person who chooses Unlock…, Retry now or Check now would then see no
   sign that it is working. D6's Settings line already puts a running
   check first. Every other position is as shown. *Alternative:* keep the
   order shown.
9. **A stored outcome shows after a relaunch.** Until this process has
   checked a repository (no attempt this session) and while it is not
   checking, the library's stored last outcome sets its state: Can't
   reach, Locked or Needs attention (design Part A). A stored Locked
   offers Check now, which asks for the passphrase, because the stored
   outcome names no key (R-5). A stored outcome never outranks Checking….
   Revision 3: as Settings does, a stored outcome does not apply to a
   paused, off or held repository, so a paused one reads Paused in both
   places (decision 10; design R-32).
10. **Settings agrees with the panel (owner's eye).** D6's Settings line
    reads the same stored outcome and the same count: after a relaunch it
    can say "Can't reach {repo}. Your work is safe." where it said
    "Checked 2 hours ago", and "N not shared yet" also counts items never
    shared and diverged ones (decision 19). D6's design is amended to
    match (design Document Amendment 8).
11. **A finished share leaves the count at once.** A finished share of an
    item (Share, Republish…, a close or an approval) or of main that
    published, by the same test the app already uses to clear a shared
    item's marks, removes it from "to share" without waiting for the next
    check, even when the share ended with its index refresh pending. A
    close or an approval that published also removes main, which it
    merges into and publishes. A share that did not publish removes
    nothing.
12. **No label while the state is not known** (reading, not checked yet,
    checking soon, waiting). The tooltip and the menu's first line say
    why. Nothing is shown that the service did not say.
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
16. **Check all includes paused repositories and skips those waiting on a
    key or host.** A paused repository is checked, as Settings' Check now
    does, since Check all is an explicit request; one waiting for a
    passphrase, a key choice or a host approval is skipped, so one click
    never raises prompts one after another (RFC 324–328); after a
    relaunch that includes one whose stored outcome says it waits on a key
    or host. It is reached through its own row's action. *Alternative:*
    skip paused ones too.
17. **Existing-test edits.** The named edits E-1 to E-16 are part of
    approving the design; those marked **Removes** are named there. Any
    other edit this Cycle's changes need is pre-authorized, recorded in
    the ledger as E-17 onward with its reason, checked by the task's
    reviewer, and called out in the review-ready summary when it removes
    an assertion.
18. **Test runs follow `AGENTS.md`.** Between tasks, only the tests a task
    wrote or changed run, by exact name. The fast tier runs once, at
    review-ready, with `--no-fail-fast`.

**Answered by the owner on 2026-11-12, after review round 1:**

19. **Problems a check lists.** A diverged item counts in "to share",
    since sharing merges it. Any other problem the check lists (a dirty
    working copy, a conflict, a branch deleted on the shared copy and the
    rest) makes the row Needs attention: "{n} items need a look. Activity
    lists them." "Up to date" means a completed check with nothing to
    share and nothing to look at. *Alternative:* Up to date anyway.
20. **A publication remote that is not SSH shows no label.** Its sentence
    says "{remote} isn't an SSH shared copy, so Manyhands can't check or
    share it.", and its menu leads with "Set up sharing…", which opens
    Settings › Remotes for that repository. *Alternative:* "This
    computer".
21. **The reason is the menu's first line.** The menu opens with the
    state's sentence, then the state's action, then the fixed items, so a
    sighted keyboard user reads why a repository needs attention. The
    owner noted that the old details block was not discoverable.
    *Alternative:* hover only, as a known limit.
22. **A chosen repository the list no longer gives stays as a row.** When
    another process unregisters the chosen repository, it stays in the
    panel as the chosen row, reading Can't open, with Retry and Remove
    from Manyhands… in its menu, until it is removed or another
    repository is chosen. *Alternatives:* drop the choice, or a notice in
    the item area.

**Proposed after review round 1 and confirmed by the owner on 2026-11-12:**

23. **The footer's "to share" with one repository (owner's eye).** When the
    only repository with work is already chosen, activating it does
    nothing (no reload) and its name reads "{n} to share in {repo}."; it
    never changes the layout, so a hidden Repositories panel stays hidden.
    *Alternative:* reselect and reload it.

**Answered by the owner on 2026-11-12, after review round 2:**

24. **A save counts at once.** A request in this app that made a
    checkpoint without publishing (a save, a new ticket or document, a
    move, a saved comment, a local repair) adds its item to "N to share"
    immediately, for a repository with a shared copy, just as a share
    removes it (decision 11). So a row never reads "Up to date" right
    after a save. *Alternative:* wait for the next check, as a known
    limit.

## Known Limits

These are visible to users; the design's Known Limits give the detail.

- "N to share" starts from the first completed check of a session and
  then follows this app's saves and shares at once; another process's or
  the CLI's work, a resumed share and a comment's publication show at the
  next check (K-1).
- After a relaunch a stored outcome stands until this app checks (K-2).
- An unshared item is named by its ID until its title is read, and for
  the session after three failed tries (K-3); a row names at most five
  items, then "and {n} more" (K-11).
- After a relaunch a locked key is offered Check now rather than
  Unlock… (K-4).
- A publication remote that is not SSH shows no label; its sentence says
  why (K-5).
- Tooltips show on hover only; their text is also each row's accessible
  description, and the state's sentence alone is its menu's first line
  (K-6).
- "Checked for updates {ago}" is worked out when the footer is drawn
  (K-7).
- The panel's Settings button stays until Cycle 04 (K-8).
- The footer's Needs attention does not yet count Can't open, Needs
  fixing, Can't reach or the Needs attention this Cycle adds; Cycle 03b
  decides (K-9).
- "Checked for updates {ago}" is the latest check of any repository, even
  while another cannot be reached (K-10).

## Open Questions

None from the owner: decisions 1–24 are settled. The design's U-1 to U-6
are checked by the tasks that first rely on them.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-12 | "start planning Wave 4 Cycle 3, start with the cycle doc and gathering requirements" | Owner direction | Research, then questions | Session, 2026-11-12 |
| Planning research | Activity is several stores with no times; no read counts unshared saves; "Share main" shares main only; the stored outcome is dropped; the details block is drawn in two places | Facts; owner questions | Decisions 1–7; design facts 5, 17, 23, 27–29 | Session, 2026-11-12 |
| Owner, 2026-11-12 | Split, "to share" source, Activity's shape, fold-ins, labels, details, footer, reminder | Owner decisions | Decisions 1–7; tickets `vh-3jpbf` and `vh-dt0aq` filed | Session, 2026-11-12 |
| Review round 1, contract and consistency (`f6d57bd4`) | 1 Blocking, 17 Important, 11 Minor; among them Retry lost for a busy resolution, stored outcomes outranking Checking…, Retry on an unread row, Remove's focus, contracts without acceptance rows, the footer's sum and single repository, Check all and locked keys, Task 3's size | Rulings; revised proposals; owner questions | Design rulings R-12, R-16, R-19, R-22–R-27; decisions 9, 11, 16 revised, 23 proposed; A10–A12; Task 3 split | Design Review Record |
| Review round 1, source and feasibility (`f6d57bd4`) | 2 Blocking, 13 Important, 13 Minor; among them the Kit list owning "selected", the kept selection without a row, `Phase::Running` never entered, the share test, Settings… on the chosen repository, the network in window tests, the privacy sweep allowing the panel, menu enablement unreadable | Facts; rulings; owner question | Design facts 33–48; R-3, R-7, R-9, R-14, R-21, R-26; decision 22 | Design Review Record |
| Owner, 2026-11-12, after round 1 | "Up to date" with a check's problems; non-SSH remotes; the reason for keyboard users; the kept selection. The owner asked what the details block was: the selected repository's, drawn in the empty item area and at the end of every open item tab, "so it was not discoverable" | Owner decisions; clarification | Decisions 19–22 | Session, 2026-11-12 |
| Owner, 2026-11-12 | Proposals 8–18 and 23, summarized in plain words: "They all look good" | Owner decisions | Decisions 8–18 and 23 confirmed | Session, 2026-11-12 |
| Review round 2, scoped re-review (`2a0f202e`) | 0 Blocking, 6 Important, 12 Minor; among them Check all after a relaunch, focus after Choose a key… and Approve host…, a stored outcome over Paused, `remove_repository`'s other callers, the row hook's path to the Workspace | Facts; rulings; named edit | Design facts 53, 57–62; R-13, R-25, R-32, R-36; E-16; decisions 9 and 16 refined | Design Review Record |
| Review round 2, adversarial (`2a0f202e`) | 1 Blocking, 10 Important, 9 Minor; among them a failed check wiping "to share", a share applied before an earlier answer, phase and conflict ignored, this app's saves not counted, titles lost to stale stamps, an unread registration shown as Can't open, a re-registered kept row, main left after a close | Facts; rulings; owner question | Design facts 49–52, 54–56; R-28–R-31, R-33–R-42; decision 11 refined; decision 24 | Design Review Record |
| Owner, 2026-11-12, after round 2 | "Count saves at once" | Owner decision | Decision 24 | Session, 2026-11-12 |
