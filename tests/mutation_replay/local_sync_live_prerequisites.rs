//! Final whole-branch QUALITY: advice requires the live executor prerequisites.
use super::*;
use super::{
    inspection_spec::{inspect_unchanged, ordinary},
    spec::authority_image,
};
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;

fn context(f: &ConflictWorld) -> git2::Repository {
    git2::Repository::open(f.world.worktree(items::TICKET_A)).unwrap()
}
fn logging(f: &ConflictWorld, enabled: bool) {
    context(f)
        .config()
        .unwrap()
        .set_bool("core.logallrefupdates", enabled)
        .unwrap();
    assert_eq!(
        context(f)
            .config()
            .unwrap()
            .get_bool("core.logallrefupdates")
            .unwrap(),
        enabled
    );
}
fn expect_external(f: &ConflictWorld, reason: LocalContextSyncReason) {
    match inspect_unchanged(f) {
        Ok(view) => {
            assert_eq!(view.next_action, LocalContextSyncNextAction::ExternalRepair);
            assert_eq!(view.reason, reason);
            assert!(!view.can_start_resolution);
        }
        Err(error) => assert!(matches!(
            error.code(),
            ResultCode::ExternalChange | ResultCode::RecoveryRequired
        )),
    }
}

#[test]
fn final_quality_pending_inspection_checks_live_logging() {
    for disposition in ["fast_forward", "merge"] {
        for phase in ["prepared", "candidate"] {
            let f = ordinary(disposition, phase, false);
            let git = context(&f);
            let config = git.commondir().join("config");
            let original = std::fs::read(&config).unwrap();
            let mut before = super::super::support::repository_git_file_bytes(&f.world.fixture);
            before.remove(Path::new("config"));
            logging(&f, false);
            let mut after = super::super::support::repository_git_file_bytes(&f.world.fixture);
            after.remove(Path::new("config"));
            assert!(
                before == after,
                "logging toggle changed Git evidence other than configuration"
            );
            expect_external(&f, LocalContextSyncReason::AuthorityMissing);
            // The published advice and actual refusal agree; this is a new execute,
            // outside inspect_unchanged's no-write/held-lease assertion window.
            let stopped = f.world.execute(mutation::REQUEST_2, f.start());
            assert_eq!(stopped.code, ResultCode::RecoveryRequired);
            let Some(MutationDataDto::LocalContextSync(data)) = stopped.data else {
                panic!("local data")
            };
            assert_eq!(data.reason, LocalContextSyncReason::AuthorityMissing);
            std::fs::write(&config, &original).unwrap();
            assert_eq!(
                inspect_unchanged(&f).unwrap().next_action,
                LocalContextSyncNextAction::Resume
            );
            context(&f)
                .config()
                .unwrap()
                .set_str("core.logallrefupdates", "always")
                .unwrap();
            assert_eq!(
                inspect_unchanged(&f).unwrap().next_action,
                LocalContextSyncNextAction::Resume
            );
            std::fs::write(config, original).unwrap();
        }
    }
}

#[test]
fn final_quality_inspection_rechecks_logging_after_session_b() {
    for disposition in ["fast_forward", "merge"] {
        for phase in ["prepared", "candidate"] {
            let f = ordinary(disposition, phase, false);
            let config = context(&f).commondir().join("config");
            let original = std::fs::read(&config).unwrap();
            let root = f.world.worktree(items::TICKET_A);
            f.world.service.set_observation_hook_for_testing(move || {
                git2::Repository::open(root)
                    .unwrap()
                    .config()
                    .unwrap()
                    .set_bool("core.logallrefupdates", false)
                    .unwrap();
            });
            let _lease = RepositoryService::hold_lease_for_testing(
                &f.world.root,
                f.world.data.path(),
                LeaseKind::Repository,
            )
            .unwrap();
            let admin = super::super::support::repository_git_file_bytes(&f.world.fixture);
            let sql = authority_image(&f.world);
            let result = f
                .world
                .service
                .inspect_local_context_sync(&f.world.resolved(), f.operation);
            std::fs::write(config, original).unwrap();
            assert_eq!(result.unwrap_err().code(), ResultCode::ExternalChange);
            assert!(admin == super::super::support::repository_git_file_bytes(&f.world.fixture));
            assert!(sql == authority_image(&f.world));
            drop(_lease);
            assert_eq!(
                inspect_unchanged(&f).unwrap().next_action,
                LocalContextSyncNextAction::Resume
            );
        }
    }
}

