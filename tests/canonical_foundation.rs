mod support;

use std::{fs, path::PathBuf, str::FromStr};

use manyhands::canonical::{
    CONFIG_PATH, CanonicalItem, InitialsProblem, ItemId, RELATIONSHIP_KEYS, RelationshipProblem,
    RelationshipProblemCode, RepositoryConfig, SlugSettingsProblem, Ticket, TicketRelationships,
    ValidationCode, compose_slug, derive_initials, is_valid_slug, normalize_initials,
    normalized_slug, ordered_comment_threads, parse_item, parse_repository_config, serialize_item,
    serialize_repository_config, short_code, slug_settings, ticket_relationships, validate_context,
};
use std::path::Path;

#[test]
fn disposable_unborn_repository_has_no_initial_commit_or_head_target() {
    let root = {
        let fixture = support::unborn_repository();
        assert_eq!(fixture.root, fixture.tempdir.path());
        assert!(fixture.root.is_dir());
        assert!(fixture.repository.is_empty().unwrap());
        assert!(
            fixture
                .repository
                .find_reference("HEAD")
                .unwrap()
                .target()
                .is_none()
        );
        fixture.root.clone()
    };

    assert!(!root.exists());
}

#[test]
fn disposable_born_repository_has_main_identity_and_one_initial_commit() {
    let root = {
        let fixture = support::born_repository();
        let head = fixture.repository.head().unwrap();
        let target = head.target().unwrap();
        let branch = fixture
            .repository
            .find_branch("main", git2::BranchType::Local)
            .unwrap();
        let commit = fixture.repository.find_commit(target).unwrap();
        let local_config = git2::Config::open(&fixture.repository.path().join("config")).unwrap();

        assert_eq!(fixture.root, fixture.tempdir.path());
        assert_eq!(head.shorthand(), Some("main"));
        assert_eq!(branch.get().target(), Some(target));
        assert_eq!(commit.parent_count(), 0);
        assert_eq!(commit.author().name(), Some("Manyhands Test"));
        assert_eq!(
            commit.author().email(),
            Some("manyhands-test@example.invalid")
        );
        assert_eq!(commit.author().when().seconds(), 0);
        assert_eq!(commit.author().when().offset_minutes(), 0);
        assert_eq!(commit.committer().name(), Some("Manyhands Test"));
        assert_eq!(
            commit.committer().email(),
            Some("manyhands-test@example.invalid")
        );
        assert_eq!(commit.committer().when().seconds(), 0);
        assert_eq!(commit.committer().when().offset_minutes(), 0);
        assert_eq!(
            local_config.get_string("user.name").unwrap(),
            "Manyhands Test"
        );
        assert_eq!(
            local_config.get_string("user.email").unwrap(),
            "manyhands-test@example.invalid"
        );
        fixture.root.clone()
    };

    assert!(!root.exists());
}

#[test]
fn disposable_fixture_sources_validate_from_repository_relative_paths() {
    let fixture = support::born_repository();
    let config = support::config_source();
    let document = support::document_source();
    let ticket = support::ticket_source();
    let root_comment = support::root_comment_source();
    let reply = support::reply_source();
    let paths = [
        PathBuf::from(CONFIG_PATH),
        PathBuf::from("docs/guide.md"),
        PathBuf::from(".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md"),
        PathBuf::from(
            ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
        ),
        PathBuf::from(
            ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAY.md",
        ),
    ];

    for (path, contents) in paths
        .iter()
        .zip([&config, &document, &ticket, &root_comment, &reply])
    {
        let full_path = fixture.root.join(path);
        fs::create_dir_all(full_path.parent().unwrap()).unwrap();
        fs::write(full_path, contents).unwrap();
    }

    assert_eq!(
        parse_repository_config(&config).unwrap().primary_branch,
        "main"
    );
    let context = validate_context([
        (paths[1].clone(), document),
        (paths[2].clone(), ticket),
        (paths[3].clone(), root_comment),
        (paths[4].clone(), reply),
    ]);

    assert!(context.problems.is_empty());
    assert_eq!(context.items.len(), 4);
}

#[test]
fn item_ids_accept_only_canonical_uppercase_crockford_spelling() {
    let canonical = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    assert_eq!(ItemId::from_str(canonical).unwrap().to_string(), canonical);

    for invalid in [
        "01arz3ndektsv4rrffq69g5fav",
        "01ARZ3NDEKTSV4RRFFQ69G5FA",
        "01ARZ3NDEKTSV4RRFFQ69G5FAI",
    ] {
        assert!(
            ItemId::from_str(invalid).is_err(),
            "{invalid} should be rejected"
        );
    }
}

#[test]
fn generated_item_ids_display_as_reparsable_canonical_spelling() {
    let generated = ItemId::generate();
    let spelling = generated.to_string();

    assert_eq!(spelling.len(), 26);
    assert!(spelling.chars().all(|character| {
        matches!(
            character,
            '0'..='9'
                | 'A'..='H'
                | 'J'..='K'
                | 'M'..='N'
                | 'P'..='T'
                | 'V'..='Z'
        )
    }));
    assert_eq!(ItemId::from_str(&spelling).unwrap(), generated);
}

#[test]
fn config_round_trips_known_and_unknown_values() {
    let source = r#"
format_version = 1
primary_branch = "main"
publication_remote = "origin"
labels = ["documentation", "urgent"]

[automation]
enabled = true
reviewers = ["ada", "lin"]
"#;

    let parsed = parse_repository_config(source).unwrap();
    assert_eq!(parsed.primary_branch, "main");
    assert_eq!(parsed.publication_remote.as_deref(), Some("origin"));
    assert!(parsed.unknown.contains_key("labels"));
    assert!(parsed.unknown.contains_key("automation"));

    let reparsed = parse_repository_config(&serialize_repository_config(&parsed).unwrap()).unwrap();
    assert_eq!(reparsed, parsed);
}

#[test]
fn config_omits_absent_publication_remote_when_serializing() {
    let config =
        parse_repository_config("format_version = 1\nprimary_branch = \"main\"\n").unwrap();

    assert_eq!(config.publication_remote, None);
    assert!(
        !serialize_repository_config(&config)
            .unwrap()
            .contains("publication_remote")
    );
}

#[test]
fn config_accepts_git_style_branch_and_remote_names() {
    let config = parse_repository_config(
        "format_version = 1\nprimary_branch = \"release/v1\"\npublication_remote = \"main\"\n",
    )
    .unwrap();

    assert_eq!(config.primary_branch, "release/v1");
    assert_eq!(config.publication_remote.as_deref(), Some("main"));
}

#[test]
fn config_rejects_head_as_primary_branch_when_parsing() {
    let error =
        parse_repository_config("format_version = 1\nprimary_branch = \"HEAD\"\n").unwrap_err();

    assert_eq!(error.path, PathBuf::from(CONFIG_PATH));
    assert_eq!(error.code, ValidationCode::InvalidField);

    for primary_branch in ["main", "HEADS"] {
        assert!(
            parse_repository_config(&format!(
                "format_version = 1\nprimary_branch = {primary_branch:?}\npublication_remote = \"HEAD\"\n"
            ))
            .is_ok(),
            "{primary_branch} should remain valid"
        );
    }
}

#[test]
fn config_rejects_invalid_git_style_branch_and_remote_names_when_parsing() {
    for (field, value) in [
        ("primary_branch", "bad..branch"),
        ("primary_branch", "branch lock"),
        ("primary_branch", "branch\nname"),
        ("primary_branch", "branch/"),
        ("primary_branch", ".branch"),
        ("primary_branch", "branch.lock"),
        ("primary_branch", "feature//x"),
        ("publication_remote", "bad..remote"),
        ("publication_remote", "remote lock"),
    ] {
        let source = if field == "primary_branch" {
            format!("format_version = 1\nprimary_branch = {value:?}\n")
        } else {
            format!(
                "format_version = 1\nprimary_branch = \"main\"\npublication_remote = {value:?}\n"
            )
        };
        let error = parse_repository_config(&source).unwrap_err();
        assert_eq!(error.path, PathBuf::from(CONFIG_PATH), "{field}={value:?}");
        assert_eq!(
            error.code,
            ValidationCode::InvalidField,
            "{field}={value:?}"
        );
    }
}

