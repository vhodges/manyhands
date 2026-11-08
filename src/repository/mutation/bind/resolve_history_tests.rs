//! `item resolve-history` through the boundary's `prepare` and `execute`, over
//! a real enabled repository (the domain fixture of `remote::resolve_tests`).
//! The remote is never contacted; a fetch is a batch written to the registry.
use super::*;
use crate::{
    repository::{
        CancellationToken, ConfirmationId, HistoryWayOut, Mutation, MutationCall, NoProgress,
        PrepareDataDto, RemotePublicationEvidence, RequestId,
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
        remote::resolve_tests::{Fixture, ITEM, item},
        resolve_history_way_out,
    },
    results::Envelope,
};

struct NoCredentials;
impl SessionCredentialProvider for NoCredentials {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("a history resolution requested credentials")
    }
}

fn mutation(f: &Fixture, choice: ResolveHistoryChoice) -> Mutation {
    Mutation::ItemResolveHistory(ItemResolveHistoryInput {
        root: f.root.path().to_owned(),
        kind: AuthoringKind::Ticket,
        id: item(),
        choice,
    })
}

fn prepare(f: &Fixture, choice: ResolveHistoryChoice) -> Envelope<PrepareDataDto> {
    f.service.prepare(&mutation(f, choice))
}

fn confirmation_of(prepared: &Envelope<PrepareDataDto>) -> ConfirmationId {
    let preview = prepared
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .expect("a preview");
    ConfirmationId::parse(&preview.confirmation_id).unwrap()
}

