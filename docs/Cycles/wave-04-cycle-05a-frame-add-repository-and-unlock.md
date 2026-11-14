---
title: "Wave 04 Cycle 05a: Common Frame, Add Repository And Unlock Key"
date: 2026-11-14
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7D7V08NZYRJBM59NYB1TGV9"
---

# Wave 04 Cycle 05a: Common Frame, Add Repository And Unlock Key

Ticket `01M7D5Z7FZTGTC1HE25KQ1MYQY`.
[Design](../plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-design.md) and
[implementation plan](../plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-implementation.md).

Status: **draft** (revision 2).

- On 2026-11-14, with Cycle 04 in flight, the owner asked to plan Wave 04
  Cycle 05 "the way Cycle 04 did". Execution waits until Cycle 04 lands on
  main (decision 1).
- The same day the owner split the Cycle on the Wave's split line
  (decision 2), answered decisions 3 and 4, and accepted the planner's
  outline and constraints with decision 5 ("Write the documents").
- Decisions 6–14 are the planner's proposals, awaiting confirmation.
- After review round 1 the owner answered decisions 15–18.
- Those marked **(owner's eye)** change something people see today or
  depart from an authority.

Revision history:

- Drafted on the ticket branch from origin/main `ea6d899b`; ticket
  checkpoints `0698f556` (created) and `2ee459a5` (renamed 05a, decisions
  1–5).
- Review round 1 read revision 1 at `9e22c765`:
  - source and feasibility: 1 Blocking, 9 Important, 12 Minor;
  - contract and consistency: 2 Blocking, 19 Important, 22 Minor.
- Revision 2 applied both, and decisions 15–18.

## Purpose And Authority

After this Cycle, the app's dialogs share one frame. Add repository is
its own sheet instead of two forms at the bottom of Settings, and the
Unlock key dialog looks like the design. More specifically:

- every dialog the app has, apart from the two Cycle 05b restyles, has a
  header, a body and a footer, with its buttons on the right;
- Add repository opens as a 470 px sheet from the Repositories panel, from
  Settings and from the routes that check a folder. It offers an existing
  folder or a new repository, shows its checks as rows with their fixes,
  offers the checked-out branch as the main branch, and lets the person
  choose how the repository shares;
- the Unlock key dialog names the key by its label and fingerprint, and
  keeps focus in the field when it refuses what was typed;
- the empty item area shows the faded app icon.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 04](../Waves/wave-04-workspace-design.md):
   - **Cycle 05:** scope, exclusions, exit evidence and split line, with
     Cycle 04's amendment to its "Common frame" (Cycle 04 Document
     Amendment 2, applied at Cycle 04's approval);
   - Owner Direction, item 6;
   - Explicit Exclusions;
   - Testing;
   - Owner Decisions 9 and 10.
3. [Desktop information architecture and editor RFC](../RFC/desktop-information-architecture-and-editor.md):
   - the Wave 04 amendment (40–55), including Git detail in dialogs and
     sheets (55);
   - onboarding, remotes and key settings (315–328): Add validates a
     local path, Create needs an empty writable location, both show and
     require the primary branch, a missing identity prompts for name and
     email, non-SSH remotes stay visible, cancelling an unlock leaves
     local work usable without repeated background prompts;
   - accessibility and keyboard (343–347).
   - The [credential RFC](../RFC/authentication-and-credential-handling.md)
     also applies: a passphrase is kept only in memory (143–150), and a
     dismissed prompt never persists approval (198).
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md): `MH-REPO-001` (explicit
   primary branch), `MH-REPO-002`, `MH-CRED-001`, `MH-NFR-003`,
   `MH-NFR-005`.
5. **The design handoff:**
   - [`05-sheets-and-dialogs.md`](../design/05-sheets-and-dialogs.md):
     the common frame, "Add repository sheet (1d)" and "Unlock key dialog
     (2h)";
   - [`04-item-area.md`](../design/04-item-area.md): "Empty pane";
   - [`06-language-and-states.md`](../design/06-language-and-states.md);
   - [`07-design-tokens.md`](../design/07-design-tokens.md);
   - [`08-gpui-kit-mapping.md`](../design/08-gpui-kit-mapping.md);
   - frames `screenshots/1d` and `2h`.
