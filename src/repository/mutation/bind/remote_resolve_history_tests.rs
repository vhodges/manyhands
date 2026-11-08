//! `remote resolve-history` through `prepare` and `execute`, over the domain
//! fixture of `remote::resolve_tests`.
use super::*;
use crate::{
    repository::Mutation,
    repository::{
        CancellationToken, ConfirmationId, HistoryMarkerState, HistoryWayOut,
        ItemResolveHistoryMutationDto, MutationCall, MutationDataDto, NoProgress, PrepareDataDto,
        RequestId, clear_cache_loss_way_out,
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
        remote::resolve_tests::Fixture,
    },
};

struct NoCredentials;
impl SessionCredentialProvider for NoCredentials {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("a history resolution requested credentials")
    }
}

fn mutation(f: &Fixture) -> Mutation {
    Mutation::RemoteResolveHistory(RemoteResolveHistoryInput {
        root: f.root.path().to_owned(),
    })
}

fn execute(
    f: &Fixture,
    confirmation: &ConfirmationId,
) -> crate::results::Envelope<MutationDataDto> {
    f.service.execute(
        MutationCall {
            request_id: RequestId::new(),
            confirmation: Some(*confirmation),
            mutation: mutation(f),
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

#[test]
fn the_flag_is_cleared_only_when_nothing_reads_unknown() {
    #[derive(Clone, Copy)]
    enum Marker {
        None,
        File,
    }
    // The invalid marker (a directory) is covered by the domain table
    // (`resolve_tests`): the registry will not even reopen over one.
    // (case, flag, the ticket's row, marker, other unknown context,
    //  prepare code, executed data, whether the context moves after prepare)
    type Row = (
        &'static str,
        bool,
        Option<&'static str>,
        Marker,
        bool,
        ResultCode,
        Option<HistoryMarkerState>,
        bool,
    );
    let cleared = |marker| Some(marker);
    let cases: [Row; 4] = [
        (
            "flag set, nothing unknown, marker",
            true,
            Some("never_published"),
            Marker::File,
            false,
            ResultCode::Ok,
            cleared(HistoryMarkerState::Removed),
            false,
        ),
        (
            "flag clear",
            false,
            None,
            Marker::None,
            false,
            ResultCode::AlreadyApplied,
            None,
            false,
        ),
        (
            "a context still unknown",
            true,
            None,
            Marker::File,
            false,
            ResultCode::RecoveryRequired,
            None,
            false,
        ),
        (
            "a context turns unknown after prepare",
            true,
            Some("never_published"),
            Marker::File,
            true,
            ResultCode::Ok,
            None,
            true,
        ),
    ];
    for (name, flag, row, marker, other, code, state, moves) in cases {
        let f = Fixture::new();
        f.lose_history(flag, row);
        match marker {
            Marker::None => {}
            Marker::File => f.publish_marker(),
        }
        let prepared = f.service.prepare(&mutation(&f));
        assert_eq!(prepared.code, code, "{name}");
        assert_eq!(
            clear_cache_loss_way_out(prepared.code),
            (code == ResultCode::RecoveryRequired).then_some(HistoryWayOut::RemoteHistory),
            "{name} way out"
        );
        let Some(preview) = prepared.data.as_ref().and_then(PrepareDataDto::preview) else {
            assert!(code != ResultCode::Ok, "{name}");
            assert_eq!(f.flag(), flag, "{name} untouched");
            continue;
        };
        let confirmation = ConfirmationId::parse(&preview.confirmation_id).unwrap();
        if other || moves {
            f.add_context();
        }
        let done = execute(&f, &confirmation);
        if moves {
            assert_eq!(done.code, ResultCode::ExternalChange, "{name}");
            assert!(f.flag(), "{name}: flag kept");
            continue;
        }
        assert_eq!(done.code, ResultCode::Ok, "{name}: {done:?}");
        assert_eq!(
            done.data,
            Some(MutationDataDto::ResolveHistory(
                ItemResolveHistoryMutationDto {
                    cache_loss_cleared: true,
                    marker: state.unwrap(),
                }
            )),
            "{name}"
        );
        assert!(!f.flag(), "{name} flag");
        assert!(!f.marker().exists(), "{name} marker file");
    }
}
