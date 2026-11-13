---
title: "Wave 04 Cycle 03b Activity And Needs Attention Design"
date: 2026-11-12
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M79CDM63GRGDMMDH8DQ5E1E3"
---

# Wave 04 Cycle 03b Activity And Needs Attention Design

## Intent And Approval Boundary

This design covers [Cycle 03b](../Cycles/wave-04-cycle-03b-activity-and-needs-attention.md):
Activity rebuilt as the design's one timed log, and the Needs attention
popover above the footer, both reading one store of rows. Activity's rows
come from the models the operations area reads today. Each row has the
time the app learned of it, a plain-language message, a repository tag, at
most one button, a menu with the rest, and the Git detail in its tooltip.
The popover is the same rows filtered to what needs the person, and the
footer's count is their number. The Cycle also folds in three pieces of
existing work:

- splitting `model/operations.rs`;
- classifying a share once, instead of on every render;
- showing the running stage where "Finishing" hides it today.

Approval covers the mechanisms, rulings, named test edits and task order.
It does not authorize implementation, pushes beyond the ticket branch,
merging or closing.

**Draft, revision 1.** Grounded at the ticket branch `b37d2c2b`, which
holds:

- origin/main `37be101d` (Cycle 01, Cycle 02 and Wave 03 D6 merged);
- Cycle 03a's published head `3086666f`, merged in on 2026-11-12 by
  decision 4.

Cycle 03a is in its final review and not merged. The 03a session holds a
later final-review commit (`43f411a0`) that was not published when this
was drafted and is not in this base. Task 0 merges main after 03a closes
and re-checks every fact against it.

Conventions:

- **Paths and crates:**
  - `D/` is `src/desktop/`; `L/` is the library under `src/`.
  - `C/` and `B/` are `gpui-component-0.6.6/src/` and `gpui-base-0.6.6/src/`
    under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (the
    locked crates).
- **References:**
  - "Fact n" refers to Baseline Facts.
  - "P-n", "R-n", "E-n", "K-n" and "U-n" are this design's Required
    Outcomes, Rulings, Named Edits, Known Limits and unverified facts.
  - "N-n" is a row of Part A's attention table.
  - "Decision n" is a Cycle 03b decision. "3a decision n", "3a R-n" and
    "3a K-n" are Cycle 03a's. "C1 R-n" is a Cycle 01 ruling. "D5" and "D6"
    are the Wave 03 desktop Cycles.
- **The handoff** is `docs/design/`: `02` is `02-layout-and-docking.md`,
  `03` is `03-panels.md`, `05` is `05-sheets-and-dialogs.md` and `06` is
  `06-language-and-states.md`.
- **Terms:**
  - A **row** is one `ActivityRow` (Part A). **The log** is the
    Workspace's `ActivityLog`, which holds the rows and their times.
  - An **attention row** is a row whose `attention` is set. The popover
    lists exactly these, and the footer counts them.
  - An **event** row records something that happened: a request, a check
    that brought something in, a finished retirement. A **state** row
    records a condition that holds now: a locked key, an unreachable
    shared copy, a pending operation, a recovery copy not saved.

    Events stay for the session until dismissed. A state leaves when it
    no longer holds (decision 5).
- **Repository state:**
  - `{repo}` is the repository's display name (`RepositoryEntry::name()`,
    C1 R-14).
  - "The repository state" is Cycle 03a's `repository_state(&RepoFacts)`
    (fact 23).
- **Item titles:**
  - "The title" is an item's title, read from the places fact 25 names.
  - "The short code" is the desktop's `short_code(&ItemId)`: the ULID's
    last eight characters (fact 20).

## Baseline Facts At `b37d2c2b`

**The operation area today.**

1. **Host.**
   - `Workspace::activity_region` (`D/view/workspace/render.rs:78-107`)
     builds, in order:
     - `finishing` (the quit row);
     - `session_section`;
     - `pending_section`;
     - `prune_abandon_focus`;
     - `layout_notices`.
   - It draws the notices first. Below them it draws a scrolling
     `v_flex().id("operation-area")` (`:91-98`), which holds the quit row
     and then, side by side, the session and pending columns (`:100-107`).
   - The Activity panel's region is `Region::Operations`
     (`D/view/workspace/focus.rs:16-22`). Its key context is the literal
     `"Operations"` (`:99-106`), and no binding names it (fact 35).
2. **The session column** (`D/view/operations.rs:663-726`, id
   `"session-section"`) lists, in order:
   1. the heading "This session";
   2. the checking holds (fact 19);
   3. the latest check of each repository (fact 20);
   4. "Recovery copies are unavailable: …";
   5. "Recovered drafts: {n}" with Review and Dismiss;
   6. record notes;
   7. recovery copies not saved, with Retry;
   8. "No requests yet";
   9. one entry per request, newest first, Dismissed ones left out (`:674`).
3. **The pending column** (`:729`, id `"pending-section"`):
   - **Each repository's pending list:**
     - while its read is not Ready: its name, the list state and, when
       Busy or Failed, Retry, which calls `load_pending` (`:756-787`);
     - when Ready: one row per pending operation, from `pending_shown()`
       (fact 18).
   - **Each pending row** shows the raw `action`, scope, `state`,
     operation ID and `next_action` (or "Nothing to do"), with Resume,
     Review changes and Abandon as offered (`:789-936`).
   - **Ids** are `(name, index)` tuples, numbered across repositories in
     drawn order.
4. **A request entry** (`:358-557`):
   - **What it shows:**
     - the command's fixed label;
     - the short item ID (lifecycle only);
     - the scope;
     - `state_label` (`:140-158`): "Queued", "Running", "Finishing" or
       an outcome label ("Done", "No change", "Partly done", "Blocked",
       "Cancelled", "Failed", "Not sent: …").
   - **A share, post, check or retirement** reads "Sharing · …" and so
     on (fact 14).
   - **A finished entry that did not succeed** adds:
     - the envelope message, code and effects (`:444-460`);
     - share offers;
     - recovery actions;
     - Open… (lifecycle);
     - Review changes;
     - Dismiss.
   - **Controls:** Cancel, Stop at a safe point, and a disabled
     "Stopping…" or "Finishing".
   - **`state_label` matches `Running { accepted: true, .. }`** (`:143`)
     before the stage arm, so an accepted request reads "Finishing"
     whatever its stage.
5. **No row has a time.** `DesktopRequest` (`D/services/requests.rs:822-840`)
   has no time field. `Operations` and the worker hold no wall clock;
   `Operations.sent` is a sequence counter (`D/model/operations.rs:118-120`).
   The holds, notes, notices and draft entries carry no time either.
6. **Lifetime.**
   - `Operations` is `#[derive(Default)]`, in memory (`:112`).
   - **`apply(Queued)`** (`:231-244`):
     - inserts a new request at index 0;
     - replaces a retried one in place, keeping its position, which also
       brings a Dismissed one back.
   - **`dismiss`** (`:605-618`) sets `Dismissed` on a request that is not
     Queued or Running, and keeps the entry. It does not remove
     `shares[id]`.
   - **A retirement** whose verdict is Retired, Attention, Unreachable or
     Retry is dismissed automatically when it finishes (`:366-368`;
     `D/model/polling/retire.rs:71`).
7. **Cycle 01's K-19 rows.** The draft-flush row (`:702-709`), a pending
   repository row (`:761-785`) and the pending controls row (`:839`) are
   `h_flex().gap_2()` with no wrap and no `min_w_0`.
   - The recovered-drafts row (`D/view/operations/recovered.rs:21`) and a
     request's first line (`:369`) are also unwrapped.
   - The rows that do wrap use `flex_wrap()`.
8. **Focus maps.**
   - **Fields** (`D/view/workspace.rs:167-176`, `:199-200`):
     `abandon_focus`, `abandon_drawn`, `control_focus`, `control_drawn`,
     `retire_asked`, `republish_deferred`.
   - **Handles:**
     - They are made by `control_focus_for` and `abandon_focus_for`
       (`D/view/workspace/requests.rs:523`, `:537`), and are not tab stops.
     - They are pruned only from `activity_region`, by
       `prune_abandon_focus` (`render.rs:82`; `requests.rs:550-555`).
   - **Production code uses element ids as focus keys:**
     - `show_holder` (`requests.rs:241`);
     - `run_recovery` (`:601`);
     - the drafts-not-saved Cancel (`D/view/workspace/lifecycle.rs:318`);
     - the dialog origins (`D/view/operations.rs:352`,
       `D/view/operations/review.rs:102`).
9. **Above the area.**
   - **The layout notices** (`D/view/workspace/layout_notices.rs:15-17`,
     `:21`, `:61`):
     - "Your panel layout couldn't be restored, so the default layout is
       back." and "Your panel layout couldn't be saved.", each with
       Dismiss;
     - not counted (C1 decision 5).
   - **The quit row** (`D/view/quit.rs:85-146`):
     - It reads "Saving drafts for recovery", "Finishing {label}",
       "Finishing a check for updates", "Finishing" or "Still stopping —
       waiting for Manyhands to reach a safe point. Your work is saved.",
       with Quit now.
     - Its ids are `quit-finishing` and `quit-now`.
     - Its focus handle `quit_focus` is a tab stop
       (`D/view/workspace.rs:365`), focused automatically only when
       Activity is displayed (C1 decision 7).
10. **Sizes.**
    - `D/model/operations.rs` is 1276 lines. Its submodules: outcomes 198,
      polls 148, retire 291, review 124, quit_wait 42.
    - `D/view/operations.rs` is 999 lines. Its submodules: attempts 232,
      attention 157, outcomes 344, record 49, recovered 47, republish 247,
      resume 174, retire 334, review 150, and the test support
      attention_support 142.
    - `D/view/workspace.rs` is 488 lines; `D/model/repository_state.rs` 497.

**Requests, shares and stages.**

