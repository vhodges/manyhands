//! Task 3 Step 0: the approved interrupted-save design checkpoint.
//! Interrupted actors use the library; retries exercise the real CLI save patch.

use std::fs;

use serde_json::{Value, json};

use manyhands::{
    repository::{
        DocumentDraft, DocumentSaveInput, FailurePoint, IndexRefreshInput, Mutation, RequestState,
    },
    results::{CheckpointEffect, DiscoveryEffect, FailureClass, Outcome, ResultCode},
};

use crate::support::{
    self, FailOnce, items,
    mutation::{
        DOCUMENT_NEW, DOCUMENT_NEW_PATH, REQUEST_1, REQUEST_2, REQUEST_3, TITLE, World,
        execute_with,
    },
};

const OTHER_DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FDP";
const OTHER_PATH: &str = "docs/notes/other.md";
const REFRESH_REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX4";
const OTHER_SAVE_REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX5";
const SAVED_BODY: &str = "The interrupted request's new body.\n";
const EDITED_TITLE: &str = "An uncommitted third-party title";

// Decode only the fields used by Step 0's unchanged assertions. Keep the full
// schema-checked envelope in diagnostics; classify from the real process exit,
// not a synthetic envelope or a second library execution.
#[derive(serde::Serialize)]
struct BinaryAnswer {
    code: ResultCode,
    outcome: Outcome,
    effects: BinaryEffects,
    status: i32,
    envelope: Value,
}

#[derive(serde::Serialize)]
struct BinaryEffects {
    discovery: DiscoveryEffect,
}

impl BinaryAnswer {
    fn failure_class(&self) -> Option<FailureClass> {
        match self.status {
            0 => None,
            1 => Some(FailureClass::Internal),
            2 => Some(FailureClass::Input),
            3 => Some(FailureClass::Blocked),
            4 => Some(FailureClass::Incomplete),
            5 => Some(FailureClass::Transient),
            130 => Some(FailureClass::Cancelled),
            status => panic!("unexpected CLI exit {status}"),
        }
    }
}

fn binary_answer(world: &World, args: &[&str], input: Option<&Value>) -> BinaryAnswer {
    let env = support::cli::CliEnv::new(world.data.path().to_owned());
    fs::write(
        env.home.path().join(".gitconfig"),
        "[core]\n\tautocrlf = false\n",
    )
    .unwrap();
    let mut args = args.to_vec();
    args.extend(["--repo", world.root.to_str().unwrap(), "--json"]);
    let bytes = if let Some(input) = input {
        args.extend(["--input", "-"]);
        serde_json::to_vec(input).unwrap()
    } else {
        vec![]
    };
    let out = super::mutation::run(&args, &env, &bytes);
    let value = support::cli::json(&out);
    support::schema::check_published("envelope.schema.json", &value).unwrap();
    BinaryAnswer {
        code: ResultCode::ALL
            .into_iter()
            .find(|code| value["code"] == code.as_str())
            .unwrap(),
        outcome: Outcome::ALL
            .into_iter()
            .find(|outcome| value["outcome"] == outcome.as_str())
            .unwrap(),
        effects: BinaryEffects {
            discovery: DiscoveryEffect::ALL
                .into_iter()
                .find(|effect| value["effects"]["discovery"] == effect.as_str())
                .unwrap(),
        },
        status: out.status,
        envelope: value,
    }
}

fn binary_save(world: &World, id: &str, request: &str, input: &Value) -> BinaryAnswer {
    binary_answer(
        world,
        &["document", "save", "--id", id, "--request-id", request],
        Some(input),
    )
}

struct InterruptedSave {
    world: World,
    observation: String,
    primary: git2::Oid,
    checkpoint: git2::Oid,
    committed_bytes: Vec<u8>,
}

impl InterruptedSave {
    fn new(row: &str) -> Self {
        let mut world = World::new();
        // Both editing contexts exist before the interruption. The second
        // document lets the fence test request genuinely new save work.
        for (request, id, path) in [
            (REQUEST_1, DOCUMENT_NEW, DOCUMENT_NEW_PATH),
            (REQUEST_2, OTHER_DOCUMENT, OTHER_PATH),
        ] {
            let created = world.execute(request, world.document_create(id, path));
            assert_eq!(created.code, ResultCode::Ok, "{row}: create");
            assert_eq!(created.effects.checkpoint, CheckpointEffect::Committed);
            assert_eq!(world.branch_commits(id), 1);
        }
        let primary = world.primary_head();
        let before = world.branch_tip(DOCUMENT_NEW).unwrap();
        let observation = world.token(DOCUMENT_NEW);
        // Arm the failure only after DocumentCreate has finished discovery.
        world.service = FailOnce::at(FailurePoint::BeforeIndexTransactionCommit)
            .open_service(world.data.path());
        let interrupted = execute_with(
            &world.service,
            REQUEST_3,
            Mutation::DocumentSave(DocumentSaveInput {
                draft: DocumentDraft {
                    title: TITLE.to_owned(),
                    body: SAVED_BODY.to_owned(),
                },
                ..world.document_save_input(DOCUMENT_NEW, &observation)
            }),
        );
        assert_eq!(interrupted.outcome, Outcome::Partial, "{row}: actor");
        assert_eq!(interrupted.failure_class(), Some(FailureClass::Incomplete));
        assert_eq!(interrupted.effects.checkpoint, CheckpointEffect::Committed);
        assert_eq!(interrupted.effects.discovery, DiscoveryEffect::Pending);
        let checkpoint = world.branch_tip(DOCUMENT_NEW).unwrap();
        assert_ne!(checkpoint, before);
        assert_eq!(world.branch_commits(DOCUMENT_NEW), 2);
        assert_eq!(interrupted.effects.commit_oid, Some(checkpoint.to_string()));
        assert_eq!(world.primary_head(), primary);
        assert_eq!(
            world.record(REQUEST_3).unwrap().state,
            RequestState::Accepted
        );
        assert_eq!(world.pending_journal_rows(), 1);
        assert!(
            world
                .journal_row(interrupted.operation_id.as_ref().unwrap())
                .in_flight()
        );
        let committed_bytes =
            fs::read(world.worktree(DOCUMENT_NEW).join(DOCUMENT_NEW_PATH)).unwrap();
        let tree = world
            .fixture
            .repository
            .find_commit(checkpoint)
            .unwrap()
            .tree()
            .unwrap();
        let entry = tree
            .get_path(std::path::Path::new(DOCUMENT_NEW_PATH))
            .unwrap();
        let blob = world.fixture.repository.find_blob(entry.id()).unwrap();
        assert_eq!(blob.content(), committed_bytes);
        drop(blob);
        drop(entry);
        drop(tree);
        // A plain service, over exactly the actor's data directory, drives
        // every retry. No failure hook is installed in this service.
        world.reopen();
        eprintln!(
            "{row}: actor code={:?}, outcome={:?}, primary={primary}, checkpoint={checkpoint}, commits=2, bytes={}, request=accepted, pending=1",
            interrupted.code,
            interrupted.outcome,
            committed_bytes.len()
        );
        Self {
            world,
            observation,
            primary,
            checkpoint,
            committed_bytes,
        }
    }

