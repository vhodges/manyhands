//! C3 repair, consent and recovery through isolated real CLI children.
use std::{fs, path::PathBuf};

#[cfg(unix)]
use crate::support::pty::PtyCli;
#[cfg(unix)]
use std::ffi::OsStr;

use manyhands::{
    repository::{DocumentRepairInput, FailurePoint, Mutation, RepositoryService, RequestState},
    results::{FailureClass, Outcome, ResultCode},
};
use serde_json::{Value, json};

use super::{index, mutation};
use crate::support::{
    self, cli, confirmation, items,
    mutation::{REQUEST_1, REQUEST_2, REQUEST_3},
    repair::{self, ADOPTED, MARKER_PATH, Shelf, TICKET_MARKER},
};

pub struct Fixture {
    pub shelf: Shelf,
    pub env: cli::CliEnv,
}

impl Fixture {
    pub fn new() -> Self {
        let shelf = Shelf::new();
        let env = cli::CliEnv::new(shelf.data.path().to_owned());
        fs::write(
            env.home.path().join(".gitconfig"),
            "[core]\n\tautocrlf = false\n",
        )
        .unwrap();
        Self { shelf, env }
    }

    fn args<'a>(&'a self, resource: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
        let mut args = vec![
            resource,
            "repair",
            "--repo",
            self.shelf.root.to_str().unwrap(),
            "--input",
            "-",
        ];
        args.extend_from_slice(extra);
        args
    }

    fn token(&self, path: &str) -> String {
        mutation::checked(
            &[
                "path",
                "observe",
                "--repo",
                self.shelf.root.to_str().unwrap(),
                "--path",
                path,
                "--json",
            ],
            &self.env,
            &[],
            0,
            "ok",
            Some("path_observation.schema.json"),
        )["data"]["observation"]
            .as_str()
            .unwrap()
            .into()
    }

    fn input(&self, path: &str, source: &str) -> Value {
        json!({"path":path,"observation":self.token(path),"source":source})
    }

    pub fn example(&self, resource: &str) -> Value {
        let bytes: &[u8] = match resource {
            "document" => include_bytes!("../fixtures/cli_input_v1/document_repair.json"),
            "ticket" => include_bytes!("../fixtures/cli_input_v1/ticket_repair.json"),
            _ => unreachable!(),
        };
        let mut value: Value = serde_json::from_slice(bytes).unwrap();
        value["observation"] = self.token(value["path"].as_str().unwrap()).into();
        value
    }

    pub fn call(
        &self,
        resource: &str,
        extra: &[&str],
        input: &Value,
        exit: i32,
        code: &str,
    ) -> Value {
        let mut args = self.args(resource, extra);
        args.push("--json");
        let out = mutation::run(&args, &self.env, &serde_json::to_vec(input).unwrap());
        private(&out, "");
        let value = cli::json(&out);
        assert_eq!(out.status, exit, "{value}");
        assert_eq!(value["code"], code, "{value}");
        assert_eq!(value["command"], format!("{resource} repair"));
        support::schema::check_published("envelope.schema.json", &value).unwrap();
        if !value["data"].is_null() {
            let schema = if extra.contains(&"--prepare") {
                "repair_preview.schema.json"
            } else {
                "repair_mutation.schema.json"
            };
            support::schema::check_published(schema, &value["data"]).unwrap();
        }
        value
    }

    pub fn prepare(&self, resource: &str, input: &Value) -> Value {
        self.call(resource, &["--prepare"], input, 0, "ok")
    }

    fn confirm(&self, resource: &str, input: &Value, consent: &str, request: &str) -> Value {
        self.call(
            resource,
            &["--confirm", consent, "--request-id", request],
            input,
            0,
            "ok",
        )
    }

    fn snapshot(&self) -> support::RepositoryAndWorktreeSnapshot {
        support::repository_and_worktree_snapshot(&self.shelf.fixture)
    }

    fn unchanged(&self, before: &support::RepositoryAndWorktreeSnapshot) {
        assert_eq!(&self.snapshot(), before);
        assert!(self.shelf.contexts().is_empty());
        assert!(self.shelf.item_branches().is_empty());
    }

    fn checkpoint(&self, id: &str, input: &Value, done: &Value) {
        assert_eq!(done["data"]["id"], id);
        assert_eq!(done["effects"]["checkpoint"], "committed");
        assert_eq!(done["effects"]["publication"], "not_requested");
        assert_eq!(self.shelf.branch_commits(id), 1);
        assert_eq!(
            done["effects"]["commit_oid"],
            self.shelf.branch_tip(id).unwrap().to_string()
        );
        assert_eq!(
            self.shelf
                .context(id, input["path"].as_str().unwrap())
                .unwrap(),
            input["source"].as_str().unwrap().as_bytes()
        );
    }

    fn golden(&self, name: &str, value: &Value) {
        let mut pairs = vec![(
            self.shelf.root.to_string_lossy().into_owned(),
            "<repository>",
        )];
        for (pointer, label) in [
            ("/request_id", "<request-id>"),
            ("/operation_id", "<operation-id>"),
            ("/effects/commit_oid", "<commit-oid>"),
            ("/data/confirmation_id", "<confirmation-id>"),
            ("/data/expires_at", "<expires-at>"),
        ] {
            if let Some(text) = value.pointer(pointer).and_then(Value::as_str) {
                pairs.push((text.to_owned(), label));
            }
        }
        cli::assert_cli_golden(
            name,
            value,
            &pairs
                .iter()
                .map(|(v, l)| (v.as_str(), *l))
                .collect::<Vec<_>>(),
        );
    }
}

