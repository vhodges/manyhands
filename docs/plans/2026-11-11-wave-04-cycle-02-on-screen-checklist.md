# Wave 04 Cycle 02 On-Screen Checklist

This list is for checking the Cycle 02 Tickets and Documents panels (rows,
list states, the Tickets filter bar and column header, the Documents tree,
the panel toolbars in the header, unreadable files) by hand on Linux Wayland,
in the dark theme, before the Cycle is called review-ready. It is the native
evidence the design asks for (design, "Testing"; Cycle document, A14, "On
screen"), and it carries the items the execution ledger left for the screen:
the doubled `circle-x` on an unreadable document row, the two Collapse all
questions, the real drag past a toolbar button (R-37), a screen reader, and
the narrow dock (K-2). Work through sections T-1 to T-13 in order. Write each
step's result in its row's last column, and a one-line summary per section in
"Results" at the end. Claude records only what you report.

Quoted text is the app's own wording. `{repo}` stands for a repository's
display name (its directory's name), `{id}` for an item's ID. "Ctrl" is the
`secondary` key of the keymap on Linux. A key written `Ctrl Shift R` is held
together; `Shift F6` likewise. "The frames" are
`docs/design/screenshots/1a-layout-classic.png` (28 px rows, Tickets over
Documents in the right dock) and `1b-layout-docked-tabs.png` (the default
layout), read with `docs/design/03-panels.md` and `07-design-tokens.md`.

The **Supports** column names the Acceptance row (A1 to A15, Cycle document,
"Acceptance And Exit Evidence") and the design's Required Outcome (P1 to
P32) the step is evidence for. A step with no automated twin says so; a step
whose twin is only a pure function (the snapshot cannot read a tooltip or a
description, design fact 11) says "pure-function twin". Steps marked
**(rare)** need a state that is hard to make by hand: try each one, and if you
cannot reach it write "skipped". Each is also covered by an automated test.
Steps marked **Owner decides** ask you to look and say what you want; they
are not pass or fail.

## Minimum pass

If you cannot do every row, these cover each Acceptance row that has
on-screen evidence at least once. Nothing below is removed from the list; the
other rows add depth.

- **A14, both panels against frames 1a and 1b in the dark theme (the owner's
  eye, no automated twin):** T-1.5, T-2.1 to T-2.6, T-3.1, T-3.7, T-5.1,
  T-5.3, T-12.1 and T-12.2.
- **A1 and A3, the lists show the library's rows and Open is the default:**
  T-2.1, T-3.2.
- **A2, nonconforming and unreadable items stay visible with their reason:**
  T-2.5, T-3.2, T-8.2 to T-8.5.
- **A4 and A8, sort, filter, Ready, Blocked and Why blocked:** T-3.3 to
  T-3.7.
- **A5 and A9, duplicate short codes, open by ID with feedback:** T-4.3,
  T-4.5 to T-4.8.
- **A6, A11 and A12, the keyboard in both panels and the toolbars:** T-9.1 to
  T-9.14, T-6.9.
- **A7, navigation sends nothing:** T-11.1, T-11.2.
- **A10, expansion survives a reload and a hidden panel:** T-5.5, T-11.3.
- **A13, the list states act and focus finds the list:** T-7.4 to T-7.6,
  T-7.8 to T-7.11.
- **A15, unsaved work shows at once:** T-11.4, T-11.5.
- **The owner's questions (Collapse all, the doubled `circle-x`):** T-6.11,
  T-6.12, T-8.6.
- **K-2, the narrow dock, and a screen reader:** T-12.3 to T-12.5, T-13.1 to
  T-13.5.

Mostly covered by automated tests, or rare, so **optional** if time is
short: T-5.9, T-7.1 to T-7.3, T-7.12, T-8.8, T-11.7, T-12.6, T-12.7 and
T-13.7.

## Before you start

- **Use a scratch data directory for everything.** This checklist makes
  repositories, drafts, a broken repository folder and an edited registry.
  Run all of it against a throwaway data directory so your real one is never
  touched.
  - **What the app honours.** The desktop reads `XDG_DATA_HOME`, not
    `MANYHANDS_DATA_DIR`; only `manyhands-cli` reads that (Cycle 01
    checklist, "Before you start"). The scratch data directory is then
    `$SCRATCH/manyhands`.
  - **Make it** (one terminal, kept open for the whole checklist):

    ```sh
    export SCRATCH=$(mktemp -d)
    touch "$SCRATCH/.start"
    mkdir "$SCRATCH/repos"
    ```

  - **Launch with it** (from the ticket worktree,
    `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M757DHFAVPEVY4TWQTVKQAMJ`),
    the release build, under Wayland:
    `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run --release --locked --features desktop --bin manyhands`
    It needs an active desktop display. A development build lags. Run it while
    no long test suite or build is running on the machine.
  - **Check it took (T-1.1).** After the first launch,
    `ls "$SCRATCH/manyhands"` lists `layout` and the registry's files. If it
    does not, the variable did not reach the app: quit and fix that first.
  - **Your real data stays untouched.** At the end,
    `find ~/.local/share/manyhands -newer "$SCRATCH/.start"` prints nothing
    (adjust the path if your `XDG_DATA_HOME` is normally set). Do not use
    Shared SSH keys or Remotes in this checklist.
  - **Where the CLI looks** (only if you want it; nothing below needs it):
    `devenv shell -- env MANYHANDS_DATA_DIR="$SCRATCH/manyhands" cargo run --locked --bin manyhands-cli -- …`
    `… -- id new` prints a new item ID, as D5's checklist uses it.
- **Three scratch repositories**, all under `$SCRATCH/repos`, made at T-1.3:
  `scratch` (the main one, with tickets and documents), `small` (two tickets,
  for the filter and list-state steps) and `empty` (nothing in it). Make each
  with Settings (Ctrl ,) › Repositories › **Create repository**, as in D5's
  and Cycle 01's checklists: its folder `$SCRATCH/repos/{name}`, primary
  branch `main`, no remote. If the form asks for a Git name and email, give
  any. Then fill it with plain `git`, from its folder, as D5's "Making a
  ticket with plain `git`" does. The IDs below are fixed 26-character IDs
  (the form the test fixtures use), so no CLI is needed. Commits are dated in
  the past so the ages in the Upd. column differ.
  - **The helpers** (paste once into the shell you use for the repositories;
    needs GNU `date`):

    ```sh
    commit() {   # $1 = message, $2 = how long ago, e.g. "5 hours ago"
      when="$(date -d "$2" +%s) +0000"
      GIT_AUTHOR_DATE="$when" GIT_COMMITTER_DATE="$when" \
        git -c user.name=Scratch -c user.email=scratch@example.invalid commit -q -m "$1"
    }
    ticket() {   # $1 number, $2 title, $3 type, $4 status, $5 extra front matter, $6 age
      dir=".manyhands/tickets/$T$1"; mkdir -p "$dir"
      printf -- "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: %s\ntitle: '%s'\ntype: %s\nstatus: %s\n%s---\nBody of %s.\n" \
        "$T$1" "$2" "$3" "$4" "$5" "$2" > "$dir/ticket.md"
      git add "$dir" && commit "Add $2" "$6"
    }
    document() {   # $1 number, $2 path under docs/, $3 title, $4 age
      mkdir -p "$(dirname "docs/$2")"
      printf -- "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: %s\ntitle: '%s'\n---\n# %s\n" \
        "$D$1" "$3" "$3" > "docs/$2"
      git add "docs/$2" && commit "Add $3" "$4"
    }
    CLOSED=$'closed_at: 2026-11-01T12:00:00Z\nclosed_by: Scratch <scratch@example.invalid>\n'
    ```

  - **`scratch`** (`cd "$SCRATCH/repos/scratch"`): 12 tickets (10 open, 2
    closed), one nonconforming ticket, 13 documents (two of them files that
    need attention):

    ```sh
    T=01ARZ3NDEKTSV4RRFFQ69G5T; D=01ARZ3NDEKTSV4RRFFQ69G5D
    ticket 01 'Dock layout persistence across restarts' cycle   'in progress' $'slug: vh-aaa01\n' '12 minutes ago'
    ticket 02 'Footer toggles for dock panels'          defect  'in progress' $'slug: vh-aaa02\n' '38 minutes ago'
    ticket 03 'Plain-language sync states'              feature ready         $'slug: vh-aaa03\n' '2 hours ago'
    ticket 04 'Short-code search in the command bar'    task    ready         $'slug: vh-aaa04\n' '5 hours ago'
    ticket 05 'Wave 03 D4: Discussion and mockups'      cycle   'not started' $'slug: vh-aaa05\n' '1 day ago'
    ticket 06 'Upstream: expose source offsets'         task    waiting       ''                  '3 days ago'
    ticket 07 'Blocked until the dock work lands'       task    blocked       $'slug: vh-aaa07\ndeps:\n- '"${T}01"$'\n' '4 hours ago'
    ticket 08 'Status says closed but the ticket is open' task  closed        $'slug: vh-aaa08\n' '6 hours ago'
    ticket 09 'Duplicate code, the open one'            task    open          $'slug: vh-dupe1\n' '7 hours ago'
    ticket 10 'Spike: try a faster index'               spike   'in review'   $'slug: vh-spike-faster-index\n' '8 hours ago'
    ticket 11 'Closed: earlier release plan'            task    done          $'slug: vh-clsd1\n'"$CLOSED" '2 days ago'
    ticket 12 'Duplicate code, the closed one'          task    done          $'slug: vh-dupe1\n'"$CLOSED" '9 days ago'
    mkdir -p ".manyhands/tickets/${T%T}N01"
    printf -- '---\nmanyhands_managed: true\nmanyhands_kind: ticket\nrepeated: 1\nrepeated: 2\n---\nBroken.\n' > ".manyhands/tickets/${T%T}N01/ticket.md"
    git add .manyhands/tickets && commit 'Add a nonconforming ticket' '1 day ago'
    document 01 charter.md              'Project charter'                  '3 days ago'
    document 02 future-ideas.md         'Future ideas'                     '3 days ago'
    document 03 PRD/mvp.md              'MVP product requirements'         '3 days ago'
    document 04 RFC/ia.md               'Information architecture RFC'     '3 days ago'
    document 05 RFC/git.md              'Git workflow RFC'                 '3 days ago'
    document 06 RFC/relationships.md    'Ticket relationships RFC'         '3 days ago'
    document 07 RFC/waves/wave-01.md    'Wave 01: Foundations'             '3 days ago'
    document 08 RFC/waves/wave-02.md    'Wave 02: Collaboration'           '3 days ago'
    document 09 cycles/cycle-01.md      'Cycle 01: Shell'                  '3 days ago'
    document 10 cycles/cycle-02.md      'Cycle 02: Tickets and documents'  '3 days ago'
    document 11 cycles/cycle-03.md      'Cycle 03: Repositories'           '3 days ago'
    printf 'Just a note, with no front matter.\n' > docs/plain-note.md
    printf -- '---\nmanyhands_managed: true\n---\nOnly a marker.\n' > docs/marker-only.md
    git add docs && commit 'Add two files that need attention' '2 days ago'
    ```

  - **`small`** (`cd "$SCRATCH/repos/small"`): one open and one closed
    ticket, no documents:

    ```sh
    T=01ARZ3NDEKTSV4RRFFQ69G5S
    ticket 01 'Small open ticket'   task open $'slug: sm-open1\n' '1 hour ago'
    ticket 02 'Small closed ticket' task done $'slug: sm-clsd1\n'"$CLOSED" '2 days ago'
    ```

  - **`empty`**: nothing to add yet (T-7.4 starts from it as created).
  - After each commit run, press Ctrl R (Refresh) in the app with that
    repository selected, so the index sees the files. With the numbers above
    the Tickets segments read **Open 10 | Closed 2 | All 12** and the
    Documents header counts **13**.
  - **A big repository (optional, only for T-7.1 to T-7.3).** To make reads
    slower, create a fourth repository `big` and fill it with 3000 tickets in
    one commit (use 1000 if your machine is slow); then Refresh:

    ```sh
    cd "$SCRATCH/repos/big"
    for i in $(seq 1 3000); do n=$(printf %08d "$i"); d=".manyhands/tickets/01ARZ3NDEKTSV4RRFF$n"; mkdir -p "$d"
      printf -- "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: 01ARZ3NDEKTSV4RRFF$n\ntitle: 'Ticket $i'\ntype: task\nstatus: open\n---\nBody $i.\n" > "$d/ticket.md"; done
    git add .manyhands/tickets && git -c user.name=Scratch -c user.email=scratch@example.invalid commit -q -m 'Add 3000 tickets'
    ```

- **A window size.** The app opens at 1280 x 840 in the default layout (Cycle
  01, "The default layout"): Repositories over Documents in the left dock
  (290 px), Tickets alone in the right dock (440 px), Activity at the foot.
  Steps that need another layout or size say so.
- **The registry edit** (T-7.8 and T-7.9) uses `python3`, which is on the
  machine; `sqlite3` is not needed.

## Known limits you may hit (not failures)

Write down if you hit one, with the step; do not report it as a failure. The
first group is this Cycle's design "Known Limits" (K-1 to K-14); the second
is from Cycle 01; the third is the execution ledger's known gaps, which the
design does not list.

- **K-1.** Tooltips show on hover only. Their text is also each element's
  accessible name or description, and the opened item shows the ID, path and
  context. (A tooltip never appears on keyboard focus.)
- **K-2.** In a narrow dock, such as the left one at 290 px, short codes and
  status pills are clipped so the title keeps 96 px.
- **K-3.** Ages are worked out when a row is drawn, so an idle list's ages do
  not tick on their own.
- **K-4.** Unreadable files are read from the last refresh's index status;
  one made unreadable since shows after the next refresh.
- **K-5.** Folder expansion, the filters and the sort last for the session
  only; saving them with the layout is Deferred Work.
- **K-6.** The readiness menu's checked item is not exposed to accessibility;
  the button's name says the choice.
- **K-7.** The sort order is in the header button's name; there is no
  `aria_sort`.
- **K-8.** Up and Down wrap at both ends, as the Kit does.
- **K-9.** New document… still starts in `docs`, not the selected folder, as
  today.
- **K-10.** Every reload reads every ticket of the repository, closed ones
  included, and the index status read reads every stored item and pending
  operation again. (A slow reload in `big` is this.)
- **K-11.** The list and the tree are not Tab stops, as today: they are
  reached by F6, `Ctrl 2`, a click, or Enter in the filter.
- **K-12.** Short codes longer than about 11 characters are clipped in the
  Code column; the tooltip shows them whole.
- **K-13.** An unreadable ticket *directory* hides its ticket without a row
  or an incomplete alert.
- **K-14.** An unreadable row does not open.
- **Cycle 01 K-3.** Only a tabbed group's displayed tab is an F6 stop.
- **Cycle 01 K-11.** A dock's only panel cannot be dragged out of it; so
  Tickets, alone in the right dock, cannot be dragged at all (T-6.7 uses
  Documents).
- **Frames, not failures.** The frames draw things this Cycle does not
  build: a More (`ellipsis`) button in each header (decision 12), the
  repository as a root node of the Documents tree and "from {k}
  repositories" in the footnote (no multi-repository scope yet), Projects
  and Teams panels, upper-case short codes (this Cycle writes them as stored,
  lowercase, decision 3), and frame 1b's 24 px rows (Wave decision 7 gives
  28 px, as frame 1a draws).
