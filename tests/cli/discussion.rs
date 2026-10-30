//! `comment add` through the binary, without a remote. What needs one is
//! Task 12a's: the table's remote rows run in `tests/cli_remote`.
//!
//! The library's own table (`tests/mutation_remote/comment.rs`) pins what
//! the library answers. These cases pin what the verb adds: its input, its
//! item lookup, its exit statuses, its output and the one write.

use std::fs;

use serde_json::{Value, json};

use super::{
    c4_support::{self, golden, input_directory},
    index::{Fixture, assert_stored_result},
    mutation,
};
use crate::support::{
    self, cli, items,
    mutation::{REQUEST_1, REQUEST_2, REQUEST_3},
    schema,
};

const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
const REPLY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM1";
const REPLY_TO_REPLY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM2";
const OTHER_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM3";
const UNKNOWN_PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM9";
/// The identity of the fixture repository, as a comment names its author.
const GIT_AUTHOR: &str = "Manyhands Test <manyhands-test@example.invalid>";
const AUTHOR: &str = "Reviewer <reviewer@example.invalid>";
const NOTE: &str = "note: publishing the comment also publishes this item's other checkpointed work; unsaved drafts are not included.\n";
/// A body whose bytes are exact: CRLF, a non-BMP character, a line that is
/// only `---`, and no final newline (Review Focus 1).
const BYTE_BODY: &str = "line one\r\nline two\r\n---\nthree 🦀\nno final newline";

fn example() -> Value {
    c4_support::example("comment_add")
}

fn bytes(input: &Value) -> Vec<u8> {
    serde_json::to_vec(input).unwrap()
}

fn extra<'a>(item: &'a str, request: &'a str) -> Vec<&'a str> {
    vec!["--id", item, "--request-id", request, "--input", "-"]
}

impl Fixture {
    /// One `comment add` of `item` under `request`, in JSON mode, with its
    /// data checked against the published schema.
    fn add(&self, item: &str, request: &str, input: &Value, status: i32, code: &str) -> Value {
        let value = self.checked(
            "comment",
            "add",
            &extra(item, request),
            &bytes(input),
            status,
            code,
        );
        assert_eq!(value["command"], "comment add");
        assert_eq!(value["request_id"], request);
        if !value["data"].is_null() {
            schema::check_published("comment_mutation.schema.json", &value["data"]).unwrap();
        }
        value
    }

    fn comments(&self, item: &str) -> Vec<Value> {
        let list = self.checked("comment", "list", &["--id", item], &[], 0, "ok");
        list["data"]["items"].as_array().unwrap().clone()
    }

    fn snapshot(&self) -> support::RepositoryAndWorktreeSnapshot {
        support::repository_and_worktree_snapshot(&self.repo)
    }

