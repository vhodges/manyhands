---
title: "Wave 04 Cycle 03b Activity And Needs Attention Design"
date: 2026-11-12
status: "approved"
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
merging or closing. After approval, a note goes on each folded-in ticket
(`01M6H7VXJJHRYHTW9S8TJ88921`, `01M6RAS63EWVHWWVBSZ836RXG2`,
`01M5CS4MH25QQ769HV61ZRKHXG`) saying which items this Cycle takes. That is
the planner's step 9 work, not a Document Amendment.

**Approved 2026-11-13** by the owner (revision 5), with Cycle decision
26; the Document Amendments below were applied in the approval commit.
Revision 5 had no independent review.

Revision 2 applied review round 1 (two reviewers) and decisions 21–24.
Revision 3 applied review round 2 (a scoped re-review and an adversarial
review of `b830e8e3`). Revision 4 applied review round 3 (a scoped
re-review and an adversarial review of `4c596645`) and decision 25, and
redesigned retirements: no row is derived from the attempt's retirement
lines (R-51). Revision 5 applies review round 4 (a scoped re-review and
an adversarial review of `05246630`) and re-grounds on Cycle 03a's
merged code.

Round 4 was the fourth round to find Blocking issues in the attention
count. So that contract is A3's N-table with its precedence and latches (R-56), and A4's T-rows. Task 5
writes them test-first with a design checkpoint. A row its implementer
cannot express goes back to the orchestrator and the owner.

The log and the attention count are stated as tables of required
outcomes (A3, A4). Grounded at the ticket branch `b37d2c2b`, which holds:

- origin/main `37be101d` (Cycle 01, Cycle 02 and Wave 03 D6 merged);
- Cycle 03a's published head `3086666f`, merged in on 2026-11-12 by
  decision 4.

Cycle 03a is merged (main `ba95e1b7`); this branch merged main at
`81789331`. Revision 5 re-read Cycle 03a's final code there and corrected
what it moved (facts 19, 21, 28, 30, 41 and 46; U-6). Task 0 still merges
main and re-checks what changed after `81789331`.

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
  - "N-n" is a row of Part A's attention table (A3), and "T-n" a row of
    its log table (A4).
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
    that brought something in, a retirement whose request was dismissed
    on its answer (S8). A **state** row
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
  - "The title" is an item's title as read, from the places B1 names
    (fact 25), never a draft's own title (decision 21).
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
     `Verdict::dismissed`, `D/model/polling/retire.rs:67-76`).
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
    - `D/view/workspace.rs` is 488 lines; `D/model/repository_state.rs` 496.

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

    `git grep -c 'WorkerEvent::Finished('` over `D/` finds 51 lines in 26
    files: 36 lines in 21 test files, and 15 lines in 5 production files
    (`D/services/mod.rs` 7, `D/services/scheduler/reconfigure.rs` 5, and
    one each in `D/model/operations.rs`, `D/services/scheduler/retire.rs`
    and `D/services/worker.rs`). Some of these lines match the event
    rather than build it.
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
    - **Variants:** 43 (`D/services/requests.rs:69-351`).
    - **`label()`** (`:418-466`) is "Fixed text: no value the user
      typed".
    - **`root()`** (`:469-515`) is `None` for the key intents, which today
      show the scope "Application".
    - **Titles:** DocumentSave, TicketSave, DocumentCreate and
      TicketCreate carry the draft's title, which a row never shows (D2;
      decision 21).
    - **Item IDs:** the other item intents carry `id: ItemId`, apart from
      DocumentRepair and TicketRepair, which carry a path. No row shows a
      path (decision 21).
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
    - **A lifecycle's parent row.** `resume_offer` gives `NotOffered` for
      a promotion's or closure's parent (`D/model/operations.rs:1186-1188`).
      The pending row gets `lifecycle_resume` instead
      (`D/view/operations.rs:879`; `D/view/operations/resume.rs:67-108`),
      for a row `LifecycleRow::of` accepts (`resume.rs:47-64`):
      - Resume opens the lifecycle dialog (`resume_lifecycle`,
        `resume.rs:112-169`, which already takes an origin);
      - a disabled "Still running…" shows instead while `worker.in_flight`
        holds for that lifecycle (`lifecycle_of`, `D/services/mod.rs:1053`).
    - **`pending_shown`** (`D/model/operations/retire.rs:236`) leaves out
      the worker's running work: `is_running` (`:213-227`) reads the
      operation each Running request's first progress named and the
      operations of this app's automatic polls (`:81-84`).
    - **`operation_of`** (`D/model/operations/retire.rs:269`) names the
      operation an answered request is about, so a request and the pending
      row of one operation count once today (N1).
    - **A Busy read drops the rows.** `apply_pending`
      (`D/model/operations.rs:668-693`) replaces a Ready list with Busy
      (`ListState::from_read`, `D/model/list_state.rs:64-70`).
    - **`entered_operations()`** (`D/model/operations/retire.rs:195`) is
      read beside the pending rows (`D/view/operations.rs:740-746`).
    - **Lists are loaded** when:
      - repositories load;
      - a finished request changed a root;
      - the window activates;
      - a Busy or Failed list's Retry is pressed;
      - `show_holder` finds no entry.
19. **Checking holds.**
    - **The model:** `PollSchedule::attention()`
      (`D/model/polling.rs:452`) groups the repositories whose phase is
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
      &PollAttempt)` (`D/model/polling.rs:390`), called from
      `scheduler::apply` (`D/services/scheduler.rs:210`) in
      `Scheduler::answered` (`D/services/scheduler/react.rs:41-93`). That
      path also marks the repository changed when `summary.changed()`
      (`:74-80`).
    - **A Check now** goes through `PollSchedule::checked_now` (`:401`),
      called from `Scheduler::checked_now`
      (`D/services/scheduler/retire_loop.rs:149-168`). A failed one goes
      through `check_failed` (`D/services/scheduler/attention.rs:81`, `:120`).
    - **Cycle 03a's unshared list** (`RepoSchedule.unshared`, its `items`
      and `problems`) is set from completed checks and changed at once by
      this app's saves and shares (`D/model/polling.rs:246-344`).
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
      - `repository_tooltip` is at `:474-490`; it is Cycle 03a's panel
        tooltip, not Activity's detail (B4).
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
      - an open item's `ItemModel::last_title()` (`D/model/item.rs:259`)
        is an `Option`, `None` until a read gives a title. The tab's
        `tab_title()` (`D/view/item.rs:447-458`) falls back to the full ID,
        so it is not a source.
    - **Nothing else** maps an `ItemId` to a title synchronously. A save's
      or create's own title is the draft's text, which no row shows
      (fact 15).
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
        (`D/view/workspace/footer_status.rs:181-205`). It runs in render,
        from `footer` (`D/view/workspace/footer.rs:51`), and does nothing
        while a dialog or sheet is open (`footer_status.rs:187-189`).
    - **Cycle 01's `focus_lost`** (`D/view/workspace/focus.rs:275-295`)
      restores a lost focus to the nearest displayed region, else the
      last region, else the item area.
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
      (`show_after_frame`, `D/view/repositories.rs:216`), only when they
      differ from those shown, compared in place. `RepositoriesPane::rows`
      is test-only (`:208-212`).
    - **A right press** opens the menu (`row.rs:212-220`).
    - **The tooltip** is also the accessible description (`:232`).
31. **The Kit `List`** (`C/list/list.rs`):
    - **Keys** (`:26-35`): it binds `escape` (Cancel, which propagates),
      `enter` (Confirm) and `up` and `down` (which wrap) in context
      `"List"`. It has no `home`, `end`, `menu` or `shift-f10`.
    - **Row size:** it measures one row and gives every row that size.
    - **Each item's wrapper** carries `aria_selected`.
32. **The Kit `Popover`** (`C/popover.rs`; `B/popover.rs`):
    - **A controlled mode:** `.open(bool)` plus `on_open_change`
      (`C/popover.rs:190-211`, forwarded at `:315`).
    - **Its trigger** must be `Selectable`. Kit `Button` is
      (`C/button/button.rs:521`), and the footer's face is a Kit `Button`
      (`D/view/footer.rs:371`).
    - **Its own rules:** it keeps its state in element state, reports no
      open change for a controlled transition, and moves and restores
      focus its own way.
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
      - four resize the window to bring a control into view:
        `attention_actions_hit` (`D/view/operations/attention_tests.rs:371`),
        `attempt_entry_actions_hit` (`D/view/operations/attempts_tests.rs:181`),
        `a_draft_not_saved_on_quit_by_keyboard`
        (`D/view/keyboard_tests/drafts/quit.rs:70`) and `polling_keyboard`
        (`D/view/keyboard_tests/shell/polling_journey.rs:133`).
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
    - `counts_what_awaits_the_person` (`D/model/attention_tests.rs:172`)
      and `retire_rows_dismissed`
      (`D/model/attention_retire_tests.rs:143`);
    - `retire_resume_resends_original`, which calls `needs_attention`
      (`D/services/scheduler_retire_tests.rs:691`);
    - in `D/view/footer_tests/mod.rs`:
      `needs_attention_counts_and_leads_to_activity` (`:279`),
      `footer_controls_do_nothing_behind_a_dialog` (`:334`),
      `toggle_names_follow_their_state` (`:426`) and
      `status_items_say_and_choose` (`:488`), whose journey reads the
      control at `D/view/footer_tests/status.rs:356` and whose status
      table asserts `focus_after_hiding(Attention)` at
      `D/view/footer_tests/status_table.rs:211`;
    - `needs_attention_works_by_enter` and
      `footer_controls_are_reached_by_tab`
      (`D/view/keyboard_tests/shell.rs:377`, `:409`), and
      `arrange_the_workspace_by_keyboard_alone`, through its `dismiss`
      helper (`D/view/keyboard_tests/shell/journey.rs:104`);
    - `a_file_that_cannot_be_read_restores_the_default_and_says_so`
      (`D/view/dock/restore_tests.rs:209`, `:219`) and
      `a_failed_write_shows_the_save_notice` (`:366`);
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
    - **PRD:** `MH-UX-001` (`docs/PRD/mvp.md:549`), `MH-NFR-003` (`:601`),
      `MH-NFR-004` (`:611-612`, work that takes time does not block the
      desktop) and `MH-NFR-005` (`:622-623`). The PRD header reads version
      0.7.
45. **The privacy rules that bar titles today.**
    - **D5's design** (`docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md:694-698`):
      "No title, body, comment, side or result text enters an operation
      entry, log or error."
    - **D5's Cycle contract 10**
      (`docs/Cycles/wave-03-desktop-05-recovery-and-completion.md:197-198`):
      "No item title, body, comment or conflict text enters an operation
      entry, log or error message."
    - **D6's Cycle contract 10**
      (`docs/Cycles/wave-03-desktop-06-polling-and-shutdown.md:353-354`):
      "No item title, body, comment or key material enters an operation
      entry, status line, log or error message."
    - **D6's design** (`docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md`):
      - `:1095`: a retirement row's `{item}` "is never a title";
      - Privacy (`:1520-1528`): "None holds a title, body, comment or key
        material" (`:1525`), and a sentinel test fails "if a title or a
        passphrase appears in an operation entry, a Retire offer, ask or
        answer, status text or the in-flight record" (`:1527-1528`).
    - **D2's rule, as the test states it** (`D/view/drafts_entry_tests.rs:122-124`):
      "An operation entry names a draft's item by its title as read when
      the draft began, or by its ID when that read failed, never by the
      draft's own title."

    Decisions 12 and 21 amend the first four for titles (Document
    Amendment 6). D2's rule stands (B1).
46. **What Cycle 03a hands on.**
    - Read repository state from `repository_state`; decide the count
      (3a K-9); the footer order; the K-19 rows; Activity and "to share"
      agree (ticket comment `01M78JR8B5RY3XZ0TXTP6ZMTE9`).
    - **Its deferred Minor 2.** Cycle 03a's ledger recorded that Activity
      could list an item the count no longer had, when an older check is
      rejected after a share
      (`docs/plans/2026-11-12-wave-04-cycle-03a-repositories-execution.md:293-298`).
    - **How it ended.** It was fixed in `43f411a0`, item 2.
      `unshared_after` keeps the list for an older attempt, and still
      removes from `last.listed` the shares heard after it
      (`D/model/polling.rs:322-344`). The test is
      `to_share_counts_and_finished_shares`
      (`D/model/polling_schedule_tests.rs:978-1034`).
    - **What is left.** A gap is recorded as a 03a follow-up: a save
      heard between the two checks is not added to `listed` (03a ledger,
      `:1404-1407`).
    - **No design change follows.** This design reads the unshared list,
      not `listed` (R-17; Part A, S6).

**The press order.**

47. **A press outside a layer runs before a click.** `on_mouse_down_out`
    runs in the capture phase on mouse down
    (`gpui-pre-0.3.6/src/elements/div.rs:263-275`), and the footer item's
    click fires on mouse up. So a press on the footer item reaches the
    popover's press-outside handler first (E2).

## Part A: One Store Of Rows

### A1. Types

New module `D/model/activity.rs`, declared in `D/model/mod.rs`, with:

- `activity/rows.rs` for the types;
- `activity/requests.rs` for S1;
- `activity/states.rs` for S2–S14;
- `activity/log.rs` for A4, with `assign_attention` (A3);
- `activity/copy.rs` for Part B's texts, `repository_detail` (B4, R-48),
  `tooltip_lines` (B5), and the pure text helpers the rows need:
  `outcome_label`, `effects_text`, `scope_name`, the lifecycle outcome
  lines (`outcome_lines`, `D/view/lifecycle/outcome.rs:364`) and
  `SHARE_MAIN` (`D/view/sidebar.rs:12`).
  - **What moves with `outcome_lines`.** It needs `Reach::of(envelope,
    reached)` (`outcome.rs:253-302`). So these move too, all from
    `outcome.rs`:
    - `Reach` (`:252-341`), with the users of its private fields
      (`states` and `done`), `Ending` (`:197-240`) and `StepState`
      (`:98-120`), which its `states` reads;
    - the constants their `words` use: `ENDING_*` and `STATE_*`
      (`:48-55`), read by `StepState::words` (`:110-121`) and
      `Ending::words` (`:232-239`), and `STILL_RUNNING`
      (`D/view/lifecycle.rs:121`), which `Ending::words` reads as
      `super::STILL_RUNNING`. Each is re-exported from its view path;
    - `StepKey` (`:86-96`), `lifecycle_data` (`:242-250`) and
      `blocked_step` (`:343-360`);
    - the line words `merged`, `not_merged`, `SHARED`, `ALREADY_SHARED`,
      `SHARING_PENDING` and `TIDIED_UP` (`:61-72`), and the reasons
      `publication_reason` and `cleanup_reason` (`:415-455`), which
      `outcome_lines` calls.
  - **Why they move.** Task 3 moves these out of view modules
    (`D/view/operations.rs`, `D/view/lifecycle/outcome.rs`,
    `D/view/sidebar.rs`; the last two import no GPUI), so the model
    depends on no view module (layering). Each is re-exported from its
    old path, so view callers do not change (R-41).

It has no GPUI dependency.

```rust
/// What a row is about; its identity across refreshes.
pub(crate) enum RowKey {
    Request(DesktopRequestId),
    Brought { root: RepoKey, seq: u64 },
    Hold(String),                                   // hold_key(&Reason), R-3
    Repository(RepoKey),
    Problem { root: RepoKey, target: Target },
    Unshared { root: RepoKey, target: Target },
    RetireOffer { root: RepoKey, item: ItemId },
    Retired(u64),                                   // S8, a Retired event: RetireEvent.seq (A2, R-51)
    RetireNote { root: RepoKey, item: ItemId },     // S8, an information event: the latest per item (R-59)
    Pending { root: RepoKey, operation: String },    // R-4
    PendingList(RepoKey),
    Flush(DraftId),
    DraftStore,
    Recovered,
    Note(Report),
}

/// A target of the unshared list. An item whose ID did not parse also has
/// `item_id: None` (`D/model/polling/attempt.rs:134-140`), so it is
/// `Unnamed(n)`, its position among that list's targets with no ID.
pub(crate) enum Target { Main, Item(ItemId), Unnamed(u32) }

pub(crate) struct ItemRef { pub root: RepoKey, pub kind: AuthoringKind, pub id: ItemId }

pub(crate) enum Kind { Info, Success, Sharing, ToShare, Failure, Locked, Working }

pub(crate) struct Offer {
    pub action: ActivityAction,
    pub label: String,
    pub enabled: bool,
}

/// What the log's attention pass (A3) reads of a row, set by its source.
pub(crate) struct Group {
    pub candidate: bool,                   // the source's own "would count" (A2's Attention column)
    pub operation: Option<String>,         // S1: operation_of(request), else an OperationResume's or OperationAbandon's operation_id; S9: the row's operation ID
    pub roots: Vec<RepoKey>,               // S3: every root the hold covers; S4, S5, S9: its root; S1: intent's root
    pub check: bool,                       // S1: a PollOnce request
    pub check_applied: bool,               // S1: RequestFacts.check_applied (N4, R-58)
    pub superseded: bool,                  // S1: RequestFacts.superseded (N22 clause 4)
    pub subject: Option<Subject>,          // S1: its Intent subject (A3; R-52, R-57); S8 Retired: Intent { Retire, root, item, None }
    pub sent: Option<u64>,                 // S1: its attempt's queue order, Operations::sent (fact 11; N2, N22, R-56); S8: RetireEvent.sent
    pub outcome: Option<Outcome>,          // S1: its finished outcome (N22); S8 Retired: Success
    pub running_operation: Option<String>, // S1: the operation it runs, or its operation when an automatic poll runs it (N21)
    pub followed: bool,                    // S1: RequestFacts.followed (N23)
    pub follow_void: bool,                 // S1: RequestFacts.follow_void (N23 undone)
    pub resend_waits: bool,                // S1: RequestFacts.resend_waits (A1's `clearable`)
}

/// A request's subject (A3; R-52, R-57).
pub(crate) enum Subject {
    Operation(String),                     // Group.operation as a subject: N22 clause 1's extra match
    Intent { family: Family, root: Option<RepoKey>, item: Option<ItemId>, target: Option<String> },
}

/// An intent with its retries and resends, and a share with any later share of the same item.
pub(crate) enum Family { ItemShare, MainShare, Check, Retire, Save, Own(JobKind) }

/// A row as derived: everything but its time, unless the source has one.
pub(crate) struct Derived {
    pub key: RowKey,
    pub kind: Kind,
    pub title: String,
    pub description: Option<String>,
    pub repo: Option<String>,
    pub button: Option<Offer>,
    pub menu: Vec<Offer>,
    pub detail: Vec<String>,
    pub attention: bool,
    pub item: Option<ItemRef>,
    pub clearable: bool,
    pub at: Option<OffsetDateTime>, // S2: Brought.at (L1, T1)
    pub group: Group,
}

