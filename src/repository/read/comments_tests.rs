use serde_yaml::{Mapping, Value};

use super::*;

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";

fn unknown(created_by: Option<Value>) -> Mapping {
    let mut unknown = Mapping::new();
    unknown.insert("mood".into(), "calm".into());
    if let Some(created_by) = created_by {
        unknown.insert(CREATED_BY.into(), created_by);
    }
    unknown
}

#[test]
fn the_author_is_taken_out_of_the_unknown_keys_whatever_it_holds() {
    let cases = [
        (None, Ok(None)),
        (
            Some(Value::from("Ada Lovelace <ada@example.invalid>")),
            Ok(Some("Ada Lovelace <ada@example.invalid>".to_owned())),
        ),
        // Kept as written: nothing is trimmed or parsed.
        (Some(Value::from(" ada ")), Ok(Some(" ada ".to_owned()))),
        (Some(Value::from("")), Err(())),
        (Some(Value::from("a\0b")), Err(())),
        (Some(Value::from(7)), Err(())),
        (Some(Value::Null), Err(())),
        (Some(Value::Sequence(vec!["ada".into()])), Err(())),
        (Some(Value::Mapping(Mapping::new())), Err(())),
    ];
    for (created_by, expected) in cases {
        let mut unknown = unknown(created_by.clone());

        assert_eq!(take_author(&mut unknown), expected, "{created_by:?}");

        assert!(!unknown.contains_key(CREATED_BY), "{created_by:?}");
        assert_eq!(unknown.len(), 1);
    }
}

#[test]
fn only_a_file_directly_inside_the_items_comment_directory_is_one_of_its_comments() {
    let directory = comment_directory(ITEM);

    assert_eq!(directory, format!(".manyhands/comments/{ITEM}/"));
    for path in [
        format!("{directory}{COMMENT}.md"),
        format!("{directory}notes.md"),
        format!("{directory}no-extension"),
    ] {
        assert!(is_file_in(&directory, &path), "{path}");
    }
    for path in [
        String::new(),
        directory.clone(),
        directory.trim_end_matches('/').to_owned(),
        format!("{directory}sub/{COMMENT}.md"),
        format!("{directory}../{ITEM}/{COMMENT}.md"),
        format!("{directory}./{COMMENT}.md"),
        format!("{directory}/{COMMENT}.md"),
        format!("{directory}{COMMENT}.md/"),
        format!("{directory}a\\b.md"),
        format!("{directory}a\0b.md"),
        format!("/{directory}{COMMENT}.md"),
        format!(".manyhands/comments/{COMMENT}/{COMMENT}.md"),
        format!(".manyhands/comments/{ITEM}x/{COMMENT}.md"),
        format!(".manyhands/worktrees/{ITEM}/{directory}{COMMENT}.md"),
        format!("docs/{COMMENT}.md"),
    ] {
        assert!(!is_file_in(&directory, &path), "{path:?}");
    }
}

#[test]
fn only_the_code_of_a_validation_problem_reaches_an_entry() {
    const SENTINEL: &str = "SENTINEL-2b9f";
    let directory = comment_directory(ITEM);
    let path = format!("{directory}{COMMENT}.md");
    let item_path = format!(".manyhands/tickets/{ITEM}/ticket.md");
    let found = |path: &str, code| canonical::ValidationProblem {
        path: PathBuf::from(path),
        code,
        message: format!("invalid YAML: {SENTINEL}"),
    };
    let problems = [
        found(&path, canonical::ValidationCode::MalformedFrontMatter),
        found(&path, canonical::ValidationCode::MissingParent),
        // Found twice, listed once.
        found(&path, canonical::ValidationCode::MalformedFrontMatter),
        // Not one of the item's comment files: the item read's to report.
        found(&item_path, canonical::ValidationCode::MissingField),
    ];
    let mut entries = BTreeMap::new();

    add_validation_codes(&mut entries, &problems, &BTreeSet::from([path.clone()]));

    assert_eq!(
        entries,
        BTreeMap::from([(
            path.clone(),
            vec![
                ProblemCode::MalformedFrontMatter,
                ProblemCode::MissingParent
            ]
        )])
    );
    let entry = nonconforming_entry(ITEM, &path, &entries[&path]);
    let serialized = serde_json::to_string(&entry).unwrap();
    assert!(!serialized.contains(SENTINEL), "{serialized}");
    assert!(!serialized.contains("invalid YAML"), "{serialized}");
    assert_eq!(
        serde_json::to_value(&entry.problems).unwrap(),
        serde_json::json!([
            {
                "code": "malformed_front_matter",
                "path": path,
                "target_id": null,
                "guidance": ProblemCode::MalformedFrontMatter.guidance(),
            },
            {
                "code": "missing_parent",
                "path": path,
                "target_id": null,
                "guidance": ProblemCode::MissingParent.guidance(),
            },
        ])
    );
}
