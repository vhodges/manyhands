//! Interruption through the binary, over SSH, and the exit evidence that
//! needs the server.
//!
//! - **Ctrl-C** through a pseudo-terminal (decision 14, R-21, R-36). Task 7's
//!   spike records what a press during a blocked network call answers.
//!   Task 13's cases press while the server holds a transfer, then let it
//!   go: for each of the five verbs whose window a press reaches, the
//!   answer, its effects, what `request show` says and what a new request
//!   then completes (design, Cancellation); a second press; a press at the
//!   unlock prompt; and a press after the last safe point, which stops C2's
//!   bounded retry (R-36).
//! - **`request cancel`** from a second invocation, for the four verbs that
//!   stop at safe points (R-35, fact 11).
//! - **Lost stdout** (R-25).
//! - **Exit evidence** (R-27) for the codes only a server gives: the
//!   transport refusals, an index hand-off refused after a publication, and
//!   a poll holding the reservation.
//!
//! The server holds the next advertisement until the case lets it go, so
//! each press or cancellation lands while the run waits on the transfer;
//! no case sleeps for the run to reach a point. The only waits are the half
//! second a press is given to land, as Task 7's spike gives it, and, after
//! the last safe point, a poll for the fixture's own mark that the push
//! command arrived. Every run goes through `scan_output`.
use super::*;
use cli_pair::{CONTEXT, CliClone, CliPair, ITEM, OTHER_ITEM, PRIMARY};
use collaboration::{
    PRIVATE_BODY, add, assert_saved, checkpoints, comment_args, comment_input, comment_path,
    connections, holds, item_sync, oid, private_body, repo_sync, request_show, resume, save,
};
use conflicts::{
    assert_effects, assert_republished, confirm, confirm_args, confirmation, no_record, prepare,
    published_then_deleted, release, sync_refused,
};
use lifecycle::Verb;
use ssh_remote::{FixtureError, fixed};
use std::io::{Read, Write};

pub const CASES: &[ssh_harness::Case] = &[
    #[cfg(unix)]
    (
        "c4_interruption_held_transfer",
        unix::c4_interruption_held_transfer,
    ),
    #[cfg(unix)]
    (
        "c4_interruption_item_sync_then_a_new_request",
        pressed::c4_interruption_item_sync_then_a_new_request,
    ),
    #[cfg(unix)]
    (
        "c4_interruption_comment_add_then_item_sync",
        pressed::c4_interruption_comment_add_then_item_sync,
    ),
    #[cfg(unix)]
    (
        "c4_interruption_conflict_resolve_then_repo_sync",
        pressed::c4_interruption_conflict_resolve_then_repo_sync,
    ),
    #[cfg(unix)]
    (
        "c4_interruption_item_republish_then_a_new_prepare",
        pressed::c4_interruption_item_republish_then_a_new_prepare,
    ),
    #[cfg(unix)]
    (
        "c4_interruption_operation_resume_then_item_sync",
        pressed::c4_interruption_operation_resume_then_item_sync,
    ),
    #[cfg(unix)]
    (
        "c4_interruption_second_press_ends_the_process",
        pressed::c4_interruption_second_press_ends_the_process,
    ),
    #[cfg(unix)]
    (
        "c4_interruption_unlock_prompt",
        unix::c4_interruption_unlock_prompt,
    ),
    (
        "c4_interruption_request_cancel_of_comment_add",
        c4_interruption_request_cancel_of_comment_add,
    ),
    (
        "c4_interruption_request_cancel_of_item_sync",
        c4_interruption_request_cancel_of_item_sync,
    ),
    (
        "c4_interruption_request_cancel_of_repo_sync",
        c4_interruption_request_cancel_of_repo_sync,
    ),
    (
        "c4_interruption_request_cancel_of_item_republish",
        c4_interruption_request_cancel_of_item_republish,
    ),
    ("c4_interruption_lost_stdout", c4_interruption_lost_stdout),
    #[cfg(unix)]
    (
        "c4_interruption_press_after_the_last_safe_point",
        pressed::c4_interruption_press_after_the_last_safe_point,
    ),
    (
        "c4_interruption_transport_refusals",
        c4_interruption_transport_refusals,
    ),
    (
        "c4_interruption_index_hand_off_refused",
        c4_interruption_index_hand_off_refused,
    ),
    (
        "c4_interruption_poll_holds_the_reservation",
        c4_interruption_poll_holds_the_reservation,
    ),
    // C5 (Task 13a).
    #[cfg(unix)]
    ("c5_interruption_press_before_integration_promote", || {
        pressed::press_before_integration(Verb::Promote)
    }),
    #[cfg(unix)]
    ("c5_interruption_press_before_integration_close", || {
        pressed::press_before_integration(Verb::Close)
    }),
    #[cfg(unix)]
    ("c5_interruption_press_after_integration_promote", || {
        pressed::press_after_integration(Verb::Promote)
    }),
    #[cfg(unix)]
    ("c5_interruption_press_after_integration_close", || {
        pressed::press_after_integration(Verb::Close)
    }),
    #[cfg(unix)]
    (
        "c5_interruption_second_press_close",
        pressed::c5_interruption_second_press_close,
    ),
    #[cfg(unix)]
    ("c5_interruption_consent_prompt_promote", || {
        pressed::consent_prompt(Verb::Promote)
    }),
    #[cfg(unix)]
    ("c5_interruption_consent_prompt_close", || {
        pressed::consent_prompt(Verb::Close)
    }),
    #[cfg(unix)]
    (
        "c5_interruption_press_at_the_consent_prompt_promote",
        || pressed::press_at_the_consent_prompt(Verb::Promote),
    ),
    #[cfg(unix)]
    ("c5_interruption_press_at_the_consent_prompt_close", || {
        pressed::press_at_the_consent_prompt(Verb::Close)
    }),
    #[cfg(unix)]
    ("c5_interruption_press_during_resume_promote", || {
        pressed::press_during_resume(Verb::Promote)
    }),
    #[cfg(unix)]
    ("c5_interruption_press_during_resume_close", || {
        pressed::press_during_resume(Verb::Close)
    }),
    #[cfg(unix)]
    (
        "c5_interruption_press_during_poll_once",
        pressed::c5_interruption_press_during_poll_once,
    ),
    #[cfg(unix)]
    (
        "c5_interruption_primary_child_fails_with_cancellation_asked_close",
        pressed::c5_interruption_primary_child_fails_with_cancellation_asked_close,
    ),
    #[cfg(target_os = "linux")]
    (
        "c5_interruption_press_at_a_safe_point_of_sync_local",
        blocked::c5_interruption_press_at_a_safe_point_of_sync_local,
    ),
    #[cfg(target_os = "linux")]
    (
        "c5_interruption_press_at_a_safe_point_of_resolve_local",
        blocked::c5_interruption_press_at_a_safe_point_of_resolve_local,
    ),
    #[cfg(target_os = "linux")]
    (
        "c5_interruption_request_cancel_of_sync_local",
        blocked::c5_interruption_request_cancel_of_sync_local,
    ),
    #[cfg(target_os = "linux")]
    (
        "c5_interruption_request_cancel_of_resolve_local",
        blocked::c5_interruption_request_cancel_of_resolve_local,
    ),
    #[cfg(target_os = "linux")]
    (
        "c5_interruption_press_at_the_result_of_poll_once",
        blocked::c5_interruption_press_at_the_result_of_poll_once,
    ),
    #[cfg(target_os = "linux")]
    (
        "c5_interruption_press_at_the_result_of_sync_local",
        blocked::c5_interruption_press_at_the_result_of_sync_local,
    ),
    #[cfg(target_os = "linux")]
    (
        "c5_interruption_press_at_the_result_of_resolve_local",
        blocked::c5_interruption_press_at_the_result_of_resolve_local,
    ),
    ("c5_interruption_request_cancel_promote", || {
        c5::request_cancel(Verb::Promote)
    }),
    ("c5_interruption_request_cancel_close", || {
        c5::request_cancel(Verb::Close)
    }),
    (
        "c5_interruption_request_cancel_of_poll_once",
        c5::c5_interruption_request_cancel_of_poll_once,
    ),
    (
        "c5_interruption_killed_poll_once_resent",
        c5::c5_interruption_killed_poll_once_resent,
    ),
    ("c5_interruption_lost_stdout_promote", || {
        c5::lost_stdout(Verb::Promote)
    }),
    ("c5_interruption_lost_stdout_close", || {
        c5::lost_stdout(Verb::Close)
    }),
    (
        "c5_interruption_lost_stdout_of_poll_once",
        c5::c5_interruption_lost_stdout_of_poll_once,
    ),
    (
        "c5_interruption_lost_stdout_of_sync_local",
        c5::c5_interruption_lost_stdout_of_sync_local,
    ),
    (
        "c5_interruption_lost_stdout_of_resolve_local",
        c5::c5_interruption_lost_stdout_of_resolve_local,
    ),
    (
        "c5_interruption_no_resident_worker",
        c5::c5_interruption_no_resident_worker,
    ),
];

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR1";
const R2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR2";
const R3: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR3";
const R4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR4";
const R5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR5";
const R6: &str = "01ARZ3NDEKTSV4RRFFQ69G5FR6";
const C1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC1";
const C2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC2";

fn sync_args<'a>(clone: &'a CliClone, request: &'a str) -> Vec<&'a str> {
    vec![
        "item",
        "sync",
        "--repo",
        clone.root(),
        "--id",
        ITEM,
        "--request-id",
        request,
    ]
}

/// Pins the world's host as `host approve` would. The world's key is
/// protected, so this is done through the library, which can unlock it.
fn approve(w: &remote_world::World) -> Result<(), FixtureError> {
    use repository::transport::{
        HostApproval, SshAuthority, SshDirection, VerifySshTransportRequest,
    };
    fixed(w.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: w.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(HostApproval {
                authority: SshAuthority {
                    host: "127.0.0.1".into(),
                    port: w.server.address().port(),
                },
                expected: None,
                presented: w.server.host_identity(),
            }),
        },
        &mut repository::keys::SessionCredentials::new(remote_world::Provider),
    ))?;
    Ok(())
}

/// A divergence of the item's branch that merges cleanly: A's first
/// comment, published, and B's save of the ticket, which is not. Returns
/// B's local tip and A's published one.
fn diverged(p: &CliPair) -> Result<(git2::Oid, git2::Oid), FixtureError> {
    add(&p.a, R1, C1, None, "first root", "ok", 0);
    let incoming = p.served(CONTEXT)?.ok_or(FixtureError)?;
    save(&p.b, R2, "Saved in clone B.\n");
    let local = p.b.tip(CONTEXT)?.ok_or(FixtureError)?;
    Ok((local, incoming))
}

/// Asserts that `merge`, the clone's tip, is one merge commit of `local`
/// and `incoming`, in that order.
fn assert_merge(clone: &CliClone, merge: git2::Oid, local: git2::Oid, incoming: git2::Oid) {
    let repo = clone.repo().unwrap();
    let commit = repo.find_commit(merge).unwrap();
    assert_eq!(commit.parent_ids().collect::<Vec<_>>(), [local, incoming]);
}

/// A JSON-mode run the case lets go on while it does other things: its
/// input written and stdin closed.
fn started(env: &CliEnv, args: &[&str], input: &[u8]) -> Started {
    let mut run = Started::spawn(env, args);
    run.child().stdin.take().unwrap().write_all(input).unwrap();
    run
}

/// A run in the background, reaped whatever happens: unless it is
/// collected, dropping it kills it and waits for it.
struct Started(Option<std::process::Child>);

impl Started {
    /// `args` in JSON mode, with stdin piped, held from the moment it is
    /// spawned.
    fn spawn(env: &CliEnv, args: &[&str]) -> Self {
        let mut args = args.to_vec();
        args.push("--json");
        Self(Some(
            cli::cli_command(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        ))
    }

    fn child(&mut self) -> &mut std::process::Child {
        self.0.as_mut().unwrap()
    }

    /// Waits for the run, and returns its output.
    fn collected(mut self) -> CliRun {
        cli::collect_cli(self.0.take().unwrap())
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// The request cancellation's data.
fn cancel_data(request: &str, state: &str, requested: bool, acknowledged: bool) -> Value {
    serde_json::json!({
        "request_id": request,
        "state": state,
        "requested": requested,
        "acknowledged": acknowledged,
    })
}

/// `args`, in JSON mode, run while the server holds its fetch. While it
/// waits, a second invocation, with an environment of its own over the
/// same data, asks `request cancel` of `request`: the request is asked, and
/// has not stopped. The fetch is let go, and the run stops at its next
/// safe point: `cancelled`, exit 130, with nothing on stderr. Asked again,
/// the request has stopped for the cancellation. Returns the run's
/// envelope.
fn cancelled_while_held(
    p: &CliPair,
    clone: &CliClone,
    args: &[&str],
    input: &[u8],
    request: &str,
) -> Result<Value, FixtureError> {
    let hold = p.server.hold_advertisement()?;
    let child = started(&clone.env, args, input);
    hold.wait_until_held()?;
    let second = CliEnv::new(clone.env.data.clone());
    let cancel = ["request", "cancel", "--request-id", request];
    let asked = checked(&second, &cancel, &[], "ok", 0, Some("cancel.schema.json"));
    assert_eq!(asked["data"], cancel_data(request, "accepted", true, false));
    hold.release()?;
    let out = child.collected();
    scan_output(&out);
    assert_eq!(out.status, 130);
    assert!(out.stderr.is_empty(), "JSON mode writes nothing on stderr");
    let value = cli::json(&out);
    schema::check_published("envelope.schema.json", &value).unwrap();
    assert_eq!(value["outcome"], "cancelled");
    assert_eq!(value["code"], "cancelled");
    assert_eq!(value["request_id"], request);
    assert!(value["recovery"].as_array().unwrap().is_empty());
    let after = checked(&second, &cancel, &[], "ok", 0, Some("cancel.schema.json"));
    assert_eq!(after["data"], cancel_data(request, "finished", false, true));
    assert_eq!(request_show(clone, request)["state"], "finished");
    Ok(value)
}

/// `comment add`, cancelled by `request cancel` while its publication
/// fetches: its comment is checkpointed, and nothing is published. A new
/// `item sync` publishes it.
fn c4_interruption_request_cancel_of_comment_add() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    private_body();
    let pushes = remote_world::accepted(&p.server);
    let stopped = cancelled_while_held(
        &p,
        a,
        &comment_args(a, ITEM, R1),
        &comment_input(C1, None, PRIVATE_BODY),
        R1,
    )?;
    let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_saved(&stopped, "pending");
    assert_eq!(oid(&stopped["effects"]["commit_oid"]), checkpoint);
    assert!(holds(&a.repo()?, checkpoint, &comment_path(C1))?);
    assert_eq!(p.served(CONTEXT)?, None);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    let published = item_sync(a, ITEM, R2, "ok", 0);
    assert_effects(
        &published,
        &[("discovery", "current"), ("publication", "published")],
    );
    assert_eq!(oid(&published["effects"]["commit_oid"]), checkpoint);
    assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    Ok(())
}

/// `item sync`, cancelled by `request cancel` while it fetches: nothing is
/// merged or pushed. A new request merges and publishes.
fn c4_interruption_request_cancel_of_item_sync() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let b = &p.b;
    let (local, incoming) = diverged(&p)?;
    let pushes = remote_world::accepted(&p.server);
    let stopped = cancelled_while_held(&p, b, &sync_args(b, R3), &[], R3)?;
    assert_effects(&stopped, &[("publication", "pending")]);
    assert!(stopped["effects"]["commit_oid"].is_null());
    assert_eq!(b.tip(CONTEXT)?, Some(local));
    assert_eq!(p.served(CONTEXT)?, Some(incoming));
    assert_eq!(remote_world::accepted(&p.server), pushes);
    let merged = item_sync(b, ITEM, R4, "ok", 0);
    let merge = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_eq!(merged["effects"]["integration"], "complete");
    assert_eq!(oid(&merged["effects"]["commit_oid"]), merge);
    assert_merge(b, merge, local, incoming);
    assert_eq!(p.served(CONTEXT)?, Some(merge));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    Ok(())
}

/// `repo sync`, cancelled by `request cancel` while it fetches: A's commit
/// on primary is not pushed. A new request publishes it.
fn c4_interruption_request_cancel_of_repo_sync() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let served = p.served(PRIMARY)?;
    let ahead = remote_world::advance(&a.repo()?, PRIMARY, "clone-a-file")?;
    let pushes = remote_world::accepted(&p.server);
    let stopped = cancelled_while_held(
        &p,
        a,
        &["repo", "sync", "--repo", a.root(), "--request-id", R1],
        &[],
        R1,
    )?;
    assert_effects(&stopped, &[("publication", "pending")]);
    assert_eq!(a.tip(PRIMARY)?, Some(ahead));
    assert_eq!(p.served(PRIMARY)?, served);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    let published = repo_sync(a, R2, "ok", 0);
    assert_eq!(published["effects"]["publication"], "published");
    assert_eq!(oid(&published["effects"]["commit_oid"]), ahead);
    assert_eq!(p.served(PRIMARY)?, Some(ahead));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    Ok(())
}