    fn retry(&self, omit_title: bool) -> BinaryAnswer {
        let mut input = json!({"observation":self.observation,"body":SAVED_BODY});
        if !omit_title {
            input["title"] = json!(TITLE);
        }
        // The actual CLI performs the fill; the original observation is sent
        // unchanged. No actor-side fill can conceal a CLI replay regression.
        binary_save(&self.world, DOCUMENT_NEW, REQUEST_3, &input)
    }

    fn edit_title(&self) {
        let source = String::from_utf8(self.committed_bytes.clone()).unwrap();
        assert_eq!(source.matches(TITLE).count(), 1);
        fs::write(
            self.world.worktree(DOCUMENT_NEW).join(DOCUMENT_NEW_PATH),
            source.replace(TITLE, EDITED_TITLE),
        )
        .unwrap();
        let item = self
            .world
            .service
            .show_item(&self.world.resolved(), &items::item_id(DOCUMENT_NEW))
            .unwrap();
        assert_eq!(item.title.as_deref(), Some(EDITED_TITLE));
        assert_eq!(item.body.as_deref(), Some(SAVED_BODY));
        assert_eq!(self.world.branch_tip(DOCUMENT_NEW), Some(self.checkpoint));
    }

    fn unchanged_checkpoint(&self) {
        assert_eq!(self.world.primary_head(), self.primary);
        assert_eq!(self.world.branch_tip(DOCUMENT_NEW), Some(self.checkpoint));
        assert_eq!(self.world.branch_commits(DOCUMENT_NEW), 2);
    }

    fn report(&self, row: &str, answer: &impl serde::Serialize, unchanged: bool) {
        eprintln!(
            "{row}: answer={}, primary={}, branch={}, commits={}, canonical/worktree unchanged={unchanged}, request(state,attempt,result)={:?}, pending={}",
            serde_json::to_string(answer).unwrap(),
            self.world.primary_head(),
            self.world.branch_tip(DOCUMENT_NEW).unwrap(),
            self.world.branch_commits(DOCUMENT_NEW),
            self.world.record(REQUEST_3).map(|record| (
                record.state,
                record.attempt,
                record.result
            )),
            self.world.pending_journal_rows()
        );
    }

    fn refused_retry(&self, row: &str, omit_title: bool, code: ResultCode, class: FailureClass) {
        let before = support::repository_and_worktree_snapshot(&self.world.fixture);
        let answer = self.retry(omit_title);
        let unchanged = support::repository_and_worktree_snapshot(&self.world.fixture) == before;
        self.report(row, &answer, unchanged);
        assert_eq!(answer.code, code, "{row}: approved table differs; STOP");
        assert_eq!(answer.failure_class(), Some(class), "{row}");
        assert!(unchanged, "{row}: nothing may be written");
        self.unchanged_checkpoint();
        assert_eq!(
            self.world.record(REQUEST_3).unwrap().state,
            RequestState::Accepted,
            "{row}"
        );
        assert_eq!(self.world.pending_journal_rows(), 1, "{row}");
    }

    fn completed_retry(&self, row: &str) {
        let before = support::repository_and_worktree_snapshot(&self.world.fixture);
        let answer = self.retry(true);
        self.report(
            row,
            &answer,
            support::repository_and_worktree_snapshot(&self.world.fixture) == before,
        );
        assert_eq!(
            answer.code,
            ResultCode::Ok,
            "{row}: approved table differs; STOP"
        );
        assert_eq!(answer.outcome, Outcome::Success);
        assert_eq!(answer.effects.discovery, DiscoveryEffect::Current);
        self.unchanged_checkpoint();
        assert_eq!(
            fs::read(self.world.worktree(DOCUMENT_NEW).join(DOCUMENT_NEW_PATH)).unwrap(),
            self.committed_bytes
        );
        assert_eq!(
            self.world.record(REQUEST_3).unwrap().state,
            RequestState::Finished
        );
        assert_eq!(self.world.pending_journal_rows(), 0);
    }

