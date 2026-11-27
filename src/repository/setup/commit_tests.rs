use super::*;
use crate::canonical::CONFIG_PATH;
use git2::{FileMode, ObjectType, Tree};
use std::{collections::BTreeMap, path::Path};

const IDENTITY: (&str, &str) = ("Setup Author", "setup-author@example.invalid");

/// Every entry of `tree`, as `path -> (mode, oid)`, directories included.
fn entries(repository: &Repository, tree: &Tree<'_>) -> BTreeMap<String, (i32, Oid)> {
    fn walk(
        repository: &Repository,
        tree: &Tree<'_>,
        prefix: &str,
        into: &mut BTreeMap<String, (i32, Oid)>,
    ) {
        for entry in tree.iter() {
            let path = format!("{prefix}{}", entry.name().unwrap());
            into.insert(path.clone(), (entry.filemode(), entry.id()));
            if entry.kind() == Some(ObjectType::Tree) {
                let child = repository.find_tree(entry.id()).unwrap();
                walk(repository, &child, &format!("{path}/"), into);
            }
        }
    }
    let mut found = BTreeMap::new();
    walk(repository, tree, "", &mut found);
    found
}

/// A repository whose HEAD commit holds `files`, written to its working
/// tree as well, with a repository-local identity.
fn repository_with(files: &[(&str, &[u8])]) -> (tempfile::TempDir, Repository, Oid) {
    let directory = tempfile::tempdir().unwrap();
    let mut options = git2::RepositoryInitOptions::new();
    options.initial_head("main");
    let repository = Repository::init_opts(directory.path(), &options).unwrap();
    let mut config = repository.config().unwrap();
    config.set_str("user.name", "Fixture").unwrap();
    config
        .set_str("user.email", "fixture@example.invalid")
        .unwrap();
    let mut index = repository.index().unwrap();
    for (path, content) in files {
        let full = directory.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, content).unwrap();
        index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let base = repository
        .commit(Some("HEAD"), &signature, &signature, "Base", &tree, &[])
        .unwrap();
    drop(tree);
    (directory, repository, base)
}

fn refs(repository: &Repository) -> Vec<(String, Option<Oid>)> {
    repository
        .references()
        .unwrap()
        .map(|reference| {
            let reference = reference.unwrap();
            (reference.name().unwrap().to_owned(), reference.target())
        })
        .collect()
}

#[test]
fn the_setup_commit_changes_only_the_configuration_of_the_base() {
    let identity = CommitIdentity {
        name: IDENTITY.0.into(),
        email: IDENTITY.1.into(),
    };
    let configuration = b"format_version = 2\nprimary_branch = \"main\"\n";
    // A first-time setup builds on a base with no `.manyhands`; a change on
    // one that holds a configuration and other files beside it, in the same
    // directory and elsewhere.
    type Files = Vec<(&'static str, &'static [u8])>;
    let cases: Vec<(&str, Files, &str)> = vec![
        (
            "first-time",
            vec![("README.md", b"readme"), ("docs/a.md", b"a")],
            ENABLE_MESSAGE,
        ),
        (
            "change",
            vec![
                ("README.md", b"readme"),
                (".manyhands/config.toml", b"old configuration"),
                (".manyhands/tickets/one.md", b"ticket"),
                ("docs/a.md", b"a"),
            ],
            CHANGE_MESSAGE,
        ),
    ];
    for (name, files, message) in cases {
        let (directory, repository, base) = repository_with(&files);
        let head_before = repository.head().unwrap().target();
        let refs_before = refs(&repository);
        let index_before = repository.index().unwrap().write_tree().unwrap();
        let files_before: Vec<_> = files
            .iter()
            .map(|(path, _)| std::fs::read(directory.path().join(path)).unwrap())
            .collect();

        let made = build(&repository, base, configuration, message, &identity).unwrap();

        let commit = repository.find_commit(made).unwrap();
        assert_eq!(
            commit.parent_ids().collect::<Vec<_>>(),
            vec![base],
            "{name}"
        );
        assert_eq!(commit.message(), Some(message), "{name}");
        for signature in [commit.author(), commit.committer()] {
            assert_eq!(signature.name(), Some(IDENTITY.0), "{name}");
            assert_eq!(signature.email(), Some(IDENTITY.1), "{name}");
        }
        // The tree is the base's, but for the one file.
        let base_entries = entries(
            &repository,
            &repository.find_commit(base).unwrap().tree().unwrap(),
        );
        let mut made_entries = entries(&repository, &commit.tree().unwrap());
        let (mode, blob) = made_entries
            .get(CONFIG_PATH)
            .copied()
            .unwrap_or_else(|| panic!("{name}: no configuration"));
        assert_eq!(mode, i32::from(FileMode::Blob), "{name}: mode 100644");
        assert_eq!(
            repository.find_blob(blob).unwrap().content(),
            configuration,
            "{name}"
        );
        // Its directory changes with it, and nothing else does: every other
        // entry is the base's, and nothing is dropped or added.
        let mut expected = base_entries.clone();
        expected.insert(CONFIG_PATH.into(), (mode, blob));
        expected.insert(".manyhands".into(), made_entries[".manyhands"]);
        assert_eq!(made_entries, expected, "{name}");
        made_entries.retain(|path, entry| base_entries.get(path) != Some(entry));
        assert_eq!(
            made_entries.keys().map(String::as_str).collect::<Vec<_>>(),
            vec![".manyhands", CONFIG_PATH],
            "{name}"
        );

        // The repository's own state is untouched: HEAD, every reference,
        // the index, and the working tree.
        assert_eq!(repository.head().unwrap().target(), head_before, "{name}");
        assert_eq!(refs(&repository), refs_before, "{name}");
        assert_eq!(
            repository.index().unwrap().write_tree().unwrap(),
            index_before,
            "{name}"
        );
        let files_after: Vec<_> = files
            .iter()
            .map(|(path, _)| std::fs::read(directory.path().join(path)).unwrap())
            .collect();
        assert_eq!(files_after, files_before, "{name}");
        assert!(repository.statuses(None).unwrap().is_empty(), "{name}");
    }

    // A file where the directory should be is no place to put one: the
    // base is left as it is and the setup does not build.
    let (_directory, repository, base) = repository_with(&[(".manyhands", b"a file")]);
    assert!(build(&repository, base, configuration, ENABLE_MESSAGE, &identity).is_err());
}
