---
title: "Wave 04 Cycle 04: Settings Sheet"
date: 2026-11-13
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7APBXSHYSWJ1AX3ZZ49GKH3"
---

# Wave 04 Cycle 04: Settings Sheet

Ticket `01M7AMAPNHHTNECXBF4CA5WPKV`.
[Design](../plans/2026-11-13-wave-04-cycle-04-settings-sheet-design.md) and
[implementation plan](../plans/2026-11-13-wave-04-cycle-04-settings-sheet-implementation.md).

Status: **draft**, revision 2.

- On 2026-11-13, with Cycle 03b in implementation, the owner asked to
  create this ticket and plan the Cycle. Execution waits until Cycle 03b
  lands on main (decision 1).
- The owner answered decisions 2–5 the same day, and accepted the
  planner's outline with decision 6 ("write the documents").
- Decisions 12 and 13 were answered after review round 1.
- Decisions 7–11 are the planner's proposals, awaiting confirmation.
- Those marked **(owner's eye)** change something people see today or
  depart from an authority.

Revision history:

- Drafted on the ticket branch at `0b7a8f3d`, one ticket checkpoint on
  origin/main `ba95e1b7`.
- Review round 1 read revision 1 at `65c898a5`:
  - source and feasibility: 0 Blocking, 10 Important, 9 Minor;
  - contract and consistency: 2 Blocking, 16 Important, 17 Minor.
- Revision 2 applies both, and decisions 12 and 13. **Revision 2 has not
  been independently reviewed.**
- The plan has 10 tasks (0–9). There is no split line: the Wave defines
  none for this Cycle.

## Purpose And Authority

Settings opens as a sheet, so opening it no longer hides the open item
(Wave, Cycle 04). After this Cycle:

- Ctrl+, or the footer's Settings opens a 720 px sheet over the
  workspace;
- its menu has Repositories, Sharing & keys and Checking for updates;
- every way into Settings opens the sheet at the right page and control;
- closing it puts focus back where it was;
- the item, its scroll position and any draft are untouched.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 04](../Waves/wave-04-workspace-design.md):
   - **Cycle 04:** scope, exclusions and exit evidence;
   - Owner Direction, item 5;
   - Explicit Exclusions;
   - Testing;
   - Owner Decisions 6 and 10.
3. [Desktop information architecture and editor RFC](../RFC/desktop-information-architecture-and-editor.md):
   - the Wave 04 amendment (40–55): Settings as a sheet exposing
     repositories, remotes, Git identity, shared SSH keys and host trust;
   - onboarding, remotes and key settings (315–328);
   - accessibility and keyboard (343–345).
   - The [credential RFC](../RFC/authentication-and-credential-handling.md)
     also applies: passphrases are held only for the session (145, 275).
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md): `MH-REPO-001`–`004`,
   `MH-CRED-001`, `MH-NFR-003`, `MH-NFR-005`.
5. **The design handoff:**
   - [`05-sheets-and-dialogs.md`](../design/05-sheets-and-dialogs.md):
     the common sheet frame and "Settings sheet";
   - [`02-layout-and-docking.md`](../design/02-layout-and-docking.md):
     the footer's Settings item;
   - [`06-language-and-states.md`](../design/06-language-and-states.md);
   - [`07-design-tokens.md`](../design/07-design-tokens.md);
   - [`08-gpui-kit-mapping.md`](../design/08-gpui-kit-mapping.md);
   - frames `screenshots/1e` and `1f`.