fn private(out: &cli::CliRun, terminal: &str) {
    for text in [
        std::str::from_utf8(&out.stdout).unwrap(),
        std::str::from_utf8(&out.stderr).unwrap(),
        terminal,
    ] {
        for sentinel in repair::SENTINELS {
            assert!(!text.contains(sentinel), "repair source leaked");
        }
    }
}

fn consent(preview: &Value) -> &str {
    preview["data"]["confirmation_id"].as_str().unwrap()
}

#[test]
fn prepare_then_confirm_adopts_the_supplied_id() {
    let f = Fixture::new();
    let input = f.example("document");
    let primary = f.shelf.primary(MARKER_PATH);
    let head = f.shelf.primary_head();
    let before = f.snapshot();
    let preview = f.prepare("document", &input);
    assert_eq!(preview["data"]["details"]["adoption"], true);
    assert_eq!(preview["data"]["details"]["id"], ADOPTED);
    assert!(preview["request_id"].is_null());
    f.unchanged(&before);
    f.golden("document_repair_preview", &preview);
    let done = f.confirm("document", &input, consent(&preview), REQUEST_1);
    assert_eq!(done["request_id"], REQUEST_1);
    assert_eq!(done["data"]["adopted"], true);
    f.checkpoint(ADOPTED, &input, &done);
    assert_eq!(f.shelf.contexts(), [ADOPTED]);
    assert_eq!(f.shelf.primary(MARKER_PATH), primary);
    assert_eq!(f.shelf.primary_head(), head);
    f.golden("document_repair_ok", &done);
}

#[test]
fn ticket_repair_prepare_and_confirm() {
    let f = Fixture::new();
    let input = f.example("ticket");
    let before = f.snapshot();
    let primary = f.shelf.primary(input["path"].as_str().unwrap());
    let head = f.shelf.primary_head();
    let preview = f.prepare("ticket", &input);
    assert_eq!(preview["data"]["details"]["adoption"], true);
    assert_eq!(preview["data"]["details"]["id"], TICKET_MARKER);
    f.unchanged(&before);
    f.golden("ticket_repair_preview", &preview);
    let done = f.confirm("ticket", &input, consent(&preview), REQUEST_1);
    assert_eq!(done["data"]["adopted"], true);
    f.checkpoint(TICKET_MARKER, &input, &done);
    assert_eq!(f.shelf.primary(input["path"].as_str().unwrap()), primary);
    assert_eq!(f.shelf.primary_head(), head);
    f.golden("ticket_repair_ok", &done);
}