/// A row of the log: a `Derived`, the time the app first learned of it,
/// and its revision, from a log-wide counter (A4).
pub(crate) struct ActivityRow {
    pub at: OffsetDateTime,
    pub revision: u64,
    pub row: Derived,
}
```

No field is unset at any point. Sources leave `attention` false; the log
sets it (A3). A source that does not name a `Group` field leaves it
`None`, empty or false.

**Deriving.**

- `RowKey` and `Target` derive `Clone, PartialEq, Eq, PartialOrd, Ord,
  Debug`.
- `Report` (`D/services/inflight.rs:185`) gains `PartialOrd, Ord`.
- `Reason` cannot derive `Ord`: it holds the library's `SharedKeyId`,
  which has no order. So a hold's key is the string `hold_key(&Reason)`
  (R-3).

**Fields.**

- **`kind`** sets the icon's colour (`03`) and, with the source, the
  icon:
  - info: secondary `info`;
  - success: green `circle-check`;
  - sharing: teal; `cloud-upload` for a share that published (as the
    mockup's "Shared"), `refresh-cw` for a check;
  - to share: amber `cloud-upload` (`06`);
  - failure: red; `wifi-off` for Can't reach, `circle-alert` otherwise;
  - locked: amber `lock`;
  - working: a teal spinner, `loader-circle` (`06`).
- **`title`** and **`description`:** Activity draws the title, then ". "
  and the description when there is one, on one line ending in an
  ellipsis (decision 22). The popover draws them on two lines (`05`).
- **`repo`** is the tag: `{repo}`, or `{repo} +{n}` for a hold over more
  than one repository (decision 13).
- **`button`** is at most one offer (decisions 8 and 24), chosen as B2
  says. `menu` holds the button's offer first, then the others in
  today's control order (B2), then Dismiss when the row can be
  dismissed.
- **`detail`** holds the tooltip's lines after the full message and the
  time (B5); the view builds the tooltip at draw (B6). The full message,
  then the detail, are also the menu's first lines (Part D).
- **`item`** is an `ItemRef` when the row is about one item. Enter opens
  it when the row has no button (decision 11). A row with neither does
  nothing on Enter (decision 24).
- **`clearable`** means "Clear finished" may take the row (A4, rule L8;
  R-47). Only these sources may propose it:
  - S1 rows that ended Success (not a retirement answered Owed, A2),
    Noop, Cancelled or Removed;
  - S1 rows that ended Partial, Blocked or Error, once settled under N22
    or N23 (A3): they need nothing (decision 9);
  - S2 rows;
  - S8's events, Retired and information alike (R-59).

  It is set only when all three hold:
  1. the row is not an attention row;
  2. either it is settled (N22, N23), or its only enabled offer is
     Dismiss or Open…. Open… reopens a lifecycle dialog, read-only.
     Today's entry offers Open… on a lifecycle that finished other than
     Success or Noop (`D/view/operations.rs:444`, `:461-472`), so a
     cancelled close or promotion is clearable;
  3. no resend waits on it (`RequestFacts.resend_waits`, A2).

  The source checks 2's offers and 3. The log, at refresh after
  attention (A4), clears `clearable` on an attention row (1), and sets it
  on a failed S1 row in `settled` whose `Group.resend_waits` is false
  (2's first half, 3).
- **`at`** is set by S2 only, to `Brought.at` (A5). L1 uses it (T1).
- **`group`** is what the log's attention pass reads (A3). Each source
  sets it as the `Group` comments say.

```rust
pub(crate) enum ActivityAction {
    Cancel(DesktopRequestId),
    Stop(DesktopRequestId),
    Open(DesktopRequestId),                     // the lifecycle dialog
    Review(DesktopRequestId),                   // the conflict sheet
    Offer(DesktopRequestId, SyncOffer),
    Recovery(DesktopRequestId, usize),          // run_recovery(id, n)
    Dismiss(RowKey),                            // S1, S2 and S8 only
    DismissRecovered,                           // S13
    DismissNote(Report),                        // S14
    Attend(Action, Reason, RepoKey),            // a hold's action, on its first root
    Repository(RepoKey, RowAction),             // Cycle 03a's row action
    OpenItem(ItemRef),
    Share { root: RepoKey, item: Option<(AuthoringKind, ItemId)> },
    Retire { root: RepoKey, kind: TargetKind, item: ItemId },
    Resume { root: RepoKey, operation: String },
    ResumeLifecycle { root: RepoKey, kind: ItemDtoKind, item: String, operation: String }, // LifecycleRow's fields (S9)
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
| Dismiss | `dismiss_request` for a request; the log's `dismissed` for S2 and S8 (A4) |
| DismissRecovered | `DraftRegistry::dismiss_recovered` (`D/services/drafts.rs:319`) |
| DismissNote | `Operations::dismiss_record_note` (fact 27) |
| Attend | `attend(action, reason, root)`, with the hold's first root, as today (`D/view/operations/attention.rs:96`, `:120-127`) |
| Repository | `row_action` (private, `D/view/workspace/repository_menu.rs:229`) |
| OpenItem | opening a tab, as a list row does |
| Share | today's Share…, or Share main |
| Retire | `retire_chosen`, with `Subject { root, kind: TargetKind, item }` (`D/view/operations/retire.rs:69-73`, `:132`) |
| Resume | `resume` |
| ResumeLifecycle | `resume_lifecycle(root, row, origin)` (private, `D/view/operations/resume.rs:112-169`), with the `LifecycleRow` rebuilt from the action's fields by `LifecycleRow::new` (S9) |
| ReviewPending | `review_pending` (private, `D/view/operations/review.rs:131`; not `pending_review`, which draws) |
| Abandon | `abandon` |
| RetryPendingList | `load_pending` |
| RetryFlush | `retry_draft_flush` |
| ReviewDrafts | `open_drafts` |

**Signature changes** (Task 6). `open_lifecycle`, `review_request`,
`review_pending`, `run_recovery`, `run_share_offer`, `retire_chosen`,
`resume_lifecycle` and `row_action` each gain an `origin: FocusHandle`
parameter, or are made reachable (`pub(in crate::desktop::view)`), as
each needs.

- `resume_lifecycle` already takes an origin; it is made reachable.

- `run_share_offer`'s Republish… origin comes today from
  `control_focus_for` (`D/view/operations/outcomes.rs:317`). Task 9's
  origin change includes it (P20).
- `abandon` already takes an origin (`D/view/workspace/requests.rs:498-503`).

The Workspace runs an action through:

- `choose_activity(&ActivityMenuChoice, origin: FocusHandle, window,
  cx)`, which re-checks the row as a menu choice does (Part D): its
  revision (T11), then its offer (T12); then
- `run_activity_action(&ActivityAction, origin: FocusHandle, window,
  cx)`, which calls the handler above.

### A2. Sources

Every row comes from exactly one row of this table. The derivation is
`derive(&Inputs) -> Vec<Derived>`, a pure function over the inputs below
and the one entry point: `request_rows` (S1) plus `state_rows` (S2–S14).
Each row carries its `Group` (A1). Attention is not assigned here: the
log assigns it last, over its final rows (A3, A4). Abandon and the other
risky actions are never the button (decision 8; B2).

| S | Source (fact) | Key | Kind | Event or state | Button | Menu after the button | Attention (`Group.candidate`) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| S1 | Each request not Dismissed (6), a ContextRetire included (R-51) | `Request` | **Working:** Queued, Running. **Success:** Success, Noop; a share that published is **sharing**. **Failure:** Partial, Blocked, Error, NotSent; a share that needs unlocking is **locked**. **Info:** Cancelled, Removed | Event | **Queued, or Running and not accepted:** Cancel, as today (`D/view/operations.rs:412-425`). **Running and accepted:** Stop at a safe point when offered, else a disabled "Stopping…" or "Finishing" (`stop_offer` gives `NotOffered` when not accepted, `D/model/operations.rs:438-440`; fact 14; F3). **Finished, not Success:** B2's precedence. **Success:** none | Today's control order (B2): Open…, Review changes, the share offers, the recovery actions; then Dismiss once it has ended (any state but Queued or Running) | N1–N4, N18, N19, N21–N24 |
| S2 | Checks that brought something in (A5) | `Brought` | Sharing | Event | None | Dismiss | Never |
| S3 | Each checking hold (19) | `Hold` | Locked for `KeyLocked`, else failure | State | The hold's action; Unlock… disabled, with its reason in the detail, when `unlock_target` finds no target | — | Always (N18) |
| S4 | Each listed repository and the kept row (23; A7) | `Repository` | By state (Part B4) | State | The state's `RowAction` | — | N5–N11, N17, N19 |
| S5 | The unshared list's problems (21) | `Problem` | Failure | State | Open, when the target has an ID and a kind | — | Always (N9) |
| S6 | The unshared list's items, main included (21) | `Unshared` | To share | State | Share… for an item with a kind, disabled while the item's share runs (label "Sharing…") or while the item is locked (today's lock words in the detail), as today's attempt entry does; Cycle 03a's `SHARE_MAIN` for main | — | Never |
| S7 | Retire… offers, `RepositoryInput.retire_offers` (20) | `RetireOffer` | Info | State | Retire…, disabled while the item is retiring (`D/view/operations/retire.rs:132-140`) | — | Never |
| S8 | Retirement events, `Operations::retire_events()` (6; R-51) | `Retired` for Retired; `RetireNote` for the others (R-59) | **Success:** Retired. **Info:** Attention, Unreachable, Retry | Event | None | Dismiss | Never (N15) |
| S9 | Pending rows, `pending_shown()` (18) | `Pending` | Failure with a next action, else info | State | Resume when offered; for a lifecycle's parent, Resume (`ResumeLifecycle`), or a disabled "Still running…" while its lifecycle is queued or running (`lifecycle_running`); else Review changes when offered, else none | Resume, Review changes and Abandon… as offered; a Resume that is unavailable, disabled, with its reason | N1, N12, N21, N22, N25 |
| S10 | A pending list that is Failed or Busy (3) | `PendingList` | Failure (Failed), info (Busy) | State | Retry | — | Failed only (N13) |
| S11 | Each recovery copy not saved (27), its item named by B1's rule | `Flush` | Failure | State | Retry | — | Always |
| S12 | The draft store unavailable (27) | `DraftStore` | Failure | State | None | — | Always |
| S13 | Recovered drafts (27) | `Recovered` | Info | State | Review | Dismiss (`DismissRecovered`) | Never |
| S14 | Record notes (27) | `Note` | Info | State | None | Dismiss (`DismissNote`) | Never |

**A retirement is shown by its request** (R-51).

- **While it runs.** A ContextRetire request is its own S1 row, as every
  request is: Queued or Running, with Cancel or Stop at a safe point
  (B2). Nothing hides it.
- **An answer that dismisses its request.** `Operations::apply(Finished)`
  dismisses a retirement at once when `retirement.verdict.dismissed()`:
  Retired, Attention, Unreachable or Retry
  (`D/model/operations.rs:366-368`; `D/model/polling/retire.rs:67-76`).
  Just before that dismissal, `apply` appends a `RetireEvent` to a new
  session list, `Operations.retire_events`, read by `retire_events()`:

  ```rust
  pub(crate) struct RetireEvent {
      pub seq: u64,                // from Operations.next_retire_event
      pub sent: u64,               // the queue order of the request that produced it (N22 clause 5)
      pub root: RepoKey,
      pub kind: AuthoringKind,
      pub item: ItemId,
      pub verdict: Verdict,
      pub changed: bool,
  }
  ```

  - `seq` comes from a counter on `Operations`, `next_retire_event: u64`.
    `sent` is the request's `Operations::sent(id)`. `root`, `kind` and
    `item` come from the request's `Intent::ContextRetire { root, kind,
    id }` (`D/services/requests.rs:220-224`); `verdict` and `changed` from
    its `Retirement` (`D/model/polling/retire.rs:99-111`).
  - The list lasts for the session and is never pruned (K-4).
  - **Retired events** are keyed `Retired(seq)`, so a later retirement of
    the same item is a new key. Each is a success row, clearable, and
    carries `Group { subject: Intent { Retire, root, item, None }, sent,
    outcome: Success }`. It is never a candidate, but N22's clause 1 sees
    it as a later success (clause 5).
  - **Information events** (Attention, Unreachable, Retry) are keyed
    `RetireNote { root, item }` (R-59). A later one for the same item
    replaces the earlier, with `at` updated (L1; T18). They are
    information rows, clearable and never counted: the hold or the next
    attempt carries them, as today's dismissal says
    (`D/model/polling/retire.rs:67-70`, D6's R-23).
- **An answer that does not dismiss** (Owed, refused, preserved, failed)
  leaves the request's S1 row in place. A failed one counts under N1–N3
  and N22, as today.
  - **Owed with a Success envelope.** The verdict is Owed when the record
    is Accepted or Unknown (`D/model/polling/retire.rs:136-160`), whatever
    the envelope says. The S1 row then uses the "didn't finish" copy,
    "Retiring “{title}” didn't finish", not "Retired …" (B2). It is not
    clearable.
  - An Owed answer whose lists are behind adds "The lists are still
    catching up." to that row's detail, the second sentence of today's
    "didn't finish" line (`unfinished_text`,
    `D/model/polling/entry.rs:138-147`; B5).
- **Nothing derives rows from the attempt's retirement lines**
  (`RetireLine`, fact 20), which a later attempt clears
  (`D/model/polling/table.rs:147-151`; `D/model/polling.rs:408`). B0
  maps each line. `PollSchedule` does not change for S8.

**A lifecycle's pending row** (S9; fact 18). `LifecycleRow` and its `of`
(`D/view/operations/resume.rs:41-64`) move to the model with R-41's
helpers and are re-exported, so S9 can read them. Its fields are private
(`:41-45`), so the move adds a constructor, `LifecycleRow::new`.

- A row `LifecycleRow::of` accepts has the button Resume, as
  `ActivityAction::ResumeLifecycle`, which calls `resume_lifecycle(..,
  origin)` and opens the lifecycle dialog.
- While its close or promotion is queued or running elsewhere
  (`Inputs.lifecycle_running`), it has a disabled "Still running…"
  instead, and is not a candidate (N25).
- Otherwise it counts as N1.

**S4 makes a row by Cycle 03a's state, its `cause` and, for Locked, its
`RowAction`** (A7; R-40). `locked()` runs before `attention_cause`, so
`cause` is `None` on every Locked state (`D/model/repository_state.rs:207-215`,
`:277-289`):

- **Attention rows:**
  - Can't open;
  - Needs fixing;
  - Can't reach;
  - Locked with `RowAction::CheckNow` (a stored outcome);
  - Needs attention with cause `Held` (no reason in its phase),
    `WaitingFailed`, `Unreadable`, `Transport` or `Stored`.
- **Covered elsewhere, no repository row:**
  - Locked with `RowAction::Unlock(_)` (a live `KeyLocked`): the key's
    hold (S3; N5);
  - `Phase(_)`: the holds (S3; N6);
  - `Conflict`: the conflict's own row (N10);
  - `Problems`: the problem rows (S5; N9, N17).
- **Information rows, never counted:**
  - This computer: "`{repo}` has no shared copy, so changes stay on this
    computer", with Set up sharing… (`03`'s example);
  - the non-SSH remote of 3a decision 20, with Set up sharing….
- **No row** for Checking…, To share, Paused, Up to date, a busy opening
  and Unknown. Their facts are elsewhere (S6), or they need nothing.

**Inputs.** The Workspace builds them at each refresh (A4). The model
reads nothing else:

```rust
pub(crate) struct Inputs<'a> {
    pub operations: &'a Operations,
    pub schedule: &'a PollSchedule,
    pub repositories: &'a [RepositoryInput],
    pub drafts: DraftsInput,
    pub requests: &'a dyn Fn(DesktopRequestId) -> RequestFacts,
    pub conflict_waits: &'a dyn Fn(&RepoKey, &str) -> bool,
    pub title: &'a dyn Fn(&RepoKey, AuthoringKind, &ItemId) -> Option<String>,
    pub primary: &'a dyn Fn(&RepoKey) -> Option<String>,
    pub unlock_target: &'a dyn Fn(&KeyId) -> bool,
    pub lifecycle_running: &'a dyn Fn(&RepoKey, &ItemId) -> bool,
}

/// Precomputed by the Workspace from Services and the worker.
pub(crate) struct RequestFacts {
    pub stop_offer: StopOffer,                   // stop_offer(id, remote.is_some())
    pub shown_offers: Vec<SyncOffer>,
    pub share_words: Option<String>,             // the Busy holder, the republish state (R-9)
    pub already_finished: bool,
    pub review: bool,
    pub recoveries: Vec<(usize, String, bool)>,  // today's shown recovery actions: label, enabled
    pub resend_waits: bool,                      // a resend waits on it (R-47)
    pub followed: bool,                          // a resend follows from it (N23)
    pub follow_void: bool,                       // its follow-up ended Removed or NotSent (N23 undone)
    pub check_applied: bool,                     // a failed PollOnce whose answer check_failed applied: the scheduler's record (N4, R-58)
    pub superseded: bool,                        // a failed PollOnce whose repository applied a completed attempt since (N22 clause 4)
}

pub(crate) struct RepositoryInput {
    pub root: RepoKey,
    pub name: String,
    pub state: RepoState,                        // with `cause` (A7)
    pub remote: Option<String>,                  // the shared copy's name only (B4, R-48)
    pub checking: bool,
    pub primary: Option<String>,                 // the main branch, when known
    pub unshared: Unshared,                      // items and problems
    pub sharing: BTreeMap<Target, ShareState>,   // per unshared item (S6)
    pub retiring: BTreeSet<ItemId>,              // Retire… offers whose item is retiring (S7)
    pub retire_offers: Vec<RetireOffer>,         // Scheduler::retire_offers (S7)
}

pub(crate) enum ShareState { Ready, Sharing, Locked(String) }

