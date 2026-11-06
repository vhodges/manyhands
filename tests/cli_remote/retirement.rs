//! The one new transport connection check: the binary's ordinary driver,
//! credentials/host trust and real Fetch advertisement cannot be proved by the
//! local synthetic seam. Edge/crash combinations deliberately stay local.
use super::*;
use cli_pair::{CONTEXT, CliPair, ITEM, OTHER_ITEM, PRIMARY};
use ssh_remote::{FixtureError, fixed};

pub const CASES: &[ssh_harness::Case] = &[(
    "retirement_cli_authenticated_live_pass",
    retirement_cli_authenticated_live_pass,
)];

fn request() -> String {
    repository::RequestId::new().to_string()
}
fn uploads(p: &CliPair) -> usize {
    p.server
        .commands()
        .iter()
        .filter(|c| c.starts_with(b"git-upload-pack "))
        .count()
}
fn close(clone: &cli_pair::CliClone, item: &str) {
    let args = ["ticket", "close", "--repo", clone.root(), "--id", item];
    let mut prepare = args.to_vec();
    prepare.extend(["--prepare", "--json"]);
    let preview = cli::json(&run(&clone.env, &prepare, &[]));
    assert_eq!(preview["code"], "ok");
    let mut confirm = args.to_vec();
    let r = request();
    confirm.extend([
        "--request-id",
        &r,
        "--confirm",
        preview["data"]["confirmation_id"].as_str().unwrap(),
        "--json",
    ]);
    let result = cli::json(&run(&clone.env, &confirm, &[]));
    assert_eq!(result["code"], "ok");
}