    fn other_requests(&self, row: &str, blocked: bool) {
        for (request, mutation) in [
            (
                REFRESH_REQUEST,
                Mutation::IndexRefresh(IndexRefreshInput {
                    root: self.world.root.clone(),
                }),
            ),
            (
                OTHER_SAVE_REQUEST,
                Mutation::DocumentSave(DocumentSaveInput {
                    draft: DocumentDraft {
                        title: TITLE.to_owned(),
                        body: "Actual new work on another document.\n".to_owned(),
                    },
                    ..self
                        .world
                        .document_save_input(OTHER_DOCUMENT, &self.world.token(OTHER_DOCUMENT))
                }),
            ),
        ] {
            let before = support::repository_and_worktree_snapshot(&self.world.fixture);
            let answer = match mutation {
                Mutation::IndexRefresh(_) => binary_answer(
                    &self.world,
                    &["index", "refresh", "--request-id", request],
                    None,
                ),
                Mutation::DocumentSave(input) => binary_save(
                    &self.world,
                    OTHER_DOCUMENT,
                    request,
                    &json!({
                        "observation":input.observation,
                        "title":input.draft.title,
                        "body":input.draft.body,
                    }),
                ),
                _ => unreachable!("only the two approved new-work probes"),
            };
            let unchanged =
                support::repository_and_worktree_snapshot(&self.world.fixture) == before;
            self.report(row, &answer, unchanged);
            eprintln!(
                "{row}: other request {request} (state,attempt,result)={:?}",
                self.world.record(request).map(|record| (
                    record.state,
                    record.attempt,
                    record.result
                ))
            );
            if blocked {
                assert_eq!(
                    answer.code,
                    ResultCode::RecoveryRequired,
                    "{row}: approved table differs; STOP"
                );
                // failure_class here is decoded from the child's actual exit.
                assert_eq!(answer.failure_class(), Some(FailureClass::Blocked));
                assert!(unchanged, "{row}: nothing may be written");
                assert_eq!(
                    self.world.record(REQUEST_3).unwrap().state,
                    RequestState::Accepted
                );
                assert_eq!(self.world.pending_journal_rows(), 1);
            } else {
                assert_eq!(
                    answer.code,
                    ResultCode::Ok,
                    "{row}: other new work must succeed"
                );
                assert_eq!(answer.outcome, Outcome::Success);
                assert_eq!(
                    self.world.record(request).unwrap().state,
                    RequestState::Finished
                );
                assert_eq!(self.world.pending_journal_rows(), 0);
            }
            self.unchanged_checkpoint();
        }
        if !blocked {
            assert_eq!(self.world.branch_commits(OTHER_DOCUMENT), 2);
        }
    }
}

#[test]
fn interrupted_saves_follow_the_required_outcomes() {
    // Each table row has an independent accepted/pending checkpoint fixture.
    // Stop at the FIRST divergent answer; do not amend the contract to fit it.
    {
        let f = InterruptedSave::new("row 1: nothing else");
        f.other_requests("row 1: nothing else", true);
    }
    {
        let f = InterruptedSave::new("row 2: unchanged omitted retry");
        f.completed_retry("row 2: unchanged omitted retry");
        f.other_requests("row 2: after completion", false);
    }
    {
        let f = InterruptedSave::new("row 3: edited omitted field");
        f.edit_title();
        f.refused_retry(
            "row 3: edited omitted field",
            true,
            ResultCode::RequestMismatch,
            FailureClass::Incomplete,
        );
    }
    {
        let f = InterruptedSave::new("row 4: edited file, full original draft");
        f.edit_title();
        f.refused_retry(
            "row 4: edited file, full original draft",
            false,
            ResultCode::ExternalChange,
            FailureClass::Incomplete,
        );
    }
    // Row 5 covers restoration after BOTH of the preceding refusal paths.
    for (omitted, code, class) in [
        (true, ResultCode::RequestMismatch, FailureClass::Incomplete),
        (false, ResultCode::ExternalChange, FailureClass::Incomplete),
    ] {
        let row = if omitted {
            "row 5: restore after row 3"
        } else {
            "row 5: restore after row 4"
        };
        let f = InterruptedSave::new(row);
        f.edit_title();
        f.refused_retry(row, omitted, code, class);
        fs::write(
            f.world.worktree(DOCUMENT_NEW).join(DOCUMENT_NEW_PATH),
            &f.committed_bytes,
        )
        .unwrap();
        f.completed_retry(row);
    }
}

// Binary fixtures use the established CLI contract repository and isolated
// child environment. No service is held open by this fixture.
struct DocumentFixture(super::index::Fixture);

const NEW_PATH: &str = "docs/new.md";
const MOVED_PATH: &str = "docs/moved.md";
const BYTE_BODY: &str = "line one\r\nline two\rthree 🦀\nno final newline";

impl DocumentFixture {
    fn new() -> Self {
        Self(super::index::Fixture::new())
    }

    fn call(
        &self,
        verb: &str,
        id: Option<&str>,
        request: &str,
        input: &Value,
        status: i32,
        code: &str,
    ) -> Value {
        let bytes = serde_json::to_vec(input).unwrap();
        let mut extra = vec!["--request-id", request, "--input", "-"];
        if let Some(id) = id {
            extra.extend(["--id", id]);
        }
        let value = self
            .0
            .checked("document", verb, &extra, &bytes, status, code);
        assert_eq!(value["request_id"], request);
        if !value["data"].is_null() {
            support::schema::check_published("document_mutation.schema.json", &value["data"])
                .unwrap();
        }
        value
    }

    fn create(&self) -> Value {
        let input: Value = serde_json::from_slice(include_bytes!(
            "../fixtures/cli_input_v1/document_create.json"
        ))
        .unwrap();
        self.call("create", None, REQUEST_1, &input, 0, "ok")
    }

    fn show(&self, id: &str) -> Value {
        self.0
            .checked("document", "show", &["--id", id], &[], 0, "ok")["data"]
            .clone()
    }

    fn token(&self, id: &str) -> String {
        self.show(id)["observation"].as_str().unwrap().to_owned()
    }

    fn save(&self, request: &str, input: &Value, status: i32, code: &str) -> Value {
        self.call("save", Some(DOCUMENT_NEW), request, input, status, code)
    }