pub(crate) struct DraftsInput {
    pub flush_failures: Vec<(DraftId, ItemRef, String, DraftErrorKind)>, // String: item_title()
    pub unavailable: Option<DraftStoreUnavailable>,
    pub recovered: Option<usize>,
}
```

- **`ShareState`** follows today's attempt entry (`D/view/operations/attempts.rs:138-217`).
- **`unlock_target`** is `Scheduler::unlock_target(key, operations).is_some()`
  (`D/services/scheduler/attention.rs:151-189`), which today's attention
  entry asks (`D/view/operations/attention.rs:81-92`). S3's Unlock… is
  enabled by it.
- **`resend_waits`** is true when dismissing the request would make
  `Resends::forget` (`D/services/resends.rs:173-178`) drop something: a
  chain link from or to it, or a republish waiting under its ID, which
  `ended` keeps for a stopped republish that goes on (`:224-235`). No
  query answers this today (`pending_for`, `:147-152`, and `waiting_for`,
  `:157-162`, ask by lifecycle run and by item), so Task 5 adds a
  read-only `Resends::waits_on(id) -> bool`, read through
  `Services::resends()` (`D/services/mod.rs:1032`).
- **`followed`** is true when `Resends::follow(id)`
  (`D/services/resends.rs:114`) names a follow-up that is not Dismissed
  (N23). **`follow_void`** is true when that follow-up ended Removed or
  NotSent: it did nothing, so N23 is undone (A3).
- **`check_applied`** is read from a record, not computed from the code
  (R-58). `check_failed` (`D/services/scheduler/attention.rs:81-123`)
  applies nothing for a code `check_now_failure` does not map
  (`D/services/poll_convert.rs:183-215`), and also nothing:
  - while a poll is in flight;
  - when the repository is held and the answer is only unreachable
    (`D/services/scheduler/attention.rs:103-116`).

  So the scheduler entity (`D/services/scheduler/entity.rs:77-139`) keeps
  `checks_applied: BTreeSet<DesktopRequestId>`, the failed Check nows
  whose answer `check_failed` applied, added to beside its
  `self.schedule.after` (`:120`), with a reader,
  `Scheduler::check_applied(id)` (N4).
- **`superseded`** is true for a failed PollOnce when its repository
  applied a completed attempt after the request finished (N22 clause 4).
  `PollSchedule` keeps a per-root count of completed attempts, incremented
  in `after` and `checked_now` on a `Completed` answer, read by
  `PollSchedule::completed_attempts(root)`. `check_failed` hears every
  failed PollOnce (`D/services/scheduler/reconfigure.rs:416-421`), and
  the scheduler notes the root's count then, in `checks_heard:
  BTreeMap<DesktopRequestId, u64>`. `Scheduler::check_superseded(id)` is
  true once the count has grown past it.
- **`lifecycle_running`** is today's `worker.in_flight` test with
  `lifecycle_of` for the item's lifecycle (fact 18), which
  `lifecycle_resume` asks (S9).
- **`running_operation`** (`Group`, N21) reads `is_running`'s two sources
  (`D/model/operations/retire.rs:81-84`, `:213-227`) through two new
  read-only getters: `Operations::running_operation(id)`, the operation a
  Running request's first progress named, and
  `Operations::poll_runs(operation)`, whether one of this app's automatic
  polls runs it. An automatic poll is not a request (fact 15), so S1
  carries its operation on the rows about it.
- **`lifecycle_lines`** are not a fact. `request_rows` computes them from
  `operations.lifecycle_answer(id)` (`D/model/operations.rs:508`) with
  `outcome_lines`, moved to the model (R-41), the one place they are
  built. It passes `Reach::of(envelope, None)`: with no stage seen, the
  lines appear only for a finished lifecycle answer.
- **`retire_offers`** are `Scheduler::retire_offers(root)`
  (`D/services/scheduler/retire_loop.rs:128-143`), which leaves out the
  items a running retirement loop will still take up, not
  `PollSchedule::retire_offers` (`D/model/polling.rs:656`).
- **`flush_failures`** carry the draft's `ItemRef`, from its target's
  root, kind and item ID, and `item_title()`: the item's title as read
  when the draft began, or the item's ID (`D/model/drafts.rs:131-137`,
  `:165-169`). When it equals the ID's text, S11 uses "the ticket
  {short}" or "the document {short}" (B1).
- **The pending rows' inputs:** `conflict_waits` reads the Workspace's
  `pending_conflicts` (`D/view/workspace.rs:408-417`;
  `D/view/operations/review.rs:109-131`), and `operations` gives
  `entered_operations()` (fact 18).
- **`RequestInputs` and `StateInputs`** are the request and state parts
  of `Inputs`, which `request_rows` and `state_rows` take (Tasks 3 and 4).

**Pinned above the list, never rows** (C1 decision 5): the layout notices
and the quit row.

### A3. Attention: Each Thing Counts Once

The footer's count is the number of attention rows not dismissed. The
popover lists exactly those rows (decision 6). This table is the required
outcome.

- **Where it runs.** `ActivityLog::assign_attention(&mut self)`, in
  `D/model/activity/log.rs`, is called only by `ActivityLog::refresh`,
  last, over the final rows, after the keeping rules L1–L3b (A4; T13).
  It reads each row's `Group` (A1). It is a method of the log, so it
  reads and writes `absorbed` and `settled` (A4), and `refresh` needs no
  `Inputs`.
- **Its test.** Task 5 writes `attention_counts_each_thing_once` first,
  row by row, over `derive` and `ActivityLog::refresh`, with a design
  checkpoint (Plan, Test-first), together with A4's T-table. A row its
  implementer cannot express goes back to the orchestrator and the owner.

Attention follows N1–N25 and the request's outcome, not B3's kind: a
Stopped or NoRemote share that failed by its envelope still counts under
N1–N3. NotSent stays uncounted (R-22). S8's events are never counted
(N15).

**A request's subject** (`Group.subject`; R-52, R-57). Every request has
the subject `Subject::Intent { family, root, item, target }`:

- **`family`** groups an intent with its retries and resends, and a
  share with any later share of the same item:
  - item share: ItemSync, ItemRepublish;
  - main share: RepoSync;
  - check: PollOnce;
  - retire: ContextRetire;
  - save: DocumentSave, TicketSave;
  - otherwise, the intent's own variant (`Intent::job_kind()`,
    `D/services/requests.rs:716`).
- **`target`** is the intent's own target (`D/services/requests.rs:69-351`):
  - the comment ID (CommentAdd);
  - the key ID (KeySelect, KeyRemove, KeyDelete; key intents have no
    root);
  - the host authority (HostApprove, HostReplace);
  - the item's path (DocumentRepair, TicketRepair, FolderCreate), used
    for identity only and never drawn;
  - the remote name (RemoteAdd, RemoteRemove, RemoteSelect);
  - the intent's own `operation_id` (OperationResume, OperationAbandon,
    ConflictResolve, LocalContextResolve);
  - none, for the rest (KeyGenerate, KeyImport and KeyClear carry no key
    ID).
- **The operation is an extra match.** A request's operation, when known
  (`Group.operation`), is also its subject as `Subject::Operation(op)`:
  N22's clause 1 holds when a later success matches either the failure's
  `Intent` subject or its operation. N2 groups by the operation when it
  is set, else by the `Intent` subject.

| N | Situation | Rows shown | Counted |
| --- | --- | --- | --- |
| N1 | A request finished Partial, Blocked or Error, and its operation has a pending row shown | The request row and the pending row | Once: as the pending row when that row is counted; when it is not (N12), as the newest request of the operation by queue order (N2) |
| N2 | Failed requests that no other rule settled or excluded (N21, N22, N23), grouped by `Group.operation` when it is set, else by subject | Each request row | Once per group, as its newest member by queue order (`Operations.sent`, fact 11), not by `at`; a group whose newest member is absorbed (N1 or N4) adds nothing more |
| N3 | A failed request that names no operation | Its row | Once |
| N4 | A failed Check now (PollOnce), for each failure class | Its request row, and its repository's row or hold when one covers its root | Absorbed, counted as the covering row, only when the covering row is a candidate hold or repository row covering its root (`Group.roots`) and `check_failed` applied the answer (`Group.check_applied`), or under N19; otherwise the request counts on its own (R-53) |
| N5 | A repository Locked by a live `KeyLocked` | The key's hold row, no repository row | Once per key, however many repositories it holds |
| N6 | A repository whose phase is `Attention(reason)` | The reason's hold row, no repository row | Once per hold |
| N7 | A repository held (`Shape::Held`) with no reason in its phase | A repository row with Resume checking | Once |
| N8 | A repository Can't open, Needs fixing, Can't reach, `Waiting { Failed }`, unreadable, a `Transport` answer, or a stored outcome | One repository row | Once |
| N9 | A repository whose check listed problems | One problem row per item, no repository row for the problems; a repository row for another state (a stored Locked, Can't open, Needs fixing) can stand beside them | Once per item, and that other row once |
| N10 | A repository whose check found a conflict pending | No repository row; the conflict's pending row or request offers Review changes | As N1 to N3 |
| N11 | A repository row whose repository starts Checking… | The row stays, its button disabled and reading "Checking…" (rule L3) | Still once, until the check answers |
| N12 | A pending row with no next action ("Nothing to do") | The row | No |
| N13 | A pending list Failed; Busy; Loading | A row; a row; nothing | Once; no; no |
| N14 | Recovery copies not saved; the draft store unavailable | One row each; one row | Once each; once |
| N15 | Requests that succeeded, were cancelled or not sent; brought-in rows; reminders; Retire… offers; S8's retirement events; recovered drafts; record notes; This computer; the non-SSH remote; layout notices; the quit row | As the sources say | Never |
| N16 | Any row dismissed | Not shown | No |
| N17 | A repository that cannot be reached, and whose kept problems make it Needs attention under Cycle 03a's order | Problem rows, no Can't reach row | Once per item (K-13) |
| N18 | A hold covers repositories A and B (tag "A +1"), and a Check now of B fails `unlock_required` while the phase stays `Attention(KeyLocked)` (`D/services/scheduler/attention.rs:89-121`) | The hold row and the request row | Once, as the hold |
| N19 | A Can't reach row R is kept by L3 because an automatic poll is in flight, and a Check now of the same repository fails (`check_failed` applies nothing while a poll is in flight, `:103-122`) | R and the request row | Once, as R |
| N20 | (withdrawn in revision 4) A retirement that is not dismissed is its request's S1 row, under N1–N3 and N22 (R-51) | — | — |
| N21 | A request runs on an operation (its progress named it), or one of this app's automatic polls runs it: `pending_shown` hides that operation's pending row (`is_running`, `D/model/operations/retire.rs:213-227`) | The running request's row; the operation's other rows | No row whose `Group.operation` is a running operation (`Group.running_operation`) counts while it runs: S1 and S9 |
| N22 | A failed request fixed another way (decision 25): any of N22's clauses below holds for its current attempt | Its row, as history, until dismissed or cleared | No: settled at the first refresh where a clause holds, and latched for that attempt (`settled`, R-56) |
| N23 | A request a resend follows from (`Resends::follow(id)`, `D/services/resends.rs:114`; `Group.followed`), for example a Resume answered `original_request_required` and resent | Both rows | No: settled at the first refresh where the follow-up exists, and latched as N22. Undone when the follow-up ends Removed or NotSent (`Group.follow_void`): the request counts again. The follow-up's own row decides |
| N24 | One failed request naming an operation that has no pending row shown, under neither N22 nor N23 | Its row | Once |
| N25 | A pending lifecycle row while its close or promotion is queued or running elsewhere (`lifecycle_running`) | The row, with a disabled "Still running…" | No |

**Precedence.** For a failed request, the first of these that applies
decides:

1. N23 (followed, latched): not counted;
2. N22 (settled, latched): not counted;
3. N21 (running): not counted;
4. N2: the rest are grouped by `Group.operation` when set, else by
   subject, and only each group's newest member by queue order goes on;
5. N4 (that member, a Check now, absorbed): counted as the covering row;
6. N1 (that member absorbed by a counted pending row): counted as that
   row;
7. N24 and N3: that member counts once.

An information row never covers or absorbs.

**N22's clauses.** A failed request settles (N22) when any of these
holds:

1. **A later success.** A later row, by queue order (`Group.sent`), with
   the same subject finished Success or Noop: its `Intent` subject, or
   its operation (`Subject::Operation`). The later row is a request, or
   an S8 Retired event (clause 5).
2. **Its operation finished.** A later OperationResume or
   OperationAbandon whose operation equals the failure's
   `Group.operation` finished Success or Noop. No pending row need have
   been seen.
3. **Its absorber left.** A row that absorbed it (N1, N4, N18, N19) at a
   refresh of its current attempt has left. For a hold or repository
   absorber, no candidate S3 or S4 row whose `Group.roots` holds its
   root now exists either. For an N1 absorber, the pending row's leaving
   is enough; there is no root condition.
4. **A later completed check.** A failed Check now (PollOnce) settles
   when its repository applied a completed attempt after the request
   finished (`Group.superseded`, A2). This is decision 25's "a later
   check succeeded".
5. **A retirement event.** S8 Retired events carry `Group { subject:
   Intent { Retire, root, item, None }, sent, outcome: Success }` and are
   never candidates. So clause 1 sees a retirement that the next attempt
   completed and that was dismissed automatically.

**The latch** (R-56). The log keeps two maps, per attempt:

- **`settled: BTreeMap<RowKey, u64>`:** a request key mapped to the
  attempt (`Group.sent`) at which it settled, at the first refresh where
  N22 or N23 holds for its current attempt.
  - Once settled, it is not counted for that attempt, whatever later
    happens to the rows that settled it: they may be cleared, dismissed
    or leave (T14).
  - The entry goes when the key's life ends (L2, T3) or when `Group.sent`
    changes: a retry or resend under the same ID is a new attempt (T15).
  - **One exception, N23 undone.** A follow-up that ended Removed or
    NotSent did nothing. So a request whose follow-up ended so is taken
    out of `settled` and counts again (T17). In this one case a latch is
    undone. A follow-up dismissed after it succeeded leaves the latch in
    place (T16).
- **`absorbed: BTreeMap<RowKey, BTreeSet<RowKey>>`:** for each failed
  request's key, the keys of the rows that absorbed it under N1, N4, N18
  or N19 at a refresh of its current attempt. It is cleared when the
  key's life ends or when `Group.sent` changes.

**N9's "no repository row"** means none for the problems. A repository row
for another state, such as a stored Locked, Can't open or Needs fixing,
can stand beside its problem rows, and both count.

**N4 absorbs only what the check reported** (R-53, R-58). `check_failed`
(`D/services/scheduler/attention.rs:81-123`; fact 21) applies a failed
Check now's answer to the schedule only for a code `check_now_failure`
turns into an attempt (`D/services/poll_convert.rs:183-215`). Even then
it applies nothing while a poll is in flight, or when the repository is
held and the answer is only unreachable (`:103-116`). So
`Group.check_applied` comes from the scheduler's record of what
`check_failed` applied (A2), never from the code.

- So a Check now is absorbed only when `Group.check_applied` is set and a
  candidate hold or repository row covers its root.
- Under N19, a row kept by L3 while a poll is in flight, it is absorbed.
- Otherwise it counts on its own: a `recovery_required` Check now beside
  a Can't reach row counts 2.

The test has an N4 row for each failure class.

**N5–N7 are 3a's attention causes.** N5 and N6 are covered by the hold
rows, which give the one action today's attention entry gives. N7's cause
is `Held` with no reason, which makes a repository row (A2).

**N10 relies on the conflict's own row.** That is a pending synchronize
row with Review changes, or the request that stopped on the conflict. A
repository row would offer nothing (fact 23, cause 3), so none is made
(R-6).

### A4. Times And The Log

`ActivityLog` (`D/model/activity/log.rs`), owned by the Workspace, holds:

- `seen: BTreeMap<RowKey, OffsetDateTime>`;
- `dismissed: BTreeSet<RowKey>`;
- `settled: BTreeMap<RowKey, u64>` and `absorbed: BTreeMap<RowKey,
  BTreeSet<RowKey>>`, per attempt (A3, the latch; R-56);
- `revisions: u64`, the log-wide revision counter;
- the current `rows: Vec<ActivityRow>`, from which L3 and L3b keep rows.

`refresh(&mut self, derived: Vec<Derived>, checking: &BTreeSet<RepoKey>,
pending_held: &BTreeSet<RepoKey>, now: OffsetDateTime) -> bool` applies
these rules, then assigns attention (A3) and settles `clearable` (A1)
last, over the final rows. It reports whether any row changed.

- **The two sets.** `checking` comes from `RepositoryInput.checking`, so
  it holds only listed repositories. `pending_held` holds the listed
  repositories that L3b's trigger names. Both are built at each refresh
  (A4's refresh points), so a repository that leaves the list leaves
  both.
- **L1, first seen.** A key seen for the first time gets `at = now`,
  unless `Derived.at` is set (S2's `Brought.at`, the time the check was
  applied, A5), which wins. A key seen again keeps its time, so a retry
  keeps its row's time and place (fact 6). One exception: a `RetireNote`
  row replaced by a later information event for its item takes `at =
  now`, and an earlier dismissal does not hide it (R-59; T18).
- **L2, a key leaves.** A key no longer derived, and not kept by L3 or
  L3b, is forgotten, time, `settled` and `absorbed` included. If it
  returns, it is new (T4). A request's key leaves when it is dismissed
  (L5, L6), so a dismissed request that runs again starts a new key life,
  at `now` and at the top (T3).
- **L3, checking keeps the row.** A `Repository(root)` row whose root is
  in `checking`, when no row is derived for that key, is kept as it was,
  with every offer disabled, the button and the menu alike, except
  Dismiss where it has one; the button reads "Checking…". It stays until
  the check answers. Its time and its count do not change while a retry
  runs (N11; fact 23). A row derived for that key wins (T6). So Retry now
  and Check now cannot send a PollOnce beside an automatic poll, which
  `check_now` does not test for (`D/services/scheduler.rs:245-260`; T5).
  A row kept by L3 leaves with its repository, which leaves `checking`.
- **L3b, an unread pending list keeps the pending rows** (R-54). A
  `Pending` row no longer derived is kept as it was, still counted, with
  every offer disabled, when its root:
  - has a derived `PendingList(root)` row that is not a candidate: a Busy
    read, which replaced the Ready rows (`apply_pending`, fact 18); or
  - has no pending list at all, because a Failed read with an
    unavailable code removed it (`D/model/operations.rs:680-684`).

  It is kept until a Ready read, or a Failed read that S10 counts, or
  until the repository leaves the list. It keeps its time (T7).
- **L5, requests.** A request row leaves only when its request is
  dismissed (fact 6). S1 then no longer derives it.
- **L6, dismissing.** Rows whose key is in `dismissed` are not shown, and
  dismissal forgets the key's time (T3).
  - **S2's and S8's Dismiss** adds the row's key to `dismissed` and drops
    its entry in `seen`.
  - **A request's Dismiss** goes through `dismiss_request`, which sets the
    request Dismissed and forgets its resends and draft (fact 6). It does
    not use `dismissed`; the key leaves by L2.
  - **S13's and S14's Dismiss** are `DismissRecovered` and
    `DismissNote(Report)`, through `dismiss_recovered` and
    `dismiss_record_note` (fact 27). Their sources then no longer derive
    the row, and it leaves by L2.
- **L7, order.** Rows are sorted newest `at` first. Equal times keep
  derivation order: sources in S order, and within a source, the source's
  own order (`requests()` newest first; repositories in list order).
- **L8, "Clear finished".** It dismisses every row that has `clearable`
  set (A1; R-47):
  - requests, through `dismiss_request`, a failure settled under N22 or
    N23 included;
  - brought-in rows (S2) and S8's events, through `dismissed`.

  Unsettled failures, attention rows, states, a row with an enabled offer
  other than Dismiss (Open… aside, A1) that is not settled, and a row a
  resend waits on stay (decision 9; T9).
- **L9, no cap** (decision 13).

Revision 3's fourth rule, for kept rows, is withdrawn (R-51). The other
rules keep their numbers.

**Revisions** (R-46).

- `ActivityRow.revision` comes from the log-wide counter `revisions`, not
  one per key. A row takes the counter's next value when its key is
  first seen.
- It takes a new one only when its `kind` or its offers (action, label,
  enabled) change, or, for a request, when its attempt (`Group.sent`) or
  its outcome (`Group.outcome`) changes. Stage words, other text and
  times never change it, so Stop and Cancel stay choosable while a
  request runs.
- A choice, from the menu, a click or Enter, carries the revision of the
  row it was made on: the drawn row, from the pane's applied rows, not
  the log's current one (Part D, C2, E2; R-55; T8).

**Required outcomes.** L1–L9 are the rules; this table is the contract.
Task 5 writes `the_log_keeps_times_and_events` from it first, with the
N-table, under the same design checkpoint (A3). T8 and T10 are window
tests: T10 is `refreshes_are_coalesced`, in Task 5's window test file,
since coalescing is the Workspace's; T8 is P15–P17's, in Task 9. The log's
test covers the other T-rows.

| T | Situation | Required outcome |
| --- | --- | --- |
| T1 | A key derived for the first time | `at` is the refresh's `now`, unless `Derived.at` is set (S2), which wins |
| T2 | A request retried (`Queued` again, not dismissed) | Same `at`, same place |
| T3 | A dismissed request that runs again (retry, Resume, or a resend by the drain) | A new key life: `at` is now, at the top (dismissal forgets its time) |
| T4 | A state key no longer derived; derived again later | Forgotten; a new `at` |
| T5 | A repository row whose root is checking, when no row is derived for that key; then the repository leaves the list | Kept as it was, every offer disabled, the button and the menu alike (its button reading "Checking…"), still counted (N11); it leaves with its repository |
| T6 | The same, but a row is derived for that key (for example Can't open after the CLI damaged the registration) | The derived row wins |
| T7 | A pending row whose repository's pending list goes Busy, or is removed by a Failed read with an unavailable code; then a Ready read, a Failed read S10 counts, and the repository leaving the list | Kept as it was, same `at`, still counted, every offer disabled; it leaves or is derived again by the read, and leaves with its repository (L3b) |
| T8 | A click or Enter on a row drawn before the log's row took a newer revision | Nothing runs, as T11 (a window test, P15–P17) |
| T9 | "Clear finished" with: a success; a stopped share whose record is Accepted (it offers Give up); a stopped republish that a lifecycle waits on (a resend link); a failure; a failure settled under N22 that still offers Retry; a state; an S8 Retired event; an S8 information event; a cancelled close (it offers Open…); a retirement answered Owed with a Success envelope | Only the success, the settled failure, the two S8 events and the cancelled close go |
| T10 | Several notifications in one effect cycle | One derive and one refresh (a window test, `refreshes_are_coalesced`) |
| T11 | A menu choice whose row's `revision` changed since the menu opened (its kind, offers, attempt or outcome changed); a Stop chosen on a running request whose stage words changed since | The first runs nothing; the Stop runs (stage words keep the revision) |
| T12 | A menu choice whose offer is gone or disabled | Nothing runs |
| T13 | Every N-row, after L1–L3b | The counts of A3 |
| T14 | "Clear finished", or Dismiss, removes the success that fixed a failure | The failure stays uncounted (R-56) |
| T15 | A retry of a settled request (`Queued` again under the same ID) | Counted again until it settles again |
| T16 | N23's follow-up dismissed after it succeeded | The request stays uncounted |
| T17 | N23's follow-up cancelled while queued (Removed) | The request counts |
| T18 | A later S8 information event for the same item, the earlier one shown or dismissed | One `RetireNote` row for the item, shown, with `at` now (R-59) |

**Refresh points.**

- `Workspace::refresh_activity(&mut self, cx)` marks the log dirty and
  defers one `Workspace::refresh_activity_now(&mut self, cx)` per effect
  cycle (`cx.defer`), so a burst of notifications derives once (R-43;
  T10). `refresh_activity_now` builds the inputs (A2), derives, and
  refreshes the log with the `checking` and `pending_held` sets and `now`
  from `scheduler.utc()` (fact 22). It
  notifies only when `refresh` reports a change. The test seam calls it
  directly (Testing). A test-only counter on the Workspace counts the
  derivations, which `refreshes_are_coalesced` reads over a burst of
  notifications (T10).
- **Cost** (`MH-NFR-004`, fact 44). The derivation is linear in the rows.
  Nothing derives per frame, and `uniform_list` draws only the visible
  rows (C2).
- **It is called from:**
  - the Workspace's operations observer (`D/view/workspace.rs:214`);
  - its scheduler observer (`:220`);
  - its drafts observer (`:217`);
  - after the repository list or a resolution is applied;
  - after pending conflicts land (`D/view/workspace/requests.rs:110-117`);
  - after an open item's title loads (the item pane's model change, or an
    observer of the panes);
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
- S2 sets `Derived.at` to `at`, which L1 uses (T1).
- An automatic one has the title "Checked `{repo}` for updates" and the
  description "Brought in 1 change." or "Brought in {n} changes." A Check
  now's has the title "Brought in 1 change to `{repo}`" or "Brought in {n}
  changes to `{repo}`", because its request row already says "Checked
  `{repo}` for updates" (Part B).

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
- Activity calls it to build each `RepositoryInput` (A2):
  `repository_state(facts)` with its `cause`, `checking()`, the primary
  branch, the shared copy's name (`facts.shared_copy`'s `name`;
  `SharedCopy { name, location }`, `D/services/scheduler/registration.rs:22-25`),
  the unshared list and its share states, and `Scheduler::retire_offers`.
- Activity does not read `repository_tooltip`, which carries the folder
  path and the shared copy's location (B4; R-48).

**The attention cause.** Cycle 03a's `RepoState` gains `pub cause:
Option<AttentionCause>`, set by `attention_cause` (fact 23):

```rust
pub(crate) enum AttentionCause {
    Phase(Reason), Held, Conflict, WaitingFailed, Unreadable, Transport, Stored, Problems,
}
```

- One variant per cause of fact 23, in its order.
- The change is in `D/model/repository_state.rs`, owned by Cycle 03a's
  code. It is safe: Cycle 03a's tests compare `RepoState` field by field
  (`D/model/repository_state_tests.rs:110-111`), so no edit follows. Task
  4 checks that, and records any edit it does need as E-17 onward.
- Activity filters repository rows by `kind`, `cause` and, for Locked,
  the state's `RowAction`, never by re-reading the schedule (R-40).
  `locked()` runs before `attention_cause`, so `cause` is `None` on every
  Locked state (A2, S4).

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

This part is the copy the owner reviews.

**Copy rules for the tables below.**

- Backticks mark placeholders and are not copy: `{repo}` is the
  repository's name.
- Parenthesized source citations are not copy. They sit in a sentence
  below each table, not in its cells.
- A title never ends with a full stop.
- Activity's line is the title, then ". " and the description when there
  is one. A description ends with a full stop.
- Counts have explicit singular and plural forms ("1 change", "{n}
  changes"), never "(s)".
- **Labels** (R-44). Today's offer and recovery labels are kept verbatim,
  even where they open a page without "…" ("Open Sharing & keys", "Set
  identity", "Set up sharing"). New labels follow the "…" rule.

### B0. Today's Area, Mapped

Every element of today's area (facts 2–4 and 9) becomes a source row, is
pinned, or goes. This is the ticket's planning item.

| Today's element | Becomes |
| --- | --- |
| The layout notices | Pinned above the list |
| The quit row | Pinned above the list |
| The session heading "This session" and "No requests yet" | Go; the empty states replace them (C1) |
| A checking hold (attention entry) | S3, `Hold` |
| An attempt entry's brought-in line | S2, `Brought` |
| An attempt entry's Unpublished lines | S6, `Unshared`: every unshared item now (decision 17) |
| An attempt entry's problem lines | S5, `Problem` |
| An attempt entry's retirement lines | No longer drawn (R-51). "Retiring “X”…": the ContextRetire request's S1 row. "Retired the copy …": an S8 Retired event. "Retiring X didn't finish." (Owed): the request's S1 row, which stays and reads "Retiring “{title}” didn't finish" even with a Success envelope (A2), with "The lists are still catching up." in its detail when today's line adds it. "Finished retiring X.": the OperationResume request's own S1 row, which says it finished |
| An attempt entry's Retire… offers | S7, `RetireOffer` |
| "Recovery copies are unavailable: …" | S12, `DraftStore` |
| "Recovered drafts: {n}", with Review and Dismiss | S13, `Recovered` |
| Record notes | S14, `Note` |
| Recovery copies not saved, with Retry | S11, `Flush` |
| A request entry | S1, `Request`; every control maps to an `ActivityAction` (A1) |
| The pending heading and "No pending operations" | Go |
| A pending repository whose list is not Ready | S10, `PendingList`; Loading makes no row |
| A pending row | S9, `Pending`, with Resume, Review changes and Abandon… |

### B1. Naming

- **An item:** “{title}”, in the typographic quotes the desktop already
  uses (`D/view/drafts/actions.rs:201`).
  - **The title comes from**, first found:
    1. an open item's `ItemModel::last_title()` (`D/model/item.rs:259`, an
       `Option`; not `tab_title()`, which returns the full ID when no
       title has loaded, `D/view/item.rs:447-458`);
    2. `share_title(root, id)` (fact 25).
  - **Without one:** "the ticket {short}" or "the document {short}", with
    the short code (fact 20; K-2).
  - **Never the intent's own title.** A save's or create's title is the
    draft's text (Wave 03 D2's rule; fact 15). So a new item reads by its
    short code until a list read gives its title.
  - A title is allowed in a row, its tooltip, its menu and the popover
    (decisions 12 and 21). A body, comment, conflict side or file path
    never is.
- **A repository:** `{repo}`, the folder name: the same as
  `RepositoryEntry::name()` and today's `scope_name`.
- **Main:** `{primary}`, the repository's main branch when known, else
  "main" (`06`).
- **A key:** "your key", never its label or path.
- **A host:** its host name. **A remote:** its name, never its URL.

### B2. Requests (S1)

The templates do not repeat the subject: Doing, Done and Verb already
contain it.

- **Running:** "{Doing}…". When a stage is known it adds " · {stage
  words}" (F3).
- **Queued:** "Waiting to {verb}".
- **Cancelled:** "Stopped {doing, first letter lower case}".
- **Removed:** "Cancelled: {verb}".
- **NotSent:**
  - "Didn't {verb}: your key's passphrase is needed again";
  - "Didn't {verb}: Manyhands could not find your SSH key store".
- **Success:** the Done column.
- **Noop:** the Done column, with the description "Nothing needed to
  change.", unless the table says otherwise.
- **Partial, Blocked and Error:**
  - title "Couldn't {verb}";
  - description "{envelope message} Your work is safe on this computer."
    The message is the library's fixed sentence for the code (fact 16;
    K-3).
- **ItemSync and RepoSync:** B3 decides the finished copy.
- **TicketClose and DocumentPromote:**
  - finished: the Done column, then the lifecycle answer's outcome lines
    joined with ". " ("Merged into {primary}", "Shared", "Tidied up"),
    as the dialog builds them (`D/view/lifecycle/outcome.rs:362-455`);
  - not succeeded: the button follows the precedence below, so a shown
    recovery action outranks Open….

**The button and the menu** (decision 8; R-44).

- **Risky actions are never the button:** Abandon…, Stop trying to share
  (Give up) and Stop and republish….
- **A finished request that did not succeed** takes, first found:
  1. Review changes, when it stopped on a conflict;
  2. the first share offer not in the risky set;
  3. the first shown recovery action not in the risky set; on a
     lifecycle entry, only one its dialog does not own (`dialog_owns`,
     `D/view/operations.rs:198-208`; fact 16);
  4. Open… (lifecycle).
- **A request not finished:**
  - Queued, or Running and not accepted: Cancel, as today
    (`D/view/operations.rs:412-425`);
  - Running and accepted: Stop at a safe point when offered, else the
    disabled "Stopping…" or "Finishing" (`stop_offer` gives `NotOffered`
    when not accepted, `D/model/operations.rs:438-440`).
- **The menu's order** after the button is today's control order: Open…,
  Review changes, the share offers, the recovery actions, then Dismiss.

| Intent | Subject | Doing | Done | Verb |
| --- | --- | --- | --- | --- |
| RepoCreate | `{repo}` | Creating `{repo}` | Created `{repo}` | create `{repo}` |
| RepoEnable | `{repo}` | Adding `{repo}` | Added `{repo}` to Manyhands | add `{repo}` |
| RepoRemove | `{repo}` | Removing `{repo}` from Manyhands | Removed `{repo}` from Manyhands | remove `{repo}` from Manyhands |
| RepoIdentitySet | `{repo}` | Setting your name for `{repo}` | Set your name for `{repo}` | set your name for `{repo}` |
| RemoteAdd | remote name, `{repo}` | Adding the shared copy {remote} to `{repo}` | Added the shared copy {remote} to `{repo}` | add the shared copy {remote} |
| RemoteRemove | remote name, `{repo}` | Removing the shared copy {remote} | Removed the shared copy {remote} from `{repo}` | remove the shared copy {remote} |
| RemoteSelect | remote name or none, `{repo}` | Choosing how `{repo}` shares | `{repo}` now shares through {remote}; with none, "`{repo}` no longer shares" | choose how `{repo}` shares |
| IndexRefresh | `{repo}` | Refreshing `{repo}` | Refreshed `{repo}` | refresh `{repo}` |
| IndexRebuild | `{repo}` | Rebuilding the search index for `{repo}` | Rebuilt the search index for `{repo}` | rebuild the search index for `{repo}` |
| PollOnce | `{repo}` | Checking `{repo}` for updates | Checked `{repo}` for updates | check `{repo}` for updates |
| PollConfigure | `{repo}` | Changing how often `{repo}` is checked | Changed how often `{repo}` is checked | change how often `{repo}` is checked |
| PollPause | `{repo}` | Pausing checks of `{repo}` | Paused checking `{repo}` for updates | pause checking `{repo}` |
| PollResume | `{repo}` | Resuming checks of `{repo}` | Resumed checking `{repo}` for updates | resume checking `{repo}` |
| RepoSync | `{primary}`, `{repo}` | Sharing {primary} | B3 | share {primary} |
| FolderCreate | `{repo}` | Creating a folder in `{repo}` | Created a folder in `{repo}` | create a folder in `{repo}` |
| KeyGenerate | your key | Creating a key | Created a key for sharing | create a key |
| KeyImport | your key | Adding a key | Added a key for sharing | add the key |
| KeySelect | your key | Choosing the key for sharing | Chose the key used for sharing | choose the key |
| KeyClear | your key | Clearing the key choice | No key is chosen for sharing now | clear the key choice |
| KeyRemove | your key | Removing a key from Manyhands | Removed a key from Manyhands | remove the key from Manyhands |
| KeyDelete | your key | Deleting a key | Deleted a key | delete the key |
| HostApprove | {host} | Trusting {host} | Trusted {host} for sharing | trust {host} |
| HostReplace | {host} | Updating the trusted key for {host} | Updated the trusted key for {host} | update the trusted key for {host} |
| OperationResume | `{repo}` | Finishing an operation in `{repo}` | Finished an operation in `{repo}` | finish the operation in `{repo}` |
| OperationAbandon | `{repo}` | Abandoning an operation in `{repo}` | Abandoned an operation in `{repo}` | abandon the operation in `{repo}` |
| ConflictResolve | `{repo}` | Saving your choices in `{repo}` | Resolved a conflict in `{repo}` | resolve the conflict in `{repo}` |
| LocalContextResolve | `{repo}` | Saving your choices in `{repo}` | Resolved a conflict on this computer in `{repo}` | resolve the conflict in `{repo}` |
| DocumentSave, TicketSave | item | Saving “{title}” | Saved “{title}” on this computer | save “{title}” |
| DocumentCreate, TicketCreate | item | Creating “{title}” | Created “{title}” on this computer | create “{title}” |
| DocumentMove | item | Moving “{title}” | Moved “{title}” | move “{title}” |
| DocumentPromote | item | Approving and merging “{title}” | Approved and merged “{title}” | approve and merge “{title}” |
| TicketClose | item | Closing “{title}” | Closed “{title}” | close “{title}” |
| ContextPrepare | item | Getting a private copy of “{title}” ready | A private copy of “{title}” is ready | get a private copy of “{title}” ready |
| ContextRelease | item | Putting away the private copy of “{title}” | Put away the private copy of “{title}” | put away the private copy of “{title}” |
| ContextRetire | item | Retiring the copy of “{title}” | Retired the copy of “{title}” on this computer | retire the copy of “{title}” |
| CommentAdd | item | Posting a comment on “{title}” | Posted a comment on “{title}” | post the comment on “{title}” |
| ItemSync | item | Sharing “{title}” | B3 | share “{title}” |
| ItemRepublish | item | Republishing “{title}” | Republished “{title}” | republish “{title}” |
| LocalContextSync | item | Repairing “{title}” on this computer | Repaired “{title}” on this computer | repair “{title}” |
| DocumentRepair | `{repo}` | Repairing a document in `{repo}` | Repaired a document in `{repo}` | repair a document in `{repo}` |
| TicketRepair | `{repo}` | Repairing a ticket in `{repo}` | Repaired a ticket in `{repo}` | repair a ticket in `{repo}` |

- **Descriptions:** RepoRemove's and KeyRemove's Done rows add the
  description "Its files are unchanged."
- **Sources, not copy:** DocumentSave's and TicketSave's Done and
  ContextPrepare's Doing follow `06`.
- **No path** appears in any row (decision 21): FolderCreate,
  DocumentRepair and TicketRepair name the repository.
- **ContextRetire answered Owed** with a Success envelope reads
  "Retiring “{title}” didn't finish", not the Done column, and is not
  clearable (A2).
- **Subject** says what each row names; "item" is “{title}” as B1 says.

### B3. A Finished Share

The classification is made once (F2). Words that exist today are kept
(fact 17). The subject is “{title}” for an item, or {primary} for main.
Attention follows A3, not this table's kind (decision 24).

| `SyncOutcome` | Kind | Title | Description |
| --- | --- | --- | --- |
| Published | sharing | Shared {subject} | — |
| Current | success | {subject} is the same as the shared copy | — |
| Stopped { completed: false } | info | Stopped sharing {subject} | Stopped before anything was sent. Your saved work is still on this computer. |
| Stopped { completed: true } | info | Stopped sharing {subject} | Stopped after part of the work finished. Your saved work is on this computer; Share sends the rest. |
| Conflict | failure | You and someone else both changed {subject} | Choose what to keep. Nothing is shared until you do. |
| BranchDeleted | failure | The shared copy of {subject} was deleted | Republish it to share it again. |
| HistoryUnresolved | failure | Couldn't confirm the shared history of {subject} | Today's sentence |
| NeedsUnlock | locked | Couldn't share {subject}: your key is locked | Unlock it, then share again. |
| NeedsKey | failure | Couldn't share {subject}: choose a key to share with | — |
| NeedsHost | failure | Couldn't share {subject}: check {host} before sharing | — |
| NeedsIdentity | failure | Couldn't share {subject}: set your name first | — |
| Busy | failure | Couldn't share {subject} yet | "May be waiting for {label}.", or "Another share may be holding this repository." |
| NoRemote | info | `{repo}` has nowhere to share {subject} to | — |
| NothingToShare | info | Nothing to share in {subject} yet | It has no saved changes. |
| Failed | failure | Couldn't share {subject} | The envelope message, as B2 |

- **Sources, not copy:** Conflict's description is `05`'s (fact 26).
  Busy's words are worked out at each refresh into `RequestFacts` (R-9).
  While a republish runs, BranchDeleted's Republish… reads
  "Republishing…".
- **The button** follows B2's precedence: Review changes when the share
  stopped on a conflict, else the first of `shown_offers` (fact 12), in
  today's order, that is not in the risky set.
- **The menu** holds the rest, in today's order. Today's labels are kept
  verbatim (R-44): "Share", "Retry", "Stop trying to share", "Unlock…",
  "Open Sharing & keys", "Check host", "Set identity", "Show it", "Set up
  sharing", "Republish…" and "Stop and republish…".
- **"Already finished"** (a Give up's replayed answer) goes in the detail.

### B4. States (S2–S14)

| S | Title | Description | Button |
| --- | --- | --- | --- |
| S2 automatic | Checked `{repo}` for updates | Brought in 1 change. / Brought in {n} changes. | — |
| S2 Check now | Brought in 1 change to `{repo}` / Brought in {n} changes to `{repo}` | — | — |
| S3 KeyLocked | Your key is locked | Checking `{repos}` is waiting for the passphrase. | Unlock… |
| S3 KeyProblem, NoKeySelected | Checking needs a key to be chosen | Holding back `{repos}`. | Choose a key… |
| S3 HostUnapproved | {host} needs your approval | Checking `{repos}` waits until you approve it. | Approve host… |
| S3 HostUnidentified | A server needs your approval | Checking `{repo}` waits until you approve it. | Approve host… |
| S3 ConfigurationRequired | Sharing isn't set up for `{repo}` | Checking can't start until it is. | Open remote settings… |
| S3 RecoveryHeld | Checking `{repo}` is held until recovery | Your work is safe. | Resume checking |
| S4 Can't open | Can't open `{repo}` | 3a's sentence | Retry |
| S4 Needs fixing | `{repo}` needs fixing | 3a's sentence | Settings… |
| S4 Can't reach | Couldn't reach `{repo}`'s shared copy | Your work is safe, and Manyhands tries again. | Retry now |
| S4 Locked, stored | Checking `{repo}` needs your key's passphrase | Check now asks for it. | Check now |
| S4 Needs attention | Checking `{repo}` needs attention | 3a's sentence for the cause | 3a's action for the cause |
| S4 This computer | `{repo}` has no shared copy, so changes stay on this computer | — | Set up sharing… |
| S4 non-SSH | 3a's sentence | — | Set up sharing… |
| S5 | “{title}” needs a look, or "{primary} in `{repo}` needs a look" | The disposition's sentence below | Open |
| S6 | “{title}” isn't shared yet, or "{primary} in `{repo}` isn't shared yet" | — | Share… (disabled as A2 says), or "Share main" for main |
| S7 | “{title}” is closed, but its copy is still on this computer; a promoted document reads "is approved, but …" | — | Retire… |
| S8 Retired | Retired the copy of “{title}” on this computer | — | — |
| S8 Attention, Unreachable, Retry | Retiring “{title}” didn't finish | It is tried again at the next check. | — |
| S9 | "{action words} didn't finish in `{repo}`", or "{action words} “{title}” didn't finish" when the item is known | the next action's sentence below | Resume, or Review changes; a lifecycle's parent: Resume, or "Still running…" disabled |
| S10 Failed | Couldn't read the unfinished operations of `{repo}` | — | Retry |
| S10 Busy | Manyhands was busy reading the unfinished operations of `{repo}` | — | Retry |
| S11 | The recovery copy of “{title}” wasn't saved, or "The recovery copy of the ticket {short} wasn't saved" (or "the document {short}") when `item_title()` is the ID | {reason} | Retry |
| S12 | Recovery copies are unavailable | {reason} | — |
| S13 | 1 draft recovered from an earlier session / {n} drafts recovered from an earlier session | — | Review |
| S14 | Today's record-note sentences | — | — |

- **Sources, not copy:** S3 KeyLocked's and S4 Can't reach's titles are
  `05`'s; S4 This computer's is `03`'s example; S4 non-SSH's sentence is
  3a decision 20's; "Share main" is Cycle 03a's `SHARE_MAIN`, not "Share
  {primary}", which moves from `D/view/sidebar.rs:12` to the model's copy
  module with R-41's helpers and is re-exported by `view/sidebar.rs`;
  S11's and S12's `{reason}` is today's `DraftError` words; S8 Retired's
  title is today's Retired line (`retired_text`, `D/model/polling/entry.rs:133-136`)
  with “{title}”, and "Still running…" is today's `STILL_RUNNING`
  (`D/view/lifecycle.rs:121`).
- **S11's item** is named by B1's rule from `item_title()`, the title as
  read when the draft began (A2, `flush_failures`), never the draft's own
  title.
- **Labels:** S4 This computer's and non-SSH's button is "Set up
  sharing…", Cycle 03a's label for the same action. S13's Review stays
  "Review", today's (R-44).
- **S6's lock state:** while the item is locked, Share… is disabled and
  today's lock words are in the detail, as today's attempt entry does
  (`attempt_entry_share_waits_for_the_lock`). While its share runs, the
  disabled button reads "Sharing…".
- **S7's Retire…** is disabled while the item is retiring.

**The repository rows.**

- **Description:** an explicit description cell wins. "3a's sentence" is
  used only where the cell says so: the state's own `sentence` lines from
  `repository_state` (fact 23), joined, when they say more than the
  title.
- **Detail:** `repository_detail(primary, remote) -> Vec<String>`, a new
  pure function in `D/model/activity/copy.rs` (R-48). It starts at "Main
  branch", without repeating the state's sentence, which is already the
  description:
  - "Main branch: {primary}" when known;
  - "Shared copy: {remote}", the remote's name only.

  It is not `repository_tooltip`, which carries "Folder: {root}", the
  full path, and "Shared copy: {name} · {location}", the address
  (`D/model/repository_state.rs:474-490`;
  `D/model/repository_state/words.rs:91-103`). Cycle 03a's panel tooltip
  is unchanged, and so is its decision.
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

The tooltip's lines, in order (decision 22):

1. The full message: the title, then ". " and the description when there
   is one.
2. The time in full: "{YYYY-MM-DD HH:MM}", local, or UTC followed by
   " UTC" when the offset is unknown (B6). The model's `detail` holds no
   time.
3. The model's `detail`, by source:
   - **A request:**
     - "{intent label} · {scope}" (today's fixed label and scope);
     - the outcome label ("Done", "Failed" …);
     - "{code}: {message}" when it did not succeed;
     - "Effects: {effects_text}" when any effect was requested;
     - "Operation {id}" when known;
     - the stage while running;
     - "Already finished" for a Give up's replayed answer;
     - "The lists are still catching up." for a retirement answered Owed
       whose lists are behind (A2, R-51).
   - **A hold:** today's headline and "Holding back {names}".
   - **A repository:** `repository_detail` (B4; R-48), never
     `repository_tooltip`.
   - **A problem:** "{disposition}: {library guidance}" (fact 20).
   - **A reminder:** "Not shared yet: {disposition}", and the lock's words
     while the item is locked (S6).
   - **A pending row:** its raw action, state, completed step, operation
     ID, next action and `updated_at`.
   - **A draft or note row:** its kind.

**The builder.** `tooltip_lines(row: &Derived, time: &str) -> Vec<String>`,
in `D/model/activity/copy.rs`, gives these lines; the view calls it at
draw with B6's time line. The same text is the row's accessible
description. The row menu's first lines are the full message, then
`detail`, each as a disabled item (Part D).

**No detail line carries a path or an address.** Today's request scope
is the repository's folder name, which stays (decision 21).

**What a row never holds** (Part G): a body, a comment, a conflict side,
a remote URL, a file path, a key's label or path, a passphrase, or a
server's own text.

### B6. Time

The row shows "HH:MM".

- `activity_time(at: OffsetDateTime, offset: Option<UtcOffset>) ->
  (String, String)`, in the model, gives the row's time and the
  tooltip's time line (B5).
- The view calls it at draw with `activity_time(row.at,
  local_offset(row.at.unix_timestamp()))`. `local_offset` is view-scoped
  (`D/view/confirm.rs:583`, fact 37), so the model never reads it.
- With no offset (non-Unix today, fact 37), both are UTC and the tooltip
  says " UTC" (K-5).

## Part C: The Activity Panel

New module `D/view/activity.rs`, with `activity/row.rs`, `activity/menu.rs`
and `activity/actions.rs`, declared in `D/view/mod.rs`. Task 5 creates and
declares `D/view/activity.rs`, for the test file of
`count_and_popover_agree` and `refreshes_are_coalesced`;
Task 6 adds `mod actions;` to it; Task 9 fills it. It replaces the session and pending columns (fact 2,
fact 3).

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
  no row is `clearable` (rule L8).
- **The filter** is `Workspace.activity_filter: bool`. It is off at
  launch and lasts for the session. When on, the list shows only
  attention rows.

**One row**, one line, 28 px high (Wave decision 7), which meets `03`'s
minimum of 24, in this order:

- the time (B6), 40 px, monospace, muted;
- the kind's icon;
- the message, the title, then ". " and the description, ending in an
  ellipsis, taking the free width (`flex_1`, `min_w_0`);
- the tag, which shrinks with an ellipsis after the message, at a fixed
  maximum width;
- the button.

The button never leaves the row: the message shrinks first (decision 22).
That fixes K-19's rows at any width (fact 7).

- **Row tooltip:** the full message (title, then description), then the
  time line (B6), then `detail`, joined with newlines, from the pure
  `tooltip_lines` (B5; decision 22). The same text is the row's
  accessible description, and its accessible name is the message (as
  Cycle 03a's rows, fact 30). The full message, then `detail`, are also
  the row menu's first lines (Part D).
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
| `enter` | Runs the highlighted row's button when enabled; with no button, opens the row's item; with a disabled button, or with neither, nothing (decisions 11 and 24) |
| `shift-f10`, `menu` | Opens the highlighted row's menu (Part D) |
| A left click on a row | Highlights it and focuses the list |
| A left click on its button | Runs the button |
| A right click | Highlights the row and opens its menu |

- **Applying rows.** The pane keeps the highlight on the same `RowKey`
  across refreshes. When that row leaves, the highlight goes to the row
  now at its position, else the last row; with no rows, the empty state.
  Rows are applied to the pane after the frame (`window.defer`, as fact
  30). Nothing notifies during render.
- **Focus when the highlighted row changes.** If the highlighted row's
  button held focus and the row left, or the row now at that position has
  no button, focus moves to the list, so Escape and the arrows keep
  working (as the popover, E2; P15).
- **A click or Enter** builds its choice from the drawn row for the
  highlighted key, from the pane's applied rows, not the log's current
  ones, with that row's key and revision. If the log's row has a newer
  revision, nothing runs (T8, T11; R-55), as in the popover (E2).
- **Drawing.** `uniform_list` draws only the visible rows. Moving the
  highlight calls `scroll_to_item`. Tests find rows through the seam
  (Testing), not by drawn index.
- **Enter on a focused row button** runs its action once (P15). The
  `enter` binding in `ACTIVITY_CONTEXT` must not also fire: it is bound on
  the list's own element only, or the button handles its own Enter.
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
  Republish…, a recovery that confirms, Open…, Review changes, a
  lifecycle's Resume through `resume_lifecycle`) passes the
  list's focus handle as the origin or restore, whether it came from the
  button, the menu or Enter. When the dialog closes, focus returns to the
  list, still highlighting the row's key (decision 23). The row counts as
  the invoking control (RFC 344–351). This replaces the focus maps of
  fact 8, which go with the old columns.
- **"Show it"** reveals Activity, turns the filter off when the holder's
  row is hidden by it, and highlights and focuses the holder's row. A
  ContextRetire holder resolves to its S1 row, which always exists while
  it runs (R-51). `show_holder`'s fallback to the pending rows highlights
  the first pending row of the repository.
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
  1. the full message (title, then description), then the row's `detail`
     lines, each as a disabled item (decisions 8 and 22; R-27);
  2. a separator;
  3. the row's offers, the button's first, then today's control order
     (B2), each enabled or disabled as the row says;
  4. Dismiss, when the row can be dismissed.
- **The choice** is a Kit `Action`, `ActivityMenuChoice { key: RowKey,
  action: ActivityAction, shown: String, revision: u64 }`, in `D/keys.rs`
  beside `RepositoryMenuChoice`. `revision` is the drawn row's revision
  when the menu opened (A4; R-46, R-55). It changes only as A4's
  Revisions say, so a running request's Stop and Cancel stay choosable
  while its stage words change (T11).
  - The menu's action handler (Task 9) calls
    `Workspace::choose_activity(&ActivityMenuChoice, origin, window, cx)`
    with the list's handle as origin (A1). The test seam calls the same
    function directly (Testing).
  - `choose_activity` finds the row by `key` in the log, and runs the
    action only if the row's current revision equals the choice's (T11)
    and the row still offers an enabled offer with the same action and
    label (T12). The log answers both (`ActivityLog::still_offered`).
  - Otherwise nothing runs, and the menu is still marked chosen (Cycle
    03a's execution ledger, Task 5 ruling R5-8).
- **Opening:**
  - by a right press on a row (`capture_any_mouse_down`,
    `prevent_default`, as fact 30), anchored at the pointer;
  - by `shift-f10` or `menu` in `ACTIVITY_CONTEXT`, for the highlighted
    row, anchored at the row's bounds, else the list's.
- **Closing and dropping:**
  - It is dropped while a dialog or sheet is open, and when its row
    leaves the log (focus to the list).
  - Escape restores focus to where it was.
  - Opening it closes the other layers (R-13).
- **Test accessors:** `activity_menu_key()` and `activity_menu_open()`.
  Every Kit menu has the same element id `"popup-menu"` (fact 33), so the
  sweeps read the row menu through `activity_menu_open()` and the menu
  entity, never by that id. The title allowance never extends to the
  dock, repository or More menus (Part G).

## Part E: The Needs Attention Popover

### E1. The Footer Item

- **Pressing Needs attention** (click, Enter or Space) toggles the
  popover. Today it reveals Activity (fact 28). The press keeps today's
  guard: nothing happens while a dialog or sheet is open.
- **Bounds:** its bounds are recorded like a panel toggle's (`Footer.bounds`,
  `D/view/footer.rs:399-412`). The popover anchors to them.
- **Tooltip:** "Things that need you. Your work is safe until you act."
  (fact 43).
- **Hiding while focused:** it follows the existing rule's direction,
  right (`D/view/footer.rs:265-283`; 3a R-38). When Needs attention hides
  because the count reached 0, focus moves to the next shown control to
  its right: the right dock group's first control, else Settings ("E1's
  control"). `focus_after_hiding(Attention, ..)` returns that, where it
  returns `None` today, and `keep_focus_in_footer` covers Attention
  (R-37).
- **It decides focus after a dialog** (R-49). `keep_focus_in_footer` runs
  in render (fact 28), so once a dialog has closed onto the hidden
  item's handle, it moves focus to E1's control before `focus_lost`
  could act (E2).
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
  `Anchor::BottomRight`. Its bottom edge sits 6 px above the footer's top
  edge (`05`: "bottom 34 px" over a 28 px footer), right-aligned to the
  item (R-38).
- It is 440 px wide. It is no taller than the window less the title bar
  and footer, and scrolls.
- **Contents:**
  - the header "Needs attention", with "Your work is safe on this
    computer. Nothing here happens until you choose." (`05`);
  - one row per attention row of the log, newest first: icon, title,
    description, button and tag, with the row's tooltip text (B5) as its
    tooltip and accessible description;
  - the link "Show everything in Activity".
- **Ids:** `"needs-attention-popover"`, `("needs-attention-row", ix)`,
  `("needs-attention-button", ix)`, `"needs-attention-show-all"`. An
  `"attention-…"` id would match `polling_privacy`'s `("attention", …)`
  prefix (fact 40).

**Keys,** in a new context `ATTENTION_POPOVER_CONTEXT`:

| Key | Does |
| --- | --- |
| `up`, `down`, `home`, `end` | Moves the highlight, with no wrap |
| `enter` | On the list: runs the highlighted row's button, or opens its item when it has none; nothing on a disabled button. On the link: activates it |
| `tab`, `shift-tab` | Three stops, wrapping: the popover's list (its own focus, holding the highlight), the highlighted row's button, and the link; Shift+Tab reversed. Focus stays inside the popover (R-12) |
| `escape` | Closes it, with focus back to the footer item |

**Opening and closing.**

- **Opening** focuses the popover's list, with the highlight on its first
  row. The Workspace keeps the list's focus handle across opens, as
  `attention_focus` (R-50).
- **The cursor.** The highlight follows its `RowKey`. When that row
  leaves, the highlight goes to the row now at its position, else the
  last row; with no rows, the popover closes (count 0). A click or Enter
  builds its choice from the drawn row for the highlighted key, from the
  popover's applied rows, with its revision. If the log's row has a newer
  revision, nothing runs (T8, T11; R-55). The same holds in Activity
  (C2).
- **Focus when the highlighted row changes.** The highlighted row can
  change while its button holds focus: the row left, or the row now at
  that position has no button. Focus then moves to the popover's list
  (`attention_focus`), so Escape and the arrows keep working (P25).
  Activity does the same for its list (C2).
- **Running a row's action** first checks the row's offer and revision
  (`still_offered`). If the check fails, nothing runs, and the popover
  stays open with the row refreshed. Otherwise it closes the popover,
  then runs the action through `choose_activity` with the footer item's
  handle as origin or restore.
  - **An action that places focus itself wins:** Show it, Open, Enter
    opening an item, and "Show everything in Activity".
  - **Any other action that opens no dialog** (Retry now, Resume and so
    on): focus goes to the footer item if shown, else E1's control.
  - **A dialog or sheet** it opens returns focus to the footer item. If
    the item has hidden by the time it closes, E1 decides: focus lands on
    E1's control (`keep_focus_in_footer`, fact 28; R-49).
    `the_popover_closes` pins this.
- **A layer opened while the popover has focus.** A user-lane passphrase
  prompt (`D/view/passphrase.rs:81`) or a right-press menu
  (`D/view/workspace/footer.rs:198`) takes `window.focused`, the
  popover's list, as its origin. When the popover is dropped, that
  handle is no longer drawn. Rule: the Workspace's `focus_lost` fallback
  (`D/view/workspace/focus.rs:275-295`) sends a lost focus that was
  `attention_focus` to the footer item if shown, else E1's control
  (R-50; U-8).
- **"Show everything in Activity"** closes the popover, turns the filter
  off, reveals Activity, and focuses its list with the highlight on the
  first row.
- **It also closes:**
  - on a press outside it (`on_mouse_down_out`). That press still
    reaches what is under it (R-11). The press keeps the focus it gave;
    if it gave none, focus goes to the footer item. The handler ignores a
    press inside the Attention control's recorded bounds (E1), so a full
    click on the footer item, whose click fires on mouse up, closes the
    popover once and leaves it closed (fact 47);
  - when the count reaches 0, with focus moved by E1's rule if it was
    inside;
  - when a dialog or sheet opens (dropped, as the menus are, fact 29);
  - when another layer opens: the row menu, ⋯ (the Activity More menu),
    the dock menu, the repository menu or Cycle 03a's More menu (R-13).
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
- **Reading.** The new getter `share_result(&self, id) ->
  Option<&SyncResult>` returns the stored result while the request is
  Finished. The view's `share_result`, `finished_share`, the item tab's
  `share_outcome` and `run_share_offer` read it. The view's
  `share_result(&services, id, cx)` keeps its signature and now returns
  the stored result.
- **A test-only form.** Today's three-argument
  `Operations::share_result(id, record, after_give_up)`
  (`D/model/operations.rs:477`) is renamed `classify_share(id, record,
  after_give_up)` and kept as `#[cfg(test)]`: a pure re-classification of
  the stored envelope, for the one test that re-classifies under another
  record on purpose (E-1).