- **Ledger, Task 7.** After a made-ready outcome's "Open ready tickets", the
  Tickets list reloads without a status read, so its unreadable-file rows
  may be missing until the next full load (Refresh, or selecting the
  repository again).
- **Ledger, Task 4.** Escape or typing leaves an older "Nothing in {repo} has
  that ID." line in place until the next Enter; clicking the already selected
  repository keeps the filter text.
- **Ledger, Task 8.** Collapse all's enabled state follows the saved folder
  state, not what is visible (T-6.12).

## T-1 Launch, scratch repositories and the first look

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-1.1 | Launch the release build with the scratch command above. Run the "Check it took" listing. | A window opens with no error in the terminal; the title is "Manyhands" (no repository selected); `$SCRATCH/manyhands` lists `layout`. Default layout: Repositories over Documents (left), Tickets (right), Activity (foot). | A14 (no automated twin) | |
| T-1.2 | Before any repository exists, look at Tickets and Documents. Hover each header toolbar button and try to click it. | Tickets: the filter bar is drawn (the Open, Closed and All segments with no counts, the readiness button, the input with the placeholder "Filter by title or code"), then "Select a repository."; there is no column header and no footnote; the header shows no count. Documents: "Select a repository.". The header buttons (`plus` in Tickets; `file-plus`, `folder-plus`, `chevrons-down-up` in Documents) look dimmed and a click does nothing. | A11, P24 (automated twin: `panel_toolbars_work_by_tab_and_enter`) | |
| T-1.3 | Click into the filter input, type `x`, press Enter. | The line under the filter bar says "Select an available repository to find in". Escape clears the text. | A9, P8 (twin: `enter_needs_a_selected_repository`) | |
| T-1.4 | Make the three repositories (and `big` if you want it) as in "Before you start", fill `scratch` and `small`, and press Ctrl R with each selected. Leave `empty` as created. | Each is in the Repositories panel. Selecting `scratch` makes the window title "Manyhands — scratch". | A1 (no automated twin) | |
| T-1.5 | Select `scratch`. Look at both panels at once, against frame 1b: Tickets on the right (440 px), Documents under Repositories on the left. | Tickets: the header "Tickets" with the count 11 (10 open tickets and the one entry that needs fixing); the filter bar; the column header; rows; the footnote "10 open tickets in scratch · 1 needs fixing". Documents: the header "Documents" with the count 13; a tree whose folders all start open. No second line under any row. The overall look is the frame's, with the "Frames, not failures" differences. | A14, A1 (appearance: no automated twin) | |
| T-1.6 | Look at the colours. | Dark theme: chrome (headers, filter bar, footnote) darker than the panel body, thin borders between regions, nothing unreadable. Amber is used for the selected row's left edge, markers and "needs fixing"; teal for the focus ring and "in progress"; red for blocked and "can't be read". | A14 (no automated twin) | |

