//! Real shared-boundary acceptance/reentry/settlement over isolated local Git.
//! No SSH evidence is claimed. Only finalizer faults are synthetic.
use super::*;
use crate::repository::keys::*;
use crate::results::{DiscoveryEffect, ResultCode};

struct NoCredentials;
impl SessionCredentialProvider for NoCredentials {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("must not implicitly unlock")
    }
}
fn retire(f: &Fixture) -> Mutation {
    Mutation::ContextRetire(ContextRetireInput {
        root: f.root.path().into(),
        kind: AuthoringKind::Ticket,
        id: ITEM.parse().unwrap(),
    })
}
fn execute(
    service: &RepositoryService,
    id: RequestId,
    mutation: Mutation,
) -> crate::results::Envelope<MutationDataDto> {
    service.execute(
        MutationCall {
            request_id: id,
            confirmation: None,
            mutation,
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

fn branch(f: &Fixture) -> String {
    let git = git2::Repository::open(f.root.path()).unwrap();
    git.remote("origin", "ssh://fixture@127.0.0.1:1/fixture")
        .unwrap();
    let config = f.root.path().join(".manyhands/config.toml");
    let text = std::fs::read_to_string(&config).unwrap() + "publication_remote = \"origin\"\n";
    std::fs::write(config, text).unwrap();
    let branch = format!("refs/heads/manyhands/ticket/{ITEM}");
    git.reference(
        &branch,
        git.head().unwrap().target().unwrap(),
        false,
        "fixture",
    )
    .unwrap();
    branch
}

fn present(f: &Fixture) -> String {
    let branch = branch(f);
    let git = git2::Repository::open(f.root.path()).unwrap();
    let path = format!(".manyhands/tickets/{ITEM}/ticket.md");
    std::fs::create_dir_all(f.root.path().join(&path).parent().unwrap()).unwrap();
    std::fs::write(f.root.path().join(&path),format!("---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\ntitle: Private fixture title\ntype: task\nstatus: closed\nclosed_at: '2026-11-04T00:00:00Z'\nclosed_by: 'Owner <owner@example.invalid>'\n---\nPrivate fixture body\n")).unwrap();
    let mut index = git.index().unwrap();
    index.add_path(Path::new(&path)).unwrap();
    index.add_path(Path::new(".manyhands/config.toml")).unwrap();
    index.write().unwrap();
    let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = git.head().unwrap().peel_to_commit().unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let tip = git
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "closed fixture",
            &tree,
            &[&parent],
        )
        .unwrap();
    git.reference(&branch, tip, true, "fixture").unwrap();
    let reference = git.find_reference(&branch).unwrap();
    let path = f.root.path().join(".manyhands/worktrees").join(ITEM);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    git.worktree(
        ITEM,
        &path,
        Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
    )
    .unwrap();
    // Synthetic earlier successful Fetch history, not a network claim. The
    // injected empty new advertisement still flows through real O/receipt SQL.
    let plan = crate::repository::remote::observation::observation_plan(f.root.path()).unwrap();
    let config = f
        .service
        .observation_configuration(f.root.path(), &plan)
        .unwrap();
    crate::repository::remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
        use crate::repository::remote::state;
        state::configure_endpoints(tx, id, &plan, &config.endpoint_digest())?;
        let prior = RemoteRefObservation::from_advertisement(
            &plan,
            plan.context(AuthoringKind::Ticket, &ITEM.parse().unwrap())
                .remote_ref(),
            tip,
            Some(tip),
        )
        .unwrap();
        state::persist_complete_advertisement(
            tx,
            id,
            &plan,
            state::generation(tx, id)?,
            &[prior],
            1,
        )?;
        Ok(())
    })
    .unwrap();
    branch
}