#[test]
fn a_stale_token_at_confirm_is_external_change() {
    for resource in ["document", "ticket"] {
        let f = Fixture::new();
        let input = f.example(resource);
        let preview = f.prepare(resource, &input);
        let path = f.shelf.root.join(input["path"].as_str().unwrap());
        let mut bytes = fs::read(&path).unwrap();
        bytes.extend_from_slice(b"Third party edit\n");
        fs::write(path, bytes).unwrap();
        let before = f.snapshot();
        let value = f.call(
            resource,
            &["--confirm", consent(&preview), "--request-id", REQUEST_1],
            &input,
            3,
            "external_change",
        );
        f.unchanged(&before);
        if resource == "ticket" {
            f.golden("ticket_repair_external_change", &value);
        }
    }
}

#[test]
fn an_invalid_corrected_source_is_refused_at_prepare() {
    for resource in ["document", "ticket"] {
        let f = Fixture::new();
        let wrong_kind = if resource == "document" {
            repair::ticket(ADOPTED, "")
        } else {
            repair::document(TICKET_MARKER)
        };
        for source in [
            "not managed",
            "---\n: : :\n\t- [\n---\n",
            "---\nmanyhands_managed: true\n---\n",
            &wrong_kind,
        ] {
            let mut input = f.example(resource);
            input["source"] = source.into();
            let before = f.snapshot();
            f.call(resource, &["--prepare"], &input, 2, "invalid_input");
            f.unchanged(&before);
        }
    }
}

#[test]
fn a_path_with_no_nonconforming_entry_is_refused() {
    let f = Fixture::new();
    for (resource, path, source, code) in [
        (
            "document",
            "docs/guide.md".to_owned(),
            repair::document(items::DOCUMENT_A),
            "not_repairable",
        ),
        (
            "document",
            "docs/missing.md".to_owned(),
            repair::document(ADOPTED),
            "path_not_found",
        ),
        (
            "document",
            repair::OUTSIDE_PATH.to_owned(),
            repair::document(ADOPTED),
            "not_repairable",
        ),
        (
            "document",
            "../outside.md".to_owned(),
            repair::document(ADOPTED),
            "invalid_path",
        ),
        (
            "ticket",
            items::ticket_path(TICKET_MARKER),
            repair::ticket(repair::UNHELD, ""),
            "not_repairable",
        ),
        (
            "document",
            repair::MISPLACED_PATH.to_owned(),
            repair::document(repair::MISPLACED),
            "not_repairable",
        ),
    ] {
        // Existing canonical files need their real token to reach the content
        // refusal; invalid/non-place paths are refused before observation.
        let token = if path.starts_with("docs/") {
            f.token(&path)
        } else {
            "v1:unread".into()
        };
        let input = json!({"path":path,"observation":token,"source":source});
        let before = f.snapshot();
        f.call(resource, &["--prepare"], &input, 2, code);
        f.unchanged(&before);
    }
}

#[test]
fn an_id_another_item_holds_is_identity_ambiguous_and_makes_no_context() {
    let f = Fixture::new();
    let input = f.input(MARKER_PATH, &repair::document(items::DOCUMENT_A));
    let before = f.snapshot();
    let value = f.call("document", &["--prepare"], &input, 3, "identity_ambiguous");
    f.unchanged(&before);
    f.golden("document_repair_identity_ambiguous", &value);
}

#[test]
fn a_closed_ticket_is_repaired_keeping_its_closure() {
    let f = Fixture::new();
    let id = repair::TICKET_CLOSED_BROKEN;
    let path = items::ticket_path(id);
    for source in [
        repair::ticket(id, ""),
        repair::ticket(id, &items::CLOSURE.replace("12:34:56", "12:34:57")),
    ] {
        let input = f.input(&path, &source);
        let before = f.snapshot();
        f.call("ticket", &["--prepare"], &input, 2, "invalid_input");
        f.unchanged(&before);
    }
    let input = f.input(&path, &repair::ticket(id, items::CLOSURE));
    let primary = f.shelf.primary(&path);
    let preview = f.prepare("ticket", &input);
    let done = f.confirm("ticket", &input, consent(&preview), REQUEST_1);
    assert_eq!(done["data"]["adopted"], false);
    f.checkpoint(id, &input, &done);
    assert_eq!(f.shelf.primary(&path), primary);
    let shown = mutation::checked(
        &[
            "ticket",
            "show",
            "--repo",
            f.shelf.root.to_str().unwrap(),
            "--id",
            id,
            "--json",
        ],
        &f.env,
        &[],
        0,
        "ok",
        Some("item.schema.json"),
    );
    assert!(!shown["data"]["closure"].is_null());
    let bytes = String::from_utf8(f.shelf.context(id, &path).unwrap()).unwrap();
    assert!(bytes.contains(items::CLOSURE));
}