    /// The comment files of `item`'s editing context, by name.
    fn files(&self, item: &str) -> Vec<String> {
        let directory = items::context_worktree(&self.repo.root, item)
            .join(".manyhands/comments")
            .join(item);
        let mut names: Vec<_> = fs::read_dir(directory)
            .map(|entries| {
                entries
                    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    fn no_request(&self, request: &str) {
        mutation::checked(
            &["request", "show", "--request-id", request, "--json"],
            &self.env,
            &[],
            2,
            "request_not_found",
            None,
        );
    }
}

#[test]
fn the_input_schema_has_two_required_fields_and_two_optional_and_is_closed() {
    let file = "comment_add.schema.json";
    let directory = input_directory();
    schema::validate(&directory, file).unwrap();
    let ok = |value: &Value| schema::check(&directory, file, value).is_ok();

    let example = example();
    assert!(ok(&example));
    let mut fields: Vec<_> = example.as_object().unwrap().keys().cloned().collect();
    fields.sort();
    assert_eq!(fields, ["body", "comment_id", "created_by", "parent_id"]);

    for (field, required) in [
        ("comment_id", true),
        ("body", true),
        ("parent_id", false),
        ("created_by", false),
    ] {
        let mut value = example.clone();
        value.as_object_mut().unwrap().remove(field);
        assert_eq!(ok(&value), !required, "{field} omitted");
        for wrong in [json!(1), json!(true), json!(["a"]), json!({})] {
            let mut value = example.clone();
            value[field] = wrong;
            assert!(!ok(&value), "{field} wrongly typed");
        }
    }
    // `null` is a root comment, and only for the parent. An empty body is a
    // body.
    for (field, null) in [
        ("comment_id", false),
        ("body", false),
        ("parent_id", true),
        ("created_by", false),
    ] {
        let mut value = example.clone();
        value[field] = Value::Null;
        assert_eq!(ok(&value), null, "{field} null");
    }
    let mut empty = example.clone();
    empty["body"] = json!("");
    assert!(ok(&empty));
    let mut unknown = example;
    unknown["observation"] = json!("not-an-input-field");
    assert!(!ok(&unknown));
}

#[test]
fn a_comment_without_a_publication_remote_is_checkpointed_with_publication_pending() {
    let f = Fixture::new();
    let input = json!({"comment_id": COMMENT, "body": "A first comment.\n"});
    let value = f.add(items::TICKET_A, REQUEST_1, &input, 0, "ok");
    assert_eq!(value["outcome"], "success");
    assert_eq!(value["message"], "The request completed.");
    assert_eq!(value["effects"]["write"], "written");
    assert_eq!(value["effects"]["checkpoint"], "committed");
    // The RFC's exit-0 rule: with no publication remote the code is `ok`,
    // and the effects say what is owed (R-6).
    assert_eq!(value["effects"]["publication"], "pending");
    assert_eq!(value["effects"]["integration"], "not_requested");
    assert_eq!(value["data"]["pending_reason"], "no_publication_remote");
    assert_eq!(value["data"]["publication"], "pending");
    assert_eq!(value["data"]["item_id"], items::TICKET_A);
    assert_eq!(value["data"]["comment_id"], COMMENT);
    assert!(value["data"]["parent_id"].is_null());
    assert_eq!(value["data"]["created_by"], GIT_AUTHOR);
    assert!(value["data"]["conflict"].is_null());
    assert_eq!(value["scope"]["item_id"], items::TICKET_A);
    assert!(value["recovery"].as_array().unwrap().is_empty());
    // The data holds no body.
    assert!(!value.to_string().contains("A first comment."));
    golden("comment_add_ok", &value, &f.repo.root);

    let comments = f.comments(items::TICKET_A);
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["id"], COMMENT);
    assert_eq!(comments[0]["body"], "A first comment.\n");
    assert_eq!(comments[0]["author"], GIT_AUTHOR);
    assert_eq!(f.files(items::TICKET_A), [format!("{COMMENT}.md")]);

    let record = f.request(REQUEST_1);
    assert_stored_result(&record, &value);
}

#[test]
fn a_comment_on_a_document_is_filed_under_the_document() {
    let f = Fixture::new();
    let input = json!({"comment_id": COMMENT, "body": "On a document.\n"});
    let value = f.add(items::DOCUMENT_A, REQUEST_1, &input, 0, "ok");
    assert_eq!(value["data"]["item_id"], items::DOCUMENT_A);
    assert_eq!(value["scope"]["item_id"], items::DOCUMENT_A);
    let comments = f.comments(items::DOCUMENT_A);
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["item_id"], items::DOCUMENT_A);
    assert!(f.comments(items::TICKET_A).is_empty());
}

#[test]
fn a_given_author_replaces_the_git_identity_and_a_body_is_written_byte_for_byte() {
    let f = Fixture::new();
    let input = json!({"comment_id": COMMENT, "body": BYTE_BODY, "created_by": AUTHOR});
    let value = f.add(items::TICKET_A, REQUEST_1, &input, 0, "ok");
    assert_eq!(value["data"]["created_by"], AUTHOR);
    let comments = f.comments(items::TICKET_A);
    assert_eq!(comments[0]["author"], AUTHOR);
    assert_eq!(comments[0]["body"], BYTE_BODY);
    let file = items::context_worktree(&f.repo.root, items::TICKET_A)
        .join(items::comment_path(items::TICKET_A, COMMENT));
    let source = fs::read(file).unwrap();
    assert!(source.ends_with(BYTE_BODY.as_bytes()), "no final newline");
}

#[test]
fn a_comment_on_a_closed_ticket_is_ticket_closed_and_changes_nothing() {
    let f = Fixture::new();
    let before = f.snapshot();
    let input = json!({"comment_id": COMMENT, "body": "Too late.\n"});
    let value = f.add(items::TICKET_B, REQUEST_1, &input, 3, "ticket_closed");
    assert_eq!(value["outcome"], "blocked");
    assert!(value["data"].is_null());
    assert_eq!(value["effects"]["write"], "not_requested");
    assert_eq!(value["effects"]["checkpoint"], "not_requested");
    assert_eq!(f.snapshot(), before);
    // No context and no comment is left.
    assert!(
        !f.repo
            .root
            .join(".manyhands/worktrees")
            .join(items::TICKET_B)
            .exists()
    );
    golden("comment_add_ticket_closed", &value, &f.repo.root);
}

#[test]
fn input_that_is_not_valid_is_refused_with_nothing_done() {
    let f = Fixture::new();
    let before = f.snapshot();
    let valid = json!({"comment_id": COMMENT, "body": "Body.\n"});

    // A malformed author is the library's refusal of the request. It is
    // input, exit 2, and writes nothing.
    for author in ["No Brackets", "Name <>", "Na<me <a@example.invalid>"] {
        let mut input = valid.clone();
        input["created_by"] = json!(author);
        let value = f.add(items::TICKET_A, REQUEST_1, &input, 2, "invalid_input");
        assert!(value["data"].is_null());
        assert_eq!(value["effects"]["write"], "not_requested", "{author}");
        assert_eq!(f.snapshot(), before, "{author}");
    }

    // The CLI refuses the others before it builds a request, with the
    // request ID named and no record made.
    let mut malformed_author_golden = None;
    for (case, input) in [
        (
            "comment_id",
            json!({"comment_id": "not-a-ulid", "body": "b"}),
        ),
        (
            "comment_id lower case",
            json!({"comment_id": COMMENT.to_lowercase(), "body": "b"}),
        ),
        (
            "parent_id",
            json!({"comment_id": COMMENT, "body": "b", "parent_id": "not-a-ulid"}),
        ),
        ("missing body", json!({"comment_id": COMMENT})),
        ("missing comment_id", json!({"body": "b"})),
        ("body type", json!({"comment_id": COMMENT, "body": 1})),
        (
            "unknown field",
            json!({"comment_id": COMMENT, "body": "b", "observation": "x"}),
        ),
        (
            "null author",
            json!({"comment_id": COMMENT, "body": "b", "created_by": null}),
        ),
    ] {
        let value = f.add(items::TICKET_A, REQUEST_2, &input, 2, "invalid_input");
        assert!(value["data"].is_null(), "{case}");
        f.no_request(REQUEST_2);
        assert_eq!(f.snapshot(), before, "{case}");
        malformed_author_golden.get_or_insert(value);
    }
    golden(
        "comment_add_invalid_input",
        &malformed_author_golden.unwrap(),
        &f.repo.root,
    );

    // A malformed `--id` is a usage of the flag, with the request named.
    let value = f.checked(
        "comment",
        "add",
        &extra("not-a-ulid", REQUEST_3),
        &bytes(&valid),
        2,
        "invalid_id",
    );
    assert_eq!(value["request_id"], REQUEST_3);
    f.no_request(REQUEST_3);
    // Input that is not JSON.
    let value = f.checked(
        "comment",
        "add",
        &extra(items::TICKET_A, REQUEST_3),
        b"{",
        2,
        "invalid_input",
    );
    assert_eq!(value["request_id"], REQUEST_3);
    assert_eq!(f.snapshot(), before);
}

#[test]
fn an_item_that_is_not_there_is_item_not_found_with_its_scope_and_no_record() {
    let f = Fixture::new();
    let before = f.snapshot();
    let input = json!({"comment_id": COMMENT, "body": "b"});
    let value = f.add(items::TICKET_ABSENT, REQUEST_1, &input, 2, "item_not_found");
    assert_eq!(value["scope"]["item_id"], items::TICKET_ABSENT);
    assert_eq!(
        value["scope"]["repository"],
        f.repo.root.canonicalize().unwrap().to_str().unwrap()
    );
    // The item lookup refused it before any request was built.
    f.no_request(REQUEST_1);
    assert_eq!(f.snapshot(), before);
}

#[test]
fn comment_add_is_not_confirmable() {
    let f = Fixture::new();
    let before = f.snapshot();
    let value = f.checked(
        "comment",
        "add",
        &["--id", items::TICKET_A, "--input", "-", "--prepare"],
        &bytes(&json!({"comment_id": COMMENT, "body": "b"})),
        2,
        "not_confirmable",
    );
    assert!(value["request_id"].is_null());
    assert_eq!(f.snapshot(), before);
}

#[test]
fn a_reply_s_parent_and_depth_are_listed() {
    let f = Fixture::new();
    let id = items::TICKET_A;
    f.add(
        id,
        REQUEST_1,
        &json!({"comment_id": COMMENT, "body": "root\n"}),
        0,
        "ok",
    );
    let reply = f.add(
        id,
        REQUEST_2,
        &json!({"comment_id": REPLY, "body": "reply\n", "parent_id": COMMENT}),
        0,
        "ok",
    );
    assert_eq!(reply["data"]["parent_id"], COMMENT);
    f.add(
        id,
        REQUEST_3,
        &json!({"comment_id": REPLY_TO_REPLY, "body": "deeper\n", "parent_id": REPLY}),
        0,
        "ok",
    );
    f.add(
        id,
        "01ARZ3NDEKTSV4RRFFQ69G5FX4",
        &json!({"comment_id": OTHER_COMMENT, "body": "another root\n", "parent_id": null}),
        0,
        "ok",
    );
    let listed: Vec<_> = f
        .comments(id)
        .iter()
        .map(|comment| {
            (
                comment["id"].as_str().unwrap().to_owned(),
                comment["parent_id"].as_str().map(str::to_owned),
                comment["depth"].as_u64().unwrap(),
            )
        })
        .collect();
    let find = |wanted: &str| listed.iter().find(|(id, ..)| id == wanted).unwrap().clone();
    assert_eq!(listed.len(), 4);
    assert_eq!(find(COMMENT), (COMMENT.into(), None, 0));
    assert_eq!(find(REPLY), (REPLY.into(), Some(COMMENT.into()), 1));
    assert_eq!(
        find(REPLY_TO_REPLY),
        (REPLY_TO_REPLY.into(), Some(REPLY.into()), 2)
    );
    assert_eq!(find(OTHER_COMMENT), (OTHER_COMMENT.into(), None, 0));
}

#[test]
fn a_parent_that_is_no_comment_of_this_item_is_item_not_found_and_writes_nothing() {
    let f = Fixture::new();
    let id = items::TICKET_A;
    let before = f.snapshot();
    for (parent, request) in [
        (UNKNOWN_PARENT, REQUEST_1),
        // Review Focus 5: the comment's own ID.
        (COMMENT, REQUEST_2),
    ] {
        let input = json!({"comment_id": COMMENT, "body": "b", "parent_id": parent});
        let value = f.add(id, request, &input, 2, "item_not_found");
        assert_eq!(value["scope"]["item_id"], id);
        assert_eq!(value["effects"]["write"], "not_requested");
        assert_eq!(f.snapshot(), before, "{parent}");
    }
    assert!(f.comments(id).is_empty());
    // A comment of another item is no parent either.
    f.add(
        items::DOCUMENT_A,
        REQUEST_3,
        &json!({"comment_id": OTHER_COMMENT, "body": "elsewhere\n"}),
        0,
        "ok",
    );
    let input = json!({"comment_id": COMMENT, "body": "b", "parent_id": OTHER_COMMENT});
    f.add(
        id,
        "01ARZ3NDEKTSV4RRFFQ69G5FX4",
        &input,
        2,
        "item_not_found",
    );
    assert!(f.comments(id).is_empty());
}

#[test]
fn a_comment_id_a_file_already_uses_is_occupied_path() {
    let f = Fixture::new();
    // The ticket's own ID, which its file holds.
    let input = json!({"comment_id": items::TICKET_A, "body": "b"});
    let value = f.add(items::TICKET_A, REQUEST_1, &input, 2, "occupied_path");
    assert_eq!(value["effects"]["write"], "not_requested");
    assert!(f.comments(items::TICKET_A).is_empty());
}

#[test]
fn a_replay_after_lost_output_returns_the_stored_result_and_writes_no_second_file() {
    let f = Fixture::new();
    let id = items::TICKET_A;
    let input = json!({"comment_id": COMMENT, "body": "Once.\n"});
    let first = f.add(id, REQUEST_1, &input, 0, "ok");
    let record = f.request(REQUEST_1);
    let before = f.snapshot();
    let refs = f.refs("refs/");
    assert_eq!(f.add(id, REQUEST_1, &input, 0, "ok"), first);
    assert_eq!(f.request(REQUEST_1), record);
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.refs("refs/"), refs);
    assert_eq!(f.files(id), [format!("{COMMENT}.md")]);
    assert_eq!(f.comments(id).len(), 1);