/// `item republish`, cancelled by `request cancel` while it fetches: the
/// branch is not recreated, and the confirmation is spent. A new prepare,
/// confirmed, republishes it.
fn c4_interruption_request_cancel_of_item_republish() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    let tip = published_then_deleted(&p)?;
    sync_refused(&p)?;
    release(a);
    let primary = a.tip(PRIMARY)?;
    let preview = prepare(a);
    let updates = p.server.receive_updates().len();
    let stopped = cancelled_while_held(
        &p,
        a,
        &confirm_args(a, ITEM, R3, confirmation(&preview)),
        &[],
        R3,
    )?;
    assert_effects(&stopped, &[("publication", "pending")]);
    assert_eq!(p.server.receive_updates().len(), updates);
    assert_eq!(p.served(CONTEXT)?, None);
    // The operation is final and its confirmation spent: a new request with
    // it is refused, and so is a new preview's confirmation for another
    // item. Neither keeps a record or reaches the server.
    let opened = connections(&p);
    let used = confirm(a, R4, &preview, "confirmation_used", 3);
    assert_effects(&used, &[]);
    no_record(a, R4);
    let again = prepare(a);
    let mismatched = a.json(
        &confirm_args(a, OTHER_ITEM, R5, confirmation(&again)),
        &[],
        "confirmation_mismatch",
        2,
    );
    assert_effects(&mismatched, &[]);
    no_record(a, R5);
    assert_eq!(connections(&p), opened);
    // The new preview, confirmed, republishes.
    let republished = confirm(a, R6, &again, "ok", 0);
    assert_republished(&p, &republished, tip, updates, primary)
}

/// R-27's exit evidence for the transport codes only a server gives, through
/// `repo sync` on a world whose key is protected. One request meets each in
/// turn and is sent again once its recovery is done: a host not yet approved
/// (`host_approval_required`, offering `host.approve`); a protected key with
/// no terminal to unlock it (`unlock_required`); a server that lets the
/// client in without its selected key (`key_rejected`); and a host that
/// presents another key than the one approved (`host_replacement_required`,
/// offering `host.replace`). Each is blocked, exit 3, with the request's own
/// retry offered and its publication pending, and nothing reaches the
/// server's branches. Meanwhile the request holds its reservation, so a new
/// one is `busy` (Known Limits, `01M4R19PC0T6ZPKM39X9DF26HX`).
fn c4_interruption_transport_refusals() -> Result<(), FixtureError> {
    let w = remote_world::World::new()?;
    private_fixture(&w.server);
    let env = CliEnv::new(w.data());
    let root = w.root.to_str().unwrap().to_owned();
    let updates = w.server.receive_updates().len();
    let sync = |request: &str, code: &str, status: i32| {
        checked(
            &env,
            &["repo", "sync", "--repo", &root, "--request-id", request],
            &[],
            code,
            status,
            None,
        )
    };
    let retry = serde_json::json!({"action": "request.retry", "operation_id": null, "arguments": {"request_id": R1}});
    let refused = |code: &str, first: Option<&str>| {
        let value = sync(R1, code, 3);
        assert_eq!(value["outcome"], "blocked");
        assert_effects(&value, &[("publication", "pending")]);
        let recovery = value["recovery"].as_array().unwrap();
        let actions: Vec<_> = recovery.iter().map(|action| &action["action"]).collect();
        match first {
            Some(first) => assert_eq!(actions, [first, "request.retry"]),
            None => assert_eq!(actions, ["request.retry"]),
        }
        assert_eq!(*recovery.last().unwrap(), retry);
    };

    refused("host_approval_required", Some("host.approve"));
    approve(&w)?;
    refused("unlock_required", None);
    let busy = sync(R2, "busy", 5);
    assert_eq!(
        busy["recovery"],
        serde_json::json!([{"action": "request.retry", "operation_id": null, "arguments": {"request_id": R2}}])
    );
    checked(
        &env,
        &["request", "show", "--request-id", R2],
        &[],
        "request_not_found",
        2,
        None,
    );
    w.server.accept_anonymous();
    refused("key_rejected", None);
    w.server.rotate_host_key()?;
    refused("host_replacement_required", Some("host.replace"));
    let record = checked(
        &env,
        &["request", "show", "--request-id", R1],
        &[],
        "ok",
        0,
        Some("request.schema.json"),
    );
    assert_eq!(record["data"]["state"], "accepted");
    assert_eq!(w.server.receive_updates().len(), updates);
    Ok(())
}

/// R-27's exit evidence for `poll_yielding`, staged as Part A stages it
/// (`tests/mutation_remote.rs`,
/// `a_poll_holding_the_reservation_is_poll_yielding_and_leaves_no_record`).
/// A's first `repo sync` makes the repository's remote known to the journal
/// a poll reserves in. A poll then holds the reservation, taken through the
/// public service as the desktop's observer takes it. `repo sync` through
/// the binary yields to it: `poll_yielding`, exit 5, with nothing done, its
/// own retry offered, no request record, and no connection to the server.
fn c4_interruption_poll_holds_the_reservation() -> Result<(), FixtureError> {
    use repository::{
        OperationId, RemoteOperationPriority, RemoteOperationTarget, RemoteRefPlan,
        RemoteReservationOutcome, RepositoryService,
    };
    let p = CliPair::new()?;
    let a = &p.a;
    repo_sync(a, R1, "already_applied", 0);
    let service = fixed(RepositoryService::open_at(&a.env.data))?;
    let plan = fixed(RemoteRefPlan::from_configuration("origin", "main"))?;
    let poll = fixed(service.reserve_remote_operation_with_priority(
        &a.root,
        OperationId::new(),
        &RemoteOperationTarget::for_poll(&plan),
        RemoteOperationPriority::Poll,
    ))?;
    assert!(matches!(poll, RemoteReservationOutcome::Reserved(_)));
    let (keys, sockets) = (connections(&p), p.server.accepted_sockets());

    let yielded = repo_sync(a, R2, "poll_yielding", 5);
    assert_eq!(yielded["outcome"], "error");
    assert_eq!(yielded["request_id"], R2);
    assert_effects(&yielded, &[]);
    assert!(yielded["effects"]["commit_oid"].is_null());
    assert_eq!(
        yielded["recovery"],
        serde_json::json!([{"action": "request.retry", "operation_id": null, "arguments": {"request_id": R2}}])
    );
    checked(
        &a.env,
        &["request", "show", "--request-id", R2],
        &[],
        "request_not_found",
        2,
        None,
    );
    assert_eq!(connections(&p), keys);
    assert_eq!(p.server.accepted_sockets(), sockets);
    drop((poll, service));
    Ok(())
}

/// The registry's refusal of an index hand-off, as the library's tables
/// stage it (`tests/mutation_remote/resume.rs`, `c4_resume_comment_hand_off`).
const FAIL_HAND_OFF: &str = "CREATE TRIGGER fail_handoff BEFORE UPDATE ON operation_records WHEN NEW.state='completed' AND NEW.action='refresh' BEGIN SELECT RAISE(ABORT,'fixed failure'); END";

/// R-27's exit evidence for `discovery_pending`, which follows a
/// publication. A's second comment is published while the registry refuses
/// its index hand-off: `comment add` answers `discovery_pending`, partial,
/// exit 4, written, committed and published, discovery pending, offering
/// `operation resume` of the comment's operation. While the refusal stands,
/// the resume says so again. Once the registry takes it, the resume does the
/// hand-off with no connection and no push, and the comment's own request
/// is finished.
fn c4_interruption_index_hand_off_refused() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let a = &p.a;
    private_body();
    add(a, R1, C1, None, PRIVATE_BODY, "ok", 0);
    let registry = a.env.data.join(repository::REGISTRY_FILE);
    fixed(fixed(rusqlite::Connection::open(&registry))?.execute_batch(FAIL_HAND_OFF))?;
    let pushes = remote_world::accepted(&p.server);
    let owed = add(a, R2, C2, None, PRIVATE_BODY, "discovery_pending", 4);
    let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_eq!(owed["outcome"], "partial");
    assert_effects(
        &owed,
        &[
            ("write", "written"),
            ("checkpoint", "committed"),
            ("discovery", "pending"),
            ("publication", "published"),
        ],
    );
    assert_eq!(oid(&owed["effects"]["commit_oid"]), checkpoint);
    assert_eq!(owed["data"]["publication"], "published");
    let comment = owed["operation_id"].as_str().unwrap().to_owned();
    let recovery = owed["recovery"].as_array().unwrap();
    assert_eq!(recovery.len(), 1, "{owed}");
    assert_eq!(recovery[0]["action"], "operation.resume");
    assert_eq!(recovery[0]["operation_id"], comment.as_str());
    assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);

    let still = resume(a, &comment, R3, "discovery_pending", 4);
    assert_eq!(still["recovery"][0]["action"], "operation.resume");
    fixed(
        fixed(rusqlite::Connection::open(&registry))?.execute_batch("DROP TRIGGER fail_handoff"),
    )?;
    let opened = connections(&p);
    let resumed = resume(a, &comment, R4, "ok", 0);
    assert_effects(
        &resumed,
        &[("discovery", "current"), ("publication", "published")],
    );
    assert_eq!(oid(&resumed["effects"]["commit_oid"]), checkpoint);
    assert!(resumed["recovery"].as_array().unwrap().is_empty());
    assert_eq!(connections(&p), opened);
    let again = add(a, R2, C2, None, PRIVATE_BODY, "ok", 0);
    assert_eq!(again["data"]["publication"], "published");
    assert_eq!(remote_world::accepted(&p.server), pushes + 1);
    assert_eq!(
        checkpoints(&a.repo()?, checkpoint, &comment_path(C2))?,
        [checkpoint]
    );
    Ok(())
}

/// A JSON-mode run whose stdout is closed before it writes anything. As
/// `collect_cli` does, it waits up to 30 s for the run, then up to one second
/// for stderr to reach its end.
fn with_stdout_closed(env: &CliEnv, args: &[&str], input: &[u8]) -> CliRun {
    let mut run = Started::spawn(env, args);
    let child = run.child();
    drop(child.stdout.take());
    child.stdin.take().unwrap().write_all(input).unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = sender.send(stderr.read_to_end(&mut bytes).map(|_| bytes));
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        // Dropping `run` kills the child and waits for it.
        assert!(
            std::time::Instant::now() < deadline,
            "CLI exceeded 30 seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    let stderr = receiver
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("stderr did not reach EOF within one second of exit")
        .unwrap();
    reader.join().unwrap();
    let out = CliRun {
        stdout: Vec::new(),
        stderr,
        status: status.code().expect("CLI terminated without an exit code"),
    };
    scan_output(&out);
    out
}

/// R-25: `comment add` and `item sync`, each with its stdout closed, exit 1
/// with nothing on stderr, and their work is done. `request show` returns
/// the stored result, and the request sent again replays it: no second
/// connection, push, comment file or checkpoint.
fn c4_interruption_lost_stdout() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    let (a, b) = (&p.a, &p.b);
    private_body();

    // A's first comment publishes its context, and its result is lost.
    let lost = with_stdout_closed(
        &a.env,
        &comment_args(a, ITEM, R1),
        &comment_input(C1, None, PRIVATE_BODY),
    );
    assert_eq!(lost.status, 1);
    assert!(lost.stderr.is_empty(), "no second diagnostic");
    let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
    let pushes = remote_world::accepted(&p.server);
    let opened = connections(&p);
    let stored = request_show(a, R1);
    assert_eq!(stored["state"], "finished");
    let result = &stored["result"];
    assert_eq!(result["code"], "ok");
    assert_saved(result, "published");
    assert_eq!(oid(&result["effects"]["commit_oid"]), checkpoint);
    let replay = add(a, R1, C1, None, PRIVATE_BODY, "ok", 0);
    for key in ["outcome", "code", "effects", "data"] {
        assert_eq!(replay[key], result[key], "{key}");
    }
    assert_eq!(connections(&p), opened);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
    let comments = fixed(std::fs::read_dir(
        a.context().join(format!(".manyhands/comments/{ITEM}")),
    ))?
    .count();
    assert_eq!(comments, 1);
    assert_eq!(
        checkpoints(&a.repo()?, checkpoint, &comment_path(C1))?,
        [checkpoint]
    );

    // B's item sync merges and publishes, and its result is lost.
    save(b, R2, "Saved in clone B.\n");
    let local = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    let lost = with_stdout_closed(&b.env, &sync_args(b, R3), &[]);
    assert_eq!(lost.status, 1);
    assert!(lost.stderr.is_empty(), "no second diagnostic");
    let merge = b.tip(CONTEXT)?.ok_or(FixtureError)?;
    assert_merge(b, merge, local, checkpoint);
    assert_eq!(p.served(CONTEXT)?, Some(merge));
    let pushes = remote_world::accepted(&p.server);
    let opened = connections(&p);
    let stored = request_show(b, R3);
    assert_eq!(stored["state"], "finished");
    let result = &stored["result"];
    assert_eq!(result["code"], "ok");
    assert_eq!(result["effects"]["publication"], "published");
    assert_eq!(result["effects"]["integration"], "complete");
    assert_eq!(oid(&result["effects"]["commit_oid"]), merge);
    let replay = item_sync(b, ITEM, R3, "ok", 0);
    for key in ["outcome", "code", "effects", "data"] {
        assert_eq!(replay[key], result[key], "{key}");
    }
    assert_eq!(connections(&p), opened);
    assert_eq!(remote_world::accepted(&p.server), pushes);
    assert_eq!(b.tip(CONTEXT)?, Some(merge));
    Ok(())
}