#[test]
fn final_quality_logging_is_not_required_for_noop_integrated_or_terminal_work() {
    for (disposition, phase) in [
        ("contained", "prepared"),
        ("contained", "index_pending"),
        ("fast_forward", "integrated"),
        ("fast_forward", "index_pending"),
        ("merge", "integrated"),
        ("merge", "index_pending"),
    ] {
        let f = ordinary(disposition, phase, false);
        logging(&f, false);
        assert_eq!(
            inspect_unchanged(&f).unwrap().next_action,
            LocalContextSyncNextAction::Resume,
            "{disposition}/{phase}"
        );
        let result = f.world.execute(mutation::REQUEST_2, f.start());
        assert_eq!(
            result.code,
            if disposition == "contained" {
                ResultCode::AlreadyApplied
            } else {
                ResultCode::Ok
            }
        );
        assert_eq!(
            inspect_unchanged(&f).unwrap().next_action,
            LocalContextSyncNextAction::None
        );
        assert!(
            !context(&f)
                .config()
                .unwrap()
                .get_bool("core.logallrefupdates")
                .unwrap()
        );
    }
    for state in ["cancelled", "abandoned", "complete"] {
        let (f, _) = super::quality::terminal_parent(state);
        logging(&f, false);
        let view = inspect_unchanged(&f).unwrap();
        assert_eq!(view.next_action, LocalContextSyncNextAction::None);
        assert_eq!(view.state.as_str(), state);
        let head = context(&f).path().join("HEAD");
        let original = std::fs::read(&head).unwrap();
        let oid = context(&f).head().unwrap().target().unwrap();
        std::fs::write(&head, format!("{oid}\n")).unwrap();
        assert_eq!(
            inspect_unchanged(&f).unwrap().next_action,
            LocalContextSyncNextAction::None
        );
        std::fs::write(head, original).unwrap();
    }
}

#[test]
fn final_quality_pending_inspection_checks_live_head_identity() {
    for between in [false, true] {
        let f = ordinary("fast_forward", "prepared", false);
        let git = context(&f);
        let path = git.path().join("HEAD");
        let original = std::fs::read(&path).unwrap();
        let oid = git.head().unwrap().target().unwrap();
        if between {
            let target = path.clone();
            f.world.service.set_observation_hook_for_testing(move || {
                std::fs::write(target, format!("{oid}\n")).unwrap()
            });
            let _lease = RepositoryService::hold_lease_for_testing(
                &f.world.root,
                f.world.data.path(),
                LeaseKind::Repository,
            )
            .unwrap();
            let admin = super::super::support::repository_git_file_bytes(&f.world.fixture);
            let sql = authority_image(&f.world);
            let result = f
                .world
                .service
                .inspect_local_context_sync(&f.world.resolved(), f.operation);
            std::fs::write(&path, &original).unwrap();
            assert_eq!(result.unwrap_err().code(), ResultCode::ExternalChange);
            assert!(admin == super::super::support::repository_git_file_bytes(&f.world.fixture));
            assert!(sql == authority_image(&f.world));
        } else {
            std::fs::write(&path, format!("{oid}\n")).unwrap();
            assert!(context(&f).head_detached().unwrap());
            expect_external(&f, LocalContextSyncReason::ForeignChange);
            std::fs::write(&path, &original).unwrap();
        }
        assert_eq!(
            inspect_unchanged(&f).unwrap().next_action,
            LocalContextSyncNextAction::Resume
        );
    }
}