#[test]
fn a_repair_interrupted_after_its_write_is_completed_by_the_same_request() {
    let f = Fixture::new();
    let input = f.example("document");
    let primary = f.shelf.primary(MARKER_PATH);
    let (confirmation, operation) = {
        let actor = support::FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence)
            .open_service(&f.env.data);
        let mutation = || {
            Mutation::DocumentRepair(DocumentRepairInput {
                root: f.shelf.root.clone(),
                path: PathBuf::from(MARKER_PATH),
                observation: input["observation"].as_str().unwrap().into(),
                source: input["source"].as_str().unwrap().into(),
            })
        };
        let preview = confirmation::prepare_with(&actor, &mutation());
        assert_eq!(preview.code, ResultCode::Ok);
        let confirmation = preview.data.unwrap().confirmation_id;
        let stopped =
            support::mutation::execute_confirmed(&actor, REQUEST_1, &confirmation, mutation());
        assert_eq!(stopped.code, ResultCode::InternalError);
        assert_eq!(stopped.outcome, Outcome::Partial);
        // Owner Fact 19: this actor has no process exit. Its envelope maps to 4.
        assert_eq!(stopped.failure_class(), Some(FailureClass::Incomplete));
        assert_eq!(stopped.effects.write.as_str(), "written");
        assert_eq!(stopped.effects.checkpoint.as_str(), "pending");
        assert_eq!(repair::repair_data(&stopped).id, ADOPTED);
        assert_eq!(f.shelf.branch_commits(ADOPTED), 0);
        assert_eq!(
            f.shelf.record(REQUEST_1).unwrap().state,
            RequestState::Accepted
        );
        (confirmation, stopped.operation_id.unwrap())
    };
    let done = f.confirm("document", &input, &confirmation, REQUEST_1);
    assert_eq!(done["operation_id"], operation);
    f.checkpoint(ADOPTED, &input, &done);
    assert_eq!(f.shelf.contexts(), [ADOPTED]);
    assert_eq!(f.shelf.pending_journal_rows(), 0);
    assert_eq!(
        f.shelf.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
    let before = f.snapshot();
    assert_eq!(
        f.confirm("document", &input, &confirmation, REQUEST_1),
        done
    );
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.shelf.branch_commits(ADOPTED), 1);
    assert_eq!(f.shelf.primary(MARKER_PATH), primary);
}