6. **Predecessors:**
   - Wave 03 D1: the Add and Create forms and their rulings (R-T12b,
     check first on Add);
   - D2: a typed passphrase does not outlive its page;
   - D3: R-60, nothing acts behind a sheet or dialog;
   - D6: a background job never prompts (B5, R-26);
   - Cycle 01: the footer, the sheet layer;
   - Cycle 03a: the Repositories panel's toolbar and empty state, and the
     empty item area's text (its decision 13);
   - Cycle 03b: Activity and Needs attention, where a declined unlock
     waits;
   - **Cycle 04:** the Settings sheet, `SheetOwner`, `show_settings` and
     `SettingsRoute`, the focus contract (its design A6 and A7), and its
     Downstream Obligations "For Cycle 05".

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`2ee459a5`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M7D5Z7FZTGTC1HE25KQ1MYQY`, open, created 2026-11-14 from main and published |
| Latest main merged into the ticket branch | Created from origin/main `ea6d899b`; main has not moved. Task 0 merges main again once Cycle 04 has merged |
| Cycle 01 merged (Wave dependency) | Yes |
| Cycle 04's planning approved | Yes: `ded8402f` on `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV` |
| **Cycle 04 merged (decision 1; Wave dependency)** | **No: in execution (its Task 0 is committed on its branch). Task 0 does not start before it merges** |
| Cycle 05b split off (decision 2) | `01M7D7JK5MSY6J471JEKG3CZXR`, open, depends on this ticket |
| GPUI Kit 0.6.6 names read | `Dialog` title and footer slots, `Sheet`, Root's layers, `SelectState` and its delegate, `ToggleGroup` (no keyboard path), `apply_config`, `Input`, `img`, from the locked source (design facts 32–41) |
| Required checks at baseline | Not run during planning; Task 0 runs them |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents in the design's Document Amendments. It changes
nothing in `src/lib.rs`, `src/repository/`, `src/results.rs`,
`src/drafts/`, `tests/` or the CLI, and adds no Cargo dependency. Every
value it shows comes from reads and requests the desktop makes today:
`inspect_repository`, `list_keys`, the confirmed `RepoEnable` and
`RepoCreate`, and the unconfirmed `RemoteSelect`.

**In flight.** Cycle 04 rewrites the Settings host, the sheet owner, the
Repositories page and the routes into it, which this Cycle moves Add
repository out of. This Cycle is planned against Cycle 04's approved
design and executed on its merged code; Task 0 re-maps every citation
(design U-1). No other open branch changes `src/desktop/` (planning
sweep, 2026-11-14).

## Scope

1. **The common frame** (design Part A): one way to build a dialog's
   header, body and footer, with the footer's buttons right-aligned,
   applied to every dialog the app has except the
   lifecycle dialog and the resolve conflict sheet (Cycle 05b), and to
   the details sheet's header. Primary buttons, the focus ring and the
   scrim take the design's colours through the Kit theme.
2. **The Add repository sheet** (Part B):
   - its own sheet on Root's one sheet layer, kept by the workspace
     (Cycle 04's `SheetOwner`, with a new case);
   - opened from the Repositories toolbar's `+`, the empty Repositories
     panel, Settings › Repositories, and the routes that check a folder;
   - from Settings it replaces Settings, and Cancel or Add brings
     Settings back at Repositories (Wave decision 10);
   - the mode control, the folder and its checks as rows with their
     fixes (Open its settings for a registered folder, decision 18), the
     main branch select, "Share changes through", the identity step, the
     note and the footer;
   - Create with Name and Location.
3. **"Share changes through"** (Part C): the folder's remotes and "Keep on
   this computer"; the choice is sent after the add succeeds (decision
   3).
4. **The Unlock key dialog** (Part D) in `05`'s layout.
5. **The empty item area's icon** (Part E).
6. **Copy** (Part F) in the `06` vocabulary.
7. **Folded in** (Part G), from `01M5F3GVDYTH1Q5V5FCS9YF7V3`, the four
   items Cycle 04 left for this Cycle:
   - the Browse reason drawn beside each Browse;
   - text while the folder chooser is open;
   - Retry for a busy folder check;
   - a folder whose path is not text named, not silently ignored.

## Explicit Exclusions And Downstream Obligations

- **Out of scope here:**
  - the lifecycle dialog and the resolve conflict sheet (Cycle 05b,
    decision 2), apart from what the theme changes for every button and
    scrim;
  - an inline error for a wrong passphrase (decision 4): the library
    reports it as `transport_unavailable` (`01M5E6DYJ6GN7DMW4424TH3V78`);
  - a "You can save changes here" check: no read tells the desktop
    whether a folder is writable;
  - the design's File › Add repository entry: the Wave excludes the
    title-bar menus;
  - the General, Appearance and Your name & email pages, theme switching
    and new settings (Wave);
  - the other items of `01M5F3GVDYTH1Q5V5FCS9YF7V3`.
- **Any library, repository-format or CLI change.** A gap found in the
  headless API goes to a separate library ticket.
- **For Cycle 05b.** It builds the lifecycle dialog and the resolve
  conflict sheet on this Cycle's frame (design Part A) and its theme
  colours. Its ticket records the Wave's "Check first".

## Required Contract

1. **One frame, behaviour unchanged.** Every dialog in Part A's list
   keeps its title, text, buttons, which control has focus when it
   opens, what Enter does, what Escape does, and where focus goes when it
   closes, and its buttons' order, except the Confirmation dialog and
   Unlock key, which put the action last as `05` draws it (decision 15).
   Unlock key's own changes are decision 14's, and the details sheet's
   description moves into its header (design A5).
   Only the layout changes: header, body, footer (Wave: "Only its frame,
   layout, copy and states change").
2. **Add repository is a sheet.** It is a right-side 470 px sheet on
   Root's sheet layer that the workspace keeps open while it is shown,
   never a page in the item area or Settings (Wave; `05`, 1d).
3. **From Settings and back.** Opened from Settings, it replaces
   Settings; Cancel, Escape, the close button, the scrim or a successful
   add bring Settings back at Repositories, with the origin Settings
   recorded when it first opened (Wave decision 10; Cycle 04's
   Downstream Obligations). A typed passphrase in Settings is emptied
   when Add repository replaces it.
4. **Focus is trapped and returns,** exactly as the design's table B9
   (G1–G20) sets out. In short:
   - Tab stays inside the sheet and its dialogs, and nothing pulls focus
     behind the sheet;
   - focus returns to the control that opened the sheet (for an Activity
     row, the list; for a popover row, the footer item, as Cycle 04), or
     to Settings when it came from Settings;
   - when that control is no longer drawn, focus goes to its region;
   - work that lands after the sheet closed moves no focus and opens no
     dialog.
5. **Every way in lands** on the sheet in the right mode, with the folder
   field focused, and for a route that names a folder, the folder filled
   and checked (design Part B2).
6. **Behaviour unchanged, with named exceptions.** Add and Create send
   what today's forms send, through today's confirmation, except **(owner's
   eye)**:
   - the mode control shows one form at a time (Part B);
   - Add is enabled only when it can go ahead, and Create needs a Main
     branch (decision 9);
   - the main branch is chosen from the folder's branches (decision 10);
   - Create takes a Name and a Location (decision 11);
   - "Share changes through" sends the chosen remote after the add
     (decision 3);
   - a successful add closes the sheet (decision 12);
   - the inspect routes land on the sheet, with Open its settings for a
     registered folder (decisions 13, 18);
   - typed text is kept when the sheet closes, except after a successful
     add (decision 16);
   - Ctrl+, does nothing while the sheet is open (design R-14);
   - the Unlock dialog's changes (decision 14);
   - footer button order on the Confirmation dialog and Unlock key
     (decision 15);
   - the folded-in fixes (Part G).
7. **R-60 holds** (Wave 03 D3). Nothing in the workspace acts behind the
   Add sheet, and nothing pulls focus behind it.
8. **No secret.** No new text, tooltip or accessible name names a
   passphrase or private key. A remote is shown as today's Remotes page
   shows it (`MH-NFR-003`; RFC 321–328).
9. **Escape** follows decision 7: on every dialog, as today; on the Add
   sheet, as Cancel, also while a request runs. Nothing is cancelled.

## Acceptance And Exit Evidence

Planned proof. The Status column is filled at the final task from the
branch's own tests, by exact name, and the execution ledger. "On screen"
rows await the owner's checks; Claude records only what the owner
reports.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Every dialog on the common frame keeps its keys, focus and return (contract 1) | `every_dialog_keeps_its_keys_and_focus` (P1); `footer_buttons_tab_in_drawn_order` (P2); the existing dialog tests, edited only as the named edits say | Planned |
| A2 | Add repository is a sheet the workspace keeps (contract 2) | `add_repository_is_a_sheet_the_workspace_keeps` (P3); `nothing_acts_behind_the_add_sheet` (P15) | Planned |
| A3 | Every way in lands (contract 5) | `each_way_in_opens_the_add_sheet` (P4) | Planned |
| A4 | Replacing Settings and returning to it (contract 3; Wave decision 10) | `add_from_settings_returns_to_settings` (P6) | Planned |
| A5 | Focus trapped, Escape closes, focus returns to the invoking control (Wave exit; contract 4) | Design B9 rows G1–G20, each read after a draw, through P3, P4, P5 `focus_returns_to_what_opened_add`, P6, `the_add_sheet_closes_every_way` (P7), `late_work_moves_no_focus_after_add_closes` (P8), P9 (G20), P10 (G16) and P15 (G18, G19) | Planned |
| A6 | Adding and creating a repository, including the identity step and each failed check's recovery action, by keyboard (Wave exit) | `add_and_create_by_keyboard` (P16); `folder_checks_are_rows_with_their_actions` (P9); the existing Add and Create tests, moved and edited only as the named edits say | Planned |
| A7 | The design's Add repository layout: mode, checks, main branch, sharing, note, footer (Wave scope) | P9; `the_main_branch_is_the_checked_out_one` (P10); `add_is_enabled_only_when_it_can_go_ahead` (P11); `create_joins_location_and_name` (P12); `share_changes_through_lists_the_folders_remotes` (P13) | Planned |
| A8 | The chosen remote is sent after the add (decision 3) | `a_chosen_remote_is_selected_after_the_add_succeeds` (P14) | Planned |
| A9 | Unlocking, Not now (the key then waiting in Needs attention) and a refused passphrase by keyboard; focus returns; a wrong passphrase as today (Wave exit, as decision 4 limits it) | P1's and P2's Unlock rows; `the_unlock_dialog_names_the_key` (P18); `a_refused_passphrase_keeps_focus_in_the_field` (P19); the Unlock evidence tests the design's Testing names, edited only as E-9 says | Planned |
| A10 | Copy in the `06` vocabulary | `frame_copy_uses_the_vocabulary` (P20) | Planned |
| A11 | No passphrase or private key text reaches any label, tooltip or Git detail line (Wave exit; contract 8) | `add_sheet_names_no_secret` (P17); `a_supplied_passphrase_is_shown_or_kept_nowhere` and the polling privacy tests, unchanged in meaning | Planned |
| A12 | The folded-in work (Part G) | P9 (Retry); `browse_says_why_beside_its_button` (P21); `a_folder_that_is_not_text_is_named` (P22) | Planned |
| A13 | The surfaces match frames 1d and 2h, the frame `05`, and the empty pane `04`, in the dark theme | On screen: the owner | Planned |

## Product-Owner Decisions (2026-11-14)

Wave 04 decisions 1–10 (2026-11-07) and the Cycle 01, 02, 03a, 03b and 04
decisions govern and are not repeated.

1. **Plan now, execute after Cycle 04.** The owner asked to plan Cycle 05
   while Cycle 04 is in flight ("Wave 4, Cycle 4 is in flight, I'd like
   to start plannning Wave 4 Cycle 5"), and to proceed without
   brainstorming questions ("just start, do it the way Cycle 04 did").
   Task 0 does not start before Cycle 04 merges.
2. **Split on the Wave's split line.** The owner chose "Split; plan 05a
   now". The Wave's "Check first" found that neither of Wave 03 D5's
   surfaces matches the design (recorded in `01M7D7JK5MSY6J471JEKG3CZXR`).
   - This ticket is 05a: the common frame, the Add repository sheet, the
     Unlock key dialog and the empty item area.
   - 05b (`01M7D7JK5MSY6J471JEKG3CZXR`) is the Close ticket and Approve
     and merge dialog and the Resolve conflict sheet, planned after 05a's
     documents are approved.
3. **"Share changes through" (owner's eye).** The owner chose "Select
   it, then share". The Add form lists the folder's remotes, non-SSH ones
   disabled with their reason, and "Keep on this computer". After a
   successful add, the desktop sends today's remote selection, the same
   request as Settings › Repositories › Sharing. If that fails, the
   repository stays added and the failure shows in Activity with its fix.
   Create has no remotes, so the select is not shown there.
4. **No inline error for a wrong passphrase.** The owner chose "Leave
   out, known limit". Unlock closes the dialog as today, and a wrong
   passphrase fails the job as it does now. The inline error covers what
   the dialog refuses itself (an empty passphrase or one with a NUL). The
   Wave's "a wrong passphrase" exit evidence means today's journey. The
   fix stays with `01M5E6DYJ6GN7DMW4424TH3V78`.
5. **The outline** ("Write the documents"), with the constraints the
   planner stated before the questions, which the owner did not change:
   - planned against Cycle 04's approved design, executed on its merged
     code;
   - no library or CLI change; no "You can save changes here" check; the
     Unlock note does not promise "until you quit", which one cached
     passphrase per session makes untrue (`01M70CF14M3YC6SR8T9JJ6XQBG`);
   - Add and Create keep their confirmation above the sheet; the prefix
     field and Create's optional name and email stay; pressing Add on an
     unchecked folder checks it first (D1 R-T12b); background jobs never
     prompt (D6);
   - each surface keeps today's Escape rule, and the Add sheet closes on
     Escape as Settings does (decision 7);
   - the common frame's look: primary buttons amber everywhere, the
     footer's main button last, the teal focus ring, one scrim colour
     (decision 15 corrects the button order);
   - "Open repository details" and the inspect recovery open the Add
     sheet with the folder checked (decision 13);
   - the four folded-in items (Part G).

**Proposed by the planner, for confirmation at approval:**

6. **The frame's look (owner's eye).**
   - Primary buttons are amber with dark text everywhere in the app, set
     once through the Kit theme. Cycle 05b's two surfaces and every
     panel's primary buttons change with them.
   - Footer buttons are right-aligned; their order is decision 15's.
   - The Kit's focus ring and input border turn teal everywhere. The
     ring keeps the Kit's fixed 3 px at 50%, not `05`'s 2 px at 18%.
   - Sheets and dialogs share one scrim colour, `#0a0b0d99`; the Kit has
     one slot.
   - *Reason:* the Wave asks that no surface stay in the old style, and
     the Kit offers these slots once for the whole app.