11. **`Operations` fields** (`D/model/operations.rs:112-166`): `requests`,
    `sent`, `sends`, `answers`, `rejected`, `pending`, `generations`,
    `changed`, `prompt`, `saving_drafts`, `quit_wait`, `removals`,
    `observed`, `stopping`, `shares` (each finished share's envelope),
    `lifecycles`, `locks`, `reviews`, `automatic_polls`, `record_notes`
    and `retirements`.
    - **Line ranges:**
      - struct 112-166; `Changed` 168-204; the main `impl` 206-710;
      - `StopOffer` 712-720; `Progressing` 722-751; `stage_text` 764-810;
        `progress_words` 812-831;
      - `RecoveryStep` 843-865; the recovery `impl` 886-1076;
      - pending-row predicates 1078-1156;
      - `ResumeOffer`, `resume_offer` and `abandon_offered` 1158-1209;
      - `RecoveryControl` and `recovery_label` 1211-1272.
12. **Shares are classified on every render.**
    - **The chain:** `session_entry` calls `Workspace::finished_share`
      (`D/view/operations/outcomes.rs:238`) for each finished share
      (`D/view/operations.rs:436`, `:478`). That reaches `share_result`
      (`outcomes.rs:98-110`), which:
      - reads `WorkerHandle::record_state(id)` and `gave_up(id)`
        (`D/services/worker.rs:593`, `:604`), locking the worker twice;
      - calls `Operations::share_result(id, record, after_give_up)`
        (`D/model/operations.rs:477`);
      - which classifies the stored envelope with `sync_outcome`
        (`D/model/publication.rs:367-420`).
    - **Other callers:**
      - `share_words` and `shown_offers` (`outcomes.rs:151-188`) add the
        Busy holder and the republish-running state, which read other
        requests at render time;
      - the item tab's `share_outcome` (`D/view/item/share.rs:218-240`)
        calls the same chain on a model change and caches it;
      - tests call `Operations::share_result(…)` directly
        (`D/view/operations/give_up_tests.rs:344`) and the view's
        `share_result` (`D/view/operations/outcomes_route_tests.rs:91`).
13. **When `record` and `gave_up` are final.** The worker:
    - sets an attempt's `record` before `finish` and before sending
      `Finished` (`D/services/worker.rs:1337-1347`, `:1351`, `:1363`), only
      for intents that stop at safe points, and does not clear it on
      requeue;
    - clears `gave_up` at requeue (`:680`) and sets it in `give_up_first`
      (`:1428`), before the library call.

    The drain reads both at drain time. It already tells the model a
    retirement's record before applying its `Finished`
    (`scheduler::note_record`, in the drain loop of `D/services/mod.rs`
    at about `:619`; `D/services/scheduler/retire.rs:47-61`).

    About 36 lines in 21 test files build `WorkerEvent::Finished(…)`
    themselves.
14. **Stage words.**
    - **Which intents have them:**
      - `Progressing::of` (`D/model/operations.rs:738-750`) gives words to
        ItemSync and RepoSync ("Sharing · …"), CommentAdd ("Posting ·
        …"), PollOnce ("Checking · …") and ContextRetire ("Retiring ·
        …").
      - The lifecycle gives TicketClose "Closing · …" and DocumentPromote
        "Merging · …" (`D/view/lifecycle/outcome.rs:522-538`).
      - Every other intent reads "Finishing" once accepted.
    - **`stage_text`** (`:768-810`) maps the 14 `ProgressStage` values
      (`L/repository/mutation/progress.rs:95-110`) to words. Two notes:
      - Writing gives none for Share and Check;
      - `progress_words` passes `primary: None` (`:815-831`).
    - **A disabled "Finishing" button**, id `request-N-finishing`, is
      drawn for any accepted request that is not offered Stop
      (`accepted_control`, `D/view/operations.rs:522-557`).
    - **Tests that pin "Finishing":**
      - `a_running_check_now_reads_its_stage_words` asserts it for the
        poll policy verbs at the Writing stage
        (`D/view/operations/check_tests.rs:111-118`);
      - `stop_is_not_offered_without_a_remote` asserts the button is
        present (`give_up_tests.rs:390-418`).
15. **Intents.**
    - **Variants:** 45 (`D/services/requests.rs:69-351`).
    - **`label()`** (`:418-466`) is "Fixed text: no value the user
      typed".
    - **`root()`** (`:469-515`) is `None` for the key intents, which today
      show the scope "Application".
    - **Titles:** DocumentSave, TicketSave, DocumentCreate and
      TicketCreate carry the title being saved.
    - **Item IDs:** the other item intents carry `id: ItemId`, apart from
      DocumentRepair and TicketRepair, which carry a path.
    - **Sent by the app, not the person:**
      - IndexRefresh, at launch for each listed repository
        (`D/view/workspace/repositories.rs:72`) and after a create, an
        enable or a post;
      - ContextRetire (the retirement loop);
      - ContextRelease (release on leave, `D/services/release.rs:314-317`);
      - ContextPrepare for a new item (`D/services/drafts/create.rs:109`);
      - PollOnce after Unlock… or a reconfiguration;
      - the resends.
    - **Not requests at all:** automatic polls (`Work::Poll`,
      `WorkerEvent::Polled`, `D/services/worker.rs:958-977`) and Resume
      checking (`D/services/scheduler/attention.rs:243-259`).
16. **Envelopes.**
    - **The message** (`L/results.rs:149`, `:184`, `:521`) is the code's
      fixed English sentence. It carries no values and is sometimes
      CLI-flavoured, for example "The command line is not valid; see
      --help.".
    - **`effects_text`** (`D/view/operations.rs:174-193`) prints raw
      contract values.
    - **Recovery actions:**
      - `RecoveryActionKind` has 12 kinds (`L/results.rs:402-432`);
      - their labels are `recovery_label` (`D/model/operations.rs:1258-1272`);
      - `recovery_step` (`:1014-1075`) turns one into a `RecoveryStep`;
      - lifecycle entries hide the controls their dialog owns
        (`D/view/operations.rs:198-208`).
17. **Share outcomes.**
    - `SyncOutcome` (`D/model/publication.rs:295-332`) has 14 variants.
    - Each one's words are in `D/model/operations/outcomes.rs:96-133`, and
      its offers in `SyncResult::offers()` (`publication.rs:427-477`).
    - **Offer labels:**
      - "Share", "Retry", "Stop trying to share", "Unlock…";
      - "Open Sharing & keys", "Check host", "Set identity", "Show it";
      - "Set up sharing", "Republish…", "Stop and republish…".
    - "Show it" focuses the possible holder's entry (`show_holder`,
      `D/view/workspace/requests.rs:231`).
18. **Pending operations.**
    - **`OperationDto`** (`L/repository/read/dto.rs:833-882`):
      - `operation_id: Option`, `family`, `owner`, `action`, `state:
        String`, `completed_step`, `next_action`;
      - `item_id: Option<String>`, `key_id`, `worktree`, `updated_at`,
        `failure_code`.
    - **Offers:**
      - `resume_offer` (`D/model/operations.rs:1179`) gives `Offered`,
        `Unavailable("Synchronization is not available yet")` or
        `NotOffered`;
      - `resume_offer_among` (`D/model/operations/polls.rs:109`)
        withholds Resume from this app's automatic polls;
      - `abandon_offered` is at `:1205`.
    - **`pending_shown`** (`D/model/operations/retire.rs:236`) leaves out
      the worker's running work.
    - **Lists are loaded** when:
      - repositories load;
      - a finished request changed a root;
      - the window activates;
      - a Busy or Failed list's Retry is pressed;
      - `show_holder` finds no entry.
19. **Checking holds.**
    - **The model:** `PollSchedule::attention()`
      (`D/model/polling.rs:447`) groups the repositories whose phase is
      `Attention(reason)` by equal reason, as
      `AttentionEntry { reason, roots }` (`:50-54`).
    - **Headlines** (`D/model/polling/attention.rs:77-87`):
      - "Checking is waiting for your key to be unlocked";
      - "… needs a key to be chosen";
      - "… needs a host to be approved";
      - "… can't start until sharing is set up";
      - "Checking is held until recovery".
    - **"Holding back {names}"** follows each headline.
    - **Actions** (`Workspace::attend`, `D/view/operations/attention.rs:120-156`):
      - Unlock…, Choose a key…, Approve host…, Open remote settings…
        and Resume checking;
      - Unlock… with no target is drawn as unavailable text (`:81-92`).
20. **The latest check.**
    - **`attempts()`** (`D/model/polling.rs:113`) yields each repository's
      `last` only, so there is no history.
    - **`AttemptSummary`** (`D/model/polling/attempt.rs:163`) has
      `brought_in`, `to_share`, `problems`, `listed` and `retire`, and
      `completed_at` for a Completed answer only. Its targets carry an
      optional `ItemId`, a kind and `main`, but no title, branch or name.
    - **What the entry shows** (`D/model/polling/entry.rs`):
      - problems use the library's `PollDisposition::guidance()`
        (`L/repository/remote/poll.rs:47-84`, CLI-flavoured), except
        Unpublished, which uses the desktop's `UNPUBLISHED` (`:30`) and
        Share…;
      - retirement lines and Retire… offers (`:106-160`);
      - the short code (`:97`).
    - **No titles:** the entry names no title by design
      (`entry.rs:19`; `D/view/operations/attempts.rs:21`).
21. **Where checks are applied.**
    - **Automatic polls** go through `PollSchedule::after(&Now,
      &PollAttempt)` (`D/model/polling.rs:385`), called from
      `scheduler::apply` (`D/services/scheduler.rs:210`) in
      `Scheduler::answered` (`D/services/scheduler/react.rs:41-93`). That
      path also marks the repository changed when `summary.changed()`
      (`:74-80`).
    - **A Check now** goes through `PollSchedule::checked_now` (`:396`),
      called from `Scheduler::checked_now`
      (`D/services/scheduler/retire_loop.rs:149-168`). A failed one goes
      through `check_failed` (`D/services/scheduler/attention.rs:81`, `:120`).
    - **Cycle 03a's unshared list** (`RepoSchedule.unshared`, its `items`
      and `problems`) is set from completed checks and changed at once by
      this app's saves and shares (`D/model/polling.rs:246-338`).
22. **The clock.**
    - **The trait:** `trait Clock { utc, mono, now }`
      (`D/model/polling/clock.rs:14`).
    - **The scheduler** holds `clock: Arc<dyn Clock>`
      (`D/services/scheduler/entity.rs:84`) and exposes `utc()` (`:236`).
    - **Tests** inject one through `use_clock` (`:241`) and
      `Services::with_clock` (`D/services/mod.rs:869`), with `TestClock`
      (`clock.rs:73-118`).
    - Views already read `scheduler.utc()` for "ago" (footer, Repositories,
      Settings).
23. **Repository state** (Cycle 03a, `D/model/repository_state.rs`):
    - **Types:**
      - `repository_state(&RepoFacts) -> RepoState` (`:207`) takes the
        first of cant_open, needs_fixing, checking, locked,
        needs_attention, rest and unknown;
      - `StateKind` (`:96`) has 13 variants, `RowAction` (`:128`) has 8;
      - `repository_tooltip` is at `:474`.
    - **`checking()`** is a Check now or a poll Queued or Running (`:89-91`;
      `in_flight`, `D/model/polling/state.rs:160`). So a repository that
      cannot be reached reads Checking… while each automatic retry runs.
    - **`attention_cause`** (`:333-378`) gives Needs attention for, in
      order:
      1. `Phase::Attention(reason)`;
      2. `Shape::Held`;
      3. `conflict_pending`;
      4. `Waiting { Failed }`;
      5. an unreadable registration;
      6. a `Transport(Unavailable | ConfigurationRequired)` answer;
      7. a stored outcome;
      8. the unshared list's problems ("{n} items need a look. Activity
         lists them.").
    - **`RepoFacts`** is built inline in `repositories::rows`
      (`D/view/repositories.rs:98-152`). It borrows locals there and has
      no builder of its own.
    - **Row actions** run through `Workspace::row_action`
      (`D/view/workspace/repository_menu.rs:229-250`).
24. **The footer count today** is `needs_attention(&Operations,
    &DraftRegistry, &PollSchedule)` (`D/model/attention.rs:36-80`). Its
    only caller is `Workspace::attention` (`D/view/workspace/footer.rs:32`).
    - **What it adds up:**
      - finished Partial, Blocked or Error requests not dismissed,
        grouped by operation;
      - recovery-copy failures;
      - an unavailable draft store;
      - every pending row, "Nothing to do" included, plus failed
        pending reads;
      - `polling.attention().len()`.
    - **What it leaves out:** Can't open, Needs fixing, Can't reach, a
      stored outcome, a held repository with no reason, a pending
      conflict, `Waiting { Failed }`, an unreadable registration and a
      check's problems (3a K-9).
25. **Titles.**
    - **What can be read synchronously:**
      - `ShareTitles::share_title(root, id)`
        (`D/view/workspace/titles.rs:78`) is a map lookup, filled from
        the selected repository's Tickets and Documents reads and from
        Cycle 03a's reads of unshared items;
      - an open tab's `tab_title()` (`D/view/item.rs:447`) comes from
        `ItemModel::last_title()` (`D/model/item.rs:259`);
      - save and create intents carry their title (fact 15).
    - **Nothing else** maps an `ItemId` to a title synchronously.
26. **No names.**
    - A check records counts only (fact 20), so "Brought in N changes
      from {name}" cannot name anyone.
    - A conflict's wording is "You and someone else both changed this …"
      (`D/model/operations/outcomes.rs:76`; `D/view/conflict.rs:113`).
27. **Drafts and notes.**
    - **Drafts** (`D/services/drafts.rs`):
      - `flush_failures()` (`:440`) gives `(DraftId, title, kind)`;
      - `unavailable()` (`:224`);
      - `recovered()` (`:304`) and `dismiss_recovered` (`:319`);
      - `flush_now` (`D/services/drafts/flush.rs:35`).
    - **Record notes:** `record_notes()` and `dismiss_record_note`
      (`D/model/operations/polls.rs:52-57`). A `Report` is `Unreadable` or
      `NotSaved`.

**Footer, menus and keys.**

28. **The footer.**
    - **Types** (`D/view/footer.rs`):
      - `Control` (`:47-62`) and `Control::ALL` (11, `:66-78`);
      - `FooterStatus { update, to_share, attention }` (`:201-206`);
      - `plan(layout, &FooterStatus)` (`:221`);
      - `attention_text` (`:129-135`): "1 needs attention" and "{n} need
        attention", hidden at 0.
    - **What Needs attention does today:**
      - pressing it runs `reveal_and_focus(PanelKind::Activity)`
        (`D/view/workspace/footer.rs:173`);
      - it has no tooltip (`footer.rs:382`);
      - only panel toggles record bounds (`:399-412`);
      - `focus_after_hiding(Attention)` is `None` (`:269-273`), and
        `keep_focus_in_footer` covers only Update check and To share
        (`D/view/workspace/footer_status.rs:181-190`).
29. **Cycle 03a's repository menu** is the pattern for a Workspace-owned
    layer (`D/view/repositories/menu.rs`;
    `D/view/workspace/repository_menu.rs`).
    - **The struct:** `RepositoryMenu { root, view: Entity<PopupMenu>, at,
      context, restore, chosen }`.
    - **The layer** (`menu.rs:161-183`):
      - It is a full-window `deferred(anchored(…))` with
        `POPUP_PRIORITY`.
      - It focuses its view on every draw, so it is dropped while a dialog
        or sheet is open (`repository_menu.rs:293-302`).
    - **A choice** is a Kit `Action`, `RepositoryMenuChoice { root, choice,
      shown }` (`D/keys.rs:150-165`), checked again against the current
      rows (`still_offered`, `menu.rs:59-74`).
    - **The footer's `DockMenu`** opens upward from a toggle's bounds with
      `Anchor::BottomLeft` (`D/view/footer/menu.rs:84`, `:115`).
30. **Cycle 03a's panel** draws its own `RowElement` inside a Kit `List`
    (`D/view/repositories/row.rs:47-286`).
    - **The rows** are applied after the frame through `window.defer`
      (`show_after_frame`, `D/view/repositories.rs:215`).
    - **A right press** opens the menu (`row.rs:212-220`).
    - **The tooltip** is also the accessible description (`:232`).
31. **The Kit `List`** (`C/list/list.rs`):
    - **Keys** (`:26-35`): it binds `escape` (Cancel, which propagates),
      `enter` (Confirm) and `up` and `down` (which wrap) in context
      `"List"`. It has no `home`, `end`, `menu` or `shift-f10`.
    - **Row size:** it measures one row and gives every row that size.
    - **Each item's wrapper** carries `aria_selected`.
32. **The Kit `Popover`** (`C/popover.rs`; `B/popover.rs`):
    - its trigger must be `Selectable`, which the footer's wrapper is not;
    - its open state is keyed element state that the app cannot reach.
    - No app code uses it.
33. **Every Kit `PopupMenu`** has element id `"popup-menu"`
    (`C/menu/popup_menu.rs:1464`) and binds `enter`, `escape` and the
    arrows in context `"PopupMenu"` (`:19-28`).
34. **Toolbars.**
    - `ToolbarButton::of(PanelKind::Activity)` is `&[]`
      (`D/view/dock/header/toolbar.rs:76-85`).
    - Panel toolbar buttons are base `Button`s with a tracked focus handle
      (`:91`, `press_toolbar` `:203`). The base `Button` has no toggled
      state.
    - The footer's toggles show their state with `aria_toggled` on their
      wrapper (`footer.rs:397`).
35. **Keys.**
    - `secondary-4` is bound to `FocusOperations` (`D/keys.rs:274`). The
      contexts are named in `D/keys.rs:169-242`.
    - `keys_tests.rs`' table has a fixed length of 26
      (`D/keys_tests.rs:15`).
36. **Dialogs.**
    - `modal_open` is `D/view/workspace/render.rs:156`, and `unless_dialog`
      (`:162`) wraps every Workspace action except Quit and QuitNow.
    - `confirm(intent, origin, …)` (`D/view/confirm.rs:161`) returns focus
      to `origin` when it closes (`:255`).
37. **Local time.**
    - `local_offset(timestamp) -> Option<UtcOffset>`
      (`D/view/confirm.rs:583`) uses `localtime_r` on Unix and is `None`
      elsewhere (`:601`).
    - `local_time` (`:562`) formats "YYYY-MM-DD HH:MM:SS" and adds " UTC"
      when the offset is unknown.
    - There is no HH:MM helper.

**Tests.**

38. **What reaches the operation area today.**
    - **Size:** 494 source lines in 54 test files, in 167 test functions
      and 25 helpers. They break down as 92 presses, 28 focus moves, 112
      label reads, 128 presence checks and 9 waits, through:
      - `request_id` (147 lines), `recovery_id` (16) and `draft_flush_id`
        (14);
      - the pending, attempt, attention, record and recovered ids;
      - `"operation-area"` (5).
    - **Inventory:** Named Edits lists each file and its tests.
    - **What no test asserts:** the headings, the columns or the drawn
      newest-first order.
    - **Containment:**
      - three tests assert a control lies inside the area's bounds:
        `retire_left_offer`, `retire_pending_row_by_keyboard` and
        `retire_rows_hit`;
      - four resize the window to bring a control into view.
