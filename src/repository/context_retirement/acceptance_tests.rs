//! Cheapest remaining acceptance: real local Git clones/SQL/index and draft
//! storage. Advertisements are injected at the owned seam: no live file-URL
//! retirement support or binary authentication evidence is claimed here.
use super::*;

#[test]
fn retirement_preserves_recoverable_item_and_comment_drafts() {
    use crate::drafts::*;
    let f = Fixture::new();
    let branch = present(&f);
    let path = f.root.path().join(".manyhands/worktrees").join(ITEM);
    let store = DraftStore::open_in(f.data.path()).unwrap();
    let mut saved = Vec::new();
    for comment in [false, true] {
        let fields = if comment {
            DraftFields::Comment {
                body: "private comment sentinel\n---\nfull reply\n".into(),
            }
        } else {
            DraftFields::Ticket {
                title: "draft sentinel".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
                deps: vec![],
                parent: None,
                body: "private item sentinel\n---\nfull body\n".into(),
            }
        };
        let target = DraftTarget {
            repository: f.root.path().to_str().unwrap().into(),
            item_kind: ItemKind::Ticket,
            item_id: ITEM.parse().unwrap(),
            comment: comment.then(|| CommentTarget {
                comment_id: "01ARZ3NDEKTSV4RRFFQ69G5FZZ".parse().unwrap(),
                parent_id: None,
            }),
        };
        let base = DraftBase {
            observation: Some("base-observation-sentinel".into()),
            path: format!(".manyhands/tickets/{ITEM}/ticket.md"),
            context: BaseContext {
                kind: ContextKind::Active,
                branch: Some(branch.clone()),
                worktree: path.to_str().unwrap().into(),
                head_oid: Some(
                    git2::Repository::open(f.root.path())
                        .unwrap()
                        .refname_to_id(&branch)
                        .unwrap()
                        .to_string(),
                ),
            },
            source: Some("full base source sentinel\n".into()),
            fields: fields.clone(),
        };
        let mut claim = store
            .create(target, Some(base.clone()), fields.clone())
            .unwrap();
        claim.write(2, fields, Some(base)).unwrap();
        saved.push(claim.draft().clone());
        drop(claim); // no held claim can accidentally hide a deleted draft.
    }
    drop(store);
    super::super::super::run::synthetic_advertisement(Ok(vec![]));
    let result = execute(&f.service, RequestId::new(), retire(&f));
    assert_eq!(result.code, ResultCode::Ok, "{result:?}");
    assert!(!path.exists());
    assert!(
        git2::Repository::open(f.root.path())
            .unwrap()
            .find_reference(&branch)
            .is_err()
    );
    let store = DraftStore::open_in(f.data.path()).unwrap();
    let listed = store.list().unwrap();
    assert_eq!(listed.len(), 2);
    for before in saved {
        let row = listed.iter().find(|row| row.id == before.id).unwrap();
        assert_eq!(row.held, Held::No);
        let Entry::Draft(after) = &row.entry else {
            panic!("draft became unreadable")
        };
        assert!(
            after == &before,
            "exact ID/revision/body/base/timestamps survive retirement"
        );
        assert_eq!(after.revision, 2);
        assert_eq!(
            markdown(after),
            markdown(&before),
            "full draft remains exportable"
        );
    }
}

fn commit_file(
    git: &git2::Repository,
    reference: &str,
    parent: git2::Oid,
    path: &str,
    source: &str,
) -> git2::Oid {
    let mut index = git2::Index::new().unwrap();
    index
        .read_tree(&git.find_commit(parent).unwrap().tree().unwrap())
        .unwrap();
    let oid = git.blob(source.as_bytes()).unwrap();
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: source.len() as u32,
            id: oid,
            flags: 0,
            flags_extended: 0,
            path: path.as_bytes().into(),
        })
        .unwrap();
    let tree = git.find_tree(index.write_tree_to(git).unwrap()).unwrap();
    let sig = git2::Signature::now("Peer", "peer@example.invalid").unwrap();
    git.commit(
        Some(reference),
        &sig,
        &sig,
        "peer fixture",
        &tree,
        &[&git.find_commit(parent).unwrap()],
    )
    .unwrap()
}

