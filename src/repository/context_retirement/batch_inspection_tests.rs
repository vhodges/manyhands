//! S1 regressions: actual original A3/C with inert producer projections. Locks
//! are normal obstructions; missing/replaced SQL tables are isolated corruption.
//! No disabled CHECK/trigger, reviewer module, cleanup or production producer.
use super::*;

fn original_identity(s: &SourceFixture, ordinal: usize) -> Result<(), RepositoryError> {
    let image: String =
        s.f.db()
            .query_row("SELECT image FROM poll_retirement_freezes", [], |r| {
                r.get(0)
            })
            .unwrap();
    let entries: serde_json::Value = serde_json::from_str(&image).unwrap();
    let k = &entries[ordinal];
    crate::repository::context_retirement::instance::verify_frozen(
        &git2::Repository::open(s.f.root.path()).unwrap(),
        &RetireContextRequest {
            root: s.f.root.path().into(),
            kind: AuthoringKind::Ticket,
            item_id: k["item_id"].as_str().unwrap().parse().unwrap(),
            operation_id: OperationId::parse(k["child_operation_id"].as_str().unwrap()).unwrap(),
        },
        git2::Oid::from_str(k["expected_tip"].as_str().unwrap()).unwrap(),
        k["instance_nonce"].as_str(),
    )
}