- **Not classified once:** `share_words`' Busy holder and
  `shown_offers`' republish state, which describe other requests now.
  They are worked out at each refresh into `RequestFacts` (A4: never from
  render; R-9).
- **This is R-8.** The `Finished` event keeps its shape, so the 51 lines
  in 26 files that name it, 36 of them in 21 test files (fact 13), do not
  change.

### F3. The Running Stage Is Shown

This folds in an item of ticket `01M5CS4MH25QQ769HV61ZRKHXG`.

- **New stage words.** `stage_words(stage: ProgressStage, primary:
  Option<&str>) -> String`, in `model/operations/progress.rs`, gives
  words to every intent outside `Progressing`. They are `stage_text`'s
  share column with no remote (fact 14), except that Writing reads
  "Saving…". Integrating and Publishing name `{primary}`, else "main".
- **`state_label`.**
  - `Running { stage: Some(s), .. }` reads "Running: {words}", whether
    accepted or not.
  - `Running { accepted: true, stage: None }` still reads "Finishing".
- **A running request row** reads "{Doing}… · {words}" (B2).
- **The disabled "Finishing" control is kept.** It becomes the row's
  disabled button, for an accepted request with no Stop on offer.

### F4. The Old Columns Go

Task 9 removes the drawing code:

- `session_section`, `pending_section` and the entry, pending, attempt,
  attention, record and recovered drawing in `D/view/operations.rs` and
  its submodules;