#[cfg(unix)]
mod unix {
    use super::*;
    use remote_world::{CONTEXT, ITEM, PASSWORD, World};
    use repository::keys::*;
    use ssh_remote::{FixtureError, fixed};
    use support::pty::PtyCli;

    /// Bytes the fixture forwards before it holds the fetch's transfer.
    const HELD_AFTER: usize = 131_072;

    /// Selects an unprotected copy of the world's key, so that a
    /// non-interactive call needs no unlock. The server accepts it: it is
    /// the same key.
    fn select_an_unprotected_key(w: &World) -> Result<(), FixtureError> {
        let key = fixed(
            fixed(ssh_key::PrivateKey::read_openssh_file(
                w.server.client_key_path(),
            ))?
            .decrypt(PASSWORD),
        )?;
        let text = fixed(key.to_openssh(ssh_key::LineEnding::LF))?;
        OUTPUT_PROBES.with(|probes| {
            probes.borrow_mut().extend(
                text.lines()
                    .filter(|line| line.len() > 40)
                    .map(str::to_owned),
            );
        });
        let path = w.directory.path().join("unprotected-key");
        fixed(std::fs::write(&path, text.as_bytes()))?;
        use std::os::unix::fs::PermissionsExt;
        fixed(std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o600),
        ))?;
        let RegisterSharedKeyOutcome::Registered(key) =
            fixed(w.service.register_shared_key(RegisterSharedKeyRequest {
                label: "unprotected".into(),
                ownership: SharedKeyOwnership::Imported,
                private_key_path: path,
                public_key_path: None,
            }))?
        else {
            return Err(FixtureError);
        };
        fixed(w.service.select_shared_key(key.id))?;
        Ok(())
    }

    /// A peer moves the remote primary on with a commit large enough that
    /// a fetch of it is still transferring when the fixture holds it.
    fn incoming(w: &World) -> Result<git2::Oid, FixtureError> {
        let mut state = 41u64;
        let payload = (0..1_600_000)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect::<Vec<_>>();
        let base = fixed(w.peer.refname_to_id("refs/heads/main"))?;
        let oid = remote_world::commit(&w.peer, base, "payload", &payload)?;
        fixed(w.peer.reference("refs/heads/main", oid, true, "incoming"))?;
        remote_world::push_peer(&w.peer, &w.server, "refs/heads/main")?;
        Ok(oid)
    }

    fn index_of<T: Copy + PartialEq>(all: &[T], value: T) -> u128 {
        all.iter().position(|known| *known == value).unwrap() as u128
    }

    /// Decision 14's spike: `item sync` is pressed with Ctrl-C while its
    /// fetch is held. Decision 14 accepts `cancelled`, exit 130, at the next
    /// safe point, or `transport_unavailable`, exit 5, if a blocking call
    /// returned `EINTR`, either with its effects intact. The spike recorded
    /// `cancelled` (Task 7's ledger entry), which this case pins. It also
    /// records the exit, the code and the effects as an observation.
    pub fn c4_interruption_held_transfer() -> Result<(), FixtureError> {
        let mut w = World::new()?;
        private_fixture(&w.server);
        let env = CliEnv::new(w.data());
        approve(&w)?;
        select_an_unprotected_key(&w)?;
        w.context()?;
        let repo = w.repo()?;
        let context = fixed(repo.refname_to_id(CONTEXT))?;
        let primary = fixed(repo.refname_to_id("refs/heads/main"))?;
        let incoming = incoming(&w)?;
        let pushes = w.server.receive_updates().len();

        let hold = w.server.hold_transfer_after(HELD_AFTER)?;
        let request = repository::RequestId::new().to_string();
        let root = w.root.to_str().unwrap().to_owned();
        let mut pty = PtyCli::spawn(
            &[
                "item",
                "sync",
                "--repo",
                &root,
                "--id",
                ITEM,
                "--non-interactive",
                "--request-id",
                &request,
            ]
            .iter()
            .map(OsStr::new)
            .collect::<Vec<_>>(),
            &env,
            &[],
        );
        hold.wait_until_held()?;
        assert!(pty.first_stderr_line().starts_with("progress: "));
        pty.send(&[3]);
        // The press is delivered while the transfer is still held.
        std::thread::sleep(std::time::Duration::from_millis(500));
        hold.release()?;
        let (out, _) = pty.finish();
        scan_output(&out);
        assert!(out.stdout.is_empty());
        let stderr = String::from_utf8(out.stderr.clone()).unwrap();
        assert!(
            !stderr.contains("progress: retrying"),
            "no retry follows a press"
        );

        // The stored result and what the remote and the clone hold.
        let stored = checked(
            &env,
            &["request", "show", "--request-id", &request],
            &[],
            "ok",
            0,
            Some("request.schema.json"),
        );
        let result = &stored["data"]["result"];
        let code = result["code"].as_str().unwrap().to_owned();
        assert!(stderr.contains(&format!("code: {code}\n")), "{stderr}");
        let bare = w.bare()?;
        let remote_primary = fixed(bare.refname_to_id("refs/heads/main"))?;
        let remote_context = bare.refname_to_id(CONTEXT).is_ok();
        let repo = w.repo()?;
        let local_context = fixed(repo.refname_to_id(CONTEXT))?;
        let local_primary = fixed(repo.refname_to_id("refs/heads/main"))?;
        let effect = |key: &str| result["effects"][key].as_str().unwrap().to_owned();
        ssh_harness::observation(&[
            out.status as u128,
            index_of(
                &manyhands::results::ResultCode::ALL.map(|code| code.as_str()),
                code.as_str(),
            ),
            index_of(
                &manyhands::results::PublicationEffect::ALL.map(|effect| effect.as_str()),
                effect("publication").as_str(),
            ),
            index_of(
                &manyhands::results::IntegrationEffect::ALL.map(|effect| effect.as_str()),
                effect("integration").as_str(),
            ),
            u128::from(remote_context),
            u128::from(local_context == context),
            u128::from(local_primary == primary),
            u128::from(remote_primary == incoming),
            (w.server.receive_updates().len() - pushes) as u128,
        ]);

        // The press cancelled the request at the next safe point: the
        // operation is final and its result is stored.
        assert_eq!((out.status, code.as_str()), (130, "cancelled"));
        assert_eq!(result["outcome"], "cancelled");
        assert!(stderr.contains("progress: stopping\n"), "{stderr}");
        assert_eq!(stored["data"]["state"], "finished");
        // Its effects are intact: nothing was pushed, merged or moved
        // locally, and the effects say exactly that.
        assert_eq!(w.server.receive_updates().len(), pushes, "nothing pushed");
        assert!(!remote_context, "the context was not published");
        assert_eq!(remote_primary, incoming);
        assert_eq!(local_context, context, "nothing merged into the context");
        assert_eq!(local_primary, primary);
        for (key, value) in [
            ("write", "not_requested"),
            ("checkpoint", "not_requested"),
            ("discovery", "not_requested"),
            ("publication", "pending"),
            ("integration", "not_requested"),
            ("cleanup", "not_requested"),
        ] {
            assert_eq!(effect(key), value, "{key}");
        }
        assert!(result["effects"]["commit_oid"].is_null());
        assert!(
            stderr.contains("effects: publication=pending\n"),
            "{stderr}"
        );
        Ok(())
    }

    /// A press at the unlock prompt of a protected key (design,
    /// Cancellation, Prompts; C2's amendment). `item sync` runs inside the
    /// Ctrl-C window, so the handler is installed while the prompt reads in
    /// raw mode, which clears `ISIG`: the byte is the prompt's own
    /// cancellation, not a press the handler sees. It answers
    /// `unlock_required`, exit 3, not `cancelled`; the process ends by
    /// itself; and the request stays accepted, for its retry. The same
    /// request, sent again with the right passphrase to a server that
    /// refuses the key, is `unlock_failed` under the same operation, and
    /// still accepted. Nothing reaches the server.
    pub fn c4_interruption_unlock_prompt() -> Result<(), FixtureError> {
        let mut w = World::new()?;
        private_fixture(&w.server);
        let env = CliEnv::new(w.data());
        approve(&w)?;
        w.context()?;
        let pushes = w.server.receive_updates().len();
        let request = repository::RequestId::new().to_string();
        let root = w.root.to_str().unwrap().to_owned();
        let args = [
            "item",
            "sync",
            "--repo",
            root.as_str(),
            "--id",
            ITEM,
            "--request-id",
            request.as_str(),
        ];
        // One run at the terminal, answering the passphrase prompt with
        // `answer`: its output and the terminal's modes before and after.
        let prompted = |answer: &[u8]| {
            let mut pty =
                PtyCli::spawn(&args.iter().map(OsStr::new).collect::<Vec<_>>(), &env, &[]);
            pty.expect("Passphrase for key");
            pty.send(answer);
            let modes = pty.lflag_before_and_after();
            let (out, transcript) = pty.finish();
            scan_output(&out);
            for bytes in [
                out.stdout.as_slice(),
                out.stderr.as_slice(),
                transcript.as_bytes(),
            ] {
                absent(bytes, PASSWORD);
            }
            assert_eq!(transcript.matches("Passphrase for key").count(), 1);
            (out, modes)
        };
        let shown = || {
            checked(
                &env,
                &["request", "show", "--request-id", &request],
                &[],
                "ok",
                0,
                Some("request.schema.json"),
            )["data"]
                .clone()
        };

        let (out, (before, after)) = prompted(&[3]);
        assert_eq!(before, after, "the terminal's modes are restored");
        assert_eq!(out.status, 3);
        assert!(out.stdout.is_empty());
        let record = shown();
        let operation = record["operations"][0]["operation_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "{}error: The key must be unlocked.\ncode: unlock_required\n\
                 request: {request}\noperation: {operation}\n\
                 effects: publication=pending\nnext: request.retry request_id={request}\n",
                collaboration::progress(&["accepted", "connecting", "fetching"])
            )
        );
        // Not a cancellation: the request stays accepted, for its retry,
        // and nothing reached the server.
        assert_eq!(record["state"], "accepted");
        assert!(record["result"].is_null());
        assert_eq!(w.server.receive_updates().len(), pushes);
        assert!(w.bare()?.refname_to_id(CONTEXT).is_err());

        // The same request, the right passphrase, a server refusing the key.
        // It was accepted already, so its progress starts at connecting.
        w.server.reject_client();
        let (out, _) = prompted(format!("{PASSWORD}\n").as_bytes());
        assert_eq!(out.status, 3);
        assert!(out.stdout.is_empty());
        assert_eq!(
            std::str::from_utf8(&out.stderr).unwrap(),
            format!(
                "{}error: The key could not be unlocked.\ncode: unlock_failed\n\
                 request: {request}\noperation: {operation}\n\
                 effects: publication=pending\nnext: request.retry request_id={request}\n",
                collaboration::progress(&["connecting", "fetching"])
            )
        );
        let record = shown();
        assert_eq!(record["state"], "accepted");
        assert_eq!(record["operations"][0]["operation_id"], operation.as_str());
        assert_eq!(w.server.receive_updates().len(), pushes);
        assert!(w.bare()?.refname_to_id(CONTEXT).is_err());
        Ok(())
    }
}

/// Ctrl-C at a pseudo-terminal, on `CliPair`: a press while the server
/// holds a transfer, for each verb the window covers, then what completes
/// the work (design, Cancellation); and a second press.
#[cfg(unix)]
mod pressed {
    use super::*;
    use collaboration::{PUBLICATION_NOTE, progress, resume_args, text};
    use conflicts::{
        observation, private_source, resolutions, resolve_args, resolve_input, show,
        stopped_on_primary, tickets_diverge,
    };
    use ssh_remote::{AdvertisementHold, FixtureBoundary};
    use support::pty::PtyCli;

    /// How long a press is given to land, as Task 7's spike gives it, while
    /// the transfer is still held.
    const LANDING: std::time::Duration = std::time::Duration::from_millis(500);

    /// What a cancelled request prints after its progress.
    const CANCELLED: &str = "error: The request was cancelled.\ncode: cancelled\n";

    /// `request show` of `request`'s single operation.
    fn the_operation(clone: &CliClone, request: &str) -> String {
        let shown = request_show(clone, request);
        let operations = shown["operations"].as_array().unwrap();
        assert_eq!(operations.len(), 1, "{shown}");
        operations[0]["operation_id"].as_str().unwrap().to_owned()
    }

    /// `args` in human mode at a terminal, with `--non-interactive`, and
    /// `input` on stdin.
    fn at_a_terminal(env: &CliEnv, args: &[&str], input: &[u8]) -> PtyCli {
        let mut args = args.to_vec();
        args.push("--non-interactive");
        PtyCli::spawn(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env, input)
    }

    /// Ctrl-C once the server holds the transfer, then the transfer let go.
    fn pressed_while_held(pty: &mut PtyCli, hold: AdvertisementHold) -> Result<(), FixtureError> {
        hold.wait_until_held()?;
        pty.send(&[3]);
        std::thread::sleep(LANDING);
        hold.release()
    }