#[test]
fn config_serialization_rejects_invalid_git_style_names() {
    let valid = RepositoryConfig {
        primary_branch: "release/v1".to_owned(),
        publication_remote: Some("main".to_owned()),
        unknown: toml::Table::new(),
    };
    assert!(serialize_repository_config(&valid).is_ok());

    for primary_branch in [
        "bad..branch",
        "branch lock",
        "branch\nname",
        "branch/",
        ".branch",
        "branch.lock",
        "feature//x",
    ] {
        let error = serialize_repository_config(&RepositoryConfig {
            primary_branch: primary_branch.to_owned(),
            publication_remote: None,
            unknown: toml::Table::new(),
        })
        .unwrap_err();
        assert_eq!(error.path, PathBuf::from(CONFIG_PATH));
        assert_eq!(error.code, ValidationCode::InvalidField);
    }

    for publication_remote in ["bad..remote", "remote lock"] {
        let error = serialize_repository_config(&RepositoryConfig {
            primary_branch: "main".to_owned(),
            publication_remote: Some(publication_remote.to_owned()),
            unknown: toml::Table::new(),
        })
        .unwrap_err();
        assert_eq!(error.path, PathBuf::from(CONFIG_PATH));
        assert_eq!(error.code, ValidationCode::InvalidField);
    }
}

#[test]
fn config_serialization_rejects_head_as_primary_branch() {
    let error = serialize_repository_config(&RepositoryConfig {
        primary_branch: "HEAD".to_owned(),
        publication_remote: None,
        unknown: toml::Table::new(),
    })
    .unwrap_err();

    assert_eq!(error.path, PathBuf::from(CONFIG_PATH));
    assert_eq!(error.code, ValidationCode::InvalidField);

    for primary_branch in ["main", "HEADS"] {
        assert!(
            serialize_repository_config(&RepositoryConfig {
                primary_branch: primary_branch.to_owned(),
                publication_remote: Some("HEAD".to_owned()),
                unknown: toml::Table::new(),
            })
            .is_ok(),
            "{primary_branch} should remain valid"
        );
    }
}

#[test]
fn config_reports_structured_errors_for_invalid_input() {
    let cases = [
        ("primary_branch = \"main\"", ValidationCode::MissingField),
        (
            "format_version = \"1\"\nprimary_branch = \"main\"",
            ValidationCode::InvalidField,
        ),
        (
            "format_version = 2\nprimary_branch = \"main\"",
            ValidationCode::InvalidField,
        ),
        ("format_version = 1", ValidationCode::MissingField),
        (
            "format_version = 1\nprimary_branch = \"\"",
            ValidationCode::InvalidField,
        ),
        (
            "format_version = 1\nprimary_branch = \"main\\u0000\"",
            ValidationCode::InvalidField,
        ),
        (
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"\"",
            ValidationCode::InvalidField,
        ),
        (
            "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\\u0000\"",
            ValidationCode::InvalidField,
        ),
        ("format_version =", ValidationCode::MalformedConfiguration),
    ];

    for (source, code) in cases {
        let error = parse_repository_config(source).unwrap_err();
        assert_eq!(error.path.to_string_lossy(), CONFIG_PATH);
        assert_eq!(error.code, code, "{source}");
        assert!(!error.message.is_empty());
    }
}

#[test]
fn valid_document_round_trip_preserves_unknown_yaml_and_exact_body() {
    let source = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: Design notes\ntags:\n  - documentation\nmetadata:\n  owner: ada\n---\n\n# Heading\nlooks: like yaml\n";

    assert_item_round_trip("docs/design.md", source);
}

#[test]
fn valid_open_ticket_round_trip_preserves_unknown_yaml_and_exact_body() {
    let source = "---\r\nmanyhands_managed: true\r\nmanyhands_kind: ticket\r\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAW\r\ntitle: Open issue\r\ntype: bug\r\nstatus: open\r\nproject: manyhands\r\nlabels:\r\n  - urgent\r\n---\r\n\r\nstatus: not front matter\r\n";

    assert_item_round_trip(
        ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md",
        source,
    );
}

#[test]
fn valid_closed_ticket_round_trip_preserves_unknown_yaml_and_exact_body() {
    let source = "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAX\ntitle: Closed issue\ntype: task\nstatus: done\nclosed_at: 2026-09-30T12:34:56Z\nclosed_by: lin\nteam: platform\nmetadata:\n  severity: low\n---\n\nclosed_at: this is body text\n";

    assert_item_round_trip(
        ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAX/ticket.md",
        source,
    );
}

#[test]
fn valid_root_comment_round_trip_preserves_unknown_yaml_and_exact_body() {
    let source = "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAY\nitem_id: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ncreated_at: 2026-09-30T12:34:56Z\nreactions:\n  - eyes\n---\n\nitem_id: body prose\n";

    assert_item_round_trip(
        ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAY.md",
        source,
    );
}

#[test]
fn valid_reply_round_trip_preserves_unknown_yaml_and_exact_body() {
    let source = "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAZ\nitem_id: 01ARZ3NDEKTSV4RRFFQ69G5FAW\nparent_id: 01ARZ3NDEKTSV4RRFFQ69G5FAY\ncreated_at: 2026-09-30T12:34:56Z\nflags:\n  resolved: false\n---\n\nparent_id: body prose\n";

    assert_item_round_trip(
        ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAW/01ARZ3NDEKTSV4RRFFQ69G5FAZ.md",
        source,
    );
}

#[test]
fn document_title_update_preserves_unknown_mapping_and_exact_body() {
    let source = "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: Before\nmetadata:\n  owner: ada\ntags:\n  - documentation\n---\n\nkey: body text\n";
    let body = "\nkey: body text\n";
    let mut item = parse_item(Path::new("docs/design.md"), source).unwrap();
    let unknown = match &item {
        manyhands::canonical::CanonicalItem::Document(document) => document.unknown.clone(),
        _ => panic!("expected document"),
    };

    let manyhands::canonical::CanonicalItem::Document(document) = &mut item else {
        panic!("expected document");
    };
    document.set_title("After".to_owned()).unwrap();

    assert_eq!(document.unknown, unknown);
    assert_eq!(document.body, body);
    let serialized = serialize_item(&item).unwrap();
    assert!(serialized.ends_with(body));
    assert_eq!(
        parse_item(Path::new("docs/design.md"), &serialized).unwrap(),
        item
    );
}

#[test]
fn document_title_update_validation_is_pathless() {
    let mut document = manyhands::canonical::Document {
        id: ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap(),
        title: "Valid title".to_owned(),
        body: String::new(),
        unknown: serde_yaml::Mapping::new(),
    };

    let error = document.set_title(String::new()).unwrap_err();

    assert!(error.path.as_os_str().is_empty());
}