39. **The harness** (`D/view/harness.rs`, on `Shell`) has:
    - `click` (`:296`), `press` (`:324`), `label` (`:336`);
    - `tab_reaches` (`:365`), `present` (`:382`), `rows` (`:409`);
    - `operations` (`:435`), `requests` (`:455`), `until` (`:474`),
      `until_drawn` (`:487`).

    Its Activity dock is 520 px (`:93`). `Workspace::abandon_focus` is a
    test-only accessor (`D/view/workspace/requests.rs:558`).
40. **Privacy sweeps.**
    - **`polling_privacy`** (`D/view/polling_privacy_tests.rs`):
      - It classifies drawn text by the element's own id prefix
        (`SURFACES`, `:157-171`).
      - It asserts that:
        - surfaces were read (`:820`, `:946-971`);
        - certain pairs were seen (`read_at`, `:984-1002`), such as
          `("attention", "Unlock…")`, `("pending row", "Resume")` and
          `("operation entry", "worktree_not_clean")`.
    - **D5's sweep** (`D/view/lifecycle/privacy_tests.rs`):
      - It classifies by ancestor container, with `"operation-area"`
        first (`:89`).
      - It allows an item's title only in `ALLOWED` (`:73-84`), which does
        not include the operation area.
      - Four tests assert that the operation area was read.
    - **The drafts sweep** asserts that drawn text starts with "Recovery
      copy of" (`D/view/drafts_privacy_tests.rs:272-276`).
41. **Tests that pin the Needs attention count or press:**
    - in `D/model/attention_tests.rs`: `counts_what_awaits_the_person`
      (`:172`) and `retire_rows_dismissed` (`:143`);
    - in `D/view/footer_tests/mod.rs`:
      `needs_attention_counts_and_leads_to_activity` (`:279`),
      `footer_controls_do_nothing_behind_a_dialog` (`:334`),
      `toggle_names_follow_their_state` (`:426`) and
      `status_items_say_and_choose` (`:488`);
    - `needs_attention_works_by_enter` and
      `footer_controls_are_reached_by_tab`
      (`D/view/keyboard_tests/shell.rs:377`, `:409`);
    - `drafts_not_saved_with_activity_hidden_focuses_the_item_area`
      (`D/view/workspace_tests.rs:537`).
42. **How desktop tests run.** They are unit tests of the `manyhands` bin
    (`src/main.rs:3`). Window tests need `desktop-test-support`
    (`Cargo.toml:12`).

**Authorities.**

43. **The handoff.**
    - **`03` Activity** (`:99-113`):
      - **Toolbar:** "Show only things that need attention" and `ellipsis`.
      - **Row:** time (40 px, mono), an icon coloured by kind, the
        message with an ellipsis, the repository tag, and an optional
        button.
      - **Tooltip:** the Git detail, never secrets.
      - **Lifetime:** newest first; "failures remain until resolved".
      - **Icon colours:** info secondary, success green, sharing teal,
        failure red, locked amber.
    - **`05` popover** (`:131-140`):
      - **Frame:** 440 px, anchored above the footer item, closed by a
        click outside or Esc.
      - **Header:** "Needs attention" and "Your work is safe on this
        computer. Nothing here happens until you choose."
      - **Rows:** one per unresolved recovery state (icon, title,
        description, button, tag, with the Git detail in the tooltip).
      - **Link:** "Show everything in Activity".
      - **Store:** "a filtered view, not a separate store".
    - **`06`:**
      - the vocabulary;
      - the writing rules: reassure, name the item, the repository and
        the step, one next action, "…" on an action that opens a
        confirmation or sheet, real counts.
    - **The mockup's footer tooltip** is "Things that need you. Your work
      is safe until you act." (`docs/design/mockups/source/Workspace.dc.html:246`).
44. **RFCs and PRD.**
    - **Desktop IA RFC** (`docs/RFC/desktop-information-architecture-and-editor.md`):
      - `:40-55`: Activity plus the popover "over the same records" is
        the persistent operation area, and failures must not exist only
        in transient notifications;
      - `:344-349`: keyboard, focus return, Escape and colour.
    - **Runtime RFC** (`docs/RFC/application-runtime-and-polling.md`):
      - `:44-49`: operations name their repository, item, operation and
        stage; recovery uses typed actions, not raw remote text;
      - `:188-190`: "still stopping" sits in the quit's Activity entry.
    - **PRD:** `MH-UX-001` (`docs/PRD/mvp.md:549`), `MH-NFR-003` (`:601`)
      and `MH-NFR-005` (`:622-623`). The PRD header reads version 0.7.
45. **D5's privacy rule** (`docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md:694-698`):
    "No title, body, comment, side or result text enters an operation
    entry, log or error." Decision 12 amends it for titles.
46. **What Cycle 03a hands on.**
    - Read repository state from `repository_state`; decide the count
      (3a K-9); the footer order; the K-19 rows; Activity and "to share"
      agree (ticket comment `01M78JR8B5RY3XZ0TXTP6ZMTE9`).
    - Cycle 03a's ledger records a deferred Minor: Activity can list an
      item the count no longer has, when an older check is rejected after
      a share (`docs/plans/2026-11-12-wave-04-cycle-03a-repositories-execution.md:286-291`;
      `D/model/polling.rs:320-324`). The unpublished final-review commit
      names "Activity/count agreement". Task 0 records which way it went.
      This design reads the unshared list, not `last.listed`, so its rows
      agree with the count either way (Part A, S6).

## Part A: One Store Of Rows

### A1. Types

New module `D/model/activity.rs`, declared in `D/model/mod.rs`, with:

- `activity/rows.rs` for the types;
- `activity/requests.rs` for S1;
- `activity/states.rs` for S2–S14;
- `activity/log.rs` for A4;
- `activity/copy.rs` for Part B's texts.

It has no GPUI dependency.

```rust
/// What a row is about; its identity across refreshes.
pub(crate) enum RowKey {
    Request(DesktopRequestId),
    Brought { root: RepoKey, seq: u64 },
    Hold(String),                                   // hold_key(&Reason), R-3
    Repository(RepoKey),
    Problem { root: RepoKey, item: Option<ItemId> }, // None is main
    Unshared { root: RepoKey, item: Option<ItemId> },
    RetireOffer { root: RepoKey, item: ItemId },
    Retirement { root: RepoKey, item: ItemId },
    Pending { root: RepoKey, operation: String },    // R-4
    PendingList(RepoKey),
    Flush(DraftId),
    DraftStore,
    Recovered,
    Note(Report),
}

pub(crate) enum Kind { Info, Success, Sharing, ToShare, Failure, Locked, Working }

pub(crate) struct Offer {
    pub action: ActivityAction,
    pub label: String,
    pub enabled: bool,
}

pub(crate) struct ActivityRow {
    pub key: RowKey,
    pub at: OffsetDateTime,
    pub kind: Kind,
    pub title: String,
    pub description: Option<String>,
    pub repo: Option<String>,
    pub button: Option<Offer>,
    pub menu: Vec<Offer>,
    pub detail: Vec<String>,
    pub attention: bool,
    pub item: Option<ItemRef>,
    pub finished: bool,
}
```

**Deriving.**

- `RowKey` derives `Clone, PartialEq, Eq, PartialOrd, Ord, Debug`.
- `Report` (`D/services/inflight.rs:185`) gains `PartialOrd, Ord`.
- `Reason` cannot derive `Ord`: it holds the library's `SharedKeyId`,
  which has no order. So a hold's key is the string `hold_key(&Reason)`
  (R-3).

**Fields.**

- **`kind`** sets the icon and its colour (`03`):
  - info, secondary;
  - success, green;
  - sharing, teal `refresh-cw`;
  - to share, amber `cloud-upload` (`06`);
  - failure, red;
  - locked, amber `lock`;
  - working, a teal spinner (`06`).
- **`title`** and **`description`:** Activity draws
  `title` + " " + `description` on one line, with an ellipsis. The
  popover draws them on two lines (`05`).
- **`repo`** is the tag: `{repo}`, or `{repo} +{n}` for a hold over more
  than one repository (decision 13).
- **`button`** is at most one offer (decision 8). `menu` holds every
  offer, the button's first, then Dismiss when the row can be dismissed.
- **`detail`** holds the tooltip's lines (Part B5); its first line is the
  menu's first line (Part D).
- **`item`** is `(root, AuthoringKind, ItemId)` when the row is about one
  item. Enter opens it when the row has no button (decision 11).
- **`finished`** means "Clear finished" may take the row (A4, rule L8).

```rust
pub(crate) enum ActivityAction {
    Cancel(DesktopRequestId),
    Stop(DesktopRequestId),
    Open(DesktopRequestId),                     // the lifecycle dialog
    Review(DesktopRequestId),                   // the conflict sheet
    Offer(DesktopRequestId, SyncOffer),
    Recovery(DesktopRequestId, usize),          // run_recovery(id, n)
    Dismiss(RowKey),
    Attend(Action, Reason),                     // a hold's action
    Repository(RepoKey, RowAction),             // Cycle 03a's row action
    OpenItem(ItemRef),
    Share { root: RepoKey, item: Option<(AuthoringKind, ItemId)> },
    Retire { root: RepoKey, item: ItemId },
    Resume { root: RepoKey, operation: String },
    ReviewPending { root: RepoKey, operation: String },
    Abandon { root: RepoKey, operation: String },
    RetryPendingList(RepoKey),
    RetryFlush(DraftId),
    ReviewDrafts,
}
```

**Each action runs an existing handler** (Part F4). Nothing new is sent:

| Action | Handler |
| --- | --- |
| Cancel | `cancel_request` |
| Stop | `stop_request` |
| Open | `open_lifecycle` |
| Review | `review_request` |
| Offer | `run_share_offer` |
| Recovery | `run_recovery(id, n)` |
| Dismiss | `dismiss_request`, or the log (A4) |
| Attend | `attend` |
| Repository | `row_action` |
| OpenItem | opening a tab, as a list row does |
| Share | today's Share…, or Share main |
| Retire | `retire_chosen` |
| Resume | `resume` |
| ReviewPending | `pending_review` |
| Abandon | `abandon` |
| RetryPendingList | `load_pending` |
| RetryFlush | `retry_draft_flush` |
| ReviewDrafts | `open_drafts` |

### A2. Sources

Every row comes from exactly one row of this table. The derivation is
`derive(&Inputs) -> Vec<Derived>`: a pure function over the models listed.
A `Derived` is a row without its time, plus a `kept` flag (S8). Abandon is
never the button (decision 8).

| S | Source (fact) | Key | Kind | Event or state | Button | Menu after the button | Attention |
| --- | --- | --- | --- | --- | --- | --- | --- |
| S1 | Each request not Dismissed (6) | `Request` | **Working:** Queued, Running. **Success:** Success, Noop; a share that published is **sharing**. **Failure:** Partial, Blocked, Error, NotSent; a share that needs unlocking is **locked**. **Info:** Cancelled, Removed | Event | **Queued:** Cancel. **Running:** Stop at a safe point when offered, else a disabled "Stopping…" or "Finishing" (fact 14; F3). **Finished, not Success:** the first share offer, else the first recovery action the row shows, else Open… (lifecycle), else Review changes. **Success:** none | Dismiss when finished; the other share offers and recovery actions, Open… and Review changes, as today's entry shows them | N1–N4 |
| S2 | Checks that brought something in (A5) | `Brought` | Sharing | Event | None | Dismiss | Never |
| S3 | Each checking hold (19) | `Hold` | Locked for `KeyLocked`, else failure | State | The hold's action; Unlock… disabled, with its reason in the detail, when it has no target | — | Always |
| S4 | Each listed repository and the kept row (23; A7) | `Repository` | By state (Part B4) | State | The state's `RowAction` | — | N5–N11 |
| S5 | The unshared list's problems (21) | `Problem` | Failure | State | Open, when the target has an ID and a kind | — | Always (N9) |
| S6 | The unshared list's items, main included (21) | `Unshared` | To share | State | Share… for an item with a kind; Share main for main | — | Never |
| S7 | Retire… offers (20) | `RetireOffer` | Info | State | Retire… | — | Never |
| S8 | Retirement lines (20) | `Retirement` | **Working:** Retiring. **Success:** Retired, Finished. **Failure:** didn't finish | Event, `kept` once final | None | Dismiss once final | Never |
| S9 | Pending rows, `pending_shown()` (18) | `Pending` | Failure with a next action, else info | State | Resume when offered, else Review changes when offered, else none | Resume, Review changes and Abandon… as offered; a Resume that is unavailable, disabled, with its reason | N12 |
| S10 | A pending list that is Failed or Busy (3) | `PendingList` | Failure (Failed), info (Busy) | State | Retry | — | Failed only (N13) |
| S11 | Each recovery copy not saved (27) | `Flush` | Failure | State | Retry | — | Always |
| S12 | The draft store unavailable (27) | `DraftStore` | Failure | State | None | — | Always |
| S13 | Recovered drafts (27) | `Recovered` | Info | State | Review | Dismiss | Never |
| S14 | Record notes (27) | `Note` | Info | State | None | Dismiss | Never |

**S4 makes a row only for these states:**

- **Attention rows:**
  - Can't open;
  - Needs fixing;
  - Can't reach;
  - Locked from a stored outcome;
  - Needs attention from a held shape with no reason in its phase,
    `Waiting { Failed }`, an unreadable registration, a `Transport` answer
    or a stored outcome (fact 23, causes 2 and 4–7).
- **Information rows, never counted:**
  - This computer: "{repo} has no shared copy, so changes stay on this
    computer", with Set up sharing (`03`'s example);
  - the non-SSH remote of 3a decision 20.
- **No row** for Checking…, To share, Paused, Up to date, a busy opening
  and Unknown. Their facts are elsewhere (S6), or they need nothing.

**Pinned above the list, never rows** (C1 decision 5): the layout notices
and the quit row.

### A3. Attention: Each Thing Counts Once

The footer's count is the number of attention rows not dismissed. The
popover lists exactly those rows (decision 6). This table is the required
outcome. Task 5 writes its test `attention_counts_each_thing_once` first,
row by row, with a design checkpoint (Plan, Test-first).