fn execute(
    f: &Fixture,
    request_id: RequestId,
    confirmation: &ConfirmationId,
) -> Envelope<MutationDataDto> {
    f.service.execute(
        MutationCall {
            request_id,
            confirmation: Some(*confirmation),
            mutation: mutation(f, ResolveHistoryChoice::Publish),
        },
        &mut SessionCredentials::new(NoCredentials),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

fn advertise_local(f: &Fixture, observed_at: i64) {
    f.advertise(f.observe().local_head.unwrap(), observed_at);
}

#[test]
fn a_prepare_previews_only_an_unknown_absent_context() {
    use ResolveHistoryChoice::{Publish, Release};
    type Setup = fn(&Fixture);
    type Case = (
        &'static str,
        Setup,
        ResolveHistoryChoice,
        ResultCode,
        bool,
        Option<HistoryWayOut>,
    );
    // (case, setup, choice, code, whether it is previewed, the way out)
    let sync = |item: &str| {
        Some(HistoryWayOut::Sync {
            item_id: item.into(),
        })
    };
    let cases: [Case; 9] = [
        (
            "unknown row, absent",
            |f| {
                f.lose_history(false, Some("history_unknown"));
                f.batch(&[], 100);
            },
            Publish,
            ResultCode::Ok,
            true,
            sync(ITEM),
        ),
        (
            "cache loss, no row, absent",
            |f| {
                f.lose_history(true, None);
                f.batch(&[], 100);
            },
            Publish,
            ResultCode::Ok,
            true,
            sync(ITEM),
        ),
        (
            "unknown row, absent, a tracking ref left by an earlier fetch",
            |f| {
                f.lose_history(false, Some("history_unknown"));
                f.batch(&[], 100);
                f.track();
            },
            Publish,
            ResultCode::RemoteBranchDeleted,
            false,
            Some(HistoryWayOut::Republish {
                item_id: ITEM.into(),
            }),
        ),
        (
            "unknown row, no current batch",
            |f| f.lose_history(false, Some("history_unknown")),
            Publish,
            ResultCode::RecoveryRequired,
            false,
            Some(HistoryWayOut::RemoteHistory),
        ),
        (
            "release is never previewed",
            |f| f.lose_history(false, Some("history_unknown")),
            Release,
            ResultCode::InvalidInput,
            false,
            Some(HistoryWayOut::ReleaseContext {
                item_id: ITEM.into(),
            }),
        ),
        (
            "already resolved",
            |f| f.lose_history(false, None),
            Publish,
            ResultCode::AlreadyApplied,
            false,
            sync(ITEM),
        ),
        (
            "present on the remote",
            |f| {
                f.lose_history(false, Some("history_unknown"));
                advertise_local(f, 200);
            },
            Publish,
            ResultCode::AlreadyApplied,
            false,
            sync(ITEM),
        ),
        (
            "published, then seen gone",
            |f| {
                f.lose_history(false, None);
                advertise_local(f, 200);
                f.batch(&[], 300);
            },
            Publish,
            ResultCode::RemoteBranchDeleted,
            false,
            Some(HistoryWayOut::Republish {
                item_id: ITEM.into(),
            }),
        ),
        (
            "published, then seen gone, under release",
            |f| {
                f.lose_history(false, None);
                advertise_local(f, 200);
                f.batch(&[], 300);
            },
            Release,
            ResultCode::InvalidInput,
            false,
            Some(HistoryWayOut::ReleaseContext {
                item_id: ITEM.into(),
            }),
        ),
    ];
    for (name, setup, choice, code, previewed, way_out) in cases {
        let f = Fixture::new();
        setup(&f);
        let prepared = prepare(&f, choice);
        assert_eq!(prepared.code, code, "{name}");
        assert_eq!(prepared.data.is_some(), previewed, "{name} preview");
        assert_eq!(
            resolve_history_way_out(choice, prepared.code, ITEM),
            way_out,
            "{name} way out"
        );
        if previewed {
            let preview = prepared
                .data
                .as_ref()
                .and_then(PrepareDataDto::preview)
                .unwrap();
            assert_eq!(
                preview.effects.iter().map(|e| e.effect).collect::<Vec<_>>(),
                [PreviewEffect::HistoryResolved],
                "{name} effects"
            );
        }
        // Nothing was written by a prepare.
        let resolved = f.evidence() == RemotePublicationEvidence::NeverPublished;
        assert_eq!(resolved, name.starts_with("already"), "{name} untouched");
    }
}

#[test]
fn a_confirm_after_the_batch_changed_is_an_external_change() {
    let f = Fixture::new();
    f.lose_history(false, Some("history_unknown"));
    f.batch(&[], 100);
    let confirmation = confirmation_of(&prepare(&f, ResolveHistoryChoice::Publish));
    // A newer fetch finds the branch on the remote.
    advertise_local(&f, 200);
    let stale = execute(&f, RequestId::new(), &confirmation);
    assert_eq!(stale.code, ResultCode::ExternalChange);
    assert!(
        stale
            .recovery
            .iter()
            .any(|action| action.action == crate::results::RecoveryActionKind::RequestPrepare),
        "a fresh prepare is named: {:?}",
        stale.recovery
    );
    assert_ne!(f.evidence(), RemotePublicationEvidence::NeverPublished);
    // The fresh prepare says there is nothing left to resolve.
    assert_eq!(
        prepare(&f, ResolveHistoryChoice::Publish).code,
        ResultCode::AlreadyApplied
    );
}

#[test]
fn a_publish_resolves_the_context_clears_the_flag_and_replays_its_result() {
    // (case, cache loss, the context's row, the marker file is there, the
    // data it reports)
    let cases = [
        (
            "flag clear",
            false,
            Some("history_unknown"),
            false,
            false,
            HistoryMarkerState::NotApplicable,
        ),
        (
            "last resolution under cache loss",
            true,
            None,
            true,
            true,
            HistoryMarkerState::Removed,
        ),
    ];
    for (name, flag, row, marker, cleared, state) in cases {
        let f = Fixture::new();
        f.lose_history(flag, row);
        f.batch(&[], 100);
        if marker {
            f.publish_marker();
        }
        assert_eq!(
            f.evidence(),
            RemotePublicationEvidence::HistoryUnknown,
            "{name}"
        );
        let confirmation = confirmation_of(&prepare(&f, ResolveHistoryChoice::Publish));
        let request_id = RequestId::new();
        let done = execute(&f, request_id, &confirmation);
        assert_eq!(done.code, ResultCode::Ok, "{name}: {done:?}");
        assert_eq!(
            done.data,
            Some(MutationDataDto::ResolveHistory(
                ItemResolveHistoryMutationDto {
                    cache_loss_cleared: cleared,
                    marker: state,
                }
            )),
            "{name}"
        );
        assert_eq!(
            f.evidence(),
            RemotePublicationEvidence::NeverPublished,
            "{name}"
        );
        assert!(!f.flag(), "{name} flag");
        assert!(!f.marker().exists(), "{name} marker file");
        // The same request, sent again, is answered by its record.
        let replay = execute(&f, request_id, &confirmation);
        assert_eq!(replay.code, done.code, "{name} replay");
        assert_eq!(replay.data, done.data, "{name} replay data");
        assert_eq!(replay.effects, done.effects, "{name} replay effects");
    }
}
