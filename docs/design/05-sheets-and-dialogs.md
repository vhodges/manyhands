# 05 · Sheets, dialogs and the Needs attention popover

**Sheets** (GPUI Kit `Sheet`, right side) are for detail and settings work. **Dialogs** (GPUI Kit `Dialog`/`AlertDialog`) are for confirmation and for blocking prompts.
- Both trap focus, close on Esc (unless an operation is running), and return focus to the invoking control.
- Scrim: `rgba(10,11,13,.55)` for sheets, `.6` for dialogs.

Common sheet frame:
- The sheet sits below the title bar, full height, and slides in from the right.
- Left border 1 px `border-strong`. Shadow `-24px 0 48px rgba(0,0,0,.45)`.
- **Header** (padding 18 20 14, bottom border): title 15 px / 600, description 12.5 px `secondary`, close `x` button.
- **Body:** padding 18 20, gap 18, scrolls.
- **Footer:** right-aligned buttons, padding 12 20, top border.

Buttons:
- **Primary:** amber, dark text, weight 600, padding 6 14, radius 6.
- **Secondary:** `hover` background, 1 px `border-strong`.

Inputs: 32 px high, radius 6, `editor` background, 1 px `border-strong`. Focus: a teal border plus a 2 px teal 18% ring.

---

## Add repository sheet (1d) · 470 px

Opens from Repositories `+`, File › Add repository, or the first-run empty state.

1. **Mode segmented control:** `folder-input` "Add an existing folder" | `folder-plus` "Create a new repository".
2. **Existing:** a **Folder** path field + `Browse…` (native picker). Live checks underneath, 12 px:
   - ✓ "This folder is a Git repository"
   - ✓ "You can save changes here"
   - ℹ "Manyhands isn't set up here yet. Adding it creates a small settings file and saves it as the first change."
   - Failure rows (red `circle-x`) each come with a recovery action: "Not a Git repository. [Create one here]", "Can't write to this folder. [Choose another]".
   - Already enabled but unregistered → ✓ "Manyhands is already set up here. It will be added to this computer." (see ticket `01M5CVB2…`).
3. **Create:** **Name** + **Location** (`Browse…`). Check: ✓ "Empty folder will be created at ~/Documents/design-research".
4. **Main branch:** a select, showing a teal `Suggested` tag on the suggested branch. Help text: "The version everyone shares. Finished tickets and approved documents are merged here." Tooltip: "Git: the primary branch used for promotion and ticket closure". Required (PRD MH-REPO-001).
5. **Share changes through:** a remote select (`origin` + mono URL), plus a "Keep on this computer" option. Help: "You can also choose "Keep on this computer" and set this up later. Only SSH addresses can be used for sharing." List non-SSH remotes but disable them, with a reason.
6. Info box (`shield-check` teal): "Removing a repository from Manyhands later never deletes, moves or changes its files."
7. **Footer:** Cancel · **Add repository** / **Create repository**. Disabled until the checks pass.
8. Missing Git identity → an inline step "Your name & email" before confirming (RFC onboarding).

## Settings sheet (1e, 1f, 2b) · 720 px

Opens from the footer gear (Ctrl ,). Left nav 214 px, items 12.5 px with 14 px icons, no wrap. Active item: `border` background.

Nav: General · Repositories · **Sharing & keys** · **Checking for updates** · Your name & email · Appearance. Only the bold three and Appearance are designed.

### Sharing & keys

- Intro: "Key used for sharing". "Manyhands uses one key for every repository it shares through. Your private key stays on this computer."
- **Key list** (bordered, radius 8). Each row:
  - Radio: selecting it makes that key the shared key.
  - Label + origin tag (`Made by Manyhands` / `From ~/.ssh`) + `In use` tag (green).
  - Mono fingerprint `ED25519 · SHA256:…`.
  - Note: "Has a passphrase · created Oct 4".
  - `Copy public key`, and a trash button. Tooltips: generated keys "Remove. Asks before deleting the key file."; imported keys "Remove from Manyhands. The file in ~/.ssh is never deleted."
  - The selected row gets a teal 5% tint.
- Buttons: **Make a new key** (primary) · "Use a key I already have…".
- Info box (`lock`): "If your key has a passphrase, Manyhands asks for it once each time you open the app and never stores it."
- **Trusted servers:** host + mono fingerprint + "Trusted Oct 4" (green `shield-check`). A host whose key changed should show the old and new fingerprints (not designed).
- No action ever displays or copies a private key.

### Checking for updates (background polling)

- Intro: "While Manyhands is open it checks each repository's shared copy and brings in other people's changes when it's safe. It never shares your changes, and never touches anything you're still editing."
- A row per repository: name; a status line ("Checked 2 min ago · up to date" / "Couldn't reach origin 7 min ago. Will retry." in red / "Paused"); an interval select (`Every 5 minutes`…); `Check now` (or `Retry now`); an on/off switch (teal when on).
- Repos without a remote: "No shared copy, nothing to check" + `Set up sharing…`.
- **Global:** "Pause all checking until I turn it back on" + switch.

### Appearance (2b)

- Theme cards, 150 px wide: a preview swatch (74 px) + label with icon: **Dark** (`moon`), **Light** (`sun`), **Match system** (`monitor`). The selected card has a 2 px amber border. Applies immediately (in the mockup "Match system" falls back to dark).

## Close ticket dialog (2c → 2d → 2e) · 580 px

Opens from **Close ticket…** in the item header. The same component, reworded, is used for **Approve and merge…** on documents.

Header: a 32 px icon tile (`archive` amber; `circle-x` red when failed; `circle-check` green when done) + title + subtitle `MH-VH-7Q3 · Wave 03 D3: Rich-text editor integration · manyhands`.

