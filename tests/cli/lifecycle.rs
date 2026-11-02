//! `document promote` and `ticket close` through the binary, in a repository
//! with no publication remote. What needs one is Task 12a's: a publication,
//! a refused push or deletion, and a child's deleted branch. Their human lines
//! are pinned by `src/bin/manyhands-cli/lifecycle_tests.rs`, and the library's
//! own tables pin the answers (`tests/mutation_remote/c5_lifecycle.rs`).

use std::{fs, path::Path, process::Command};

use manyhands::{
    repository::{FailurePoint, Mutation, TicketCloseInput},
    results::ResultCode,
};
use rusqlite::types::ValueRef;
use serde_json::Value;

use super::{c4_support::environment, mutation};
use crate::support::{
    self, FailOnce,
    cli::{self, CliEnv, CliRun},
    items,
    mutation::{
        DOCUMENT_NEW, DOCUMENT_NEW_PATH, REQUEST_1, REQUEST_2, REQUEST_3, SENTINELS, TITLE, World,
        execute_confirmed,
    },
};

const SAVE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX8";
const SLUG: &str = "mh-vh-k9x2b";
const BLOCKED: &str = "lifecycle_blocked.schema.json";
const PREVIEW: &str = "lifecycle_preview.schema.json";
const MUTATION: &str = "lifecycle_mutation.schema.json";
const CLOSED_BY_NAME: &str = "SENTINEL-CLOSED-BY-5e1d";
const CLOSED_BY_EMAIL: &str = "sentinel-closed-by-5e1d@example.invalid";

#[derive(Clone, Copy)]
struct Verb {
    resource: &'static str,
    name: &'static str,
    id: &'static str,
    golden: &'static str,
}

const PROMOTE: Verb = Verb {
    resource: "document",
    name: "promote",
    id: DOCUMENT_NEW,
    golden: "document_promote",
};
const CLOSE: Verb = Verb {
    resource: "ticket",
    name: "close",
    id: items::TICKET_A,
    golden: "ticket_close",
};
const BOTH: [Verb; 2] = [PROMOTE, CLOSE];

impl Verb {
    fn command(self) -> String {
        format!("{} {}", self.resource, self.name)
    }

    fn branch(self) -> String {
        if self.resource == "document" {
            World::document_branch(self.id)
        } else {
            World::branch(self.id)
        }
    }

    fn file(self) -> String {
        if self.resource == "document" {
            DOCUMENT_NEW_PATH.to_owned()
        } else {
            items::ticket_path(self.id)
        }
    }
}

/// The verb's item with an editing context primary does not hold.
fn saved(world: &World, verb: Verb) {
    let mutation = if verb.resource == "document" {
        world.document_create(verb.id, DOCUMENT_NEW_PATH)
    } else {
        let token = world.token(verb.id);
        world.save(verb.id, &token)
    };
    let answer = world.execute(SAVE, mutation);
    assert_eq!(answer.code, ResultCode::Ok, "{:?}", answer.data);
}

struct Case {
    world: World,
    env: CliEnv,
    verb: Verb,
}

impl Case {
    fn new(verb: Verb) -> Self {
        let world = World::new();
        saved(&world, verb);
        let env = environment(world.data.path());
        Self { world, env, verb }
    }

    fn root(&self) -> &str {
        self.world.root.to_str().unwrap()
    }