- the focus maps of fact 8, with `prune_abandon_focus`;
- **the model text the drawing alone used** (`D/model/polling/entry.rs`),
  which the removal leaves unused in the non-test build:
  - `AttemptEntry`, `attempt_entry`, `EntryLine`, `LineAction`;
  - `RetireLine::text`, `retiring_text`, `finished_text`,
    `unfinished_text`;
  - `offer_text`, `RetireOffer::text`.

  Outside tests, only `D/view/operations/attempts.rs` uses them (`:40`,
  `:74`, `:99`), and `D/view/operations/retire.rs:120` uses
  `RetireOffer::text`, which calls `offer_text`. Task 9 deletes each, or
  gates it `#[cfg(test)]` where tests still import it:
  `D/view/operations/retiring_tests.rs:18`,
  `D/view/operations/retire_tests.rs:21`,
  `D/model/polling_retiring_tests.rs:122`, `D/model/polling/entry_tests.rs`
  and `D/services/scheduler_retire_tests.rs:1300`. Any that S7 or S8
  reuse moves to `D/model/activity/copy.rs` instead. `RetireLine` itself
  and `RetireOffer` stay: they are `PollSchedule`'s data and S7's input.

Every handler of A1's table stays, now called from
`Workspace::choose_activity` and `Workspace::run_activity_action(&ActivityAction,
origin: FocusHandle, window, cx)` in `D/view/activity/actions.rs`.
`D/view/operations.rs` keeps the handlers and `state_label` (fact 14). The
pure helpers `outcome_label`, `scope_name` and `effects_text` live in the
model from Task 3 and are re-exported from it (R-41). It drops below 500
lines, which Task 9 checks.

## Part G: Privacy

**What a row may hold** (contract 7). A row, its tooltip and detail, its
menu, its tag and a popover row hold only:

- the desktop's fixed copy;
- titles as read (decisions 12 and 21), never a draft's own title (B1);
- repository, remote, host and branch names (a remote by its name only,
  `repository_detail`, B4; R-48);
- result codes and the library's fixed messages (fact 16);
- effect values;
- operation IDs and item IDs;
- times and counts.

They never hold:

