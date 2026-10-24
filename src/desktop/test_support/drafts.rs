//! Drafts for the views' tests: a stored draft of an item as the library
//! reads it now, and a second actor in a child process of the test binary
//! that discards a draft, saves an item or removes a registration.
//!
//! Draft types derive no `Debug`; tests read them through `match`.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use manyhands::{
    drafts::{DraftBase, DraftFields, DraftId, DraftStore, DraftTarget, Entry, Held, ItemKind},
    repository::{
        CancellationToken, DocumentDraft, DocumentSaveInput, ItemDtoKind, Mutation, MutationCall,
        NoProgress, RepositoryService, RequestId, keys::SessionCredentials,
    },
    results::ResultCode,
};

use super::{
    fixtures::{Fixture, NoPassphrase, item_id, remove_registration},
    git_home,
};

/// The title the second actor saves a document with.
pub(crate) const SOMEONE_ELSES_TITLE: &str = "Someone else's title";

/// Stores a draft of the item `id` of `root`, over the item as it reads
/// now, with its body as `body`, and lets go of it: it is listed
/// unclaimed, as a draft left by an earlier session is.
pub(crate) fn store_draft(
    fixture: &Fixture,
    root: &Path,
    kind: ItemDtoKind,
    id: &str,
    body: &str,
) -> DraftId {
    let service = fixture.service();
    let repo = service.resolve_repository(root).unwrap();
    let shown = service.show_item(&repo, &item_id(id)).unwrap();
    let mut current = DraftFields::of(&shown).expect("a conforming item");
    match &mut current {
        DraftFields::Document { body: old, .. } | DraftFields::Ticket { body: old, .. } => {
            *old = body.to_owned()
        }
        DraftFields::Comment { .. } => unreachable!(),
    }
    let target = DraftTarget {
        repository: root.to_str().unwrap().to_owned(),
        item_kind: match kind {
            ItemDtoKind::Document => ItemKind::Document,
            ItemDtoKind::Ticket => ItemKind::Ticket,
        },
        item_id: item_id(id),
        comment: None,
    };
    let created = fixture
        .draft_store()
        .create(target, DraftBase::of(&shown), current);
    match created {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

/// Who holds the stored draft `id`, if the store still keeps it.
pub(crate) fn held(store: &DraftStore, id: DraftId) -> Option<Held> {
    store
        .list()
        .unwrap()
        .into_iter()
        .find(|listing| listing.id == id)
        .map(|listing| match listing.entry {
            Entry::Draft(_) => listing.held,
            Entry::Unreadable(_) => panic!("the draft must be readable"),
        })
}

/// The stored body of the draft `id`.
pub(crate) fn stored_body(store: &DraftStore, id: DraftId) -> Option<String> {
    store
        .list()
        .unwrap()
        .into_iter()
        .find(|listing| listing.id == id)
        .map(|listing| match listing.entry {
            Entry::Draft(draft) => match draft.current {
                DraftFields::Document { body, .. }
                | DraftFields::Ticket { body, .. }
                | DraftFields::Comment { body } => body,
            },
            Entry::Unreadable(_) => panic!("the draft must be readable"),
        })
}

/// What the second actor does.
pub(crate) enum Actor<'a> {
    /// Claims the draft and discards it.
    Discard(DraftId),
    /// Saves the document `id` of `root` retitled as `SOMEONE_ELSES_TITLE`.
    SaveDocument { root: &'a Path, id: &'a str },
    /// Removes the registration of `root`.
    Unregister(&'a Path),
}

const ACTION: &str = "MANYHANDS_TEST_DRAFT_ACTOR";
const DATA: &str = "MANYHANDS_TEST_DRAFT_ACTOR_DATA";
const ROOT: &str = "MANYHANDS_TEST_DRAFT_ACTOR_ROOT";
const ITEM: &str = "MANYHANDS_TEST_DRAFT_ACTOR_ITEM";

/// The second actor, in a process of its own over the parent's data
/// directory. Does nothing unless the parent named an action.
#[test]
fn draft_actor_child() {
    let Ok(action) = std::env::var(ACTION) else {
        return;
    };
    git_home::isolated();
    let data = PathBuf::from(std::env::var_os(DATA).unwrap());
    match action.as_str() {
        "discard" => {
            let store = DraftStore::open_in(&data).unwrap();
            let id: DraftId = std::env::var(ITEM).unwrap().parse().unwrap();
            let discarded = store.claim(id).and_then(|claim| claim.discard());
            assert!(discarded.is_ok(), "the draft must be discarded");
        }
        "save-document" => {
            let root = PathBuf::from(std::env::var_os(ROOT).unwrap());
            let id = std::env::var(ITEM).unwrap();
            let service = RepositoryService::open_at(&data).unwrap();
            let repo = service.resolve_repository(&root).unwrap();
            let shown = service.show_item(&repo, &item_id(&id)).unwrap();
            let saved = service.execute(
                MutationCall {
                    request_id: RequestId::new(),
                    confirmation: None,
                    mutation: Mutation::DocumentSave(DocumentSaveInput {
                        root,
                        id: item_id(&id),
                        draft: DocumentDraft {
                            title: SOMEONE_ELSES_TITLE.to_owned(),
                            body: shown.body.unwrap_or_default(),
                        },
                        observation: shown.observation,
                    }),
                },
                &mut SessionCredentials::new(NoPassphrase),
                &mut NoProgress,
                &CancellationToken::new(),
            );
            assert_eq!(saved.code, ResultCode::Ok, "{}", saved.message);
        }
        "unregister" => {
            let root = PathBuf::from(std::env::var_os(ROOT).unwrap());
            let service = RepositoryService::open_at(&data).unwrap();
            remove_registration(&service, &root);
        }
        other => panic!("no such action: {other}"),
    }
}

/// Runs `draft_actor_child` in a child process of the test binary, and
/// checks it ran exactly that test.
pub(crate) fn second_actor(fixture: &Fixture, actor: Actor<'_>) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "desktop::test_support::drafts::draft_actor_child",
            "--nocapture",
        ])
        .env(DATA, fixture.data_dir());
    match actor {
        Actor::Discard(id) => command.env(ACTION, "discard").env(ITEM, id.to_string()),
        Actor::SaveDocument { root, id } => command
            .env(ACTION, "save-document")
            .env(ROOT, root)
            .env(ITEM, id),
        Actor::Unregister(root) => command.env(ACTION, "unregister").env(ROOT, root),
    };
    let output = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the child failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("1 passed"),
        "the child ran no test:\n{stdout}"
    );
}