#[test]
fn canonical_paths_accept_only_matching_lexical_locations() {
    let document = document_source();
    let ticket = ticket_source("01K6YQ2A4D8F1H3J5K7M9N0P2Q");
    let comment = comment_source("01K6YQ3B6E9G2J4K6M8N0P2R4S", "01K6YQ1Z2V6B8N4M3R5T7W9X0A");
    let cases = [
        ("docs/guide.md", document.as_str(), None),
        ("docs/guides/authoring.md", document.as_str(), None),
        (
            ".manyhands/tickets/01K6YQ2A4D8F1H3J5K7M9N0P2Q/ticket.md",
            ticket.as_str(),
            None,
        ),
        (
            ".manyhands/comments/01K6YQ1Z2V6B8N4M3R5T7W9X0A/01K6YQ3B6E9G2J4K6M8N0P2R4S.md",
            comment.as_str(),
            None,
        ),
        (
            "/docs/guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        ("", document.as_str(), Some(ValidationCode::InvalidPath)),
        (
            "./docs/guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs/../guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "Docs/guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs/guide.txt",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs/",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs//guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs/guide.md/",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs/segment\\..\\guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs\\segment\\..\\guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs\\\\guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs/a\\b.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs\\guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "docs/a\\..\\guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            ".manyhands/worktrees/docs/guide.md",
            document.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            ".manyhands/tickets/01K6YQ2A4D8F1H3J5K7M9N0P2Q/ticket.md",
            document.as_str(),
            Some(ValidationCode::KindPathMismatch),
        ),
        (
            ".manyhands/comments/01K6YQ1Z2V6B8N4M3R5T7W9X0A/01K6YQ3B6E9G2J4K6M8N0P2R4S.md",
            document.as_str(),
            Some(ValidationCode::KindPathMismatch),
        ),
        (
            "docs/guide.md",
            ticket.as_str(),
            Some(ValidationCode::KindPathMismatch),
        ),
        (
            "docs/guide.md",
            comment.as_str(),
            Some(ValidationCode::KindPathMismatch),
        ),
        (
            "tickets/open.md",
            ticket.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            "comments/root.md",
            comment.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            ".manyhands/tickets/01K6YQ2A4D8F1H3J5K7M9N0P2Q/not-ticket.md",
            ticket.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            ".manyhands/tickets/01K6YQ2A4D8F1H3J5K7M9N0P2Q/nested/ticket.md",
            ticket.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            ".manyhands/comments/01K6YQ1Z2V6B8N4M3R5T7W9X0A/not-an-id.md",
            comment.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            ".manyhands/comments/01K6YQ1Z2V6B8N4M3R5T7W9X0A/01K6YQ3B6E9G2J4K6M8N0P2R4S.md/extra",
            comment.as_str(),
            Some(ValidationCode::InvalidPath),
        ),
        (
            ".manyhands/tickets/01K6YQ1Z2V6B8N4M3R5T7W9X0A/ticket.md",
            ticket.as_str(),
            Some(ValidationCode::InvalidField),
        ),
        (
            ".manyhands/comments/01K6YQ2A4D8F1H3J5K7M9N0P2Q/01K6YQ3B6E9G2J4K6M8N0P2R4S.md",
            comment.as_str(),
            Some(ValidationCode::InvalidField),
        ),
        (
            ".manyhands/comments/01K6YQ1Z2V6B8N4M3R5T7W9X0A/01K6YQ2A4D8F1H3J5K7M9N0P2Q.md",
            comment.as_str(),
            Some(ValidationCode::InvalidField),
        ),
    ];

    for (path, source, expected_code) in cases {
        let original = source.to_owned();
        match expected_code {
            None => assert!(parse_item(Path::new(path), source).is_ok(), "{path}"),
            Some(code) => {
                let error = parse_item(Path::new(path), source).unwrap_err();
                assert_eq!(error.path, PathBuf::from(path), "{path}");
                assert_eq!(error.code, code, "{path}");
                assert!(!error.message.is_empty(), "{path}");
            }
        }
        assert_eq!(source, original, "{path}");
    }
}

#[test]
fn malformed_item_sources_report_original_paths_without_mutation() {
    let document = document_source();
    let cases = [
        (
            "marker only",
            "---\nmanyhands_managed: true\n---\n",
            ValidationCode::MissingField,
        ),
        (
            "malformed YAML",
            "---\nmanyhands_managed: [\n---\n",
            ValidationCode::MalformedFrontMatter,
        ),
        (
            "missing title",
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\n---\n",
            ValidationCode::MissingField,
        ),
        (
            "wrong title type",
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: 1\n---\n",
            ValidationCode::InvalidField,
        ),
        (
            "lowercase item ID",
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01arz3ndektsv4rrffq69g5fav\ntitle: Guide\n---\n",
            ValidationCode::InvalidField,
        ),
        (
            "invalid timestamp",
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: 01K6YQ3B6E9G2J4K6M8N0P2R4S\nitem_id: 01K6YQ1Z2V6B8N4M3R5T7W9X0A\ncreated_at: yesterday\n---\n",
            ValidationCode::InvalidField,
        ),
        (
            "kind path mismatch",
            document.as_str(),
            ValidationCode::KindPathMismatch,
        ),
    ];

    for (name, source, code) in cases {
        let path = ".manyhands/tickets/01K6YQ2A4D8F1H3J5K7M9N0P2Q/ticket.md";
        let original = source.to_owned();
        let error = parse_item(Path::new(path), source).unwrap_err();
        assert_eq!(error.path, PathBuf::from(path), "{name}");
        assert_eq!(error.code, code, "{name}");
        assert!(!error.message.is_empty(), "{name}");
        assert_eq!(source, original, "{name}");
    }
}

#[cfg(unix)]
#[test]
fn canonical_paths_reject_non_utf8_components() {
    use std::os::unix::ffi::OsStringExt;

    let path = PathBuf::from(std::ffi::OsString::from_vec(b"docs/\xff.md".to_vec()));
    let error = parse_item(&path, &document_source()).unwrap_err();

    assert_eq!(error.path, path);
    assert_eq!(error.code, ValidationCode::InvalidPath);
    assert!(!error.message.is_empty());
}

fn document_source() -> String {
    "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: Guide\n---\n".to_owned()
}

fn ticket_source(id: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: Ticket\ntype: task\nstatus: open\n---\n"
    )
}

fn comment_source(id: &str, item_id: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {item_id}\ncreated_at: 2026-09-30T12:34:56Z\n---\n"
    )
}

fn assert_item_round_trip(path: &str, source: &str) {
    let body = source.rsplit_once("\n---\n").map_or_else(
        || source.rsplit_once("\r\n---\r\n").unwrap().1,
        |(_, body)| body,
    );
    let item = parse_item(Path::new(path), source).unwrap();
    assert_eq!(item.body(), body);

    let serialized = serialize_item(&item).unwrap();
    assert!(serialized.ends_with(body));
    assert_eq!(parse_item(Path::new(path), &serialized).unwrap(), item);
}

#[test]
fn context_validates_document_and_ticket_comment_threads() {
    let document = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let ticket = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    let document_root = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
    let document_reply = "01ARZ3NDEKTSV4RRFFQ69G5FAY";
    let ticket_root = "01ARZ3NDEKTSV4RRFFQ69G5FAZ";
    let ticket_reply = "01K6YQ1Z2V6B8N4M3R5T7W9X0A";
    let context = validate_context([
        source("docs/guide.md", document_source_with_id(document)),
        source(&ticket_path(ticket), ticket_source(ticket)),
        source(
            &comment_path(document, document_root),
            comment(document_root, document, None, "2026-09-30T12:00:00Z"),
        ),
        source(
            &comment_path(document, document_reply),
            comment(
                document_reply,
                document,
                Some(document_root),
                "2026-09-30T12:01:00Z",
            ),
        ),
        source(
            &comment_path(ticket, ticket_root),
            comment(ticket_root, ticket, None, "2026-09-30T12:02:00Z"),
        ),
        source(
            &comment_path(ticket, ticket_reply),
            comment(
                ticket_reply,
                ticket,
                Some(ticket_root),
                "2026-09-30T12:03:00Z",
            ),
        ),
    ]);

    assert!(context.problems.is_empty());
    assert_eq!(context.items.len(), 6);
    let threads = ordered_comment_threads(&context);
    assert_eq!(thread_ids(&threads), vec![document_root, ticket_root]);
    assert_eq!(thread_ids(&threads[0].replies), vec![document_reply]);
    assert_eq!(thread_ids(&threads[1].replies), vec![ticket_reply]);
}