**Stage: confirm (2c)**
- Lead: "Closing runs these steps in order. If one fails, it stops there and nothing after it happens."
- **Step list** (bordered, numbered 20 px circles):
  1. **Save your latest edits** – Anything unsaved in this tab is saved first
  2. **Share with origin** – Bring in other people's changes, then send yours
  3. **Merge into main** – The files below become part of the main version everyone shares
  4. **Mark as closed** – Closed by Vince Hodges · today
  5. **Tidy up** – Remove the private working copy, on this computer and on origin
- **Files that will be merged into main {n}:** one row per changed path. Each row has an icon, the mono path, a tag (`ticket` / `document` / `code`; code is tinted teal) and `+182 −12`. If any non-item path is present, add the note "This working copy includes code as well as the ticket. All of it is merged." (RFC: the effect summary must enumerate the whole branch).
- **Becomes ready to start:** shows the tickets unblocked by this close (MH-CONTENT-005).
- Git detail line (mono 11 px, `muted`): `Git: merge manyhands/ticket/… into main · push origin · delete local and remote branch + worktree`.
- Footer hint "Enter to confirm · Esc to cancel"; buttons: Cancel · **Close ticket**.
- Local-only repositories: step 2 reads "Keep on this computer (sharing pending)", and the outcome reports "publication pending".

**Stage: running (2d)**
- Lead: "You can keep working. This continues in the background and shows up in Activity."
- Step states: done (green `circle-check` 18 px) · current (spinner, 16 px teal arc) · pending (numbered, muted). The current step's description shows progress ("Sending 3 saved changes to origin…").
- The only button is **Stop at a safe point**. Clicking the backdrop or Esc hides the dialog; the work continues, and progress stays in Activity and the footer.

**Stage: failed (2e)**
- The failed step shows a red `circle-x`, a red title, and a description naming the cause ("origin refused the key "Vince's laptop". Nothing after this step ran.").
- Outcome box (red 8% tint, `shield-check`): "Nothing was merged or tidied up. Your saved work is still on this computer, and Retry picks up from step 2."
- Buttons: Not now · Open Sharing & keys (contextual fix) · **Retry**. Retry reuses the same operation (RFC: Retry reuses the original operation).

**Stage: done**
- "Ticket closed". Outcome (green): "Shared, merged into main and tidied up." The ready list is shown as "Now ready to start". Button: **Done**.

A changed observation (new path, new remote state) **invalidates** the confirmation and re-renders the effect summary (RFC).

## Resolve conflict sheet (2g) · 920 px

Opens from the conflict banner, the Needs attention popover, or the Activity "Review" action.

- Title "Choose what to keep · {ticket title}". Description: "You and Jo Smith both changed this ticket since it was last shared. Pick a version for each part, or keep both."
- Meta line: type icon, short code, repo, progress (`1 of 2 chosen`, amber, then green when complete).
- **One card per conflicting part** (metadata field or body region):
  - Header: part name (600) · kind (`Ticket detail` / `Description`) · state (`Choose one` amber / `Chosen` green) · choice buttons (`Keep mine`, `Keep Jo's`, and `Keep both` for text). The selected choice is filled teal with white text.
  - Body: 3 equal columns, **Before** (last shared copy) | **Yours** (name · time) | **Theirs** (name · time). Mono 11.5 px lines. Added lines get a green 14% tint and a `+` gutter; changed lines get an amber 14% tint and a `~` gutter; unchanged lines are `secondary`.
- **Result** preview: mono lines in an `editor` box, updated live. Note: "Saving keeps the result on this computer. You can still edit it in the ticket afterwards, and Share sends it to everyone."
- Footer: the Git detail (mono, ellipsis: `Git: conflict in … · base … · ours … · theirs …`) · Cancel · **Save resolution**, enabled only when every part is chosen.
- Saving validates, checkpoints and clears the banner. There is **no automatic winner** (RFC).
- Noncanonical conflicts (code, binary, rename/delete) get a different card: "This file needs fixing outside Manyhands" + Open folder + Check again (RFC external recovery). Not designed visually; reuse the card frame.

## Unlock key dialog (2h) · 440 px

Shown on first SSH use per session (including the first poll), or from Needs attention.
- Header: `lock` amber tile, "Unlock your key", subtitle "Sharing from manyhands is waiting for your passphrase."
- Key card: `key-round`, label, mono fingerprint.
- A **Passphrase** masked input (focused on open) + show/hide `eye` toggle.
- Note (`shield-check`): "Kept in memory until you quit Manyhands. It's never written to disk. If you choose Not now, you can keep working and unlock later from Needs attention."
- Buttons: Not now · **Unlock**. A wrong passphrase shows an inline error under the field and keeps the focus there.
- Never re-prompt modally in the background after Not now (RFC). The prompt moves into Needs attention.

## Needs attention popover (2f) · 440 px

Anchored above the footer's attention item (bottom 34 px, right-aligned to it). Click outside or Esc closes it.
- Header: "Needs attention" / "Your work is safe on this computer. Nothing here happens until you choose."
- **One row per unresolved recovery state:** icon (colored) · title (500) · description (12 px `secondary`) · action button · repo tag. The tooltip has the Git detail. Examples:
  - `circle-alert` red: "You and Jo Smith both changed "Footer toggles for dock panels"". "Choose what to keep. Nothing is shared until you do." [Review changes]
  - `lock` amber: "Your key is locked". "Sharing from manyhands is waiting for the passphrase." [Unlock…]
  - `wifi-off` red: "Couldn't reach zorite's shared copy". "Last tried 7 minutes ago. Your work is safe, and Manyhands tries again in 5 minutes." [Retry now]
- Footer link: "Show everything in Activity" (opens and shows the Activity panel).
- The same items also appear in Activity. The popover is a filtered view, not a separate store.