7. **Escape (owner's eye).** `05` says sheets and dialogs close on Escape
   "unless an operation is running". This Cycle keeps each dialog's rule
   as it is today: Escape closes and cancels nothing, except New ticket,
   New document, New folder and Move, which keep waiting for their own
   write. The Add sheet closes on Escape as Cancel, also while a request
   runs, as Settings does (Cycle 04 decision 6): the Kit sheet cannot
   refuse it. `05` is amended to say so.
8. **Test edits beyond the named list** found by Task 0's sweep are
   pre-authorized, as in Cycle 04 decision 7.
   - Each is recorded in the ledger as an E-number past the design's list,
     with its reason, and checked by the task's reviewer.
   - An assertion removal is called out in the ledger and the review-ready
     summary.
9. **Add is enabled only when it can go ahead (owner's eye).**
   - The footer's Add repository is enabled when the folder field is
     filled and its check, once landed, did not fail, found the folder
     not yet added, found its configuration usable and found a branch
     checked out (design B10's table). Create repository is enabled when
     Name, Location and Main branch are filled (RFC 318: both show and
     require the primary branch).
   - Pressing Add on a folder not yet checked checks it first and goes
     on when the check passes (D1 R-T12b).
   - Today's "Enter the folder's path." and "Enter the primary branch."
     messages go, since the buttons are disabled instead.
   - *Reason:* `05` says "Disabled until the checks pass"; R-T12b's
     check-first keeps one press enough.
10. **The main branch is the checked-out one (owner's eye).** In Add, the
    Main branch select lists the folder's local branches. The checked-out
    one is chosen and tagged Suggested; the others are listed but cannot
    be chosen, with the reason "Check it out first. Manyhands starts from
    the branch you have checked out." Create keeps a text field, "main"
    by default, since a new repository has no branches. *Reason:* the
    library's enable takes only the checked-out branch (`RepoEnableInput`),
    and `05` and the RFC ask for a select that shows the choice.
11. **Create takes a Name and a Location (owner's eye),** as `05` draws
    it. The repository is made at Location/Name, and a line says "Empty
    folder will be created at {path}". Create keeps its optional name and
    email and its short-code prefix, and Add keeps its prefix, which `05`
    leaves out. *Reason:* behaviour unchanged.
12. **A successful add closes the sheet (owner's eye).**
    - It closes once the confirmation has closed itself.
    - Opened from Settings, Settings comes back at Repositories with the
      new repository as Settings' chosen repository (Cycle 04's
      `Manage` route); the Repositories panel's selection is unchanged.
      Otherwise focus returns to the origin.
    - The chosen remote is sent when the add succeeds, also if the sheet
      was closed while it ran, since nothing is cancelled (decision 7).
    - *Reason:* today the forms stay filled after an add
      (`01M5F3GVDYTH1Q5V5FCS9YF7V3` lists it as a defect); a sheet that
      closes leaves nothing to clear.
13. **The inspect routes land on the Add sheet (owner's eye).** "Open
    repository details" in the lifecycle dialog and the `repo.inspect`
    and `remote.history` recoveries open the Add sheet with the folder
    filled and checked, as they fill and check today's Add form. A
    registered repository reads "Already added to Manyhands" with its
    configuration's state and problems, and Add is disabled; decision 18
    adds Open its settings. *Reason:* Cycle 04 moves these routes with Add
    repository (its W-22 and W-27); the check is what they show today.
14. **The Unlock key dialog (owner's eye).**
    - A key card shows the key's label and fingerprint. The fingerprint
      comes from the key list the desktop already reads, by the key's ID;
      until it arrives, or if it is absent, the card shows the label
      only.
    - The field's label is "Passphrase", since the card names the key.
    - The note is always shown: "Kept in memory only, never written to
      disk. If you choose Not now, you can keep working and unlock later
      from Needs attention."
    - A refused passphrase shows its message under the field and puts
      focus back in the field.
    - The subtitle is shown, as today, only while a share waits.
    - There is no show and hide toggle (Wave decision 9).

**Answered after review round 1:**

15. **Button order.** The owner chose "Keep today's order". Footers are
    right-aligned and keep today's order, since several dialogs put the
    safe choice first on purpose (the quit asks, Discard a draft, the
    repair discard, Keep draft before Save). Only the Confirmation dialog
    and Unlock key put the action last, as `05` draws them (Cancel ·
    Confirm, Not now · Unlock). This corrects the outline of decision 5
    ("the footer's main button last") and settles that part of decision
    6.
16. **Typed text on close.** The owner chose "Keep it; clear after
    success". Cancel, Escape, the close button and a click outside keep
    the Add sheet's typed folder, branch, name and email for its next
    open, as Settings keeps typed text (Cycle 04 decision 13). The sheet
    empties after a successful add; quitting loses it with the app. A
    pending Add is always dropped.
17. **The default remote, and Keep on this computer.** The owner chose
    "Configured, else Keep". "Share changes through" defaults to the
    remote the folder's settings already name, else "Keep on this
    computer", so nothing is set up to share unless the person picks a
    remote. Choosing Keep on a folder whose settings name a remote sends
    today's "stop sharing" selection after the add.
18. **Open its settings.** The owner chose "Rows plus an Open settings
    action". For a registered folder the Add sheet shows "Already added",
    its configuration's state and problems, and an "Open its settings"
    action that closes the sheet and opens Settings › Repositories on that
    repository. Add stays disabled.

## Known Limits

Design K-1 to K-14:

- no "You can save changes here" check (no read);
- no inline error for a wrong passphrase (decision 4);
- one scrim colour; a dialog over a sheet draws two scrims;
- the Kit's 3 px focus ring at 50%, in teal;
- the sheet's header and footer borders stop short of its edges, since
  the Kit pads those slots;
- the Unlock note makes no "until you quit" promise
  (`01M70CF14M3YC6SR8T9JJ6XQBG`);
- "Open repository details" lands on the Add sheet's check (decision 13);
- the mode control's two options are separate Tab stops with Enter and
  Space, not arrow keys, as Cycle 02's segments are;
- no File › Add repository entry (no title-bar menus);
- a share choice waiting for its add is lost if the app quits first;
- a dialog's footer scrolls with its body when the dialog is taller than
  the window;
- Create one here is offered on any folder that is not a repository,
  though the library refuses one that is not empty;
- any of the secondary button, input background or sheet border and
  shadow the Kit theme has no key for stays the Kit's;
- a failed share selection after an add offers no new action.

## Open Questions

None blocking. Design U-1 (Cycle 04's merged names) is checked in Task 0.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-14 | Plan Cycle 05 now; execution after Cycle 04; no brainstorming questions | Owner direction | Decision 1; ticket created and published (`0698f556`) | Session, 2026-11-14 |
| Planning research | The Wave's "Check first" of D5's two surfaces; the dialog inventory and the Kit's frame APIs; Add and Create today and their library inputs; the passphrase dialog and the wrong-passphrase defect; Cycle 04's sheet owner and focus contract; lessons from Cycles 02, 03a and 03b | Facts; owner questions; rulings | Design facts; decisions 2–4; rulings | Session, 2026-11-14 |
| Owner, 2026-11-14 | Split; the remote select; the wrong passphrase; the outline | Owner decisions | Decisions 2–5; 05b ticket `01M7D7JK5MSY6J471JEKG3CZXR` created and published (`ee49a195`); this ticket renamed (`2ee459a5`) | Session, 2026-11-14 |
| Test sweep, 2026-11-14 | Footers in the Kit's slot would break focus-gated closes and privacy sweeps; several dialogs put the safe choice first on purpose | Facts; owner question | Design R-19; decision 15 | Research report, 2026-11-14 |
| Review round 1, source and feasibility (revision 1, `9e22c765`) | 1 Blocking, 9 Important, 12 Minor. Blocking: the Kit's segmented `ToggleGroup` has no keyboard path. Important: the success close races the confirmation's own close; an older attempt's finish drops the share; a repository with no commits; Keep over a configured remote; `apply_config` switches the mode and ignores `rgba`; task order; E-1's hidden removals; missed tests; P6 could not fail | Blocking, Important, Minor | Revision 2 (design Review Record) | Review report, 2026-11-14 |
| Review round 1, contract and consistency (revision 1, `9e22c765`) | 2 Blocking, 19 Important, 22 Minor. Blocking: Unlock's exit evidence unproven; nothing kept focus from being pulled behind the Add sheet. Important: enablement contradictions; Create's branch; registered folders' configuration; clearing on close; success tracking; Cycle 04's origin rows; Settings opened over the sheet; Keep; the as-built note; the mode control against Cycle 02; secondary slots; Task 3's size; P16; decision 6 against decision 5; decision 12's reading; contract 1 | Blocking, Important, Minor; owner questions | Revision 2; decisions 15–18 | Review report, 2026-11-14 |
| Owner, 2026-11-14, after review round 1 | Button order; typed text on close; the default remote and Keep; registered folders on the inspect routes | Owner decisions | Decisions 15–18 | Session, 2026-11-14 |