#[test]
fn context_reports_duplicate_ids_across_kinds_and_retains_unrelated_items() {
    let duplicate = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let independent = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    let context = validate_context([
        source("docs/duplicate.md", document_source_with_id(duplicate)),
        source(&ticket_path(duplicate), ticket_source(duplicate)),
        source(
            &comment_path(independent, duplicate),
            comment(duplicate, independent, None, "2026-09-30T12:00:00Z"),
        ),
        source("docs/independent.md", document_source_with_id(independent)),
    ]);

    assert_eq!(
        problem_paths(&context, ValidationCode::DuplicateId),
        vec![
            PathBuf::from("docs/duplicate.md"),
            PathBuf::from(ticket_path(duplicate)),
            PathBuf::from(comment_path(independent, duplicate)),
        ]
    );
    assert_eq!(context.items.len(), 1);
    assert!(matches!(
        context.items.as_slice(),
        [CanonicalItem::Document(document)] if document.id.to_string() == independent
    ));
}

#[test]
fn context_reports_invalid_comment_relationships_and_cycles() {
    let document = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let other_document = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    let missing_item = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
    let missing_parent = "01ARZ3NDEKTSV4RRFFQ69G5FAY";
    let cross_parent = "01ARZ3NDEKTSV4RRFFQ69G5FAZ";
    let other_root = "01K6YQ1Z2V6B8N4M3R5T7W9X0A";
    let cycle_a = "01K6YQ2A4D8F1H3J5K7M9N0P2Q";
    let cycle_b = "01K6YQ3B6E9G2J4K6M8N0P2R4S";
    let cycle_c = "01K6YQ4C7F0H3J5M7N9P1R3T5V";
    let context = validate_context([
        source("docs/one.md", document_source_with_id(document)),
        source("docs/two.md", document_source_with_id(other_document)),
        source(
            &comment_path(missing_item, "01K6YQ5D8G1J4K6N8P0R2T4V6X"),
            comment(
                "01K6YQ5D8G1J4K6N8P0R2T4V6X",
                missing_item,
                None,
                "2026-09-30T12:00:00Z",
            ),
        ),
        source(
            &comment_path(document, missing_parent),
            comment(
                missing_parent,
                document,
                Some("01K6YQ6E9H2K5M7P9R1T3V5X7Z"),
                "2026-09-30T12:00:00Z",
            ),
        ),
        source(
            &comment_path(other_document, other_root),
            comment(other_root, other_document, None, "2026-09-30T12:00:00Z"),
        ),
        source(
            &comment_path(document, cross_parent),
            comment(
                cross_parent,
                document,
                Some(other_root),
                "2026-09-30T12:00:00Z",
            ),
        ),
        source(
            &comment_path(document, cycle_a),
            comment(cycle_a, document, Some(cycle_b), "2026-09-30T12:00:00Z"),
        ),
        source(
            &comment_path(document, cycle_b),
            comment(cycle_b, document, Some(cycle_c), "2026-09-30T12:00:00Z"),
        ),
        source(
            &comment_path(document, cycle_c),
            comment(cycle_c, document, Some(cycle_a), "2026-09-30T12:00:00Z"),
        ),
    ]);

    assert_eq!(
        problem_paths(&context, ValidationCode::MissingCommentItem),
        vec![PathBuf::from(comment_path(
            missing_item,
            "01K6YQ5D8G1J4K6N8P0R2T4V6X"
        ))]
    );
    assert_eq!(
        problem_paths(&context, ValidationCode::MissingParent),
        vec![PathBuf::from(comment_path(document, missing_parent))]
    );
    assert_eq!(
        problem_paths(&context, ValidationCode::CrossItemParent),
        vec![PathBuf::from(comment_path(document, cross_parent))]
    );
    assert_eq!(
        problem_paths(&context, ValidationCode::CommentCycle),
        vec![
            PathBuf::from(comment_path(document, cycle_a)),
            PathBuf::from(comment_path(document, cycle_b)),
            PathBuf::from(comment_path(document, cycle_c))
        ]
    );
    assert_eq!(context.items.len(), 3);
}

#[test]
fn context_excludes_descendants_of_nonconforming_comments() {
    let document = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let independent_document = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    let invalid_parent = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
    let descendant = "01ARZ3NDEKTSV4RRFFQ69G5FAY";
    let independent_root = "01ARZ3NDEKTSV4RRFFQ69G5FAZ";
    let context = validate_context([
        source("docs/guide.md", document_source_with_id(document)),
        source(
            "docs/independent.md",
            document_source_with_id(independent_document),
        ),
        source(
            &comment_path(document, invalid_parent),
            comment(
                invalid_parent,
                document,
                Some("01K6YQ1Z2V6B8N4M3R5T7W9X0A"),
                "2026-09-30T12:00:00Z",
            ),
        ),
        source(
            &comment_path(document, descendant),
            comment(
                descendant,
                document,
                Some(invalid_parent),
                "2026-09-30T12:01:00Z",
            ),
        ),
        source(
            &comment_path(independent_document, independent_root),
            comment(
                independent_root,
                independent_document,
                None,
                "2026-09-30T12:02:00Z",
            ),
        ),
    ]);

    assert_eq!(
        problem_paths(&context, ValidationCode::MissingParent),
        vec![
            PathBuf::from(comment_path(document, invalid_parent)),
            PathBuf::from(comment_path(document, descendant)),
        ]
    );
    assert!(context.problems.iter().any(|problem| {
        problem.path == comment_path(document, descendant)
            && problem.code == ValidationCode::MissingParent
            && problem.message.contains("nonconforming")
    }));
    assert_eq!(context.items.len(), 3);
    assert!(context.items.iter().any(|item| {
        matches!(item, CanonicalItem::Document(value) if value.id.to_string() == document)
    }));
    assert!(context.items.iter().any(|item| {
        matches!(item, CanonicalItem::Document(value) if value.id.to_string() == independent_document)
    }));
    let threads = ordered_comment_threads(&context);
    assert_eq!(thread_ids(&threads), vec![independent_root]);
    assert_eq!(all_thread_ids(&threads), vec![independent_root]);
}

#[test]
fn context_retains_valid_items_when_another_source_is_malformed() {
    let document = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let context = validate_context([
        source("docs/broken.md", "not front matter".to_owned()),
        source("docs/guide.md", document_source_with_id(document)),
    ]);

    assert_eq!(
        problem_paths(&context, ValidationCode::MissingFrontMatter),
        vec![PathBuf::from("docs/broken.md")]
    );
    assert!(matches!(
        context.items.as_slice(),
        [CanonicalItem::Document(value)]
            if value.id.to_string() == document && value.title == "Guide" && value.body.is_empty()
    ));
}