6. **Predecessors:**
   - Wave 03 D1: the settings pages and their rulings (R-B4, R-T11a,
     R-T12a, R-T12b, R-T14a, R-T15a, R-T15b, R-T16a, R-T17b);
   - D2: a typed passphrase does not outlive its page;
   - D3: R-60;
   - D6: Checking for updates, and no global pause (its decision 8);
   - Cycle 01: the footer, the sheet layer, Quit now's focus;
   - Cycle 03a: the panel's Settings entries, the repository menu;
   - Cycle 03b: Activity and the popover, which open Settings, and their
     focus rules (03b decision 23, R-49, R-50).

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`0b7a8f3d`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M7AMAPNHHTNECXBF4CA5WPKV`, open, created 2026-11-13 from main and published |
| Latest main merged into the ticket branch | Created from origin/main `ba95e1b7`. Task 0 merges main again once Cycle 03b has merged |
| Cycle 01 merged (Wave dependency) | Yes |
| Wave 03 D6 merged | Yes: main `48653d01` |
| **Cycle 03b merged (decision 1)** | **No: in implementation on `01M784Y94Q3AXT6XJX38F5VK1V`. Task 0 does not start before it merges** |
| GPUI Kit 0.6.6 names read | `Sheet`, Root's sheet calls, `Radio`, the dialog layer and GPUI's deferred dispatch, from the locked source (design facts 26, 33–39) |
| Required checks at baseline | Not run during planning; Task 0 runs them |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents in the design's Document Amendments. It changes
nothing in `src/lib.rs`, `src/repository/`, `src/results.rs`,
`src/drafts/`, `tests/` or the CLI, and adds no Cargo dependency.

**In flight.** Cycle 03b edits:

- the footer, the attention actions, the keymap,
  `D/view/workspace/focus.rs` and `render.rs`;
- several tests this Cycle also edits;
- the triggers of many routes into Settings, which become Activity rows,
  row menus and popover rows (design fact 25).

This Cycle is planned against 03b's approved design and executed on 03b's
merged code (decision 1; design U-5). No other open branch touches the
Settings, footer, content-area or keymap files (planning sweep,
2026-11-13).

## Scope

1. **The sheet** (design Part A):
   - Settings on Root's sheet layer, kept by the workspace;
   - opened by one function from every way in;
   - closed by Escape, the close button, the scrim, Show in Activity or
     quitting;
   - focus returned to what opened it;
   - nothing behind it acts or takes focus.
2. **The frame** (Part B):
   - the header with "Applies to this computer only.";
   - the three-entry menu;
   - its own scrolling body and page keys;
   - the passphrase rule.
3. **The pages** (Part C):
   - Repositories with Sharing (today's Remotes) and one chosen
     repository;
   - Sharing & keys: the key list in `05`'s layout with today's actions,
     the two key forms, the info box, Trusted servers and Check host;
   - Checking for updates, with Show in Activity closing Settings.
4. **Ways in** (Part D): all 35 triggers, Cycle 03b's included, each
   landing on its page and control with its origin. Approve host… with no
   server named uses the row's repository.
5. **Copy** (Part E) in the `06` vocabulary.
6. **What leaves** (Part F): the Settings page in the item area, Back to
   tabs, and the Repositories panel's Settings button.
7. **Folded in** (Part G), from `01M5F3GVDYTH1Q5V5FCS9YF7V3`:
   - the shared scroll;
   - a newer repository choice on reopen;
   - the list read twice;
   - the key delete focus by key;
   - the identity prefill.

   Also the planner's own fix: Repositories is read again on open.

## Explicit Exclusions And Downstream Obligations

- **Out of scope here:**
  - the General, Your name & email and Appearance pages, and new settings
    (Wave);
  - a "Pause all checking" switch (D6 decision 8);
  - the passphrase note, key dates, key algorithm and trusted dates, which
    the library does not record (decision 2);
  - the Add repository sheet, its forms' fixes from
    `01M5F3GVDYTH1Q5V5FCS9YF7V3`, and the common dialog frame (Cycle 05);
  - the other items of `01M5F3GVDYTH1Q5V5FCS9YF7V3` (decisions 3 and 12).
- **Any library, repository-format or CLI change.** A gap found in the
  headless API goes to a separate library ticket.
- **For Cycle 05.** Add repository moves out of the Repositories page into
  its own sheet, which replaces Settings until Cancel or Add (Wave
  decision 10). This Cycle's sheet owner has no case for that, so Cycle 05
  adds:
  - an owner variant for the Add repository sheet, so `sync_sheet` neither
    keeps it as Settings nor closes it;
  - emptying a typed passphrase when Add repository replaces Settings;
  - reopening Settings at Repositories after Cancel or Add, with the
    origin recorded when Settings first opened.

  The routes that move are W-5, W-6, W-22 and W-27. Cycle 05's common
  frame keeps Settings' Escape rule (decision 6; design Amendment 2).

## Required Contract

1. **A sheet, not a page.** Settings opens as a right-side 720 px sheet
   over the workspace and never replaces the item area's contents (Wave;
   `05`).
2. **The item is untouched.** Opening and closing Settings leaves the
   shown tab, its scroll position and any draft as they were (Wave exit
   evidence).
3. **The details sheet gives way.** Settings opened while an item's
   details sheet is open closes that sheet first and loses no edit (Wave
   decision 6).
4. **Focus is trapped and returns.**
   - Tab stays inside the sheet, and Escape closes it.
   - Focus returns to the control that opened it.
   - When that control is no longer drawn, the workspace's own rules
     place focus. Cycle 03b's footer rules apply first (R-49, R-50), then
     the nearest region.
   - A quiet close (quitting, Show in Activity) moves no focus back.
   - (`05`; RFC 343–345.)
5. **Every way in lands.** Each existing route into Settings opens the
   sheet at its page and control, by keyboard as well as pointer (Wave;
   design Part D).
6. **Behaviour unchanged, with named exceptions.** Every Settings control
   sends what today's sends, through the same confirmation where one
   exists. Only placement, layout and words change (Wave: "Behavior
   unchanged"), except **(owner's eye)**:
   - each open chooses a repository (R-14);
   - the Remotes page's own list goes (decision 10);
   - the key forms open from their buttons (R-18);
   - routes land on fields (decision 8);
   - Approve host… uses the row's repository (decision 9);
   - a click outside closes Settings (decision 13);
   - Drafts stays shown under the sheet (R-31);
   - the folded-in fixes (Part G).
7. **R-60 holds** (Wave 03 D3). Nothing in the workspace acts behind
   Settings, and nothing pulls focus behind it. The one exception is
   Ctrl+, behind the details sheet, which opens Settings (decision 6;
   Wave decision 6).
8. **No secret.** No new text, tooltip or accessible name names a
   passphrase, private key, remote URL or file path beyond what today's
   pages show, and no action displays or copies a private key
   (`MH-NFR-003`; RFC 321–328).
9. **A typed passphrase does not outlive the page.** It is emptied when
   Settings closes, when the page leaves Sharing & keys, and when the
   other key form is chosen (D2).

## Acceptance And Exit Evidence

Planned proof. The Status column is filled at the final task from the
branch's own tests, by exact name, and the execution ledger. "On screen"
rows await the owner's checks; Claude records only what the owner
reports.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Settings opens as a sheet from the footer and Ctrl ,, and the workspace keeps it (Wave scope; contract 1) | `settings_is_a_sheet_the_workspace_keeps` (P1); `each_route_lands_on_its_page_and_control` (P4); E-18's footer tests | Planned |
| A2 | Opening and closing Settings leaves the open item, its scroll position and any draft untouched (Wave exit; contract 2) | `settings_leaves_the_item_as_it_was` (P2) | Planned |
| A3 | Opening Settings while a details sheet is open closes it first and loses no edit (Wave exit; contract 3) | `settings_replaces_the_details_sheet_and_keeps_its_edits` (P3); E-2 | Planned |
| A4 | Focus trapped, Escape closes, focus returns to the invoking control (Wave scope; contract 4) | `focus_returns_to_what_opened_settings` (P5); `settings_closes_every_way` (P6); `focus_stays_inside_settings` (P23) | Planned |
| A5 | Every existing settings journey and recovery route passes through the sheet by keyboard (Wave exit; contract 5) | `routes_by_keyboard` (P20); `sharing_and_keys_by_keyboard` (P19); P4; E-18; the keyboard journeys in `D/view/keyboard_tests/settings.rs` and `polling_keyboard`, edited only as E-5, E-12, E-14 and E-15 say | Planned |
| A6 | The pages folded into the design's menu (Wave scope) | `settings_has_three_pages` (P16); `one_repository_is_chosen` (P10); `the_key_forms_open_from_their_buttons` (P13) | Planned |
| A7 | Behaviour unchanged, with contract 6's exceptions | `key_rows_send_what_todays_controls_sent` (P12); today's page tests pass, edited only as the named edits say | Planned |
| A8 | Settings removed from the content region and the sidebar (Wave scope) | `the_old_settings_places_are_gone` (P17) | Planned |
| A9 | Nothing acts behind Settings or pulls focus behind it; none opens behind a dialog or while quitting (contract 7) | `nothing_acts_behind_settings` (P7); `focus_stays_inside_settings` (P23); `late_work_moves_no_focus` (P22); `settings_does_not_open_behind_a_dialog_or_while_quitting` (P15) | Planned |
| A10 | Copy in the `06` vocabulary (Wave scope) | `settings_copy_uses_the_vocabulary` (P18) | Planned |
| A11 | A typed passphrase lives only on Sharing & keys (contract 9) | `a_typed_passphrase_lives_only_on_sharing_and_keys` (P9); `settings_closes_every_way` (P6) | Planned |
| A12 | Show in Activity closes Settings and focuses Activity (decision 5) | `show_in_activity_closes_settings_and_focuses_activity` (P14) | Planned |
| A13 | The folded-in work (decisions 3 and 12) | `settings_scrolls_on_its_own` (P8); `one_repository_is_chosen` (P10); `every_page_is_read_again_on_open` (P11); `removing_a_key_returns_focus_by_key` (P24); `a_slow_identity_read_fills_only_empty_fields` (P25) | Planned |
| A14 | No secret (contract 8) | `settings_copy_names_no_secret` (P21); today's Keys privacy tests and `polling_privacy` pass unchanged in meaning | Planned |
| A15 | The sheet matches frames 1e and 1f in the dark theme | On screen: the owner | Planned |

## Product-Owner Decisions (2026-11-13)

Wave 04 decisions 1–10 (2026-11-07) and the Cycle 01, 02, 03a and 03b
decisions govern and are not repeated.

1. **Plan now, execute after Cycle 03b.** The owner asked to create the
   ticket and plan this Cycle while Cycle 03b is in implementation: "Yes,
   create the ticket and start planning - execution would wait until 3b
   lands on main". Task 0 does not start before 03b merges.
2. **The key list (owner's eye):** `05`'s rows with today's actions.
   - Each row has the label, a "Made by Manyhands" or "Imported" tag, "In
     use" on the selected key, and the fingerprint.
   - The radio picks the key in use.
   - "Stop using this key" stays; the trash only removes the key from
     Manyhands; "Delete key file…" stays on generated keys.
   - The passphrase note, the dates and the algorithm are left out,
     recorded as a known limit (the library records none of them).
3. **Fold-in:** only what the move rewrites from
   `01M5F3GVDYTH1Q5V5FCS9YF7V3`. The owner chose: the shared scroll
   position, reopening ignoring a newer repository choice, and
   "Repositories not read again on reopen". Its Add repository items wait
   for Cycle 05; the rest stay in that ticket, with a note.

   *Corrected by decision 12:* the third item is not in that ticket. It
   is the planner's own finding.
4. **No footer (owner's eye),** as frames 1e and 1f draw it. The header
   holds "Settings", "Applies to this computer only." and the close
   button. The Wave's "footer" is amended.
5. **Show in Activity** closes Settings and focuses Activity, showing it
   if hidden.
6. **The outline** ("write the documents"), with the constraints the
   planner stated before the questions, which the owner did not change:
   - three pages; no General, Appearance or "Your name & email" page,
     since the Wave keeps name and email per repository on Repositories;
   - no "Pause all checking" switch (D6 decision 8);
   - Ctrl+, opens Settings over the details sheet, closing it first, and
     does nothing while Settings is open;
   - Escape always closes Settings, also while something is running;
     nothing is cancelled; pages keep their state; a typed passphrase is
     still emptied on close or page switch;
   - focus returns to what opened Settings, to a closed dialog's own
     origin, or to the Activity row (03b decision 23);
   - Settings scrolls on its own, and the page keys work in it;
   - Add repository stays on the Repositories page until Cycle 05;
   - the menu uses plain controls, as today (R-T12a).

**Proposed by the planner, awaiting confirmation:**

7. **Test edits beyond E-1 to E-18** found by Task 0's sweep are
   pre-authorized.
   - Each is recorded in the ledger as E-19 onward with its reason, and
     checked by the task's reviewer.
   - An assertion removal is called out in the ledger and the
     review-ready summary.
   - *Reason:* Cycle 03b edits several of the same tests before this Cycle
     starts, so the final list is only known at Task 0.
8. **Routes land on a field (owner's eye).** Where today a route focuses
   only the page's entry, routes that name a field now focus it:
   - Add or create… and Add (W-5, W-6), and the inspect routes (W-22,
     W-27), on the Add folder field;
   - every Set up sharing…, Share… with no remote and Open remote
     settings… route (W-9 to W-11, W-15, W-24, W-31, W-35) on the
     add-remote Name field;
   - Choose a key… and Open Sharing & keys (W-12, W-16, W-21, W-23) on
     the key list;
   - Check host routes with no server named (W-13, W-17, W-20, W-25) on
     Check host.

   *Reason:* the Wave asks for "the right page and field" (design Part D;
   R-22, R-24).
9. **Approve host… with no server named uses the row's repository
   (owner's eye)** (W-25, W-33; R-23). *Reason:* today it uses the
   Repositories panel's selection, which can be another repository. Cycle
   03b's handler test for this row changes with it (E-17).
10. **The Remotes page's own repository list goes (owner's eye).** Manage
    on the Repositories list chooses the repository for both its identity
    form and its remotes (R-14, R-15). *Reason:* two lists and two choices
    on one page would disagree (Cycle 03a's R5-2 lesson).
11. **The trash says "Remove from Manyhands. Keeps the key file." (owner's
    eye),** not `05`'s "Remove. Asks before deleting the key file.",
    which decision 2 makes untrue (R-17).

**Answered after review round 1:**

12. **Fold-in, corrected and extended.** The owner chose "Take both".
    - The planner had presented "Repositories not read again on reopen"
      as one of `01M5F3GVDYTH1Q5V5FCS9YF7V3`'s items, which it is not. It
      stays as the planner's own fix (R-32).
    - The ticket's "the first open reads the list twice" is resolved by
      decision 10.
    - Two more items in code this Cycle rewrites are taken: deleting a key
      returns focus by key, not by row position; and a slow name-and-email
      read fills only empty fields.
13. **A click outside closes Settings,** as for the details sheet. It also
    empties a typed passphrase; other typed text is kept. The owner chose
    "Click outside closes".

## Known Limits

Design K-1 to K-7:

- no passphrase note, key dates, key algorithm or trusted dates (decision
  2);
- no Pause all switch;
- `Shown::Tabs` names "Settings closed" in tests;
- the design's General, Your name & email and Appearance pages are not
  shown;
- a server whose key changed is handled by today's Check host and
  Replace…;
- the Kit sheet's top margin is 34 px against the app's 38 px title bar,
  as for the details sheet;
- a relationship picker's typed search text is not kept when Settings
  replaces the details sheet.

## Open Questions

None blocking. Design U-3 is checked on screen, and U-5 in Task 0.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-13 | Plan Cycle 04 now; execution after 03b | Owner direction | Decision 1; ticket created and published (`0b7a8f3d`) | Session, 2026-11-13 |
| Planning research | The workspace closes foreign sheets; Ctrl+, refused behind the details sheet against Wave decision 6; one shared scroll; passphrase clearing tied to the content page; the Kit's focus record when a sheet is replaced; key and host data the library lacks; the design's footer, nav and Pause all against the Wave and D6; 03b's overlap | Facts; owner questions; rulings | Design facts; decisions 2–6; rulings R-2, R-6–R-8, R-12, R-13 | Session, 2026-11-13 |
| Owner, 2026-11-13 | Key list; fold-in; footer; Show in Activity | Owner decisions | Decisions 2–5 | Session, 2026-11-13 |
| Owner, 2026-11-13 | The outline and its constraints | Owner decision | Decision 6 | Session, 2026-11-13 |
| Review round 1, source and feasibility (revision 1, `65c898a5`) | 0 Blocking, 10 Important, 9 Minor: quitting and `open_item` bypass the close; the chip origin; route focus lost to `open_sheet`; the passphrase wiped on Hosts routes; the page keys' handler; unnamed test breaks; a missing way in (W-33); `focus_content` and async refocus; test accessors | Important, Minor | Revision 2 (design Review Record) | Review report, 2026-11-13 |
| Review round 1, contract and consistency (revision 1, `65c898a5`) | 2 Blocking, 16 Important, 17 Minor. Blocking: no keyboard proof of the routes; Cycle 03b's triggers missing from Part D. Important: 03b's focus fallbacks, quit focus, the passphrase predicate, Cycle 05's dependency, Escape against `05`, interruptions, contract 6's exceptions, decision 3's attribution, privacy and focus-trap proof, scroll leak, named edits, Task 1's size, pushes, a pending details request, decisions 9 and 11 | Blocking, Important, Minor; owner questions | Revision 2: contracts 4, 6, 7, 9; A4, A5, A9, A11, A13, A14; Downstream Obligations; decisions 7–11 revised; decisions 12, 13 | Review report, 2026-11-13 |
| Owner, 2026-11-13, after review round 1 | Decision 3's attribution and two more items; the click outside | Owner decisions | Decisions 12 and 13 | Session, 2026-11-13 |