#[test]
fn retirement_guarded_folded_and_refusal_table_preserves_full_resources_and_index() {
    for case in ["closed", "folded", "dirty", "unmatched", "open", "unknown"] {
        let f = Fixture::new();
        let branch = present(&f);
        let git = git2::Repository::open(f.root.path()).unwrap();
        let path = f.root.path().join(".manyhands/worktrees").join(ITEM);
        let local = git2::Repository::open(&path).unwrap();
        let primary = git.head().unwrap().target().unwrap();
        if case == "dirty" {
            std::fs::write(
                path.join("private ignored or untracked bytes"),
                b"full\0dirty\ncontent",
            )
            .unwrap();
        }
        if case == "unmatched" || case == "folded" {
            let tip = commit_file(
                &git,
                &branch,
                primary,
                "unique-work",
                "full committed content\n",
            );
            local
                .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                .unwrap();
            if case == "folded" {
                // A different primary-only entry makes this a distinct commit,
                // not the ancestor shortcut (identical timestamps can collide).
                let folded = commit_file(
                    &git,
                    "refs/heads/main",
                    primary,
                    "primary-only",
                    "unrelated primary change\n",
                );
                let folded = commit_file(
                    &git,
                    "refs/heads/main",
                    folded,
                    "unique-work",
                    "full committed content\n",
                );
                assert!(!git.graph_descendant_of(folded, tip).unwrap());
                git.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                    .unwrap();
            }
        }
        if case == "open" {
            let ticket = format!(".manyhands/tickets/{ITEM}/ticket.md");
            let closed = std::fs::read_to_string(f.root.path().join(&ticket)).unwrap();
            let open = closed
                .lines()
                .filter(|l| !l.starts_with("closed_at:") && !l.starts_with("closed_by:"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            commit_file(&git, "refs/heads/main", primary, &ticket, &open);
            git.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                .unwrap();
        }
        if case == "unknown" {
            f.db()
                .execute(
                    "UPDATE remote_context_states SET publication_evidence='history_unknown'",
                    [],
                )
                .unwrap();
        }
        let current = git.head().unwrap().target().unwrap();
        git.reference(
            "refs/remotes/origin/main",
            current,
            true,
            "fixture Fetch setup",
        )
        .unwrap();
        assert_eq!(
            execute(
                &f.service,
                RequestId::new(),
                Mutation::IndexRefresh(crate::repository::IndexRefreshInput {
                    root: f.root.path().into()
                })
            )
            .code,
            ResultCode::Ok
        );
        let resolved = f.service.resolve_repository(f.root.path()).unwrap();
        let before_index = serde_json::to_value(
            f.service
                .list_tickets(&resolved, &crate::repository::TicketFilter::default())
                .unwrap(),
        )
        .unwrap();
        let before_disk = disk_image(&path);
        let refs = || {
            git.references()
                .unwrap()
                .map(|r| {
                    let r = r.unwrap();
                    (r.name().unwrap().to_owned(), r.target())
                })
                .collect::<BTreeMap<_, _>>()
        };
        let before_refs = refs();
        let before_worktrees = git
            .worktrees()
            .unwrap()
            .iter()
            .flatten()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if case != "dirty" {
            super::super::super::run::synthetic_advertisement(Ok(vec![]));
        }
        let result = execute(&f.service, RequestId::new(), retire(&f));
        if matches!(case, "closed" | "folded") {
            assert_eq!(result.code, ResultCode::Ok, "{case}: {result:?}");
            assert!(!path.exists());
            assert!(git.find_reference(&branch).is_err());
            let shown = f
                .service
                .show_item(&resolved, &ITEM.parse().unwrap())
                .unwrap();
            assert_eq!(
                serde_json::to_value(shown).unwrap()["closure"]["state"],
                "closed"
            );
        } else {
            assert!(
                matches!(
                    result.code,
                    ResultCode::ExternalChange
                        | ResultCode::WorktreeNotClean
                        | ResultCode::RecoveryRequired
                ),
                "{case}: {result:?}"
            );
            assert_eq!(
                disk_image(&path),
                before_disk,
                "{case}: every working byte, node type and link target retained"
            );
            assert_eq!(
                refs(),
                before_refs,
                "{case}: every local/tracking ref retained"
            );
            assert_eq!(
                git.worktrees()
                    .unwrap()
                    .iter()
                    .flatten()
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                before_worktrees,
                "{case}: registration retained"
            );
            assert_eq!(
                serde_json::to_value(
                    f.service
                        .list_tickets(&resolved, &crate::repository::TicketFilter::default())
                        .unwrap()
                )
                .unwrap(),
                before_index,
                "{case}: index/read state retained"
            );
        }
    }
}

#[test]
fn retirement_d14_two_clone_new_same_id_edit_gets_a_new_materialization_intent() {
    for crash in [false, true] {
        let f = Fixture::new();
        let git = git2::Repository::open(f.root.path()).unwrap();
        git.remote("origin", "ssh://fixture@127.0.0.1:1/fixture")
            .unwrap();
        let config_path = f.root.path().join(".manyhands/config.toml");
        let config =
            std::fs::read_to_string(&config_path).unwrap() + "publication_remote = \"origin\"\n";
        let primary = commit_file(
            &git,
            "refs/heads/main",
            git.head().unwrap().target().unwrap(),
            ".manyhands/config.toml",
            &config,
        );
        git.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        let peer_dir = tempfile::tempdir().unwrap();
        // A distinct clone publishes fixture history into a disposable bare.
        let bare_dir = tempfile::tempdir().unwrap();
        let bare = git2::Repository::init_bare(bare_dir.path()).unwrap();
        git.remote_anonymous(bare_dir.path().to_str().unwrap())
            .unwrap()
            .push(&["refs/heads/main:refs/heads/main"], None)
            .unwrap();
        bare.set_head("refs/heads/main").unwrap();
        let peer =
            git2::Repository::clone(bare_dir.path().to_str().unwrap(), peer_dir.path()).unwrap();
        let branch = format!("refs/heads/manyhands/document/{ITEM}");
        let doc = "docs/retirement-lifecycle.md";
        let source = |body: &str| {
            format!(
                "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {ITEM}\ntitle: Lifecycle\n---\n{body}\n"
            )
        };
        let t = commit_file(&peer, &branch, primary, doc, &source("T initial edit"));
        peer.find_remote("origin")
            .unwrap()
            .push(&[&format!("{branch}:{branch}")], None)
            .unwrap();
        let fetch = || {
            git.remote_anonymous(bare_dir.path().to_str().unwrap())
                .unwrap()
                .fetch(&["+refs/heads/*:refs/remotes/origin/*"], None, None)
                .unwrap();
        };
        fetch();
        crate::repository::remote::synthetic_fetch(vec![
            ("refs/heads/main".into(), primary),
            (branch.clone(), t),
        ]);
        let default = || {
            Mutation::PollOnce(PollOnceInput {
                root: f.root.path().into(),
            })
        };
        assert_eq!(
            execute(&f.service, RequestId::new(), default()).code,
            ResultCode::Ok
        );
        let old: (String,String) = f.db().query_row("SELECT operation_ulid,target_oid FROM remote_poll_materializations WHERE kind='document' AND item_id=?1",[ITEM],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(old.1, t.to_string());
        let path = f.root.path().join(".manyhands/worktrees").join(ITEM);
        assert_eq!(
            std::fs::read_to_string(path.join(doc)).unwrap(),
            source("T initial edit")
        );
        // A folds the exact document content into primary and deletes its edit.
        let promoted = commit_file(
            &peer,
            "refs/heads/main",
            primary,
            doc,
            &source("T initial edit"),
        );
        peer.find_remote("origin")
            .unwrap()
            .push(
                &["refs/heads/main:refs/heads/main", &format!(":{branch}")],
                None,
            )
            .unwrap();
        peer.find_reference(&branch).unwrap().delete().unwrap();
        fetch();
        crate::repository::remote::synthetic_fetch(vec![("refs/heads/main".into(), promoted)]);
        let observed = execute(&f.service, RequestId::new(), default());
        assert_eq!(observed.code, ResultCode::Ok, "{observed:?}");
        assert!(path.exists(), "default poll never retires");
        let retirement = || {
            Mutation::ContextRetire(ContextRetireInput {
                root: f.root.path().into(),
                kind: AuthoringKind::Document,
                id: ITEM.parse().unwrap(),
            })
        };
        let request = RequestId::new();
        if crash {
            RepositoryService::set_retirement_after_branch_hook_for_testing(|| {
                panic!("loss after Git absence, before intent settlement")
            });
        }
        super::super::super::run::synthetic_advertisement(Ok(vec![]));
        let retired = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute(&f.service, request, retirement())
        }));
        let retired = if crash {
            assert!(retired.is_err());
            let db = f.db();
            let observations = row_image(&db, "remote_observation_batches");
            let projection = row_image(&db, "remote_context_states");
            let tracking = git
                .references_glob("refs/remotes/*")
                .unwrap()
                .map(|r| {
                    let r = r.unwrap();
                    (r.name().unwrap().to_owned(), r.target())
                })
                .collect::<BTreeMap<_, _>>();
            let result = execute(&f.service, request, retirement());
            assert_eq!(
                row_image(&db, "remote_observation_batches"),
                observations,
                "M-only recovery never observes"
            );
            assert_eq!(
                row_image(&db, "remote_context_states"),
                projection,
                "M-only recovery never rewrites the remote projection"
            );
            assert_eq!(
                git.references_glob("refs/remotes/*")
                    .unwrap()
                    .map(|r| {
                        let r = r.unwrap();
                        (r.name().unwrap().to_owned(), r.target())
                    })
                    .collect::<BTreeMap<_, _>>(),
                tracking,
                "M-only recovery never changes tracking refs"
            );
            result
        } else {
            retired.unwrap()
        };
        assert_eq!(retired.code, ResultCode::Ok, "{retired:?}");
        assert!(!path.exists());
        assert!(git.find_reference(&branch).is_err());
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT count(*) FROM remote_poll_materializations WHERE item_id=?1",
                    [ITEM],
                    |r| r.get::<_, usize>(0)
                )
                .unwrap(),
            0
        );
        // Later same-ID work Q is a different lifetime, not T resurrected.
        let q = commit_file(
            &peer,
            &branch,
            promoted,
            doc,
            &source("Q later same-ID edit"),
        );
        peer.find_remote("origin")
            .unwrap()
            .push(&[&format!("{branch}:{branch}")], None)
            .unwrap();
        fetch();
        crate::repository::remote::synthetic_fetch(vec![
            ("refs/heads/main".into(), promoted),
            (branch.clone(), q),
        ]);
        let later = execute(&f.service, RequestId::new(), default());
        assert_eq!(later.code, ResultCode::Ok, "{later:?}");
        let new: (String,String) = f.db().query_row("SELECT operation_ulid,target_oid FROM remote_poll_materializations WHERE item_id=?1",[ITEM],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_ne!(new.0, old.0);
        assert_eq!(new.1, q.to_string());
        assert_eq!(git.refname_to_id(&branch).unwrap(), q);
        assert_eq!(
            std::fs::read_to_string(path.join(doc)).unwrap(),
            source("Q later same-ID edit")
        );
        let repo = f.service.resolve_repository(f.root.path()).unwrap();
        assert!(
            f.service
                .show_item(&repo, &ITEM.parse().unwrap())
                .unwrap()
                .body
                .unwrap()
                .contains("Q later same-ID edit")
        );
        let before = disk_image(&path);
        assert_eq!(
            execute(&f.service, request, retirement()).code,
            ResultCode::Ok
        );
        assert_eq!(
            disk_image(&path),
            before,
            "old retirement never inspects or deletes Q"
        );
    }
}