| N | Situation | Rows shown | Counted |
| --- | --- | --- | --- |
| N1 | A request finished Partial, Blocked or Error, and its operation has a pending row shown | The request row and the pending row | Once, as the pending row |
| N2 | Two or more failed requests about one operation (`operation_of`, fact 18) | Each request row | Once, as the newest |
| N3 | A failed request that names no operation | Its row | Once |
| N4 | A failed Check now (PollOnce) | Its request row, and its repository's row or hold | Once, as the repository row or the hold |
| N5 | A repository Locked by a live `KeyLocked` | The key's hold row, no repository row | Once per key, however many repositories it holds |
| N6 | A repository whose phase is `Attention(reason)`, or held with `RecoveryHeld` in its phase | The reason's hold row, no repository row | Once per hold |
| N7 | A repository held (`Shape::Held`) with no reason in its phase | A repository row with Resume checking | Once |
| N8 | A repository Can't open, Needs fixing, Can't reach, `Waiting { Failed }`, unreadable, a `Transport` answer, or a stored outcome | One repository row | Once |
| N9 | A repository whose check listed problems | One problem row per item, no repository row | Once per item |
| N10 | A repository whose check found a conflict pending | No repository row; the conflict's pending row or request offers Review changes | As N1 to N3 |
| N11 | A repository row whose repository starts Checking… | The row stays, its button disabled and reading "Checking…" (rule L3) | Still once, until the check answers |
| N12 | A pending row with no next action ("Nothing to do") | The row | No |
| N13 | A pending list Failed; Busy; Loading | A row; a row; nothing | Once; no; no |
| N14 | Recovery copies not saved; the draft store unavailable | One row each; one row | Once each; once |
| N15 | Requests that succeeded, were cancelled or not sent; brought-in rows; reminders; Retire… offers; retirement lines; recovered drafts; record notes; This computer; the non-SSH remote; layout notices; the quit row | As the sources say | Never |
| N16 | Any row dismissed | Not shown | No |

**N4 counts no double failure.** A failed Check now applies its answer to
the repository through `check_failed` (fact 21), so the repository's state
says it.

**N5–N7 are 3a's attention causes.** They are covered by the hold rows,
which give the one action today's attention entry gives.

**N10 relies on the conflict's own row.** That is a pending synchronize
row with Review changes, or the request that stopped on the conflict. A
repository row would offer nothing (fact 23, cause 3), so none is made
(R-6).

### A4. Times And The Log

`ActivityLog` (`D/model/activity/log.rs`), owned by the Workspace, holds:

- `seen: BTreeMap<RowKey, OffsetDateTime>`;
- `kept: BTreeMap<RowKey, ActivityRow>`;
- `dismissed: BTreeSet<RowKey>`;
- the current `rows: Vec<ActivityRow>`.

`refresh(&mut self, derived: Vec<Derived>, checking: &BTreeSet<RepoKey>,
now: OffsetDateTime) -> bool` applies these rules and reports whether any
row changed:

- **L1, first seen.** A key seen for the first time gets `at = now`. A key
  seen again keeps its time, so a retry keeps its row's time and place
  (fact 6).
- **L2, a state leaves.** A state key no longer derived is forgotten, time
  included. If it returns, it is new.
- **L3, checking keeps the row.** A `Repository(root)` row whose root is
  in `checking` is kept as it was, with its button disabled and labelled
  "Checking…", until the check answers. Its time and its count do not
  change while a retry runs (N11; fact 23).
- **L4, a kept event stays.** A `kept` row (S8) no longer derived stays as
  its last row. Its button and menu go, except Dismiss.
- **L5, requests.** A request row leaves only when its request is
  dismissed (fact 6). S1 then no longer derives it.
- **L6, dismissing.** Rows whose key is in `dismissed` are not shown.
  - **Another row's Dismiss** adds its key to `dismissed`.
  - **A request's Dismiss** goes through `dismiss_request`, which sets the
    request Dismissed and forgets its resends and draft (fact 6). It does
    not use `dismissed`.
- **L7, order.** Rows are sorted newest `at` first. Equal times keep
  derivation order: sources in S order, and within a source, the source's
  own order (`requests()` newest first; repositories in list order).
- **L8, "Clear finished".** It dismisses every row that has `finished`
  set and is not an attention row:
  - requests that ended Success, Noop, Cancelled or Removed, through
    `dismiss_request`;
  - brought-in rows and final retirement lines, through `dismissed`.

  Failures, attention rows and states stay (decision 9).
- **L9, no cap** (decision 13).

**Refresh points.**

- `Workspace::refresh_activity(&mut self, cx)` derives, then refreshes
  with `now` from `scheduler.utc()` (fact 22). It notifies only when
  `refresh` reports a change.
- **It is called from:**
  - the Workspace's operations observer (`D/view/workspace.rs:214`);
  - its scheduler observer (`:220`);
  - its drafts observer (`:217`);
  - after the repository list or a resolution is applied;
  - after `ShareTitles` learns titles;
  - after any dismissal.
- **Never from render.** The rows are ready before any frame draws them
  (3a decision 3's rule). Activity hidden changes nothing (exit evidence).

### A5. Checks That Brought Something In

`PollSchedule` gains:

```rust
pub(crate) struct Brought {
    pub seq: u64,
    pub root: Root,
    pub count: u32,
    pub at: OffsetDateTime,
    pub automatic: bool,
}
```

`brought: Vec<Brought>`, read through `brought(&self) -> &[Brought]`.

- `after` (an automatic check) and `checked_now` (a Check now) append one
  when the summary they return has `brought_in > 0`. They take `at` from
  their `&Now`'s `utc` (`D/model/polling/clock.rs:29-32`).
- The list lasts for the session and is never pruned (K-4).
- An automatic one reads "Checked {repo} for updates. Brought in {n}
  change(s)." A Check now's reads "Brought in {n} change(s) to {repo}.",
  because its request row already says "Checked {repo} for updates."
  (Part B).

Quiet checks add nothing (decision 7).

### A6. The Count

`ActivityLog::attention_count(&self) -> usize` counts the shown rows with
`attention`.

- `Workspace::attention` returns it, so `FooterStatus.attention` is the
  popover's row count.
- `needs_attention` and `D/model/attention.rs` are deleted, with their
  tests (E-3).

### A7. Repository Facts, Read Once

`repositories::rows` builds `RepoFacts` inline (fact 23). Task 4 moves
that building, unchanged, into a function both readers call:

```rust
pub(crate) fn each_repository<R>(
    workspace: &Workspace,
    cx: &App,
    f: impl FnMut(&RepoFacts<'_>) -> R,
) -> Vec<R>
```

- It visits the listed repositories in list order, then the kept row.
- `rows()` becomes `each_repository(.., |facts| row_of(facts))`.
- Activity calls it for each repository's `repository_state(facts)`,
  `repository_tooltip(facts, &state)`, `checking()` and schedule.

So Activity reads the same state through the same function, with no
second derivation (Cycle 03a's obligation; R-5).

## Part B: Copy

Every user-visible string is a `pub(crate) const`, or a formatting
function in `D/model/activity/copy.rs` for strings with values. Tests
compare these. The register is `06`'s:

- sentence case;
- no exclamation marks;
- `…` (U+2026) on an action that opens a dialog, sheet or page;
- real counts;
- a failure says the work is safe.

This part is the copy the owner reviews. It maps every row and action of
today's operations area to the log (the ticket's planning item).

### B1. Naming

- **An item:** “{title}”, in the typographic quotes the desktop already
  uses (`D/view/drafts/actions.rs:201`).
  - **The title comes from**, first found:
    1. the intent's own title (a save or create, fact 15);
    2. an open tab's `tab_title()`;
    3. `share_title(root, id)` (fact 25).
  - **Without one:** "the ticket {short}" or "the document {short}", with
    the short code (fact 20; K-2).
  - A title is allowed in a row, its tooltip, its menu and the popover
    (decision 12). A body, comment or conflict side never is.
- **A repository:** `{repo}`, the folder name: the same as
  `RepositoryEntry::name()` and today's `scope_name`.
- **Main:** `{primary}`, the repository's main branch when known, else
  "main" (`06`).
- **A key:** "your key", never its label or path.
- **A host:** its host name. **A remote:** its name, never its URL.

### B2. Requests (S1)

- **Running:** "{Doing} {subject}…". When a stage is known it adds " ·
  {stage words}" (F3).
- **Queued:** "Waiting to {verb} {subject}".
- **Cancelled:** "Stopped {doing, lower case} {subject}".
- **Removed:** "Cancelled: {verb} {subject}".
- **NotSent:**
  - "Didn't {verb} {subject}: your key's passphrase is needed again.";
  - "Didn't {verb} {subject}: Manyhands could not find your SSH key
    store.".
- **Success:** the Done column.
- **Noop:** the Done column with " Nothing needed to change." added,
  unless the table says otherwise.
- **Partial, Blocked and Error:**
  - title "Couldn't {verb} {subject}";
  - description "{envelope message} Your work is safe on this computer."
    The message is the library's fixed sentence for the code (fact 16;
    K-3).
- **ItemSync and RepoSync:** B3 decides the finished copy.
- **TicketClose and DocumentPromote:**
  - finished: the Done column, then the lifecycle answer's outcome lines
    joined with ". " ("Merged into {primary}", "Shared", "Tidied up"),
    as the dialog builds them (`D/view/lifecycle/outcome.rs:362-455`);
  - not succeeded: their button is Open… (fact 16).

| Intent | Subject | Doing | Done | Verb |
| --- | --- | --- | --- | --- |
| RepoCreate | `{repo}` | Creating `{repo}` | Created `{repo}` | create `{repo}` |
| RepoEnable | `{repo}` | Adding `{repo}` | Added `{repo}` to Manyhands | add `{repo}` |
| RepoRemove | `{repo}` | Removing `{repo}` from Manyhands | Removed `{repo}` from Manyhands. Its files are unchanged. | remove `{repo}` from Manyhands |
| RepoIdentitySet | `{repo}` | Setting your name for `{repo}` | Set your name for `{repo}` | set your name for `{repo}` |
| RemoteAdd | remote name, `{repo}` | Adding the shared copy {remote} to `{repo}` | Added the shared copy {remote} to `{repo}` | add the shared copy {remote} |
| RemoteRemove | remote name, `{repo}` | Removing the shared copy {remote} | Removed the shared copy {remote} from `{repo}` | remove the shared copy {remote} |
| RemoteSelect | remote name or none, `{repo}` | Choosing how `{repo}` shares | `{repo}` now shares through {remote}; with none, "`{repo}` no longer shares" | choose how `{repo}` shares |
| IndexRefresh | `{repo}` | Refreshing `{repo}` | Refreshed `{repo}` | refresh `{repo}` |
| IndexRebuild | `{repo}` | Rebuilding the search index for `{repo}` | Rebuilt the search index for `{repo}` | rebuild the search index for `{repo}` |
| PollOnce | `{repo}` | Checking `{repo}` for updates | Checked `{repo}` for updates. | check `{repo}` for updates |
| PollConfigure | `{repo}` | Changing how often `{repo}` is checked | Changed how often `{repo}` is checked | change how often `{repo}` is checked |
| PollPause | `{repo}` | Pausing checks of `{repo}` | Paused checking `{repo}` for updates | pause checking `{repo}` |
| PollResume | `{repo}` | Resuming checks of `{repo}` | Resumed checking `{repo}` for updates | resume checking `{repo}` |
| RepoSync | `{primary}`, `{repo}` | Sharing {primary} | B3 | share {primary} |
| FolderCreate | the folder's path | Creating the folder {path} | Created the folder {path} | create the folder {path} |
| KeyGenerate | your key | Creating a key | Created a key for sharing | create a key |
| KeyImport | your key | Adding a key | Added a key for sharing | add the key |
| KeySelect | your key | Choosing the key for sharing | Chose the key used for sharing | choose the key |
| KeyClear | your key | Clearing the key choice | No key is chosen for sharing now | clear the key choice |
| KeyRemove | your key | Removing a key from Manyhands | Removed a key from Manyhands. Its files are unchanged. | remove the key from Manyhands |
| KeyDelete | your key | Deleting a key | Deleted a key | delete the key |
| HostApprove | {host} | Trusting {host} | Trusted {host} for sharing | trust {host} |
| HostReplace | {host} | Updating the trusted key for {host} | Updated the trusted key for {host} | update the trusted key for {host} |
| OperationResume | `{repo}` | Finishing an operation in `{repo}` | Finished an operation in `{repo}` | finish the operation in `{repo}` |
| OperationAbandon | `{repo}` | Abandoning an operation in `{repo}` | Abandoned an operation in `{repo}` | abandon the operation in `{repo}` |
| ConflictResolve | `{repo}` | Saving your choices in `{repo}` | Resolved a conflict in `{repo}` | resolve the conflict in `{repo}` |
| LocalContextResolve | `{repo}` | Saving your choices in `{repo}` | Resolved a conflict on this computer in `{repo}` | resolve the conflict in `{repo}` |
| DocumentSave, TicketSave | item | Saving “{title}” | Saved “{title}” on this computer (`06`) | save “{title}” |
| DocumentCreate, TicketCreate | item | Creating “{title}” | Created “{title}” on this computer | create “{title}” |
| DocumentMove | item | Moving “{title}” | Moved “{title}” | move “{title}” |
| DocumentPromote | item | Approving and merging “{title}” | Approved and merged “{title}” | approve and merge “{title}” |
| TicketClose | item | Closing “{title}” | Closed “{title}” | close “{title}” |
| ContextPrepare | item | Getting a private copy of “{title}” ready (`06`) | A private copy of “{title}” is ready | get a private copy of “{title}” ready |
| ContextRelease | item | Putting away the private copy of “{title}” | Put away the private copy of “{title}” | put away the private copy of “{title}” |
| ContextRetire | item | Retiring the copy of “{title}” | Retired the copy of “{title}” on this computer | retire the copy of “{title}” |
| CommentAdd | item | Posting a comment on “{title}” | Posted a comment on “{title}” | post the comment on “{title}” |
| ItemSync | item | Sharing “{title}” | B3 | share “{title}” |
| ItemRepublish | item | Republishing “{title}” | Republished “{title}” | republish “{title}” |
| LocalContextSync | item | Repairing “{title}” on this computer | Repaired “{title}” on this computer | repair “{title}” |
| DocumentRepair, TicketRepair | the path | Repairing {path} | Repaired {path} | repair {path} |

### B3. A Finished Share

The classification is made once (F2). Words that exist today are kept
(fact 17). The subject is “{title}” for an item, or {primary} for main.

