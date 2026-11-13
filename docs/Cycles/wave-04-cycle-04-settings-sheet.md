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

Status: **draft**, revision 1, not yet independently reviewed. On
2026-11-13, with Cycle 03b in implementation, the owner asked to create
this ticket and plan the Cycle, with execution waiting until Cycle 03b
lands on main (decision 1). The owner answered decisions 2–5 the same day
and accepted the planner's outline with decision 6 ("write the
documents"). Decisions 7–11 are the planner's proposals, awaiting
confirmation. Those marked **(owner's eye)** change something people see
today or depart from an authority.

- Drafted on the ticket branch at `0b7a8f3d`, one ticket checkpoint on
  origin/main `ba95e1b7`.
- The plan has 10 tasks (0–9). If the owner splits the Cycle, 04-1 is
  Tasks 0–2 (the sheet with today's five pages) and 04-2 is Tasks 3–9.

## Purpose And Authority

Settings opens as a sheet, so opening it no longer hides the open item
(Wave, Cycle 04). After this Cycle, Ctrl+, or the footer's Settings opens
a 720 px sheet over the workspace. Its menu has Repositories, Sharing &
keys and Checking for updates. Every way into Settings opens the sheet at
the right page and control, and closing it puts focus back where it was.
The item, its scroll position and any draft are untouched.

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
   - accessibility (343–345).
   - The [credential RFC](../RFC/authentication-and-credential-handling.md)
     also applies: passphrases held only for the session (145, 275).
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
   - D6: Checking for updates, and no global pause (its decision 8);
   - Cycle 01: the footer, the sheet layer, R-60;
   - Cycle 03a: the panel's Settings entries, the repository menu;
   - Cycle 03b: Activity and the popover, which open Settings.

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`0b7a8f3d`) |
| --- | --- |
| Cycle ticket before the plan (`AGENTS.md`) | `01M7AMAPNHHTNECXBF4CA5WPKV`, open, created 2026-11-13 from main and published |
| Latest main merged into the ticket branch | Created from origin/main `ba95e1b7`. Task 0 merges main again once Cycle 03b has merged |
| Cycle 01 merged (Wave dependency) | Yes |
| Wave 03 D6 merged | Yes: main `48653d01` |
| **Cycle 03b merged (decision 1)** | **No: in implementation on `01M784Y94Q3AXT6XJX38F5VK1V`. Task 0 does not start before it merges** |
| GPUI Kit 0.6.6 names read | `Sheet`, Root's sheet calls, `Radio` and the dialog layer, from the locked source (design facts 30–37) |
| Required checks at baseline | Not run during planning; Task 0 runs them |
| Testers | Linux (Wayland): the product owner. macOS and Windows: native CI after merge |

**Dependency boundary.** This Cycle changes `src/desktop/` and its tests
only, plus the documents in the design's Document Amendments. It changes
nothing in `src/lib.rs`, `src/repository/`, `src/results.rs`,
`src/drafts/`, `tests/` or the CLI, and adds no Cargo dependency.

**In flight.** Cycle 03b edits the footer, the attention actions, the
keymap, `D/view/workspace/focus.rs` and `render.rs`, and several tests this
Cycle also edits. This Cycle is planned against 03b's approved design and
executed on 03b's merged code (decision 1; design U-5). No other open
branch touches the Settings, footer, content-area or keymap files
(planning sweep, 2026-11-13).

## Scope

1. **The sheet** (design Part A): Settings on Root's sheet layer, kept by
   the workspace, opened by one function from every way in, closed by
   Escape, the close button, the scrim, Show in Activity or quitting, with
   focus returned to what opened it.
2. **The frame** (Part B): the header with "Applies to this computer
   only.", the three-entry menu, its own scrolling body and page keys, and
   the passphrase rule.
3. **The pages** (Part C):
   - Repositories with Sharing (today's Remotes) and one chosen
     repository;
   - Sharing & keys: the key list in `05`'s layout with today's actions,
     the two key forms, the info box, Trusted servers and Check host;
   - Checking for updates, with Show in Activity closing Settings.
4. **Ways in** (Part D): all 32 triggers, each landing on its page and
   control; Approve host… with no server named uses the row's repository.
5. **Copy** (Part E) in the `06` vocabulary.
6. **What leaves** (Part F): the Settings page in the item area, Back to
   tabs, and the Repositories panel's Settings button.
7. **Folded in** (Part G) from `01M5F3GVDYTH1Q5V5FCS9YF7V3`: the shared
   scroll, a newer repository choice on reopen, Repositories read again.

## Explicit Exclusions And Downstream Obligations

- **Out of scope here:**
  - the General, Your name & email and Appearance pages, and new settings
    (Wave);
  - a "Pause all checking" switch (D6 decision 8);
  - the passphrase note, key dates, key algorithm and trusted dates, which
    the library does not record (decision 2);
  - the Add repository sheet, its forms' fixes from
    `01M5F3GVDYTH1Q5V5FCS9YF7V3`, and the common dialog frame (Cycle 05);
  - the other items of `01M5F3GVDYTH1Q5V5FCS9YF7V3` (decision 3).
- **Any library, repository-format or CLI change.** A gap found in the
  headless API goes to a separate library ticket.
- **For Cycle 05:** Add repository moves out of the Repositories page into
  its own sheet, which replaces Settings until Cancel or Add (Wave decision
  10). Its routes are W-5, W-6, W-22 and W-27.

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
4. **Focus is trapped and returns.** Tab stays inside the sheet; Escape
   closes it; focus returns to the control that opened it, or the content
   region when that control is gone (`05`; RFC 343–345).
5. **Every way in lands.** Each existing route into Settings opens the
   sheet at its page and control (Wave; design Part D).
6. **Behaviour unchanged.** Every Settings control sends what today's
   sends, through the same confirmation where one exists. Only placement,
   layout and words change, apart from the fixes design Parts D and G
   name (Wave: "Behavior unchanged").
7. **R-60 holds.** Nothing in the workspace acts behind Settings.
8. **No secret.** No new text or tooltip names a passphrase, private key,
   remote URL or file path beyond what today's pages show, and no action
   displays or copies a private key (`MH-NFR-003`; RFC 321–328).
9. **A typed passphrase does not outlive the page.** It is emptied when
   Settings closes or leaves Sharing & keys (D2).

## Acceptance And Exit Evidence

Planned proof. The Status column is filled at the final task from the
branch's own tests, by exact name, and the execution ledger. "On screen"
rows await the owner's checks; Claude records only what the owner
reports.

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Settings opens as a sheet from the footer and Ctrl ,, and the workspace keeps it (Wave scope; contract 1) | `settings_is_a_sheet_the_workspace_keeps` (P1); `each_way_in_lands_on_its_page_and_control` W-1 to W-3 (P4) | Planned |
| A2 | Opening and closing Settings leaves the open item, its scroll position and any draft untouched (Wave exit; contract 2) | `settings_leaves_the_item_as_it_was` (P2) | Planned |
| A3 | Opening Settings while a details sheet is open closes it first and loses no edit (Wave exit; contract 3) | `settings_replaces_the_details_sheet_and_keeps_its_edits` (P3); E-2 | Planned |
| A4 | Focus trapped, Escape closes, focus returns to the invoking control (Wave scope; contract 4) | `focus_returns_to_what_opened_settings` (P5); `settings_closes_every_way` (P6) | Planned |
| A5 | Every existing settings journey and recovery route passes through the sheet by keyboard (Wave exit; contract 5) | P4's table; the keyboard journeys in `D/view/keyboard_tests/settings.rs` and `polling_keyboard`, edited only as E-5 says | Planned |
| A6 | The pages folded into the design's menu (Wave scope) | `settings_has_three_pages` (P16); `one_repository_is_chosen` (P10); `the_key_forms_open_from_their_buttons` (P13) | Planned |
| A7 | Behaviour unchanged (contract 6) | `key_rows_send_what_todays_controls_sent` (P12); today's page tests pass, edited only as E-5 to E-11 say | Planned |
| A8 | Settings removed from the content region and the sidebar (Wave scope) | `the_old_settings_places_are_gone` (P17) | Planned |
| A9 | Nothing acts behind Settings; none opens behind a dialog or while quitting (contract 7) | `nothing_acts_behind_settings` (P7); `settings_does_not_open_behind_a_dialog_or_while_quitting` (P15) | Planned |
| A10 | Copy in the `06` vocabulary (Wave scope) | `settings_copy_uses_the_vocabulary` (P18) | Planned |
| A11 | A typed passphrase is emptied on close and on leaving Sharing & keys (contract 9) | `a_typed_passphrase_is_emptied_on_close_and_on_leaving_sharing_and_keys` (P9); `settings_closes_every_way` (P6) | Planned |
| A12 | Show in Activity closes Settings and focuses Activity (decision 5) | `show_in_activity_closes_settings_and_focuses_activity` (P14) | Planned |
| A13 | The folded-in work (decision 3) | `settings_scrolls_on_its_own` (P8); `one_repository_is_chosen` (P10); `every_page_is_read_again_on_open` (P11) | Planned |
| A14 | No secret (contract 8) | Today's Keys privacy tests and `polling_privacy` pass unchanged in meaning; P12 reads the new tooltips | Planned |
| A15 | The sheet matches frames 1e and 1f in the dark theme | On screen: the owner | Planned |

## Product-Owner Decisions (2026-11-13)

Wave 04 decisions 1–10 (2026-11-07) and the Cycle 01, 02, 03a and 03b
decisions govern and are not repeated.

1. **Plan now, execute after Cycle 03b.** The owner asked to create the
   ticket and plan this Cycle while Cycle 03b is in implementation:
   "Yes, create the ticket and start planning - execution would wait
   until 3b lands on main". Task 0 does not start before 03b merges.
2. **The key list (owner's eye):** `05`'s rows with today's actions. Each
   row has the label, a "Made by Manyhands" or "Imported" tag, "In use" on
   the selected key, and the fingerprint; the radio picks the key in use.
   "Stop using this key" stays; the trash only removes the key from
   Manyhands; "Delete key file…" stays on generated keys. The passphrase
   note, the dates and the algorithm are left out, recorded as a known
   limit (the library records none of them).
3. **Fold-in:** only what the move rewrites from
   `01M5F3GVDYTH1Q5V5FCS9YF7V3`: the shared scroll position, reopening
   ignoring a newer repository choice, and Repositories not read again on
   reopen. Its Add repository items wait for Cycle 05; the rest stay in
   that ticket, with a note.
4. **No footer (owner's eye),** as frames 1e and 1f draw it: the header
   holds "Settings", "Applies to this computer only." and the close
   button. The Wave's "footer" is amended.
5. **Show in Activity** closes Settings and focuses Activity, showing it
   if hidden.
6. **The outline** ("write the documents"), with the constraints the
   planner stated before the questions and the owner did not change:
   - three pages; no General, Appearance or "Your name & email" page,
     since the Wave keeps name and email per repository on Repositories;
   - no "Pause all checking" switch (D6 decision 8);
   - Ctrl+, opens Settings over the details sheet, closing it first, and
     does nothing while Settings is open;
   - Escape always closes Settings; nothing is cancelled; pages keep their
     state; a typed passphrase is still emptied on close or page switch;
   - focus returns to what opened Settings, to a closed dialog's own
     origin, or to the Activity row (03b decision 23);
   - Settings scrolls on its own, and the page keys work in it;
   - Add repository stays on the Repositories page until Cycle 05;
   - the menu uses plain controls, as today (R-T12a).

**Proposed by the planner, awaiting confirmation:**

7. **Test edits beyond E-1 to E-11** found by Task 0's sweep are
   pre-authorized, recorded in the ledger as E-12 onward with their
   reason, and checked by the task's reviewer. An assertion removal is
   called out in the ledger and the review-ready summary. *Reason:* Cycle
   03b edits several of the same tests before this Cycle starts, so the
   final list is only known at Task 0.
8. **Routes land on a field (owner's eye).** Where today a route focuses
   only the page's entry, routes that name a field now focus it: Add or
   create… and Add on the Add folder field; Set up sharing… and Open
   remote settings… on the add-remote Name field; Choose a key… and Open
   Sharing & keys on the key list; Check host routes on Check host (design
   Part D; R-22, R-24). *Reason:* the Wave asks for "the right page and
   field".
9. **Approve host… with no server named uses the row's repository**
   (W-25; R-23). *Reason:* today it uses the Repositories panel's
   selection, which can be another repository.
10. **The Remotes page's own repository list goes (owner's eye).** Manage
    on the Repositories list chooses the repository for both its identity
    form and its remotes (R-14, R-15). *Reason:* two lists and two choices
    on one page would disagree (Cycle 03a's R5-2 lesson).
11. **The generated key's trash says "Remove from Manyhands"**, not `05`'s
    "Remove. Asks before deleting the key file.", which decision 2 makes
    untrue (R-17).

## Known Limits

Design K-1 to K-6:

- no passphrase note, key dates, key algorithm or trusted dates (decision
  2);
- no Pause all switch;
- `Shown::Tabs` names "Settings closed" in tests;
- the design's General, Your name & email and Appearance pages are not
  shown;
- a server whose key changed is handled by today's Check host and
  Replace…;
- the Kit sheet's top margin is 34 px against the app's 38 px title bar,
  as for the details sheet.

## Open Questions

None blocking. Design U-1 to U-5 are checked by the tasks that first rely
on them.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-13 | Plan Cycle 04 now; execution after 03b | Owner direction | Decision 1; ticket created and published (`0b7a8f3d`) | Session, 2026-11-13 |
| Planning research | The workspace closes foreign sheets; Ctrl+, refused behind the details sheet against Wave decision 6; one shared scroll; passphrase clearing tied to the content page; the Kit's focus record when a sheet is replaced; key and host data the library lacks; the design's footer, nav and Pause all against the Wave and D6; 03b's overlap | Facts; owner questions; rulings | Design facts 1–43; decisions 2–6; rulings R-2, R-6–R-8, R-12, R-13 | Session, 2026-11-13 |
| Owner, 2026-11-13 | Key list; fold-in; footer; Show in Activity | Owner decisions | Decisions 2–5 | Session, 2026-11-13 |
| Owner, 2026-11-13 | The outline and its constraints | Owner decision | Decision 6 | Session, 2026-11-13 |