## T-2 Tickets rows against the frames (dark theme)

Select `scratch`, Open segment, nothing typed, nothing selected. The right
dock is 440 px wide.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-2.1 | Look at the list as first drawn. | Open is the chosen segment (A3). Ten rows in this order, newest first: Dock layout persistence across restarts, Footer toggles for dock panels, Plain-language sync states, Blocked until the dock work lands, Short-code search in the command bar, Status says closed but the ticket is open, Duplicate code, the open one, Spike: try a faster index, Wave 03 D4: Discussion and mockups, Upstream: expose source offsets. The nonconforming entry follows them, last. The Upd. column reads about `12m`, `38m`, `2h`, `4h`, `5h`, `6h`, `7h`, `8h`, `1d`, `3d` (a little more if you waited). | A1, A3, P3 (twin: `derive_filters_counts_and_sorts`) | |
| T-2.2 | Compare one row with a frame-1a row, cell by cell, left to right. | One 28 px line: a 14 px type icon (cycle `repeat`, defect `bug`, feature `sparkles`, task `list-checks`, the `spike` ticket a plain `ticket`); the short code in a small mono font, lowercase, `secondary`, such as `vh-aaa01`; the title with an ellipsis when it is long; the status pill; the age at the right, `muted`. "Upstream: expose source offsets" shows `—` where the code goes. | A14 (appearance: no automated twin) | |
| T-2.3 | Look at the pills. | Each pill's text is the status as written. In progress and open: teal. Ready and closed: green. Blocked: red. In review: amber. Not started and waiting: neutral grey. The "Status says closed but the ticket is open" row is listed under Open and its pill reads `closed`. | A14, P4 (pure-function twin: `statuses_tint_by_their_words`) | |
| T-2.4 | **Owner decides.** Look at the pill's corners against frame 1a. | The frames and `03-panels.md` draw the pill with a 9 px radius (`07`: "9 (status pills)"); the code draws it with a 4 px radius and 6 px side padding. Say which you want. | A14 (appearance: no automated twin) | |
| T-2.5 | Look at the last row, the nonconforming entry. | An amber `triangle-alert` where the type icon goes; the title cell shows its path (`.manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5N01/ticket.md`); no short code; the pill "needs fixing" in amber. Click it: its tab opens. | A2, P6, P23 (twin: `derive_filters_counts_and_sorts`) | |
| T-2.6 | Hover a row. Click another row. | Hover: a lighter background. Click: that ticket opens in the item area, and the row is selected: the `selection` background, a 2 px amber line at its left edge, and the title in a heavier weight. The row you did not click is unchanged. | A14, R-7 (appearance: no automated twin) | |
| T-2.7 | With a row selected, press Ctrl 2. | Focus is in the Tickets list: the selected row also draws a 2 px teal ring inside its edge, and the Tickets region draws its ring. Press Ctrl 3, then Ctrl 2 again: the same. | A14, A6 (no automated twin for the ring: appearance) | |
| T-2.8 | Choose the All segment. Look at "Closed: earlier release plan" and "Duplicate code, the closed one". | Both rows are listed, their titles in the dimmer `secondary` colour, each with a `circle-check` marker after the title. Under Closed, the same titles are dim but carry no marker. Under Open, the other rows have no marker. | P5 (twin: `markers_follow_drafts_conflicts_and_closure`; the colour is appearance) | |
| T-2.9 | Choose Open. Select "Wave 03 D4: Discussion and mockups". Shrink the window's height until the list scrolls, then scroll it. | The filter bar and the column header stay in place; only the rows move; the footnote stays at the foot. Selection stays on its row. Restore the window height. | A14 (no automated twin) | |

## T-3 Filter bar, counts, readiness, sort, footnote and Why blocked

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-3.1 | Look at the filter bar. | A segmented control reading Open 10, Closed 2, All 12 (three joined segments) with each count muted and the chosen segment on the `border` colour with `text`; then a `sliders-horizontal` icon button; then the input (24 px high, a `list-filter` icon at its left, the placeholder "Filter by title or code"). Open 10 + Closed 2 = All 12. | A14, P10 (twin: `the_filter_bar_works_without_a_read`) | |
| T-3.2 | Click Closed, then All, then Open again. Read the header count and the footnote each time. | Closed: the two closed tickets and the nonconforming entry; header 3; footnote "2 closed tickets in scratch · 1 needs fixing". All: all twelve and the entry; header 13; "12 tickets in scratch · 1 needs fixing". Open: header 11; "10 open tickets in scratch · 1 needs fixing". The entry is listed under every choice. | A1, A2, P2, P6, P29 (twin: `the_filter_bar_works_without_a_read`) | |
| T-3.3 | Click the readiness button. | A menu with "Any readiness", "Ready" and "Blocked", with the current one checked. Choose Blocked: the button shows its icon and the word "Blocked"; the list is "Blocked until the dock work lands" and the entry; footnote "1 blocked ticket in scratch · 1 needs fixing"; under the list a Why blocked panel reads "Why blocked" and "Select a ticket to see why it is blocked". The segment counts do not change. | A8, P11 (twin: `the_filter_bar_works_without_a_read`) | |
| T-3.4 | Click the blocked ticket's row. | The panel lists "Waits for open ticket: vh-aaa01 Dock layout persistence across restarts". (Its ticket tab opens too.) | A8, P11 | |
| T-3.5 | Choose Ready in the menu, then Any readiness. | Ready: nine tickets and the entry; footnote "9 ready tickets in scratch · 1 needs fixing"; no Why blocked panel. Any readiness: back to ten tickets; the button shows its icon only. | A8, P11 | |
| T-3.6 | Choose Closed, then choose Ready in the menu. Then choose Blocked and click Closed. | Ready while Closed is chosen switches the segment to Open. Closed while Blocked is chosen sets Any readiness. | P11 (R-26; twin: `the_filter_bar_works_without_a_read`) | |
| T-3.7 | Click the TITLE column header; click it again. Then CODE (twice), STATUS, the type arrow at the far left, then UPD. (twice). | First click: ascending, an `arrow-up` on TITLE in the `text` colour (the other headings muted, with a faint `arrow-up-down`); second: `arrow-down`, reversed. The nonconforming entry stays last in every order. Under CODE the ticket with no code stays last whichever way it sorts. UPD. starts as `arrow-up` (newest first) and the second click gives oldest first. | A4, P12 (twin: `the_filter_bar_works_without_a_read`) | |
| T-3.8 | Type `dock` in the input. | The list narrows as you type: three tickets (Dock layout persistence across restarts, Footer toggles for dock panels, Blocked until the dock work lands) and the entry. The segments read Open 3, Closed 0, All 3; header 4; footnote `3 open tickets in scratch match "dock" · 1 needs fixing`. Escape clears the text and everything returns. | A4, P2, P10 | |
| T-3.9 | Select "Wave 03 D4: Discussion and mockups". Type `dock`. Press Ctrl 2, then Enter. | The selected row is hidden by the filter; nothing is selected; Enter opens nothing new. Escape in the input, to clear. | P30 (twin: `the_filter_bar_works_without_a_read`) | |
| T-3.10 | Set Blocked and TITLE descending, type `x`. Select `small` in Repositories, then `scratch` again. | On `small` the input is empty (the text is cleared on a change of repository); Blocked and the TITLE descending order are kept. Set Any readiness and UPD. again. | P17 (twin: `the_text_is_cleared_once_per_change_of_repository`) | |
| T-3.11 | Choose the Closed segment and a sort, then press Ctrl R and wait. | The segment, readiness and sort are as you left them; nothing is cleared by a refresh of the same repository. | P17 | |