    /// The pressed run's output, scanned: cancelled, exit 130, nothing on
    /// stdout. Returns stderr.
    fn cancelled(pty: PtyCli) -> String {
        let (out, _) = pty.finish();
        scan_output(&out);
        assert_eq!(out.status, 130, "{}", text(&out.stderr));
        assert!(out.stdout.is_empty());
        text(&out.stderr).to_owned()
    }

    /// `request show` of a request a press cancelled: finished, with the
    /// result the run printed.
    fn stored(clone: &CliClone, request: &str) -> Value {
        let shown = request_show(clone, request);
        assert_eq!(shown["state"], "finished");
        let result = shown["result"].clone();
        assert_eq!(result["outcome"], "cancelled");
        assert_eq!(result["code"], "cancelled");
        result
    }

    /// `item sync`, pressed while it fetches: `cancelled`, exit 130, at the
    /// safe point after the fetch, with its effects intact: nothing merged,
    /// pushed or moved, and the effects claim no more. `request show`
    /// agrees; the same request returns the stored result, with no
    /// connection; a new request merges and publishes.
    pub fn c4_interruption_item_sync_then_a_new_request() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let b = &p.b;
        let (local, incoming) = diverged(&p)?;
        let pushes = remote_world::accepted(&p.server);
        let hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(&b.env, &sync_args(b, R3), &[]);
        pressed_while_held(&mut pty, hold)?;
        let stderr = cancelled(pty);
        let operation = the_operation(b, R3);
        assert_eq!(
            stderr,
            format!(
                "{}{CANCELLED}request: {R3}\noperation: {operation}\neffects: publication=pending\n",
                progress(&[
                    "accepted",
                    "connecting",
                    "fetching",
                    "observing",
                    "stopping"
                ])
            )
        );
        assert_eq!(b.tip(CONTEXT)?, Some(local));
        assert_eq!(p.served(CONTEXT)?, Some(incoming));
        assert_eq!(remote_world::accepted(&p.server), pushes);
        let result = stored(b, R3);
        assert_effects(&result, &[("publication", "pending")]);
        assert!(result["effects"]["commit_oid"].is_null());

        let opened = connections(&p);
        let replay = item_sync(b, ITEM, R3, "cancelled", 130);
        assert_eq!(replay["operation_id"], operation.as_str());
        assert_eq!(replay["effects"], result["effects"]);
        assert!(replay["recovery"].as_array().unwrap().is_empty());
        assert_eq!(connections(&p), opened);