    fn observe(&self, path: &str, context: Option<&std::path::Path>) -> String {
        let mut extra = vec!["--path", path];
        if let Some(context) = context {
            extra.extend(["--context", context.to_str().unwrap()]);
        }
        self.0.checked("path", "observe", &extra, &[], 0, "ok")["data"]["observation"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn context(&self, id: &str) -> std::path::PathBuf {
        self.0.repo.root.join(".manyhands/worktrees").join(id)
    }

    fn tip(&self, id: &str) -> Option<git2::Oid> {
        self.0
            .repo
            .repository
            .find_branch(&World::document_branch(id), git2::BranchType::Local)
            .ok()
            .and_then(|branch| branch.get().target())
    }

    fn commits(&self, id: &str) -> usize {
        let Some(tip) = self.tip(id) else {
            return 0;
        };
        let mut walk = self.0.repo.repository.revwalk().unwrap();
        walk.push(tip).unwrap();
        walk.hide(self.0.repo.repository.head().unwrap().target().unwrap())
            .unwrap();
        walk.count()
    }

    fn snapshot(&self) -> support::RepositoryAndWorktreeSnapshot {
        support::repository_and_worktree_snapshot(&self.0.repo)
    }

    fn edit_title(&self, id: &str) {
        let item = self.show(id);
        let path = std::path::Path::new(item["context"]["worktree"].as_str().unwrap())
            .join(item["path"].as_str().unwrap());
        let source = fs::read_to_string(&path).unwrap();
        let title = item["title"].as_str().unwrap();
        assert_eq!(source.matches(title).count(), 1);
        fs::write(path, source.replace(title, EDITED_TITLE)).unwrap();
    }

    fn move_input(&self, id: &str, destination: &str, context: Option<&std::path::Path>) -> Value {
        json!({"observation":self.token(id), "destination_path":destination, "destination_observation":self.observe(destination, context)})
    }

    fn lose_cache(&self) {
        for suffix in ["-wal", "-shm", ""] {
            let path = self
                .0
                .data
                .path()
                .join(format!("manyhands.sqlite3{suffix}"));
            if path.exists() {
                fs::remove_file(path).unwrap();
            }
        }
    }
}

fn equal_values(before: &Value, after: &Value, except: &[&str]) {
    for field in [
        "id",
        "kind",
        "path",
        "title",
        "type",
        "status",
        "project",
        "team",
        "closure",
        "slug",
        "parent",
        "deps",
        "unknown_metadata",
        "problems",
        "body",
    ] {
        if !except.contains(&field) {
            assert_eq!(before[field], after[field], "field {field}");
        }
    }
}

fn document_golden(name: &str, value: &Value, root: &std::path::Path) {
    let mut replacements = vec![(
        root.canonicalize().unwrap().to_string_lossy().into_owned(),
        "<repository>",
    )];
    for (pointer, label) in [
        ("/request_id", "<request-id>"),
        ("/operation_id", "<operation-id>"),
        ("/effects/commit_oid", "<commit-oid>"),
    ] {
        if let Some(text) = value.pointer(pointer).and_then(Value::as_str) {
            replacements.push((text.to_owned(), label));
        }
    }
    support::cli::assert_cli_golden(
        name,
        value,
        &replacements
            .iter()
            .map(|(v, label)| (v.as_str(), *label))
            .collect::<Vec<_>>(),
    );
}

#[test]
fn document_create_commits_one_checkpoint_offline() {
    let f = DocumentFixture::new();
    let primary = f.0.repo.repository.head().unwrap().target().unwrap();
    assert!(f.0.repo.repository.remotes().unwrap().is_empty());
    let value = f.create();
    assert_eq!(value["effects"]["checkpoint"], "committed");
    assert_eq!(value["effects"]["publication"], "not_requested");
    assert_eq!(
        value["effects"]["commit_oid"],
        f.tip(DOCUMENT_NEW).unwrap().to_string()
    );
    assert_eq!(f.commits(DOCUMENT_NEW), 1);
    assert_eq!(f.show(DOCUMENT_NEW)["path"], NEW_PATH);
    assert_eq!(
        f.0.repo.repository.head().unwrap().target().unwrap(),
        primary
    );
    document_golden("document_create_ok", &value, &f.0.repo.root);
}

#[test]
fn document_create_replays_and_refuses_a_used_or_escaping_path() {
    let f = DocumentFixture::new();
    let first = f.create();
    let before = f.snapshot();
    assert_eq!(f.create(), first);
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.commits(DOCUMENT_NEW), 1);
    let mut input = json!({"item_id":OTHER_DOCUMENT,"path":"docs/guide.md","title":"Fresh title","body":"fresh body"});
    let occupied = f.call("create", None, REQUEST_2, &input, 2, "occupied_path");
    document_golden("document_create_occupied_path", &occupied, &f.0.repo.root);
    let outside = tempfile::tempdir().unwrap();
    let absolute = outside.path().join("SECRET-VALUE.md");
    for path in [
        "../SECRET-VALUE.md",
        "docs/../../SECRET-VALUE.md",
        absolute.to_str().unwrap(),
    ] {
        input["path"] = json!(path);
        let value = f.call("create", None, REQUEST_2, &input, 2, "invalid_path");
        assert!(!value.to_string().contains("SECRET-VALUE"));
        assert_eq!(f.snapshot(), before);
    }
    assert!(!absolute.exists());
    input["path"] = json!("docs/other.md");
    input["item_id"] = json!("SECRET-VALUE");
    let value = f.call("create", None, REQUEST_2, &input, 2, "invalid_input");
    assert!(!value.to_string().contains("SECRET-VALUE"));
    let human = human_call(&f, "create", None, REQUEST_2, &input);
    assert_eq!(human.status, 2);
    assert!(human.stdout.is_empty());
    let detail = String::from_utf8(human.stderr).unwrap();
    assert!(detail.contains("item_id"));
    assert!(!detail.contains("SECRET-VALUE"));
    assert_eq!(f.snapshot(), before);
}

#[test]
fn a_body_only_save_keeps_every_other_value() {
    let f = DocumentFixture::new();
    f.create();
    // Give the library-written file unknown metadata, then commit it through
    // save so the value-preservation comparison begins from a library write.
    let path = f.context(DOCUMENT_NEW).join(NEW_PATH);
    let source = fs::read_to_string(&path).unwrap().replacen(
        "---\n",
        "---\ncustom: {nested: [one, two]}\n",
        1,
    );
    fs::write(&path, source).unwrap();
    f.save(
        REQUEST_2,
        &json!({"observation":f.token(DOCUMENT_NEW),"title":"Prepared title"}),
        0,
        "ok",
    );
    let before = f.show(DOCUMENT_NEW);
    assert!(!before["unknown_metadata"].as_object().unwrap().is_empty());
    let value = f.save(
        REQUEST_3,
        &json!({"observation":before["observation"],"body":BYTE_BODY}),
        0,
        "ok",
    );
    let after = f.show(DOCUMENT_NEW);
    equal_values(&before, &after, &["body"]);
    assert_eq!(
        after["body"].as_str().unwrap().as_bytes(),
        BYTE_BODY.as_bytes()
    );
    assert_eq!(f.commits(DOCUMENT_NEW), 3);
    document_golden("document_save_ok", &value, &f.0.repo.root);
}

#[test]
fn a_save_that_omits_every_field_is_a_noop_for_a_library_written_document() {
    let f = DocumentFixture::new();
    f.create();
    let item = f.show(DOCUMENT_NEW);
    let before = f.snapshot();
    let value = f.save(
        REQUEST_2,
        &json!({"observation":item["observation"]}),
        0,
        "already_applied",
    );
    assert_eq!(value["outcome"], "noop");
    assert_eq!(f.snapshot(), before);
    equal_values(&item, &f.show(DOCUMENT_NEW), &[]);
    assert_eq!(f.commits(DOCUMENT_NEW), 1);
}

#[test]
fn a_save_that_omits_every_field_normalizes_a_hand_written_document() {
    let f = DocumentFixture::new();
    let before = f.show(items::DOCUMENT_A);
    let source = fs::read(f.0.repo.root.join("docs/guide.md")).unwrap();
    let value = f.call(
        "save",
        Some(items::DOCUMENT_A),
        REQUEST_1,
        &json!({"observation":before["observation"]}),
        0,
        "ok",
    );
    assert_eq!(value["effects"]["checkpoint"], "committed");
    equal_values(&before, &f.show(items::DOCUMENT_A), &[]);
    assert_ne!(
        fs::read(f.context(items::DOCUMENT_A).join("docs/guide.md")).unwrap(),
        source
    );
    assert_eq!(f.commits(items::DOCUMENT_A), 1);
}

#[test]
fn a_title_change_by_another_actor_makes_a_body_only_save_external_change() {
    let f = DocumentFixture::new();
    f.create();
    let token = f.token(DOCUMENT_NEW);
    f.edit_title(DOCUMENT_NEW);
    let before = f.snapshot();
    f.save(
        REQUEST_2,
        &json!({"observation":token,"body":"new body"}),
        3,
        "external_change",
    );
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.commits(DOCUMENT_NEW), 1);
}

