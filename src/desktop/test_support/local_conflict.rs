//! A real local conflict: a ticket whose work context and primary each
//! changed its file, synchronized locally through the library, as
//! `tests/cli/local_sync.rs` makes one (`Case::conflicted`).

use std::path::Path;

use manyhands::{
    repository::{
        AuthoringKind, CancellationToken, LocalContextConflictContentDto, LocalContextSyncInput,
        LocalContextSyncInspectionDto, Mutation, MutationCall, MutationDataDto, NoProgress,
        OperationId, RequestId, keys::SessionCredentials,
    },
    results::ResultCode,
};

use super::fixtures::{Fixture, NoPassphrase, item_id};

impl Fixture {
    /// Commits the ticket `id` to `root`'s primary branch titled "Base",
    /// saves it in its work context titled `mine`, then commits it to
    /// primary again titled `theirs`, and synchronizes the context with
    /// primary through the library's `context sync-local`. Both changed the
    /// title, so the synchronization stops at a conflict: its operation.
    pub(crate) fn conflict_locally(
        &self,
        root: &Path,
        id: &str,
        mine: &str,
        theirs: &str,
    ) -> OperationId {
        self.commit_ticket(root, id, "Base", "");
        self.refresh(root);
        self.save_ticket(root, id, mine);
        self.commit_ticket(root, id, theirs, "");
        let service = self.service();
        let repo = service.resolve_repository(root).unwrap();
        let observation = service
            .observe_local_context_sync(&repo, AuthoringKind::Ticket, &item_id(id))
            .unwrap()
            .observation;
        let answer = service.execute(
            MutationCall {
                request_id: RequestId::new(),
                confirmation: None,
                mutation: Mutation::LocalContextSync(LocalContextSyncInput {
                    root: root.to_owned(),
                    kind: AuthoringKind::Ticket,
                    id: item_id(id),
                    observation,
                }),
            },
            &mut SessionCredentials::new(NoPassphrase),
            &mut NoProgress,
            &CancellationToken::new(),
        );
        assert_eq!(
            answer.code,
            ResultCode::RecoveryRequired,
            "{}",
            answer.message
        );
        let Some(MutationDataDto::LocalContextSync(data)) = answer.data else {
            panic!("a local synchronization answers its own data");
        };
        OperationId::parse(data.operation_id.as_deref().unwrap()).unwrap()
    }

    /// The library's inspection of the local synchronization `operation`,
    /// and the content read of each of its tokens.
    pub(crate) fn read_local_conflict(
        &self,
        root: &Path,
        operation: OperationId,
    ) -> (
        LocalContextSyncInspectionDto,
        Vec<LocalContextConflictContentDto>,
    ) {
        let service = self.service();
        let repo = service.resolve_repository(root).unwrap();
        let inspection = service
            .inspect_local_context_sync(&repo, operation)
            .unwrap();
        let contents = inspection
            .paths
            .iter()
            .map(|path| {
                service
                    .read_local_context_conflict(&repo, operation, &path.token)
                    .unwrap()
            })
            .collect();
        (inspection, contents)
    }
}