#[cfg(unix)]
fn copy_fixture_directory(from: &Path, to: &Path) {
    std::fs::create_dir(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            copy_fixture_directory(&entry.path(), &target);
        } else {
            assert!(kind.is_file(), "fixture Git metadata is regular");
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
#[cfg(unix)]
#[test]
fn final_quality_pending_inspection_checks_common_directory_and_registration() {
    for foreign_common in [false, true] {
        for between in [false, true] {
            let f = ordinary("fast_forward", "prepared", false);
            let worktree = f.world.worktree(items::TICKET_A);
            let git = context(&f);
            let oid = git.head().unwrap().target();
            let admin = git.path().to_owned();
            let common = git.commondir().canonicalize().unwrap();
            let gitfile = worktree.join(".git");
            let original_link = std::fs::read(&gitfile).unwrap();
            let alternate = f.world.data.path().join("alternate-git");
            let target: PathBuf = if foreign_common {
                copy_fixture_directory(&common, &alternate);
                let target = alternate.join(admin.strip_prefix(&common).unwrap());
                std::fs::write(
                    target.join("commondir"),
                    format!("{}\n", alternate.display()),
                )
                .unwrap();
                target
            } else {
                alternate.clone()
            };
            let change = {
                let admin = admin.clone();
                let target = target.clone();
                let common = common.clone();
                let gitfile = gitfile.clone();
                move || {
                    if !foreign_common {
                        std::fs::rename(&admin, &target).unwrap();
                        std::fs::write(target.join("commondir"), format!("{}\n", common.display()))
                            .unwrap();
                    }
                    std::fs::write(gitfile, format!("gitdir: {}\n", target.display())).unwrap();
                }
            };
            let _lease = RepositoryService::hold_lease_for_testing(
                &f.world.root,
                f.world.data.path(),
                LeaseKind::Repository,
            )
            .unwrap();
            let old_common_link = std::fs::read(admin.join("commondir")).unwrap();
            let root_admin = super::super::support::repository_git_file_bytes(&f.world.fixture);
            let mut expected_admin = super::super::support::confirmation::tree(if foreign_common {
                &alternate
            } else {
                &admin
            });
            if !foreign_common {
                expected_admin.insert(
                    PathBuf::from("commondir"),
                    Some(format!("{}\n", common.display()).into_bytes()),
                );
            }
            if between {
                f.world.service.set_observation_hook_for_testing(change);
            } else {
                change();
            }
            let sql = authority_image(&f.world);
            let files = super::super::support::confirmation::tree(&worktree);
            let result = f
                .world
                .service
                .inspect_local_context_sync(&f.world.resolved(), f.operation);
            // Verify this is a readable Git namespace with the same HEAD OID and
            // tracked image, differing only in the live identity/registration.
            let moved = context(&f);
            assert_eq!(moved.head().unwrap().target(), oid);
            assert_eq!(moved.workdir(), Some(worktree.as_path()));
            if foreign_common {
                assert_ne!(moved.commondir().canonicalize().unwrap(), common);
            } else {
                assert_eq!(moved.commondir().canonicalize().unwrap(), common);
                assert!(f.world.fixture.repository.worktrees().unwrap().is_empty());
            }
            drop(moved);
            assert!(expected_admin == super::super::support::confirmation::tree(&alternate));
            if !foreign_common {
                std::fs::write(target.join("commondir"), old_common_link).unwrap();
                std::fs::rename(&target, &admin).unwrap();
            }
            std::fs::write(&gitfile, &original_link).unwrap();
            if between {
                assert_eq!(result.unwrap_err().code(), ResultCode::ExternalChange);
            } else {
                match result {
                    Ok(view) => {
                        assert_eq!(view.next_action, LocalContextSyncNextAction::ExternalRepair);
                        assert!(!view.can_start_resolution);
                    }
                    Err(e) => assert!(matches!(
                        e.code(),
                        ResultCode::ExternalChange | ResultCode::RecoveryRequired
                    )),
                }
            }
            assert!(sql == authority_image(&f.world));
            assert!(
                root_admin == super::super::support::repository_git_file_bytes(&f.world.fixture)
            );
            // Gitfile is part of the namespace mutation, not a read effect.
            let mut files = files;
            files.insert(PathBuf::from(".git"), Some(original_link));
            assert!(files == super::super::support::confirmation::tree(&worktree));
            drop(_lease);
            assert_eq!(
                inspect_unchanged(&f).unwrap().next_action,
                LocalContextSyncNextAction::Resume
            );
        }
    }
}