| `SyncOutcome` | Kind | Title | Description |
| --- | --- | --- | --- |
| Published | sharing | Shared {subject} | — |
| Current | success | {subject} is the same as the shared copy | — |
| Stopped { completed: false } | info | Stopped sharing {subject} | Stopped before anything was sent. Your saved work is still on this computer. |
| Stopped { completed: true } | info | Stopped sharing {subject} | Stopped after part of the work finished. Your saved work is on this computer; Share sends the rest. |
| Conflict | failure | You and someone else both changed {subject} | Choose what to keep. Nothing is shared until you do. (`05`; fact 26) |
| BranchDeleted | failure | The shared copy of {subject} was deleted | Republish it to share it again. A republish running reads "Republishing…". |
| HistoryUnresolved | failure | Couldn't confirm the shared history of {subject} | Today's sentence |
| NeedsUnlock | locked | Couldn't share {subject}: your key is locked | Unlock it, then share again. |
| NeedsKey | failure | Couldn't share {subject}: choose a key to share with | — |
| NeedsHost | failure | Couldn't share {subject}: check {host} before sharing | — |
| NeedsIdentity | failure | Couldn't share {subject}: set your name first | — |
| Busy | failure | Couldn't share {subject} yet | "May be waiting for {label}", or "Another share may be holding this repository" (read at render, R-9) |
| NoRemote | info | {repo} has nowhere to share {subject} to | — |
| NothingToShare | info | Nothing to share in {subject} yet | It has no saved changes. |
| Failed | failure | Couldn't share {subject} | The envelope message, as B2 |

- **The button** is the first of `shown_offers` (fact 12), in today's
  order.
- **The menu** holds the rest. Today's labels are kept: "Share", "Retry",
  "Stop trying to share", "Unlock…", "Open Sharing & keys", "Check host",
  "Set identity", "Show it", "Set up sharing", "Republish…" and "Stop and
  republish…".
- **"Already finished"** (a Give up's replayed answer) goes in the detail.

### B4. States (S2–S14)

| S | Title | Description | Button |
| --- | --- | --- | --- |
| S2 automatic | Checked `{repo}` for updates. Brought in {n} change(s). | — | — |
| S2 Check now | Brought in {n} change(s) to `{repo}`. | — | — |
| S3 KeyLocked | Your key is locked (`05`) | Checking `{repos}` is waiting for the passphrase. | Unlock… |
| S3 KeyProblem, NoKeySelected | Checking needs a key to be chosen | Holding back `{repos}`. | Choose a key… |
| S3 HostUnapproved | {host} needs your approval | Checking `{repos}` waits until you approve it. | Approve host… |
| S3 HostUnidentified | A server needs your approval | Checking `{repo}` waits until you approve it. | Approve host… |
| S3 ConfigurationRequired | Sharing isn't set up for `{repo}` | Checking can't start until it is. | Open remote settings… |
| S3 RecoveryHeld | Checking `{repo}` is held until recovery | Your work is safe. | Resume checking |
| S4 Can't open | Can't open `{repo}` | 3a's sentence | Retry |
| S4 Needs fixing | `{repo}` needs fixing | 3a's sentence | Settings… |
| S4 Can't reach | Couldn't reach `{repo}`'s shared copy (`05`) | Your work is safe, and Manyhands tries again. | Retry now |
| S4 Locked, stored | Checking `{repo}` needs your key's passphrase | Check now asks for it. | Check now |
| S4 Needs attention | Checking `{repo}` needs attention | 3a's sentence for the cause | 3a's action for the cause |
| S4 This computer | `{repo}` has no shared copy, so changes stay on this computer (`03`) | — | Set up sharing… |
| S4 non-SSH | 3a's sentence (3a decision 20) | — | Set up sharing… |
| S5 | “{title}” needs a look, or "{primary} in `{repo}` needs a look" | The disposition's sentence below | Open |
| S6 | “{title}” isn't shared yet, or "{primary} in `{repo}` isn't shared yet" | — | Share…, or Share {primary} (3a's label) |
| S7 | “{title}” is closed, but its copy is still on this computer; a promoted document reads "is approved, but …" | — | Retire… |
| S8 | Today's retirement lines with “{title}” in place of the code: "Retiring “{title}”…", "Retired the copy of “{title}” on this computer.", "Finished retiring “{title}”.", "Retiring “{title}” didn't finish." | " The lists are still catching up." when today's adds it | — |
| S9 | "{action words} didn't finish in `{repo}`", or "{action words} “{title}” didn't finish" when the item is known | the next action's sentence below | Resume, or Review changes |
| S10 Failed | Couldn't read the unfinished operations of `{repo}` | — | Retry |
| S10 Busy | Manyhands was busy reading the unfinished operations of `{repo}` | — | Retry |
| S11 | The recovery copy of “{title}” wasn't saved | {reason} (today's `DraftError` words) | Retry |
| S12 | Recovery copies are unavailable | {reason} | — |
| S13 | {n} draft(s) recovered from an earlier session | — | Review |
| S14 | Today's record-note sentences | — | — |

**The repository rows.**

- **Description:** the state's own `sentence` lines from
  `repository_state` (fact 23), joined, when they say more than the
  title.
- **Tooltip:** `repository_tooltip`.
- **Button label:** the label Cycle 03a's menu gives the `RowAction`.
- **"{repos}":** the held repositories' names, joined with ", ". The tag
  is `{repo} +{n}` (decision 13).

**S5 dispositions:**

| Disposition | Sentence |
| --- | --- |
| Dirty | It has changes on this computer that aren't saved yet. |
| Conflicted | It has a conflict to resolve. |
| Inaccessible | Manyhands can't read it. |
| Nonconforming | It needs fixing before it can be shared. |
| BlockedByLocalState | Something on this computer is in the way of updating it. |
| MaterializationIncomplete | Bringing it in didn't finish. |
| Malformed | Its file can't be read as a ticket or document. |
| RemotelyDeleted | Its shared copy was deleted. |
| HistoryUnknown | Manyhands can't confirm its shared history. |
| NotReached | The check didn't reach it this time. |

**S9 action words** (`OperationAction`, `L/repository/read/dto.rs:756-781`):

| Action | Words |
| --- | --- |
| create_and_enable, enable | Adding a repository |
| remove_registration | Removing a repository |
| add_remote | Adding a shared copy |
| remove_remote | Removing a shared copy |
| set_publication_remote | Choosing how it shares |
| refresh | Refreshing |
| rebuild | Rebuilding the search index |
| prepare_context | Getting a private copy ready |
| release_context | Putting away a private copy |
| retire_context | Retiring a copy |
| save_document | Saving a document |
| save_ticket | Saving a ticket |
| submit_comment | Posting a comment |
| repair_item | Repairing an item |
| context_sync_local | Repairing a copy on this computer |
| poll | Checking for updates |
| synchronize_context | Sharing |
| synchronize_primary | Sharing {primary} |
| promote | Approving and merging |
| close | Closing |
| generate_key | Creating a key |
| delete_key | Deleting a key |

An action with no entry reads "An operation". The table is total at
`b37d2c2b`.

**S9 next actions:**

| Next action | Sentence |
| --- | --- |
| resume | Resume finishes it. |
| retry_generation | Resume tries again. |
| review_deletion_again | Review it again. |
| inspect_retained_files | Some files were kept for you to look at. |
| none | Nothing to do. |

The disabled Resume keeps today's reason: "Synchronization is not
available yet".

### B5. Tooltip And Detail

`detail` lines, in order:

1. The time in full: "{YYYY-MM-DD HH:MM}", local, or UTC followed by
   " UTC" when the offset is unknown (B6).
