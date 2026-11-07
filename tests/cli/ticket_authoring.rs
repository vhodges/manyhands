//! C3 ticket adapters through isolated real CLI children.
use std::{fs, path::PathBuf};

use super::{
    index::{Fixture, assert_stored_result},
    mutation,
    repository::require_no_system_identity,
};
use crate::support::{
    self, cli, items,
    mutation::{REQUEST_1, REQUEST_2, REQUEST_3},
};
use manyhands::repository::{RepositoryService, RequestId};
use serde_json::{Value, json};

const NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT0";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FT1";
const REFRESH: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX4";
const BYTE_BODY: &str = "line one\r\nline two\rthree 🦀\nno final newline";

struct Tickets(Fixture);

impl Tickets {
    fn new() -> Self {
        let (repo, enabled) = items::relationship_repository();
        let env = cli::CliEnv::new(enabled.data_directory.path().to_owned());
        fs::write(
            env.home.path().join(".gitconfig"),
            "[core]\n\tautocrlf = false\n",
        )
        .unwrap();
        drop(enabled.service);
        Self(Fixture {
            repo,
            env,
            data: enabled.data_directory,
        })
    }

    fn contract() -> Self {
        Self(Fixture::new())
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
        let mut extra = vec!["--request-id", request, "--input", "-"];
        if let Some(id) = id {
            extra.extend(["--id", id]);
        }
        let value = self.0.checked(
            "ticket",
            verb,
            &extra,
            &serde_json::to_vec(input).unwrap(),
            status,
            code,
        );
        assert_eq!(value["command"], format!("ticket {verb}"));
        assert_eq!(value["request_id"], request);
        if !value["data"].is_null() {
            support::schema::check_published("ticket_mutation.schema.json", &value["data"])
                .unwrap();
        }
        value
    }

    fn human(&self, verb: &str, id: Option<&str>, request: &str, input: &Value) -> cli::CliRun {
        let mut extra = vec!["--request-id", request, "--input", "-"];
        if let Some(id) = id {
            extra.extend(["--id", id]);
        }
        mutation::run(
            &self.0.args("ticket", verb, &extra),
            &self.0.env,
            &serde_json::to_vec(input).unwrap(),
        )
    }