fn retirement_cli_authenticated_live_pass() -> Result<(), FixtureError> {
    let p = CliPair::new()?;
    polling::a_dependent(&p)?;
    polling::published(
        &p.a,
        &request(),
        &request(),
        "private retirement item body sentinel\n",
    );
    let shown = p.a.json(
        &["ticket", "show", "--repo", p.a.root(), "--id", OTHER_ITEM],
        &[],
        "ok",
        0,
    );
    let bytes = fixed(serde_json::to_vec(
        &serde_json::json!({"observation":shown["data"]["observation"],"body":"private retirement second body sentinel\n"}),
    ))?;
    p.a.json(
        &[
            "ticket",
            "save",
            "--repo",
            p.a.root(),
            "--id",
            OTHER_ITEM,
            "--input",
            "-",
            "--request-id",
            &request(),
        ],
        &bytes,
        "ok",
        0,
    );
    p.a.json(
        &[
            "item",
            "sync",
            "--repo",
            p.a.root(),
            "--id",
            OTHER_ITEM,
            "--request-id",
            &request(),
        ],
        &[],
        "ok",
        0,
    );
    let materialized = polling::polled(&p.b, &request());
    assert_eq!(
        materialized["data"]["targets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["disposition"] == "materialized")
            .count(),
        2
    );
    let old = p.b.tip(CONTEXT)?.ok_or(FixtureError)?;
    // Publishing another context can replace the latest advertisement image;
    // closure requires fresh Fetch history of its own target, not guessed state.
    polling::polled(&p.a, &request());
    polling::close_in_a(&p, &request())?;
    polling::polled(&p.a, &request());
    close(&p.a, OTHER_ITEM);
    let served = polling::refs(&p.bare()?)?;
    let pushes = remote_world::accepted(&p.server);
    let before_default = uploads(&p);
    let default = polling::polled(&p.b, &request());
    let default_uploads = uploads(&p) - before_default;
    assert_eq!(
        default_uploads, 2,
        "ordinary Fetch authenticates, then reconnects for its authoritative download advertisement"
    );
    schema::check_published("poll_mutation.schema.json", &default["data"]).unwrap();
    assert_eq!(default["data"].as_object().unwrap().len(), 5);
    assert_eq!(
        default["data"]["targets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["disposition"] == "closed")
            .count(),
        2
    );
    assert!(p.b.context().exists(), "default preserves old local copies");
    let other_branch = format!("refs/heads/manyhands/ticket/{OTHER_ITEM}");
    assert!(p.b.tip(&other_branch)?.is_some());
    let old_read = p.b.json(
        &["ticket", "show", "--repo", p.b.root(), "--id", ITEM],
        &[],
        "ok",
        0,
    );
    assert_eq!(
        old_read["data"]["closure"]["state"], "open",
        "default poll retains own-worktree read precedence"
    );
    let old_ready =
        p.b.json(&["ticket", "ready", "--repo", p.b.root()], &[], "ok", 0);
    assert!(
        !old_ready["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == OTHER_ITEM)
    );

    let standalone = request();
    let args = [
        "context",
        "retire",
        "--repo",
        p.b.root(),
        "--kind",
        "ticket",
        "--id",
        ITEM,
        "--request-id",
        &standalone,
    ];
    // Add private canonical bodies to output probes only after source reads and
    // saves; retirement envelopes may never carry them or any key material.
    OUTPUT_PROBES.with(|probes| {
        probes.borrow_mut().extend([
            "private retirement item body sentinel".into(),
            "private retirement second body sentinel".into(),
        ])
    });
    let before = uploads(&p);
    let retired = p.b.json(&args, &[], "ok", 0);
    schema::check_published("context_retirement_mutation.schema.json", &retired["data"]).unwrap();
    assert_eq!(retired["data"]["state"], "retired");
    assert_eq!(
        uploads(&p) - before,
        1,
        "standalone performs its fresh authenticated observation"
    );
    assert!(!p.b.context().exists());
    assert_eq!(p.b.tip(CONTEXT)?, None);
    let ready_after =
        p.b.json(&["ticket", "ready", "--repo", p.b.root()], &[], "ok", 0);
    assert!(
        ready_after["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == OTHER_ITEM),
        "retirement exposes primary closure and unblocks the still-open peer dependent"
    );
    // A new local lifetime at the same ID/tip must survive old request replay.
    let git = p.b.repo()?;
    let reference = fixed(git.reference(CONTEXT, old, false, "new fixture lifetime"))?;
    fixed(git.worktree(
        ITEM,
        &p.b.context(),
        Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
    ))?;
    let before = uploads(&p);
    let replay = p.b.json(&args, &[], "ok", 0);
    assert_eq!(replay["data"], retired["data"]);
    assert_eq!(uploads(&p), before, "replay never observes");
    assert!(p.b.context().exists());

    let pass = request();
    let args = [
        "poll",
        "once",
        "--repo",
        p.b.root(),
        "--retire-completed",
        "--request-id",
        &pass,
    ];
    let before = uploads(&p);
    let result = p.b.json(&args, &[], "ok", 0);
    schema::check_published("poll_retirement_mutation.schema.json", &result["data"]).unwrap();
    assert_eq!(result["data"]["completion"], "completed");
    let rows = result["data"]["retirements"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|r| r["state"] == "retired" && r["result"]["materialization_settled"] == true),
        "{result}"
    );
    assert_eq!(
        uploads(&p) - before,
        default_uploads,
        "one ordinary Fetch's wire cost for both children; no per-child observation"
    );
    assert!(!p.b.context().exists());
    assert!(
        !p.b.root
            .join(".manyhands/worktrees")
            .join(OTHER_ITEM)
            .exists()
    );
    assert_eq!(p.b.tip(&other_branch)?, None);
    assert_eq!(p.b.tip(PRIMARY)?, p.served(PRIMARY)?);
    // Bodies are expected in reads, but never in retirement or replay output.
    OUTPUT_PROBES.with(|probes| {
        probes
            .borrow_mut()
            .retain(|s| !s.starts_with("private retirement"))
    });
    for item in [ITEM, OTHER_ITEM] {
        let shown = p.b.json(
            &["ticket", "show", "--repo", p.b.root(), "--id", item],
            &[],
            "ok",
            0,
        );
        assert_eq!(shown["data"]["closure"]["state"], "closed");
    }
    let ready =
        p.b.json(&["ticket", "ready", "--repo", p.b.root()], &[], "ok", 0);
    assert!(
        !ready["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == ITEM || i["id"] == OTHER_ITEM)
    );
    let list =
        p.b.json(&["ticket", "list", "--repo", p.b.root()], &[], "ok", 0);
    assert!(
        list["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["id"] == ITEM || i["id"] == OTHER_ITEM)
            .all(|i| i["closure"]["state"] == "closed")
    );
    assert_eq!(
        polling::refs(&p.bare()?)?,
        served,
        "no retirement/default-poll pushes or remote ref changes"
    );
    assert_eq!(remote_world::accepted(&p.server), pushes);
    let db = rusqlite::Connection::open(p.b.env.data.join(repository::REGISTRY_FILE)).unwrap();
    assert_eq!(db.query_row("SELECT count(*) FROM retirement_advertisement_provenance WHERE execution_kind='poll_parent'",[],|r|r.get::<_,usize>(0)).unwrap(),1,"one retained authoritative Fetch advertisement, not the initial transport authentication advertisement");
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_operation_records WHERE poll_kind='observe'",
            [],
            |r| r.get::<_, usize>(0)
        )
        .unwrap(),
        1,
        "only the earlier standalone observation; neither pass child observes again"
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM remote_poll_materializations",
            [],
            |r| r.get::<_, usize>(0)
        )
        .unwrap(),
        0
    );
    Ok(())
}