#[test]
fn comment_order_is_deterministic_after_serialization_and_reparse() {
    let document = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let root_later = "01ARZ3NDEKTSV4RRFFQ69G5FAZ";
    let root_early_high = "01K6YQ3B6E9G2J4K6M8N0P2R4S";
    let root_early_low = "01K6YQ2A4D8F1H3J5K7M9N0P2Q";
    let reply_later = "01K6YQ5D8G1J4K6N8P0R2T4V6X";
    let reply_early_high = "01K6YQ6E9H2K5M7P9R1T3V5X7Z";
    let reply_early_low = "01K6YQ4C7F0H3J5M7N9P1R3T5V";
    let sources = vec![
        source("docs/guide.md", document_source_with_id(document)),
        source(
            &comment_path(document, reply_later),
            comment(
                reply_later,
                document,
                Some(root_early_low),
                "2026-09-30T12:02:00Z",
            ),
        ),
        source(
            &comment_path(document, root_later),
            comment(root_later, document, None, "2026-09-30T12:01:00Z"),
        ),
        source(
            &comment_path(document, reply_early_high),
            comment(
                reply_early_high,
                document,
                Some(root_early_low),
                "2026-09-30T12:00:00Z",
            ),
        ),
        source(
            &comment_path(document, root_early_high),
            comment(root_early_high, document, None, "2026-09-30T12:00:00Z"),
        ),
        source(
            &comment_path(document, reply_early_low),
            comment(
                reply_early_low,
                document,
                Some(root_early_low),
                "2026-09-30T12:00:00Z",
            ),
        ),
        source(
            &comment_path(document, root_early_low),
            comment(root_early_low, document, None, "2026-09-30T12:00:00Z"),
        ),
    ];
    let context = validate_context(sources.clone());
    let expected_roots = vec![root_early_low, root_early_high, root_later]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let expected_replies = vec![reply_early_low, reply_early_high, reply_later]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    assert_eq!(
        thread_ids(&ordered_comment_threads(&context)),
        expected_roots
    );
    assert_eq!(
        thread_ids(&ordered_comment_threads(&context)[0].replies),
        expected_replies
    );

    let reparsed_sources = context
        .items
        .iter()
        .map(|item| {
            let path = match item {
                CanonicalItem::Document(_) => PathBuf::from("docs/guide.md"),
                CanonicalItem::Comment(comment) => {
                    comment_path(document, &comment.id.to_string()).into()
                }
                CanonicalItem::Ticket(_) => unreachable!(),
            };
            (path, serialize_item(item).unwrap())
        })
        .collect::<Vec<_>>();
    let reparsed = validate_context(reparsed_sources);

    assert_eq!(
        thread_ids(&ordered_comment_threads(&reparsed)),
        expected_roots
    );
    assert_eq!(
        thread_ids(&ordered_comment_threads(&reparsed)[0].replies),
        expected_replies
    );
}

fn source(path: &str, contents: String) -> (PathBuf, String) {
    (PathBuf::from(path), contents)
}

fn document_source_with_id(id: &str) -> String {
    format!("---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: Guide\n---\n")
}

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

fn comment_path(item_id: &str, id: &str) -> String {
    format!(".manyhands/comments/{item_id}/{id}.md")
}

fn comment(id: &str, item_id: &str, parent_id: Option<&str>, created_at: &str) -> String {
    let parent =
        parent_id.map_or_else(String::new, |parent_id| format!("parent_id: {parent_id}\n"));
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {item_id}\n{parent}created_at: {created_at}\n---\n"
    )
}

fn thread_ids(threads: &[manyhands::canonical::CommentThread]) -> Vec<String> {
    threads
        .iter()
        .map(|thread| thread.comment.id.to_string())
        .collect()
}

fn all_thread_ids(threads: &[manyhands::canonical::CommentThread]) -> Vec<String> {
    let mut ids = Vec::new();
    for thread in threads {
        ids.push(thread.comment.id.to_string());
        ids.extend(all_thread_ids(&thread.replies));
    }
    ids
}

fn problem_paths(
    context: &manyhands::canonical::ValidatedContext,
    code: ValidationCode,
) -> Vec<PathBuf> {
    context
        .problems
        .iter()
        .filter(|problem| problem.code == code)
        .map(|problem| problem.path.clone())
        .collect()
}

// ---------------------------------------------------------------------
// Ticket relationships: `slug`, `parent` and `deps`.
// ---------------------------------------------------------------------

const RELATED_TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const RELATED_PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB0";
const RELATED_FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
const RELATED_SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB2";

fn parsed_ticket(source: &str) -> Ticket {
    match parse_item(Path::new(&ticket_path(RELATED_TICKET)), source).unwrap() {
        CanonicalItem::Ticket(ticket) => ticket,
        other => panic!("not a ticket: {other:?}"),
    }
}

/// A ticket whose front matter ends with `extra`: lines that each end in a
/// newline.
fn related_ticket(extra: &str) -> Ticket {
    parsed_ticket(&format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {RELATED_TICKET}\n\
         title: Related\ntype: task\nstatus: open\n{extra}---\nBody.\n"
    ))
}

fn related(extra: &str) -> TicketRelationships {
    ticket_relationships(&related_ticket(extra))
}

fn id(text: &str) -> ItemId {
    text.parse().unwrap()
}

fn relationship_problem(
    code: RelationshipProblemCode,
    detail: Option<&str>,
) -> RelationshipProblem {
    RelationshipProblem {
        code,
        detail: detail.map(id),
    }
}

#[test]
fn a_ticket_without_relationship_fields_has_an_empty_view() {
    assert_eq!(related(""), TicketRelationships::default());
    assert_eq!(related("deps: []\n"), TicketRelationships::default());
}

#[test]
fn the_view_reads_valid_fields_in_block_and_flow_form() {
    let expected = TicketRelationships {
        slug: Some("mh-vh-k9x2b".to_owned()),
        parent: Some(id(RELATED_PARENT)),
        // In the file's order, which need not be sorted.
        deps: vec![id(RELATED_SECOND), id(RELATED_FIRST)],
        problems: Vec::new(),
    };

    let block = related(&format!(
        "slug: mh-vh-k9x2b\nparent: \"{RELATED_PARENT}\"\ndeps:\n  - \"{RELATED_SECOND}\"\n  - \"{RELATED_FIRST}\"\n"
    ));
    let flow = related(&format!(
        "slug: \"mh-vh-k9x2b\"\nparent: {RELATED_PARENT}\ndeps: [{RELATED_SECOND}, \"{RELATED_FIRST}\"]\n"
    ));

    assert_eq!(block, expected);
    assert_eq!(flow, expected);
}

#[test]
fn the_relationship_keys_stay_in_the_unknown_mapping() {
    let ticket = related_ticket(&format!(
        "slug: mh-vh-k9x2b\nparent: {RELATED_PARENT}\ndeps: [{RELATED_FIRST}]\nother: kept\n"
    ));

    for key in ["slug", "parent", "deps", "other"] {
        assert!(ticket.unknown.contains_key(key), "{key}");
    }
    assert_eq!(RELATIONSHIP_KEYS, ["slug", "parent", "deps"]);
}

#[test]
fn slugs_follow_the_grammar() {
    for valid in [
        "vh-k9x2b",
        "mh-vh-k9x2b",
        "a-12-00000",
        "abcdefgh-abc-zzzzzzzz",
        "mh-vh-0123456z",
        "1-22-abcde",
    ] {
        assert!(is_valid_slug(valid), "{valid}");
    }
    for invalid in [
        "",
        "k9x2b",
        "v-k9x2b",
        "vhvh-k9x2b",
        "vh-k9x2",
        "vh-k9x2b0000",
        "mh-vh-k9x2b-extra",
        "abcdefghi-vh-k9x2b",
        "-vh-k9x2b",
        "vh--k9x2b",
        "vh-k9x2b-",
        // Crockford Base32 has no i, l, o or u.
        "vh-k9i2b",
        "vh-k9l2b",
        "vh-k9o2b",
        "vh-k9u2b",
        // Lowercase only: a slug is stored the way it is displayed.
        "VH-K9X2B",
        "mh-Vh-k9x2b",
        "vh-k9x2B",
        "vh_k9x2b",
        "vh-k9x2b\n",
        " vh-k9x2b",
        "vé-k9x2b",
    ] {
        assert!(!is_valid_slug(invalid), "{invalid:?}");
    }
}

#[test]
fn a_malformed_slug_is_ignored_and_reported() {
    for extra in [
        "slug: Not-A-Slug\n",
        "slug: 12\n",
        "slug: [vh-k9x2b]\n",
        // Unquoted, YAML reads these as a number and a boolean.
        "slug: 1e-12345\n",
        "slug: true\n",
        "slug: \"\"\n",
        "slug: VH-K9I2B\n",
        "slug: \" vh-k9x2b\"\n",
    ] {
        assert_eq!(
            related(extra),
            TicketRelationships {
                problems: vec![relationship_problem(
                    RelationshipProblemCode::InvalidSlug,
                    None
                )],
                ..Default::default()
            },
            "{extra:?}"
        );
    }
}