        let merged = item_sync(b, ITEM, R4, "ok", 0);
        assert_ne!(merged["operation_id"], operation.as_str());
        let merge = b.tip(CONTEXT)?.ok_or(FixtureError)?;
        assert_effects(
            &merged,
            &[
                ("discovery", "current"),
                ("publication", "published"),
                ("integration", "complete"),
            ],
        );
        assert_eq!(oid(&merged["effects"]["commit_oid"]), merge);
        assert_merge(b, merge, local, incoming);
        assert_eq!(p.served(CONTEXT)?, Some(merge));
        assert_eq!(remote_world::accepted(&p.server), pushes + 1);
        Ok(())
    }

    /// `comment add`, pressed while its publication fetches (comment table,
    /// "Ctrl-C during publication, before a push"): `cancelled`, exit 130,
    /// the comment written and checkpointed, publication pending, and
    /// nothing pushed. The operation is final (fact 56): the same request
    /// returns its stored result, and a new `item sync` publishes the
    /// comment.
    pub fn c4_interruption_comment_add_then_item_sync() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        private_body();
        let pushes = remote_world::accepted(&p.server);
        let hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(
            &a.env,
            &comment_args(a, ITEM, R1),
            &comment_input(C1, None, PRIVATE_BODY),
        );
        pressed_while_held(&mut pty, hold)?;
        let stderr = cancelled(pty);
        let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
        let operation = the_operation(a, R1);
        assert!(
            stderr.starts_with(&format!(
                "{}{CANCELLED}request: {R1}\noperation: {operation}\n\
                 effects: write=written checkpoint=committed discovery=current publication=pending commit={checkpoint}\n",
                progress(&["accepted", "writing", "fetching", "observing", "stopping"])
            )),
            "{stderr}"
        );
        assert!(
            stderr.contains("Pending reason: synchronization\nPublication: pending\n"),
            "{stderr}"
        );
        assert!(stderr.ends_with(PUBLICATION_NOTE), "{stderr}");
        assert!(holds(&a.repo()?, checkpoint, &comment_path(C1))?);
        assert_eq!(p.served(CONTEXT)?, None);
        assert_eq!(remote_world::accepted(&p.server), pushes);
        let result = stored(a, R1);
        assert_saved(&result, "pending");
        assert_eq!(oid(&result["effects"]["commit_oid"]), checkpoint);
        assert_eq!(result["data"]["pending_reason"], "synchronization");

        let opened = connections(&p);
        let replay = add(a, R1, C1, None, PRIVATE_BODY, "cancelled", 130);
        assert_eq!(replay["effects"], result["effects"]);
        assert_eq!(replay["data"], result["data"]);
        assert_eq!(connections(&p), opened);

        let published = item_sync(a, ITEM, R2, "ok", 0);
        assert_effects(
            &published,
            &[("discovery", "current"), ("publication", "published")],
        );
        assert_eq!(oid(&published["effects"]["commit_oid"]), checkpoint);
        assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
        assert!(holds(&p.bare()?, checkpoint, &comment_path(C1))?);
        assert_eq!(remote_world::accepted(&p.server), pushes + 1);
        Ok(())
    }

    /// `conflict resolve`, pressed while its continuation fetches
    /// (interrupted-resolution table, "Ctrl-C during publication"):
    /// `cancelled`, exit 130, the resolution written and committed, nothing
    /// pushed. The synchronization is final (fact 56): `conflict show` reads
    /// it cancelled, and a new `repo sync` publishes the resolution commit.
    pub fn c4_interruption_conflict_resolve_then_repo_sync() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let b = &p.b;
        let (local, incoming) = tickets_diverge(&p)?;
        let synchronization = stopped_on_primary(&p, "resolve");
        let shown = show(b, &synchronization);
        private_source();
        let input = resolve_input(observation(&shown), &resolutions(&shown, "Shared"));
        let pushes = remote_world::accepted(&p.server);
        let hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(&b.env, &resolve_args(b, &synchronization, R4), &input);
        pressed_while_held(&mut pty, hold)?;
        let stderr = cancelled(pty);
        let commit = b.tip(PRIMARY)?.ok_or(FixtureError)?;
        assert_merge(b, commit, local, incoming);
        // The resolve holds no operation of its own (R-35): no operation
        // line, and the data names the synchronization.
        assert_eq!(
            stderr,
            format!(
                "{}{CANCELLED}request: {R4}\n\
                 effects: write=written checkpoint=committed discovery=pending publication=pending integration=complete commit={commit}\n\
                 Commit oid: {commit}\nConflict: none\nOperation id: {synchronization}\n\
                 Publication: pending\nType: conflict_resolution\n",
                progress(&["accepted", "writing", "fetching", "observing", "stopping"])
            )
        );
        assert_eq!(p.served(PRIMARY)?, Some(incoming));
        assert_eq!(remote_world::accepted(&p.server), pushes);
        let result = stored(b, R4);
        assert!(
            request_show(b, R4)["operations"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_effects(
            &result,
            &[
                ("write", "written"),
                ("checkpoint", "committed"),
                ("discovery", "pending"),
                ("publication", "pending"),
                ("integration", "complete"),
            ],
        );
        assert_eq!(oid(&result["effects"]["commit_oid"]), commit);
        assert_eq!(result["data"]["operation_id"], synchronization.as_str());
        assert_eq!(result["data"]["publication"], "pending");
        let read = show(b, &synchronization);
        assert_eq!(read["data"]["state"], "cancelled");
        assert_eq!(read["data"]["next_action"], "synchronize");

        let opened = connections(&p);
        let replay = b.json(
            &resolve_args(b, &synchronization, R4),
            &input,
            "cancelled",
            130,
        );
        assert_eq!(replay["effects"], result["effects"]);
        assert_eq!(replay["data"], result["data"]);
        assert_eq!(connections(&p), opened);

        let published = repo_sync(b, R5, "ok", 0);
        assert_effects(
            &published,
            &[("discovery", "current"), ("publication", "published")],
        );
        assert_eq!(oid(&published["effects"]["commit_oid"]), commit);
        assert_eq!(p.served(PRIMARY)?, Some(commit));
        assert_eq!(remote_world::accepted(&p.server), pushes + 1);
        Ok(())
    }

    /// `item republish --confirm`, pressed while it fetches (republish
    /// table, "Ctrl-C during execution"): `cancelled`, exit 130, and the
    /// branch is not recreated. The operation is final (fact 56) and its
    /// confirmation spent: a new request with it is `confirmation_used`, and
    /// a new prepare, confirmed, republishes.
    pub fn c4_interruption_item_republish_then_a_new_prepare() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let tip = published_then_deleted(&p)?;
        sync_refused(&p)?;
        release(a);
        let primary = a.tip(PRIMARY)?;
        let preview = prepare(a);
        let updates = p.server.receive_updates().len();
        let hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(
            &a.env,
            &confirm_args(a, ITEM, R3, confirmation(&preview)),
            &[],
        );
        pressed_while_held(&mut pty, hold)?;
        let stderr = cancelled(pty);
        let operation = the_operation(a, R3);
        assert_eq!(
            stderr,
            format!(
                "{}{CANCELLED}request: {R3}\noperation: {operation}\neffects: publication=pending\n",
                progress(&[
                    "accepted",
                    "connecting",
                    "fetching",
                    "observing",
                    "stopping"
                ])
            )
        );
        assert_eq!(p.served(CONTEXT)?, None);
        assert_eq!(p.server.receive_updates().len(), updates);
        assert_eq!(a.tip(CONTEXT)?, Some(tip));
        let result = stored(a, R3);
        assert_effects(&result, &[("publication", "pending")]);

        let opened = connections(&p);
        let replay = confirm(a, R3, &preview, "cancelled", 130);
        assert_eq!(replay["effects"], result["effects"]);
        assert_eq!(connections(&p), opened);
        let used = confirm(a, R4, &preview, "confirmation_used", 3);
        assert_effects(&used, &[]);
        no_record(a, R4);

        let again = prepare(a);
        assert_eq!(oid(&again["data"]["details"]["local_oid"]), tip);
        let republished = confirm(a, R5, &again, "ok", 0);
        assert_republished(&p, &republished, tip, updates, primary)
    }

    /// `operation resume` of a comment whose publication stopped, pressed
    /// while it fetches: `cancelled`, exit 130, with nothing pushed. Resume
    /// holds no operation of its own; the comment's synchronization is final
    /// (fact 56), so the comment's own request finds it cancelled, and a new
    /// `item sync` publishes the comment.
    pub fn c4_interruption_operation_resume_then_item_sync() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        private_body();
        add(a, R1, C1, None, PRIVATE_BODY, "ok", 0);
        let served = p.served(CONTEXT)?;
        p.server.disconnect_at(FixtureBoundary::Advertisement);
        let stopped = add(a, R2, C2, None, PRIVATE_BODY, "transport_unavailable", 4);
        p.server.clear_fault();
        let checkpoint = a.tip(CONTEXT)?.ok_or(FixtureError)?;
        let comment = stopped["operation_id"].as_str().unwrap().to_owned();
        let synchronization = stopped["data"]["synchronization_id"].clone();
        let pushes = remote_world::accepted(&p.server);
        let hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(&a.env, &resume_args(a, &comment, R3), &[]);
        pressed_while_held(&mut pty, hold)?;
        let stderr = cancelled(pty);
        assert!(
            stderr.starts_with(&format!(
                "{}{CANCELLED}request: {R3}\neffects: publication=pending\n",
                progress(&["accepted", "observing", "fetching", "observing", "stopping"])
            )),
            "{stderr}"
        );
        assert!(
            stderr.contains("Pending reason: synchronization\nPublication: pending\n"),
            "{stderr}"
        );
        assert!(stderr.ends_with(PUBLICATION_NOTE), "{stderr}");
        assert_eq!(p.served(CONTEXT)?, served);
        assert_eq!(a.tip(CONTEXT)?, Some(checkpoint));
        assert_eq!(remote_world::accepted(&p.server), pushes);
        let result = stored(a, R3);
        assert!(
            request_show(a, R3)["operations"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_effects(&result, &[("publication", "pending")]);
        assert_eq!(result["data"]["synchronization_id"], synchronization);
        assert_eq!(result["data"]["publication"], "pending");

        let opened = connections(&p);
        let replay = resume(a, &comment, R3, "cancelled", 130);
        assert_eq!(replay["effects"], result["effects"]);
        assert_eq!(connections(&p), opened);
        // The comment's own request: its publication ended cancelled.
        let own = add(a, R2, C2, None, PRIVATE_BODY, "cancelled", 130);
        assert_saved(&own, "pending");
        assert_eq!(oid(&own["effects"]["commit_oid"]), checkpoint);
        assert_eq!(connections(&p), opened);

        let published = item_sync(a, ITEM, R4, "ok", 0);
        assert_eq!(published["effects"]["publication"], "published");
        assert_eq!(oid(&published["effects"]["commit_oid"]), checkpoint);
        assert_eq!(p.served(CONTEXT)?, Some(checkpoint));
        assert!(holds(&p.bare()?, checkpoint, &comment_path(C2))?);
        assert_eq!(remote_world::accepted(&p.server), pushes + 1);
        Ok(())
    }

    /// A second press ends the process by `SIGINT` (R-21): interruption, not
    /// rollback. Nothing is merged or pushed, and nothing more is printed.
    /// `request show` has the request still accepted, its operation
    /// pending. That operation holds the reservation, so a new request is
    /// `busy`, as a blocked synchronization's is (Known Limits,
    /// `01M4R19PC0T6ZPKM39X9DF26HX`). The interrupted request, sent again,
    /// takes its operation up and completes the work; a new request then
    /// finds nothing to do.
    pub fn c4_interruption_second_press_ends_the_process() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let b = &p.b;
        let (local, incoming) = diverged(&p)?;
        let pushes = remote_world::accepted(&p.server);
        let hold = p.server.hold_advertisement()?;
        let pty = {
            let mut pty = at_a_terminal(&b.env, &sync_args(b, R3), &[]);
            hold.wait_until_held()?;
            pty.send(&[3]);
            std::thread::sleep(LANDING);
            pty.send(&[3]);
            pty
        };
        let (code, signal, stdout, stderr) = pty.finish_any();
        // The server lets the dead client's connection go.
        drop(hold);
        scan_output(&CliRun {
            stdout: stdout.clone().into_bytes(),
            stderr: stderr.clone().into_bytes(),
            status: 0,
        });
        assert_eq!((code, signal), (None, Some(libc::SIGINT)));
        // Nothing more is printed: no result.
        assert!(stdout.is_empty(), "{stdout}");
        assert_eq!(stderr, progress(&["accepted", "connecting", "fetching"]));
        assert_eq!(b.tip(CONTEXT)?, Some(local));
        assert_eq!(p.served(CONTEXT)?, Some(incoming));
        assert_eq!(remote_world::accepted(&p.server), pushes);

        let shown = request_show(b, R3);
        assert_eq!(shown["state"], "accepted");
        assert!(shown["result"].is_null());
        let operation = the_operation(b, R3);
        let listed = checked(
            &b.env,
            &["operation", "list", "--repo", b.root()],
            &[],
            "ok",
            0,
            None,
        );
        let items = listed["data"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 1, "{listed}");
        assert_eq!(items[0]["operation_id"], operation.as_str());
        assert_eq!(items[0]["action"], "synchronize_context");
        assert!(items[0]["failure_code"].is_null());

        let busy = item_sync(b, ITEM, R4, "busy", 5);
        assert_eq!(
            busy["recovery"],
            serde_json::json!([{"action": "request.retry", "operation_id": null, "arguments": {"request_id": R4}}])
        );
        assert_eq!(remote_world::accepted(&p.server), pushes);

        let merged = item_sync(b, ITEM, R3, "ok", 0);
        assert_eq!(merged["operation_id"], operation.as_str());
        let merge = b.tip(CONTEXT)?.ok_or(FixtureError)?;
        assert_eq!(merged["effects"]["integration"], "complete");
        assert_eq!(merged["effects"]["publication"], "published");
        assert_eq!(oid(&merged["effects"]["commit_oid"]), merge);
        assert_merge(b, merge, local, incoming);
        assert_eq!(p.served(CONTEXT)?, Some(merge));
        assert_eq!(remote_world::accepted(&p.server), pushes + 1);
        item_sync(b, ITEM, R4, "already_applied", 0);
        assert_eq!(remote_world::accepted(&p.server), pushes + 1);
        Ok(())
    }

    /// How long the server keeps back its answer to the push: the window in
    /// which the press lands, after the last safe point.
    const STALL: std::time::Duration = std::time::Duration::from_secs(5);
    /// How long the client may take, under load, to send its push command
    /// once the push's advertisement is let go.
    const ARRIVAL: std::time::Duration = std::time::Duration::from_secs(30);

    /// R-36: a press that lands after the last safe point of an `item sync`
    /// whose push the server then refuses. The library's last check of the
    /// cancellation in a push is on its connection, before the push is sent
    /// (`with_scoped_action_remote`); a refused push returns without
    /// another. So the press is seen by no safe point: the run answers
    /// `push_rejected`, exit 4, a partial C2's bounded retry would re-run,
    /// and the latched press stops that retry: one push, no `retrying`.
    ///
    /// The staging holds each connection in turn until the push's, then
    /// stalls the server's answer to it. The fixture's receive race marks
    /// when the client's push command arrives, which it sends only after
    /// that last check: another clone's publication lands on the branch at
    /// that moment. The press is made then.
    ///
    /// The same request, sent again without a press and with the race
    /// re-armed, is retried by C2: four refused pushes. Once the server
    /// takes pushes, it publishes.
    pub fn c4_interruption_press_after_the_last_safe_point() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let b = &p.b;
        let (local, incoming) = diverged(&p)?;
        // The server refuses updates of the branch it has checked out.
        fixed(p.bare()?.set_head(CONTEXT))?;
        p.server.reject_primary_updates(true)?;
        // Another clone's publication, made on the server as the push
        // command arrives.
        let competing = remote_world::commit(&p.bare()?, incoming, "competing", b"competing")?;
        p.server.race_update(CONTEXT, incoming, competing)?;
        let updates = p.server.receive_updates().len();
        let mut hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(&b.env, &sync_args(b, R3), &[]);
        loop {
            hold.wait_until_held()?;
            let pushing = p
                .server
                .commands()
                .last()
                .is_some_and(|command| command.starts_with(b"git-receive-pack "));
            if pushing {
                break;
            }
            let next = p.server.hold_advertisement()?;
            hold.release()?;
            hold = next;
        }
        p.server.stall_at(FixtureBoundary::Transfer, STALL);
        hold.release()?;
        // The stall starts only once the command has arrived, so how long
        // the client takes to send it does not shorten the press's window.
        let deadline = std::time::Instant::now() + ARRIVAL;
        while p.served(CONTEXT)? != Some(competing) {
            assert!(
                std::time::Instant::now() < deadline,
                "the push command did not arrive"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        pty.send(&[3]);
        std::thread::sleep(LANDING);
        let (out, _) = pty.finish();
        p.server.clear_fault();
        scan_output(&out);
        assert_eq!(out.status, 4, "{}", text(&out.stderr));
        assert!(out.stdout.is_empty());
        // The merge was made before the push, and stays.
        let merge = b.tip(CONTEXT)?.ok_or(FixtureError)?;
        assert_merge(b, merge, local, incoming);
        let operation = the_operation(b, R3);
        // C2's refusal, with no `retrying` and no `stopping`.
        assert_eq!(
            text(&out.stderr),
            format!(
                "{}error: The remote rejected the push.\ncode: push_rejected\n\
                 request: {R3}\noperation: {operation}\n\
                 effects: publication=pending integration=complete commit={merge}\n\
                 next: request.retry request_id={R3}\n",
                progress(&["accepted", "connecting", "fetching", "observing", "pushing"])
            )
        );
        // One push, refused: no second execute ran.
        let refused = &p.server.receive_updates()[updates..];
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert!(!refused[0].accepted);
        assert_eq!(refused[0].reference, CONTEXT);
        assert_eq!(refused[0].new_oid, merge);
        assert_eq!(p.served(CONTEXT)?, Some(competing));
        // The request stays accepted, for its retry.
        let record = request_show(b, R3);
        assert_eq!(record["state"], "accepted");
        assert!(record["result"].is_null());

        // The control, staged as the pressed run is, with no press: the race
        // re-armed, so another clone's publication lands again as the push
        // command arrives, and the server refuses the push. C2's bounded
        // retry re-runs the same request three more times: four refused
        // pushes, one per execute. The first merges the publication the
        // pressed run met; the second merges the one this run met, and the
        // last two push that merge again.
        let again = remote_world::commit(&p.bare()?, competing, "again", b"again")?;
        p.server.race_update(CONTEXT, competing, again)?;
        let updates = p.server.receive_updates().len();
        let retried = item_sync(b, ITEM, R3, "push_rejected", 4);
        assert_eq!(retried["outcome"], "partial");
        assert_eq!(retried["operation_id"], operation.as_str());
        assert_eq!(p.served(CONTEXT)?, Some(again));
        let refused = &p.server.receive_updates()[updates..];
        assert_eq!(refused.len(), 4, "{refused:?}");
        assert!(refused.iter().all(|update| !update.accepted));
        let first = refused[0].new_oid;
        assert_merge(b, first, merge, competing);
        let remerged = b.tip(CONTEXT)?.ok_or(FixtureError)?;
        assert_merge(b, remerged, first, again);
        assert!(
            refused[1..].iter().all(|update| update.new_oid == remerged),
            "{refused:?}"
        );

        // The server takes pushes again: the same request publishes.
        p.server.reject_primary_updates(false)?;
        fixed(p.bare()?.set_head(PRIMARY))?;
        let published = item_sync(b, ITEM, R3, "ok", 0);
        assert_eq!(published["effects"]["publication"], "published");
        assert_eq!(oid(&published["effects"]["commit_oid"]), remerged);
        assert_eq!(p.served(CONTEXT)?, Some(remerged));
        Ok(())
    }

    // ------------------------------------------------------------------ C5

    use super::c5::{
        args as lifecycle_args, begun_outside, behind, confirmed, held_by, nothing_left, poll_args,
        polled, record, reserved,
    };
    use lifecycle::{finished_once, merges};

    /// A cancelled lifecycle answer at a terminal: exit 130, nothing on
    /// stdout, `code: cancelled` and the request's own retry. Returns stderr.
    fn lifecycle_cancelled(pty: PtyCli, request: &str) -> String {
        let stderr = cancelled(pty);
        assert!(stderr.contains(CANCELLED), "{stderr}");
        assert!(
            stderr.contains(&format!("next: request.retry request_id={request}\n")),
            "{stderr}"
        );
        assert!(
            !stderr.contains("progress: retrying"),
            "no retry follows a press"
        );
        stderr
    }

    /// Decision 8 at a terminal: the verb's confirmed request, pressed while
    /// the server holds the context child's fetch, before integration. It
    /// answers `cancelled`, exit 130, and the record stays accepted: nothing
    /// is merged or pushed, and the child is let go. A new ID's prepare is
    /// held by the request (decision 33); the same request then finishes,
    /// once; and once it has, a new prepare has nothing to do.
    pub fn press_before_integration(verb: Verb) -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        verb.begin(a, R1);
        let preview = verb.prepare(a, "ok", 0);
        let pushes = remote_world::accepted(&p.server);
        let hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(&a.env, &confirmed(verb, a, R2, &preview), &[]);
        pressed_while_held(&mut pty, hold)?;
        lifecycle_cancelled(pty, R2);
        let shown = record(a, R2);
        assert_eq!(shown["state"], "accepted", "{shown}");
        assert!(shown["result"].is_null(), "{shown}");
        assert_eq!(merges(&a.repo()?)?, 0);
        assert_eq!(remote_world::accepted(&p.server), pushes, "nothing pushed");
        assert!(!reserved(a), "the cancellation let the child go");

        held_by(a, verb, R2);
        let done = verb.send(a, R2, "ok", 0);
        assert_eq!(done["effects"]["cleanup"], "complete", "{done}");
        assert_eq!(record(a, R2)["state"], "finished");
        finished_once(&p, verb, None)?;
        nothing_left(a, verb);
        Ok(())
    }

    /// The same past integration: each connection is held in turn until
    /// primary holds the merge, and the press lands at the publication's
    /// fetch. `cancelled`, exit 130, integration complete and publication
    /// pending, the record accepted; a new ID's prepare is held by the
    /// request; the same request publishes and cleans up, with one merge.
    pub fn press_after_integration(verb: Verb) -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        verb.begin(a, R1);
        let preview = verb.prepare(a, "ok", 0);
        let served = p.served(PRIMARY)?;
        let mut hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(&a.env, &confirmed(verb, a, R2, &preview), &[]);
        loop {
            hold.wait_until_held()?;
            if merges(&a.repo()?)? == 1 {
                break;
            }
            let next = p.server.hold_advertisement()?;
            hold.release()?;
            hold = next;
        }
        pty.send(&[3]);
        std::thread::sleep(LANDING);
        hold.release()?;
        let stderr = lifecycle_cancelled(pty, R2);
        assert!(stderr.contains("integration=complete"), "{stderr}");
        let shown = record(a, R2);
        assert_eq!(shown["state"], "accepted", "{shown}");
        assert_eq!(merges(&a.repo()?)?, 1);
        assert_eq!(p.served(PRIMARY)?, served, "primary is not published");

        held_by(a, verb, R2);
        let done = verb.send(a, R2, "ok", 0);
        assert_eq!(done["effects"]["publication"], "published", "{done}");
        finished_once(&p, verb, None)?;
        nothing_left(a, verb);
        Ok(())
    }

    /// A second press ends the process by `SIGINT` (R-21, design: "a second
    /// press or a kill leaves the record"): nothing more is printed, nothing
    /// is merged or pushed, the record stays accepted with no result, and the
    /// same request then finishes, once. One verb: the window is the CLI's,
    /// shared by both (decision 27).
    pub fn c5_interruption_second_press_close() -> Result<(), FixtureError> {
        let verb = Verb::Close;
        let p = CliPair::new()?;
        let a = &p.a;
        verb.begin(a, R1);
        let preview = verb.prepare(a, "ok", 0);
        let pushes = remote_world::accepted(&p.server);
        let hold = p.server.hold_advertisement()?;
        let pty = {
            let mut pty = at_a_terminal(&a.env, &confirmed(verb, a, R2, &preview), &[]);
            hold.wait_until_held()?;
            pty.send(&[3]);
            std::thread::sleep(LANDING);
            pty.send(&[3]);
            pty
        };
        let (code, signal, stdout, stderr) = pty.finish_any();
        drop(hold);
        scan_output(&CliRun {
            stdout: stdout.clone().into_bytes(),
            stderr: stderr.clone().into_bytes(),
            status: 0,
        });
        assert_eq!((code, signal), (None, Some(libc::SIGINT)));
        assert!(stdout.is_empty(), "{stdout}");
        assert!(!stderr.contains("code: "), "no result: {stderr}");
        assert_eq!(merges(&a.repo()?)?, 0);
        assert_eq!(remote_world::accepted(&p.server), pushes);
        let shown = record(a, R2);
        assert_eq!(shown["state"], "accepted", "{shown}");
        assert!(shown["result"].is_null());

        let done = verb.send(a, R2, "ok", 0);
        assert_eq!(done["effects"]["cleanup"], "complete", "{done}");
        finished_once(&p, verb, None)
    }

    /// The verb at a terminal with neither a request ID nor a confirmation:
    /// the interactive flow. A request ID is allocated and noted first.
    fn interactive(env: &CliEnv, verb: Verb, clone: &CliClone) -> PtyCli {
        let args = lifecycle_args(verb, clone);
        PtyCli::spawn(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env, &[])
    }

    /// The request ID the interactive flow allocated, from its note.
    fn allocated(pty: &PtyCli) -> String {
        let line = pty.first_stderr_line();
        line.strip_prefix("request: ")
            .unwrap_or_else(|| panic!("no request note: {line}"))
            .to_owned()
    }

    /// The interactive consent of both verbs (design matrix, 13a): the
    /// preview is shown at the terminal, `y` consents, and the run completes
    /// under the allocated request, once.
    pub fn consent_prompt(verb: Verb) -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        verb.begin(a, R1);
        let mut pty = interactive(&a.env, verb, a);
        let request = allocated(&pty);
        pty.expect("Proceed? [y/N] ");
        pty.send(b"y\n");
        let (out, transcript) = pty.finish();
        scan_output(&out);
        assert_eq!(out.status, 0, "{}", text(&out.stderr));
        assert!(
            text(&out.stdout).starts_with(&format!("The request completed.\nrequest: {request}\n")),
            "{}",
            text(&out.stdout)
        );
        assert_eq!(transcript.matches("Proceed? [y/N] ").count(), 1);
        assert_eq!(record(a, &request)["state"], "finished");
        finished_once(&p, verb, None)
    }

    /// A press at the consent prompt: the prompt reads in raw mode, so the
    /// byte is its own refusal, not a press the window sees. `cancelled`, exit
    /// 130, the terminal's modes restored, no request recorded, nothing moved
    /// at either end, and a new prepare is ready: nothing holds the item.
    pub fn press_at_the_consent_prompt(verb: Verb) -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        verb.begin(a, R1);
        let (local, served) = (lifecycle::refs(&a.repo()?)?, lifecycle::refs(&p.bare()?)?);
        let opened = connections(&p);
        let mut pty = interactive(&a.env, verb, a);
        let request = allocated(&pty);
        pty.expect("Proceed? [y/N] ");
        pty.send(&[3]);
        let (before, after) = pty.lflag_before_and_after();
        assert_eq!(before, after, "the terminal's modes are restored");
        let (out, _) = pty.finish();
        scan_output(&out);
        assert_eq!(out.status, 130, "{}", text(&out.stderr));
        assert!(out.stdout.is_empty());
        assert!(
            text(&out.stderr).contains(CANCELLED),
            "{}",
            text(&out.stderr)
        );
        checked(
            &a.env,
            &["request", "show", "--request-id", &request],
            &[],
            "request_not_found",
            2,
            None,
        );
        assert_eq!(lifecycle::refs(&a.repo()?)?, local);
        assert_eq!(lifecycle::refs(&p.bare()?)?, served);
        assert_eq!(connections(&p), opened, "the prompt connected to nothing");
        verb.prepare(a, "ok", 0);
        Ok(())
    }

    /// Ctrl-C during `operation resume` of a lifecycle no request holds
    /// (design, Resume: "Ctrl-C is as the verb's"; R-34): pressed while the
    /// server holds the context child's fetch, it answers `cancelled`, exit
    /// 130, nothing merged or pushed. A resume keeps no record of its own, and
    /// a later resume finishes the work, once.
    pub fn press_during_resume(verb: Verb) -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        verb.begin(a, R1);
        let operation = begun_outside(a, verb)?;
        let pushes = remote_world::accepted(&p.server);
        let hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(&a.env, &resume_args(a, &operation, R2), &[]);
        pressed_while_held(&mut pty, hold)?;
        let stderr = cancelled(pty);
        assert!(stderr.contains(CANCELLED), "{stderr}");
        assert_eq!(merges(&a.repo()?)?, 0);
        assert_eq!(remote_world::accepted(&p.server), pushes, "nothing pushed");
        checked(
            &a.env,
            &["request", "show", "--request-id", R2],
            &[],
            "request_not_found",
            2,
            None,
        );
        let done = resume(a, &operation, R3, "ok", 0);
        assert_eq!(done["effects"]["publication"], "published", "{done}");
        finished_once(&p, verb, None)
    }

    /// `poll once` pressed while the server holds its fetch: `cancelled`,
    /// exit 130, nothing fast-forwarded. Its stop after the reservation is
    /// final (R-18): the record is finished with that answer, the same request
    /// replays it with no connection, and a new request polls.
    pub fn c5_interruption_press_during_poll_once() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let before = a.tip(PRIMARY)?;
        let incoming = behind(&p)?;
        let hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(&a.env, &poll_args(a, R1), &[]);
        pressed_while_held(&mut pty, hold)?;
        let stderr = cancelled(pty);
        assert!(stderr.contains(CANCELLED), "{stderr}");
        assert_eq!(a.tip(PRIMARY)?, before, "nothing fast-forwarded");
        let result = stored(a, R1);
        let opened = connections(&p);
        let replay = polled(a, R1, "cancelled", 130);
        assert_eq!(replay["effects"], result["effects"]);
        assert_eq!(connections(&p), opened);
        polled(a, R2, "ok", 0);
        assert_eq!(a.tip(PRIMARY)?, Some(incoming));
        Ok(())
    }

    /// The row the library tables owe: a primary child that fails while
    /// cancellation is asked (`publish`'s `Err` arm, `promotion/run.rs`).
    /// The closure is integrated; the press lands while the server holds the
    /// primary publication's fetch, and the server then drops that
    /// connection, so the child fails rather than stopping. The runner lets
    /// the child go (decision 18): `cancelled`, exit 130, integration
    /// complete, no reservation held, the record accepted. The same request
    /// then publishes and cleans up, with one merge. One verb: both reach the
    /// same `publish` (decision 27).
    pub fn c5_interruption_primary_child_fails_with_cancellation_asked_close()
    -> Result<(), FixtureError> {
        let verb = Verb::Close;
        let p = CliPair::new()?;
        let a = &p.a;
        verb.begin(a, R1);
        let preview = verb.prepare(a, "ok", 0);
        let served = p.served(PRIMARY)?;
        let mut hold = p.server.hold_advertisement()?;
        let mut pty = at_a_terminal(&a.env, &confirmed(verb, a, R2, &preview), &[]);
        loop {
            hold.wait_until_held()?;
            if merges(&a.repo()?)? == 1 {
                break;
            }
            let next = p.server.hold_advertisement()?;
            hold.release()?;
            hold = next;
        }
        pty.send(&[3]);
        std::thread::sleep(LANDING);
        p.server.disconnect_at(FixtureBoundary::Advertisement);
        hold.release()?;
        let stderr = lifecycle_cancelled(pty, R2);
        p.server.clear_fault();
        assert!(stderr.contains("integration=complete"), "{stderr}");
        assert!(!reserved(a), "the failed child was let go");
        assert_eq!(record(a, R2)["state"], "accepted");
        assert_eq!(p.served(PRIMARY)?, served);
        assert_eq!(merges(&a.repo()?)?, 1);

        let done = verb.send(a, R2, "ok", 0);
        assert_eq!(done["effects"]["publication"], "published", "{done}");
        finished_once(&p, verb, None)
    }
}