    fn create(&self) -> Value {
        self.call("create", None, REQUEST_1, &create_input(), 0, "ok")
    }
    fn show(&self, id: &str) -> Value {
        self.0
            .checked("ticket", "show", &["--id", id], &[], 0, "ok")["data"]
            .clone()
    }
    fn token(&self, id: &str) -> String {
        self.show(id)["observation"].as_str().unwrap().into()
    }
    fn save(&self, request: &str, input: &Value, status: i32, code: &str) -> Value {
        self.call("save", Some(NEW), request, input, status, code)
    }
    fn context(&self, id: &str) -> PathBuf {
        self.0.repo.root.join(".manyhands/worktrees").join(id)
    }
    fn path(&self, id: &str) -> PathBuf {
        let item = self.show(id);
        PathBuf::from(item["context"]["worktree"].as_str().unwrap())
            .join(item["path"].as_str().unwrap())
    }
    fn tip(&self, id: &str) -> Option<git2::Oid> {
        self.0
            .repo
            .repository
            .find_branch(&format!("manyhands/ticket/{id}"), git2::BranchType::Local)
            .ok()
            .and_then(|b| b.get().target())
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
        let path = self.path(id);
        let text = fs::read_to_string(&path).unwrap();
        let title = item["title"].as_str().unwrap();
        let line = format!("title: {title}\n");
        assert_eq!(text.matches(&line).count(), 1);
        fs::write(path, text.replace(&line, "title: Third party title\n")).unwrap();
    }
    fn check(&self, id: &str, deps: &str, parent: Option<&str>) -> Value {
        let mut extra = vec!["--id", id, "--deps", deps];
        if let Some(parent) = parent {
            extra.extend(["--parent", parent]);
        }
        let before = self.snapshot();
        let refs = self.0.refs("refs/");
        let answer = self.0.checked("ticket", "check", &extra, &[], 0, "ok");
        assert_eq!(self.snapshot(), before);
        assert_eq!(self.0.refs("refs/"), refs);
        answer["data"].clone()
    }
    fn no_record(&self, request: &str) {
        let service = RepositoryService::open_at(self.0.data.path()).unwrap();
        assert!(
            service
                .request_record(RequestId::parse(request).unwrap())
                .unwrap()
                .is_none()
        );
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

fn create_input() -> Value {
    serde_json::from_slice(include_bytes!(
        "../fixtures/cli_input_v1/ticket_create.json"
    ))
    .unwrap()
}
fn simple_create(id: &str) -> Value {
    json!({"item_id":id,"title":"New ticket","type":"task","status":"open","initials":"vh"})
}

fn deps(item: &Value) -> Vec<&str> {
    item["deps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|dep| dep["id"].as_str().unwrap())
        .collect()
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

fn golden(name: &str, value: &Value, f: &Tickets) {
    let mut pairs = vec![(
        f.0.repo
            .root
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        "<repository>",
    )];
    for (pointer, label) in [
        ("/request_id", "<request-id>"),
        ("/operation_id", "<operation-id>"),
        ("/effects/commit_oid", "<commit-oid>"),
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

fn checkpoint(f: &Tickets, id: &str, answer: &Value, count: usize) {
    assert_eq!(answer["effects"]["checkpoint"], "committed");
    assert_eq!(answer["effects"]["publication"], "not_requested");
    assert_eq!(
        answer["effects"]["commit_oid"],
        f.tip(id).unwrap().to_string()
    );
    assert_eq!(f.commits(id), count);
}

#[test]
fn ticket_create_writes_relationships_and_a_short_code_offline() {
    let f = Tickets::new();
    let primary = f.0.repo.repository.head().unwrap().target().unwrap();
    assert!(f.0.repo.repository.remotes().unwrap().is_empty());
    let answer = f.create();
    checkpoint(&f, NEW, &answer, 1);
    let item = f.show(NEW);
    assert_eq!(deps(&item), [items::RELATED_B, items::RELATED_C]);
    assert_eq!(item["parent"]["id"], items::RELATED_B);
    assert!(item["slug"].as_str().unwrap().starts_with("vh-"));
    assert_eq!(answer["data"]["slug"], item["slug"]);
    assert_eq!(
        f.0.repo.repository.head().unwrap().target().unwrap(),
        primary
    );
    golden("ticket_create_ok", &answer, &f);
}

#[test]
fn ticket_create_reads_null_deps_and_parent_as_none() {
    for relationships in [
        json!({}),
        json!({"deps":null,"parent":null}),
        json!({"deps":[]}),
    ] {
        let f = Tickets::new();
        let mut input = simple_create(NEW);
        input
            .as_object_mut()
            .unwrap()
            .extend(relationships.as_object().unwrap().clone());
        f.call("create", None, REQUEST_1, &input, 0, "ok");
        let item = f.show(NEW);
        assert_eq!(item["deps"], json!([]));
        assert!(item["parent"].is_null());
        assert_eq!(item["body"], "");
    }
}

#[test]
fn ticket_create_refuses_a_dependency_cycle_and_a_parent_cycle_before_any_context_exists() {
    for parent in [false, true] {
        let f = Tickets::new();
        let id = if parent { OTHER } else { items::TICKET_ABSENT };
        let mut input = simple_create(id);
        if parent {
            let mut first = simple_create(NEW);
            first["parent"] = json!(OTHER);
            f.call("create", None, REQUEST_1, &first, 0, "ok");
            input["parent"] = json!(NEW);
        } else {
            input["deps"] = json!([items::RELATED_D]);
        }
        let before = f.snapshot();
        let refs = f.0.refs("refs/");
        let answer = f.call("create", None, REQUEST_2, &input, 2, "relationship_cycle");
        assert_eq!(f.snapshot(), before);
        assert_eq!(f.0.refs("refs/"), refs);
        assert!(f.tip(id).is_none());
        assert!(!f.context(id).exists());
        f.no_record(REQUEST_2);
        if !parent {
            golden("ticket_create_relationship_cycle", &answer, &f);
        }
    }
}

#[test]
fn ticket_save_refuses_a_dependency_cycle_and_a_parent_cycle() {
    for (id, patch) in [
        (items::RELATED_B, json!({"deps":[items::RELATED_C]})),
        (items::RELATED_B, json!({"parent":items::RELATED_C})),
    ] {
        let f = Tickets::new();
        let mut input = patch;
        input["observation"] = json!(f.token(id));
        let before = f.snapshot();
        f.call("save", Some(id), REQUEST_1, &input, 2, "relationship_cycle");
        assert_eq!(f.snapshot(), before);
        assert!(f.tip(id).is_none());
    }
}

#[test]
fn ticket_check_and_ticket_save_agree() {
    let f = Tickets::new();
    let proposal = f.check(items::RELATED_B, items::RELATED_C, Some(items::RELATED_C));
    let before = f.snapshot();
    let answer = f.call("save", Some(items::RELATED_B), REQUEST_1, &json!({"observation":f.token(items::RELATED_B),"deps":[items::RELATED_C],"parent":items::RELATED_C}), 2, "relationship_cycle");
    assert_eq!(proposal["rejection"]["ids"], answer["data"]["rejected"]);
    assert_eq!(proposal["rejection"]["code"], answer["code"]);
    assert_eq!(f.snapshot(), before);
}

#[test]
fn a_self_reference_is_a_cycle_of_one_and_a_repeat_is_removed() {
    let f = Tickets::new();
    let mut input = simple_create(NEW);
    input["deps"] = json!([NEW]);
    let answer = f.call("create", None, REQUEST_1, &input, 2, "relationship_cycle");
    assert_eq!(answer["data"]["rejected"], json!([NEW]));
    assert_eq!(f.check(NEW, NEW, None)["rejection"]["ids"], json!([NEW]));
    let answer = f.create();
    let proposal = f.check(
        NEW,
        &format!(
            "{},{},{}",
            items::RELATED_C,
            items::RELATED_B,
            items::RELATED_C
        ),
        Some(items::RELATED_B),
    );
    assert!(proposal["rejection"].is_null());
    assert_eq!(proposal["deps"], json!(deps(&f.show(NEW))));
    assert_eq!(answer["data"]["rejected"], json!([]));
}

#[test]
fn an_unknown_target_is_accepted_and_listed_unresolved() {
    let f = Tickets::new();
    let mut input = simple_create(NEW);
    input["deps"] = json!([OTHER]);
    input["parent"] = json!(OTHER);
    let answer = f.call("create", None, REQUEST_1, &input, 0, "ok");
    assert_eq!(answer["data"]["unresolved"], json!([OTHER]));
    let proposal = f.check(NEW, OTHER, Some(OTHER));
    assert_eq!(proposal["unresolved"], json!([OTHER]));
    assert!(proposal["rejection"].is_null());
    assert_eq!(deps(&f.show(NEW)), [OTHER]);
    assert_eq!(f.show(NEW)["deps"][0]["state"], "unresolved");
}

#[test]
fn ticket_project_null_clears_and_omitted_deps_preserve_every_other_value() {
    let f = Tickets::contract();
    let before = f.show(items::TICKET_A);
    assert_eq!(before["project"], "manyhands");
    let answer = f.call(
        "save",
        Some(items::TICKET_A),
        REQUEST_1,
        &json!({"observation":before["observation"],"project":null}),
        0,
        "ok",
    );
    let after = f.show(items::TICKET_A);
    assert!(after["project"].is_null());
    equal_values(&before, &after, &["project"]);
    checkpoint(&f, items::TICKET_A, &answer, 1);
}

#[test]
fn ticket_deps_empty_and_null_clear_and_parent_null_clears() {
    for deps in [json!([]), Value::Null] {
        let f = Tickets::new();
        f.create();
        let before = f.show(NEW);
        f.save(
            REQUEST_2,
            &json!({"observation":before["observation"],"deps":deps,"parent":null}),
            0,
            "ok",
        );
        let after = f.show(NEW);
        assert_eq!(after["deps"], json!([]));
        assert!(after["parent"].is_null());
        equal_values(&before, &after, &["deps", "parent"]);
    }
}

#[test]
fn a_ticket_body_only_save_keeps_every_other_value_and_body_bytes() {
    let f = Tickets::new();
    f.create();
    let path = f.path(NEW);
    let source = fs::read_to_string(&path).unwrap().replacen(
        "---\n",
        "---\ncustom: {nested: [one, two]}\n",
        1,
    );
    fs::write(path, source).unwrap();
    f.save(
        REQUEST_2,
        &json!({"observation":f.token(NEW),"title":"Prepared title"}),
        0,
        "ok",
    );
    let before = f.show(NEW);
    assert!(!before["unknown_metadata"].as_object().unwrap().is_empty());
    let answer = f.save(
        REQUEST_3,
        &json!({"observation":before["observation"],"body":BYTE_BODY}),
        0,
        "ok",
    );
    let after = f.show(NEW);
    equal_values(&before, &after, &["body"]);
    assert_eq!(
        after["body"].as_str().unwrap().as_bytes(),
        BYTE_BODY.as_bytes()
    );
    checkpoint(&f, NEW, &answer, 3);
    golden("ticket_save_ok", &answer, &f);
}

#[test]
fn a_ticket_save_omitting_every_field_is_a_committed_library_written_noop() {
    let f = Tickets::new();
    f.create();
    let item = f.show(NEW);
    let before = f.snapshot();
    let bytes = fs::read(f.path(NEW)).unwrap();
    let answer = f.save(
        REQUEST_2,
        &json!({"observation":item["observation"]}),
        0,
        "already_applied",
    );
    assert_eq!(answer["outcome"], "noop");
    assert_eq!(f.snapshot(), before);
    assert_eq!(fs::read(f.path(NEW)).unwrap(), bytes);
    equal_values(&item, &f.show(NEW), &[]);
    assert_eq!(f.commits(NEW), 1);
}

#[test]
fn a_ticket_retry_after_an_omitted_field_edit_is_mismatch_with_stored_result() {
    let f = Tickets::new();
    f.create();
    let input = json!({"observation":f.token(NEW),"body":"Saved body"});
    let first = f.save(REQUEST_2, &input, 0, "ok");
    let record = f.0.request(REQUEST_2);
    f.edit_title(NEW);
    let before = f.snapshot();
    f.save(REQUEST_2, &input, 2, "request_mismatch");
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.0.request(REQUEST_2), record);
    assert_stored_result(&record, &first);
    assert_eq!(f.commits(NEW), 2);
}

#[test]
fn a_ticket_save_of_a_document_is_item_scoped_and_keeps_request_id() {
    let f = Tickets::contract();
    let before = f.snapshot();
    let answer = f.call(
        "save",
        Some(items::DOCUMENT_A),
        REQUEST_1,
        &json!({"observation":"opaque"}),
        2,
        "item_not_found",
    );
    assert_eq!(answer["scope"]["item_id"], items::DOCUMENT_A);
    assert_eq!(
        answer["scope"]["repository"],
        f.0.repo.root.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(f.snapshot(), before);
    f.no_record(REQUEST_1);
    golden("ticket_save_item_not_found", &answer, &f);
}

#[test]
fn ticket_save_decision_seven_distinguishes_omitted_and_full_drafts() {
    for lost_cache in [false, true] {
        let f = if lost_cache {
            Tickets::new()
        } else {
            let repo = support::born_repository();
            let data = tempfile::tempdir().unwrap();
            let env = cli::CliEnv::new(data.path().to_owned());
            fs::write(
                env.home.path().join(".gitconfig"),
                "[core]\n\tautocrlf = false\n",
            )
            .unwrap();
            Tickets(Fixture { repo, env, data })
        };
        if lost_cache {
            f.create();
            f.lose_cache();
        }
        let before = f.snapshot();
        let omitted = f.save(
            REQUEST_2,
            &json!({"observation":"opaque","body":"body"}),
            3,
            "repository_not_registered",
        );
        assert_eq!(omitted["recovery"], json!([]));
        let full = f.save(REQUEST_2, &json!({"observation":"opaque","body":"body","title":"title","type":"task","status":"open","project":null,"team":null}), if lost_cache {3} else {2}, if lost_cache {"repository_not_registered"} else {"repository_not_enabled"});
        if lost_cache {
            assert!(
                full["recovery"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["action"] == "index.rebuild")
            );
        }
        assert_eq!(f.snapshot(), before);
    }
}

#[test]
fn a_ticket_in_this_projects_format_takes_deps_from_a_save_of_only_observation_and_deps() {
    let f = Tickets::new();
    let mut input = simple_create(NEW);
    input["project"] = json!("manyhands");
    input["team"] = json!("core");
    input["body"] = json!(BYTE_BODY);
    f.call("create", None, REQUEST_1, &input, 0, "ok");
    let item = f.show(NEW);
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: \"{NEW}\"\ntitle: \"New ticket\"\ntype: \"task\"\nstatus: \"open\"\nproject: \"manyhands\"\nteam: \"core\"\nwave: \"03\"\ncycle: \"C3\"\nslug: \"{}\"\n---\n{BYTE_BODY}",
        item["slug"].as_str().unwrap()
    );
    fs::write(f.path(NEW), source).unwrap();
    let repo = git2::Repository::open(f.context(NEW)).unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_path(std::path::Path::new(&items::ticket_path(NEW)))
        .unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Quoted project ticket",
        &tree,
        &[&parent],
    )
    .unwrap();
    drop(tree);
    drop(parent);
    drop(index);
    drop(repo);
    f.0.index("refresh", REFRESH);
    assert!(!f.0.repo.root.join(items::ticket_path(NEW)).exists());
    let before = f.show(NEW);
    assert_eq!(before["unknown_metadata"]["wave"], "03");
    let answer = f.save(
        REQUEST_2,
        &json!({"observation":before["observation"],"deps":[items::RELATED_B,items::RELATED_C]}),
        0,
        "ok",
    );
    let after = f.show(NEW);
    equal_values(&before, &after, &["deps"]);
    assert_eq!(deps(&after), [items::RELATED_B, items::RELATED_C]);
    assert_eq!(
        after["body"].as_str().unwrap().as_bytes(),
        BYTE_BODY.as_bytes()
    );
    checkpoint(&f, NEW, &answer, 3);
}

#[test]
fn status_closed_is_ordinary_metadata() {
    let f = Tickets::new();
    f.create();
    f.save(
        REQUEST_2,
        &json!({"observation":f.token(NEW),"status":"closed"}),
        0,
        "ok",
    );
    assert_eq!(f.show(NEW)["closure"]["state"], "open");
    let list =
        f.0.checked("ticket", "list", &["--closure", "open"], &[], 0, "ok");
    assert!(
        list["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == NEW && item["status"] == "closed")
    );
}

#[test]
fn a_lifecycle_closed_ticket_refuses_save_and_slug_assign() {
    let f = Tickets::new();
    let before = f.snapshot();
    for verb in ["save", "slug-assign"] {
        let answer = f.call(
            verb,
            Some(items::RELATED_A),
            REQUEST_1,
            &json!({"observation":f.token(items::RELATED_A)}),
            3,
            "ticket_closed",
        );
        assert_eq!(f.snapshot(), before);
        if verb == "save" {
            golden("ticket_save_ticket_closed", &answer, &f);
        }
    }
}

#[test]
fn slug_assign_adds_a_code_and_is_a_noop_when_repeated() {
    let f = Tickets::new();
    let id = items::RELATED_D;
    assert!(f.show(id)["slug"].is_null());
    let answer = f.call(
        "slug-assign",
        Some(id),
        REQUEST_1,
        &json!({"observation":f.token(id),"initials":"vh"}),
        0,
        "ok",
    );
    checkpoint(&f, id, &answer, 1);
    assert!(f.show(id)["slug"].as_str().unwrap().starts_with("vh-"));
    golden("ticket_slug_assign_ok", &answer, &f);
    let before = f.snapshot();
    let answer = f.call(
        "slug-assign",
        Some(id),
        REQUEST_2,
        &json!({"observation":f.token(id),"initials":"vh"}),
        0,
        "already_applied",
    );
    assert_eq!(answer["outcome"], "noop");
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.commits(id), 1);
}

#[test]
fn slug_assign_on_a_ticket_with_another_code_is_slug_already_assigned() {
    let f = Tickets::new();
    let before = f.snapshot();
    let id = items::RELATED_B;
    assert_eq!(f.show(id)["slug"], "mh-vh-k9x2b");
    let answer = f.call(
        "slug-assign",
        Some(id),
        REQUEST_1,
        &json!({"observation":f.token(id),"initials":"ab"}),
        2,
        "slug_already_assigned",
    );
    assert_eq!(f.snapshot(), before);
    golden("ticket_slug_assign_slug_already_assigned", &answer, &f);
}

#[test]
fn a_stale_token_on_save_and_slug_assign_is_external_change() {
    for verb in ["save", "slug-assign"] {
        let f = Tickets::new();
        let id = if verb == "save" {
            f.create();
            NEW
        } else {
            items::RELATED_D
        };
        let token = f.token(id);
        f.edit_title(id);
        let before = f.snapshot();
        let mut input = json!({"observation":token});
        if verb == "save" {
            input["body"] = json!("Changed body");
        } else {
            input["initials"] = json!("vh");
        }
        f.call(verb, Some(id), REQUEST_2, &input, 3, "external_change");
        assert_eq!(f.snapshot(), before);
    }
}

#[test]
fn closure_and_slug_keys_are_refused() {
    let f = Tickets::new();
    f.create();
    let before = f.snapshot();
    for verb in ["create", "save"] {
        for field in ["closed_at", "closed_by", "slug"] {
            let mut input = if verb == "create" {
                simple_create(OTHER)
            } else {
                json!({"observation":f.token(NEW)})
            };
            input[field] = json!("SECRET-VALUE");
            let id = (verb == "save").then_some(NEW);
            let value = f.call(verb, id, REQUEST_2, &input, 2, "invalid_input");
            assert!(!value.to_string().contains("SECRET-VALUE"));
            let human = f.human(verb, id, REQUEST_2, &input);
            assert_eq!(human.status, 2);
            assert!(human.stdout.is_empty());
            let detail = String::from_utf8(human.stderr).unwrap();
            assert!(detail.contains(field));
            assert!(!detail.contains("SECRET-VALUE"));
            assert_eq!(f.snapshot(), before);
            f.no_record(REQUEST_2);
        }
    }
}

#[test]
fn each_ticket_verb_replays_a_finished_request() {
    for verb in ["create", "save", "slug-assign"] {
        let f = Tickets::new();
        let (id, input) = match verb {
            "create" => (None, create_input()),
            "save" => {
                f.create();
                (
                    Some(NEW),
                    json!({"observation":f.token(NEW),"body":"Saved body"}),
                )
            }
            _ => (
                Some(items::RELATED_D),
                json!({"observation":f.token(items::RELATED_D),"initials":"vh"}),
            ),
        };
        let first = f.call(verb, id, REQUEST_2, &input, 0, "ok");
        let before = f.snapshot();
        let record = f.0.request(REQUEST_2);
        assert_eq!(f.call(verb, id, REQUEST_2, &input, 0, "ok"), first);
        assert_eq!(f.snapshot(), before);
        assert_eq!(f.0.request(REQUEST_2), record);
        assert_stored_result(&record, &first);
        assert_eq!(
            f.commits(id.unwrap_or(NEW)),
            if verb == "save" { 2 } else { 1 }
        );
    }
}

#[test]
fn a_committed_ticket_save_retried_after_cache_loss_makes_no_second_commit() {
    let f = Tickets::new();
    f.create();
    let input = json!({"observation":f.token(NEW),"body":"Saved body"});
    f.save(REQUEST_2, &input, 0, "ok");
    let before = f.snapshot();
    let tip = f.tip(NEW);
    f.lose_cache();
    let refused = f.save(REQUEST_2, &input, 3, "repository_not_registered");
    assert_eq!(refused["recovery"], json!([]));
    f.0.index("rebuild", REFRESH);
    let answer = f.save(REQUEST_2, &input, 0, "already_applied");
    assert_eq!(answer["outcome"], "noop");
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.tip(NEW), tip);
    assert_eq!(f.commits(NEW), 2);
}

#[test]
fn a_save_without_identity_is_identity_required_and_the_same_request_succeeds_after_identity_set() {
    require_no_system_identity();
    let f = Tickets::new();
    let id = items::RELATED_D;
    {
        let mut config = f.0.repo.repository.config().unwrap();
        config.remove("user.name").unwrap();
        config.remove("user.email").unwrap();
    }
    let old = f.token(id);
    let refused = f.call(
        "save",
        Some(id),
        REQUEST_1,
        &json!({"observation":old,"body":"Saved body"}),
        3,
        "identity_required",
    );
    assert!(
        refused["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["action"] == "repo.identity_set")
    );
    f.no_record(REQUEST_1);
    assert!(f.context(id).exists());
    assert_eq!(f.commits(id), 0);
    let bytes = include_bytes!("../fixtures/cli_input_v1/repo_identity_set.json");
    let preview = f.0.checked(
        "repo",
        "identity-set",
        &["--prepare", "--input", "-"],
        bytes,
        0,
        "ok",
    );
    f.0.checked(
        "repo",
        "identity-set",
        &[
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
            "--request-id",
            REQUEST_2,
            "--input",
            "-",
        ],
        bytes,
        0,
        "ok",
    );
    let fresh = f.token(id);
    let answer = f.call(
        "save",
        Some(id),
        REQUEST_1,
        &json!({"observation":fresh,"body":"Saved body"}),
        0,
        "ok",
    );
    checkpoint(&f, id, &answer, 1);
}

#[test]
fn a_create_without_initials_is_initials_required_and_the_same_request_succeeds_with_initials() {
    require_no_system_identity();
    let f = Tickets::new();
    f.0.repo
        .repository
        .config()
        .unwrap()
        .set_str("user.name", "Ünal Önder")
        .unwrap();
    let mut input = simple_create(NEW);
    input.as_object_mut().unwrap().remove("initials");
    let before = f.snapshot();
    let refused = f.call("create", None, REQUEST_1, &input, 3, "initials_required");
    assert!(
        refused["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["action"] == "repo.identity_set")
    );
    f.no_record(REQUEST_1);
    assert_eq!(f.snapshot(), before);
    assert!(f.tip(NEW).is_none());
    assert!(!f.context(NEW).exists());
    input["initials"] = json!("vh");
    let answer = f.call("create", None, REQUEST_1, &input, 0, "ok");
    checkpoint(&f, NEW, &answer, 1);
}

#[test]
fn ticket_inputs_are_closed_typed_and_private_and_relationship_text_is_library_validated() {
    let f = Tickets::new();
    f.create();
    for verb in ["create", "save", "slug-assign"] {
        let mut extra = vec!["--request-id", REQUEST_2, "--input", "-", "--json"];
        if verb != "create" {
            extra.extend(["--id", NEW]);
        }
        for bytes in [
            b"SECRET-VALUE".as_slice(),
            b"{}",
            br#"{"observation":"token","observation":"SECRET-VALUE"}"#,
            br#"{"observation":"token","extra":"SECRET-VALUE"}"#,
            br#"{"observation":null}"#,
        ] {
            let before = f.snapshot();
            let answer = mutation::checked(
                &f.0.args("ticket", verb, &extra),
                &f.0.env,
                bytes,
                2,
                "invalid_input",
                None,
            );
            assert_eq!(answer["request_id"], REQUEST_2);
            assert!(!answer.to_string().contains("SECRET-VALUE"));
            assert_eq!(f.snapshot(), before);
        }
    }
    for verb in ["create", "save"] {
        let id = (verb == "save").then_some(NEW);
        for patch in [
            json!({"deps":[7]}),
            json!({"deps":[null]}),
            json!({"deps":[{"x":"SECRET-VALUE"}]}),
            json!({"deps":"SECRET-VALUE"}),
            json!({"parent":7}),
            json!({"title":null}),
            json!({"body":null}),
        ] {
            let mut input = if verb == "create" {
                simple_create(OTHER)
            } else {
                json!({"observation":f.token(NEW)})
            };
            input
                .as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            let before = f.snapshot();
            f.call(verb, id, REQUEST_2, &input, 2, "invalid_input");
            assert_eq!(f.snapshot(), before);
        }
        for field in ["deps", "parent"] {
            let mut input = if verb == "create" {
                simple_create(OTHER)
            } else {
                json!({"observation":f.token(NEW)})
            };
            input[field] = if field == "deps" {
                json!(["SECRET-VALUE"])
            } else {
                json!("SECRET-VALUE")
            };
            let before = f.snapshot();
            let answer = f.call(verb, id, REQUEST_2, &input, 2, "invalid_relationship");
            assert!(!answer["message"].as_str().unwrap().contains("SECRET-VALUE"));
            assert_eq!(f.snapshot(), before);
        }
    }
}

#[test]
fn ticket_save_and_slug_examples_run_verbatim_and_refuse_the_placeholder_token() {
    let f = Tickets::new();
    for (verb, bytes) in [
        (
            "save",
            include_bytes!("../fixtures/cli_input_v1/ticket_save.json").as_slice(),
        ),
        (
            "slug-assign",
            include_bytes!("../fixtures/cli_input_v1/ticket_slug_assign.json").as_slice(),
        ),
    ] {
        let before = f.snapshot();
        f.0.checked(
            "ticket",
            verb,
            &[
                "--id",
                items::RELATED_D,
                "--request-id",
                REQUEST_1,
                "--input",
                "-",
            ],
            bytes,
            3,
            "external_change",
        );
        assert_eq!(f.snapshot(), before);
    }
}

#[test]
fn ticket_verbs_have_human_success_refusal_and_fresh_noop_output() {
    for verb in ["create", "save", "slug-assign"] {
        let f = Tickets::new();
        let (id, input) = match verb {
            "create" => (None, create_input()),
            "save" => {
                f.create();
                (
                    Some(NEW),
                    json!({"observation":f.token(NEW),"body":"Saved body"}),
                )
            }
            _ => (
                Some(items::RELATED_D),
                json!({"observation":f.token(items::RELATED_D),"initials":"vh"}),
            ),
        };
        let out = f.human(verb, id, REQUEST_2, &input);
        assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
        assert!(String::from_utf8_lossy(&out.stderr).contains("progress:"));
        let replay = f.call(verb, id, REQUEST_2, &input, 0, "ok");
        cli::assert_cli_text_golden(
            &format!("ticket_{}_text", verb.replace('-', "_")),
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
        let noop = if verb == "create" {
            input.clone()
        } else {
            let mut value = input.clone();
            value["observation"] = json!(f.token(id.unwrap()));
            value
        };
        let before = f.snapshot();
        let out = f.human(verb, id, REQUEST_3, &noop);
        assert_eq!(out.status, 0, "{}", String::from_utf8_lossy(&out.stderr));
        assert!(
            String::from_utf8_lossy(&out.stdout).starts_with("Already applied; nothing changed.\n")
        );
        assert_eq!(f.snapshot(), before);
        let mut bad = noop;
        if verb == "create" {
            bad["item_id"] = json!(OTHER);
            bad["deps"] = json!([OTHER]);
        } else {
            bad["observation"] = json!("stale");
            if verb == "save" {
                bad["body"] = json!("Different body");
            } else {
                f.edit_title(id.unwrap());
            }
        }
        let before = f.snapshot();
        let out = f.human(verb, id, REFRESH, &bad);
        assert_eq!(out.status, if verb == "create" { 2 } else { 3 });
        assert!(out.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&out.stderr).contains(if verb == "create" {
                "relationship_cycle"
            } else {
                "external_change"
            })
        );
        assert_eq!(f.snapshot(), before);
    }
}

#[test]
fn ticket_creation_with_bad_slug_configuration_is_refused_without_a_context() {
    let f = Tickets::new();
    let path = f.0.repo.root.join(".manyhands/config.toml");
    let source = fs::read_to_string(&path).unwrap();
    fs::write(
        path,
        format!("{source}\nticket_slug_prefix = \"Not Valid\"\n"),
    )
    .unwrap();
    items::commit(
        &f.0.repo,
        &[".manyhands/config.toml"],
        items::COMMITTED_AT + 500,
    );
    let before = f.snapshot();
    let answer = f.call(
        "create",
        None,
        REQUEST_1,
        &simple_create(NEW),
        3,
        "invalid_slug_configuration",
    );
    assert!(
        answer["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["action"] == "repo.inspect")
    );
    assert_eq!(f.snapshot(), before);
    f.no_record(REQUEST_1);
    assert!(f.tip(NEW).is_none());
    assert!(!f.context(NEW).exists());
}

#[test]
fn ticket_full_draft_updates_each_field_and_preserves_omitted_relationships() {
    let f = Tickets::new();
    f.create();
    let before = f.show(NEW);
    let answer = f.save(REQUEST_2, &json!({"observation":before["observation"],"title":"Updated title","type":"cycle","status":"active","body":BYTE_BODY,"project":"other","team":null}), 0, "ok");
    let after = f.show(NEW);
    for (field, value) in [
        ("title", "Updated title"),
        ("type", "cycle"),
        ("status", "active"),
        ("body", BYTE_BODY),
        ("project", "other"),
    ] {
        assert_eq!(after[field], value);
    }
    assert!(after["team"].is_null());
    equal_values(
        &before,
        &after,
        &["title", "type", "status", "body", "project", "team"],
    );
    checkpoint(&f, NEW, &answer, 2);
}

#[test]
fn ticket_optional_nonnullable_fields_and_id_errors_keep_values_private() {
    let f = Tickets::new();
    f.create();
    let before = f.snapshot();
    for field in ["project", "team", "initials"] {
        let mut input = simple_create(OTHER);
        input[field] = Value::Null;
        f.call("create", None, REQUEST_2, &input, 2, "invalid_input");
    }
    for verb in ["save", "slug-assign"] {
        let answer = f.call(
            verb,
            Some("SECRET-VALUE"),
            REQUEST_2,
            &json!({"observation":"opaque"}),
            2,
            "invalid_id",
        );
        assert!(!answer.to_string().contains("SECRET-VALUE"));
        let extra = [
            "--id",
            NEW,
            "--request-id",
            REQUEST_2,
            "--input",
            "-",
            "--context",
            "SECRET-VALUE",
            "--json",
        ];
        let answer = mutation::checked(
            &f.0.args("ticket", verb, &extra),
            &f.0.env,
            b"{}",
            2,
            "usage_error",
            None,
        );
        assert!(!answer.to_string().contains("SECRET-VALUE"));
    }
    let mut input = simple_create("SECRET-VALUE");
    let answer = f.call("create", None, REQUEST_2, &input, 2, "invalid_input");
    assert!(!answer.to_string().contains("SECRET-VALUE"));
    input = json!({"observation":f.token(items::RELATED_D),"initials":null});
    f.call(
        "slug-assign",
        Some(items::RELATED_D),
        REQUEST_2,
        &input,
        2,
        "invalid_input",
    );
    assert_eq!(f.snapshot(), before);
}
