//! Request identity and records: the request and confirmation IDs, the
//! intent digest, the three request tables, the journal lookup and
//! `show_request`.

use std::path::Path;

use manyhands::repository::{
    ConfirmationId, RequestId,
    request_store::{DigestSalt, FieldValue, IntentDigest, ScopeKey},
};

mod support;

const REQUEST_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ0";
const REQUEST_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FQ1";
const CONFIRMATION_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FN0";

fn request_id(id: &str) -> RequestId {
    RequestId::parse(id).unwrap()
}

fn confirmation_id(id: &str) -> ConfirmationId {
    ConfirmationId::parse(id).unwrap()
}

#[test]
fn request_and_confirmation_ids_are_canonical_ulids() {
    assert_eq!(request_id(REQUEST_A).to_string(), REQUEST_A);
    assert_eq!(confirmation_id(CONFIRMATION_A).to_string(), CONFIRMATION_A);
    assert_ne!(RequestId::new(), RequestId::new());
    assert_ne!(ConfirmationId::new(), ConfirmationId::new());
    for malformed in ["", "01arz3ndektsv4rrffq69g5fq0", "not-a-ulid", " "] {
        assert!(RequestId::parse(malformed).is_err(), "{malformed:?}");
        assert!(ConfirmationId::parse(malformed).is_err(), "{malformed:?}");
    }
}

/// What a ticket save would feed the digest, each part replaceable.
#[derive(Clone)]
struct Intent<'a> {
    salt: DigestSalt,
    command: &'a str,
    repository: ScopeKey,
    target: &'a str,
    title: FieldValue<'a>,
    body: FieldValue<'a>,
    status: FieldValue<'a>,
    deps: FieldValue<'a>,
    parent: FieldValue<'a>,
    observation: FieldValue<'a>,
}

impl Intent<'_> {
    fn digest(&self) -> IntentDigest {
        IntentDigest::builder(self.salt, self.command, &self.repository, self.target)
            .field("title", self.title)
            .field("body", self.body)
            .field("status", self.status)
            .field("deps", self.deps)
            .field("parent", self.parent)
            .field("observation", self.observation)
            .finish()
    }
}

const DEPS: &[&str] = &["01ARZ3NDEKTSV4RRFFQ69G5FC1"];

fn intent() -> Intent<'static> {
    Intent {
        salt: DigestSalt::Request(request_id(REQUEST_A)),
        command: "ticket save",
        repository: ScopeKey::from_stored("/projects/example"),
        target: "01ARZ3NDEKTSV4RRFFQ69G5FC0",
        title: FieldValue::Text("Title"),
        body: FieldValue::Text("Body.\n"),
        status: FieldValue::Text("open"),
        deps: FieldValue::List(DEPS),
        parent: FieldValue::Null,
        observation: FieldValue::Text("token-1"),
    }
}