#[test]
fn a_short_code_written_in_uppercase_is_read_in_lowercase() {
    for written in ["MH-VH-K9X2B", "Mh-vH-k9X2b", "\"1E-12345\""] {
        let ticket = related_ticket(&format!("slug: {written}\n"));
        let view = ticket_relationships(&ticket);

        assert_eq!(
            view.slug.as_deref(),
            Some(written.trim_matches('"').to_lowercase().as_str())
        );
        assert!(view.problems.is_empty(), "{written}");
        // The file's own spelling is what a save writes back.
        let serialized = serialize_item(&CanonicalItem::Ticket(ticket)).unwrap();
        assert!(
            serialized.contains(written.trim_matches('"')),
            "{serialized}"
        );
    }
    assert_eq!(
        normalized_slug("MH-VH-K9X2B").as_deref(),
        Some("mh-vh-k9x2b")
    );
    assert_eq!(normalized_slug("vh-k9x2b").as_deref(), Some("vh-k9x2b"));
    for invalid in ["VH-K9I2B", "VH_K9X2B", "VÉ-K9X2B", "", "K9X2B"] {
        assert_eq!(normalized_slug(invalid), None, "{invalid:?}");
    }
}

#[test]
fn a_key_with_a_null_value_is_read_as_absent() {
    for extra in [
        "slug: ~\n",
        "slug:\n",
        "slug: null\n",
        "parent: ~\n",
        "parent:\n",
        "deps: ~\n",
        "deps:\n",
        "slug:\nparent:\ndeps:\n",
    ] {
        assert_eq!(related(extra), TicketRelationships::default(), "{extra:?}");
    }
    // A null entry of a list is not an absent list.
    assert_eq!(
        related("deps: [~]\n").problems,
        [relationship_problem(
            RelationshipProblemCode::WrongType,
            None
        )]
    );
}

#[test]
fn a_value_of_the_wrong_type_is_ignored_and_reported() {
    let wrong_type = || relationship_problem(RelationshipProblemCode::WrongType, None);
    for extra in [
        format!("deps: {RELATED_FIRST}\n"),
        "deps: 7\n".to_owned(),
        format!("deps:\n  first: {RELATED_FIRST}\n"),
        "parent: 7\n".to_owned(),
        format!("parent: [{RELATED_PARENT}]\n"),
    ] {
        assert_eq!(
            related(&extra),
            TicketRelationships {
                problems: vec![wrong_type()],
                ..Default::default()
            },
            "{extra:?}"
        );
    }

    // One bad entry does not cost the list its good ones.
    assert_eq!(
        related(&format!(
            "deps: [{RELATED_FIRST}, 7, [{RELATED_PARENT}], {RELATED_SECOND}, ~]\n"
        )),
        TicketRelationships {
            deps: vec![id(RELATED_FIRST), id(RELATED_SECOND)],
            problems: vec![wrong_type(), wrong_type(), wrong_type()],
            ..Default::default()
        }
    );
}

#[test]
fn a_value_that_is_not_an_item_id_is_ignored_and_reported() {
    let invalid_id = || relationship_problem(RelationshipProblemCode::InvalidId, None);
    let lowercase = RELATED_FIRST.to_lowercase();

    assert_eq!(
        related(&format!(
            "parent: mh-vh-k9x2b\ndeps: [{lowercase}, {RELATED_FIRST}, \"\", {RELATED_FIRST}X]\n"
        )),
        TicketRelationships {
            deps: vec![id(RELATED_FIRST)],
            problems: vec![invalid_id(), invalid_id(), invalid_id(), invalid_id()],
            ..Default::default()
        }
    );
}

#[test]
fn a_reference_to_the_ticket_itself_is_ignored_and_reported() {
    let itself =
        || relationship_problem(RelationshipProblemCode::SelfReference, Some(RELATED_TICKET));

    assert_eq!(
        related(&format!(
            "parent: {RELATED_TICKET}\ndeps: [{RELATED_TICKET}, {RELATED_FIRST}]\n"
        )),
        TicketRelationships {
            deps: vec![id(RELATED_FIRST)],
            problems: vec![itself(), itself()],
            ..Default::default()
        }
    );
}

#[test]
fn a_repeated_dependency_is_kept_once_and_reported() {
    let duplicate =
        |id| relationship_problem(RelationshipProblemCode::DuplicateDependency, Some(id));

    assert_eq!(
        related(&format!(
            "parent: {RELATED_FIRST}\ndeps: [{RELATED_SECOND}, {RELATED_FIRST}, {RELATED_SECOND}, {RELATED_SECOND}]\n"
        )),
        TicketRelationships {
            // A parent may also be a dependency.
            parent: Some(id(RELATED_FIRST)),
            deps: vec![id(RELATED_SECOND), id(RELATED_FIRST)],
            problems: vec![duplicate(RELATED_SECOND), duplicate(RELATED_SECOND)],
            ..Default::default()
        }
    );
}

#[test]
fn every_kind_of_invalid_value_is_reported_while_the_valid_ones_are_kept() {
    let view = related(&format!(
        "slug: nope\nparent: {RELATED_TICKET}\ndeps: [{RELATED_FIRST}, {RELATED_FIRST}, x, 1]\n"
    ));

    assert_eq!(view.slug, None);
    assert_eq!(view.parent, None);
    assert_eq!(view.deps, [id(RELATED_FIRST)]);
    let mut codes: Vec<_> = view.problems.iter().map(|problem| problem.code).collect();
    codes.sort_by_key(|code| {
        RelationshipProblemCode::ALL
            .iter()
            .position(|all| all == code)
    });
    assert_eq!(codes, RelationshipProblemCode::ALL);
}

#[test]
fn relationship_fields_keep_their_values_through_serialization() {
    let mut ticket = related_ticket(&format!(
        "slug: mh-vh-k9x2b\nparent: {RELATED_PARENT}\ndeps: [{RELATED_SECOND}, {RELATED_FIRST}]\nother: kept\n"
    ));
    let before = ticket_relationships(&ticket);
    assert_eq!(before.deps.len(), 2);
    assert!(before.problems.is_empty());
    ticket.title = "Renamed".to_owned();

    let serialized = serialize_item(&CanonicalItem::Ticket(ticket)).unwrap();

    let reparsed = parsed_ticket(&serialized);
    // The values survive; the dependencies come back in the canonical
    // order, sorted by ID, and not in the order the file had them.
    let canonical = TicketRelationships {
        deps: vec![id(RELATED_FIRST), id(RELATED_SECOND)],
        ..before.clone()
    };
    assert_ne!(canonical, before);
    assert_eq!(ticket_relationships(&reparsed), canonical);
    assert_eq!(reparsed.title, "Renamed");
    // Each key is written once.
    for key in ["slug:", "parent:", "deps:", "other:"] {
        assert_eq!(
            serialized
                .lines()
                .filter(|line| line.starts_with(key))
                .count(),
            1,
            "{key} in {serialized}"
        );
    }
    // In the serializer's own form the file comes back byte for byte.
    assert_eq!(
        serialize_item(&CanonicalItem::Ticket(reparsed)).unwrap(),
        serialized
    );

    // Invalid values are kept in the file too: nothing is repaired.
    let invalid = related_ticket("slug: Not-A-Slug\ndeps: 7\nparent: [x]\n");
    let unknown = invalid.unknown.clone();
    let serialized = serialize_item(&CanonicalItem::Ticket(invalid)).unwrap();
    assert_eq!(parsed_ticket(&serialized).unknown, unknown);
}