#[test]
fn ordinary_retirement_partial_cancel_has_truthful_flags_and_preserving_escape() {
    let f = Fixture::new();
    let branch = present(&f);
    let id = RequestId::new();
    let cancel = CancellationToken::new();
    let other = cancel.clone();
    RepositoryService::set_retirement_intent_hook_for_testing(move || other.cancel());
    super::super::run::synthetic_advertisement(Ok(Vec::new()));
    let stopped = f.service.execute(
        MutationCall {
            request_id: id,
            confirmation: None,
            mutation: retire(&f),
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &cancel,
    );
    assert_eq!(stopped.code, ResultCode::Cancelled, "{stopped:?}");
    assert_eq!(stopped.outcome, crate::results::Outcome::Partial);
    let data = serde_json::to_value(&stopped.data).unwrap();
    assert_eq!(data["worktree_removal"], "observed");
    assert_eq!(data["branch_removal"], "none");
    assert!(
        !f.root
            .path()
            .join(".manyhands/worktrees")
            .join(ITEM)
            .exists()
    );
    let operation = OperationId::parse(stopped.operation_id.as_deref().unwrap()).unwrap();
    let original = execute(
        &f.service,
        RequestId::new(),
        Mutation::OperationResume(OperationResumeInput {
            root: f.root.path().into(),
            operation_id: operation,
        }),
    );
    assert_eq!(original.code, ResultCode::OriginalRequestRequired);
    let mutation = || {
        Mutation::OperationAbandon(OperationAbandonInput {
            root: f.root.path().into(),
            operation_id: operation,
        })
    };
    let preview = f
        .service
        .prepare(&mutation())
        .data
        .unwrap()
        .into_preview()
        .unwrap();
    let kept = f.service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: Some(ConfirmationId::parse(&preview.confirmation_id).unwrap()),
            mutation: mutation(),
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(kept.code, ResultCode::Ok, "{kept:?}");
    assert!(
        git2::Repository::open(f.root.path())
            .unwrap()
            .find_reference(&branch)
            .is_ok()
    );
    assert_eq!(
        execute(&f.service, id, retire(&f)).code,
        ResultCode::ExternalChange
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM retirement_observation_pins",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(f.db().query_row("SELECT count(*) FROM retirement_observation_pin_releases WHERE reason='consumer_terminal'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
}

#[test]
fn ordinary_retirement_local_q1_matrix_is_stable_private_and_never_implicitly_approves() {
    use crate::repository::transport::HostKeyIdentity;
    use crate::repository::transport::SshTransportErrorKind as E;
    for case in ["retired", "dirty", "before_w", "key", "host"] {
        let f = Fixture::new();
        let branch = present(&f);
        let id = RequestId::new();
        let cancel = CancellationToken::new();
        let expected = match case {
            "dirty" => {
                std::fs::write(
                    f.root
                        .path()
                        .join(".manyhands/worktrees")
                        .join(ITEM)
                        .join("private-untracked"),
                    "private dirty source",
                )
                .unwrap();
                ResultCode::WorktreeNotClean
            }
            "before_w" => {
                let other = cancel.clone();
                RepositoryService::set_retirement_worktree_hook_for_testing(move || other.cancel());
                ResultCode::Cancelled
            }
            "key" => ResultCode::NoSelectedKey,
            "host" => ResultCode::HostApprovalRequired,
            _ => ResultCode::Ok,
        };
        let advertisement = match case {
            "key" => Err(E::NoSelectedKey),
            "host" => Err(E::HostApprovalRequired {
                presented: HostKeyIdentity {
                    algorithm: "synthetic-private-algorithm".into(),
                    sha256: "synthetic-private-fingerprint".into(),
                },
            }),
            _ => Ok(Vec::new()),
        };
        if case != "dirty" {
            super::super::run::synthetic_advertisement(advertisement);
        }
        let first = f.service.execute(
            MutationCall {
                request_id: id,
                confirmation: None,
                mutation: retire(&f),
            },
            &mut SessionCredentials::new(NoCredentials),
            &mut NoProgress,
            &cancel,
        );
        assert_eq!(first.code, expected, "{case}: {first:?}");
        assert_eq!(
            f.service.show_request(id).data.unwrap().state,
            RequestState::Finished,
            "{case}"
        );
        let data = serde_json::to_value(&first.data).unwrap();
        assert_eq!(
            data["state"],
            if case == "retired" {
                "retired"
            } else {
                "refused"
            }
        );
        assert_eq!(
            data["worktree_removal"],
            if case == "retired" {
                "observed"
            } else {
                "none"
            }
        );
        assert_eq!(
            data["branch_removal"],
            if case == "retired" {
                "observed"
            } else {
                "none"
            }
        );
        let json = serde_json::to_string(&first).unwrap();
        for private in [
            "Private fixture title",
            "Private fixture body",
            "private dirty source",
            "synthetic-private-algorithm",
            "synthetic-private-fingerprint",
        ] {
            assert!(!json.contains(private), "{case}: private evidence escaped");
        }
        let git = git2::Repository::open(f.root.path()).unwrap();
        if case == "retired" {
            assert!(git.find_reference(&branch).is_err());
            git.reference(
                &branch,
                git.head().unwrap().target().unwrap(),
                false,
                "new context",
            )
            .unwrap();
        } else {
            assert!(git.find_reference(&branch).is_ok());
        }
        let before = disk_image(f.root.path());
        let replay = execute(&f.service, id, retire(&f));
        assert_eq!(replay.code, expected);
        assert_eq!(first.data, replay.data);
        assert_eq!(
            before,
            disk_image(f.root.path()),
            "{case}: final replay cannot inspect/reselect/delete"
        );
    }
}

#[test]
fn ordinary_retirement_cancellation_after_acceptance_stops_before_auth_and_is_final() {
    for token in [true, false] {
        let f = Fixture::new();
        let branch = branch(&f);
        let id = RequestId::new();
        let cancel = CancellationToken::new();
        let other = cancel.clone();
        let data = f.data.path().to_owned();
        RepositoryService::set_retirement_selected_hook_for_testing(move || {
            if token {
                other.cancel();
            } else {
                let result = RepositoryService::open_at(&data)
                    .unwrap()
                    .cancel_request(id);
                assert!(result.data.unwrap().requested);
            }
        });
        let stopped = f.service.execute(
            MutationCall {
                request_id: id,
                confirmation: None,
                mutation: retire(&f),
            },
            &mut SessionCredentials::new(NoCredentials),
            &mut NoProgress,
            &cancel,
        );
        assert_eq!(stopped.code, ResultCode::Cancelled, "{stopped:?}");
        assert_eq!(
            f.service.show_request(id).data.unwrap().state,
            RequestState::Finished
        );
        assert!(
            git2::Repository::open(f.root.path())
                .unwrap()
                .find_reference(&branch)
                .is_ok()
        );
        assert_eq!(
            execute(&f.service, id, retire(&f)).code,
            ResultCode::Cancelled
        );
    }
}

#[test]
fn ordinary_retirement_final_and_lost_result_replay_preserve_new_context_and_mismatches() {
    for lost in [false, true] {
        let f = Fixture::new();
        let id = RequestId::new();
        if lost {
            // End after the local result exists but before boundary settlement.
            f.service.set_request_hook_for_testing(|| {});
            let producer = RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::BeforeRequestSettlement,
            )
            .unwrap();
            assert_eq!(
                execute(&producer, id, retire(&f)).code,
                ResultCode::InternalError
            );
        } else {
            assert_eq!(
                execute(&f.service, id, retire(&f)).code,
                ResultCode::AlreadyApplied
            );
        }
        let git = git2::Repository::open(f.root.path()).unwrap();
        let tip = git.head().unwrap().target().unwrap();
        let branch = format!("refs/heads/manyhands/ticket/{ITEM}");
        git.reference(&branch, tip, false, "new lifetime").unwrap();
        let before = disk_image(f.root.path());
        let replay = execute(&f.service, id, retire(&f));
        assert_eq!(replay.code, ResultCode::AlreadyApplied, "{replay:?}");
        assert_eq!(before, disk_image(f.root.path()));
        assert_eq!(git.refname_to_id(&branch).unwrap(), tip);
        for (kind, item) in [
            (AuthoringKind::Document, ITEM),
            (AuthoringKind::Ticket, "01ARZ3NDEKTSV4RRFFQ69G5FAW"),
        ] {
            assert_eq!(
                execute(
                    &f.service,
                    id,
                    Mutation::ContextRetire(ContextRetireInput {
                        root: f.root.path().into(),
                        kind,
                        id: item.parse().unwrap()
                    })
                )
                .code,
                ResultCode::RequestMismatch
            );
        }
        assert_eq!(
            execute(
                &f.service,
                id,
                Mutation::ContextRelease(ContextReleaseInput {
                    root: f.root.path().into(),
                    kind: AuthoringKind::Ticket,
                    id: ITEM.parse().unwrap()
                })
            )
            .code,
            ResultCode::RequestMismatch
        );
        assert_eq!(f.db().query_row("SELECT count(*) FROM context_retirement_records WHERE batch_operation_ulid IS NOT NULL",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
}

#[test]
fn ordinary_retirement_accepted_but_never_claimed_settles_without_selection_or_observation() {
    for cancelled in [false, true] {
        let f = Fixture::new();
        let id = RequestId::new();
        let producer = RepositoryService::open_at_with_failure_point_for_testing(
            f.data.path(),
            FailurePoint::BeforeRequestDomainCall,
        )
        .unwrap();
        assert_eq!(
            execute(&producer, id, retire(&f)).code,
            ResultCode::InternalError
        );
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM context_retirement_records", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let branch = present(&f); // Today's context is NOT that old request's selection.
        if cancelled {
            assert!(f.service.cancel_request(id).data.unwrap().requested);
        }
        let before = disk_image(f.root.path());
        let replay = execute(&f.service, id, retire(&f));
        assert_eq!(
            replay.code,
            if cancelled {
                ResultCode::Cancelled
            } else {
                ResultCode::ExternalChange
            },
            "{replay:?}"
        );
        assert_eq!(
            replay.outcome,
            if cancelled {
                crate::results::Outcome::Cancelled
            } else {
                crate::results::Outcome::Blocked
            }
        );
        assert_eq!(before, disk_image(f.root.path()));
        assert!(
            git2::Repository::open(f.root.path())
                .unwrap()
                .find_reference(&branch)
                .is_ok()
        );
        assert_eq!(
            f.service.show_request(id).data.unwrap().state,
            RequestState::Finished
        );
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
        assert_eq!(f.db().query_row("SELECT count(*) FROM context_retirement_records WHERE initial_resources='invalid' AND materialization_capture='unverified' AND phase='refused'", [], |r|r.get::<_, i64>(0)).unwrap(), 1);
    }
}

#[test]
fn ordinary_retirement_index_only_resume_spends_own_request_and_preserves_recreation() {
    let f = Fixture::new();
    let id = RequestId::new();
    f.db().execute_batch("CREATE TRIGGER retirement_fixture_fail_final BEFORE UPDATE OF phase ON context_retirement_records WHEN NEW.phase='complete' BEGIN SELECT RAISE(ABORT,'isolated fault'); END").unwrap();
    let first = execute(&f.service, id, retire(&f));
    assert_eq!(first.code, ResultCode::DiscoveryPending, "{first:?}");
    let operation = OperationId::parse(first.operation_id.as_deref().unwrap()).unwrap();
    f.db()
        .execute_batch("DROP TRIGGER retirement_fixture_fail_final")
        .unwrap();
    let git = git2::Repository::open(f.root.path()).unwrap();
    let tip = git.head().unwrap().target().unwrap();
    let branch = format!("refs/heads/manyhands/ticket/{ITEM}");
    git.reference(&branch, tip, false, "new lifetime").unwrap();
    let result = execute(
        &f.service,
        RequestId::new(),
        Mutation::OperationResume(OperationResumeInput {
            root: f.root.path().into(),
            operation_id: operation,
        }),
    );
    assert_eq!(
        result.code,
        ResultCode::Ok,
        "ordinary I recovery must not use generic unfenced refresh: {result:?}"
    );
    assert_eq!(result.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(git.refname_to_id(&branch).unwrap(), tip);
    let old = execute(&f.service, id, retire(&f));
    assert_eq!(old.code, ResultCode::AlreadyApplied);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM context_retirement_index_claims WHERE operation_ulid=?1",
                [operation.to_string()],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
}
