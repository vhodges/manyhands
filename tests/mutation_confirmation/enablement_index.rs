use super::*;

fn assert_configuration_current(root: &Path) {
    let repository = git2::Repository::open(root).unwrap();
    let path = Path::new(canonical::CONFIG_PATH);
    let head = repository
        .head()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .get_path(path)
        .unwrap();
    let entry = repository
        .index()
        .unwrap()
        .get_path(path, 0)
        .expect("configuration must be indexed");
    assert_eq!((entry.id, entry.mode), (head.id(), head.filemode() as u32));
    assert_eq!(repository.status_file(path).unwrap(), git2::Status::CURRENT);
}

#[test]
fn partial_create_retry_and_finished_replay_keep_configuration_current() {
    for exists in [false, true] {
        let mut desk = Desk::new();
        let site = Site::new();
        if exists {
            fs::create_dir(&site.root).unwrap();
        }
        let (confirmation, _) = desk.preview(&site.create());
        desk.fail_once_at(FailurePoint::AfterRepositoryInitialization);
        assert_eq!(
            desk.execute(REQUEST_1, &confirmation, site.create())
                .outcome,
            Outcome::Partial
        );
        desk.reopen();
        let finished = desk.execute(REQUEST_1, &confirmation, site.create());
        assert_enabled(&finished, &site.root);
        assert_configuration_current(&site.root);
        desk.reopen();
        let replayed = desk.execute(REQUEST_1, &confirmation, site.create());
        assert_eq!(
            serde_json::to_value(&finished).unwrap(),
            serde_json::to_value(&replayed).unwrap()
        );
        assert_configuration_current(&site.root);
        assert_eq!(initializations(&site.root), 1);
    }
}

#[test]
fn enable_commit_interruption_and_lost_result_leave_index_current() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    desk.fail_once_at(FailurePoint::AfterInitializationCommit);
    assert_eq!(
        desk.execute(REQUEST_1, &confirmation, plain.enable())
            .outcome,
        Outcome::Partial
    );
    desk.fail_once_at(FailurePoint::BeforeRequestSettlement);
    desk.execute(REQUEST_1, &confirmation, plain.enable());
    desk.reopen();
    assert_enabled(
        &desk.execute(REQUEST_1, &confirmation, plain.enable()),
        &plain.root,
    );
    assert_configuration_current(&plain.root);
    assert_eq!(plain.initializations(), 1);
}

#[test]
fn finished_enable_replay_preserves_later_staged_configuration() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let finished = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_enabled(&finished, &plain.root);
    assert_configuration_current(&plain.root);
    let repository = git2::Repository::open(&plain.root).unwrap();
    let path = Path::new(canonical::CONFIG_PATH);
    fs::write(plain.root.join(path), "user staging after enable\n").unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(path).unwrap();
    index.write().unwrap();
    let before = fs::read(repository.path().join("index")).unwrap();
    desk.reopen();
    let replayed = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(
        serde_json::to_value(finished).unwrap(),
        serde_json::to_value(replayed).unwrap()
    );
    assert_eq!(fs::read(repository.path().join("index")).unwrap(), before);
    assert_eq!(
        fs::read(plain.root.join(path)).unwrap(),
        b"user staging after enable\n"
    );
}

#[test]
fn interrupted_enable_with_failed_index_write_repairs_on_f2_retry_after_head_moves() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let (confirmation, _) = desk.preview(&plain.enable());
    let repository = git2::Repository::open(&plain.root).unwrap();
    let lock = repository.path().join("index.lock");
    fs::write(&lock, b"foreign lock").unwrap();
    desk.fail_once_at(FailurePoint::AfterInitializationCommit);
    let partial = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(partial.outcome, Outcome::Partial);
    let initialized = plain.head();
    assert_eq!(partial.effects.commit_oid, Some(initialized.to_string()));
    assert!(
        repository
            .index()
            .unwrap()
            .get_path(Path::new(canonical::CONFIG_PATH), 0)
            .is_none()
    );
    fs::remove_file(lock).unwrap();
    let parent = repository.find_commit(initialized).unwrap();
    let signature = repository.signature().unwrap();
    let advanced = repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "HEAD moved",
            &parent.tree().unwrap(),
            &[&parent],
        )
        .unwrap();
    desk.reopen();
    let finished = desk.execute(REQUEST_1, &confirmation, plain.enable());
    assert_eq!(
        assert_enabled(&finished, &plain.root),
        initialized.to_string()
    );
    assert_eq!(plain.head(), advanced);
    assert_configuration_current(&plain.root);
    assert_eq!(
        desk.execute(REQUEST_1, &confirmation, plain.enable()),
        finished
    );
}