/// The front matter lines `serialize_item` writes in place of `extra` for a
/// ticket laid out as the serializer lays one out: `extra` first, then the
/// fields every ticket has. The parser keeps the order of the other keys
/// only for a file in that layout.
fn serialized_extra(extra: &str) -> String {
    let required = format!(
        "id: {RELATED_TICKET}\ntitle: Related\ntype: task\nstatus: open\n\
         manyhands_managed: true\nmanyhands_kind: ticket\n"
    );
    let ticket = parsed_ticket(&format!("---\n{extra}{required}---\nBody.\n"));
    let serialized = serialize_item(&CanonicalItem::Ticket(ticket)).unwrap();
    serialized
        .strip_prefix("---\n")
        .and_then(|rest| rest.strip_suffix("---\nBody.\n"))
        .and_then(|front_matter| front_matter.strip_suffix(required.as_str()))
        .unwrap_or_else(|| panic!("unexpected layout in {serialized}"))
        .to_owned()
}

#[test]
fn valid_dependencies_are_written_as_a_sorted_block_sequence() {
    let canonical = format!("deps:\n- {RELATED_PARENT}\n- {RELATED_FIRST}\n- {RELATED_SECOND}\n");
    for extra in [
        format!("deps: [{RELATED_SECOND}, {RELATED_PARENT}, {RELATED_FIRST}]\n"),
        format!("deps:\n  - {RELATED_FIRST}\n  - {RELATED_SECOND}\n  - {RELATED_PARENT}\n"),
        canonical.clone(),
    ] {
        assert_eq!(serialized_extra(&extra), canonical, "{extra:?}");
    }
}

#[test]
fn dependencies_keep_their_place_among_the_other_keys_when_sorted() {
    assert_eq!(
        serialized_extra(&format!(
            "zeta: 1\ndeps: [{RELATED_SECOND}, {RELATED_FIRST}]\nalpha: kept\n"
        )),
        format!("zeta: 1\ndeps:\n- {RELATED_FIRST}\n- {RELATED_SECOND}\nalpha: kept\n")
    );
}

#[test]
fn an_empty_or_null_relationship_is_written_by_omitting_the_key() {
    for extra in ["deps: []\n", "deps: ~\n", "parent: ~\n", "deps:\nparent:\n"] {
        assert_eq!(
            serialized_extra(&format!("before: 1\n{extra}after: 2\n")),
            "before: 1\nafter: 2\n",
            "{extra:?}"
        );
    }
    // A null short code is not a relationship: it stays for the save that
    // assigns one.
    assert_eq!(serialized_extra("slug: ~\n"), "slug: null\n");
}

#[test]
fn dependencies_with_a_problem_are_written_as_they_were_read() {
    for extra in [
        // A repeated entry.
        format!("deps: [{RELATED_SECOND}, {RELATED_FIRST}, {RELATED_SECOND}]\n"),
        // An entry that is not an item ID.
        format!("deps: [{RELATED_SECOND}, x, {RELATED_FIRST}]\n"),
        // An entry of the wrong type.
        format!("deps: [{RELATED_SECOND}, 7, {RELATED_FIRST}]\n"),
        // The ticket itself.
        format!("deps: [{RELATED_TICKET}, {RELATED_FIRST}]\n"),
    ] {
        let ticket = related_ticket(&extra);
        assert!(!ticket_relationships(&ticket).problems.is_empty());
        let unknown = ticket.unknown.clone();
        let serialized = serialize_item(&CanonicalItem::Ticket(ticket)).unwrap();
        let reparsed = parsed_ticket(&serialized).unknown;
        assert_eq!(reparsed, unknown, "{extra:?}");
        // Mapping equality ignores nothing inside a sequence: the entries
        // are in the order they were read.
        assert_eq!(reparsed.get("deps"), unknown.get("deps"), "{extra:?}");
    }
}

/// A short code and item IDs that a YAML reader takes for numbers when
/// they are not quoted.
const NUMERIC_SLUG: &str = "1e-12345";
const NUMERIC_ID: &str = "01234567890123456789012345";
const NUMERIC_TICKET: &str = "01234567890123456789012346";
/// The bytes the serializer emits for them, pinned.
const NUMERIC_GOLDEN: &str = "---\n\
slug: '1e-12345'\n\
parent: '01234567890123456789012345'\n\
deps:\n\
- '01234567890123456789012345'\n\
- 01ARZ3NDEKTSV4RRFFQ69G5FB1\n\
id: '01234567890123456789012346'\n\
title: Numbers\n\
type: task\n\
status: open\n\
manyhands_managed: true\n\
manyhands_kind: ticket\n\
---\n\
Body.\n";

#[test]
fn strings_that_read_as_numbers_are_written_so_that_they_read_back_as_strings() {
    let mut unknown = serde_yaml::Mapping::new();
    unknown.insert("slug".into(), NUMERIC_SLUG.into());
    unknown.insert("parent".into(), NUMERIC_ID.into());
    unknown.insert(
        "deps".into(),
        serde_yaml::Value::Sequence(vec![RELATED_FIRST.into(), NUMERIC_ID.into()]),
    );
    let ticket = Ticket {
        id: id(NUMERIC_TICKET),
        title: "Numbers".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: None,
        closed_at: None,
        closed_by: None,
        body: "Body.\n".to_owned(),
        unknown,
    };

    let serialized = serialize_item(&CanonicalItem::Ticket(ticket)).unwrap();

    assert_eq!(serialized, NUMERIC_GOLDEN);
    let CanonicalItem::Ticket(reparsed) =
        parse_item(Path::new(&ticket_path(NUMERIC_TICKET)), &serialized).unwrap()
    else {
        panic!("not a ticket");
    };
    assert_eq!(reparsed.id, id(NUMERIC_TICKET));
    assert_eq!(
        ticket_relationships(&reparsed),
        TicketRelationships {
            slug: Some(NUMERIC_SLUG.to_owned()),
            parent: Some(id(NUMERIC_ID)),
            deps: vec![id(NUMERIC_ID), id(RELATED_FIRST)],
            problems: Vec::new(),
        }
    );
    // Unquoted, the same text is a number and not a short code.
    assert_eq!(
        related(&format!("slug: {NUMERIC_SLUG}\n")).problems,
        [relationship_problem(
            RelationshipProblemCode::InvalidSlug,
            None
        )]
    );
}

#[test]
fn a_document_keeps_the_relationship_keys_as_ordinary_unknown_metadata() {
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {RELATED_TICKET}\ntitle: D\nslug: mh-vh-k9x2b\ndeps: [{RELATED_FIRST}]\n---\n"
    );
    let CanonicalItem::Document(document) = parse_item(Path::new("docs/d.md"), &source).unwrap()
    else {
        panic!("not a document");
    };
    assert!(document.unknown.contains_key("slug"));
    assert!(document.unknown.contains_key("deps"));
}

fn slug_vectors() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slug_v1/vectors.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn short_codes_match_the_independently_computed_vectors() {
    let vectors = slug_vectors();
    assert!(!vectors["method"].as_str().unwrap().is_empty());
    let cases = vectors["codes"].as_array().unwrap();
    assert!(cases.len() >= 8);
    for case in cases {
        let ulid = case["ulid"].as_str().unwrap();
        for length in 5..=8 {
            assert_eq!(
                short_code(ulid, length),
                case["codes"][length.to_string()].as_str().unwrap(),
                "{ulid} at length {length}"
            );
        }
    }
}

#[test]
fn a_longer_short_code_begins_with_the_shorter_one() {
    for case in slug_vectors()["codes"].as_array().unwrap() {
        let ulid = case["ulid"].as_str().unwrap();
        for length in 6..=8 {
            assert!(short_code(ulid, length).starts_with(&short_code(ulid, 5)));
            assert_eq!(short_code(ulid, length).len(), length);
        }
    }
}