- a body, a comment or a conflict side;
- a remote URL or address (`RemoteAdd`'s is never shown, nor
  `SharedCopy.location`, which 03a's tooltip shows);
- a file path, a repository's folder path included (03a's tooltip's
  "Folder: {root}" is not Activity's detail);
- a key's label or path;
- a passphrase;
- a server's text.

**Sweeps at the swap** (Task 9, E-11, E-12). The drawn area changes in
Task 9, so its sweeps change with it:

- **D5's sweeps** (`D/view/lifecycle/privacy_tests.rs`,
  `privacy_promote_tests.rs` and `privacy_remote_tests.rs`):
  - They gain a title-only allowance: the `TITLE` and `READY` sentinels
    (both titles) may appear inside `"operation-area"`.
  - `BODY`, `COMMENT_TEXT`, `BASE` and `THEIRS` stay forbidden
    everywhere they are today.
  - The four `a16_` tests still assert that the operation area was read
    (fact 40).
- **`polling_privacy`:**
  - It gains the same title allowance (decision 21).
  - Its `SURFACES` are updated explicitly for the new ids. Its `read_at`
    pairs keep their meaning: a hold row's "Unlock…", a pending row's
    "Resume" and its menu's "Abandon…", the quit row's words.
  - It adds a surface for the log's rows read from the model: every
    title, description, detail line, offer label and tag, so tooltips
    are swept without hovering. `read_at`'s `("operation entry",
    "worktree_not_clean")` pair (`D/view/polling_privacy_tests.rs:997`)
    is now read from that surface, from the row's detail.
  - It gains two sentinels (P28): one in a repository root's parent
    folder name, and one in a remote URL's path and user-info, not its
    host, since host names are allowed (S3, B3). Both are swept through
    the model-rows surface here, and through the popover in Task 12
    (E-14).
- **The drafts sweeps** expect the new S11 title: `D/view/drafts_privacy_tests.rs`
  (`:272-276`) and `D/view/drafts_entry_tests.rs` (`:207-212`, which
  scans `"operation-area"` for a label starting "Recovery copy of"). Both
  are E-11's, in Task 9 only.

Task 9 runs `polling_privacy` and D5's four `a16_` tests.

**Sweeps of the new layers** (Task 12, E-14):

- the popover (`"needs-attention-popover"`) and the row menu, read
  through `activity_menu_open()` and the menu entity, not the shared
  `"popup-menu"` id (fact 33), so the title allowance never extends to
  the dock, repository or More menus;
- the positive control: the title sentinel is seen in an Activity row,
  and a body sentinel planted in the same item is not;
- the folder and remote sentinels of E-12, swept through the popover.

**Document amendments.** D5's and D6's privacy rules get one dated
amendment (Document Amendment 6).

## Part H: Keyboard And Focus

| Surface | By keyboard |
| --- | --- |
| Activity | `secondary-4` or F6 to the list; arrows, Home and End; Enter presses the button or opens the item; `shift-f10` or `menu` opens the row menu; Tab reaches the filter, ⋯, the list and the highlighted row's button |
| Row menu | The Kit menu's keys; Escape returns focus |
| ⋯ menu | As Cycle 03a's More |
| Footer Needs attention | A tab stop; Enter or Space toggles the popover; hiding while focused, focus moves right (E1) |
| Popover | Arrows, Home, End and Enter; Tab cycles the list, the highlighted row's button and the link; Enter on the link activates it; Escape returns to the footer item |
| After a dialog | From Activity: the list, the row still highlighted (decision 23). From the popover: the footer item, or E1's control when it has hidden (E1, E2; R-49) |
| After a layer opened from the popover | A passphrase prompt or a right-press menu that took the popover's focus as its origin: the footer item, or E1's control (E2; R-50; U-8) |
| Behind a dialog or sheet | Nothing acts: every new action goes through `unless_dialog`, and every layer is dropped (R-60, fact 36) |

Every control has an accessible name and visible focus (RFC 344-349).
Colour is never the only signal: every row has an icon and words
(contract 6).

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | Each source S1–S14, in each of its states; a ContextRetire running, then answered with each verdict, Owed with a Success envelope included; a lifecycle's pending row, idle and queued or running elsewhere; Queued, and Running not accepted | Its rows, with key, kind, title, description, button, menu, `clearable` and `group` as A1, A2 and B2–B4 say; every row has a kind and a title; every request's subject is its `Intent` subject with its target (R-57); a running ContextRetire is its S1 row, a dismissed answer one S8 event (`Retired(seq)`, or the item's `RetireNote`, R-59), any other answer its S1 row, an Owed one reading "didn't finish" and not clearable (R-51); the lifecycle row offers Resume (`ResumeLifecycle`), or a disabled "Still running…" and no candidate (N25); Cancel for Queued and for Running not accepted (B2) |
| P2 | Each row of A3's table, N1–N25 (N20 withdrawn), with an N4 row for each failure class and a row for each of N22's clauses, over `derive` and `ActivityLog::refresh` | The rows shown and the count it says, read from the log and from the footer |
| P3 | Each row of A4's table, T1–T18 (the rules L1–L9 in action): T10 through `refreshes_are_coalesced`, a window test (Task 5), and T8 through P15–P17 (Task 9) | The outcome the row says; a retried request keeps its time and place (T2); a dismissed request that runs again is a new row at the top (T3) |
| P4 | Every intent of B2's table, in each request state | Its title and description |
| P5 | Each `SyncOutcome`, with a record Accepted, Finished and Unknown, and after a Give up | B3's words; the button is the first offer, the menu holds the rest in today's order |
| P6 | Each state row of B4, each disposition and each pending action and next action | Its words; the pending action table is total |
| P7 | A time with a known offset, an unknown one, and around midnight | "HH:MM" and the detail's full time; " UTC" with no offset |
| P8 | An automatic check and a Check now that brought in changes; ones that brought in nothing; a failed one | One brought-in row each for the first two, with their two sentences; none for the rest |
| P9 | A finished share with a noted attempt; a retry; a fabricated `Finished` with no noted attempt | The result is classified once, from the noted record, when it finishes; the retry clears it; `Unknown` and no Give up |
| P10 | A running request with a stage, of an intent with stage words and of one without; accepted with no stage; accepted with no Stop | Its words; "Finishing"; the disabled "Finishing" button |
| P11 | The operations model split | Every existing test of `D/model/operations*` passes unchanged |
| P12 | `each_repository` | Cycle 03a's panel tests pass unchanged after `RepoFacts` building moves (run by name in Task 4) |
| P13 | A journey with one row of each counted kind, then each resolved | Each step asserts the expected list of attention `RowKey`s, written from the N-table, the popover's rows and the footer's count, and they agree |
| P14 | Activity with rows; with the filter on; with none; a row's tooltip | The rows newest first; only attention rows; the two empty texts; the tooltip text is `tooltip_lines`' (B5): the full message, the time line, then the detail |
| P15 | Activity by keyboard | Up and down stop at the ends; Home and End; Enter presses the button, opens the item when there is none, or does nothing (a disabled button, or neither); Enter on the focused row button runs its action once, and the list's `enter` does not also fire; Enter on a row drawn before the log's row took a newer revision runs nothing (T8); the highlighted row leaving, or the row now at its position having no button, while its button holds focus moves focus to the list (C2); `shift-f10` and `menu` open the menu; Tab runs filter, ⋯, list, highlighted row's button (this clause in Task 10, with the filter) |
| P16 | A left click on a row; on its button; a right click; a click on a button drawn before the log's row took a newer revision | Highlights only; runs the button once; opens the menu for that row; nothing runs (T8) |
| P17 | The row menu by pointer and by keyboard; a choice whose row changed in between | The menu's first item is the full message, then the detail lines (all disabled), the offers, Dismiss; for the changed row, nothing runs, and the menu closes (T8, T11) |
| P18 | Each `ActivityAction` kind, `ResumeLifecycle` included, through `choose_activity` (Task 6), then from a row's drawn button and from its menu (Task 9) | Its handler of A1's table, sending exactly what today's control sends |
| P19 | "Clear finished" with a success; a stopped share whose record is Accepted (it offers Give up); a stopped republish that a lifecycle waits on; a failure; a failure settled under N22 that still offers Retry; an attention row; a state; an S8 Retired event; an S8 information event; a cancelled close (it offers Open…) | Only the success, the settled failure, the two S8 events and the cancelled close go; the stopped share and the stopped republish stay (T9; R-47); disabled when nothing is `clearable` |
| P20 | Republish…, a recovery that confirms and Open…, each chosen from a row's button, its menu and Enter; Abandon…, chosen from the row's menu, never its button (B2); each then cancelled | Focus back in the list, the same row highlighted (decision 23) |
| P21 | Activity moved into the right dock and set to 320 px with the layout model's API, as Cycle 01's dock tests do, with the longest message of each kind, a long repository name and "+N"; each row scrolled into view (`scroll_to_item`) before its bounds are read | Every row's button lies inside the list's bounds |
| P22 | The footer item by click, twice; by Enter, then Escape; by Space, then Escape | The popover opens with the attention rows, newest first, then closes |
| P23 | The popover by keyboard, for every attention source and each button it can carry (the table below); Enter on a row whose offer or revision changed since it was drawn | Opened by keyboard, the row highlighted by arrows, Enter runs the button through `choose_activity` with the footer item's handle as origin, after `still_offered` passes and the popover closes; for the changed row, nothing runs and the popover stays open with the row refreshed (E2); Home and End move; Tab stays inside; Escape returns to the footer item |
| P24 | The popover with Activity hidden; "Show everything in Activity" | It works; Activity is shown with the filter off and its list focused |
| P25 | The count reaching 0 while the popover is open; a dialog or sheet opening; another layer opening; a press outside; a full click on the footer item while the popover is open; a dialog from the popover closing after the item hid; a dialog or menu opened while the popover had focus, then closed; the highlighted row leaving, or the row now at its position having no button, while its button holds focus | It closes each time, but for the last case; focus moves by E1's rule when it was inside; a press outside keeps the focus it gave, else the footer item; the full click leaves it closed (fact 47); the dialog's focus lands on E1's control (R-49); the dialog's or menu's focus lands on the footer item, or E1's control (R-50); in the last case it stays open, focus moves to its list (`attention_focus`), and Escape and the arrows still work (E2) |
| P26 | The focused footer item hiding | Focus moves to the next shown control to its right: the right dock group's first control, else Settings |
| P27 | The toolbar, the row menu keys, Enter in the list, the footer item and the popover keys behind a dialog; behind a sheet | Nothing acts |
| P28 | Sentinels in a title, body, comment, conflict side, remote URL, key label and passphrase, through every surface; one in a repository root's parent folder name and one in a remote URL's path and user-info (host names are allowed, S3, B3), through the model-rows surface and the popover (E-12, E-14) | Only the title appears, and only where Part G allows |
| P29 | Rows arriving while Activity is hidden, then Activity shown | The same rows and times as if it had been shown |
| P30 | A check that answers while the repository's row is shown, and a retry that starts | The row stays through the check (rule L3) and leaves or stays by the answer |
| P31 | Moving the highlight, toggling the filter, opening and closing the row menu, ⋯ and the popover, and "Show everything in Activity" | Nothing is sent (contract 3) |
| P32 | A window with items not shared yet, main among them | The footer's "N to share" (Cycle 03a's `ShareFacts` total, `D/view/workspace/footer_status.rs:59`) equals the number of S6 rows of that moment, main counted as one |

**P23's table.** For each source and button, the popover is opened by
keyboard, the row highlighted by arrows, and Enter runs it through
`choose_activity` with the footer item's origin. Together with P18, which
maps every `ActivityAction` to its handler, every popover action is proved
by keyboard.

| Source | Buttons |
| --- | --- |
| S1, a counted failed request | Review changes; each share offer that can be a button (Share, Retry, Unlock…, Open Sharing & keys, Check host, Set identity, Show it, Set up sharing, Republish…); each recovery action that can be a button; Open… (lifecycle) |
| S3 | Each `Reason`'s action |
| S4 | Each attention state's `RowAction`: Retry, Reread, Settings…, Retry now, Check now, Resume checking, and Attend's |
| S5 | Open |
| S9 | Resume, Review changes, a lifecycle's Resume (`ResumeLifecycle`) |
| S10, S11 | Retry |

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
      `navigation_sends_nothing` (P31, with the recording backend),
      `reminders_match_to_share` (P32, Task 10), `count_and_popover_agree`
      (P13), `refreshes_are_coalesced` (P3's T10). Task 5 creates the
      file, for the last two.
  - **`D/view/needs_attention_tests.rs`:**
    - `the_popover_opens_lists_and_closes` (P22),
      `popover_actions_by_keyboard` (P23);
    - `the_popover_works_with_activity_hidden` (P24),
      `the_popover_closes` (P25),
      `nothing_acts_behind_a_dialog_or_sheet` (P27).
  - **Other:** P26 is in the footer's status table (E-13); P28 is the
    sweeps (E-12, E-14).
- **P23 without a journey per action.** `popover_actions_by_keyboard` is
  a table: it drives its fixtures through the scheduler `World`,
  `PollingBackend` answers and fabricated `WorkerEvent`s, so it does not
  need one journey per action.
- **Test-first:**
  - `attention_counts_each_thing_once` is written from A3's table, and
    `the_log_keeps_times_and_events` from A4's T-table, before any of
    their code, under one design checkpoint (Plan, Test-first);
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
- `act_refused(shell, &RowKey, label, cx)`
- `request(shell, DesktopRequestId) -> DesktopRequest`
- `pending_key(shell, root, action) -> RowKey`

**How `act` works** (R-45). `act` does not dispatch a window action.

- **The seam.** `act(shell, key, label, cx)` cannot pass vacuously:
  1. it first refreshes the log synchronously, by calling
     `refresh_activity_now` directly (A4);
  2. it panics unless exactly one enabled offer in `row.menu` has that
     label. `row.menu` holds every offer, the button's included, so the
     button is not counted twice;
  3. it builds the `ActivityMenuChoice` for that offer, with the row's
     current revision, and calls `Workspace::choose_activity(&ActivityMenuChoice,
     origin: FocusHandle, window, cx)` directly, inside
     `workspace.update`. It runs the same re-check a menu choice runs
     (Part D).
- **A deliberate refusal.** `act_refused(shell, key, label, cx)` asserts
  that the offer is absent or disabled, and that `choose_activity` runs
  nothing (T12).
- **A stale revision** (T11) does not go through `act_refused`. Task 6
  builds an `ActivityMenuChoice` with an old revision and calls
  `choose_activity` directly.
- **The reason.** `Window::dispatch_action` goes to the focused node (or
  the root when nothing is focused) and runs deferred
  (`gpui-pre-0.3.6/src/window.rs:2442-2454`, `:6244-6252`).
  `FocusHandle::dispatch_action` does nothing for an undrawn handle
  (`:628-636`). A Kit menu choice focuses `action_context` first
  (`C/menu/popup_menu.rs:892-902`).
- **The menu's action handler** (Task 9) calls the same
  `choose_activity`, with the list's handle.
- **Origins during Tasks 6–8.** `run_activity_action` receives an origin.
  The seam passes today's control handle for that action
  (`control_focus_for` and `abandon_focus_for` by the old id), so
  converted tests that also read focus stay green until Task 9 replaces
  the origins with the list's handle (C3; decision 23).
- **Task 9 reruns E-5's and E-6's tests by name**, a doubt the plan
  names, because the swap changes the origins `act` passes and deletes
  the focus maps.

**The migration.** Tasks 6 to 8 move the tests of fact 38 onto the seam
while the old columns are still drawn. They pass before and after, and
assert the same behaviour (E-5, E-6). Task 9 then swaps the drawing
without touching them.

- **Excluded from conversion.** Every test in E-7 is left out of E-5 and
  E-6. Those tests keep pressing by element id until Task 9 rewrites them
  to Part H's keys.
- **The read conversion rule** (E-5, E-6). A label read that compared
  today's words becomes a read of model state:

  | Today's read | Becomes |
  | --- | --- |
  | state | `request(..).state` |
  | outcome words | `share_result(id)`'s `SyncOutcome` |
  | offers | `offered(..)` returning `Offer`s whose actions are compared |
  | code, message, effects | `request(..).state`'s `Finished` fields |
  | command | `request(..).intent` |
  | pending command, next | the pending row's `RowKey` and the model's `OperationDto` |

  These are **Removes** of drawn-text assertions. The copy is asserted by
  P4–P7. The ledger lists each removed text assertion per file.
- **What proves the drawn controls.** A converted test no longer proves
  its control is drawn. P18 proves each action kind from a drawn button
  and the menu, and P23 from the popover (R-21).

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
| E-1 | `D/view/operations/give_up_tests.rs` `a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up` (`:340-347`) | Its direct three-argument `share_result(id, record, gave_up)` call becomes `classify_share(id, record, gave_up)`, the `#[cfg(test)]` form, since it re-classifies under another record on purpose (F2). `D/view/operations/outcomes_route_tests.rs:91` calls the view's `share_result(&services, id, cx)`, which keeps its signature and now returns the stored result: no edit | 2 |
| E-2 | `D/view/operations/check_tests.rs` `a_running_check_now_reads_its_stage_words` (`:111-118`) | The poll policy verbs at the Writing stage read "Running: Saving…", not `FINISHING` (F3) | 2 |
| E-3 | `D/model/attention_tests.rs` `counts_what_awaits_the_person` (`:172`); `D/model/attention_retire_tests.rs` `retire_rows_dismissed` (`:143`); the helpers of `D/model/attention_polling_tests.rs`; `D/services/scheduler_retire_tests.rs` `retire_resume_resends_original` (its `needs_attention` call at `:691`) | `needs_attention` goes (A6). Each case of the first two becomes a row of `attention_counts_each_thing_once`, named in the ledger. `retire_resume_resends_original` is a scheduler `World` test with no Workspace, so it builds an `ActivityLog` from the pure `derive` over the `World`'s operations and schedule, with no repository inputs, calls `refresh`, and reads `attention_count()`. **Removes** the module | 5 |
| E-4 | `D/view/footer_tests/mod.rs` `needs_attention_counts_and_leads_to_activity` (`:279`); `D/view/keyboard_tests/shell.rs` `needs_attention_works_by_enter` (`:377`) and `footer_controls_are_reached_by_tab` (`:409`); `D/view/workspace_tests.rs` `drafts_not_saved_with_activity_hidden_focuses_the_item_area` (`:537`); every other reader of the count or the Attention control (fact 41): `D/view/dock/restore_tests.rs` (`:209`, `:219`, `:366`), `D/view/keyboard_tests/shell/journey.rs:104`, `D/view/footer_tests/status.rs:356` (the `status_items_say_and_choose` journey) and `D/view/footer_tests/status_table.rs:211` | A count they assert changes only where N1–N25 count their fixture differently; each change is recorded in the ledger with its N row. Task 5 runs all of them by name | 5 |
| E-5 | Under `D/view/operations/` and `D/view/operations_tests.rs`, leaving out every E-7 test. **Part 1 (Task 6):** **`operations_tests.rs`:** `launch_refreshes_each_repository_in_the_background_and_a_manual_refresh_runs_first`, `a_failed_refresh_and_rebuild_stay_until_dismissed_and_retry_succeeds` and its helper `recovery_of`, `a_lease_held_by_another_process_is_the_refreshs_typed_answer`, `a_pending_local_operation_offers_resume_and_abandon_confirms_it`, `resume_finishes_a_pending_local_operation`, `an_abandon_by_a_second_actor_is_answered_and_the_row_goes`. **`give_up_tests.rs`:** helpers `code`, `accepted_share`, `give_up`; `give_up_is_offered_exactly_when_the_record_is_accepted`, `give_up_is_offered_on_exactly_the_rows_that_name_it`, `a_give_up_left_waiting_on_unlock_offers_unlock_not_give_up`, `giving_up_a_finished_record_says_already_finished`, `stop_is_not_offered_without_a_remote`, `a_stopped_resume_over_an_unsettled_push_reads_not_confirmed`, `a_stopped_resume_that_then_fails_the_transport_reports_the_reservation`. **`outcomes_tests.rs`:** helpers `offered`, `outcome`, `assert_row`, and its 13 tests. **`outcomes_route_tests.rs`:** `no_selected_key_opens_sharing_and_keys`, `a_host_code_checks_the_host_with_its_authority`, `unlock_clears_the_declined_key_and_retries_the_same_request`, `share_after_a_finished_locked_key_clears_its_declined_key`, `busy_without_a_known_holder_points_to_pending_operations`, `a_final_failure_on_a_finished_record_offers_share_not_retry`. **`check_tests.rs`:** the state read of E-2. **`record_tests.rs`:** `an_unreadable_record_is_reported_once_in_activity`. **Part 2 (Task 7), their tests outside E-7 only:** **`attempts_tests.rs`:** `dispositions_listed`, `attempt_entry_share_waits_for_the_lock`. **`attention_tests.rs`:** `attention_locked_key`, `attention_host`, `attention_actions_by_reason`, `attention_recovery_held`. **`republish_follow_tests.rs`:** all seven tests. **`republish_stop_tests.rs`:** `under_a_resume` and its callers, and `a_stop_that_cannot_be_queued_says_so`. **`republish_tests.rs`:** `a_deleted_branch_offers_republish`, `republish_confirms_then_shares`, `a_stopped_republish_sends_the_same_request`, `an_item_shares_stop_and_republish_opens_republish`, `republish_waits_for_another_dialog_to_close`, `an_unreadable_record_offers_give_up_and_it_sends_the_give_up`, `republish_opens_when_the_dialog_it_waits_for_is_cancelled`. **`resume_tests.rs`:** `close_row` and the six tests that press Resume, plus `a_lifecycle_row_offers_resume_after_restart`. **`retire_tests.rs`:** `retire_left_unkept_ask`, `retire_left_kept_draft_ask`, `retire_flush_failure_submits_nothing`, `retire_unkept_text_is_asked_before_drafts`, `retire_after_a_quit_closed_its_question`. **`retiring_tests.rs`:** none (its one test, `retire_rows_hit`, is in E-7) | A press by element id becomes `act(shell, key, LABEL)`. A presence check becomes `offered`. A read of today's words becomes a read of model state, by Testing's read conversion rule: `request`, `share_result`, `offered`, or the pending row's key and `OperationDto`. These are **Removes** of drawn-text assertions: the copy is asserted by P4–P7, and the ledger lists each removed text assertion per file. A pending row by index becomes `pending_key`. The lifecycle Resume tests (`resume_tests.rs`) press Resume with `act(.., "Resume")` on the pending row (S9's `ResumeLifecycle`); their meaning is unchanged. The seam passes today's control handle as origin (R-45). Keyboard and focus steps stay as they are until E-7. Every other assertion keeps its meaning; the ledger lists each file's test functions before and after | 6, 7 |
| E-6 | Outside `D/view/operations/`, with the same edit as E-5, leaving out every E-7 test. **`D/view/conflict/entry_tests.rs`:** `row_of`, `the_banner_offers_review_changes`, `a_pending_resolve_row_offers_review_changes_after_restart`, `a_local_synchronization_offers_resume`, `resolve_conflict_opens_the_childs_sheet`, `an_applied_resolution_clears_the_banner_and_its_review`, `a_resume_that_settles_the_conflict_clears_its_review`. **`D/view/conflict/repair_tests.rs`:** its three tests. **`D/view/confirm_tests.rs`:** the six that press Abandon other than `escape_closes_the_dialog_with_no_request_and_focus_returns_to_abandon` (E-7). **`D/view/find_tests.rs`:** `find_does_nothing_while_a_dialog_is_open`. **`D/view/tabs_tests.rs`:** `the_tab_keys_do_nothing_while_a_dialog_is_open`. **`D/view/item/share_tests.rs`:** its nine tests of fact 38. **`D/view/item/edit_release_tests.rs`:** `retry_control` and its three tests. **`D/view/item/posting_tests/`:** `outside.rs` `recovery_labels` and its three tests; `decisions.rs` `a_post_stopped_on_a_conflict_raises_the_items_banner`; `extras.rs` `quitting_during_an_accepted_post_leaves_it_saved_and_cancelled`; `recovery.rs` `p3_a_stopped_post_offers_share_not_retry`; `review.rs` `an_operation_area_retry_is_followed_and_posts_one_comment`. **`D/view/lifecycle/`:** `outcome_tests.rs` `the_operation_entry_reopens_the_dialog`; `outcome_canned_tests.rs` `reopen` with `each_way_out_offers_its_button`, `busy_after_the_merge_is_failed_not_still_running` and `a_lifecycle_entry_keeps_the_recovery_its_dialog_does_not_own`; `interruption_tests.rs` `p5_a_close_stranded_at_closing_resumes_after_relaunch` and `a_close_lost_before_settlement_is_complete_and_stays_accepted`; `made_ready_tests.rs` `entry_texts` and `done_lists_made_ready_with_slug_and_title`; `privacy_consent_tests.rs` (two `a17_` tests); `privacy_promote_tests.rs` `a16_a_promotion_stopped_untidied_and_done_shows_no_item_text`. **`D/view/lifecycle_tests.rs`:** `a_blocked_prepare_lists_blockers_and_way_out`. **`D/view/dock_tests/state.rs`:** `a_request_finishes_while_activity_is_hidden`. **`D/view/drafts_entry_tests.rs`:** its two tests, less the `"operation-area"` scan at `:207-212`, which is E-11's, in Task 9 only. **`D/services/drafts_tests.rs`:** its two tests. **`D/services/drafts/save_tests.rs`:** `dismissing_a_pending_save_in_the_operation_area_lets_another_be_sent`. **`D/view/settings/keys_tests.rs`:** `the_operation_area_retry_of_a_protected_generation_asks_on_the_page`. **`D/view/settings/remotes_tests.rs`:** its three tests. **`D/view/settings/repositories_tests.rs`:** `a_partial_create_is_completed_by_retry_under_the_same_request`, including its literal `format!("request-{}-code")` (`:569`). **`D/view/settings/repositories_manage_tests.rs`:** `a_release_that_failed_during_removal_completes_once_retry_stores_the_text` | As E-5. `p5_a_close_stranded_at_closing_resumes_after_relaunch` presses Resume with `act(.., "Resume")` on the pending row (S9's `ResumeLifecycle`); its meaning is unchanged. **Removes** drawn-text assertions as E-5 does | 8 |
| E-7 | Keyboard and focus steps, rewritten to Part H's keys. Left out of E-5 and E-6: these keep pressing by element id until Task 9. **`operations_tests.rs`:** `operations_keep_focus`, `dismiss_and_retry_from_the_keyboard_leave_focus_in_the_operation_area`, `a_recovery_dialog_gives_focus_back_to_the_action_that_opened_it`. **`attention_tests.rs`:** `attention_actions_hit`, `attention_actions_by_keyboard`. **`attempts_tests.rs`:** `attempt_entry_actions_hit`, `attempt_entry_ids_follow_the_repository`. **`retire_tests.rs`:** `retire_left_offer`, `retire_pending_row_by_keyboard`. **`retiring_tests.rs`:** `retire_rows_hit`: its drawn retirement-line text assertions (`:113-126`) are **Removes**, since R-51 no longer draws those lines; its Tab and Dismiss step (`:142-149`) moves to the refused request's S1 row; its bounds checks are E-8's. **`outcomes_route_tests.rs`:** `show_it_focuses_the_entry_of_the_possible_holder`. **`keyboard_tests.rs`:** `Walk::open_the_operation_area` and `Walk::start_a_confirmed_action_and_escape`, in `every_core_action_works_by_keyboard_alone`. **`keyboard_tests/actions.rs`:** `resume_by_keyboard`, `cancel_a_queued_request_by_keyboard`. **`keyboard_tests/confirm.rs`:** `open_abandon` and its four tests. **`keyboard_tests/conflict.rs`:** `resolve_a_local_conflict_by_keyboard`, with its own copy of `PENDING_REVIEW`. **`keyboard_tests/conflict/sheet.rs`:** `body_sheet` and its two tests. **`keyboard_tests/drafts/quit.rs`:** `a_draft_not_saved_on_quit_by_keyboard`. **`keyboard_tests/shell/polling_journey.rs`:** `polling_keyboard`. **`confirm_tests.rs`:** `escape_closes_the_dialog_with_no_request_and_focus_returns_to_abandon` (`:145-159`). **`passphrase_tests.rs`:** `a_prompt_above_the_confirmation_dialog_is_answered_by_its_own_dialog` (its step at `:334`). **`workspace_tests.rs`:** `a_draft_that_cannot_be_saved_asks_before_quitting` (`:475`, which asserts focus on `draft_flush_id(id, "retry")`). **`dock_tests/state.rs`:** `show_it_reveals_a_hidden_activity` | A control is reached by focusing Activity, moving the highlight to its row, then Enter, or `shift-f10` and the menu. "Focus returns to the control" becomes "focus returns to the list, the row highlighted" (C3; decision 23). `Workspace::abandon_focus` goes. **Removes** `retire_rows_hit`'s retirement-line text assertions, each listed in the ledger | 9 |
| E-8 | The bounds assertions of `retire_left_offer` (`:209-217`), `retire_pending_row_by_keyboard` (`:620-631`) and `retire_rows_hit` (`:129-138`), and the four window resizes made to reach a control (fact 38): `attention_actions_hit` (`attention_tests.rs:371`), `attempt_entry_actions_hit` (`attempts_tests.rs:181`), `a_draft_not_saved_on_quit_by_keyboard` (`keyboard_tests/drafts/quit.rs:70`) and `polling_keyboard` (`keyboard_tests/shell/polling_journey.rs:133`) | Containment is P21's test, `rows_fit_a_narrow_activity`, for every row kind. **Removes** the per-test bounds checks and resizes | 9 |
| E-9 | `D/keys_tests.rs` `each_key_is_bound_to_its_action` | Gains each binding Parts C2 and E2 add, tested where its keys land: Activity's in Task 9, the popover's in Task 11; its fixed length follows | 9, 11 |
| E-10 | `D/view/dock_tests/toolbars.rs` `panel_toolbars_work_by_tab_and_enter` | Activity's filter and ⋯ join the loop; `ToolbarButton::ALL`'s length follows | 10 |
| E-11 | `D/view/drafts_privacy_tests.rs` (`:272-276`); `D/view/drafts_entry_tests.rs` (`:207-212`, which scans `"operation-area"` for a label starting "Recovery copy of") | Expect S11's title, "The recovery copy of" | 9 |
| E-12 | D5's three sweeps (`D/view/lifecycle/privacy_tests.rs` `ALLOWED` and `Seen`, used by `privacy_promote_tests.rs` and `privacy_remote_tests.rs`); `D/view/polling_privacy_tests.rs` `polling_privacy` | The sweeps at the swap (Part G): the title-only allowance (`TITLE` and `READY` inside `"operation-area"`) in D5's sweeps and in `polling_privacy` (decision 21); `SURFACES` updated explicitly for the new ids; `polling_privacy`'s model-rows surface, which keeps `read_at`'s `("operation entry", "worktree_not_clean")` pair, now from the row's detail (`:997`); two sentinels, one in a repository root's parent folder name and one in a remote URL's path and user-info (host names are allowed), swept through the model-rows surface (P28). Task 9 runs `polling_privacy` and D5's four `a16_` tests | 9 |
| E-13 | `D/view/footer_tests/mod.rs`: `needs_attention_counts_and_leads_to_activity` becomes `needs_attention_counts_and_opens_the_popover`, and `status_items_say_and_choose`' status table (`focus_after_hiding(Attention, ..)` at `D/view/footer_tests/status_table.rs:211`) and journey (`D/view/footer_tests/status.rs:356`); `D/view/keyboard_tests/shell.rs` `needs_attention_works_by_enter` (`:377`) and `footer_controls_are_reached_by_tab` (`:409`); `D/view/keyboard_tests/shell/journey.rs:104`; `D/view/dock/restore_tests.rs` (`:209`, `:219`, `:366`); `D/view/footer_tests/mod.rs` `footer_controls_do_nothing_behind_a_dialog` (`:334`), which presses `Control::Attention` behind a dialog | A press and Enter open the popover, not Activity (E1); the hidden item's focus moves right (P26). Task 11 runs all of them by name | 11 |
| E-14 | D5's three sweeps and `polling_privacy`, as E-12 left them | The popover (`"needs-attention-popover"`) and row-menu surfaces, the row menu read through `activity_menu_open()` and the menu entity, not `"popup-menu"`; the positive control (Part G); E-12's folder and remote sentinels, swept through the popover (P28) | 12 |
| E-15 | Appearance tests broken by the deliberate redesign, found in any task | Deleted, not updated (Wave Testing), and named in the ledger with what they asserted; the task's reviewer confirms each. A test that checks a control is visible in order to reach it is behaviour, and stops for the owner. **Removes** | — |
| E-16 | `D/assets_tests.rs` `every_item_view_icon_loads` and its `ICONS` array | Each icon Parts A and C use is checked first (`list-filter`, `ellipsis`, `refresh-cw`, `cloud-upload`, `circle-check`, `circle-alert`, `wifi-off`, `lock`, `info`, `loader-circle`); a missing one is added, and the array's length follows | 9 |