    // The same request for another comment is another request.
    let other = json!({"comment_id": OTHER_COMMENT, "body": "Twice.\n"});
    let value = f.add(id, REQUEST_1, &other, 2, "request_mismatch");
    assert!(value["data"].is_null());
    assert_eq!(f.snapshot(), before);
    assert_eq!(f.files(id), [format!("{COMMENT}.md")]);
}

#[test]
fn the_human_text_names_the_publication_and_carries_the_note() {
    let f = Fixture::new();
    let input = json!({"comment_id": COMMENT, "body": "Human.\n"});
    let human = mutation::run(
        &f.args("comment", "add", &extra(items::TICKET_A, REQUEST_1)),
        &f.env,
        &bytes(&input),
    );
    assert_eq!(
        human.status,
        0,
        "{}",
        String::from_utf8_lossy(&human.stderr)
    );
    // C2's progress, then the note.
    let stderr = std::str::from_utf8(&human.stderr).unwrap();
    assert!(stderr.starts_with("progress: accepted\n"), "{stderr}");
    assert!(stderr.ends_with(NOTE), "{stderr}");
    assert_eq!(stderr.matches("note:").count(), 1, "{stderr}");
    let stdout = std::str::from_utf8(&human.stdout).unwrap();
    assert!(
        stdout.contains("write=written checkpoint=committed"),
        "{stdout}"
    );
    assert!(stdout.contains("publication=pending"), "{stdout}");
    assert!(
        stdout.contains("Pending reason: no_publication_remote\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("Human."), "no body is printed");
    // The IDs the run made are read from the text itself.
    let after = |prefix: &str| -> String {
        stdout
            .split(prefix)
            .nth(1)
            .and_then(|rest| rest.split(['\n', ' ']).next())
            .unwrap_or_else(|| panic!("no {prefix:?} in {stdout}"))
            .to_owned()
    };
    let operation = after("operation: ");
    let synchronization = after("Synchronization id: ");
    let commit = after("commit=");
    let root = f.repo.root.canonicalize().unwrap();
    cli::assert_cli_text_golden(
        "comment_add_text",
        stdout,
        &[
            (root.to_str().unwrap(), "<repository>"),
            (REQUEST_1, "<request-id>"),
            (&operation, "<operation-id>"),
            (&synchronization, "<synchronization-id>"),
            (&commit, "<commit-oid>"),
        ],
    );

    // JSON mode prints neither the note nor anything on stderr: `add` checks
    // the one envelope.
    let json = f.add(
        items::TICKET_A,
        REQUEST_2,
        &json!({"comment_id": OTHER_COMMENT, "body": "Json.\n"}),
        0,
        "ok",
    );
    assert!(!json.to_string().contains("note:"));
}

#[test]
fn every_comment_add_refusal_in_human_mode_names_its_code_and_prints_no_note() {
    let f = Fixture::new();
    let input = json!({"comment_id": COMMENT, "body": "Too late.\n"});
    let human = mutation::run(
        &f.args("comment", "add", &extra(items::TICKET_B, REQUEST_1)),
        &f.env,
        &bytes(&input),
    );
    assert_eq!(human.status, 3);
    assert!(human.stdout.is_empty());
    let stderr = std::str::from_utf8(&human.stderr).unwrap();
    assert!(stderr.contains("code: ticket_closed\n"), "{stderr}");
    assert!(!stderr.contains("note:"), "{stderr}");
}

#[test]
fn the_example_input_runs_through_the_binary() {
    let f = Fixture::new();
    let original = example();
    let directory = input_directory();
    schema::check(&directory, "comment_add.schema.json", &original).unwrap();
    let value = f.add(items::TICKET_A, REQUEST_1, &original, 0, "ok");
    assert_eq!(f.request(REQUEST_1)["data"]["state"], "finished");
    // Every field of the example reached the comment.
    assert_eq!(value["data"]["comment_id"], original["comment_id"]);
    assert_eq!(value["data"]["created_by"], original["created_by"]);
    assert_eq!(value["data"]["parent_id"], original["parent_id"]);
    let comments = f.comments(items::TICKET_A);
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["id"], original["comment_id"]);
    assert_eq!(comments[0]["body"], original["body"]);
    assert_eq!(comments[0]["author"], original["created_by"]);

    // The same input as a reply to what it wrote.
    let mut reply = original.clone();
    reply["comment_id"] = json!(REPLY);
    reply["parent_id"] = original["comment_id"].clone();
    let value = f.add(items::TICKET_A, REQUEST_2, &reply, 0, "ok");
    assert_eq!(value["data"]["parent_id"], original["comment_id"]);
    schema::check(&directory, "comment_add.schema.json", &reply).unwrap();
}