#[test]
fn initials_follow_the_vectors() {
    let vectors = slug_vectors();
    let cases = vectors["initials"].as_array().unwrap();
    assert!(cases.len() >= 8);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert_eq!(
            derive_initials(name).as_deref(),
            case["initials"].as_str(),
            "{name:?}"
        );
    }
}

#[test]
fn normalize_initials_lowercases_and_accepts_two_or_three_characters() {
    assert_eq!(normalize_initials("VH").as_deref(), Ok("vh"));
    assert_eq!(normalize_initials("aB9").as_deref(), Ok("ab9"));
}

#[test]
fn normalize_initials_rejects_everything_else() {
    for text in ["v", "abcd", "", "v.", "a b", "é1", "vé", "-ab", "v\n"] {
        assert_eq!(
            normalize_initials(text),
            Err(InitialsProblem::Invalid),
            "{text:?}"
        );
    }
}

fn config_with(extra: &str) -> RepositoryConfig {
    parse_repository_config(&format!(
        "format_version = 1\nprimary_branch = \"main\"\n{extra}"
    ))
    .unwrap()
}

#[test]
fn slug_settings_default_to_no_prefix_and_five_characters() {
    assert_eq!(slug_settings(&config_with("")), Ok((None, 5)));
}

#[test]
fn slug_settings_accept_a_valid_prefix_and_each_length() {
    for length in 5..=8 {
        let config = config_with(&format!(
            "ticket_slug_prefix = \"mh2\"\nticket_slug_code_length = {length}\n"
        ));
        assert_eq!(
            slug_settings(&config),
            Ok((Some("mh2".to_string()), length))
        );
    }
    assert_eq!(
        slug_settings(&config_with("ticket_slug_prefix = \"12345678\"\n")),
        Ok((Some("12345678".to_string()), 5))
    );
}

#[test]
fn slug_settings_reject_an_invalid_prefix() {
    for value in [
        "\"toolongpre\"",
        "\"MH\"",
        "\"\"",
        "\"m-h\"",
        "7",
        "[\"mh\"]",
    ] {
        let config = config_with(&format!("ticket_slug_prefix = {value}\n"));
        assert_eq!(
            slug_settings(&config),
            Err(SlugSettingsProblem::InvalidPrefix),
            "{value}"
        );
    }
}

#[test]
fn slug_settings_reject_an_invalid_code_length() {
    for value in ["4", "9", "6.5", "\"6\"", "true", "-6"] {
        let config = config_with(&format!("ticket_slug_code_length = {value}\n"));
        assert_eq!(
            slug_settings(&config),
            Err(SlugSettingsProblem::InvalidCodeLength),
            "{value}"
        );
    }
}

#[test]
fn composed_slugs_are_valid_short_codes() {
    let vectors = slug_vectors();
    for case in vectors["codes"].as_array().unwrap() {
        let ulid = case["ulid"].as_str().unwrap();
        for length in 5..=8 {
            let code = short_code(ulid, length);
            for prefix in [None, Some("mh"), Some("12345678")] {
                for initials in ["vh", "a9z"] {
                    let slug = compose_slug(prefix, initials, &code);
                    assert!(is_valid_slug(&slug), "{slug}");
                    assert!(slug.ends_with(&format!("-{code}")));
                }
            }
        }
    }
    assert_eq!(compose_slug(Some("mh"), "vh", "k9x2b"), "mh-vh-k9x2b");
    assert_eq!(compose_slug(None, "vh", "k9x2b"), "vh-k9x2b");
}

const AUTHORED_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAY";
const AUTHORED_ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const AUTHOR: &str = "Manyhands Test <manyhands-test@example.invalid>";
/// The bytes the serializer emits for a comment that names its author,
/// pinned.
const AUTHORED_COMMENT_GOLDEN: &str = "---\n\
id: 01ARZ3NDEKTSV4RRFFQ69G5FAY\n\
item_id: 01ARZ3NDEKTSV4RRFFQ69G5FAV\n\
created_at: 2026-09-30T12:34:56Z\n\
created_by: Manyhands Test <manyhands-test@example.invalid>\n\
manyhands_managed: true\n\
manyhands_kind: comment\n\
---\n\
Body.\n";

fn authored_comment(created_by: Option<&str>) -> manyhands::canonical::Comment {
    manyhands::canonical::Comment {
        id: id(AUTHORED_COMMENT),
        item_id: id(AUTHORED_ITEM),
        parent_id: None,
        created_at: time::OffsetDateTime::from_unix_timestamp(1_790_771_696).unwrap(),
        created_by: created_by.map(str::to_owned),
        body: "Body.\n".to_owned(),
        unknown: serde_yaml::Mapping::new(),
    }
}

fn parsed_comment(extra: &str) -> manyhands::canonical::Comment {
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {AUTHORED_COMMENT}\nitem_id: {AUTHORED_ITEM}\ncreated_at: 2026-09-30T12:34:56Z\n{extra}---\nBody.\n"
    );
    let CanonicalItem::Comment(comment) = parse_item(
        Path::new(&comment_path(AUTHORED_ITEM, AUTHORED_COMMENT)),
        &source,
    )
    .unwrap() else {
        panic!("not a comment");
    };
    comment
}

#[test]
fn a_comments_author_is_written_as_created_by_and_read_back_into_the_field() {
    let comment = authored_comment(Some(AUTHOR));

    let serialized = serialize_item(&CanonicalItem::Comment(comment.clone())).unwrap();

    assert_eq!(serialized, AUTHORED_COMMENT_GOLDEN);
    let CanonicalItem::Comment(reparsed) = parse_item(
        Path::new(&comment_path(AUTHORED_ITEM, AUTHORED_COMMENT)),
        &serialized,
    )
    .unwrap() else {
        panic!("not a comment");
    };
    assert_eq!(reparsed.created_by.as_deref(), Some(AUTHOR));
    assert!(reparsed.unknown.is_empty());
    assert_eq!(reparsed, comment);
}

#[test]
fn a_comment_without_created_by_is_conforming_and_names_nobody() {
    let comment = parsed_comment("");

    assert_eq!(comment.created_by, None);
    assert!(comment.unknown.is_empty());
    let serialized = serialize_item(&CanonicalItem::Comment(comment)).unwrap();
    assert!(!serialized.contains("created_by"), "{serialized}");
}

#[test]
fn a_created_by_that_is_no_identity_stays_in_the_unknown_mapping() {
    let cases: [(&str, serde_yaml::Value); 5] = [
        (
            "created_by:\n  - ada\n",
            serde_yaml::Value::Sequence(vec!["ada".into()]),
        ),
        ("created_by: 7\n", 7.into()),
        ("created_by: \"\"\n", "".into()),
        ("created_by: \"a\\0b\"\n", "a\0b".into()),
        ("created_by: null\n", serde_yaml::Value::Null),
    ];
    for (extra, value) in cases {
        let comment = parsed_comment(extra);

        assert_eq!(comment.created_by, None, "{extra}");
        assert_eq!(comment.unknown.get("created_by"), Some(&value), "{extra}");
        assert_eq!(comment.unknown.len(), 1, "{extra}");
        // Written back as it was read.
        let serialized = serialize_item(&CanonicalItem::Comment(comment.clone())).unwrap();
        let CanonicalItem::Comment(reparsed) = parse_item(
            Path::new(&comment_path(AUTHORED_ITEM, AUTHORED_COMMENT)),
            &serialized,
        )
        .unwrap() else {
            panic!("not a comment");
        };
        assert_eq!(reparsed, comment, "{extra}");
    }
}

#[test]
fn a_comment_author_that_is_empty_or_holds_a_nul_is_not_serialized() {
    for created_by in ["", "a\0b"] {
        let problem = serialize_item(&CanonicalItem::Comment(authored_comment(Some(created_by))))
            .unwrap_err();

        assert_eq!(problem.code, ValidationCode::InvalidField, "{created_by:?}");
    }
}