## Document Amendments

Applied by the planner in the approval commit, not by an implementation
task:

1. **Wave 04, Cycle 03b section.** "Amended 2026-11-13 by Cycle 03b":
   - the count includes every attention row once (decision 6);
   - checks that changed something get rows (decision 7);
   - titles appear in Activity and the popover, under D5's and D6's rules
     (decisions 12 and 21);
   - the K-19 rows are one-line rows (decision 22);
   - "its single action" becomes "at most one action" (decision 24);
   - "failures appear in both Activity and the popover" means every
     counted failure (decision 24).

   The Risks row "Needs attention misses the attention states D6 adds" is
   marked settled.
2. **`03-panels.md`, Activity:**
   - the filter shows the rows Needs attention counts;
   - ⋯ holds "Clear finished";
   - each row has a menu (right click, `shift-f10`, `menu`) whose first
     lines are the full message, then the detail (decision 22);
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
   - when it hides while focused, focus moves to the next shown control to
     its right (E1).
5. **`06-language-and-states.md`:** a row for retirement ("Retired the
   copy of … on this computer", Activity), and a note that Activity's
   full copy is Cycle 03b's design Part B.
6. **The privacy rules of D5 and D6** (fact 45). One dated amendment,
   applied to four texts:
   - D5's design, Privacy (`docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md:694`);
   - D5's Cycle contract 10 (`docs/Cycles/wave-03-desktop-05-recovery-and-completion.md:197-198`);
   - D6's Cycle contract 10 (`docs/Cycles/wave-03-desktop-06-polling-and-shutdown.md:353-354`);
   - D6's design (`docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md:1095`
     and its Privacy section, `:1520-1528`).

   Each reads: "Amended 2026-11-13 by Wave 04 Cycle 03b decisions 12 and
   21: a title may appear in an Activity row, its tooltip and menu, and
   the Needs attention popover; no body, comment, side, result text or
   path does."
7. **Cycle 01's design, K-19:** "Fixed by Wave 04 Cycle 03b's one-line
   rows."
8. **Cycle 03a's Cycle document and design, K-9:** "Decided by Cycle 03b
   decision 6."
9. **`08-gpui-kit-mapping.md`:**
   - Needs attention is a Workspace-owned layer, following the app's dock
     and repository menus, not the Kit `Popover`, whose controlled mode
     would split its rules across two mechanisms (R-10; fact 32);
   - Activity's list is GPUI's `uniform_list` (R-36).
10. **D6's decision 13 and design W-6** (`docs/Cycles/wave-03-desktop-06-polling-and-shutdown.md:459`,
    and the scope line at `:184`;
    `docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md:1249`,
    and the "Rows wrap inside Activity" sentence at `:1095`): "Amended
    2026-11-13 by Wave 04 Cycle 03b decision 22: Activity's rows are one
    line; the message ends in an ellipsis and the button never leaves the
    row; the full text is the row's tooltip, accessible description and
    menu."
11. **Cycle 01's decision 3** (`docs/Cycles/wave-04-cycle-01-workspace-shell.md:315-321`):
    "Superseded by Cycle 03b decision 6 (2026-11-12) and decision 25
    (2026-11-13): the count is the popover's rows; a pending row with
    nothing to do no longer counts; a failed request stops counting once
    fixed, and its row stays in Activity until dismissed or cleared."