const HOLD: &str = "MANYHANDS_TEST_DRAFT_HOLDER_DATA";
const HOLD_ID: &str = "MANYHANDS_TEST_DRAFT_HOLDER_ID";
const HOLDING: &str = "manyhands-test-draft-holder-claimed";

/// Another Manyhands process holding a draft's claim, as the library's
/// claims test holds one: the claim is let go of when this is dropped.
pub(crate) struct HeldElsewhere {
    child: std::process::Child,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

/// The holder, in a process of its own over the parent's data directory:
/// claims the draft, says so, and holds the claim until stdin closes.
/// Does nothing unless the parent named a draft.
#[test]
fn draft_holder_child() {
    use std::io::{Read, Write};

    let Some(data) = std::env::var_os(HOLD) else {
        return;
    };
    let store = DraftStore::open_in(Path::new(&data)).unwrap();
    let id: DraftId = std::env::var(HOLD_ID).unwrap().parse().unwrap();
    let _claim = store.claim(id).expect("the draft must be claimed");
    println!("{HOLDING}");
    std::io::stdout().flush().unwrap();
    let mut rest = String::new();
    std::io::stdin().read_to_string(&mut rest).unwrap();
}

/// Claims the draft `id` in a child process of the test binary, and
/// returns once the child holds it.
pub(crate) fn hold_elsewhere(fixture: &Fixture, id: DraftId) -> HeldElsewhere {
    use std::io::{BufRead, BufReader};

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "desktop::test_support::drafts::draft_holder_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(HOLD, fixture.data_dir())
        .env(HOLD_ID, id.to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut seen = String::new();
    loop {
        let mut line = String::new();
        if stdout.read_line(&mut line).unwrap() == 0 {
            panic!("the holder ended before claiming:\n{seen}");
        }
        seen.push_str(&line);
        // libtest names the test on the same line, before the marker.
        if line.trim_end().ends_with(HOLDING) {
            break;
        }
    }
    HeldElsewhere { child, stdout }
}

impl Drop for HeldElsewhere {
    fn drop(&mut self) {
        use std::io::Read;

        drop(self.child.stdin.take());
        // Read to its end, so the child's last lines find a reader.
        let mut rest = Vec::new();
        let _ = self.stdout.read_to_end(&mut rest);
        let _ = self.child.wait();
    }
}

/// A folder held at a mode for a test: its mode as it was comes back on
/// drop, a panic's included, so the temporary folder can be removed.
#[cfg(unix)]
pub(crate) struct FolderMode {
    folder: PathBuf,
    was: u32,
}

#[cfg(unix)]
impl FolderMode {
    pub(crate) fn set(folder: &Path, mode: u32) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let was = std::fs::metadata(folder).unwrap().permissions().mode();
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(mode)).unwrap();
        Self {
            folder: folder.to_owned(),
            was,
        }
    }
}

#[cfg(unix)]
impl Drop for FolderMode {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.folder, std::fs::Permissions::from_mode(self.was));
    }
}

/// Whether the tests run as the superuser, whom modes do not bind.
#[cfg(unix)]
pub(crate) fn superuser() -> bool {
    // SAFETY: `geteuid` reads the process's effective user ID; it has no
    // preconditions and cannot fail.
    unsafe { libc::geteuid() == 0 }
}