#[test]
fn a_confirmation_presented_after_cache_loss_is_confirmation_not_found() {
    // This fixture owns no long-lived service, so all SQLite handles are closed
    // before sidecars and the database are removed, including on Windows.
    let f = index::Fixture::new();
    items::write(&f.repo.root, MARKER_PATH, &repair::marker_only());
    items::commit(&f.repo, &[MARKER_PATH], items::COMMITTED_AT + 1);
    {
        let service = RepositoryService::open_at(&f.env.data).unwrap();
        items::refresh_completely(&service, &f.repo.root);
    }
    let token = f.checked("path", "observe", &["--path", MARKER_PATH], &[], 0, "ok");
    let input = serde_json::to_vec(&json!({"path":MARKER_PATH,"observation":token["data"]["observation"],"source":repair::document(ADOPTED)})).unwrap();
    let preview = f.checked(
        "document",
        "repair",
        &["--prepare", "--input", "-"],
        &input,
        0,
        "ok",
    );
    let before = support::repository_and_worktree_snapshot(&f.repo);
    for suffix in ["-wal", "-shm", ""] {
        let path = f.env.data.join(format!("manyhands.sqlite3{suffix}"));
        if path.exists() {
            fs::remove_file(path).unwrap();
        }
    }
    f.checked(
        "index",
        "rebuild",
        &["--request-id", REQUEST_2],
        &[],
        0,
        "ok",
    );
    let value = f.checked(
        "document",
        "repair",
        &[
            "--confirm",
            consent(&preview),
            "--request-id",
            REQUEST_1,
            "--input",
            "-",
        ],
        &input,
        3,
        "confirmation_not_found",
    );
    assert!(
        value["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["action"] == "request.prepare")
    );
    assert_eq!(support::repository_and_worktree_snapshot(&f.repo), before);
    support::mutation::assert_request_stores_exclude(&f.env.data, &repair::SENTINELS);
}

#[cfg(unix)]
#[test]
fn interactive_consent_reads_the_source_from_stdin_and_the_answer_from_the_terminal() {
    for resource in ["document", "ticket"] {
        let f = Fixture::new();
        let input = f.example(resource);
        let before = f.snapshot();
        for answer in ["n\n", "y\n"] {
            let args = f.args(resource, &["--request-id", REQUEST_1]);
            let mut p = PtyCli::spawn(
                &args.iter().map(OsStr::new).collect::<Vec<_>>(),
                &f.env,
                &serde_json::to_vec(&input).unwrap(),
            );
            p.expect("Proceed? [y/N] ");
            p.send(answer.as_bytes());
            let (out, transcript) = p.finish();
            private(&out, &transcript);
            assert!(transcript.contains(&format!("Preview of {resource} repair")));
            if answer == "n\n" {
                assert_eq!(out.status, 130);
                assert!(out.stdout.is_empty());
                assert!(
                    String::from_utf8(out.stderr)
                        .unwrap()
                        .contains("code: cancelled")
                );
                f.unchanged(&before);
                assert!(f.shelf.record(REQUEST_1).is_none());
            } else {
                assert_eq!(out.status, 0);
                let id = if resource == "document" {
                    ADOPTED
                } else {
                    TICKET_MARKER
                };
                assert_eq!(f.shelf.branch_commits(id), 1);
                assert_eq!(
                    f.shelf
                        .context(id, input["path"].as_str().unwrap())
                        .unwrap(),
                    input["source"].as_str().unwrap().as_bytes()
                );
            }
        }
    }
}

#[test]
fn without_a_terminal_a_repair_needs_a_confirmation() {
    for resource in ["document", "ticket"] {
        let f = Fixture::new();
        let input = f.example(resource);
        let before = f.snapshot();
        f.call(
            resource,
            &["--request-id", REQUEST_1],
            &input,
            3,
            "confirmation_required",
        );
        f.unchanged(&before);
    }
}

#[test]
fn each_repair_replays_a_finished_request() {
    for resource in ["document", "ticket"] {
        let f = Fixture::new();
        let input = f.example(resource);
        let preview = f.prepare(resource, &input);
        let done = f.confirm(resource, &input, consent(&preview), REQUEST_1);
        let before = f.snapshot();
        assert_eq!(
            f.confirm(resource, &input, consent(&preview), REQUEST_1),
            done
        );
        assert_eq!(f.snapshot(), before);
        let id = if resource == "document" {
            ADOPTED
        } else {
            TICKET_MARKER
        };
        f.checkpoint(id, &input, &done);
    }
}

#[test]
fn repair_confirmations_refuse_changed_input_and_another_request() {
    let f = Fixture::new();
    let input = f.example("document");
    let preview = f.prepare("document", &input);
    let mut changed = input.clone();
    changed["source"] = format!("{}new body\n", input["source"].as_str().unwrap()).into();
    let before = f.snapshot();
    f.call(
        "document",
        &["--confirm", consent(&preview), "--request-id", REQUEST_1],
        &changed,
        2,
        "confirmation_mismatch",
    );
    f.unchanged(&before);
    f.confirm("document", &input, consent(&preview), REQUEST_1);
    let before = f.snapshot();
    f.call(
        "document",
        &["--confirm", consent(&preview), "--request-id", REQUEST_2],
        &input,
        3,
        "confirmation_used",
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn each_repair_has_a_fresh_request_noop_with_its_own_confirmation() {
    for resource in ["document", "ticket"] {
        let f = Fixture::new();
        let input = f.example(resource);
        let first = f.prepare(resource, &input);
        let second = f.prepare(resource, &input);
        assert_ne!(consent(&first), consent(&second));
        f.confirm(resource, &input, consent(&first), REQUEST_1);
        let before = f.snapshot();
        let noop = f.call(
            resource,
            &["--confirm", consent(&second), "--request-id", REQUEST_2],
            &input,
            0,
            "already_applied",
        );
        assert_eq!(noop["outcome"], "noop");
        assert_eq!(noop["request_id"], REQUEST_2);
        assert!(noop["effects"]["commit_oid"].is_null());
        assert_eq!(f.snapshot(), before);
        let preview = f.call(resource, &["--prepare"], &input, 0, "already_applied");
        assert!(preview["data"].is_null());
        assert_eq!(f.snapshot(), before);
    }
}

#[test]
fn both_repairs_have_human_output() {
    for resource in ["document", "ticket"] {
        let f = Fixture::new();
        let input = f.example(resource);
        let preview = f.prepare(resource, &input);
        let out = mutation::run(
            &f.args(
                resource,
                &["--confirm", consent(&preview), "--request-id", REQUEST_1],
            ),
            &f.env,
            &serde_json::to_vec(&input).unwrap(),
        );
        private(&out, "");
        assert_eq!(out.status, 0);
        assert!(String::from_utf8_lossy(&out.stderr).contains("progress:"));
        let replay = f.confirm(resource, &input, consent(&preview), REQUEST_1);
        let operation = replay["operation_id"].as_str().unwrap();
        let oid = replay["effects"]["commit_oid"].as_str().unwrap();
        cli::assert_cli_text_golden(
            &format!("{resource}_repair_text"),
            std::str::from_utf8(&out.stdout).unwrap(),
            &[
                (f.shelf.root.to_str().unwrap(), "<repository>"),
                (REQUEST_1, "<request-id>"),
                (operation, "<operation-id>"),
                (oid, "<commit-oid>"),
            ],
        );
        let before = f.snapshot();
        let refusal = mutation::run(
            &f.args(resource, &["--request-id", REQUEST_3]),
            &f.env,
            b"{}",
        );
        assert_eq!(refusal.status, 2);
        assert!(refusal.stdout.is_empty());
        assert!(String::from_utf8_lossy(&refusal.stderr).contains("code: invalid_input"));
        assert_eq!(f.snapshot(), before);
    }
}

#[test]
fn repair_inputs_are_closed_typed_and_private() {
    for resource in ["document", "ticket"] {
        let f = Fixture::new();
        let good = f.example(resource);
        let mut bad = vec![
            b"{".to_vec(),
            br#"{"source":"SENTINEL-corrected-a41c","source":"duplicate"}"#.to_vec(),
        ];
        for field in ["path", "observation", "source"] {
            let mut missing = good.clone();
            missing.as_object_mut().unwrap().remove(field);
            bad.push(serde_json::to_vec(&missing).unwrap());
            for value in [Value::Null, json!(42), json!([])] {
                let mut wrong = good.clone();
                wrong[field] = value;
                bad.push(serde_json::to_vec(&wrong).unwrap());
            }
        }
        let mut unknown = good;
        unknown["closed_at"] = repair::SOURCE_SENTINEL.into();
        bad.push(serde_json::to_vec(&unknown).unwrap());
        for input in bad {
            let before = f.snapshot();
            for json in [true, false] {
                let mut args = f.args(resource, &["--request-id", REQUEST_1]);
                if json {
                    args.push("--json");
                }
                let out = mutation::run(&args, &f.env, &input);
                private(&out, "");
                assert_eq!(out.status, 2);
                if json {
                    let value = cli::json(&out);
                    assert_eq!(value["code"], "invalid_input");
                    assert_eq!(value["request_id"], REQUEST_1);
                } else {
                    assert!(out.stdout.is_empty());
                    assert!(String::from_utf8_lossy(&out.stderr).contains("code: invalid_input"));
                }
            }
            f.unchanged(&before);
        }
    }
}