12. **Cycle 03a's design R-38** (the footer's focus after hiding):
    "Extended by Cycle 03b: Needs attention follows the same rule."

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | Rows come from the models the operation area reads today; nothing new is stored on disk (decision 2) | Rows and times are lost at quit (K-1) |
| R-2 | The log lives in the Workspace and is refreshed at A4's refresh points, never in render | A change that reaches no refresh point leaves a row stale until the next one; P13 and P29 cover each source |
| R-3 | A hold's key is `hold_key(&Reason)`: the variant's name and its key ID, authority or root. `Reason` has no order (fact 19), and adding one would need the library's `SharedKeyId` to change | None while the string names the variant and its value |
| R-4 | A pending row's key is its operation ID, or "#{n}", its position in its repository's list, when it has none | A pending row with no ID can take a new time when an earlier one leaves (K-10) |
| R-5 | `RepoFacts` building moves, unchanged, into `each_repository` (A7). Cycle 03a's `RepoState` gains `cause: Option<AttentionCause>`, set by `attention_cause` | None: one function, and Cycle 03a's tests, which compare `RepoState` field by field, pass unchanged (P12) |
| R-6 | A pending conflict makes no repository row (N10; decision 20) | After a relaunch with the pending list unread, nothing in Activity says why checking waits; the panel's row still reads Needs attention (K-9) |
| R-7 | A failed Check now counts as the candidate hold or repository row covering its root (`Group.roots`) when `check_failed` applied its answer, or as the row L3 keeps (N19); otherwise as the request (N4, N18, N19; R-53) | None: an applied answer is in the schedule (fact 21), so the covering row says it, and a row kept by L3 still covers the root |
| R-8 | A share is classified once by the drain noting its record and Give up before `apply` (F2), not by changing `WorkerEvent::Finished` | The drain notes the attempt before applying `Finished`. Its resends retry other requests before `apply` (`D/services/mod.rs:718`, `:733`). No resend targets a share today; a share resend would have to come after the note |
| R-9 | The Busy holder's words and the republish state are worked out at each refresh into `RequestFacts`, not classified once (A4: never from render) | They change while the row is shown, which is what they describe (K-12) |
| R-10 | The popover is a Workspace-owned layer, not the Kit `Popover` (fact 32). The app's own layer pattern (the dock and repository menus) already holds the rules this popover needs: dropped under a modal, one layer at a time, focus restore, the anchor from recorded bounds, and its own key context for the list. The controlled Kit `Popover` keeps its state in element state, reports no open change for a controlled transition, and moves and restores focus its own way, so those rules would be split across two mechanisms | — |
| R-11 | A press outside the popover closes it and still reaches what is under it, except the footer item, which toggles (decision 18) | One press both closes and acts (K-11) |
| R-12 | Tab cycles inside the popover; Escape leaves it (decision 18) | Tab cannot reach the footer while it is open |
| R-13 | One layer at a time: the row menu, ⋯ (the Activity More menu), the dock menu, the repository menu and Cycle 03a's More menu. Opening any of them closes the popover, and opening the popover closes them (E2) | — |
| R-14 | Checking…, To share, Paused, Up to date, a busy opening and Unknown make no repository row | Their facts are elsewhere: Checking… keeps a row through rule L3, and To share has its reminders |
| R-15 | This computer and the non-SSH remote make an information row, never counted, with Set up sharing… (decision 16) | A row per local-only repository stays all session; the filter hides it |
| R-16 | Both automatic checks and Check now add brought-in rows, with different sentences (A5) | — |
| R-17 | Problem rows come from the unshared list's problems, and reminders from its items, not from the last attempt's `listed`, so Activity agrees with the count and the repository row (fact 46; decision 17) | More reminder rows than today's Share… offers, which covered Unpublished items only |
| R-18 | The layout notices and the quit row stay above the list, with today's ids and texts; F3 applies to request rows, and the quit row's "Finishing {label}" names its job | — |
| R-19 | (withdrawn in revision 4) Replaced by R-51 | — |
| R-20 | The list's container keeps the id `"operation-area"`, so D5's sweeps keep their surface (fact 40) | — |
| R-21 | Converted tests act through the seam, `choose_activity` with an `ActivityMenuChoice` (R-45), not the drawn control | A converted test no longer proves its control is drawn; P18 and P23 prove each action kind from a drawn button, the menu and the popover |
| R-22 | A request not sent is a failure row, but not counted, as today (N15). Its kinds are a key generation's passphrase needed again and a missing key store (`D/services/requests.rs:805-811`). It is not the Unlock dialog, which is S3's hold | A key generation waiting for its passphrase again, or a missing key store, is not in the popover; its row says so |
| R-23 | "Clear finished" takes only the rows L8 names | — |
| R-24 | `needs_attention` is deleted; the log's count replaces it (A6) | — |
| R-25 | Decision 23 (the owner's): see C3 | — |
| R-26 | Times show "HH:MM" local, or UTC with " UTC" in the detail when the offset is unknown | Two rows from different days can show the same time; the detail has the date (K-5) |
| R-27 | The row menu's first items are the full message, then the detail lines, all disabled (decisions 8 and 22) | A long detail makes a long menu |
| R-28 | A repository row stays through a check of its repository (L3; decision 19) | A row may say Can't reach for a few seconds after the shared copy is back |
| R-29 | Up and down stop at the ends in Activity and the popover; Home and End are bound there | — |
| R-30 | An item with no title found reads "the ticket {short}" or "the document {short}" | K-2 |
| R-31 | A failure's description is the envelope's fixed message plus "Your work is safe on this computer." | Some messages name CLI commands (K-3) |
| R-32 | S9's action table is total at `b37d2c2b`; an action it lacks reads "An operation" | — |
| R-33 | Only the highlighted row's button is a tab stop in Activity | — |
| R-34 | F1's moves re-export every item, so no path changes | — |
| R-35 | Intents outside `Progressing` get stage words from `stage_words` (F3) | Some stages read generically ("Getting ready…") |
| R-36 | Activity's list is GPUI's `uniform_list` with its own keys, not the Kit `List`, whose click presses Confirm (C2) | U-2 |
| R-37 | The footer item, hiding while focused, passes focus to the next shown control to its right: the right dock group's first control, else Settings, as 3a R-38's rule (E1) | — |
| R-38 | The popover anchors at the footer item's top right, `Anchor::BottomRight`, its bottom edge 6 px above the footer's top edge, 440 px wide (`05`: "bottom 34 px" over a 28 px footer) | — |
| R-39 | A retry clears a share's stored result and noted attempt (F2) | — |
| R-40 | Activity filters repository rows by `kind` and `cause`, never by re-reading the schedule (A2, A7). Locked is split by the state's `RowAction`: `Unlock(_)` (live) makes no row, as N5's hold covers it; `CheckNow` (stored) makes a row. `locked()` runs before `attention_cause`, so `cause` is `None` for both | A new cause needs a variant and an S4 decision |
| R-41 | The pure text helpers the rows need (`outcome_label`, `effects_text`, `scope_name`, the lifecycle outcome lines with the types and constants they use) move to the model and are re-exported from their old paths, so the model depends on no view module (layering) and view callers do not change (A1) | — |
| R-42 | (withdrawn in revision 4) Replaced by R-51 | — |
| R-43 | Refreshes are coalesced: `refresh_activity` marks the log dirty, and one refresh runs per effect cycle (A4; `MH-NFR-004`) | A row appears one effect cycle after its model changes |
| R-44 | Today's offer and recovery labels are kept verbatim; new labels follow the "…" rule (Part B) | Some labels that open a page lack "…" ("Open Sharing & keys", "Set identity", "Set up sharing") |
| R-45 | The seam calls `choose_activity` directly, after a synchronous refresh and with the row's current revision, with today's control handle as origin until Task 9; it panics unless exactly one enabled offer has the label (Testing) | — |
| R-46 | A row revision, from a log-wide counter, guards choices: a choice runs only when the row's revision still equals the one it was made against. A row takes a new revision only when its kind or offers change, or, for a request, its attempt or outcome; stage words and times never change it (A4, Part D; T11) | A choice made just as the row's kind or offers change does nothing; the person chooses again |
| R-47 | `clearable` has three conditions: not an attention row; either settled (N22, N23), or Dismiss its only enabled offer, Open… aside (it reopens a lifecycle dialog, read-only); no resend waits on it (A1, L8; T9) | Some finished rows stay after "Clear finished" (a stopped share that offers Give up, a republish a lifecycle waits on); Dismiss takes them |
| R-48 | Activity's repository detail is `repository_detail`, not Cycle 03a's tooltip, which carries the folder path and the shared copy's address (B4) | Activity's detail is shorter than the panel's tooltip; the panel keeps the full tooltip |
| R-49 | E1 decides focus after a dialog from the popover: when the footer item has hidden, `keep_focus_in_footer`, running in render, moves focus to E1's control before `focus_lost` could act (E1, E2) | Focus lands right of where the item was, not in the last region |
| R-50 | The Workspace keeps the popover's focus handle across opens (`attention_focus`). Its `focus_lost` fallback sends a lost focus that was `attention_focus` to the footer item if shown, else E1's control (E2) | U-8. If it does not hold, opening a layer from the popover first moves focus to the footer item |
| R-51 | A retirement is shown by its request while it runs and by a history event once its request was dismissed automatically; nothing derives rows from the attempt's retirement lines, which a later attempt clears (`D/model/polling/table.rs:147-151`; `D/model/polling.rs:408`) (A2, S8) | Today's "didn't finish" line for an Owed answer becomes the request's own row, "didn't finish" even with a Success envelope; its "catching up" sentence moves to that row's detail |
| R-52 | Failed requests that no other rule settled or excluded count once per group, by `Group.operation` when set, else by subject, as the group's newest member by queue order; a failure stops counting once fixed (A3: N2, N22; decision 25) | A family groups more than one intent: a later save of an item fixes an earlier failed save of it, though a different draft |
| R-53 | A failed Check now is absorbed by a covering row only when `check_failed` applied its answer, as the scheduler records (R-58), or under N19 (A3, N4) | A Check now that failed for a reason the schedule does not hold counts beside the repository's row |
| R-54 | A Busy read of a pending list, or a Failed read with an unavailable code that removes it, keeps its pending rows, counted, every offer disabled, until a Ready read, a Failed read S10 counts, or the repository leaving the list (A4, L3b) | A row may stay after its operation ended, until the next read lands |
| R-55 | A click, Enter or menu choice carries the drawn row's key and revision, from the pane's or popover's applied rows; a newer revision in the log runs nothing (A4, C2, E2; T8) | A press just as the row changes does nothing; the person presses again |
| R-56 | "Fixed" and "followed" are latched per attempt: `settled` maps a request key to the attempt (`Group.sent`) at which N22 or N23 first held, and the request is not counted for that attempt whatever later happens to the rows that settled it. The entry goes when the key's life ends or `Group.sent` changes; `absorbed` is per attempt too. The one exception: N23's follow-up ending Removed or NotSent undoes the latch (A3, A4; T14–T17) | A failure stays uncounted after what fixed it is dismissed or cleared; only a retry or resend under its ID counts it again |
| R-57 | Every request's subject is `Subject::Intent { family, root, item, target }`, with the intent's own target (A3); its operation, when known, is an extra match for N22's clause 1 | Two requests of one family about different targets never settle each other; a target the list misses lets one success settle a different thing |
| R-58 | `RequestFacts.check_applied` reads a set of request IDs that `check_failed` adds to when it applies an answer, kept by the scheduler entity with a reader, not computed from the code: `check_failed` applies nothing for an unmapped code, while a poll is in flight, or when a held repository's answer is only unreachable (A2, N4) | The set grows for the session, one ID per applied failed Check now (K-4) |
| R-59 | S8's information events (Attention, Unreachable, Retry) take the key `RetireNote { root, item }`: a later one for the same item replaces the earlier, with `at` updated, and they are clearable. Retired events keep `Retired(seq)` (A2, A4; T18) | Only the latest information about an item's retirement is shown |

## Known Limits

- **K-1.** Rows and their times last for the session. After a relaunch,
  pending operations, stored check outcomes, recovered drafts and the
  unshared list come back from their own stores, timed by the new
  session (decision 2).
- **K-2.** An item is named by its short code when no title is known
  (B1). A new item reads so until a list read gives its title.
- **K-3.** A failure's reason is the library's fixed sentence for its
  code, and some name CLI commands (fact 16).
- **K-4.** Nothing is capped or pruned within a session; "Clear finished"
  is the relief (decision 13).
- **K-5.** Where the local offset is unknown (non-Unix today), times are
  UTC and the detail says so.
- **K-6.** No names: "Brought in N changes" names no one, and a conflict
  is "you and someone else" (fact 26).
- **K-7.** "Tries again" gives no minutes, since retries back off.
- **K-8.** Tooltips show on hover only. The full message and the detail
  are also each row's accessible description and its menu's first lines.
- **K-9.** A pending conflict with no pending row loaded has no Activity
  row (R-6).
- **K-10.** A pending row with no operation ID can change its time
  (R-4).
- **K-11.** A press outside the popover also acts (R-11).
- **K-12.** A Busy share's "May be waiting for …" can change while shown
  (R-9).
- **K-13.** An unreachable repository with problems from an earlier check
  shows the problems, not Can't reach, as its panel row does (N17).

## Unverified

- **U-1.** `ActivityMenuChoice` derives the Kit `Action`. That needs
  `Clone`, `PartialEq` and `Debug` on `RowKey`, `ActivityAction` and
  their fields (`SyncOffer`, the polling `Action`, `RowAction`,
  `ItemRef`, `Target`, `TargetKind`, `Report`, and the library's
  `ItemDtoKind` in `ResumeLifecycle`). Task 3 checks each, and adds a
  derive to a desktop type that lacks one. A library type that lacks one
  stops the task for the owner.
- **U-2.** `uniform_list` (`gpui-pre-0.3.6/src/elements/uniform_list.rs:22`,
  re-exported by `gpui_kit::*`) draws and scrolls the rows with a tracked
  scroll handle, as C2 needs. Task 9 checks first with a scratch test. If
  it fails, the fallback is the Kit's `VirtualList`, with the same keys,
  recorded as an amendment.
- **U-4.** Keys bound in `ACTIVITY_CONTEXT` reach the handlers while the
  list's focus handle holds focus. Task 9 checks first.
- **U-5.** `PollSchedule::checked_now`'s in-flight branch returns a
  summary to append from (A5). Task 4 checks.
- **U-6.** Cycle 03a's final code was re-read at `81789331` (revision 5);
  Task 0 re-checks only what changes after that.
- **U-7.** An open item's model is found from `(root, kind, id)` for
  B1's title, `ItemModel::last_title()` (`TabContent::find`,
  `TabKey::Item`). Task 3 checks.
- **U-8.** When a passphrase prompt or a right-press menu opened from the
  popover closes onto `attention_focus`, which is no longer drawn, the
  Workspace's `focus_lost` (`D/view/workspace/focus.rs:275-295`) runs and
  can tell that the lost focus was `attention_focus` (R-50). Task 11
  checks with a scratch test. If it does not hold, opening such a layer
  first moves focus to the footer item.

U-3 of revision 2 (the order of a press outside and the footer item's
click) is settled: fact 47.

## Alternatives Rejected

- **A log stored on disk:** the owner chose this session only (decision
  2).
- **The Kit `Popover`.** Its controlled mode could be driven from the
  footer's Kit `Button`, but it keeps its state in element state, reports
  no open change for a controlled transition, and moves and restores
  focus its own way. The app's dock and repository menus already hold the
  rules this popover needs, which would otherwise be split across two
  mechanisms (R-10; fact 32).
- **The Kit `List`** presses Confirm on a click (C2).
- **Adding the record to `WorkerEvent::Finished`,** as ticket
  `01M6H7VXJJHRYHTW9S8TJ88921` suggested, would touch the 51 lines in 26
  files that name it, 36 of them in 21 test files (fact 13; R-8).
- **Deriving rows in render:** times would be when a frame was drawn,
  and a hidden Activity would learn nothing (R-2).
- **Converting tests straight to the new drawing** would make one task
  carry the new panel and 494 test lines at once (Testing).
- **Leaving Can't reach out of the count:** the owner chose the design's
  popover (decision 6).

## Risks

| Risk | Control |
| --- | --- |
| The test migration loses or weakens an assertion | Tasks 6 to 8 are pure moves onto the seam, green before and after; the ledger lists each file's test functions before and after, and each removed drawn-text assertion; the reviewer checks each converted assertion keeps its meaning |
| A model change reaches no refresh point, so a row is stale | P13 and P29 drive every source; each refresh point is named in A4 |
| The copy is wrong for a case | Pure copy tables (P4–P7); the owner checks rows on screen (Task 13's checklist) |
| Main moves after `81789331` and changes what this design cites | Task 0 merges main and re-reads each fact whose file changed after `81789331` (U-6) |
| The attention count's contract is still wrong after four rounds of review, and revision 5 had no independent review | It is the N-table with its precedence, latches and T-rows, written test-first under Task 5's design checkpoint; a row its implementer cannot express goes back to the orchestrator and the owner |
| The Cycle does not fit one review | 14 tasks (0–13), larger than 03a's 11. The split line: 03b-1 is Tasks 0–10 (model, copy, the footer count, the test migration, the panel, its row menu and toolbar); 03b-2 is Tasks 11–13 (popover, privacy surfaces, gates) |
| Window tests flake under load | New window tests run alone first; the ledger names any flake (Cycles 01, 02, 03a) |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Owner, 2026-11-12 | Plan 03b now, 03a being substantially complete | Owner direction | Decision 4; 03a's published head merged | Session, 2026-11-12 |
| Planning research | No row has a time; the count and the panel disagree (3a K-9); the Kit `Popover` and `List` do not fit; shares classified on render; "Finishing" hides the stage; titles barred by D5; 494 test lines reach the area by id | Facts; owner questions; rulings | Facts 1–46; decisions 4–13; R-8, R-10, R-21, R-36 | This document |
| Owner, 2026-11-12 | Resolved rows, the count, checks, extra actions, clearing, problem items, Enter, the outline, titles | Owner decisions | Decisions 4–13 | Session, 2026-11-12 |
| Review round 1, contract and consistency (revision 1, `74c9aabb`) | 3 Blocking, 24 Important, 14 Minor: titles under D6's rule (B1); paths in rows (B2); popover actions by keyboard (B3); copy rules; attention table; focus; task numbering | Blocking, Important, Minor | Revision 2: facts 15, 25, 45; B0–B5; N1, N4, N17; P23, P31, P32; Document Amendments 6 and 10–13; decisions 21–24 | Change list, 2026-11-13 |
| Review round 1, source and feasibility (revision 1, `74c9aabb`) | 1 Blocking, 15 Important, 11 Minor: how `act` reaches the handlers; inputs to the derivation; the attention cause; handler names and signatures; the Kit `Popover`; tests the inventory missed; counts and line references | Blocking, Important, Minor | Revision 2: A1, A2, A7, Testing, E-1 to E-16; facts 10, 13, 15, 18, 32, 41, 44; R-40 to R-45; 14 tasks | Change list, 2026-11-13 |
| Owner, 2026-11-13 | Titles under D6's rule; one-line rows; focus after a dialog; at most one action and counted failures | Owner decisions | Decisions 21–24 | Session, 2026-11-13 |
| Review round 2, scoped re-review and adversarial (revision 2, `b830e8e3`) | Scoped: 1 Blocking, 7 Important, 13 Minor. Adversarial: 1 Blocking (the same), 12 Important, 9 Minor. Blocking: S4's detail carried the folder path and the remote's address. Most others: the log and the attention post-pass; focus after the popover; the seam passing vacuously; inputs the derivation lacked | Blocking, Important, Minor | Revision 3: A1 (`Group`, `Derived.at`, `ActivityRow.revision`, `clearable`, the S8 key), A2 (inputs, S4, S7, S11, R-42), A3 (N4 in one phrasing, N6, N15, N18–N21; attention runs in the log), A4 (L1–L3, L6, L8, T1–T13, `refresh_activity_now`), B2–B5, C1, C2, Part D (revision), E1, E2, Part G, Testing (`act`, `act_refused`), P2, P3, P9, P10, P13, P14, P17, P19–P21, P25, P28, P32, E-3, E-6, E-9, E-12–E-14, fact 47 (U-3 settled), U-8, R-7–R-9, R-40, R-42, R-45, R-46–R-50; Document Amendment 13 and F3's key-page sentence withdrawn | Change list, 2026-11-13 |
| Owner, 2026-11-13, after review round 3 | Should a fixed failure stop counting by itself? | Owner decision | Decision 25: it stops counting once fixed and stays as history (N22 and N23) | Session, 2026-11-13 |
| Review round 3, scoped re-review and adversarial (revision 3, `4c596645`) | Scoped: 0 Blocking, 6 Important, 8 Minor. Adversarial: 6 Blocking, 7 Important, 4 Minor. Most sit in how retirements became rows (the S8 starts, kept rows, R-42) and when a failed request stops counting. Others: a lifecycle's Resume on its pending row; L3's offers; a Busy read; the revision; T10's owner; `clearable` and Open…; the sentinels | Blocking, Important, Minor | Revision 4: retirements redesigned (A1, A2 S1 and S8, B0, B4, B5; R-51); the kept rows, the fourth log rule, T7, T8, N20, R-19 and R-42 withdrawn; the subject and N2, N4, N9, N21–N24 with their precedence (R-52, R-53); S9's lifecycle Resume; L3, L3b (R-54), the revisions and the drawn revision (R-46, R-55); new T7 and T8; T5, T9–T11, T13; P1–P3, P10, P15–P19, P28; `outcome_lines`' moves; `repository_detail`; Testing (`act`, T11); Document Amendments 2 and 11. Scoped findings 1–3 (the S8 starts, kept counting, N20 against N21) are resolved by the redesign | Change list, 2026-11-13 |
| Review round 4, scoped re-review with re-grounding and adversarial (revision 4, `05246630`) | Scoped: 1 Blocking, 5 Important, 8 Minor, and 11 moved citations. Adversarial: 5 Blocking, 6 Important, 3 Minor. Almost all Blocking findings sit in decision 25's "fixed" rule: rows that fixed a failure leaving, a retry, the subject, the operation, a later check, a retirement event. Others: `check_applied`, `clearable` for fixed rows, S8's information events, S9's lifecycle row while in flight, the popover's focus and revision check, dead code, `retire_rows_hit`, `outcome_lines`' constants, L3b's trigger, stale text. Cycle 03a merged in between (main `ba95e1b7`) | Blocking, Important, Minor | Revision 5, **with no independent review**: the latch (`settled` per attempt, R-56); exact subjects with `target` (R-57); N22's five clauses; N2 by operation; N23 undone; N25; `check_applied` recorded by `check_failed` (R-58); `superseded`; `clearable` for settled failures (A1, R-47, T9, P19); S8's `RetireNote` (R-59); the Owed copy; `LifecycleRow::new`; L3b's trigger and `pending_held`; T5, T7, T14–T18; E2's focus and revision check (P23, P25); F4's dead code; E-7; `outcome_lines`' constants; R-41; Document Amendment 11. Re-grounded at `81789331`: facts 19, 21, 28, 30, 41 and 46, A2, R-51, P32, E-4, E-13, U-6 and Risks | Change list, 2026-11-13 |
| Owner, 2026-11-13 | Approval message: documents, named edits, decisions 14–20, no further review, execution method, authorization | Owner decisions | Decisions 14–20 confirmed; decision 26 | Session, 2026-11-13 |