## T-4 The filter input as Find (open by short code and by ID)

Open segment, Any readiness, nothing typed, `scratch` selected.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-4.1 | Click in the item area, then press Ctrl P. Then hide Tickets with its footer toggle and press Ctrl P again. | The caret is in the filter input, with the Kit's focus ring. With Tickets hidden, Ctrl P shows it and focuses the input. | A9, A6 (twin: `secondary_p_focuses_the_filter_input`) | |
| T-4.2 | Type `vh-aaa0`. | Seven tickets (the ones with codes `vh-aaa01` to `vh-aaa08`, less the one with no code) and the entry; Open 7. A small clear control shows in the input; click it: the text goes. | A4, P2 | |
| T-4.3 | Type `vh-dupe1`. | One ticket (Duplicate code, the open one) and the entry; Open 1, Closed 1, All 2; footnote `1 open ticket in scratch matches "vh-dupe1" · 1 needs fixing`; above the rows a line "1 more ticket matches with other filters." with a **Show all** button. | A5, P7 (twin: `derive_filters_counts_and_sorts`) | |
| T-4.4 | Press Show all. | The All segment is chosen, readiness Any; both tickets with `vh-dupe1` are listed; footnote `2 tickets in scratch match "vh-dupe1" · 1 needs fixing`; focus is in the filter input. Click each of the two rows: each opens its own ticket (one is closed), and each tab is for a different ticket. | A5, P7, R-28 (twin: `list_states_act_and_a_reload_keeps_the_rows`) | |
| T-4.5 | With the two matches listed, press Enter in the input. | Focus moves to the list with the first row selected; nothing opens. | P9 (twin: `several_matches_move_focus_to_the_list_on_the_first`) | |
| T-4.6 | Choose Open. Replace the text with `vh-clsd1`. Look, then press Enter. | Before Enter: only the entry is listed, with the hint "1 more ticket matches with other filters.". Enter opens "Closed: earlier release plan" (it is closed and the filter hides it); focus stays in the input. | A9, P9 | |
| T-4.7 | Paste `01ARZ3NDEKTSV4RRFFQ69G5T12` (the closed duplicate's ID) and press Enter, with Open chosen. Do the same with `01ARZ3NDEKTSV4RRFFQ69G5D01`. | The line under the filter bar may flash "Opening…"; then the ticket opens, closed or not, whatever the segment; the document ID opens "Project charter". Focus stays in the input each time. | A9, P8 (twin: `a_ulid_opens_its_item_whatever_the_filters`) | |
| T-4.8 | Paste `01ARZ3NDEKTSV4RRFFQ69G5ZZ9` and press Enter. Press Enter again. | "Nothing in scratch has that ID." and nothing opens; the second Enter asks again and says the same. | A9, P8 | |
| T-4.9 | Type `abc` in the input and press Home, then End. Look at the list's selection. | In the input, Home and End move the caret (they are the input's); the list's selection does not move. Only with the list focused do they select the first and last row (T-9.8). Clear the text. | P18 | |
| T-4.10 | Open New ticket… (header button) and, with its dialog open, press Ctrl P. Cancel the dialog. | While the dialog is open, Ctrl P does nothing. | A11 (Cycle contract 8; twin: `find_does_nothing_while_a_dialog_is_open`) | |

## T-5 The Documents tree against the frames