#[test]
fn the_digest_is_stable_for_equal_input() {
    assert_eq!(intent().digest(), intent().digest());
    let text = intent().digest().to_string();
    assert_eq!(text.len(), 64);
    assert!(
        text.bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    assert_eq!(IntentDigest::from_stored(&text), Some(intent().digest()));
    assert_eq!(IntentDigest::from_stored("not a digest"), None);
}

#[test]
fn the_digest_differs_for_each_changed_part() {
    let base = intent();
    let changed: Vec<(&str, Intent<'_>)> = vec![
        (
            "title",
            Intent {
                title: FieldValue::Text("Other"),
                ..base.clone()
            },
        ),
        (
            "body",
            Intent {
                body: FieldValue::Text("Body!\n"),
                ..base.clone()
            },
        ),
        (
            "status",
            Intent {
                status: FieldValue::Text("closed"),
                ..base.clone()
            },
        ),
        (
            "deps",
            Intent {
                deps: FieldValue::List(&["01ARZ3NDEKTSV4RRFFQ69G5FC2"]),
                ..base.clone()
            },
        ),
        (
            "parent",
            Intent {
                parent: FieldValue::Text("01ARZ3NDEKTSV4RRFFQ69G5FC2"),
                ..base.clone()
            },
        ),
        (
            "observation token",
            Intent {
                observation: FieldValue::Text("token-2"),
                ..base.clone()
            },
        ),
        (
            "target",
            Intent {
                target: "01ARZ3NDEKTSV4RRFFQ69G5FC9",
                ..base.clone()
            },
        ),
        (
            "command",
            Intent {
                command: "document save",
                ..base.clone()
            },
        ),
        (
            "repository",
            Intent {
                repository: ScopeKey::from_stored("/projects/other"),
                ..base.clone()
            },
        ),
        (
            "repository against application",
            Intent {
                repository: ScopeKey::application(),
                ..base.clone()
            },
        ),
        (
            "salt",
            Intent {
                salt: DigestSalt::Request(request_id(REQUEST_B)),
                ..base.clone()
            },
        ),
        (
            "salt kind",
            Intent {
                salt: DigestSalt::Confirmation(confirmation_id(REQUEST_A)),
                ..base.clone()
            },
        ),
    ];

    let mut seen = vec![base.digest()];
    for (part, intent) in changed {
        let digest = intent.digest();
        assert!(!seen.contains(&digest), "a changed {part} kept a digest");
        seen.push(digest);
    }
}

#[test]
fn absent_null_and_empty_are_three_different_inputs() {
    let with = |parent| Intent { parent, ..intent() }.digest();
    let values = [
        with(FieldValue::Absent),
        with(FieldValue::Null),
        with(FieldValue::Text("")),
        with(FieldValue::Bytes(b"")),
        with(FieldValue::List(&[])),
        with(FieldValue::List(&[""])),
        with(FieldValue::Flag(false)),
        with(FieldValue::Flag(true)),
    ];
    for (index, value) in values.iter().enumerate() {
        for other in &values[index + 1..] {
            assert_ne!(value, other);
        }
    }
}

#[test]
fn field_boundaries_cannot_be_shifted() {
    let fields = |title, body| {
        Intent {
            title: FieldValue::Text(title),
            body: FieldValue::Text(body),
            ..intent()
        }
        .digest()
    };
    assert_ne!(fields("ab", "c"), fields("a", "bc"));

    let header = |command, target| {
        Intent {
            command,
            target,
            ..intent()
        }
        .digest()
    };
    assert_ne!(header("ab", "c"), header("a", "bc"));

    let list = |deps| {
        Intent {
            deps: FieldValue::List(deps),
            ..intent()
        }
        .digest()
    };
    assert_ne!(list(&["ab", "c"]), list(&["a", "bc"]));
    assert_ne!(list(&["abc"]), list(&["ab", "c"]));

    // A field's name is part of it: the same values under other names.
    let scope = ScopeKey::from_stored("/projects/example");
    let named = |first, second| {
        IntentDigest::builder(DigestSalt::Request(request_id(REQUEST_A)), "c", &scope, "t")
            .field(first, FieldValue::Text("x"))
            .field(second, FieldValue::Text("x"))
            .finish()
    };
    assert_ne!(named("ab", "c"), named("a", "bc"));
}

fn create_digest(root: &Path) -> IntentDigest {
    let scope = ScopeKey::for_new_repository(root).unwrap();
    IntentDigest::builder(
        DigestSalt::Request(request_id(REQUEST_A)),
        "repo create",
        &scope,
        scope.as_str(),
    )
    .field("primary_branch", FieldValue::Text("main"))
    .finish()
}

#[test]
fn the_digest_for_repo_create_is_the_same_before_and_after_the_root_exists() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("new-repository");

    let before = ScopeKey::for_new_repository(&root).unwrap();
    let digest_before = create_digest(&root);
    std::fs::create_dir(&root).unwrap();

    assert_eq!(ScopeKey::for_new_repository(&root).unwrap(), before);
    assert_eq!(ScopeKey::for_repository(&root).unwrap(), before);
    assert_eq!(create_digest(&root), digest_before);
    let scope = ScopeKey::for_repository(&root).unwrap();
    assert_eq!(
        IntentDigest::builder(
            DigestSalt::Request(request_id(REQUEST_A)),
            "repo create",
            &scope,
            scope.as_str(),
        )
        .field("primary_branch", FieldValue::Text("main"))
        .finish(),
        digest_before
    );
    // The parent is resolved, so another spelling of it is the same key.
    assert_eq!(
        ScopeKey::for_new_repository(&parent.path().join(".").join("new-repository")).unwrap(),
        before
    );
    assert_eq!(ScopeKey::application().as_str(), "application");
    assert!(ScopeKey::for_new_repository(&parent.path().join("absent").join("new")).is_none());
    assert!(ScopeKey::for_repository(&parent.path().join("absent")).is_none());
}