fn assert_unstarted(s: &SourceFixture) {
    let db = s.f.db();
    assert_eq!(counts(&db), (1, 1, 1));
    assert_eq!(decision(&db), ("eligible".into(), None));
    assert_eq!(
        db.query_row(
            "SELECT next_ordinal,stage FROM poll_retirement_cursors",
            [],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        )
        .unwrap(),
        (0, "active".into())
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operation_records WHERE action='retire_context'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM context_retirement_attempts",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(db.query_row("SELECT count(*) FROM poll_retirement_targets WHERE state!='waiting' OR final_result IS NOT NULL", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
}

#[test]
fn retirement_batch_specfix_locks_are_unknown_not_positive_change() {
    for lock in ["ref", "admin", "packed"] {
        for changed_nonce in [false, true] {
            let s = prepared_batch();
            let live = run_original(&s);
            assert!(original_identity(&s, 0).is_ok());
            let child = live.next(&s.f.service, s.f.root.path()).unwrap();
            if changed_nonce {
                change_first(&s);
            }
            let path = s.f.root.path().join(match lock {
                "ref" => format!(".git/refs/heads/manyhands/ticket/{ITEM}.lock"),
                "admin" => format!(".git/worktrees/{ITEM}/index.lock"),
                "packed" => ".git/packed-refs.lock".into(),
                _ => unreachable!(),
            });
            std::fs::write(&path, b"fixture lock").unwrap();
            let db = s.f.db();
            let before = registry_image(&db);
            let disk = disk_image(s.f.root.path());
            assert!(
                child.refuse_if_changed(&s.f.service).is_err(),
                "{lock}, changed_nonce={changed_nonce}"
            );
            assert_eq!(before, registry_image(&db));
            assert_eq!(disk, disk_image(s.f.root.path()));
            assert_unstarted(&s);
            // Remove ONLY the fixture lock, not any evidence/guard/identity.
            std::fs::remove_file(path).unwrap();
            let before = registry_image(&db);
            let disk = disk_image(s.f.root.path());
            let result = live
                .next(&s.f.service, s.f.root.path())
                .unwrap()
                .refuse_if_changed(&s.f.service);
            if changed_nonce {
                assert!(
                    original_identity(&s, 0).is_err(),
                    "strict original admission still rejects a changed nonce"
                );
                assert_eq!(
                    result.unwrap().code,
                    crate::results::ResultCode::ExternalChange
                );
            } else {
                original_identity(&s, 0).expect("full original verification restored");
                assert!(
                    result.is_err(),
                    "unchanged target still has no cleanup authority"
                );
                assert_eq!(before, registry_image(&db));
                assert_unstarted(&s);
            }
            assert_eq!(disk, disk_image(s.f.root.path()));
        }
    }
}

#[test]
fn retirement_batch_specfix_missing_sql_evidence_never_finalizes_either_child() {
    for changed_nonce in [false, true] {
        let s = prepared_batch();
        let live = run_original(&s);
        let first = live.next(&s.f.service, s.f.root.path()).unwrap();
        let retained = live.next(&s.f.service, s.f.root.path()).unwrap();
        if changed_nonce {
            change_first(&s);
        }
        let db = s.f.db();
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM remote_poll_materializations",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        // Deliberate isolated schema loss, no disabled CHECK or trigger.
        db.execute_batch("DROP TABLE remote_poll_materializations")
            .unwrap();
        assert!(original_identity(&s, 1).is_ok());
        if !changed_nonce {
            original_identity(&s, 0).unwrap();
        }
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        for child in [first, retained] {
            assert!(child.refuse_if_changed(&s.f.service).is_err());
            assert_eq!(before, registry_image(&db));
            assert_unstarted(&s);
        }
        // A newly obtained handle also cannot consume missing evidence, advance
        // to ordinal 1 or terminalize/release the two-child batch.
        if let Ok(child) = live.next(&s.f.service, s.f.root.path()) {
            assert!(child.refuse_if_changed(&s.f.service).is_err());
        }
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_unstarted(&s);
    }
}

#[test]
fn retirement_batch_specfix_sql_decode_failure_is_not_hidden_by_changed_nonce() {
    for changed_nonce in [false, true] {
        let s = prepared_batch();
        let live = run_original(&s);
        let child = live.next(&s.f.service, s.f.root.path()).unwrap();
        if changed_nonce {
            change_first(&s);
        }
        let db = s.f.db();
        // Explicit isolated wrong-schema/type corruption, not a supported M row.
        // Query preparation works; typed row decoding fails. No guards disabled.
        db.execute_batch("DROP TABLE remote_poll_materializations; CREATE TABLE remote_poll_materializations(repository_id INTEGER,kind TEXT,item_id TEXT,operation_ulid BLOB,target_oid TEXT,step TEXT)").unwrap();
        let tip = git2::Repository::open(s.f.root.path())
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap()
            .to_string();
        db.execute("INSERT INTO remote_poll_materializations SELECT id,'ticket',?1,x'01',?2,'complete' FROM repositories", params![ITEM, tip]).unwrap();
        assert!(
            db.prepare("SELECT operation_ulid FROM remote_poll_materializations")
                .is_ok()
        );
        assert!(
            db.query_row(
                "SELECT operation_ulid FROM remote_poll_materializations",
                [],
                |r| r.get::<_, String>(0)
            )
            .is_err()
        );
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(child.refuse_if_changed(&s.f.service).is_err());
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_unstarted(&s);
    }
}

#[test]
fn retirement_batch_specfix_uncertain_marker_cannot_borrow_positive_materialization_change() {
    for case in ["missing", "partial", "oversized", "directory"] {
        let s = prepared_batch();
        let live = run_original(&s);
        let child = live.next(&s.f.service, s.f.root.path()).unwrap();
        s.f.materialization("complete"); // Real canonical M tuple, absent at freeze.
        let marker = s.f.root.path().join(format!(
            ".git/worktrees/{ITEM}/manyhands-retirement-instance"
        ));
        match case {
            "missing" => std::fs::remove_file(&marker).unwrap(),
            "partial" => std::fs::write(&marker, b"manyhands-retirement-instance-v1\n").unwrap(),
            "oversized" => std::fs::write(&marker, vec![b'x'; 193]).unwrap(),
            "directory" => {
                std::fs::remove_file(&marker).unwrap();
                std::fs::create_dir(&marker).unwrap();
            }
            _ => unreachable!(),
        }
        let db = s.f.db();
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(child.refuse_if_changed(&s.f.service).is_err(), "{case}");
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_unstarted(&s);
    }
}

#[test]
fn retirement_batch_specfix_well_formed_nonce_tip_and_materialization_changes_are_final() {
    for case in ["nonce", "tip", "materialization"] {
        let s = prepared_batch();
        let live = run_original(&s);
        let child = live.next(&s.f.service, s.f.root.path()).unwrap();
        original_identity(&s, 0).unwrap();
        match case {
            "nonce" => change_first(&s),
            "tip" => {
                let git = git2::Repository::open(s.f.root.path()).unwrap();
                let tip = git.head().unwrap().target().unwrap();
                git.reference(
                    &format!("refs/heads/manyhands/ticket/{ITEM}"),
                    tip,
                    true,
                    "changed to another valid fixed tip",
                )
                .unwrap();
                assert!(original_identity(&s, 0).is_err());
            }
            "materialization" => {
                s.f.materialization("complete");
            }
            _ => unreachable!(),
        }
        let db = s.f.db();
        let materializations = row_image(&db, "remote_poll_materializations");
        let disk = disk_image(s.f.root.path());
        let report = child.refuse_if_changed(&s.f.service).unwrap();
        assert_eq!(
            report,
            ContextRetirementReport::refused(crate::results::ResultCode::ExternalChange)
        );
        assert_eq!(counts(&db), (1, 1, 1));
        assert_eq!(
            db.query_row(
                "SELECT next_ordinal FROM poll_retirement_cursors",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            materializations,
            row_image(&db, "remote_poll_materializations")
        );
        assert_eq!(disk, disk_image(s.f.root.path()));
        record::audit_connection(&db).unwrap();
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}

#[cfg(unix)]
#[test]
fn retirement_batch_specfix_redirected_marker_is_unknown_even_with_positive_m_change() {
    let s = prepared_batch();
    let live = run_original(&s);
    let child = live.next(&s.f.service, s.f.root.path()).unwrap();
    s.f.materialization("complete");
    let marker = s.f.root.path().join(format!(
        ".git/worktrees/{ITEM}/manyhands-retirement-instance"
    ));
    let alternate = marker.with_extension("alternate");
    std::fs::write(
        &alternate,
        format!(
            "manyhands-retirement-instance-v1\nticket\n{ITEM}\n{}\n",
            OperationId::new()
        ),
    )
    .unwrap();
    std::fs::remove_file(&marker).unwrap();
    std::os::unix::fs::symlink(&alternate, &marker).unwrap();
    let db = s.f.db();
    let before = registry_image(&db);
    let disk = disk_image(s.f.root.path());
    assert!(child.refuse_if_changed(&s.f.service).is_err());
    assert_eq!(before, registry_image(&db));
    assert_eq!(disk, disk_image(s.f.root.path()));
    assert_unstarted(&s);
}

#[test]
fn retirement_batch_specfix_branch_only_lock_cannot_settle_the_last_hold() {
    for lock in ["ref", "packed"] {
        let s = prepared_batch();
        let live = run_original(&s);
        change_first(&s);
        live.next(&s.f.service, s.f.root.path())
            .unwrap()
            .refuse_if_changed(&s.f.service)
            .unwrap();
        let child = live.next(&s.f.service, s.f.root.path()).unwrap();
        let path = s.f.root.path().join(if lock == "ref" {
            ".git/refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW.lock"
        } else {
            ".git/packed-refs.lock"
        });
        std::fs::write(&path, b"fixture lock").unwrap();
        let db = s.f.db();
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(child.refuse_if_changed(&s.f.service).is_err());
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_eq!(counts(&db), (1, 1, 1));
        assert_eq!(decision(&db), ("eligible".into(), None));
        std::fs::remove_file(path).unwrap();
        original_identity(&s, 1).unwrap();
        assert!(
            live.next(&s.f.service, s.f.root.path())
                .unwrap()
                .refuse_if_changed(&s.f.service)
                .is_err()
        );
        assert_eq!(before, registry_image(&db));
    }
}

#[test]
fn retirement_batch_specfix_incomplete_or_ambiguous_m_is_not_positive_evidence() {
    for case in ["incomplete", "ambiguous"] {
        let s = prepared_batch();
        let live = run_original(&s);
        let child = live.next(&s.f.service, s.f.root.path()).unwrap();
        change_first(&s);
        let (owner, tip) = s.f.materialization(if case == "incomplete" {
            "intended"
        } else {
            "complete"
        });
        let db = s.f.db();
        if case == "ambiguous" {
            db.execute("INSERT INTO remote_poll_materializations SELECT id,'document',?1,?2,?3,'complete' FROM repositories", params![ITEM, owner, tip]).unwrap();
        }
        let before = registry_image(&db);
        let disk = disk_image(s.f.root.path());
        assert!(child.refuse_if_changed(&s.f.service).is_err(), "{case}");
        assert_eq!(before, registry_image(&db));
        assert_eq!(disk, disk_image(s.f.root.path()));
        assert_unstarted(&s);
    }
}

#[test]
fn retirement_batch_specfix_captured_complete_m_compares_exactly_without_consuming_replacement() {
    for replaced in [false, true] {
        let mut s = SourceFixture::new();
        a2_projections(&s.f.db());
        let mut first = candidate_id(&s, true, ITEM);
        let mut second = candidate_id(&s, false, "01ARZ3NDEKTSV4RRFFQ69G5FAW");
        second["ordinal"] = 1.into();
        let (old_owner, old_oid) = s.f.materialization("complete");
        first["materialization_capture"] = "complete".into();
        first["materialization_operation_id"] = old_owner.clone().into();
        first["materialization_oid"] = old_oid.clone().into();
        first["materialization_step"] = "complete".into();
        let entries = vec![first, second];
        input(&s, &serde_json::to_string(&entries).unwrap(), 2);
        projections(&s, &entries);
        s.seal().unwrap();
        let live = run_original(&s);
        let child = live.next(&s.f.service, s.f.root.path()).unwrap();
        let db = s.f.db();
        db.execute(
            "DELETE FROM remote_poll_materializations WHERE item_id=?1",
            [ITEM],
        )
        .unwrap();
        if replaced {
            s.f.materialization("complete");
        }
        original_identity(&s, 0).unwrap();
        let materializations = row_image(&db, "remote_poll_materializations");
        let disk = disk_image(s.f.root.path());
        assert_eq!(
            child.refuse_if_changed(&s.f.service).unwrap().code,
            crate::results::ResultCode::ExternalChange
        );
        assert_eq!(
            materializations,
            row_image(&db, "remote_poll_materializations")
        );
        assert_eq!(disk, disk_image(s.f.root.path()));
        let captured: (String, String, bool) = db.query_row("SELECT materialization_operation_ulid,materialization_oid,materialization_settled FROM context_retirement_records", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
        assert_eq!(captured, (old_owner, old_oid, false));
        assert_eq!(counts(&db), (1, 1, 1));
        assert!(RepositoryService::open_at(s.f.data.path()).is_ok());
    }
}