The left dock is 290 px wide, Documents under Repositories. `scratch`
selected.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-5.1 | Look at the tree against frame 1b's Documents panel. | Folders first, all open (a first look has every folder open, unlike the frame): `cycles` (three documents), `PRD` (one), `RFC` (with a nested `waves`), then the documents by path: "Project charter", "Future ideas", and the two files that need attention by their file names (`marker-only.md`, `plain-note.md`) in amber. Documents are named by title, not file name. There is no repository root node. | A14 (appearance: no automated twin) | |
| T-5.2 | Look at one row closely. | One 28 px line: indent 8 px plus 14 px per level; a 12 px chevron (`chevron-down` on an open folder, a blank spacer on a document); a 14 px icon (`folder-open` open folder, `file-text` document, `file-exclamation-point` amber for a file that needs attention); the label; then, for the two files that need attention, a trailing amber `triangle-alert`. The header reads "Documents" with the count 13. | A14 (appearance), P23 | |
| T-5.3 | Click `RFC`'s row. | It collapses: `chevron-right`, the icon `folder`, and a muted `5` at the right (its descendant documents, nested ones included). Click it again: it opens and the `5` goes. Collapse `PRD` (shows `1`) and `cycles` (`3`). | A14, P21, P22 (twin: `rows_read_as_the_design_writes_them`) | |
| T-5.4 | Select a folder row and press Left, Right, Enter and Enter again. Then select "Project charter" and press Enter. | Left collapses, Right expands, Enter toggles (a click does the same), Enter on a document opens it. Up and Down wrap at both ends (K-8). | A6, P21 (twin: `folders_keep_their_state_and_toggle_by_key`) | |
| T-5.5 | Collapse `RFC`. Press Ctrl R and wait for the job to end. Then hide Documents with its footer toggle and show it again. | `RFC` is still collapsed after the reload and after the hidden panel; the selection is where you left it; the other folders are as you left them. | A10, P19, P26 (twin: `expansion_survives_rebuilds_and_collapse_all`) | |
| T-5.6 | Open `RFC` again (click it) and select a document inside its folder `waves`. Press Ctrl R. | After the reload the same document is still selected. | A10 (R-27; twin: `a_refresh_keeps_the_selected_document`) | |
| T-5.7 | Select the collapsed `cycles` folder, press New folder… (the `folder-plus` header button), type `archive`, Create. | The dialog's parent is the selected folder. After it closes, `cycles` is open and shows `archive` with the muted word "empty" where a count would be, and no chevron. | R-27, P22 (twin: `new_folder_creates_and_explains_local_folders_and_lists_it`) | |
| T-5.8 | Click the `archive` row and press Enter on it. | Nothing toggles or opens: an empty folder cannot toggle. | P21, P22 (twin: `folders_keep_their_state_and_toggle_by_key`) | |
| T-5.9 | Collapse `RFC`. Open "Future ideas" (click it), press its Move… button, and move it to `docs/RFC/future-ideas.md` (the dialog's own wording is not checked). | After the move `RFC` is open and shows the moved document; its selection follows. | R-27 (twin: `move_from_the_sheet_commits_both_paths_and_the_tab_follows`) | |
| T-5.10 | Click `plain-note.md`, then `marker-only.md`. | Each opens its tab with its own banner (wording not checked): the first offers to adopt it; the second says what needs fixing. | A2, P23 | |
| T-5.11 | Shrink the window's height so the tree scrolls. | A scrollbar shows at the tree's right edge (as in frame 1a); the header and toolbar stay. Restore the window. | A14 (no automated twin) | |

## T-6 The panel toolbars in the header

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-6.1 | Look at both headers against frame 1a. | Tickets: icon, "Tickets", the count, and at the right one `plus` button. Documents: icon, "Documents", the count, and at the right `file-plus`, `folder-plus` and `chevrons-down-up` buttons. Each button is 26 x 24 px with a 14 px grey icon. The frames also draw a More button: this Cycle does not (decision 12). | A14 (appearance), A11 | |
| T-6.2 | Hover each button. | A lighter background and a lighter icon. The cursor is a pointer. | A14 (appearance) | |
| T-6.3 | Click `plus` in the Tickets header. Cancel (Escape). | The "New ticket" dialog opens; after Cancel, focus goes back to where it was. | A11, P24 (twin: `panel_toolbars_work_by_tab_and_enter`) | |
| T-6.4 | Click `file-plus` in the Documents header. Look at its Folder field with `RFC` selected in the tree. Cancel. | The "New document" dialog opens with Title, Folder and File name; the Folder starts as `docs` whatever folder is selected (K-9). | A11, P24 | |
| T-6.5 | Select `RFC`, click `folder-plus`, type `x`, look, Cancel. | The "New folder" dialog, with a "Folder name" and a note that "A new folder stays on this computer until a document is saved in it."; the path it makes shows as you type, below the selected folder. | A11 | |
| T-6.6 | Click `chevrons-down-up` with some folders open. | Every folder that has contents collapses; the button now looks dimmed; keyboard focus is in the tree (the Documents region has its ring), not on the dimmed button. If the selected row was inside a collapsed folder, the top-level folder above it is selected. | A11, P20 (twin: `folders_keep_their_state_and_toggle_by_key`) | |
| T-6.7 | In the left dock, press on the `folder-plus` button, move the pointer about 30 px away while holding, and release outside the button. Then drag the word "Documents" in its header onto Tickets' header (or the right dock), and release. | The button press starts no drag: no drag preview, the panel stays, no dialog opens if you released outside the button. Dragging the title does drag the panel (positive control; a drop indicator shows). Undo with the dock menu (right-click its footer toggle) or by dragging it back. Tickets cannot be the dragged panel here: it is alone in its dock (Cycle 01 K-11). | A11, P24, R-37 (twin: a press and small move drags nothing; no automated twin for a real drag) | |
| T-6.8 | Drag Documents onto Tickets' header so they form a tabbed group. Look at the strip, then click the Tickets tab, then the Documents tab. | The strip has the tabs "Tickets" and "Documents"; the toolbar of the **active** tab only shows at the strip's right end: `plus` under Tickets, the three Documents buttons under Documents (the group you just made has Documents active). A toolbar button is clickable and starts no drag. | A11, P24 (twin: `panel_toolbars_work_by_tab_and_enter`, tabbed phase) | |
| T-6.9 | Open the New ticket dialog. With it open, click the Documents toolbar buttons and press Ctrl N. | Nothing acts behind the dialog: no second dialog, no collapse. Cancel the dialog. | A11, P24 (twin: `panel_toolbars_work_by_tab_and_enter`, dialog phase) | |
| T-6.10 | After the moves in T-6.7 and T-6.8, press each Documents button and the Tickets button again. | They still work after a panel was dragged and docked elsewhere (the buttons keep their keyboard focus through a dock move). | A11, R-37 (twin: `panel_toolbars_work_by_tab_and_enter`) | |
| T-6.11 | **Owner decides: Collapse all and empty folders.** Select `scratch`, make sure at least one other folder is open, and make an empty folder `drawer` with `folder-plus` (if not there). Press Collapse all. Note `drawer`. Then make a document in it: New document…, set the Folder to `docs/drawer`, Create. | Note what you see: `drawer` stays as it was when Collapse all ran (an empty folder has no chevron and never collapsed); when the document arrives, `drawer` shows open with its document. Cycle decision 13 reads "Collapse all collapses every folder"; this build collapses only folders that have contents, so an empty folder is not left collapsed. Say whether you want that, or an empty folder collapsed after Collapse all. | P20, decision 13 (twin: `expansion_survives_rebuilds_and_collapse_all`; the wording difference is the ledger's flag for you) | |
| T-6.12 | **Owner decides: when Collapse all is enabled.** Click `RFC` and then its folder `waves` until both are open, then collapse `RFC` by hand (click its row). Collapse by hand any other folder that is open (`drawer` is, now it has a document), so every folder you can see is collapsed, with `waves` still open inside the collapsed `RFC`. Look at `chevrons-down-up`. Press it. | Note what you see: the button is **still enabled**, because `waves` is still marked open in the saved folder state, though nothing visible is open. Pressing it changes nothing you can see, then it dims. Say whether the button should dim when nothing visible is open. | decision 13, P20 (ledger, Task 8: a known gap; twin: none for the visible case) | |
| T-6.13 | Press Ctrl R and look at the Activity panel. | Collapse all and every toolbar press sent nothing: Activity shows only the refresh you just asked for. | A7, P25 | |

## T-7 Every list state

Alerts show above the rows (or alone) with one action. A list stays drawn
under every state (R-34). Retry, Refresh and Rebuild move focus to the panel's
own list or tree (R-28).

Make the states with these (the shell helpers from "Before you start" must be
loaded for the third). **Stale and never refreshed** edit the scratch
registry while the app is **quit**; each must print `1`, or stop: the path or
the column differs and nothing was changed:

```sh
# stale (T-7.8)
python3 -I - <<'EOF'
import os, sqlite3
c = sqlite3.connect(os.environ["SCRATCH"] + "/manyhands/manyhands.sqlite3")
print(c.execute("UPDATE repositories SET refresh_required = 1 WHERE root_path LIKE '%/scratch'").rowcount)
c.commit()
EOF
# never refreshed (T-7.9)
python3 -I - <<'EOF'
import os, sqlite3
c = sqlite3.connect(os.environ["SCRATCH"] + "/manyhands/manyhands.sqlite3")
print(c.execute("UPDATE repositories SET refreshed_at = NULL WHERE root_path LIKE '%/small'").rowcount)
c.execute("DELETE FROM contexts WHERE repository_id IN (SELECT id FROM repositories WHERE root_path LIKE '%/small')")
c.commit()
EOF
# nonconforming-only (T-7.7), from the folder of `empty`
cd "$SCRATCH/repos/empty"; T=01ARZ3NDEKTSV4RRFFQ69G5
mkdir -p ".manyhands/tickets/${T}N02"
printf -- '---\nmanyhands_managed: true\nmanyhands_kind: ticket\nrepeated: 1\nrepeated: 2\n---\nBroken.\n' > ".manyhands/tickets/${T}N02/ticket.md"
git add .manyhands && commit 'Add a nonconforming ticket' '1 hour ago'
```

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-7.1 | **(rare) First load.** Select `big` from another repository, and look at Tickets and Documents the instant they appear. | Six grey static bars (a skeleton) where rows will be, with no animation and no count in the header; read by a screen reader as "Loading tickets" and "Loading documents". On a small repository this is too fast to see: if you cannot catch it write "skipped". | A13, P15, P16 (twin: `the_tree_says_each_list_state_and_acts_on_it`) | |
| T-7.2 | **(rare) A reload over kept rows.** With `big` selected, press Ctrl R and watch the header. | While the lists reload, the rows stay and the header count reads "Updating…"; it goes back to a count when the reload lands. If you cannot catch it write "skipped". | A13, P16 (twin: `list_states_act_and_a_reload_keeps_the_rows`) | |
| T-7.3 | **(rare) Busy.** Over kept rows: "{repo} was busy, so this list wasn't updated." with Retry. With nothing kept: "{repo} is busy right now." with Retry. Make it by running `manyhands-cli index rebuild` on `big` in a second terminal (it needs `--repo`; see its `--help` for the rest) while the app reloads it (Ctrl R). | One of the two alerts, with Retry. If you cannot make it write "skipped". | A13, P15, P16 (twin: `the_tree_says_each_list_state_and_acts_on_it`) | |
| T-7.4 | Select `empty`. Look at Tickets (Open, then Closed, then All) and Documents. | Open: "No open tickets in empty." with a **New ticket…** button; Closed: "No closed tickets in empty." and no button; All: "No tickets in empty yet." with **New ticket…**. Documents: "No documents in empty yet." with **New document…**. Click each button: its dialog opens (Cancel it). | A13, P15 (twin: `list_states_act_and_a_reload_keeps_the_rows`) | |
| T-7.5 | Select `small`, Open. Type `zzz`. Press Clear filters. | "No tickets match these filters." with **Clear filters**; pressing it empties the text, chooses Any readiness, and puts focus in the filter input. | A13, P15 (twin: `list_states_act_and_a_reload_keeps_the_rows`) | |
| T-7.6 | Still in `small`, Open: type `small closed`. Press Show all. | "No tickets here match. 1 more ticket matches with other filters." with **Show all**; pressing it chooses All and Any readiness, lists the closed ticket, and focus is in the filter input. | A13, P7, P15 | |
| T-7.7 | In `empty`, run the "nonconforming-only" snippet above and press Ctrl R. Then in Documents make a folder `drawer` with New folder…. Look at Collapse all. | Tickets shows the one "needs fixing" row and the footnote "0 open tickets in empty · 1 needs fixing", **not** "No open tickets in empty.". Documents shows `drawer` with "empty", not "No documents in empty yet.": a list with only such rows is never the empty state. `chevrons-down-up` is dimmed (a folder with nothing in it has nothing to collapse). | A2, A11, P27 (twin: `each_state_has_its_own_body`, `expansion_survives_rebuilds_and_collapse_all`) | |
| T-7.8 | Stale. Quit the app (Ctrl Q), run the "stale" snippet above (it must print `1`), start the app, select `scratch`. Press the alert's **Refresh**. | Above the rows: "The list for scratch may be out of date." with **Refresh** (Documents shows the same line). Pressing it submits a refresh (a job shows in Activity), keyboard focus goes to the Tickets list, and the alert goes when the job ends. If the app refreshed it by itself and no alert shows, write that. | A13, P15, R-28 (twin: `list_states_act_and_a_reload_keeps_the_rows`, registry SQL) | |
| T-7.9 | Never refreshed. Quit, run the "never refreshed" snippet above (it must print `1`), start, select `small`. Press **Refresh**. | "small hasn't been scanned yet, so this list may be empty." with **Refresh**, in both panels; the list under it is empty. Pressing it refreshes; the two tickets appear and the alert goes. | A13, P15 (twin: registry SQL, as T-7.8) | |
| T-7.10 | Failed. With `scratch` selected and its lists showing, rename its folder: `mv "$SCRATCH/repos/scratch" "$SCRATCH/repos/scratch.away"`. Press Ctrl R. | Tickets: "Couldn't read the tickets in scratch. Nothing was changed." with a second line (the library's message, such as "The repository cannot be read.") and **Retry**; Documents: "Couldn't read the documents in scratch. Nothing was changed." with the second line and **Retry**. If the panels say something else (for example "Select a repository.") or the Repositories row changes, write down what you saw. | A13, P15, P16 (twin: `list_states_act_and_a_reload_keeps_the_rows`, failed read) | |
| T-7.11 | Press Ctrl 2, then F6 and Shift F6, with the failed list showing. Put the folder back (`mv` it back) and press **Retry**. | The Tickets region holds focus and draws its ring even though its list is empty (the list stays drawn). After Retry the rows are back, the filter text and the sort you had are kept, and focus is in the list (Documents: the tree). | A13, P15, P31 (twin: `list_states_act_and_a_reload_keeps_the_rows`) | |
| T-7.12 | **(rare) Index unavailable.** "The search index for {repo} can't be read. Nothing was changed." with **Rebuild the search index**. By hand this needs the registry file unreadable while the app runs, which risks more than the lists; try it only if you wish, on the scratch directory only, and write "skipped" if the app does anything but show this alert. Pressing Rebuild should submit a rebuild and focus the list. | The alert and button as quoted, or "skipped". | A13, P15 (twin: `the_tree_says_each_list_state_and_acts_on_it`) | |
| T-7.13 | Press Ctrl R and Ctrl Shift R in turn with a repository selected. | Refresh and Rebuild each submit a job (Activity) and leave the filters and folders as they were. | P15 | |

## T-8 Unreadable files

Select `scratch`, Open segment. These steps add files the library cannot read
(invalid UTF-8), then remove them. They are uncommitted, as the automated test
makes them. From the repository's folder:

```sh
cd "$SCRATCH/repos/scratch"
mkdir -p .manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FM0
printf '\xff\xfe\x00' > .manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FM0/ticket.md
printf '\xff\xfe\x00' > docs/broken.md
```

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-8.1 | Make the files above. Look at the lists without refreshing. | No change yet: the lists show what the last refresh saw (K-4). | A2 (K-4) | |
| T-8.2 | Press Ctrl R and wait. Look at Tickets under Open. | A row after the "needs fixing" one, labelled with its path `.manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FM0/ticket.md`, a red `circle-x` where the type icon goes, no code, and the red pill "can't be read". It stays under Closed and All. The footnote ends "· 1 can't be read" (after "· 1 needs fixing"); the header count is one higher than before (12 if you made no ticket). | A2, P6, P28 (twin: `an_unreadable_file_is_listed_with_its_reason`) | |
| T-8.3 | Click that row, then press Enter on it with the list focused. Select a normal ticket and then press Down to the unreadable row. | It is selected (the selected treatment) but no tab opens, and Enter opens nothing: such a row never opens (K-14). | A2, P28, R-38 (twin: `an_unreadable_file_is_listed_with_its_reason`) | |
| T-8.4 | Look at Documents. | `broken.md` is listed among the documents by file name in red, with a red `circle-x` icon at its left; the header count is one higher than before the file was there. | A2, P28 | |
| T-8.5 | Click `broken.md`, then Enter on it. | Selected; nothing opens. | A2, P28, R-38 | |
| T-8.6 | **Owner decides: the doubled `circle-x`.** Look at the `broken.md` row. | Note what you see: the row has a red `circle-x` as its leading icon and again as a trailing mark at its right (the design table asks for both). Does it look doubled? Say whether you want the trailing one kept, dropped, or changed. | A14 (ledger, Task 7: the owner's item; no automated twin) | |
| T-8.7 | Hover both unreadable rows now (T-10.5 gives the text to expect), then remove the files and press Ctrl R: `rm -r .manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FM0 docs/broken.md`. | The two rows go after the refresh; the footnote and the counts return to their earlier values. | A2, P28 | |
| T-8.8 | **(rare) An unreadable ticket directory (K-13).** Skip as root. `mkdir .manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FM1 && chmod 000 .manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FM1`, press Ctrl R. Afterwards `chmod 700` and `rmdir` it. | No row and no incomplete alert for it: an unreadable ticket directory hides its ticket without a row (K-13). | A2 (K-13) | |

## T-9 The keyboard, with focus visible

Use no pointer in this section. `scratch` selected, Open, Any readiness,
nothing typed. Every stop must show a ring.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-9.1 | Press Ctrl 2 to focus the Tickets list, then press Shift Tab repeatedly until focus is on the header's `plus` button. Write down the order of the stops you cross. | Going backwards from the list: the five column headers (UPD., STATUS, TITLE, CODE, the type arrow), the hint's Show all when a hint is showing, the filter input, the readiness button, the All, Closed and Open segments, then the header's New ticket… button. Each shows a ring. (The input's clear control may also be a stop: write down.) | A6, A11 (twin: `change_the_filters` and `every_core_action_works_by_keyboard_alone`; the ring is appearance) | |
| T-9.2 | Press Tab forward through the same stops. Then press Tab on from UPD. | The same order forward. After the last column header, focus does not stop on the list (K-11); it goes on to the next region's stop. | A6 | |
| T-9.3 | On the New ticket… button press Enter. Cancel with Escape. Press Space. Cancel. | Each opens the dialog; after Escape focus is back on the button. | A11, P24 | |
| T-9.4 | Tab to Closed; press Enter; Tab to All; press Space; Shift Tab to Open; press Enter. Press Left and Right on a segment. | Enter and Space choose the segment and the counts do not move. Left and Right do nothing: each segment is a stop of its own (no arrow-key movement is designed). | A6 (twin: `change_the_filters`) | |
| T-9.5 | Tab to the readiness button. Press Enter. | A menu opens with focus in it and no item highlighted. Down moves; Enter on "Ready" chooses it, the menu closes, and focus is back on the button, which now shows the word "Ready". Open it again, press Escape: nothing changes and focus is back on the button. Choose "Any readiness". | A6, A8, P11 | |
| T-9.6 | Tab to the input and type `dock`; press Escape; type `dock` again; press Enter. | Typing narrows as it goes; Escape clears; Enter with rows listed moves focus to the list with the first row selected, and nothing opens. | A6, P9 | |
| T-9.7 | Press Ctrl P, then Shift Tab to the column headers, Tab to TITLE, press Enter, then Space. | Enter sorts TITLE ascending, Space reverses it; the ring shows on the header. Sort by UPD. again. | A6, P12 (twin: `change_the_filters`) | |
| T-9.8 | Press Ctrl 2. Press Down, Down, Up, then End, Home. Press Down at the last row, then Up at the first. | The selected row moves; End selects the last row, Home the first (and scroll to it); Down at the end wraps to the first, Up at the first wraps to the last (K-8). | A6, P18 (twin: `the_filter_bar_works_without_a_read`) | |
| T-9.9 | With a row selected, press Enter. | That ticket opens in the item area. | A6 | |
| T-9.10 | Choose Blocked with the menu. Press Ctrl 2, then Down to select "Blocked until the dock work lands", then Tab. Press PageUp, PageDown, Home and End. Choose Any readiness again. | Tab goes to the Why blocked panel, a stop of its own, with a ring; its text scrolls with those keys when it is long. | A8, A6 (twin: `a_long_why_blocked_panel_scrolls_by_keyboard`) | |
| T-9.11 | Focus the header's `plus` button (T-9.1). Press F6, then Shift F6, then F6 until the ring is back on Tickets. | While the button holds focus the Tickets region draws its ring (the button is counted as part of its panel; write down which rings you see, the button's own and the panel's); F6 moves on to the next region and Shift F6 back. | A11, P24 (twin: `panel_toolbars_work_by_tab_and_enter`) | |
| T-9.12 | Focus the Documents region by F6 (or click a document, then Ctrl 2). Shift Tab to the toolbar: you should reach `chevrons-down-up`, `folder-plus`, `file-plus`. Press Ctrl N on `folder-plus`, then on the Tickets `plus` button. | The Documents stops are the three buttons in order (write down whether a dimmed button is a stop). Ctrl N from a Documents button opens New document…, from the Tickets button New ticket…: the panel's kind, whichever of its buttons has focus. Cancel each. | A11, P24 | |
| T-9.13 | In the tree: Up, Down, Left, Right, Enter on a folder, Enter on a document, Home, End. Then Tab to `chevrons-down-up` and press Enter. | As T-5.4. Collapse all collapses every folder with contents; focus ends in the tree and the button is dimmed. | A6, A11, P18, P20 (twin: `folders_keep_their_state_and_toggle_by_key`) | |
| T-9.14 | Press F6 and Shift F6 through all regions with Tickets and Documents showing. | Each visible region takes its turn with the ring: Tickets and Documents are each one stop (the list or tree, or a toolbar button of it), never a hidden panel (Cycle 01). | A6 | |

## T-10 Tooltips

Hover with the pointer and wait about a second. A tooltip never shows from
keyboard focus (K-1). There is a pure-function twin for the text of each
(`ticket_tooltip`, `doc_row`); the snapshot cannot read a tooltip, so none has
a screen twin. Write down the exact text you read.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-10.1 | Hover the row "Dock layout persistence across restarts". | Lines, in this order: `vh-aaa01 · scratch`; `ID 01ARZ3NDEKTSV4RRFFQ69G5T01`; `Type cycle`; `Primary` (no private working copy yet); `Git: branch main · worktree {the path of scratch}` (write down the branch). | P13, R-14 (pure-function twin: `tooltips_carry_the_id_context_and_reasons`) | |
| T-10.2 | Hover "Upstream: expose source offsets" and "Blocked until the dock work lands". | The first starts `No short code · scratch` and still carries its ID. The second has the line `Waits for open ticket: vh-aaa01 Dock layout persistence across restarts`. | P13, R-14 | |
| T-10.3 | Choose All and hover a closed ticket's row, then its `circle-check` marker. | The row's tooltip ends with the line `Closed`; the marker's own tooltip reads `Closed`. | P5, R-14 | |
| T-10.4 | Hover the nonconforming row, then its amber icon and its pill. Hover the type icon of a normal row. | The row: `Needs fixing: {the guidance for its first problem} Open it to fix.` (write the guidance). Its amber icon: `Needs fixing`. A normal row's type icon: `Type task` (or the ticket's type). The pill and the age have no tooltip of their own. | A2, R-14 (pure-function twin: `tooltips_carry_the_id_context_and_reasons`) | |
| T-10.5 | Hover the two unreadable rows (do it at T-8.7; if you did not, make the T-8 files again and press Ctrl R). | `Can't be read: {guidance}` then the path; no `Git:` line (they are in the primary worktree). The ticket's red icon: `Can't be read`. | A2, P28 (pure-function twin: `unreadable_files_go_to_their_list`) | |
| T-10.6 | Hover each column header (the type arrow, CODE, TITLE, STATUS, UPD.) and the readiness button. | `Sort by type`, `Sort by code`, `Sort by title`, `Sort by status`, `Sort by updated`; and `Show ready or blocked tickets`. | P12, R-14 | |
| T-10.7 | Hover the header buttons. | `plus`: `New ticket…` with the New key beside it (Ctrl N on Linux). `file-plus`: `New document…` with the key. `folder-plus`: `New folder…`, no key. `chevrons-down-up`: `Collapse all folders`, no key. | A11, P24 (no automated twin for the key shown) | |
| T-10.8 | Hover the count in a header, and look while a reload runs (T-7.2) if you caught it. | Write down whether any tooltip shows on the count or on "Updating…": the design and the code give none. | P29 (no automated twin) | |
| T-10.9 | Hover Documents rows: `RFC`, "Project charter", `archive` (empty), `plain-note.md`, `marker-only.md`. | `RFC`: its path (`docs/RFC`). "Project charter": its path (`docs/charter.md`). `archive`: `Empty folder. Shared once it contains a document.` `plain-note.md`: `Not a managed document yet. Open to adopt it.` then its path. `marker-only.md`: `Needs fixing: {guidance} Open it to fix.` then its path (write the guidance). | P22, P23 (pure-function twin: `rows_read_as_the_design_writes_them`) | |
| T-10.10 | Hover a ticket's `pencil` marker and a document's (see T-11.4, T-11.5). | `You have unsaved work on this.` | P14 | |
| T-10.11 | Hover the long short code `vh-spike-faster-index` in "Spike: try a faster index" (K-12). | The Code column clips it; the tooltip's first line shows it whole. | K-12 | |

## T-11 Unsaved work, a quiet Activity, and what is kept

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-11.1 | Look at Activity. Then, with Tickets and Documents showing: choose each segment, a readiness, a sort, type and clear text, select rows, press Home and End, collapse and open folders, press Collapse all. | Activity gains no entry from any of it, nothing in the footer changes, and no tab opens, closes or reloads (only Enter or a click opens an item). | A7, P25 (twin: `the_filter_bar_works_without_a_read`, `folders_keep_their_state_and_toggle_by_key`) | |
| T-11.2 | Press Ctrl R; look at Activity. | One refresh job, because you asked for it; it ends and the lists reload. | A7 | |
| T-11.3 | Choose Closed, TITLE descending, type `a`, collapse `RFC`, then hide Tickets and Documents with their footer toggles and show them again. | The segment, text, sort, selection and the collapsed folder are as you left them. | A10, P26 (twin: `a_hidden_panel_keeps_its_state`) | |
| T-11.4 | Open "Plain-language sync states", press Edit, type a few words in the body, and do not save. Look at its row in Tickets with the tab still showing. | A `pencil` marker appears after the title at once, without a reload. Hover it: `You have unsaved work on this.` | A15, P14 (twin: `markers_appear_without_a_read`) | |
| T-11.5 | Open "Future ideas" (or any document), press Edit, type a few words, do not save. Look at its row in Documents. | A trailing amber `pencil` at the right of its row, at once. | A15, P14 (twin: `a_document_with_unsaved_work_is_marked_at_once`) | |
| T-11.6 | Save (Ctrl S) one of them, or discard its edit. | Write down when the pencil goes (on save, on discard, or only after a reload). | P14 | |
| T-11.7 | **(rare) The conflict marker.** A `circle-alert` after a title, named "This has a conflict to resolve. Open it to choose what to keep.", shows when a pending conflict names the ticket. It needs a conflict as made in D5's "Making a local conflict". If you cannot make one write "skipped". | The red marker beside the ticket's title. | P14 (twin: `markers_follow_drafts_conflicts_and_closure`) | |
| T-11.8 | Quit (Ctrl Q) and start again with the same `$SCRATCH`. Select `scratch`. | Open is chosen, no text, Updated newest first, every folder open: none of the filters, the sort or the folder state survived the restart (K-5). | K-5 (no automated twin) | |

## T-12 The frames in both layouts, the narrow dock and a tabbed group

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-12.1 | Build layout 1a as Cycle 01's W-7.2 does: move Documents to the right dock with its dock menu ("Dock right"), so the right dock stacks Tickets over Documents; drag the right dock's edge to about 400 px. Compare with frame 1a. | Tickets above Documents in one 400 px column, each with its own header, toolbar and count; the list, filter bar and tree as in T-2 and T-5 at this width. | A14 (appearance: no automated twin) | |
| T-12.2 | Put the layout back (Documents: "Dock left", then Repositories over Documents). | Frame 1b's arrangement again. | A14 | |
| T-12.3 | Make the left dock narrow: drag its edge to about 250 px, then about 200 px. Look at Documents (it is in that dock). | The tree rows truncate their labels with an ellipsis; counts and the trailing marks stay at the right; the header's three buttons keep their place at the right and nothing overlaps them (the title and count give way first). Write down at what width the title starts to cut. | K-2 (no automated twin) | |
| T-12.4 | Hide Repositories (Ctrl 1 shows it again) so there is room, then move Tickets to the left dock (right-click its footer toggle, "Dock left"). With the left dock at about 250 px and then about 220 px look at Tickets. | Short codes and status pills clip first so the title keeps at least 96 px (K-2); the age stays; the header's `plus` stays clear of the title and count. Write down what the **filter bar** does at this width: its segments (with counts), readiness button and input sit in one row that does not wrap; write which of them clips or overlaps, if any. | K-2 (no automated twin) | |
| T-12.5 | **Owner decides.** From T-12.4: is the narrow Tickets panel usable? | Note what you see; say whether the filter bar should wrap or shrink at narrow widths. | K-2 (this is a finding, not a listed limit) | |
| T-12.6 | Put Documents and Tickets in one tabbed group in the left dock (drag one onto the other's header) and hover the strip. | The tabs read "Tickets" and "Documents" (with three or more tabs, "Tickets" and "Docs"); only the active tab's toolbar shows at the strip's right; the active tab has the amber underline. | A11, P24 | |
| T-12.7 | **(rare) "Updating…" on a tab.** In the group, with `big` selected and the Tickets tab not active, press Ctrl R and watch its tab. | While the tickets reload their tab reads "Tickets · Updating…"; a three-tab group reads "Docs · Updating…". If you cannot catch it write "skipped". | P29 (twin: `panel_toolbars_work_by_tab_and_enter`) | |
| T-12.8 | Restore the layout (as T-12.2). | Default layout. | A14 | |

## T-13 Orca

Turn on Orca (Super Alt S, or your desktop's switch). Tests read accessible
names and states from the Kit's snapshots only; what a screen reader announces
is checked only here (K-6, K-7). Write down what you hear, word for word.

| # | Do | Expect | Supports | Result / notes |
| --- | --- | --- | --- | --- |
| T-13.1 | Focus the Tickets list (Ctrl 2) and move to "Dock layout persistence across restarts". | Orca reads the title as the row's name, with its list position and that it is selected; the row's description (the tooltip lines) may be read after it. The name is the title, not the ID. | A6, K-1 (no automated twin; the name is asserted, the reading is not) | |
| T-13.2 | Move to a row with no short code and one for a nonconforming entry. | The no-code row's cell is named "No short code"; the entry is named by its path and "needs fixing". | A2, P13 | |
| T-13.3 | Tab to the Closed segment and then to All. | "Closed, 2 tickets" (and for All, "All, 12 tickets"), as a tab, with whether it is selected. With one ticket "1 ticket". | A6 (twin asserts the name; the reading is not) | |
| T-13.4 | Tab to the readiness button; choose Ready by keyboard; Tab back to it. | "Readiness: any", then "Readiness: ready". The menu's checked item is not announced (K-6). | A8, K-6 | |
| T-13.5 | Tab to the column headers, press Enter on TITLE. | "Sort by title" while inactive; after Enter "Sort by title, A to Z"; Enter again "Sort by title, Z to A". On UPD. "Sort by updated, newest first" and "oldest first". There is no sort state beyond the name (K-7). | A6, K-7 | |
| T-13.6 | Tab to the header's toolbar buttons, and to a dimmed one. | "New ticket…" as a button; "New document…", "New folder…" and "Collapse all folders"; a dimmed one is announced as unavailable (write down how). | A11, P24 | |
| T-13.7 | In Documents, move through a folder, a document, a collapsed folder and the empty folder. | A folder: its name and expanded or collapsed (and expanded again after Enter); a document: its title; a collapsed folder's count; the empty folder: its name with "empty" and, in its description, "Empty folder. Shared once it contains a document." | A6, P21, P22 | |
| T-13.8 | Move to an alert (T-7.8 state) and to the empty state's button. | The alert's text is read, then its button's name ("Refresh", "Retry" or "Rebuild the search index"); the empty state's text and "New ticket…". | A13, P15 | |

## Results

Write the build (commit), display server, scale factor, and one line per
section.

| # | Check | Result (pass / fail / note) |
| --- | --- | --- |
| — | Build, display server, scale factor | |
| T-1 | Launch, scratch repositories, the first look (A1, A11, A14, P24) | |
| T-2 | Tickets rows against the frames (A1 to A3, A14; P3 to P6) | |
| T-3 | Filter bar, counts, readiness, sort, footnote, Why blocked (A3, A4, A8; P2, P10 to P12, P17, P29, P30) | |
| T-4 | The filter input as Find (A5, A6, A9; P7 to P9, P18) | |
| T-5 | The Documents tree (A10, A14; P19 to P23, P26; R-27) | |
| T-6 | The panel toolbars in the header (A11; P20, P24, P25; R-37) | |
| T-7 | Every list state (A13; P15, P16, P27, P31) | |
| T-8 | Unreadable files (A2; P28; K-4, K-13, K-14; R-38) | |
| T-9 | The keyboard, with focus visible (A6, A8, A11, A12; P9, P11, P12, P18, P24) | |
| T-10 | Tooltips (A11; P13, P14, P22, P23; K-1, K-12) | |
| T-11 | Unsaved work, a quiet Activity, what is kept (A7, A10, A15; P14, P25, P26; K-5) | |
| T-12 | Layouts 1a and 1b, the narrow dock, a tabbed group (A14; K-2; P29) | |
| T-13 | Orca (A6, A8, A11, A13; K-6, K-7) | |
| — | Known limits hit (K-numbers, steps) | |

## Owner decisions to make

Collect your answers here; none is a pass or fail. Claude records only what
you report.

| # | Question | Where | Your answer |
| --- | --- | --- | --- |
| 1 | **Collapse all and empty folders.** Cycle decision 13 reads "Collapse all collapses every folder". This build collapses only folders that have contents; an empty folder is left as it is, and opens when it gains its first document (a new folder starts open). Keep that, or collapse empty folders too (so one collapsed that way stays collapsed when it gains a document)? | T-6.11 | |
| 2 | **When Collapse all is enabled.** Its enabled state follows the saved folder state, not what is visible: after you collapse a parent by hand whose nested folder was open, the button stays enabled until pressed, and pressing it changes nothing visible. Keep that, or dim it whenever nothing visible is open? | T-6.12 | |
| 3 | **The doubled `circle-x`.** An unreadable document row draws `circle-x` twice (a leading icon and a trailing mark), as the design table reads. Does it look doubled? Keep both, or drop one? | T-8.6 | |
| 4 | **The pill's corners.** The frames draw a 9 px radius; the build draws 4 px. Which do you want? | T-2.4 | |
| 5 | **The narrow filter bar.** The segments, the readiness button and the input are one row that does not wrap; a narrow dock clips it. Should it wrap or shrink? | T-12.4, T-12.5 | |

## Not reached

List here anything the checklist could not reach, and why.