#[test]
fn a_retry_after_lost_output_replays() {
    let f = DocumentFixture::new();
    f.create();
    let input = json!({"observation":f.token(DOCUMENT_NEW),"body":SAVED_BODY});
    let first = f.save(REQUEST_2, &input, 0, "ok");
    let record = f.0.request(REQUEST_2);
    let before = f.snapshot();
    assert_eq!(f.save(REQUEST_2, &input, 0, "ok"), first);
    assert_eq!(f.0.request(REQUEST_2), record);
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.commits(DOCUMENT_NEW), 2);
}

#[test]
fn a_retry_after_another_actor_changed_an_omitted_field_is_request_mismatch() {
    let f = DocumentFixture::new();
    f.create();
    let input = json!({"observation":f.token(DOCUMENT_NEW),"body":SAVED_BODY});
    let first = f.save(REQUEST_2, &input, 0, "ok");
    let record = f.0.request(REQUEST_2);
    f.edit_title(DOCUMENT_NEW);
    let before = f.snapshot();
    f.save(REQUEST_2, &input, 2, "request_mismatch");
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.0.request(REQUEST_2), record);
    super::index::assert_stored_result(&record, &first);
    assert_eq!(f.commits(DOCUMENT_NEW), 2);
}

#[test]
fn a_document_save_of_a_ticket_id_is_item_not_found_with_its_scope() {
    let f = DocumentFixture::new();
    let before = f.snapshot();
    let value = f.call(
        "save",
        Some(items::TICKET_A),
        REQUEST_1,
        &json!({"observation":"opaque","body":"body"}),
        2,
        "item_not_found",
    );
    assert_eq!(value["scope"]["item_id"], items::TICKET_A);
    assert_eq!(
        value["scope"]["repository"],
        f.0.repo.root.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(value["request_id"], REQUEST_1);
    assert_eq!(f.snapshot(), before);
    let service = manyhands::repository::RepositoryService::open_at(f.0.data.path()).unwrap();
    assert!(
        service
            .request_record(manyhands::repository::RequestId::parse(REQUEST_1).unwrap())
            .unwrap()
            .is_none()
    );
    document_golden("document_save_item_not_found", &value, &f.0.repo.root);
}

#[test]
fn a_never_enabled_root_answers_by_whether_a_field_is_omitted() {
    let repo = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let env = support::cli::CliEnv::new(data.path().to_owned());
    fs::write(
        env.home.path().join(".gitconfig"),
        "[core]\n\tautocrlf = false\n",
    )
    .unwrap();
    let before = support::repository_and_worktree_snapshot(&repo);
    for (input, status, code) in [
        (
            json!({"observation":"opaque","body":"body"}),
            3,
            "repository_not_registered",
        ),
        (
            json!({"observation":"opaque","title":"title","body":"body"}),
            2,
            "repository_not_enabled",
        ),
    ] {
        let value = super::mutation::checked(
            &[
                "document",
                "save",
                "--repo",
                repo.root.to_str().unwrap(),
                "--id",
                DOCUMENT_NEW,
                "--request-id",
                REQUEST_1,
                "--input",
                "-",
                "--json",
            ],
            &env,
            &serde_json::to_vec(&input).unwrap(),
            status,
            code,
            None,
        );
        assert_eq!(value["request_id"], REQUEST_1);
        if code == "repository_not_registered" {
            assert_eq!(value["recovery"], json!([]));
            document_golden(
                "document_save_repository_not_registered",
                &value,
                &repo.root,
            );
        }
        assert_eq!(support::repository_and_worktree_snapshot(&repo), before);
    }
}

#[test]
fn a_lost_cache_answers_by_whether_a_field_is_omitted() {
    let f = DocumentFixture::new();
    f.create();
    let token = f.token(DOCUMENT_NEW);
    f.lose_cache();
    let before = f.snapshot();
    let omitted = f.save(
        REQUEST_2,
        &json!({"observation":token,"body":"body"}),
        3,
        "repository_not_registered",
    );
    assert_eq!(omitted["recovery"], json!([]));
    let full = f.save(
        REQUEST_2,
        &json!({"observation":token,"title":"title","body":"body"}),
        3,
        "repository_not_registered",
    );
    assert!(
        full["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["action"] == "index.rebuild")
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn a_body_round_trips_byte_for_byte() {
    let f = DocumentFixture::new();
    f.call(
        "create",
        None,
        REQUEST_1,
        &json!({"item_id":DOCUMENT_NEW,"path":NEW_PATH,"title":"Byte title","body":BYTE_BODY}),
        0,
        "ok",
    );
    assert_eq!(
        f.show(DOCUMENT_NEW)["body"].as_str().unwrap().as_bytes(),
        BYTE_BODY.as_bytes()
    );
    f.save(
        REQUEST_2,
        &json!({"observation":f.token(DOCUMENT_NEW),"title":"Next title"}),
        0,
        "ok",
    );
    assert_eq!(
        f.show(DOCUMENT_NEW)["body"].as_str().unwrap().as_bytes(),
        BYTE_BODY.as_bytes()
    );
    let replacement = format!("replacement\r\n{BYTE_BODY}");
    f.save(
        REQUEST_3,
        &json!({"observation":f.token(DOCUMENT_NEW),"body":replacement}),
        0,
        "ok",
    );
    assert_eq!(
        f.show(DOCUMENT_NEW)["body"].as_str().unwrap().as_bytes(),
        replacement.as_bytes()
    );
}

#[test]
fn a_committed_save_retried_after_cache_loss_makes_no_second_commit() {
    let f = DocumentFixture::new();
    f.create();
    let input = json!({"observation":f.token(DOCUMENT_NEW),"body":SAVED_BODY});
    f.save(REQUEST_2, &input, 0, "ok");
    let before = f.snapshot();
    f.lose_cache();
    f.0.index("rebuild", REQUEST_3);
    let value = f.save(REQUEST_2, &input, 0, "already_applied");
    assert_eq!(value["outcome"], "noop");
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.commits(DOCUMENT_NEW), 2);
}

#[test]
fn commit_success_refresh_failure_is_completed_by_the_same_request() {
    let f = InterruptedSave::new("commit success, refresh failure");
    let env = support::cli::CliEnv::new(f.world.data.path().to_owned());
    fs::write(
        env.home.path().join(".gitconfig"),
        "[core]\n\tautocrlf = false\n",
    )
    .unwrap();
    let input = json!({"observation":f.observation,"body":SAVED_BODY});
    let value = super::mutation::checked(
        &[
            "document",
            "save",
            "--repo",
            f.world.root.to_str().unwrap(),
            "--id",
            DOCUMENT_NEW,
            "--request-id",
            REQUEST_3,
            "--input",
            "-",
            "--json",
        ],
        &env,
        &serde_json::to_vec(&input).unwrap(),
        0,
        "ok",
        Some("document_mutation.schema.json"),
    );
    assert_eq!(value["effects"]["discovery"], "current");
    f.unchanged_checkpoint();
    assert_eq!(
        fs::read(f.world.worktree(DOCUMENT_NEW).join(DOCUMENT_NEW_PATH)).unwrap(),
        f.committed_bytes
    );
    let listed = super::mutation::checked(
        &[
            "document",
            "list",
            "--repo",
            f.world.root.to_str().unwrap(),
            "--json",
        ],
        &env,
        &[],
        0,
        "ok",
        Some("item_list.schema.json"),
    );
    assert!(
        listed["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == DOCUMENT_NEW && item["title"] == TITLE)
    );
    assert_eq!(
        f.world.record(REQUEST_3).unwrap().state,
        RequestState::Finished
    );
    assert_eq!(f.world.pending_journal_rows(), 0);
}

#[test]
fn document_move_checkpoints_both_paths() {
    let f = DocumentFixture::new();
    f.create();
    let source = fs::read(f.context(DOCUMENT_NEW).join(NEW_PATH)).unwrap();
    let input = f.move_input(DOCUMENT_NEW, MOVED_PATH, Some(&f.context(DOCUMENT_NEW)));
    let value = f.call("move", Some(DOCUMENT_NEW), REQUEST_2, &input, 0, "ok");
    assert_eq!(value["effects"]["checkpoint"], "committed");
    assert_eq!(value["effects"]["publication"], "not_requested");
    assert!(!f.context(DOCUMENT_NEW).join(NEW_PATH).exists());
    assert_eq!(
        fs::read(f.context(DOCUMENT_NEW).join(MOVED_PATH)).unwrap(),
        source
    );
    let tree =
        f.0.repo
            .repository
            .find_commit(f.tip(DOCUMENT_NEW).unwrap())
            .unwrap()
            .tree()
            .unwrap();
    assert!(tree.get_path(std::path::Path::new(NEW_PATH)).is_err());
    assert!(tree.get_path(std::path::Path::new(MOVED_PATH)).is_ok());
    assert_eq!(f.commits(DOCUMENT_NEW), 2);
    document_golden("document_move_ok", &value, &f.0.repo.root);
}

#[test]
fn a_first_move_of_a_document_on_primary_uses_the_roots_token() {
    let f = DocumentFixture::new();
    assert!(!f.context(items::DOCUMENT_A).exists());
    let input = f.move_input(items::DOCUMENT_A, MOVED_PATH, None);
    f.call("move", Some(items::DOCUMENT_A), REQUEST_1, &input, 0, "ok");
    assert_eq!(f.show(items::DOCUMENT_A)["path"], MOVED_PATH);
    assert!(!f.context(items::DOCUMENT_A).join("docs/guide.md").exists());
    assert_eq!(f.commits(items::DOCUMENT_A), 1);
}

#[test]
fn a_move_onto_a_file_is_occupied_path() {
    let f = DocumentFixture::new();
    f.create();
    let input = f.move_input(
        DOCUMENT_NEW,
        "docs/guide.md",
        Some(&f.context(DOCUMENT_NEW)),
    );
    let before = f.snapshot();
    let value = f.call(
        "move",
        Some(DOCUMENT_NEW),
        REQUEST_2,
        &input,
        2,
        "occupied_path",
    );
    assert_eq!(f.snapshot(), before);
    document_golden("document_move_occupied_path", &value, &f.0.repo.root);
}

#[test]
fn a_destination_token_from_the_wrong_context_is_external_change() {
    let f = DocumentFixture::new();
    f.create();
    let input = f.move_input(DOCUMENT_NEW, MOVED_PATH, None);
    let before = f.snapshot();
    f.call(
        "move",
        Some(DOCUMENT_NEW),
        REQUEST_2,
        &input,
        3,
        "external_change",
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn a_destination_that_gained_a_file_after_it_was_observed_is_external_change() {
    let f = DocumentFixture::new();
    f.create();
    let input = f.move_input(DOCUMENT_NEW, MOVED_PATH, Some(&f.context(DOCUMENT_NEW)));
    fs::write(
        f.context(DOCUMENT_NEW).join(MOVED_PATH),
        "Someone else's file\n",
    )
    .unwrap();
    let before = f.snapshot();
    f.call(
        "move",
        Some(DOCUMENT_NEW),
        REQUEST_2,
        &input,
        3,
        "external_change",
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn a_move_to_its_own_path_is_a_noop() {
    let f = DocumentFixture::new();
    f.create();
    let input = f.move_input(DOCUMENT_NEW, NEW_PATH, Some(&f.context(DOCUMENT_NEW)));
    let before = f.snapshot();
    let value = f.call(
        "move",
        Some(DOCUMENT_NEW),
        REQUEST_2,
        &input,
        0,
        "already_applied",
    );
    assert_eq!(value["outcome"], "noop");
    assert_eq!(f.snapshot(), before);
    let primary = f.move_input(items::DOCUMENT_A, "docs/guide.md", None);
    assert!(f.tip(items::DOCUMENT_A).is_none());
    assert!(!f.context(items::DOCUMENT_A).exists());
    let value = f.call(
        "move",
        Some(items::DOCUMENT_A),
        REQUEST_3,
        &primary,
        0,
        "already_applied",
    );
    assert_eq!(value["outcome"], "noop");
    assert!(value["effects"]["commit_oid"].is_null());
    assert!(f.tip(items::DOCUMENT_A).is_none());
    assert!(!f.context(items::DOCUMENT_A).exists());
    assert_eq!(f.snapshot(), before);
}

#[test]
fn a_move_to_an_escaping_destination_is_refused() {
    let f = DocumentFixture::new();
    f.create();
    let outside = tempfile::tempdir().unwrap();
    let absolute = outside.path().join("SECRET-VALUE.md");
    let before = f.snapshot();
    for path in [
        "../SECRET-VALUE.md",
        "docs/../../SECRET-VALUE.md",
        absolute.to_str().unwrap(),
    ] {
        let value = f.call("move", Some(DOCUMENT_NEW), REQUEST_2, &json!({"observation":f.token(DOCUMENT_NEW),"destination_path":path,"destination_observation":"opaque"}), 2, "invalid_path");
        assert!(!value.to_string().contains("SECRET-VALUE"));
        assert_eq!(f.snapshot(), before);
    }
    assert!(!absolute.exists());
}

#[test]
fn a_stale_source_token_is_external_change() {
    let f = DocumentFixture::new();
    f.create();
    let input = f.move_input(DOCUMENT_NEW, MOVED_PATH, Some(&f.context(DOCUMENT_NEW)));
    f.edit_title(DOCUMENT_NEW);
    let before = f.snapshot();
    f.call(
        "move",
        Some(DOCUMENT_NEW),
        REQUEST_2,
        &input,
        3,
        "external_change",
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn a_move_replays_a_finished_request() {
    let f = DocumentFixture::new();
    f.create();
    let input = f.move_input(DOCUMENT_NEW, MOVED_PATH, Some(&f.context(DOCUMENT_NEW)));
    let first = f.call("move", Some(DOCUMENT_NEW), REQUEST_2, &input, 0, "ok");
    let before = f.snapshot();
    assert_eq!(
        f.call("move", Some(DOCUMENT_NEW), REQUEST_2, &input, 0, "ok"),
        first
    );
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.commits(DOCUMENT_NEW), 2);
    super::index::assert_stored_result(&f.0.request(REQUEST_2), &first);
}

fn human_call(
    f: &DocumentFixture,
    verb: &str,
    id: Option<&str>,
    request: &str,
    input: &Value,
) -> support::cli::CliRun {
    let mut extra = vec!["--request-id", request, "--input", "-"];
    if let Some(id) = id {
        extra.extend(["--id", id]);
    }
    super::mutation::run(
        &f.0.args("document", verb, &extra),
        &f.0.env,
        &serde_json::to_vec(input).unwrap(),
    )
}

#[test]
fn a_human_save_reports_its_progress_on_stderr() {
    let f = DocumentFixture::new();
    f.create();
    let input = json!({"observation":f.token(DOCUMENT_NEW),"body":SAVED_BODY});
    let out = human_call(&f, "save", Some(DOCUMENT_NEW), REQUEST_2, &input);
    assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("progress:"));
    // The JSON replay checks empty stderr through cli::json.
    f.save(REQUEST_2, &input, 0, "ok");
}

#[test]
fn document_verbs_have_human_success_refusal_and_noop_output() {
    for verb in ["create", "save", "move"] {
        let f = DocumentFixture::new();
        let create: Value = serde_json::from_slice(include_bytes!(
            "../fixtures/cli_input_v1/document_create.json"
        ))
        .unwrap();
        if verb != "create" {
            f.create();
        }
        let input = match verb {
            "create" => create,
            "save" => json!({"observation":f.token(DOCUMENT_NEW),"body":SAVED_BODY}),
            _ => f.move_input(DOCUMENT_NEW, MOVED_PATH, Some(&f.context(DOCUMENT_NEW))),
        };
        let id = (verb != "create").then_some(DOCUMENT_NEW);
        let out = human_call(&f, verb, id, REQUEST_2, &input);
        assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
        let replay = f.call(verb, id, REQUEST_2, &input, 0, "ok");
        support::cli::assert_cli_text_golden(
            &format!("document_{verb}_text"),
            std::str::from_utf8(&out.stdout).unwrap(),
            &[
                (REQUEST_2, "<request-id>"),
                (replay["operation_id"].as_str().unwrap(), "<operation-id>"),
                (
                    replay["effects"]["commit_oid"].as_str().unwrap(),
                    "<commit-oid>",
                ),
                (f.0.repo.root.to_str().unwrap(), "<repository>"),
            ],
        );
        let noop = match verb {
            "create" => input.clone(),
            "save" => json!({"observation":f.token(DOCUMENT_NEW)}),
            _ => f.move_input(DOCUMENT_NEW, MOVED_PATH, Some(&f.context(DOCUMENT_NEW))),
        };
        let before = f.snapshot();
        let out = human_call(&f, verb, id, REQUEST_3, &noop);
        assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
        assert!(
            String::from_utf8_lossy(&out.stdout).starts_with("Already applied; nothing changed.\n")
        );
        assert_eq!(f.snapshot(), before);
        let mut bad = input.clone();
        let (status, code) = match verb {
            "create" => {
                bad["item_id"] = json!(OTHER_DOCUMENT);
                bad["path"] = json!("docs/guide.md");
                (2, "occupied_path")
            }
            "save" => {
                bad["observation"] = json!("stale");
                bad["body"] = json!("different body");
                (3, "external_change")
            }
            _ => {
                bad = f.move_input(
                    DOCUMENT_NEW,
                    "docs/guide.md",
                    Some(&f.context(DOCUMENT_NEW)),
                );
                (2, "occupied_path")
            }
        };
        let out = human_call(&f, verb, id, REFRESH_REQUEST, &bad);
        assert_eq!(out.status, status);
        assert!(String::from_utf8_lossy(&out.stderr).contains(code));
        assert!(out.stdout.is_empty());
        assert_eq!(f.snapshot(), before);
    }
}

#[test]
fn document_inputs_are_closed_typed_and_private_and_bad_ids_keep_request_id() {
    let f = DocumentFixture::new();
    f.create();
    for verb in ["create", "save", "move"] {
        let mut extra = vec!["--request-id", REQUEST_2, "--input", "-", "--json"];
        if verb != "create" {
            extra.extend(["--id", DOCUMENT_NEW]);
        }
        for input in [
            b"SECRET-VALUE".as_slice(),
            b"{}",
            br#"{"observation":"token","title":null}"#,
            br#"{"observation":"token","body":7}"#,
            br#"{"observation":"token","body":"SECRET-VALUE","body":"duplicate"}"#,
            br#"{"observation":"token","extra":"SECRET-VALUE"}"#,
        ] {
            let before = f.snapshot();
            let out = super::mutation::run(&f.0.args("document", verb, &extra), &f.0.env, input);
            let value = support::cli::json(&out);
            assert_eq!(out.status, 2, "{value}");
            assert_eq!(value["code"], "invalid_input");
            assert_eq!(value["request_id"], REQUEST_2);
            assert!(!String::from_utf8_lossy(&out.stdout).contains("SECRET-VALUE"));
            assert_eq!(f.snapshot(), before);
        }
    }
    for (verb, input) in [
        ("save", json!({"observation":"token"})),
        (
            "move",
            json!({"observation":"token","destination_path":MOVED_PATH,"destination_observation":"token"}),
        ),
    ] {
        let value = f.call(
            verb,
            Some("SECRET-VALUE"),
            REQUEST_2,
            &input,
            2,
            "invalid_id",
        );
        assert!(!value.to_string().contains("SECRET-VALUE"));
    }
}