2. By source:
   - **A request:**
     - "{intent label} · {scope}" (today's fixed label and scope);
     - the outcome label ("Done", "Failed" …);
     - "{code}: {message}" when it did not succeed;
     - "Effects: {effects_text}" when any effect was requested;
     - "Operation {id}" when known;
     - the stage while running;
     - "Already finished" for a Give up's replayed answer.
   - **A hold:** today's headline and "Holding back {names}".
   - **A repository:** `repository_tooltip`.
   - **A problem:** "{disposition}: {library guidance}" (fact 20).
   - **A reminder:** "Not shared yet: {disposition}".
   - **A pending row:** its raw action, state, completed step, operation
     ID, next action and `updated_at`.
   - **A draft or note row:** its kind.

**What a row never holds** (Part G): a body, a comment, a conflict side,
a remote URL, a key's label or path, a passphrase, or a server's own
text.

### B6. Time

The row shows "HH:MM".

- `activity_time(at: OffsetDateTime, offset: Option<UtcOffset>) ->
  (String, String)` gives the row's time and the detail's first line.
- The view passes `local_offset(at.unix_timestamp())` (fact 37).
- With no offset (non-Unix today, fact 37), both are UTC and the detail
  says " UTC" (K-5).

## Part C: The Activity Panel

New module `D/view/activity.rs`, with `activity/row.rs`, `activity/menu.rs`
and `activity/actions.rs`, declared in `D/view/mod.rs`. It replaces the
session and pending columns (fact 2, fact 3).

### C1. Layout

`Workspace::activity_region` draws, top to bottom:

1. **The layout notices**, as today (C1 decision 5; R-18).
2. **The quit row**, as today, with its ids and texts (R-18).
3. **The list**, in a container that keeps the id `"operation-area"`
   (R-20).

**The panel header's toolbar.** `ToolbarButton::of(PanelKind::Activity)`
becomes `[ActivityFilter, ActivityMore]` (fact 34).

- **ActivityFilter** shows `list-filter`. Its accessible name and tooltip
  are "Show only things that need attention" (`03`). It carries its state
  as `aria_toggled` on a wrapper, like the footer toggles.
- **ActivityMore** shows `ellipsis`. It opens a Workspace-owned menu,
  like Cycle 03a's More, with one item, "Clear finished", disabled when
  no row is `finished` (rule L8).
- **The filter** is `Workspace.activity_filter: bool`. It is off at
  launch and lasts for the session. When on, the list shows only
  attention rows.

**One row**, one line, at least 24 px high (`03`), in this order:

- the time (B6), 40 px, monospace, muted;
- the kind's icon;
- the message, `title` then `description`, with an ellipsis, taking the
  free width (`flex_1`, `min_w_0`);
- the tag;
- the button.

The button never leaves the row: the message shrinks first. That fixes
K-19's rows at any width (fact 7).

- **Row tooltip:** `detail` joined with newlines. That is also the row's
  accessible description, and its accessible name is the message (as
  Cycle 03a's rows, fact 30).
- **Ids:** `("activity-row", ix)` for a row and `("activity-button", ix)`
  for its button. The pane maps an index to a `RowKey`.

**Empty states:**

- "Nothing yet this session", with the filter off;
- "Nothing needs attention", with it on.

### C2. The List

The panel draws its own list with GPUI's `uniform_list` (through
`gpui_kit::*`). It does not use the Kit `List`, which presses Confirm on
every click of a row (`C/list/list.rs:503-514`), so a click would run the
row's button (R-36). The list keeps its own cursor (the highlighted row)
and binds its keys in a new context, `ACTIVITY_CONTEXT = "Activity"`, in
`D/keys.rs`:

| Key | Does |
| --- | --- |
| `up`, `down` | Moves the highlight; stops at the ends, with no wrap |
| `home`, `end` | Highlights the first or last row |
| `enter` | Runs the highlighted row's button when enabled; with no button, opens the row's item; otherwise nothing (decision 11) |
| `shift-f10`, `menu` | Opens the highlighted row's menu (Part D) |
| A left click on a row | Highlights it and focuses the list |
| A left click on its button | Runs the button |
| A right click | Highlights the row and opens its menu |

- **Applying rows.** The pane keeps the highlight on the same `RowKey`
  across refreshes. When that row leaves, the highlight goes to the row
  now at its position, or the last row. Rows are applied to the pane
  after the frame (`window.defer`, as fact 30). Nothing notifies during
  render.
- **Focus.**
  - The list has its own focus handle, inside the `Region::Operations`
    region, so the region's ring and F6 behave as today. Focusing the
    region (`secondary-4`, F6, the footer, Settings' "Show in Activity")
    focuses the list.
  - Tab runs through the two toolbar buttons, the list and the
    highlighted row's button. Rows' other buttons are not tab stops
    (R-33).

### C3. Where Focus Goes

- **An action that opens a dialog or sheet from a row** (Abandon…,
  Republish…, a recovery that confirms, Open…, Review changes) passes the
  list's focus handle as the origin or restore. When the dialog closes,
  focus returns to the list, still highlighting the row's key (R-25).
  This replaces the focus maps of fact 8, which go with the old columns.
- **"Show it"** reveals Activity, turns the filter off when the holder's
  row is hidden by it, and highlights and focuses the holder's row.
  `show_holder`'s fallback to the pending rows highlights the first
  pending row of the repository.
- **Drafts not saved's Cancel** (fact 8) highlights the failed draft's
  Flush row and focuses the list when Activity is displayed. Otherwise
  it focuses the item area, as today.
- **Quit now's focus** is C1 decision 7's, unchanged.

## Part D: The Row Menu

The row menu follows Cycle 03a's repository menu (fact 29) in
`D/view/activity/menu.rs`.

```rust
pub(crate) struct ActivityMenu {
    pub(crate) key: RowKey,
    view: Entity<PopupMenu>,
    at: Point<Pixels>,
    pub(crate) context: FocusHandle,
    pub(crate) restore: Option<FocusHandle>,
    pub(crate) chosen: bool,
    _dismissed: Subscription,
}
```

- **Items:**
  1. the row's `detail` lines after the time, each as a disabled item
     (decision 8; R-27);
  2. a separator;
  3. the row's offers, the button's first, each enabled or disabled as
     the row says;
  4. Dismiss, when the row can be dismissed.
- **The choice** is a Kit `Action`, `ActivityMenuChoice { key: RowKey,
  action: ActivityAction, shown: String }`, in `D/keys.rs` beside
  `RepositoryMenuChoice`.
  - When chosen, the Workspace finds the row by `key` in the log, and runs
    the action only if the row still offers an enabled offer with the
    same action and label.
  - Otherwise nothing runs, and the menu is still marked chosen (3a R5-7).
- **Opening:**
  - by a right press on a row (`capture_any_mouse_down`,
    `prevent_default`, as fact 30), anchored at the pointer;
  - by `shift-f10` or `menu` in `ACTIVITY_CONTEXT`, for the highlighted
    row, anchored at the row's bounds, else the list's.
- **Closing and dropping:**
  - It is dropped while a dialog or sheet is open, and when its row
    leaves the log (focus to the list).
  - Escape restores focus to where it was.
  - Opening it closes the popover (R-13).
- **Test accessors:** `activity_menu_key()` and `activity_menu_open()`.
  Every Kit menu has the same element id (fact 33).

## Part E: The Needs Attention Popover

### E1. The Footer Item

- **Pressing Needs attention** (click, Enter or Space) toggles the
  popover. Today it reveals Activity (fact 28). The press keeps today's
  guard: nothing happens while a dialog or sheet is open.
- **Bounds:** its bounds are recorded like a panel toggle's (`Footer.bounds`,
  `D/view/footer.rs:399-412`). The popover anchors to them.
- **Tooltip:** "Things that need you. Your work is safe until you act."
  (fact 43).
- **Hiding while focused:** when it hides because the count reached 0,
  focus moves to the control before it in its group: To share, else
  Update check, else the right dock's first panel toggle. So
  `focus_after_hiding(Attention, ..)` names that control, and
  `keep_focus_in_footer` covers Attention (R-37).
- **Label:** unchanged ("1 needs attention", "{n} need attention").

### E2. The Layer

`AttentionPopover` lives in `D/view/needs_attention.rs`, owned by the
Workspace as `Option<AttentionPopover>`, with:

- `focus: FocusHandle`;
- `cursor: Option<RowKey>`;
- `restore: FocusHandle`, the footer item's.

**Drawing.**

- `Workspace::render` mounts it after the dock menu and the repository
  menu (fact 29). It is a `deferred(anchored()…)` at `POPUP_PRIORITY`,
  positioned at the footer item's bounds' top right, with
  `Anchor::BottomRight` and an 8 px margin, so it opens above the item,
  right-aligned to it (`05`).
- It is 440 px wide. It is no taller than the window less the title bar
  and footer, and scrolls.
- **Contents:**
  - the header "Needs attention", with "Your work is safe on this
    computer. Nothing here happens until you choose." (`05`);
  - one row per attention row of the log, newest first: icon, title,
    description, button and tag, with the row's detail as its tooltip
    and accessible description;
  - the link "Show everything in Activity".
- **Ids:** `"attention-popover"`, `("attention-row", ix)`,
  `("attention-button", ix)`, `"attention-show-all"`.

**Keys,** in a new context `ATTENTION_POPOVER_CONTEXT`:

| Key | Does |
| --- | --- |
| `up`, `down`, `home`, `end` | Moves the highlight, with no wrap |
| `enter` | Runs the highlighted row's button, or opens its item when it has none |
| `tab`, `shift-tab` | Cycles the highlighted row's button and the link, staying inside the popover (R-12) |
| `escape` | Closes it, with focus back to the footer item |

**Opening and closing.**

- **Opening** focuses the popover, with the highlight on its first row.
- **Running a row's action** closes the popover first, then runs the
  action with the footer item's handle as origin or restore. A dialog or
  sheet it opens returns focus there. If the item has hidden by then, E1's
  rule applies.
- **"Show everything in Activity"** closes the popover, turns the filter
  off, reveals Activity, and focuses its list with the highlight on the
  first row.
- **It also closes:**
  - on a press outside it (`on_mouse_down_out`). That press still
    reaches what is under it (R-11), unless it is the footer item, whose
    own press toggles (U-3);
  - when the count reaches 0, with focus moved by E1's rule if it was
    inside;
  - when a dialog or sheet opens (dropped, as the menus are, fact 29);
  - when a row menu, the dock menu or the repository menu opens (R-13).
- **With Activity hidden,** the popover reads the log, which does not
  depend on Activity being drawn (A4), so it works (exit evidence).

## Part F: Folded-In Work

### F1. Splitting `model/operations.rs`

This folds in the request of ticket `01M6H7VXJJHRYHTW9S8TJ88921` (item 6)
and ticket `01M6RAS63EWVHWWVBSZ836RXG2`. The moves are pure, with no
behaviour change. `operations.rs` re-exports every moved item with
`pub(crate) use`, so no caller and no test path changes (R-34).

| New file | Takes (fact 11 line ranges) |
| --- | --- |
| `model/operations/progress.rs` | `Progressing`, `ShareTarget::of`, `stage_text`, `progress_words`, `stopped_text` (722-841) |
| `model/operations/recovery.rs` | `RecoveryStep`, `parse_authority`, the recovery `impl`, `RecoveryControl`, `recovery_control`, `RETRY_SHARING`, `recovery_label` (843-1076, 1211-1272) |
| `model/operations/pending.rs` | `pending`, `load_pending`, `apply_pending`, `retain_pending`, the pending-row predicates, `ResumeOffer`, `resume_offer`, `abandon_offered` (652-708, 1078-1209) |
| `model/operations/lifecycle.rs` | `lifecycle_answer`, `lifecycle_answered`, `locks`, `locks_mut` (501-550) |

`operations.rs` keeps the struct, `Changed`, requests, `apply`, stop
offers and shares. It ends near 600 lines, and each new file stays under
500.

### F2. A Share Classified Once

This folds in item 1 of ticket `01M6H7VXJJHRYHTW9S8TJ88921`.

- **Recording the attempt.** `Operations` gains:
  - `share_attempts: HashMap<DesktopRequestId, (RecordState, bool)>`;
  - `share_results: HashMap<DesktopRequestId, SyncResult>`;
  - `note_share_attempt(id, record: RecordState, after_give_up: bool)`.

  The drain calls `note_share_attempt` for a `Finished(id, _)` whose
  request has a `ShareTarget`, before `apply`, beside `note_record`
  (fact 13). It passes `record_state(id).unwrap_or(RecordState::Unknown)`
  and `gave_up(id).is_some()`.
- **Classifying.** `apply(Finished)` stores the envelope as today, then
  classifies it once into `share_results[id]` with `sync_outcome`. With no
  noted attempt it uses `Unknown` and `false`, today's defaults for a
  request the worker never ran (fact 13).
- **Clearing.** `apply(Queued)` for a retry clears both maps, as it clears
  `shares`.
- **Reading.** `share_result(&self, id) -> Option<&SyncResult>` returns
  the stored result while the request is Finished. The view's
  `share_result`, `finished_share`, the item tab's `share_outcome` and
  `run_share_offer` read it.
- **Still read at render:** `share_words`' Busy holder and
  `shown_offers`' republish state, which describe other requests now
  (R-9).
- **This is R-8.** The `Finished` event keeps its shape, so the 36 test
  sites that build it (fact 13) do not change.

### F3. The Running Stage Is Shown

This folds in an item of ticket `01M5CS4MH25QQ769HV61ZRKHXG`.

- **New stage words.** `stage_words(stage: ProgressStage, primary:
  Option<&str>) -> &'static str`, in `model/operations/progress.rs`,
  gives words to every intent outside `Progressing`. They are
  `stage_text`'s share column with no remote (fact 14), except that
  Writing reads "Saving…".
- **`state_label`.**
  - `Running { stage: Some(s), .. }` reads "Running: {words}", whether
    accepted or not.
  - `Running { accepted: true, stage: None }` still reads "Finishing".
  - Settings' key page, which also uses `state_label` (fact 14), follows.
- **A running request row** reads "{Doing} {subject}… · {words}" (B2).
- **The disabled "Finishing" control is kept.** It becomes the row's
  disabled button, for an accepted request with no Stop on offer.

### F4. The Old Columns Go

Task 8 removes the drawing code:

- `session_section`, `pending_section` and the entry, pending, attempt,
  attention, record and recovered drawing in `D/view/operations.rs` and
  its submodules;
- the focus maps of fact 8, with `prune_abandon_focus`.

Every handler of A1's table stays, now called from
`Workspace::run_activity_action(&ActivityAction, origin: FocusHandle,
window, cx)` in `D/view/activity/actions.rs`. `D/view/operations.rs`
keeps the handlers and the id-free helpers (`outcome_label`,
`state_label`, `scope_name`, `effects_text`) that other code uses (fact
14), and drops below 500 lines.

## Part G: Privacy

**What a row may hold** (contract 7). A row, its tooltip and detail, its
menu, its tag and a popover row hold only:

- the desktop's fixed copy;
- titles (decision 12);
- repository, remote, host and branch names;
- result codes and the library's fixed messages (fact 16);
- effect values;
- operation IDs and item IDs;
- times and counts.

They never hold:

- a body, a comment or a conflict side;
- a remote URL (`RemoteAdd`'s is never shown);
- a key's label or path;
- a passphrase;
- a server's text.

**Sweeps** (Task 10):

- **D5's sweeps** (`D/view/lifecycle/privacy_tests.rs`,
  `privacy_promote_tests.rs` and `privacy_remote_tests.rs`):
  - They gain a title-only allowance: the `TITLE` and `READY` sentinels
    (both titles) may appear inside `"operation-area"`, `"attention-popover"`
    and a row menu.
  - `BODY`, `COMMENT_TEXT`, `BASE` and `THEIRS` stay forbidden
    everywhere they are today.
  - The four tests still assert that the operation area was read
    (fact 40).
- **`polling_privacy`:**
  - Its surfaces follow the new ids. Its `read_at` pairs keep their
    meaning: a hold row's "Unlock…", a pending row's "Resume" and its
    menu's "Abandon…", the quit row's words, and a request row's code in
    its detail.
  - It adds a surface for the log's rows read from the model: every
    title, description, detail line, offer label and tag, so tooltips
    are swept without hovering.
- **The drafts sweep** expects the new S11 title.

**Document amendment.** D5's privacy rule gets a dated amendment
(Document Amendment 6).

## Part H: Keyboard And Focus

| Surface | By keyboard |
| --- | --- |
| Activity | `secondary-4` or F6 to the list; arrows, Home and End; Enter presses the button or opens the item; `shift-f10` or `menu` opens the row menu; Tab reaches the filter, ⋯, the list and the highlighted row's button |
| Row menu | The Kit menu's keys; Escape returns focus |
| ⋯ menu | As Cycle 03a's More |
| Footer Needs attention | A tab stop; Enter or Space toggles the popover |
| Popover | Arrows, Home, End and Enter; Tab cycles inside; Escape returns to the footer item |
| After a dialog | From Activity: the list, the row still highlighted. From the popover: the footer item, or E1's fallback |
| Behind a dialog or sheet | Nothing acts: every new action goes through `unless_dialog`, and every layer is dropped (R-60, fact 36) |

Every control has an accessible name and visible focus (RFC 344-349).
Color is never the only signal: every row has an icon and words.

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | Each source S1–S14, in each of its states | Its rows, with key, kind, title, description, button, menu and attention as A2 and B2–B4 say |
| P2 | Each row of A3's table, N1–N16 | The rows shown and the count it says, read from the log and from the footer |
| P3 | A4's rules L1–L9: a first sighting, a retry, a state that leaves and returns, a repository that starts and ends a check, a final retirement line that goes, a dismissal, equal times, "Clear finished", many rows | Each rule's result; a retried request keeps its time and place; a dismissed request that is retried comes back (fact 6) |
| P4 | Every intent of B2's table, in each request state | Its title and description |
| P5 | Each `SyncOutcome`, with a record Accepted, Finished and Unknown, and after a Give up | B3's words; the button is the first offer, the menu holds the rest in today's order |
| P6 | Each state row of B4, each disposition and each pending action and next action | Its words; the pending action table is total |
| P7 | A time with a known offset, an unknown one, and around midnight | "HH:MM" and the detail's full time; " UTC" with no offset |
| P8 | An automatic check and a Check now that brought in changes; ones that brought in nothing; a failed one | One brought-in row each for the first two, with their two sentences; none for the rest |
| P9 | A finished share whose worker record later changes; a retry; a fabricated `Finished` with no noted attempt | The result classified when it finished is the one shown; the retry clears it; `Unknown` and no Give up |
| P10 | A running request with a stage, of an intent with stage words and of one without; accepted with no stage; accepted with no Stop | Its words; "Finishing"; the disabled "Finishing" button |
| P11 | The operations model split | Every existing test of `D/model/operations*` passes unchanged |
| P12 | `each_repository` | Cycle 03a's panel tests pass unchanged; a repository row's state equals `repository_state` of the same facts |
| P13 | A journey with one row of each counted kind, then each resolved | The footer's count, the popover's rows and the attention rows agree at every step |
| P14 | Activity with rows; with the filter on; with none | The rows newest first; only attention rows; the two empty texts |
| P15 | Activity by keyboard | Up and down stop at the ends; Home and End; Enter presses the button, opens the item when there is none, or does nothing; `shift-f10` and `menu` open the menu; Tab runs filter, ⋯, list, highlighted row's button |
| P16 | A left click on a row; on its button; a right click | Highlights only; runs the button once; opens the menu for that row |
| P17 | The row menu by pointer and by keyboard; a choice whose row changed in between | The detail lines (disabled), the offers, Dismiss; nothing runs, and the menu closes |
| P18 | Each `ActivityAction` kind, from a row's button and from its menu | Its handler of A1's table, sending exactly what today's control sends |
| P19 | "Clear finished" with finished successes, a failure, an attention row and a state | Only the finished rows that need nothing go; disabled when there are none |
| P20 | Abandon…, a recovery that confirms, and Open… chosen from a row, then cancelled | Focus back in the list, the same row highlighted |
| P21 | Activity at 320 px wide, with the longest message of each kind | Every row's button lies inside the list's bounds |
| P22 | The footer item by click, Enter and Space, twice | The popover opens with the attention rows, newest first, then closes |
| P23 | The popover by keyboard, for each action kind it offers (Unlock…, Retry now, Retry, Review changes, Resume, Approve host…, Resume checking, Open) | Arrows, Home and End move; Enter runs the action after closing the popover; Tab stays inside; Escape returns to the footer item |
| P24 | The popover with Activity hidden; "Show everything in Activity" | It works; Activity is shown with the filter off and its list focused |
| P25 | The count reaching 0 while the popover is open; a dialog or sheet opening; another layer opening; a press outside; a press on the footer item | It closes each time; focus moves by E1's rule when it was inside; the footer press closes it once and does not reopen it |
| P26 | The focused footer item hiding | Focus moves to To share, else Update check, else the right dock's first toggle |
| P27 | The toolbar, the row menu keys, Enter in the list, the footer item and the popover keys behind a dialog; behind a sheet | Nothing acts |
| P28 | Sentinels in a title, body, comment, conflict side, remote URL, key label and passphrase, through every surface | Only the title appears, and only where Part G allows |
| P29 | Rows arriving while Activity is hidden, then Activity shown | The same rows and times as if it had been shown |
| P30 | A check that answers while the repository's row is shown, and a retry that starts | The row stays through the check (rule L3) and leaves or stays by the answer |

## Testing

**Layers** (`AGENTS.md`, Testing Strategy):

- **Model tests** (default):
  - `D/model/activity_tests.rs` (and the submodules' tests) hold one table
    per function: `each_source_makes_its_rows` (P1),
    `attention_counts_each_thing_once` (P2, test-first),
    `the_log_keeps_times_and_events` (P3), `request_copy_table` (P4),
    `share_copy_table` (P5), `state_copy_table` (P6) and
    `activity_time_formats` (P7);
  - `brought_in_is_kept_for_the_session` (P8) in the polling model's
    tests;
  - `shares_are_classified_once` (P9) and `stages_are_shown` (P10) in the
    operations model's tests.
- **Window tests** (`desktop-test-support`):
  - **`D/view/activity_tests.rs`:**
    - `rows_draw_from_the_log_and_filter` (P14), `activity_by_keyboard`
      (P15), `rows_by_pointer` (P16);
    - `the_row_menu_by_pointer_and_keyboard` (P17),
      `each_action_runs_its_handler` (P18),
      `clear_finished_keeps_failures_and_states` (P19);
    - `a_dialog_from_a_row_returns_focus_to_its_row` (P20),
      `rows_fit_a_narrow_activity` (P21),
      `activity_hidden_keeps_the_log` (P29),
      `a_checking_repository_keeps_its_row` (P30),
      `count_and_popover_agree` (P13).
  - **`D/view/needs_attention_tests.rs`:**
    - `the_popover_opens_lists_and_closes` (P22),
      `popover_actions_by_keyboard` (P23);
    - `the_popover_works_with_activity_hidden` (P24),
      `the_popover_closes` (P25),
      `nothing_acts_behind_a_dialog_or_sheet` (P27).
  - **Other:** P26 is in the footer's status table (E-13); P28 is the
    sweeps (E-14).
- **Test-first:**
  - `attention_counts_each_thing_once` is written from A3's table before
    any of its code (Plan, Test-first);
  - every other behaviour's test is seen to fail first;
  - no appearance tests: nothing asserts colours, icons, sizes or that a
    view renders. The one bounds assertion, P21, is the reachability of
    a control (K-19).

**The test seam** (R-21). Fact 38 counts 494 lines in 54 files that press,
read or find operations-area controls by element id. Task 6 adds
`D/view/activity_support.rs` (`all(test, feature =
"desktop-test-support")`). Its functions work by row key and offer label,
not element id:

- `row(shell, &RowKey) -> Option<ActivityRow>`
- `rows(shell) -> Vec<ActivityRow>`
- `offered(shell, &RowKey, label) -> Option<Offer>`
- `act(shell, &RowKey, label, cx)`
- `request(shell, DesktopRequestId) -> DesktopRequest`
- `pending_key(shell, root, action) -> RowKey`

`act` dispatches `ActivityMenuChoice` for that offer: the same action and
the same path a menu choice takes (Part D). Tasks 6 and 7 move the tests
of fact 38 onto the seam while the old columns are still drawn. They pass
before and after, and assert the same behaviour (E-5, E-6). Task 8 then
swaps the drawing without touching them. Keyboard and focus steps stay on
the drawn controls and are rewritten in Task 8 (E-7). P18 proves that
each action kind is reachable from a drawn button and from the menu.

**Window rules** (Cycles 01, 02 and 03a ledgers):

- Read focus in the same update that causes it.
- Window tests that turn polling on answer every poll explicitly.
- Guard tests use key presses or dispatched actions, with a dialog and
  with a sheet.
- Fixture data only: no network, no real waits, and the clock injected
  through `Services::with_clock` (fact 22).
- A new window test runs alone first. Window tests have flaked under
  load in all three earlier Cycles.

## Named Edits To Pre-Existing Tests

At `b37d2c2b`. Task 0 re-counts them with the sweep in the plan, over
tests and production code. Any other site is recorded as a further edit
(decision 14). Edits that remove assertions are marked **Removes**.

| E | Tests | Edit | Task |
| --- | --- | --- | --- |
| E-1 | `D/view/operations/give_up_tests.rs` `a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up` (`:344`); `D/view/operations/outcomes_route_tests.rs` `a_host_code_checks_the_host_with_its_authority` (`:91`) | The direct `share_result(id, record, gave_up)` calls note the attempt (`note_share_attempt`), then read the stored `share_result(id)` (F2) | 2 |
| E-2 | `D/view/operations/check_tests.rs` `a_running_check_now_reads_its_stage_words` (`:111-118`) | The poll policy verbs at the Writing stage read "Running: Saving…", not `FINISHING` (F3) | 2 |
| E-3 | `D/model/attention_tests.rs` `counts_what_awaits_the_person` (`:172`); `D/model/attention_retire_tests.rs` `retire_rows_dismissed` (`:143`); the helpers of `D/model/attention_polling_tests.rs`; `D/services/scheduler_retire_tests.rs` `retire_resume_resends_original` (`:691`) | `needs_attention` goes (A6). Each case of the first two becomes a row of `attention_counts_each_thing_once`, named in the ledger. The third test reads `attention_count()`. **Removes** the module | 5 |
| E-4 | `D/view/footer_tests/mod.rs` `needs_attention_counts_and_leads_to_activity` (`:279`); `D/view/keyboard_tests/shell.rs` `needs_attention_works_by_enter` (`:377`); `D/view/workspace_tests.rs` `drafts_not_saved_with_activity_hidden_focuses_the_item_area` (`:537`) | A count they assert changes only where N1–N16 count their fixture differently; each change is recorded in the ledger with its N row | 5 |
| E-5 | Under `D/view/operations/` and `D/view/operations_tests.rs`. **`operations_tests.rs`:** `launch_refreshes_each_repository_in_the_background_and_a_manual_refresh_runs_first`, `a_failed_refresh_and_rebuild_stay_until_dismissed_and_retry_succeeds` and its helper `recovery_of`, `a_lease_held_by_another_process_is_the_refreshs_typed_answer`, `a_pending_local_operation_offers_resume_and_abandon_confirms_it`, `resume_finishes_a_pending_local_operation`, `an_abandon_by_a_second_actor_is_answered_and_the_row_goes`. **`attempts_tests.rs`:** `dispositions_listed`, `attempt_entry_share_waits_for_the_lock`. **`attention_tests.rs`:** `attention_locked_key`, `attention_host`, `attention_actions_by_reason`, `attention_recovery_held`. **`give_up_tests.rs`:** helpers `code`, `accepted_share`, `give_up`; `give_up_is_offered_exactly_when_the_record_is_accepted`, `give_up_is_offered_on_exactly_the_rows_that_name_it`, `a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up`, `giving_up_a_finished_record_says_already_finished`, `stop_is_not_offered_without_a_remote`, `a_stopped_resume_over_an_unsettled_push_reads_not_confirmed`, `a_stopped_resume_that_then_fails_the_transport_reports_the_reservation`. **`outcomes_tests.rs`:** helpers `offered`, `outcome`, `assert_row`, and its 13 tests. **`outcomes_route_tests.rs`:** `no_selected_key_opens_sharing_and_keys`, `a_host_code_checks_the_host_with_its_authority`, `unlock_clears_the_declined_key_and_retries_the_same_request`, `share_after_a_finished_locked_key_clears_its_declined_key`, `busy_without_a_known_holder_points_to_pending_operations`, `a_final_failure_on_a_finished_record_offers_share_not_retry`. **`record_tests.rs`:** `an_unreadable_record_is_reported_once_in_activity`. **`republish_follow_tests.rs`:** all seven tests. **`republish_stop_tests.rs`:** `under_a_resume` and its callers, and `a_stop_that_cannot_be_queued_says_so`. **`republish_tests.rs`:** `a_deleted_branch_offers_republish`, `republish_confirms_then_shares`, `a_stopped_republish_sends_the_same_request`, `an_item_shares_stop_and_republish_opens_republish`, `republish_waits_for_another_dialog_to_close`, `an_unreadable_record_offers_give_up_and_it_sends_the_give_up`, `republish_opens_when_the_dialog_it_waits_for_is_cancelled`. **`resume_tests.rs`:** `close_row` and the six tests that press Resume, plus `a_lifecycle_row_offers_resume_after_restart`. **`retire_tests.rs`:** `retire_left_unkept_ask`, `retire_left_kept_draft_ask`, `retire_flush_failure_submits_nothing`, `retire_unkept_text_is_asked_before_drafts`, `retire_after_a_quit_closed_its_question`. **`check_tests.rs`:** the state read of E-2 | A press by element id becomes `act(shell, key, LABEL)`. A presence check becomes `offered`. A read of a state, code, message, outcome, command, effects, next action or scope becomes a model read through the seam: `request`, `share_result`, or the row's detail. A pending row by index becomes `pending_key`. Keyboard and focus steps stay as they are until E-7. Every assertion keeps its meaning; the ledger lists each file's test functions before and after | 6 |
| E-6 | Outside `D/view/operations/`, with the same edit as E-5. **`D/view/conflict/entry_tests.rs`:** `row_of`, `the_banner_offers_review_changes`, `a_pending_resolve_row_offers_review_changes_after_restart`, `a_local_synchronization_offers_resume`, `resolve_conflict_opens_the_childs_sheet`, `an_applied_resolution_clears_the_banner_and_its_review`, `a_resume_that_settles_the_conflict_clears_its_review`. **`D/view/conflict/repair_tests.rs`:** its three tests. **`D/view/confirm_tests.rs`:** the seven that press Abandon. **`D/view/find_tests.rs`:** `find_does_nothing_while_a_dialog_is_open`. **`D/view/tabs_tests.rs`:** `the_tab_keys_do_nothing_while_a_dialog_is_open`. **`D/view/passphrase_tests.rs`:** `a_prompt_above_the_confirmation_dialog_is_answered_by_its_own_dialog`. **`D/view/item/share_tests.rs`:** its nine tests of fact 38. **`D/view/item/edit_release_tests.rs`:** `retry_control` and its three tests. **`D/view/item/posting_tests/`:** `outside.rs` `recovery_labels` and its three tests; `decisions.rs` `a_post_stopped_on_a_conflict_raises_the_items_banner`; `extras.rs` `quitting_during_an_accepted_post_leaves_it_saved_and_cancelled`; `recovery.rs` `p3_a_stopped_post_offers_share_not_retry`; `review.rs` `an_operation_area_retry_is_followed_and_posts_one_comment`. **`D/view/lifecycle/`:** `outcome_tests.rs` `the_operation_entry_reopens_the_dialog`; `outcome_canned_tests.rs` `reopen` with `each_way_out_offers_its_button`, `busy_after_the_merge_is_failed_not_still_running` and `a_lifecycle_entry_keeps_the_recovery_its_dialog_does_not_own`; `interruption_tests.rs` `p5_a_close_stranded_at_closing_resumes_after_relaunch` and `a_close_lost_before_settlement_is_complete_and_stays_accepted`; `made_ready_tests.rs` `entry_texts` and `done_lists_made_ready_with_slug_and_title`; `privacy_consent_tests.rs` (two `a17_` tests); `privacy_promote_tests.rs` `a16_a_promotion_stopped_untidied_and_done_shows_no_item_text`. **`D/view/lifecycle_tests.rs`:** `a_blocked_prepare_lists_blockers_and_way_out`. **`D/view/dock_tests/state.rs`:** `a_request_finishes_while_activity_is_hidden`. **`D/view/drafts_entry_tests.rs`:** its two tests. **`D/view/workspace_tests.rs`:** `a_draft_that_cannot_be_saved_asks_before_quitting`. **`D/services/drafts_tests.rs`:** its two tests. **`D/services/drafts/save_tests.rs`:** `dismissing_a_pending_save_in_the_operation_area_lets_another_be_sent`. **`D/view/settings/keys_tests.rs`:** `the_operation_area_retry_of_a_protected_generation_asks_on_the_page`. **`D/view/settings/remotes_tests.rs`:** its three tests. **`D/view/settings/repositories_tests.rs`:** `a_partial_create_is_completed_by_retry_under_the_same_request`, including its literal `format!("request-{}-code")` (`:569`). **`D/view/settings/repositories_manage_tests.rs`:** `a_release_that_failed_during_removal_completes_once_retry_stores_the_text` | As E-5 | 7 |
| E-7 | Keyboard and focus steps, rewritten to Part H's keys. **`operations_tests.rs`:** `operations_keep_focus`, `dismiss_and_retry_from_the_keyboard_leave_focus_in_the_operation_area`, `a_recovery_dialog_gives_focus_back_to_the_action_that_opened_it`. **`attention_tests.rs`:** `attention_actions_hit`, `attention_actions_by_keyboard`. **`attempts_tests.rs`:** `attempt_entry_actions_hit`, `attempt_entry_ids_follow_the_repository`. **`retire_tests.rs`:** `retire_left_offer`, `retire_pending_row_by_keyboard`. **`retiring_tests.rs`:** `retire_rows_hit`. **`outcomes_route_tests.rs`:** `show_it_focuses_the_entry_of_the_possible_holder`. **`keyboard_tests.rs`:** `Walk::open_the_operation_area` and `Walk::start_a_confirmed_action_and_escape`, in `every_core_action_works_by_keyboard_alone`. **`keyboard_tests/actions.rs`:** `resume_by_keyboard`, `cancel_a_queued_request_by_keyboard`. **`keyboard_tests/confirm.rs`:** `open_abandon` and its four tests. **`keyboard_tests/conflict.rs`:** `resolve_a_local_conflict_by_keyboard`, with its own copy of `PENDING_REVIEW`. **`keyboard_tests/conflict/sheet.rs`:** `body_sheet` and its two tests. **`keyboard_tests/drafts/quit.rs`:** `a_draft_not_saved_on_quit_by_keyboard`. **`keyboard_tests/shell/polling_journey.rs`:** `polling_keyboard`. **`confirm_tests.rs`:** `escape_closes_the_dialog_with_no_request_and_focus_returns_to_abandon`, `:156`. **`passphrase_tests.rs`:** `:334`. **`dock_tests/state.rs`:** `show_it_reveals_a_hidden_activity` | A control is reached by focusing Activity, moving the highlight to its row, then Enter, or `shift-f10` and the menu. "Focus returns to the control" becomes "focus returns to the list, the row highlighted" (C3). `Workspace::abandon_focus` goes | 8 |
| E-8 | The bounds assertions of `retire_left_offer` (`:209-217`), `retire_pending_row_by_keyboard` (`:620-631`) and `retire_rows_hit` (`:129-138`), and the window resizes made to reach a control in `attention_actions_hit` and `attempt_entry_actions_hit` | Containment is P21's test, `rows_fit_a_narrow_activity`, for every row kind. **Removes** the per-test bounds checks | 8 |
| E-9 | `D/keys_tests.rs` `each_key_is_bound_to_its_action` | Gains each binding Parts C2 and E2 add; its fixed length follows | 8, 9 |
| E-10 | `D/view/dock_tests/toolbars.rs` `panel_toolbars_work_by_tab_and_enter` | Activity's filter and ⋯ join the loop; `ToolbarButton::ALL`'s length follows | 8 |
| E-11 | `D/view/drafts_privacy_tests.rs` (`:272-276`) | Expects S11's title, "The recovery copy of" | 8 |
| E-12 | `D/view/polling_privacy_tests.rs` `polling_privacy` | `SURFACES` and `read_at` follow the new ids, with the same pairs (Part G), so the sweep stays green at Task 8 | 8 |
| E-13 | `D/view/footer_tests/mod.rs`: `needs_attention_counts_and_leads_to_activity` becomes `needs_attention_counts_and_opens_the_popover`, and `status_items_say_and_choose`' status table (`focus_after_hiding(Attention, ..)`); `D/view/keyboard_tests/shell.rs` `needs_attention_works_by_enter` | A press and Enter open the popover, not Activity (E1); the hidden item's focus goes to the control before it (P26) | 9 |
| E-14 | D5's three sweeps (`D/view/lifecycle/privacy_tests.rs` `ALLOWED` and `Seen`, used by `privacy_promote_tests.rs` and `privacy_remote_tests.rs`); `polling_privacy` | The title-only allowance and the model rows' surface, popover and row menu (Part G) | 10 |
| E-15 | Appearance tests broken by the deliberate redesign, found in any task | Deleted, not updated (Wave Testing), and named in the ledger with what they asserted; the task's reviewer confirms each. A test that checks a control is visible in order to reach it is behaviour, and stops for the owner. **Removes** | — |
| E-16 | `D/assets_tests.rs` `every_item_view_icon_loads` and its `ICONS` array | Each icon Parts A and C use is checked first (`list-filter`, `ellipsis`, `refresh-cw`, `cloud-upload`, `circle-check`, `circle-alert`, `wifi-off`, `lock`, `info`, `loader-circle`); a missing one is added, and the array's length follows | 8 |

## Document Amendments

Applied by the planner in the approval commit, not by an implementation
task:

1. **Wave 04, Cycle 03b section.** "Amended 2026-11-12 by Cycle 03b":
   - the count includes every attention row once (decision 6);
   - checks that changed something get rows (decision 7);
   - titles appear in Activity and the popover (decision 12);
   - the K-19 rows are one-line rows.

   The Risks row "Needs attention misses the attention states D6 adds" is
   marked settled.
2. **`03-panels.md`, Activity:**
   - the filter shows the rows Needs attention counts;
   - ⋯ holds "Clear finished";
   - each row has a menu (right click, `shift-f10`, `menu`) whose first
     lines are the detail;
   - Enter presses the button;
   - the kinds "to share" (amber) and "working" (teal spinner);
   - the layout notices and the quit row sit above the list;
   - rows are one line.
3. **`05-sheets-and-dialogs.md`, Needs attention popover:**
   - the keys (Part H);
   - Tab stays inside, and Escape returns to the footer item;
   - it closes at zero, under a dialog, and when another menu opens;
   - a press outside also reaches what is under it;
   - "tries again" carries no fixed minutes.
4. **`02-layout-and-docking.md`, Footer item 5:**
   - its tooltip;
   - when it hides while focused, focus moves to the item before it.
5. **`06-language-and-states.md`:** a row for retirement ("Retired the
   copy of … on this computer", Activity), and a note that Activity's
   full copy is Cycle 03b's design Part B.
6. **D5's design, Privacy** (`docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md:694`):
   "Amended 2026-11-12 by Wave 04 Cycle 03b decision 12: a title may
   appear in an Activity row, its tooltip and menu, and the Needs
   attention popover; no body, comment, side or result text does."
7. **Cycle 01's design, K-19:** "Fixed by Wave 04 Cycle 03b's one-line
   rows."
8. **Cycle 03a's Cycle document and design, K-9:** "Decided by Cycle 03b
   decision 6."
9. **`08-gpui-kit-mapping.md`:**
   - Needs attention is a Workspace-owned layer, since the locked Kit
     `Popover` cannot be opened from the footer's control (fact 32);
   - Activity's list is GPUI's `uniform_list` (R-36).

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | Rows come from the models the operation area reads today; nothing new is stored on disk (decision 2) | Rows and times are lost at quit (K-1) |
| R-2 | The log lives in the Workspace and is refreshed at A4's refresh points, never in render | A change that reaches no refresh point leaves a row stale until the next one; P13 and P29 cover each source |
| R-3 | A hold's key is `hold_key(&Reason)`: the variant's name and its key ID, authority or root. `Reason` has no order (fact 19), and adding one would need the library's `SharedKeyId` to change | None while the string names the variant and its value |
| R-4 | A pending row's key is its operation ID, or "#{n}", its position in its repository's list, when it has none | A pending row with no ID can take a new time when an earlier one leaves (K-10) |
| R-5 | `RepoFacts` building moves, unchanged, into `each_repository` (A7) | None: one function, and Cycle 03a's tests pass unchanged (P12) |
| R-6 | A pending conflict makes no repository row (N10; decision 20) | After a relaunch with the pending list unread, nothing in Activity says why checking waits; the panel's row still reads Needs attention (K-9) |
| R-7 | A failed Check now counts as its repository's row or hold, not as a request (N4) | None: `check_failed` applies its answer to the schedule (fact 21) |
| R-8 | A share is classified once by the drain noting its record and Give up before `apply` (F2), not by changing `WorkerEvent::Finished` | Wrong if a later attempt's record could be read for an earlier `Finished`; a retry is queued only after the drain applied it (fact 13) |
| R-9 | The Busy holder's words and the republish state stay worked out at render | They change while the row is shown, which is what they describe (K-12) |
| R-10 | The popover is a Workspace-owned layer, not the Kit `Popover` (fact 32) | — |
| R-11 | A press outside the popover closes it and still reaches what is under it, except the footer item, which toggles (decision 18) | One press both closes and acts (K-11) |
| R-12 | Tab cycles inside the popover; Escape leaves it (decision 18) | Tab cannot reach the footer while it is open |
| R-13 | One layer at a time: opening a row menu, the dock menu, the repository menu or ⋯ closes the popover, and opening the popover closes them | — |
| R-14 | Checking…, To share, Paused, Up to date, a busy opening and Unknown make no repository row | Their facts are elsewhere: Checking… keeps a row through rule L3, and To share has its reminders |
| R-15 | This computer and the non-SSH remote make an information row, never counted, with Set up sharing… (decision 16) | A row per local-only repository stays all session; the filter hides it |
| R-16 | Both automatic checks and Check now add brought-in rows, with different sentences (A5) | — |
| R-17 | Problem rows come from the unshared list's problems, and reminders from its items, not from the last attempt's `listed`, so Activity agrees with the count and the repository row (fact 46; decision 17) | More reminder rows than today's Share… offers, which covered Unpublished items only |
| R-18 | The layout notices and the quit row stay above the list, with today's ids and texts; F3 applies to request rows, and the quit row's "Finishing {label}" names its job | — |
| R-19 | A final retirement line stays as a kept row when the next check clears it (L4) | — |
| R-20 | The list's container keeps the id `"operation-area"`, so D5's sweeps keep their surface (fact 40) | — |
| R-21 | Converted tests act through `ActivityMenuChoice` (the seam), not the drawn control | A converted test no longer proves its control is drawn; P18 proves each action kind from a drawn button and from the menu |
| R-22 | A request not sent is a failure row, but not counted, as today (N15) | A request waiting for the passphrase again is not in the popover; its row says so |
| R-23 | "Clear finished" takes only the rows L8 names | — |
| R-24 | `needs_attention` is deleted; the log's count replaces it (A6) | — |
| R-25 | A dialog or sheet opened from a row returns focus to the list with the row highlighted, replacing the focus maps of fact 8 | Focus returns to the row, not to the exact button |
| R-26 | Times show "HH:MM" local, or UTC with " UTC" in the detail when the offset is unknown | Two rows from different days can show the same time; the detail has the date (K-5) |
| R-27 | The row menu's first items are the detail lines, disabled (decision 8) | A long detail makes a long menu |
| R-28 | A repository row stays through a check of its repository (L3; decision 19) | A row may say Can't reach for a few seconds after the shared copy is back |
| R-29 | Up and down stop at the ends in Activity and the popover; Home and End are bound there | — |
| R-30 | An item with no title found reads "the ticket {short}" or "the document {short}" | K-2 |
| R-31 | A failure's description is the envelope's fixed message plus "Your work is safe on this computer." | Some messages name CLI commands (K-3) |
| R-32 | S9's action table is total at `b37d2c2b`; an action it lacks reads "An operation" | — |
| R-33 | Only the highlighted row's button is a tab stop in Activity | — |
| R-34 | F1's moves re-export every item, so no path changes | — |
| R-35 | Intents outside `Progressing` get stage words from `stage_words` (F3) | Some stages read generically ("Getting ready…") |
| R-36 | Activity's list is GPUI's `uniform_list` with its own keys, not the Kit `List`, whose click presses Confirm (C2) | U-2 |
| R-37 | The footer item, hiding while focused, passes focus to the control before it (E1) | — |
| R-38 | The popover anchors at the footer item's top right, `Anchor::BottomRight`, 440 px wide | — |
| R-39 | A retry clears a share's stored result and noted attempt (F2) | — |

## Known Limits

- **K-1.** Rows and their times last for the session. After a relaunch,
  pending operations, stored check outcomes, recovered drafts and the
  unshared list come back from their own stores, timed by the new
  session (decision 2).
- **K-2.** An item is named by its short code when no title is known
  (B1).
- **K-3.** A failure's reason is the library's fixed sentence for its
  code, and some name CLI commands (fact 16).
- **K-4.** Nothing is capped or pruned within a session; "Clear finished"
  is the relief (decision 13).
- **K-5.** Where the local offset is unknown (non-Unix today), times are
  UTC and the detail says so.
- **K-6.** No names: "Brought in N changes" names no one, and a conflict
  is "you and someone else" (fact 26).
- **K-7.** "Tries again" gives no minutes, since retries back off.
- **K-8.** Tooltips show on hover only. The detail is also each row's
  accessible description and its menu's first lines.
- **K-9.** A pending conflict with no pending row loaded has no Activity
  row (R-6).
- **K-10.** A pending row with no operation ID can change its time
  (R-4).
- **K-11.** A press outside the popover also acts (R-11).
- **K-12.** A Busy share's "May be waiting for …" can change while shown
  (R-9).

## Unverified

- **U-1.** `ActivityMenuChoice` derives the Kit `Action`. That needs
  `Clone`, `PartialEq` and `Debug` on `RowKey`, `ActivityAction` and
  their fields (`SyncOffer`, the polling `Action`, `RowAction`,
  `ItemRef`). Task 3 checks each, and adds a derive to a desktop type
  that lacks one.
- **U-2.** `uniform_list` (`gpui-pre-0.3.6/src/elements/uniform_list.rs:22`,
  re-exported by `gpui_kit::*`) draws and scrolls the rows with a tracked
  scroll handle, as C2 needs. Task 8 checks first with a scratch test. If
  it fails, the fallback is the Kit's `VirtualList`, with the same keys,
  recorded as an amendment.
- **U-3.** The popover's `on_mouse_down_out` fires before the footer
  item's own click. Task 9 checks the order, and makes a press on the
  item close the popover once (P25).
- **U-4.** Keys bound in `ACTIVITY_CONTEXT` reach the handlers while the
  list's focus handle holds focus. Task 8 checks first.
- **U-5.** `PollSchedule::checked_now`'s in-flight branch returns a
  summary to append from (A5). Task 4 checks.
- **U-6.** Cycle 03a's final review, merged before Task 0, leaves facts
  20–23, 28–30 and 46 true. Task 0 re-reads each and corrects this design
  first.
- **U-7.** An open tab is found from `(root, kind, id)` for B1's title
  (`TabContent::find`, `TabKey::Item`). Task 3 checks.

## Alternatives Rejected

- **A log stored on disk:** the owner chose this session only (decision
  2).
- **The Kit `Popover`** cannot be opened from the footer's wrapper or by
  the app (fact 32).
- **The Kit `List`** presses Confirm on a click (C2).
- **Adding the record to `WorkerEvent::Finished`,** as ticket
  `01M6H7VXJJHRYHTW9S8TJ88921` suggested, would touch 36 test sites
  (R-8).
- **Deriving rows in render:** times would be when a frame was drawn,
  and a hidden Activity would learn nothing (R-2).
- **Converting tests straight to the new drawing** would make one task
  carry the new panel and 494 test lines at once (Testing).
- **Leaving Can't reach out of the count:** the owner chose the design's
  popover (decision 6).

## Risks

| Risk | Control |
| --- | --- |
| The test migration loses or weakens an assertion | Tasks 6 and 7 are pure moves onto the seam, green before and after; the ledger lists each file's test functions before and after; the reviewer checks each converted assertion keeps its meaning |
| A model change reaches no refresh point, so a row is stale | P13 and P29 drive every source; each refresh point is named in A4 |
| The copy is wrong for a case | Pure copy tables (P4–P7); the owner checks rows on screen (Task 11's checklist) |
| Cycle 03a's final review changes what this design cites | Task 0 merges main after 03a closes and re-reads facts 20–23, 28–30 and 46 (U-6) |
| The Cycle does not fit one review | 12 tasks (0–11), as large as 03a's. The split line: 03b-1 is Tasks 0–8 (model, copy, the panel and its row menu, the footer count); 03b-2 is Tasks 9–11 (popover, privacy, gates) |
| Window tests flake under load | New window tests run alone first; the ledger names any flake (Cycles 01, 02, 03a) |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-12 | Plan 03b now, 03a being substantially complete | Owner direction | Decision 4; 03a's published head merged | Session, 2026-11-12 |
| Planning research | No row has a time; the count and the panel disagree (3a K-9); the Kit `Popover` and `List` do not fit; shares classified on render; "Finishing" hides the stage; titles barred by D5; 494 test lines reach the area by id | Facts; owner questions; rulings | Facts 1–46; decisions 5–13; R-8, R-10, R-21, R-36 | This document |
| Owner, 2026-11-12 | Resolved rows, the count, checks, extra actions, clearing, problem items, Enter, the outline, titles | Owner decisions | Decisions 4–13 | Session, 2026-11-12 |