/// C5's interruption evidence (Task 13a), with what every platform runs:
/// `request cancel` from a second invocation, a killed `poll once` resent,
/// lost stdout and the no-resident-worker check. The Ctrl-C presses are in
/// `pressed` and `blocked`.
mod c5 {
    use super::*;
    use collaboration::connections;
    use lifecycle::{finished_once, merges, peer_commit};

    /// The verb's words, `--repo` and `--id`.
    pub(super) fn args(verb: Verb, clone: &CliClone) -> Vec<&str> {
        let [resource, name] = verb.words();
        vec![resource, name, "--repo", clone.root(), "--id", verb.id()]
    }

    /// The verb's confirmed request, as the binary takes it.
    pub(super) fn confirmed<'a>(
        verb: Verb,
        clone: &'a CliClone,
        request: &'a str,
        preview: &'a Value,
    ) -> Vec<&'a str> {
        let mut args = args(verb, clone);
        args.extend([
            "--request-id",
            request,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ]);
        args
    }

    pub(super) fn actions(value: &Value) -> Vec<&str> {
        value["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .map(|action| action["action"].as_str().unwrap())
            .collect()
    }

    /// Decision 33: while `request` holds the item, a new ID's prepare is
    /// refused, `recovery_required` with no data, and its one action is
    /// `request` sent again.
    pub(super) fn held_by(clone: &CliClone, verb: Verb, request: &str) {
        let held = verb.prepare(clone, "recovery_required", 3);
        assert!(held["data"].is_null(), "{held}");
        assert_eq!(actions(&held), ["request.retry"], "{held}");
        assert_eq!(held["recovery"][0]["arguments"]["request_id"], request);
    }

    /// Once the work is done, a new prepare has nothing to do: it is not
    /// `ok`. Returns its code.
    pub(super) fn nothing_left(clone: &CliClone, verb: Verb) -> String {
        let mut args = args(verb, clone);
        args.extend(["--prepare", "--json"]);
        let out = run(&clone.env, &args, &[]);
        let value = cli::json(&out);
        assert_ne!(value["code"], "ok", "{value}");
        value["code"].as_str().unwrap().to_owned()
    }

    /// Whether a remote operation holds the repository's reservation, as
    /// `poll status` reads it.
    pub(super) fn reserved(clone: &CliClone) -> bool {
        let status = clone.json(&["poll", "status", "--repo", clone.root()], &[], "ok", 0);
        !status["data"]["active_operation_id"].is_null()
    }

    /// The request's stored state and result, through `request show`.
    pub(super) fn record(clone: &CliClone, request: &str) -> Value {
        request_show(clone, request)
    }

    /// A parent begun through the domain's own entry point and stopped at
    /// its first safe point, as `tests/cli_remote/lifecycle.rs` begins one:
    /// no request record holds it.
    pub(super) fn begun_outside(clone: &CliClone, verb: Verb) -> Result<String, FixtureError> {
        use repository::{
            CloseTicketRequest, DocumentPromotionPreflight, FailurePoint, OperationId,
            PreviewDocumentPromotionRequest, PreviewTicketClosureRequest, PromoteDocumentRequest,
            RepositoryService, TicketClosurePreflight, keys::SessionCredentials,
        };
        let service: RepositoryService =
            support::FailOnce::at(FailurePoint::AfterConfirmed).open_service(&clone.env.data);
        let operation_id = OperationId::new();
        let item_id: manyhands::canonical::ItemId = fixed(verb.id().parse())?;
        let mut session = SessionCredentials::new(remote_world::Provider);
        match verb {
            Verb::Promote => {
                let DocumentPromotionPreflight::Ready(preview) = fixed(
                    service.preview_document_promotion(PreviewDocumentPromotionRequest {
                        root: clone.root.clone(),
                        item_id: item_id.clone(),
                    }),
                )?
                else {
                    return Err(FixtureError);
                };
                let _stopped = service.promote_document(
                    PromoteDocumentRequest {
                        root: clone.root.clone(),
                        operation_id,
                        item_id,
                        expected: preview.expectation(),
                        approval: None,
                        confirmed_identity: None,
                    },
                    &mut session,
                );
            }
            Verb::Close => {
                let TicketClosurePreflight::Ready(preview) =
                    fixed(service.preview_ticket_closure(PreviewTicketClosureRequest {
                        root: clone.root.clone(),
                        item_id: item_id.clone(),
                        confirmed_identity: None,
                    }))?
                else {
                    return Err(FixtureError);
                };
                let _stopped = service.close_ticket(
                    CloseTicketRequest {
                        root: clone.root.clone(),
                        operation_id,
                        item_id,
                        expected: preview.expectation(),
                        approval: None,
                        confirmed_identity: None,
                    },
                    &mut session,
                );
            }
        }
        Ok(operation_id.to_string())
    }

    pub(super) fn poll_args<'a>(clone: &'a CliClone, request: &'a str) -> Vec<&'a str> {
        vec![
            "poll",
            "once",
            "--repo",
            clone.root(),
            "--request-id",
            request,
        ]
    }

    /// A peer's commit on the remote's primary, which A's poll brings in.
    pub(super) fn behind(p: &CliPair) -> Result<git2::Oid, FixtureError> {
        peer_commit(p, "incoming.txt", b"incoming\n")
    }

    /// One `poll once` in JSON mode.
    pub(super) fn polled(clone: &CliClone, request: &str, code: &str, status: i32) -> Value {
        clone.json(&poll_args(clone, request), &[], code, status)
    }

    pub(super) fn sync_local_args<'a>(clone: &'a CliClone, request: &'a str) -> Vec<&'a str> {
        vec![
            "context",
            "sync-local",
            "--repo",
            clone.root(),
            "--id",
            ITEM,
            "--input",
            "-",
            "--request-id",
            request,
        ]
    }

    pub(super) fn resolve_local_args<'a>(
        clone: &'a CliClone,
        operation: &'a str,
        request: &'a str,
    ) -> Vec<&'a str> {
        vec![
            "context",
            "resolve-local",
            "--repo",
            clone.root(),
            "--operation-id",
            operation,
            "--input",
            "-",
            "--request-id",
            request,
        ]
    }

    /// `context sync-local`'s input: the observation `observe-local` reads now.
    pub(super) fn observed(clone: &CliClone) -> Vec<u8> {
        let shown = clone.json(
            &[
                "context",
                "observe-local",
                "--repo",
                clone.root(),
                "--id",
                ITEM,
            ],
            &[],
            "ok",
            0,
        );
        serde_json::json!({ "observation": shown["data"]["observation"] })
            .to_string()
            .into_bytes()
    }

    /// A's context of `ITEM`, saved, and A's primary moved on by a peer's
    /// commit of another file, polled in: `context sync-local` merges.
    /// Returns the context's tip and primary's.
    pub(super) fn local_merge(p: &CliPair) -> Result<(git2::Oid, git2::Oid), FixtureError> {
        let a = &p.a;
        save(a, R1, "Saved in clone A.\n");
        let incoming = behind(p)?;
        lifecycle::poll(a, R2);
        assert_eq!(a.tip(PRIMARY)?, Some(incoming));
        Ok((a.tip(CONTEXT)?.ok_or(FixtureError)?, incoming))
    }

    /// A merged `context sync-local`: the context's tip has both parents.
    pub(super) fn assert_locally_merged(
        p: &CliPair,
        context: git2::Oid,
        primary: git2::Oid,
    ) -> Result<(), FixtureError> {
        let merge = p.a.tip(CONTEXT)?.ok_or(FixtureError)?;
        assert_merge(&p.a, merge, context, primary);
        Ok(())
    }

    /// A's saved ticket, and a peer's edit of the same ticket on primary,
    /// polled in: `context sync-local` stops on the conflict, exit 4. Returns
    /// the synchronization's operation and a complete resolution input, A's
    /// side kept.
    pub(super) fn local_conflict(p: &CliPair) -> Result<(String, Vec<u8>), FixtureError> {
        let a = &p.a;
        save(a, R1, "Saved in clone A.\n");
        let moved = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\ntitle: Moved on primary\n\
             type: task\nstatus: open\n---\nBody moved on primary too.\n"
        );
        let incoming = peer_commit(p, &Verb::Close.path(), moved.as_bytes())?;
        lifecycle::poll(a, R2);
        assert_eq!(a.tip(PRIMARY)?, Some(incoming));
        let stopped = a.json(
            &sync_local_args(a, R3),
            &observed(a),
            "recovery_required",
            4,
        );
        assert_eq!(stopped["data"]["state"], "conflict_pending", "{stopped}");
        let operation = stopped["data"]["operation_id"].as_str().unwrap().to_owned();
        let show = |extra: &[&str]| {
            let mut args = vec![
                "context",
                "show-local",
                "--repo",
                a.root(),
                "--operation-id",
                operation.as_str(),
            ];
            args.extend_from_slice(extra);
            a.json(&args, &[], "ok", 0)
        };
        let view = show(&[]);
        let resolutions: Vec<Value> = view["data"]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                let token = entry["token"].as_str().unwrap();
                let sides = show(&["--token", token]);
                serde_json::json!({ "token": token, "source": sides["data"]["local"] })
            })
            .collect();
        let input = serde_json::json!({
            "observation": view["data"]["observation"],
            "resolutions": resolutions,
        });
        Ok((operation, input.to_string().into_bytes()))
    }

    /// The stored state of the local synchronization `operation`.
    pub(super) fn local_state(clone: &CliClone, operation: &str) -> String {
        let shown = clone.json(
            &[
                "context",
                "show-local",
                "--repo",
                clone.root(),
                "--operation-id",
                operation,
            ],
            &[],
            "ok",
            0,
        );
        shown["data"]["state"].as_str().unwrap().to_owned()
    }

    /// `request cancel` of `request` from a second invocation, with an
    /// environment of its own over the same data: asked, and not stopped.
    pub(super) fn asked(clone: &CliClone, request: &str) {
        let second = CliEnv::new(clone.env.data.clone());
        let asked = checked(
            &second,
            &["request", "cancel", "--request-id", request],
            &[],
            "ok",
            0,
            Some("cancel.schema.json"),
        );
        assert_eq!(
            asked["data"],
            cancel_data(request, "accepted", true, false),
            "{asked}"
        );
    }

    /// A JSON-mode run's `cancelled` answer: exit 130, nothing on stderr.
    pub(super) fn cancelled_json(out: &CliRun, request: &str) -> Value {
        scan_output(out);
        assert_eq!(out.status, 130, "{}", String::from_utf8_lossy(&out.stdout));
        assert!(out.stderr.is_empty(), "JSON mode writes nothing on stderr");
        let value = cli::json(out);
        schema::check_published("envelope.schema.json", &value).unwrap();
        assert_eq!(value["outcome"], "cancelled");
        assert_eq!(value["code"], "cancelled");
        assert_eq!(value["request_id"], request);
        value
    }

    /// The verb's confirmed request, cancelled by `request cancel` while the
    /// server holds the context child's fetch (decision 8): `cancelled`, exit
    /// 130, offering the same request; the record stays accepted and nothing
    /// is merged or pushed. A new ID's prepare is held by the request
    /// (decision 33); the same request then finishes, once; and a new prepare
    /// finds nothing left to do.
    pub(super) fn request_cancel(verb: Verb) -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        verb.begin(a, R1);
        let preview = verb.prepare(a, "ok", 0);
        let pushes = remote_world::accepted(&p.server);
        let hold = p.server.hold_advertisement()?;
        let child = started(&a.env, &confirmed(verb, a, R2, &preview), &[]);
        hold.wait_until_held()?;
        asked(a, R2);
        hold.release()?;
        let value = cancelled_json(&child.collected(), R2);
        assert_eq!(actions(&value), ["request.retry"], "{value}");
        let shown = record(a, R2);
        assert_eq!(shown["state"], "accepted", "{shown}");
        assert!(shown["result"].is_null());
        assert_eq!(merges(&a.repo()?)?, 0);
        assert_eq!(remote_world::accepted(&p.server), pushes, "nothing pushed");
        assert!(!reserved(a), "the cancellation let the child go");

        held_by(a, verb, R2);
        let done = verb.send(a, R2, "ok", 0);
        assert_eq!(done["operation_id"], value["operation_id"]);
        assert_eq!(record(a, R2)["state"], "finished");
        finished_once(&p, verb, None)?;
        nothing_left(a, verb);
        Ok(())
    }

    /// `poll once`, cancelled by `request cancel` while the server holds its
    /// fetch: `cancelled`, exit 130, nothing fast-forwarded. A poll's stop
    /// after its reservation is final (R-18): the record is finished, the same
    /// request replays the answer with no connection, and a new request polls.
    pub(super) fn c5_interruption_request_cancel_of_poll_once() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let before = a.tip(PRIMARY)?;
        let incoming = behind(&p)?;
        let value = cancelled_while_held(&p, a, &poll_args(a, R1), &[], R1)?;
        assert_eq!(value["command"], "poll once");
        assert_eq!(a.tip(PRIMARY)?, before, "nothing fast-forwarded");
        let opened = connections(&p);
        let replay = polled(a, R1, "cancelled", 130);
        assert_eq!(replay["effects"], value["effects"]);
        assert_eq!(connections(&p), opened);
        polled(a, R2, "ok", 0);
        assert_eq!(a.tip(PRIMARY)?, Some(incoming));
        Ok(())
    }

    /// A `poll once` killed while the server holds its fetch, then sent again
    /// under its ID: the record is still accepted, holding one operation; the
    /// resend takes that operation up and polls, `ok`, and the request is
    /// finished. The library stages the kill with a failure point
    /// (`c5_poll_once_killed_resent_restarts`); this is a real kill.
    pub(super) fn c5_interruption_killed_poll_once_resent() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let before = a.tip(PRIMARY)?;
        let incoming = behind(&p)?;
        let hold = p.server.hold_advertisement()?;
        let child = started(&a.env, &poll_args(a, R1), &[]);
        hold.wait_until_held()?;
        // Dropping the run kills the process it started, by its handle, and
        // reaps it.
        drop(child);
        drop(hold);
        assert_eq!(a.tip(PRIMARY)?, before);
        let shown = record(a, R1);
        assert_eq!(shown["state"], "accepted", "{shown}");
        let operations = shown["operations"].as_array().unwrap();
        assert_eq!(operations.len(), 1, "{shown}");
        let operation = operations[0]["operation_id"].clone();

        let resent = polled(a, R1, "ok", 0);
        assert_eq!(resent["operation_id"], operation, "{resent}");
        assert_eq!(a.tip(PRIMARY)?, Some(incoming));
        assert_eq!(record(a, R1)["state"], "finished");
        Ok(())
    }

    /// R-25 for the lifecycle verbs: the confirmed request with its stdout
    /// closed exits 1, with nothing on stderr, and its work is done. `request
    /// show` holds the result, and the same request replays it: no second
    /// connection, merge or checkpoint.
    pub(super) fn lost_stdout(verb: Verb) -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        verb.begin(a, R1);
        let preview = verb.prepare(a, "ok", 0);
        let lost = with_stdout_closed(&a.env, &confirmed(verb, a, R2, &preview), &[]);
        assert_eq!(lost.status, 1);
        assert!(lost.stderr.is_empty(), "no second diagnostic");
        finished_once(&p, verb, None)?;
        let stored = record(a, R2);
        assert_eq!(stored["state"], "finished");
        let result = &stored["result"];
        assert_eq!(result["code"], "ok", "{stored}");
        let (pushes, opened) = (remote_world::accepted(&p.server), connections(&p));
        let primary = a.tip(PRIMARY)?;
        let replay = verb.send(a, R2, "ok", 0);
        for key in ["outcome", "code", "effects", "data"] {
            assert_eq!(replay[key], result[key], "{key}");
        }
        assert_eq!(connections(&p), opened);
        assert_eq!(remote_world::accepted(&p.server), pushes);
        assert_eq!(a.tip(PRIMARY)?, primary);
        finished_once(&p, verb, None)
    }

    /// R-25 for `poll once`: its fast-forward is done, `request show` holds
    /// the answer, and the replay makes no connection.
    pub(super) fn c5_interruption_lost_stdout_of_poll_once() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let incoming = behind(&p)?;
        let lost = with_stdout_closed(&a.env, &poll_args(a, R1), &[]);
        assert_eq!(lost.status, 1);
        assert!(lost.stderr.is_empty(), "no second diagnostic");
        assert_eq!(a.tip(PRIMARY)?, Some(incoming));
        let stored = record(a, R1);
        assert_eq!(stored["state"], "finished");
        let result = &stored["result"];
        assert_eq!(result["code"], "ok", "{stored}");
        let opened = connections(&p);
        let replay = polled(a, R1, "ok", 0);
        for key in ["outcome", "code", "effects", "data"] {
            assert_eq!(replay[key], result[key], "{key}");
        }
        assert_eq!(connections(&p), opened);
        Ok(())
    }

    /// R-25 for `context sync-local`: the merge is made once; the replay
    /// makes no second commit.
    pub(super) fn c5_interruption_lost_stdout_of_sync_local() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let (context, primary) = local_merge(&p)?;
        let input = observed(a);
        let lost = with_stdout_closed(&a.env, &sync_local_args(a, R3), &input);
        assert_eq!(lost.status, 1);
        assert!(lost.stderr.is_empty(), "no second diagnostic");
        assert_locally_merged(&p, context, primary)?;
        let merge = a.tip(CONTEXT)?;
        let stored = record(a, R3);
        assert_eq!(stored["state"], "finished");
        let result = &stored["result"];
        assert_eq!(result["code"], "ok", "{stored}");
        let replay = a.json(&sync_local_args(a, R3), &input, "ok", 0);
        for key in ["outcome", "code", "effects", "data"] {
            assert_eq!(replay[key], result[key], "{key}");
        }
        assert_eq!(a.tip(CONTEXT)?, merge, "no second commit");
        Ok(())
    }

    /// R-25 for `context resolve-local`: the resolution is committed once; the
    /// replay makes no second commit.
    pub(super) fn c5_interruption_lost_stdout_of_resolve_local() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let (operation, input) = local_conflict(&p)?;
        let lost = with_stdout_closed(&a.env, &resolve_local_args(a, &operation, R4), &input);
        assert_eq!(lost.status, 1);
        assert!(lost.stderr.is_empty(), "no second diagnostic");
        assert_eq!(local_state(a, &operation), "complete");
        let resolved = a.tip(CONTEXT)?;
        let stored = record(a, R4);
        assert_eq!(stored["state"], "finished");
        let result = &stored["result"];
        assert_eq!(result["code"], "ok", "{stored}");
        let replay = a.json(&resolve_local_args(a, &operation, R4), &input, "ok", 0);
        for key in ["outcome", "code", "effects", "data"] {
            assert_eq!(replay[key], result[key], "{key}");
        }
        assert_eq!(a.tip(CONTEXT)?, resolved, "no second commit");
        Ok(())
    }

    /// The processes still in the session `session`, which `setsid` made the
    /// child lead (`tests/support/cli.rs`): every process whose session ID is
    /// that child's PID.
    #[cfg(target_os = "linux")]
    pub(super) fn in_session(session: u32) -> Vec<u32> {
        let mut found = Vec::new();
        for entry in std::fs::read_dir("/proc").unwrap() {
            let Some(pid) = entry
                .ok()
                .and_then(|entry| entry.file_name().to_str()?.parse::<u32>().ok())
            else {
                continue;
            };
            let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
                continue;
            };
            // After the command's closing parenthesis: state, ppid, pgrp,
            // session.
            let Some(end) = stat.rfind(')') else { continue };
            let fields: Vec<&str> = stat[end + 1..].split_whitespace().collect();
            if fields.get(3).and_then(|field| field.parse::<u32>().ok()) == Some(session) {
                found.push(pid);
            }
        }
        found
    }

    /// Elsewhere on Unix, the session's process group, which `setsid` also
    /// made the child lead: a process left in it answers a null signal.
    #[cfg(all(unix, not(target_os = "linux")))]
    pub(super) fn in_session(session: u32) -> Vec<u32> {
        // SAFETY: a null signal to a process group sends nothing.
        if unsafe { libc::kill(-(session as libc::pid_t), 0) } == 0 {
            vec![session]
        } else {
            Vec::new()
        }
    }

    /// `args` in JSON mode, run to its end: the process exited, and both its
    /// pipes reached EOF within a second of its exit (`collect_cli`); on Unix
    /// no process remains in its session. Returns its envelope.
    fn exited_alone(env: &CliEnv, args: &[&str], code: &str) -> Value {
        let mut args = args.to_vec();
        args.push("--json");
        let child = cli::cli_command(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env)
            .spawn()
            .unwrap();
        #[cfg(unix)]
        let session = child.id();
        let out = cli::collect_cli(child);
        scan_output(&out);
        let value = cli::json(&out);
        assert_eq!(value["code"], code, "{value}");
        #[cfg(unix)]
        assert_eq!(in_session(session), Vec::<u32>::new(), "a process lingers");
        value
    }

    /// The positive control: a child that leaves a sleeping process behind,
    /// its own pipes closed, exits and its pipes reach EOF, and the session
    /// check finds the process it left. The process is then ended by its PID.
    #[cfg(unix)]
    fn the_check_sees_a_lingering_process() {
        use std::os::unix::process::CommandExt;
        let mut command = std::process::Command::new("sh");
        command
            .args(["-c", "sleep 60 </dev/null >/dev/null 2>&1 & exit 0"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        // SAFETY: as `tests/support/cli.rs`: setsid only, after fork.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        let child = command.spawn().unwrap();
        let session = child.id();
        let out = cli::collect_cli(child);
        assert_eq!(out.status, 0);
        let lingering = in_session(session);
        assert!(
            !lingering.is_empty(),
            "the check must see the sleeping child"
        );
        #[cfg(target_os = "linux")]
        for pid in &lingering {
            // SAFETY: a signal to the one process this control started.
            unsafe { libc::kill(*pid as libc::pid_t, libc::SIGKILL) };
        }
        #[cfg(not(target_os = "linux"))]
        // SAFETY: the group this control's child made and leads.
        unsafe {
            libc::kill(-(session as libc::pid_t), libc::SIGKILL)
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !in_session(session).is_empty() {
            assert!(std::time::Instant::now() < deadline, "the control lingers");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// No resident worker (the Cycle's acceptance row): after `poll once`,
    /// `ticket close` and `document promote`, each run to completion over
    /// SSH, the process exited, both pipes reached EOF and, on Unix, nothing
    /// remains in the child's session. The positive control comes first.
    pub(super) fn c5_interruption_no_resident_worker() -> Result<(), FixtureError> {
        #[cfg(unix)]
        the_check_sees_a_lingering_process();
        let p = CliPair::new()?;
        let a = &p.a;
        let incoming = behind(&p)?;
        exited_alone(&a.env, &poll_args(a, R1), "ok");
        assert_eq!(a.tip(PRIMARY)?, Some(incoming));
        for (verb, save, request) in [(Verb::Close, R2, R3), (Verb::Promote, R4, R5)] {
            verb.begin(a, save);
            let preview = verb.prepare(a, "ok", 0);
            let done = exited_alone(&a.env, &confirmed(verb, a, request, &preview), "ok");
            assert_eq!(done["effects"]["cleanup"], "complete", "{done}");
            assert_eq!(p.served(verb.branch())?, None);
        }
        Ok(())
    }
}

/// Ctrl-C for the verbs with no server to hold: the two context mutations,
/// which run locally, and any verb's press after its last safe point.
///
/// One of the run's own pipes is made one page long and filled first, so the
/// run's next write to it waits: its stderr, after the lines given, or its
/// stdout, at the result's first write. A progress line is written before
/// the safe point that reads the cancellation (`local_sync/control.rs`,
/// `request_safe_point`), so a press made while the run waits on a progress
/// line lands before that read. The result is written after the Ctrl-C
/// window has closed, so a press there ends the process, as Ctrl-C does
/// outside a window. The press is `SIGINT` to the run's PID, as a terminal's
/// line discipline sends it on Ctrl-C. Linux only: the pipe's length is set
/// with `F_SETPIPE_SZ`, and the wait is read from `/proc`.
#[cfg(target_os = "linux")]
mod blocked {
    use super::*;
    use c5::{
        asked, assert_locally_merged, behind, local_conflict, local_merge, local_state, observed,
        poll_args, polled, record, resolve_local_args, sync_local_args,
    };
    use std::{
        fs::File,
        os::fd::{AsRawFd, FromRawFd, OwnedFd},
        process::{Child, Stdio},
        time::{Duration, Instant},
    };

    /// How long a press is given to land.
    const LANDING: Duration = Duration::from_millis(500);
    /// How long the run may take, under load, to reach its wait.
    const REACH: Duration = Duration::from_secs(30);

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Stream {
        Stdout,
        Stderr,
    }

    /// A human-mode run with one pipe held.
    struct Held {
        child: Option<Child>,
        held: File,
        stream: Stream,
        filler: usize,
        capacity: usize,
        other: Option<std::thread::JoinHandle<Vec<u8>>>,
    }

    /// What a held run came to.
    struct Ended {
        code: Option<i32>,
        signal: Option<i32>,
        stdout: String,
        stderr: String,
    }

    impl Held {
        /// `args` in human mode, with `input` on stdin, and `stream` held
        /// after the `before` lines.
        fn spawn(
            env: &CliEnv,
            args: &[&str],
            input: &[u8],
            stream: Stream,
            before: &[&str],
        ) -> Self {
            let mut fds = [0; 2];
            // SAFETY: two writable descriptor slots.
            assert_eq!(unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
            // SAFETY: the OS call just opened both, and nothing else owns them.
            let (read, write) =
                unsafe { (File::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
            // SAFETY: a live pipe descriptor; the size is a page.
            let capacity = unsafe { libc::fcntl(write.as_raw_fd(), libc::F_SETPIPE_SZ, 4096) };
            assert!(capacity > 0, "{}", std::io::Error::last_os_error());
            let capacity = capacity as usize;
            let prior: usize = before.iter().map(|line| line.len()).sum();
            let filler = capacity - prior;
            let mut write = File::from(write);
            write.write_all(&vec![b'#'; filler]).unwrap();
            let mut command =
                cli::cli_command(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env);
            command.stdin(Stdio::piped());
            match stream {
                Stream::Stdout => command.stdout(Stdio::from(write)),
                Stream::Stderr => command.stderr(Stdio::from(write)),
            };
            let mut child = command.spawn().unwrap();
            drop(command);
            child.stdin.take().unwrap().write_all(input).unwrap();
            let mut pipe: Box<dyn Read + Send> = match stream {
                Stream::Stdout => Box::new(child.stderr.take().unwrap()),
                Stream::Stderr => Box::new(child.stdout.take().unwrap()),
            };
            let other = std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = pipe.read_to_end(&mut bytes);
                bytes
            });
            Self {
                child: Some(child),
                held: read,
                stream,
                filler,
                capacity,
                other: Some(other),
            }
        }

        fn pid(&self) -> u32 {
            self.child.as_ref().unwrap().id()
        }

        /// Waits until the held pipe is full, with every `before` line in it,
        /// and the run waits to write more.
        fn wait_until_held(&mut self) {
            let deadline = Instant::now() + REACH;
            loop {
                let mut queued: libc::c_int = 0;
                // SAFETY: a live pipe descriptor and an int to write to.
                assert_eq!(
                    unsafe { libc::ioctl(self.held.as_raw_fd(), libc::FIONREAD, &mut queued) },
                    0
                );
                let waiting = std::fs::read_to_string(format!("/proc/{}/wchan", self.pid()))
                    .is_ok_and(|wchan| wchan.contains("pipe_write"));
                if queued as usize == self.capacity && waiting {
                    return;
                }
                if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                    panic!("the run ended before its wait: {status}");
                }
                assert!(Instant::now() < deadline, "the run did not reach its wait");
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        /// Ctrl-C, as the terminal sends it: `SIGINT` to the run.
        fn press(&self) {
            // SAFETY: a signal to the one process this case started.
            assert_eq!(
                unsafe { libc::kill(self.pid() as libc::pid_t, libc::SIGINT) },
                0
            );
            std::thread::sleep(LANDING);
        }

        /// Lets the held pipe go, and waits for the run's end.
        fn finish(mut self) -> Ended {
            use std::os::unix::process::ExitStatusExt;
            let mut held = self.held.try_clone().unwrap();
            let reader = std::thread::spawn(move || {
                let mut bytes = Vec::new();
                held.read_to_end(&mut bytes).unwrap();
                bytes
            });
            let mut child = self.child.take().unwrap();
            let deadline = Instant::now() + REACH;
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("the run did not end");
                }
                std::thread::sleep(Duration::from_millis(5));
            };
            let held = reader.join().unwrap();
            assert!(held[..self.filler].iter().all(|byte| *byte == b'#'));
            let held = String::from_utf8(held[self.filler..].to_vec()).unwrap();
            let other = String::from_utf8(self.other.take().unwrap().join().unwrap()).unwrap();
            let (stdout, stderr) = match self.stream {
                Stream::Stdout => (held, other),
                Stream::Stderr => (other, held),
            };
            let out = CliRun {
                stdout: stdout.clone().into_bytes(),
                stderr: stderr.clone().into_bytes(),
                status: 0,
            };
            scan_output(&out);
            Ended {
                code: status.code(),
                signal: status.signal(),
                stdout,
                stderr,
            }
        }
    }

    impl Drop for Held {
        fn drop(&mut self) {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    const ACCEPTED: &str = "progress: accepted\n";

    /// A run stopped at its first safe point: `cancelled`, exit 130, after
    /// `progress: stopping`.
    fn assert_cancelled(ended: &Ended, request: &str) {
        assert_eq!(
            (ended.code, ended.signal),
            (Some(130), None),
            "{}",
            ended.stderr
        );
        assert!(ended.stdout.is_empty(), "{}", ended.stdout);
        assert!(ended.stderr.starts_with(ACCEPTED), "{}", ended.stderr);
        assert!(
            ended.stderr.contains("progress: stopping\n"),
            "{}",
            ended.stderr
        );
        assert!(
            ended
                .stderr
                .contains(&format!("code: cancelled\nrequest: {request}\n")),
            "{}",
            ended.stderr
        );
    }

    /// `context sync-local`, pressed while it waits on its first safe point's
    /// progress line (the gate's "cancelled at a safe point"): `cancelled`,
    /// exit 130, nothing written, the context and primary as they were. Its
    /// operation stopped before any effect, so it ends `cancelled` and the
    /// request is final: the same request replays the answer, and a new
    /// request, with a fresh observation, merges.
    pub(super) fn c5_interruption_press_at_a_safe_point_of_sync_local() -> Result<(), FixtureError>
    {
        let p = CliPair::new()?;
        let a = &p.a;
        let (context, primary) = local_merge(&p)?;
        let input = observed(a);
        let mut run = Held::spawn(
            &a.env,
            &sync_local_args(a, R3),
            &input,
            Stream::Stderr,
            &[ACCEPTED],
        );
        run.wait_until_held();
        run.press();
        let ended = run.finish();
        assert_cancelled(&ended, R3);
        assert_eq!(a.tip(CONTEXT)?, Some(context), "nothing merged");
        assert_eq!(a.tip(PRIMARY)?, Some(primary));
        let shown = record(a, R3);
        assert_eq!(shown["state"], "finished", "{shown}");
        assert_eq!(shown["result"]["code"], "cancelled");
        let replay = a.json(&sync_local_args(a, R3), &input, "cancelled", 130);
        assert_eq!(replay["effects"], shown["result"]["effects"]);
        a.json(&sync_local_args(a, R4), &observed(a), "ok", 0);
        assert_locally_merged(&p, context, primary)
    }

    /// `context resolve-local`, pressed at its first safe point: `cancelled`,
    /// exit 130. A resolution cancelled keeps its request accepted, for the
    /// same request to finish (the gate: "effects kept, retry or abandon";
    /// `cancelled_partial_resolution_remains_accepted_and_resendable`): sent
    /// again with the same sources, it completes.
    pub(super) fn c5_interruption_press_at_a_safe_point_of_resolve_local()
    -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let (operation, input) = local_conflict(&p)?;
        let mut run = Held::spawn(
            &a.env,
            &resolve_local_args(a, &operation, R4),
            &input,
            Stream::Stderr,
            &[ACCEPTED],
        );
        run.wait_until_held();
        run.press();
        let ended = run.finish();
        assert_cancelled(&ended, R4);
        assert_ne!(local_state(a, &operation), "complete");
        let shown = record(a, R4);
        assert_eq!(shown["state"], "accepted", "{shown}");
        let done = a.json(&resolve_local_args(a, &operation, R4), &input, "ok", 0);
        assert_eq!(done["data"]["state"], "complete", "{done}");
        assert_eq!(local_state(a, &operation), "complete");
        Ok(())
    }

    /// `context sync-local`, cancelled by `request cancel` from a second
    /// invocation while it waits at its first safe point's progress line: it
    /// stops there, `cancelled`, exit 130, nothing written; a new request
    /// merges.
    pub(super) fn c5_interruption_request_cancel_of_sync_local() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let (context, primary) = local_merge(&p)?;
        let mut run = Held::spawn(
            &a.env,
            &sync_local_args(a, R3),
            &observed(a),
            Stream::Stderr,
            &[ACCEPTED],
        );
        run.wait_until_held();
        asked(a, R3);
        let ended = run.finish();
        assert_cancelled(&ended, R3);
        assert_eq!(a.tip(CONTEXT)?, Some(context), "nothing merged");
        assert_eq!(record(a, R3)["state"], "finished");
        a.json(&sync_local_args(a, R4), &observed(a), "ok", 0);
        assert_locally_merged(&p, context, primary)
    }

    /// `context resolve-local`, cancelled by `request cancel` from a second
    /// invocation at its first safe point: `cancelled`, exit 130, and the same
    /// request then completes.
    pub(super) fn c5_interruption_request_cancel_of_resolve_local() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let (operation, input) = local_conflict(&p)?;
        let mut run = Held::spawn(
            &a.env,
            &resolve_local_args(a, &operation, R4),
            &input,
            Stream::Stderr,
            &[ACCEPTED],
        );
        run.wait_until_held();
        asked(a, R4);
        let ended = run.finish();
        assert_cancelled(&ended, R4);
        assert_eq!(record(a, R4)["state"], "accepted");
        a.json(&resolve_local_args(a, &operation, R4), &input, "ok", 0);
        assert_eq!(local_state(a, &operation), "complete");
        Ok(())
    }

    /// A press after the last safe point: the run waits on its result's
    /// first write, after the window has closed, and Ctrl-C ends it there by
    /// `SIGINT`, as outside a window it does. The work stands: the record is
    /// finished `ok`, and the same request replays that answer.
    fn pressed_at_the_result(env: &CliEnv, args: &[&str], input: &[u8]) -> Ended {
        let mut run = Held::spawn(env, args, input, Stream::Stdout, &[]);
        run.wait_until_held();
        run.press();
        let ended = run.finish();
        assert_eq!(
            (ended.code, ended.signal),
            (None, Some(libc::SIGINT)),
            "{}",
            ended.stderr
        );
        assert!(ended.stdout.is_empty(), "{}", ended.stdout);
        assert!(
            !ended.stderr.contains("progress: stopping"),
            "{}",
            ended.stderr
        );
        ended
    }

    /// The stored `ok` of a request whose result was not written, and the
    /// same request's replay of it.
    fn stored_ok(clone: &CliClone, request: &str, replay: Value) {
        let shown = record(clone, request);
        assert_eq!(shown["state"], "finished", "{shown}");
        let result = &shown["result"];
        assert_eq!(result["code"], "ok", "{shown}");
        for key in ["outcome", "code", "effects", "data"] {
            assert_eq!(replay[key], result[key], "{key}");
        }
    }

    pub(super) fn c5_interruption_press_at_the_result_of_poll_once() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let incoming = behind(&p)?;
        pressed_at_the_result(&a.env, &poll_args(a, R1), &[]);
        assert_eq!(a.tip(PRIMARY)?, Some(incoming));
        let opened = collaboration::connections(&p);
        stored_ok(a, R1, polled(a, R1, "ok", 0));
        assert_eq!(collaboration::connections(&p), opened);
        Ok(())
    }

    pub(super) fn c5_interruption_press_at_the_result_of_sync_local() -> Result<(), FixtureError> {
        let p = CliPair::new()?;
        let a = &p.a;
        let (context, primary) = local_merge(&p)?;
        let input = observed(a);
        pressed_at_the_result(&a.env, &sync_local_args(a, R3), &input);
        assert_locally_merged(&p, context, primary)?;
        let merge = a.tip(CONTEXT)?;
        stored_ok(a, R3, a.json(&sync_local_args(a, R3), &input, "ok", 0));
        assert_eq!(a.tip(CONTEXT)?, merge, "no second commit");
        Ok(())
    }

    pub(super) fn c5_interruption_press_at_the_result_of_resolve_local() -> Result<(), FixtureError>
    {
        let p = CliPair::new()?;
        let a = &p.a;
        let (operation, input) = local_conflict(&p)?;
        pressed_at_the_result(&a.env, &resolve_local_args(a, &operation, R4), &input);
        assert_eq!(local_state(a, &operation), "complete");
        let resolved = a.tip(CONTEXT)?;
        stored_ok(
            a,
            R4,
            a.json(&resolve_local_args(a, &operation, R4), &input, "ok", 0),
        );
        assert_eq!(a.tip(CONTEXT)?, resolved, "no second commit");
        Ok(())
    }
}