    fn args<'a>(&'a self, id: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
        let mut args = vec![
            self.verb.resource,
            self.verb.name,
            "--repo",
            self.root(),
            "--id",
            id,
        ];
        args.extend_from_slice(extra);
        args
    }

    fn json(&self, extra: &[&str], status: i32, code: &str, schema: Option<&str>) -> Value {
        let mut args = self.args(self.verb.id, extra);
        args.push("--json");
        let value = mutation::checked(&args, &self.env, &[], status, code, schema);
        assert_eq!(value["command"], self.verb.command());
        value
    }

    fn json_of(&self, resource: &str, verb: &str, extra: &[&str]) -> Value {
        let mut args = vec![resource, verb, "--repo", self.root()];
        args.extend_from_slice(extra);
        args.push("--json");
        mutation::checked(&args, &self.env, &[], 0, "ok", None)
    }

    fn text(&self, extra: &[&str]) -> CliRun {
        mutation::run(&self.args(self.verb.id, extra), &self.env, &[])
    }

    fn prepared(&self) -> Value {
        self.json(&["--prepare"], 0, "ok", Some(PREVIEW))
    }

    fn confirmation(&self) -> String {
        self.prepared()["data"]["confirmation_id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn confirmed(&self, request: &str, status: i32, code: &str, schema: Option<&str>) -> Value {
        let confirmation = self.confirmation();
        self.json(
            &["--request-id", request, "--confirm", &confirmation],
            status,
            code,
            schema,
        )
    }

    fn git(&self, args: &[&str]) -> String {
        let mut command = Command::new("git");
        cli::cli_environment(&mut command, &self.env);
        let output = command
            .arg("-C")
            .arg(&self.world.root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn refs(&self) -> String {
        self.git(&["for-each-ref", "refs/"])
    }

    fn merges(&self) -> usize {
        self.git(&["rev-list", "--merges", "main"]).lines().count()
    }

    fn forget_identity(&self) {
        let repository = git2::Repository::open(&self.world.root).unwrap();
        let config = repository.config().unwrap();
        let mut local = config.open_level(git2::ConfigLevel::Local).unwrap();
        local.remove("user.name").unwrap();
        local.remove("user.email").unwrap();
    }

    /// A commit on primary that changes the verb's file, as a peer's would.
    fn primary_moved(&self, source: &str) {
        items::write(&self.world.root, &self.verb.file(), source);
        items::commit(
            &self.world.fixture,
            &[&self.verb.file()],
            items::COMMITTED_AT + 60,
        );
    }

    /// Primary moved to the item's branch, as a merge outside Manyhands would.
    fn primary_holds_the_branch(&self) {
        self.git(&["merge", "--ff-only", "--quiet", &self.verb.branch()]);
    }
}

/// What a JSON golden replaces: the root and everything a run makes up.
fn golden(name: &str, value: &Value, root: &Path) {
    let mut replacements = vec![(
        fs::canonicalize(root).unwrap().to_str().unwrap().to_owned(),
        "<repository>",
    )];
    for (pointer, label) in [
        ("/operation_id", "<operation-id>"),
        ("/data/confirmation_id", "<confirmation-id>"),
        ("/data/expires_at", "<expires-at>"),
        ("/data/details/context_oid", "<context-oid>"),
        ("/data/details/primary_oid", "<primary-oid>"),
        ("/data/checkpoint_oid", "<checkpoint-oid>"),
        ("/data/integration_oid", "<commit-oid>"),
        ("/effects/commit_oid", "<commit-oid>"),
    ] {
        if let Some(text) = value.pointer(pointer).and_then(Value::as_str)
            && !replacements.iter().any(|(known, _)| known == text)
        {
            replacements.push((text.into(), label));
        }
    }
    cli::assert_cli_golden(
        name,
        value,
        &replacements
            .iter()
            .map(|(value, label)| (value.as_str(), *label))
            .collect::<Vec<_>>(),
    );
}

fn fixed_id(token: &str) -> bool {
    [
        items::TICKET_A,
        items::TICKET_C,
        DOCUMENT_NEW,
        REQUEST_1,
        REQUEST_2,
        REQUEST_3,
    ]
    .contains(&token)
}

fn ulid(token: &str) -> bool {
    token.len() == 26
        && token
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
}

fn scrubbed_token(token: &str) -> String {
    if token.len() == 40 && token.chars().all(|c| c.is_ascii_hexdigit()) {
        "<oid>".into()
    } else if ulid(token) && !fixed_id(token) {
        "<id>".into()
    } else if token.len() >= 20 && token.contains('T') && token.contains(':') {
        "<time>".into()
    } else {
        token.to_owned()
    }
}

/// Human output with what a run makes up replaced: commits, the IDs no
/// fixture fixes, the times, and the root.
fn scrubbed(text: &str, root: &Path) -> String {
    let root = root.canonicalize().unwrap();
    let text = text.replace(root.to_str().unwrap(), "<repository>");
    let mut out = String::new();
    let mut token = String::new();
    for character in text.chars() {
        if character.is_alphanumeric() || matches!(character, '-' | ':' | '.' | '+') {
            token.push(character);
        } else {
            out.push_str(&scrubbed_token(&token));
            token.clear();
            out.push(character);
        }
    }
    out.push_str(&scrubbed_token(&token));
    out
}

fn text_golden(name: &str, run: &CliRun, root: &Path) {
    let text = format!(
        "{}--- stderr\n{}--- exit {}\n",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
        run.status
    );
    cli::assert_cli_text_golden(name, &scrubbed(&text, root), &[]);
}

fn stdout(run: &CliRun) -> &str {
    std::str::from_utf8(&run.stdout).unwrap()
}

fn stderr(run: &CliRun) -> &str {
    std::str::from_utf8(&run.stderr).unwrap()
}

#[test]
fn a_preview_shows_every_effect_and_changes_nothing() {
    for verb in BOTH {
        let case = Case::new(verb);
        let refs = case.refs();
        let preview = case.prepared();
        assert_eq!(preview["data"]["details"]["remote"], Value::Null);
        assert_eq!(
            preview["data"]["details"]["publication"],
            "pending_no_remote"
        );
        golden(
            &format!("{}_preview", verb.golden),
            &preview,
            &case.world.root,
        );
        assert_eq!(case.refs(), refs);

        let human = case.text(&["--prepare"]);
        assert_eq!(human.status, 0, "{}", stderr(&human));
        let text = stdout(&human);
        for line in [
            "Slug: none\n",
            "Will merge the editing branch into primary.\n",
            "Will remove the editing worktree.\n",
            "Will delete the local editing branch.\n",
            "Confirmation: ",
            "--confirm ",
        ] {
            assert!(text.contains(line), "{line}: {text}");
        }
        assert_eq!(
            text.contains("Will record the closure on the editing branch.\n"),
            verb.name == "close",
            "{text}"
        );
        assert!(!text.contains("Will publish primary"), "{text}");
        text_golden(
            &format!("{}_preview_text", verb.golden),
            &human,
            &case.world.root,
        );
    }
}

#[test]
fn a_preview_of_a_ticket_with_a_slug_shows_it() {
    let world = World::new();
    items::write(
        &world.root,
        &items::ticket_path(items::TICKET_C),
        &items::ticket_source(items::TICKET_C, "Slugged", &format!("slug: {SLUG}\n")),
    );
    items::commit(
        &world.fixture,
        &[&items::ticket_path(items::TICKET_C)],
        items::COMMITTED_AT + 1,
    );
    items::refresh_completely(&world.service, &world.root);
    let token = world.token(items::TICKET_C);
    let answer = world.execute(SAVE, world.save(items::TICKET_C, &token));
    assert_eq!(answer.code, ResultCode::Ok, "{:?}", answer.data);
    let env = environment(world.data.path());
    let root = world.root.to_str().unwrap();
    let args = [
        "ticket",
        "close",
        "--repo",
        root,
        "--id",
        items::TICKET_C,
        "--prepare",
    ];
    let human = mutation::run(&args, &env, &[]);
    assert_eq!(human.status, 0, "{}", stderr(&human));
    assert!(
        stdout(&human).contains(&format!("Slug: {SLUG}\n")),
        "{}",
        stdout(&human)
    );
}

#[test]
fn local_only_completion_is_ok_with_publication_pending() {
    for verb in BOTH {
        let case = Case::new(verb);
        let done = case.confirmed(REQUEST_1, 0, "ok", Some(MUTATION));
        assert_eq!(done["outcome"], "success");
        assert_eq!(done["recovery"], serde_json::json!([]));
        assert_eq!(done["effects"]["publication"], "pending");
        assert_eq!(done["effects"]["integration"], "complete");
        assert_eq!(done["effects"]["cleanup"], "complete");
        assert_eq!(done["data"]["publication_pending"], "no_publication_remote");
        assert_eq!(done["data"]["integration"], "merged");
        golden(
            &format!("{}_local_only", verb.golden),
            &done,
            &case.world.root,
        );

        // The verb's effects on Git: one merge, and no context left.
        assert_eq!(case.merges(), 1);
        assert!(!case.refs().contains(&verb.branch()));
        assert!(!case.world.worktree(verb.id).exists());
        let source = fs::read_to_string(case.world.root.join(verb.file())).unwrap();
        assert_eq!(
            source.contains("\nclosed_at: "),
            verb.name == "close",
            "{source}"
        );
        if verb.name == "close" {
            assert!(
                source.contains("closed_by: Manyhands Test <manyhands-test@example.invalid>"),
                "{source}"
            );
        }

        // The same request gives the same answer, and nothing happens twice.
        let again = case.json(&["--request-id", REQUEST_1], 0, "ok", Some(MUTATION));
        assert_eq!(again, done);
        assert_eq!(case.merges(), 1);
    }
}

#[test]
fn local_only_completion_in_text_says_what_is_pending() {
    for verb in BOTH {
        let case = Case::new(verb);
        let confirmation = case.confirmation();
        let human = case.text(&["--request-id", REQUEST_1, "--confirm", &confirmation]);
        assert_eq!(human.status, 0, "{}", stderr(&human));
        assert!(stdout(&human).contains("Publication pending: no_publication_remote\n"));
        assert!(stderr(&human).contains("repo sync"), "{}", stderr(&human));
        text_golden(
            &format!("{}_local_only_text", verb.golden),
            &human,
            &case.world.root,
        );
    }
}

#[test]
fn a_closure_names_what_it_made_ready_with_slugs() {
    let world = World::new();
    let dependent = items::ticket_path(items::TICKET_C);
    items::write(
        &world.root,
        &dependent,
        &items::ticket_source(
            items::TICKET_C,
            "Dependent",
            &format!("slug: {SLUG}\ndeps: [{}]\n", items::TICKET_A),
        ),
    );
    items::commit(&world.fixture, &[&dependent], items::COMMITTED_AT + 1);
    items::refresh_completely(&world.service, &world.root);
    saved(&world, CLOSE);
    let env = environment(world.data.path());
    let case = Case {
        world,
        env,
        verb: CLOSE,
    };
    let confirmation = case.confirmation();
    let human = case.text(&["--request-id", REQUEST_1, "--confirm", &confirmation]);
    assert_eq!(human.status, 0, "{}", stderr(&human));
    assert!(
        stdout(&human).contains(&format!("made ready: {} {SLUG}\n", items::TICKET_C)),
        "{}",
        stdout(&human)
    );
    assert!(stdout(&human).contains("Made ready state: current\n"));
    text_golden("ticket_close_made_ready_text", &human, &case.world.root);
    let replay = case.json(&["--request-id", REQUEST_1], 0, "ok", Some(MUTATION));
    assert_eq!(
        replay["data"]["made_ready"],
        serde_json::json!([{"id": items::TICKET_C, "slug": SLUG}])
    );
    golden("ticket_close_made_ready", &replay, &case.world.root);
}

#[test]
fn a_dirty_primary_blocks_naming_the_blocker() {
    for verb in BOTH {
        let case = Case::new(verb);
        fs::write(case.world.root.join("dirty-primary.txt"), b"dirty\n").unwrap();
        let refs = case.refs();
        let blocked = case.json(&["--prepare"], 3, "recovery_required", Some(BLOCKED));
        assert_eq!(blocked["outcome"], "blocked");
        assert_eq!(
            blocked["data"]["blockers"],
            serde_json::json!(["primary_dirty"])
        );
        golden(
            &format!("{}_blocked", verb.golden),
            &blocked,
            &case.world.root,
        );

        let human = case.text(&["--prepare"]);
        assert_eq!(human.status, 3);
        assert!(human.stdout.is_empty());
        assert!(
            stderr(&human).contains("primary_dirty"),
            "{}",
            stderr(&human)
        );
        assert!(
            stderr(&human).contains("next: repo.inspect"),
            "{}",
            stderr(&human)
        );
        text_golden(
            &format!("{}_blocked_text", verb.golden),
            &human,
            &case.world.root,
        );
        assert_eq!(case.refs(), refs);
    }
}

#[test]
fn no_usable_identity_is_identity_required_with_its_action() {
    for verb in BOTH {
        let case = Case::new(verb);
        case.forget_identity();
        let refs = case.refs();
        let refused = case.json(&["--prepare"], 3, "identity_required", Some(BLOCKED));
        assert_eq!(refused["recovery"][0]["action"], "repo.identity_set");
        assert!(refused["request_id"].is_null());
        golden(
            &format!("{}_identity_required", verb.golden),
            &refused,
            &case.world.root,
        );

        let human = case.text(&["--prepare"]);
        assert_eq!(human.status, 3);
        assert!(
            stderr(&human).contains("next: repo.identity_set"),
            "{}",
            stderr(&human)
        );
        text_golden(
            &format!("{}_identity_required_text", verb.golden),
            &human,
            &case.world.root,
        );
        assert_eq!(case.refs(), refs);
    }
}

#[test]
fn input_failures_are_usage_and_id_errors_with_nothing_done() {
    for verb in BOTH {
        let case = Case::new(verb);
        let refs = case.refs();
        // No input, no ID, no request: the grammar's own answers.
        let with_input = case.json(
            &["--request-id", REQUEST_1, "--input", "-"],
            2,
            "usage_error",
            None,
        );
        assert!(with_input["request_id"].is_null());
        let args = [
            verb.resource,
            verb.name,
            "--repo",
            case.root(),
            "--prepare",
            "--json",
        ];
        let no_id = mutation::checked(&args, &case.env, &[], 2, "usage_error", None);
        assert_eq!(no_id["command"], verb.command());
        case.json(&[], 2, "usage_error", None);
        for extra in [
            vec!["--prepare", "--request-id", REQUEST_1],
            vec!["--prepare", "--confirm", REQUEST_2],
        ] {
            case.json(&extra, 2, "usage_error", None);
        }
        // A malformed ID and confirmation.
        let args = [
            verb.resource,
            verb.name,
            "--repo",
            case.root(),
            "--id",
            "not-a-ulid",
            "--prepare",
            "--json",
        ];
        let invalid = mutation::checked(&args, &case.env, &[], 2, "invalid_id", None);
        assert!(invalid["request_id"].is_null());
        golden(
            &format!("{}_invalid_id", verb.golden),
            &invalid,
            &case.world.root,
        );
        case.json(
            &["--request-id", REQUEST_1, "--confirm", "not-a-ulid"],
            2,
            "invalid_id",
            None,
        );
        // Consent is the verb's: none given, and one no preview made.
        case.json(
            &["--request-id", REQUEST_1],
            3,
            "confirmation_required",
            None,
        );
        case.json(
            &["--request-id", REQUEST_1, "--confirm", REQUEST_2],
            3,
            "confirmation_not_found",
            None,
        );
        // An item that isn't there, or isn't the verb's kind.
        let other = if verb.name == "close" {
            items::DOCUMENT_A
        } else {
            items::TICKET_A
        };
        for id in [items::TICKET_ABSENT, other] {
            let mut args = case.args(id, &["--prepare"]);
            args.push("--json");
            let refused = mutation::checked(&args, &case.env, &[], 2, "item_not_found", None);
            assert_eq!(refused["scope"]["item_id"], id);
        }
        let human = case.text(&["--prepare", "--request-id", REQUEST_1]);
        assert_eq!(human.status, 2);
        assert!(human.stdout.is_empty());
        assert_eq!(case.refs(), refs);
    }
}

#[test]
fn a_context_primary_already_holds_is_only_cleaned_up() {
    for verb in BOTH {
        let case = Case::new(verb);
        if verb.name == "close" {
            // A closure already on the branch: nothing is left to record.
            let path = verb.file();
            let worktree = case.world.worktree(verb.id);
            let source = fs::read_to_string(worktree.join(&path)).unwrap();
            let closed = source.replacen(
                "---\n",
                "---\nclosed_at: 2027-01-15T08:00:00Z\nclosed_by: Manyhands Test <manyhands-test@example.invalid>\n",
                1,
            );
            fs::write(worktree.join(&path), closed).unwrap();
            let mut git = Command::new("git");
            cli::cli_environment(&mut git, &case.env);
            let status = git
                .arg("-C")
                .arg(&worktree)
                .args([
                    "-c",
                    "user.name=Manyhands Test",
                    "-c",
                    "user.email=manyhands-test@example.invalid",
                    "commit",
                    "--quiet",
                    "--all",
                    "--message",
                    "closed outside",
                ])
                .status()
                .unwrap();
            assert!(status.success());
        }
        case.primary_holds_the_branch();
        let main = case.git(&["rev-parse", "main"]);
        let preview = case.prepared();
        assert_eq!(
            preview["data"]["details"]["integration"],
            "already_contained"
        );
        let confirmation = preview["data"]["confirmation_id"].as_str().unwrap();
        let human_preview = case.text(&["--prepare"]);
        assert!(
            !stdout(&human_preview).contains("Will merge the editing branch into primary.\n"),
            "{}",
            stdout(&human_preview)
        );
        let done = case.json(
            &["--request-id", REQUEST_1, "--confirm", confirmation],
            0,
            "ok",
            Some(MUTATION),
        );
        assert_eq!(done["recovery"], serde_json::json!([]));
        assert_eq!(done["data"]["integration"], "already_contained");
        golden(&format!("{}_noop", verb.golden), &done, &case.world.root);
        assert_eq!(case.git(&["rev-parse", "main"]), main, "no new commit");
        assert_eq!(case.merges(), 0);
        assert!(!case.refs().contains(&verb.branch()));
    }
}

#[test]
fn a_closed_ticket_is_ticket_closed_at_prepare() {
    let case = Case::new(CLOSE);
    case.confirmed(REQUEST_1, 0, "ok", Some(MUTATION));
    let refused = case.json(&["--prepare"], 3, "ticket_closed", Some(BLOCKED));
    assert_eq!(
        refused["data"]["blockers"],
        serde_json::json!(["already_closed"])
    );
}

#[test]
fn a_request_or_confirmation_for_another_item_is_a_mismatch() {
    for verb in BOTH {
        let case = Case::new(verb);
        let other = if verb.name == "close" {
            items::TICKET_B
        } else {
            items::DOCUMENT_A
        };
        // A confirmation of this item, presented for another.
        let confirmation = case.confirmation();
        let mut args = case.args(
            other,
            &["--request-id", REQUEST_2, "--confirm", &confirmation],
        );
        args.push("--json");
        mutation::checked(&args, &case.env, &[], 2, "confirmation_mismatch", None);

        // A finished request, sent again for another item.
        let done = case.confirmed(REQUEST_1, 0, "ok", Some(MUTATION));
        let mut args = case.args(other, &["--request-id", REQUEST_1]);
        args.push("--json");
        let refused = mutation::checked(&args, &case.env, &[], 2, "request_mismatch", None);
        assert_eq!(refused["recovery"], serde_json::json!([]));
        golden(
            &format!("{}_request_mismatch", verb.golden),
            &refused,
            &case.world.root,
        );
        let human = mutation::run(
            &case.args(other, &["--request-id", REQUEST_1]),
            &case.env,
            &[],
        );
        assert_eq!(human.status, 2);
        text_golden(
            &format!("{}_request_mismatch_text", verb.golden),
            &human,
            &case.world.root,
        );
        // And the request still answers what it did.
        let replay = case.json(&["--request-id", REQUEST_1], 0, "ok", Some(MUTATION));
        assert_eq!(replay, done);
    }
}

#[test]
fn a_primary_moved_after_the_preview_is_external_change() {
    for verb in BOTH {
        let case = Case::new(verb);
        let confirmation = case.confirmation();
        fs::write(case.world.root.join("moved.txt"), b"moved\n").unwrap();
        items::commit(
            &case.world.fixture,
            &["moved.txt"],
            items::COMMITTED_AT + 60,
        );
        let refused = case.json(
            &["--request-id", REQUEST_1, "--confirm", &confirmation],
            3,
            "external_change",
            None,
        );
        assert_eq!(case.merges(), 0, "{refused}");
    }
}

#[test]
fn a_blocked_prepare_offers_its_way_out() {
    for verb in BOTH {
        let case = Case::new(verb);
        let conflicting = if verb.resource == "document" {
            items::document_source(verb.id, "Moved on primary", "")
        } else {
            items::ticket_source(verb.id, "Moved on primary", "")
        };
        case.primary_moved(&conflicting);
        let blocked = case.json(&["--prepare"], 3, "recovery_required", Some(BLOCKED));
        assert_eq!(
            blocked["data"]["blockers"],
            serde_json::json!(["integration_conflict"]),
            "{blocked}"
        );
        let human = case.text(&["--prepare"]);
        assert_eq!(human.status, 3);
        let text = stderr(&human);
        assert!(
            text.contains("next: manyhands-cli context observe-local "),
            "{text}"
        );
        assert!(text.contains("Then prepare again.\n"), "{text}");
        text_golden(
            &format!("{}_prepare_conflict_text", verb.golden),
            &human,
            &case.world.root,
        );
    }
}

#[test]
fn a_missing_context_names_the_save_that_makes_one() {
    for (verb, save) in [(PROMOTE, "document save"), (CLOSE, "ticket save")] {
        let case = Case::new(verb);
        case.primary_holds_the_branch();
        let worktree = case.world.worktree(verb.id);
        case.git(&["worktree", "remove", "--force", worktree.to_str().unwrap()]);
        case.git(&[
            "branch",
            "-D",
            "--quiet",
            verb.branch().trim_start_matches("refs/heads/"),
        ]);
        case.json_of("index", "refresh", &["--request-id", REQUEST_2]);
        let blocked = case.json(&["--prepare"], 3, "recovery_required", Some(BLOCKED));
        assert_eq!(
            blocked["data"]["blockers"],
            serde_json::json!(["context_missing"])
        );
        let human = case.text(&["--prepare"]);
        assert_eq!(human.status, 3);
        let text = stderr(&human);
        assert!(
            text.contains(&format!("next: manyhands-cli {save} ")),
            "{text}"
        );
        assert!(text.contains("Then prepare again.\n"), "{text}");
        text_golden(
            &format!("{}_prepare_context_missing_text", verb.golden),
            &human,
            &case.world.root,
        );
    }
}

#[test]
fn a_no_op_in_text_says_what_it_found() {
    let case = Case::new(PROMOTE);
    case.primary_holds_the_branch();
    let confirmation = case.confirmation();
    let human = case.text(&["--request-id", REQUEST_1, "--confirm", &confirmation]);
    assert_eq!(human.status, 0, "{}", stderr(&human));
    assert!(
        stdout(&human).contains("Integration: already_contained\n"),
        "{}",
        stdout(&human)
    );
    text_golden("document_promote_noop_text", &human, &case.world.root);
}

/// Every cell of application data that holds `sentinels`, as `table.column`.
fn cells_holding(data: &Path, sentinels: &[&str]) -> Vec<String> {
    let connection = items::index(data);
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(tables.iter().any(|table| table == "request_records"));
    assert!(tables.iter().any(|table| table == "confirmation_records"));
    assert!(tables.iter().any(|table| table == "ticket_closures"));
    assert!(tables.iter().any(|table| table == "document_promotions"));
    let mut found = Vec::new();
    for table in tables {
        let columns: Vec<String> = connection
            .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
            .unwrap()
            .query_map([&table], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let mut statement = connection
            .prepare(&format!("SELECT * FROM \"{table}\""))
            .unwrap();
        let mut rows = statement.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for (index, column) in columns.iter().enumerate() {
                let bytes: Vec<u8> = match row.get_ref(index).unwrap() {
                    ValueRef::Text(bytes) | ValueRef::Blob(bytes) => bytes.to_vec(),
                    _ => continue,
                };
                let text = String::from_utf8_lossy(&bytes);
                if sentinels.iter().any(|sentinel| text.contains(sentinel)) {
                    found.push(format!("{table}.{column}"));
                }
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Cycle 10: `closed_by` is in the preview, canonical content, the index and
/// Git, and a title in content and the index. Nothing else keeps either. Each
/// `expected` sentinel must be in the index, or the scan is reading nothing.
fn assert_only_the_index_holds(data: &Path, expected: &[&str]) {
    let all = [CLOSED_BY_NAME, CLOSED_BY_EMAIL, SENTINELS[0]];
    let beyond: Vec<_> = cells_holding(data, &all)
        .into_iter()
        .filter(|cell| !cell.starts_with("discovered_items."))
        .collect();
    assert!(beyond.is_empty(), "{beyond:?}");
    for sentinel in expected {
        assert!(
            cells_holding(data, &[sentinel])
                .iter()
                .any(|cell| cell.starts_with("discovered_items.")),
            "the index holds no {sentinel}"
        );
    }
}

#[test]
fn application_data_holds_no_closed_by_or_title_beyond_the_index() {
    for verb in BOTH {
        let case = Case::new(verb);
        case.world.set_local_config("user.name", CLOSED_BY_NAME);
        case.world.set_local_config("user.email", CLOSED_BY_EMAIL);
        let title = SENTINELS[0];
        assert!(TITLE.contains(title));
        let data = case.world.data.path();
        let preview = case.prepared();
        let shown = serde_json::to_string(&preview).unwrap();
        assert_eq!(
            shown.contains(CLOSED_BY_NAME),
            verb.name == "close",
            "{shown}"
        );
        assert!(!shown.contains(title), "{shown}");
        assert_only_the_index_holds(data, &[title]);
        let confirmation = preview["data"]["confirmation_id"].as_str().unwrap();
        let done = case.json(
            &["--request-id", REQUEST_1, "--confirm", confirmation],
            0,
            "ok",
            Some(MUTATION),
        );
        let answered = serde_json::to_string(&done).unwrap();
        assert!(!answered.contains(CLOSED_BY_NAME), "{answered}");
        assert!(!answered.contains(CLOSED_BY_EMAIL), "{answered}");
        assert!(!answered.contains(title), "{answered}");
        if verb.name == "close" {
            assert_only_the_index_holds(data, &[title, CLOSED_BY_NAME, CLOSED_BY_EMAIL]);
        } else {
            assert_only_the_index_holds(data, &[title]);
        }
    }
}

/// A closure stopped past its checkpoint, whose merge primary then refuses.
struct Refused {
    case: Case,
    operation: String,
}

impl Refused {
    fn new() -> Self {
        Self::with_identity(None)
    }

    fn with_identity(identity: Option<(&str, &str)>) -> Self {
        let mut world = World::with_service(|data| {
            FailOnce::at(FailurePoint::AfterClosureCheckpoint).open_service(data)
        });
        saved(&world, CLOSE);
        if let Some((name, email)) = identity {
            world.set_local_config("user.name", name);
            world.set_local_config("user.email", email);
        }
        let mutation = || {
            Mutation::TicketClose(TicketCloseInput {
                root: world.root.clone(),
                id: items::item_id(items::TICKET_A),
            })
        };
        let preview = world.service.prepare(&mutation());
        let confirmation = preview
            .data
            .as_ref()
            .and_then(|data| data.preview())
            .unwrap()
            .confirmation_id
            .clone();
        let stopped = execute_confirmed(&world.service, REQUEST_1, &confirmation, mutation());
        assert_ne!(stopped.code, ResultCode::Ok, "{:?}", stopped.data);
        let operation = stopped.operation_id.clone().unwrap();
        world.reopen();
        let env = environment(world.data.path());
        let case = Case {
            world,
            env,
            verb: CLOSE,
        };
        case.primary_moved(&items::ticket_source(
            items::TICKET_A,
            "Moved on primary",
            "",
        ));
        Self { case, operation }
    }
}

#[test]
fn a_stop_past_the_checkpoint_keeps_closed_by_and_title_to_the_index() {
    let refused = Refused::with_identity(Some((CLOSED_BY_NAME, CLOSED_BY_EMAIL)));
    let case = &refused.case;
    case.json(
        &["--request-id", REQUEST_1],
        4,
        "recovery_required",
        Some(MUTATION),
    );
    assert_only_the_index_holds(case.world.data.path(), &[SENTINELS[0]]);
}

#[test]
fn a_refused_checkpoint_merge_says_how_to_repair_it_after_retrying() {
    let refused = Refused::new();
    let case = &refused.case;
    let main = case.git(&["rev-parse", "main"]);
    let answer = case.json(
        &["--request-id", REQUEST_1],
        4,
        "recovery_required",
        Some(MUTATION),
    );
    assert_eq!(answer["outcome"], "partial");
    assert_eq!(answer["effects"]["checkpoint"], "committed");
    assert_eq!(answer["effects"]["integration"], "not_requested");
    assert_eq!(
        answer["data"]["blockers"],
        serde_json::json!(["integration_conflict"])
    );
    assert_eq!(answer["recovery"][0]["action"], "request.retry");
    assert_eq!(answer["recovery"][1]["action"], "repo.inspect");
    golden(
        "ticket_close_checkpoint_merge_refused",
        &answer,
        &case.world.root,
    );

    let human = case.text(&["--request-id", REQUEST_1]);
    assert_eq!(human.status, 4);
    assert!(human.stdout.is_empty());
    let text = stderr(&human);
    // Named blockers won't change by themselves, so the driver doesn't repeat it.
    assert_eq!(text.matches("progress: retrying\n").count(), 0, "{text}");
    for verb in [
        "context observe-local",
        "context sync-local",
        "context show-local",
        "context resolve-local",
    ] {
        assert!(
            text.contains(&format!("next: manyhands-cli {verb} ")),
            "{verb}: {text}"
        );
    }
    let repair = text.find("context resolve-local").unwrap();
    let retry = text.find("next: request.retry").unwrap();
    assert!(repair < retry, "the repair comes before the retry: {text}");
    assert!(text.contains("Blockers: integration_conflict\n"), "{text}");
    text_golden(
        "ticket_close_checkpoint_merge_refused_text",
        &human,
        &case.world.root,
    );
    assert_eq!(case.git(&["rev-parse", "main"]), main);
    assert_eq!(case.merges(), 0);
}

/// `operation resume` of a parent a request holds: the conflict read
/// `collaboration::seen` makes for a resume finds no conflict under a parent's
/// ID, and adds nothing, in place of failing.
#[test]
fn a_resume_of_a_parent_a_request_holds_names_the_request_and_nothing_else() {
    let refused = Refused::new();
    let case = &refused.case;
    let args = |json: bool| {
        let mut args = vec![
            "operation",
            "resume",
            "--repo",
            case.root(),
            "--operation-id",
            refused.operation.as_str(),
            "--request-id",
            REQUEST_2,
        ];
        if json {
            args.push("--json");
        }
        args
    };
    let value = mutation::checked(
        &args(true),
        &case.env,
        &[],
        3,
        "original_request_required",
        None,
    );
    assert_eq!(value["recovery"][0]["arguments"]["request_id"], REQUEST_1);
    let human = mutation::run(
        &{
            let mut text = args(false);
            text[7] = REQUEST_3;
            text
        },
        &case.env,
        &[],
    );
    assert_eq!(human.status, 3);
    assert!(human.stdout.is_empty());
    let text = stderr(&human);
    assert!(
        text.contains(&format!(
            "next: send request {REQUEST_1} again with its original command and input\n"
        )),
        "{text}"
    );
    assert!(
        !text.contains("note:"),
        "no supersede note, no read's failure: {text}"
    );
    assert!(!text.contains("internal"), "{text}");
    // Nothing was continued.
    assert_eq!(case.merges(), 0);
}

#[test]
fn a_repository_never_enabled_is_refused() {
    for verb in BOTH {
        let temp = tempfile::tempdir().unwrap();
        let env = CliEnv::new(temp.path().join("absent-data"));
        let unregistered = support::born_repository();
        let root = unregistered.root.to_str().unwrap();
        let args = [
            verb.resource,
            verb.name,
            "--repo",
            root,
            "--id",
            verb.id,
            "--prepare",
            "--json",
        ];
        let value = mutation::checked(&args, &env, &[], 2, "repository_not_enabled", None);
        assert_eq!(value["command"], verb.command());
    }
}
