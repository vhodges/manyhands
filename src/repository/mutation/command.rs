//! The commands the boundary binds, as a closed set.
//!
//! Everything that depends on which command a request is of is a `match`
//! on `Command` with no wildcard arm, so a command added here does not
//! compile until each of those places has decided about it. A record
//! stores the command's name; a stored name is read back with `parse`.

use super::progress::ProgressStage;

macro_rules! commands {
    ($($(#[$meta:meta])* $variant:ident => $name:literal),+ $(,)?) => {
        /// A command the boundary binds.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(crate) enum Command {
            $($(#[$meta])* $variant),+
        }

        impl Command {
            /// Every command, each once.
            pub(crate) const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            /// The command's published name: what an envelope's `command`
            /// holds, what a record stores and what the intent digest is
            /// made with. This is the one place the name of a command of
            /// `execute` is written. Three names are of no such command
            /// and are written where they are answered: `request show`,
            /// `request cancel` and `host observe`.
            pub(crate) const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),+
                }
            }
        }
    };
}

commands! {
    DocumentCreate => "document create",
    DocumentSave => "document save",
    DocumentMove => "document move",
    TicketCreate => "ticket create",
    TicketSave => "ticket save",
    TicketSlugAssign => "ticket slug-assign",
    RemoteAdd => "remote add",
    RemoteSelect => "remote select",
    IndexRefresh => "index refresh",
    IndexRebuild => "index rebuild",
    KeyGenerate => "key generate",
    KeyImport => "key import",
    KeySelect => "key select",
    KeyClear => "key clear",
    KeyRemove => "key remove",
    RepoCreate => "repo create",
    RepoEnable => "repo enable",
    RemoteRemove => "remote remove",
    RepoRemove => "repo remove",
    KeyDelete => "key delete",
    OperationAbandon => "operation abandon",
    ItemSync => "item sync",
    RepoSync => "repo sync",
    OperationResume => "operation resume",
    RepoIdentitySet => "repo identity-set",
    HostApprove => "host approve",
    HostReplace => "host replace",
    FolderCreate => "folder create",
    DocumentRepair => "document repair",
    TicketRepair => "ticket repair",
    LocalContextSync => "context sync-local",
    LocalContextResolve => "context resolve-local",
    CommentAdd => "comment add",
}

impl Command {
    /// The command a record stored the name of. `None` for a name that is
    /// no bound command's: a record this build did not write.
    pub(crate) fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.name() == name)
    }

    /// Whether a request of the command has an operation: a domain
    /// operation under an operation ID, with a row in a journal. An
    /// import, a selection and a clearing of a key each have one, of the
    /// key journal's family: the change is one transaction, and is
    /// recorded against the ID in it. A removal of a key is one
    /// transaction with neither. A record of the command holds an
    /// operation exactly when this says so, and one that does not is not
    /// a record any call of this build wrote.
    pub(crate) const fn has_operation(self) -> bool {
        match self {
            Self::LocalContextSync => true,
            Self::LocalContextResolve => false,
            // The comment's own operation, in the local journal: its
            // write and its checkpoint. The synchronization that
            // publishes it is a child the comment's binding names, made
            // under an ID of its own that no record holds (fact 79).
            Self::CommentAdd => true,
            Self::DocumentCreate
            | Self::DocumentSave
            | Self::DocumentMove
            | Self::TicketCreate
            | Self::TicketSave
            | Self::TicketSlugAssign
            | Self::RemoteAdd
            | Self::RemoteSelect
            | Self::IndexRefresh
            | Self::IndexRebuild
            | Self::KeyGenerate
            | Self::KeyImport
            | Self::KeySelect
            | Self::KeyClear
            | Self::RepoCreate
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove
            | Self::KeyDelete
            // The refresh that puts the folder in the index: making the
            // directory records nothing, and the refresh's row is the
            // request's one row.
            | Self::FolderCreate
            // One operation of the local journal: the repair, with its
            // context, its write, its checkpoint and its hand-off.
            | Self::DocumentRepair
            | Self::TicketRepair
            // One operation, in the remote journal. Its index hand-off
            // is a row of the local journal under the same ID, which
            // the remote row answers for: it owes work until the
            // hand-off is done.
            | Self::ItemSync
            | Self::RepoSync => true,
            // Never made twice, and so recorded against nothing: a key's
            // ID is not registered again.
            Self::KeyRemove => false,
            // The operation an abandon names is another request's, and
            // is its target. The abandon itself is one transaction, with
            // no operation ID and no row of its own.
            Self::OperationAbandon => false,
            // So is the operation a resume names: it is taken up under
            // its own ID, in its own journal, and the resume's record
            // holds none.
            Self::OperationResume => false,
            // A few keys of Git configuration, set by one replacement
            // of the file under the repository lease, or not at all: no
            // operation ID and no row. What shows the work to be done
            // is the configuration itself.
            Self::RepoIdentitySet => false,
            // A pin, written by the one transaction of the pin store that
            // the verification of the host ends with, or not at all: no
            // operation ID and no row. What shows the work to be done is
            // the pin store itself.
            Self::HostApprove | Self::HostReplace => false,
        }
    }

    /// Whether an accepted record of the command may hold no operation
    /// although the command has one: a build from before an import, a
    /// selection and a clearing of a key were recorded against an
    /// operation accepted them with none. Such a record is entered as
    /// that build entered it, with no operation, and is not refused for
    /// being from before. `false` for a command that has always had an
    /// operation, and for one that has none.
    pub(crate) const fn was_accepted_without_operation(self) -> bool {
        match self {
            Self::LocalContextSync | Self::LocalContextResolve | Self::CommentAdd => false,
            Self::KeyImport | Self::KeySelect | Self::KeyClear => true,
            Self::DocumentCreate
            | Self::DocumentSave
            | Self::DocumentMove
            | Self::TicketCreate
            | Self::TicketSave
            | Self::TicketSlugAssign
            | Self::RemoteAdd
            | Self::RemoteSelect
            | Self::IndexRefresh
            | Self::IndexRebuild
            | Self::KeyGenerate
            | Self::KeyRemove
            | Self::RepoCreate
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove
            | Self::KeyDelete
            | Self::OperationAbandon
            | Self::ItemSync
            | Self::RepoSync
            | Self::OperationResume
            | Self::RepoIdentitySet
            | Self::HostApprove
            | Self::HostReplace
            | Self::FolderCreate
            | Self::DocumentRepair
            | Self::TicketRepair => false,
        }
    }

    /// Whether the command is how the index comes back. When the index is
    /// unavailable and no request record can be read, the boundary brings
    /// the index back for such a command, and for no other, before it
    /// records and runs it.
    pub(crate) const fn restores_index(self) -> bool {
        match self {
            Self::LocalContextSync | Self::LocalContextResolve | Self::CommentAdd => false,
            Self::IndexRebuild => true,
            Self::DocumentCreate
            | Self::DocumentSave
            | Self::DocumentMove
            | Self::TicketCreate
            | Self::TicketSave
            | Self::TicketSlugAssign
            | Self::RemoteAdd
            | Self::RemoteSelect
            | Self::IndexRefresh
            | Self::KeyGenerate
            | Self::KeyImport
            | Self::KeySelect
            | Self::KeyClear
            | Self::KeyRemove
            | Self::RepoCreate
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove
            | Self::KeyDelete
            | Self::OperationAbandon
            | Self::ItemSync
            | Self::RepoSync
            | Self::OperationResume
            | Self::RepoIdentitySet
            | Self::HostApprove
            | Self::HostReplace
            | Self::FolderCreate
            | Self::DocumentRepair
            | Self::TicketRepair => false,
        }
    }

    /// Whether the command makes the repository it works in. Before it
    /// has, there is no repository to set anything on: an identity it
    /// lacks is supplied with the request.
    pub(crate) const fn creates_its_repository(self) -> bool {
        match self {
            Self::LocalContextSync | Self::LocalContextResolve | Self::CommentAdd => false,
            Self::RepoCreate => true,
            Self::DocumentCreate
            | Self::DocumentSave
            | Self::DocumentMove
            | Self::TicketCreate
            | Self::TicketSave
            | Self::TicketSlugAssign
            | Self::RemoteAdd
            | Self::RemoteSelect
            | Self::IndexRefresh
            | Self::IndexRebuild
            | Self::KeyGenerate
            | Self::KeyImport
            | Self::KeySelect
            | Self::KeyClear
            | Self::KeyRemove
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove
            | Self::KeyDelete
            | Self::OperationAbandon
            | Self::ItemSync
            | Self::RepoSync
            | Self::OperationResume
            | Self::RepoIdentitySet
            | Self::HostApprove
            | Self::HostReplace
            | Self::FolderCreate
            | Self::DocumentRepair
            | Self::TicketRepair => false,
        }
    }

    /// Whether the command's operation has safe points: places it can
    /// be stopped at once it is accepted, and a reservation it holds
    /// from its first one on. Only a synchronization has, and a comment's
    /// submission, at the safe points of the synchronization that
    /// publishes it. Such a command honors a cancellation after
    /// acceptance and reports a stage at each safe point, and a request
    /// of it that stopped with its reservation held is continued by the
    /// same request, whatever stopped it.
    pub(crate) const fn stops_at_safe_points(self) -> bool {
        match self {
            Self::LocalContextSync | Self::LocalContextResolve => true,
            Self::ItemSync | Self::RepoSync => true,
            // `request cancel` sets the request's flag, which the
            // publication's observer reads at its next safe point.
            Self::CommentAdd => true,
            Self::DocumentCreate
            | Self::DocumentSave
            | Self::DocumentMove
            | Self::TicketCreate
            | Self::TicketSave
            | Self::TicketSlugAssign
            | Self::RemoteAdd
            | Self::RemoteSelect
            | Self::IndexRefresh
            | Self::IndexRebuild
            | Self::KeyGenerate
            | Self::KeyImport
            | Self::KeySelect
            | Self::KeyClear
            | Self::KeyRemove
            | Self::RepoCreate
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove
            | Self::KeyDelete
            | Self::OperationAbandon
            | Self::RepoIdentitySet
            | Self::HostApprove
            | Self::HostReplace
            | Self::FolderCreate
            | Self::DocumentRepair
            | Self::TicketRepair => false,
            // A resume has no operation. One that takes up another
            // request's synchronization honors its own call's token at
            // that operation's safe points and reports their stages, but
            // nothing is held for it between calls: the reservation is
            // the operation's, and the request that began it holds that.
            // So `request cancel` asks nothing of a resume, and a resume
            // that stopped is not offered again by the rule for a
            // synchronization.
            Self::OperationResume => false,
        }
    }

    /// Whether a finished result of the command can be that of a
    /// synchronization: one that was stopped or fenced with its push not
    /// confirmed tells its caller to look at the repository, and says
    /// so again each time it is given again. A synchronization's own
    /// result is one, and so is that of a resume that took one up, and of
    /// a comment's submission, which ends as its publication does. The
    /// finished results of every other command offer nothing.
    pub(crate) const fn finishes_as_a_synchronization(self) -> bool {
        match self {
            Self::LocalContextSync | Self::LocalContextResolve => true,
            Self::ItemSync | Self::RepoSync | Self::OperationResume | Self::CommentAdd => true,
            Self::DocumentCreate
            | Self::DocumentSave
            | Self::DocumentMove
            | Self::TicketCreate
            | Self::TicketSave
            | Self::TicketSlugAssign
            | Self::RemoteAdd
            | Self::RemoteSelect
            | Self::IndexRefresh
            | Self::IndexRebuild
            | Self::KeyGenerate
            | Self::KeyImport
            | Self::KeySelect
            | Self::KeyClear
            | Self::KeyRemove
            | Self::RepoCreate
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove
            | Self::KeyDelete
            | Self::OperationAbandon
            | Self::RepoIdentitySet
            | Self::HostApprove
            | Self::HostReplace
            | Self::FolderCreate
            | Self::DocumentRepair
            | Self::TicketRepair => false,
        }
    }

    /// The stage a request of the command reports before its domain
    /// call: what that call is about to do.
    pub(crate) const fn stage(self) -> ProgressStage {
        match self {
            Self::LocalContextSync | Self::LocalContextResolve => ProgressStage::Observing,
            Self::IndexRefresh | Self::IndexRebuild => ProgressStage::Indexing,
            Self::ItemSync | Self::RepoSync => ProgressStage::Connecting,
            // An approval connects to the host to see the key it
            // presents, and does nothing else there.
            Self::HostApprove | Self::HostReplace => ProgressStage::Connecting,
            // What a resume does first is look at the operation it
            // names. What it then does is that operation's to say.
            Self::OperationResume => ProgressStage::Observing,
            Self::DocumentCreate
            | Self::DocumentSave
            | Self::DocumentMove
            | Self::TicketCreate
            | Self::TicketSave
            | Self::TicketSlugAssign
            | Self::RemoteAdd
            | Self::RemoteSelect
            | Self::KeyGenerate
            | Self::KeyImport
            | Self::KeySelect
            | Self::KeyClear
            | Self::KeyRemove
            | Self::RepoCreate
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove
            | Self::KeyDelete
            | Self::OperationAbandon
            | Self::RepoIdentitySet
            // What a folder creation does first is make the directory.
            | Self::FolderCreate
            | Self::DocumentRepair
            | Self::TicketRepair
            // What a comment's submission does first is write the
            // comment and checkpoint it. Its publication then reports
            // the stages of its synchronization's safe points.
            | Self::CommentAdd => ProgressStage::Writing,
        }
    }

    /// Whether the command's index hand-off is no operation's to owe:
    /// when it fails, nothing is left pending under the request's
    /// operation ID, the registration is marked stale and the request
    /// is over. What then brings the index up to date is any refresh,
    /// and `discovery_pending` offers that in place of a resume, which
    /// would find nothing to take up.
    pub(crate) const fn hands_off_to_any_refresh(self) -> bool {
        match self {
            Self::LocalContextSync | Self::LocalContextResolve | Self::CommentAdd => false,
            Self::FolderCreate => true,
            Self::DocumentCreate
            | Self::DocumentSave
            | Self::DocumentMove
            | Self::TicketCreate
            | Self::TicketSave
            | Self::TicketSlugAssign
            | Self::RemoteAdd
            | Self::RemoteSelect
            | Self::IndexRefresh
            | Self::IndexRebuild
            | Self::KeyGenerate
            | Self::KeyImport
            | Self::KeySelect
            | Self::KeyClear
            | Self::KeyRemove
            | Self::RepoCreate
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove
            | Self::KeyDelete
            | Self::OperationAbandon
            | Self::ItemSync
            | Self::RepoSync
            | Self::OperationResume
            | Self::RepoIdentitySet
            | Self::HostApprove
            | Self::HostReplace
            // A repair's hand-off is its own operation's to owe, as a
            // save's is.
            | Self::DocumentRepair
            | Self::TicketRepair => false,
        }
    }

    /// Whether the command changes something its caller must first be
    /// shown: `prepare` gives a preview of it and a confirmation, and
    /// `execute` runs it only for a request that presents that
    /// confirmation. For any other command `prepare` is
    /// `not_confirmable`, and so is an `execute` that presents one.
    pub(crate) const fn needs_confirmation(self) -> bool {
        match self {
            // A comment adds a file that nothing held, in the item's own
            // context, and publishes it as a synchronization would: there
            // is nothing to show first.
            Self::LocalContextSync | Self::LocalContextResolve | Self::CommentAdd => false,
            Self::RepoCreate
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove
            | Self::KeyDelete
            | Self::OperationAbandon
            | Self::RepoIdentitySet
            | Self::HostApprove
            | Self::HostReplace
            // A repair gives a file its identity, or writes a whole
            // file over one that could not be read: its caller is shown
            // the ID, and whether it is being assigned, first.
            | Self::DocumentRepair
            | Self::TicketRepair => true,
            Self::DocumentCreate
            | Self::DocumentSave
            | Self::DocumentMove
            | Self::TicketCreate
            | Self::TicketSave
            | Self::TicketSlugAssign
            | Self::RemoteAdd
            | Self::RemoteSelect
            | Self::IndexRefresh
            | Self::IndexRebuild
            | Self::KeyGenerate
            | Self::KeyImport
            | Self::KeySelect
            | Self::KeyClear
            | Self::KeyRemove
            | Self::ItemSync
            | Self::RepoSync
            // A directory that holds nothing is made and nothing else:
            // there is nothing to show first.
            | Self::FolderCreate
            // A resume does only what an operation already owes and
            // was consented to when it began.
            | Self::OperationResume => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, ProgressStage};

    #[test]
    fn a_stored_name_is_read_back_as_its_command() {
        for command in Command::ALL {
            assert_eq!(Command::parse(command.name()), Some(command));
        }
    }

    #[test]
    fn the_published_names_are_the_ones_records_hold() {
        assert_eq!(Command::DocumentCreate.name(), "document create");
        assert_eq!(Command::DocumentSave.name(), "document save");
        assert_eq!(Command::DocumentMove.name(), "document move");
        assert_eq!(Command::TicketCreate.name(), "ticket create");
        assert_eq!(Command::TicketSave.name(), "ticket save");
        assert_eq!(Command::TicketSlugAssign.name(), "ticket slug-assign");
        assert_eq!(Command::RemoteAdd.name(), "remote add");
        assert_eq!(Command::RemoteSelect.name(), "remote select");
        assert_eq!(Command::IndexRefresh.name(), "index refresh");
        assert_eq!(Command::IndexRebuild.name(), "index rebuild");
        assert_eq!(Command::KeyGenerate.name(), "key generate");
        assert_eq!(Command::KeyImport.name(), "key import");
        assert_eq!(Command::KeySelect.name(), "key select");
        assert_eq!(Command::KeyClear.name(), "key clear");
        assert_eq!(Command::KeyRemove.name(), "key remove");
        assert_eq!(Command::RepoCreate.name(), "repo create");
        assert_eq!(Command::RepoEnable.name(), "repo enable");
        assert_eq!(Command::RemoteRemove.name(), "remote remove");
        assert_eq!(Command::RepoRemove.name(), "repo remove");
        assert_eq!(Command::KeyDelete.name(), "key delete");
        assert_eq!(Command::OperationAbandon.name(), "operation abandon");
        assert_eq!(Command::ItemSync.name(), "item sync");
        assert_eq!(Command::RepoSync.name(), "repo sync");
        assert_eq!(Command::OperationResume.name(), "operation resume");
        assert_eq!(Command::RepoIdentitySet.name(), "repo identity-set");
        assert_eq!(Command::HostApprove.name(), "host approve");
        assert_eq!(Command::HostReplace.name(), "host replace");
        assert_eq!(Command::FolderCreate.name(), "folder create");
        assert_eq!(Command::DocumentRepair.name(), "document repair");
        assert_eq!(Command::TicketRepair.name(), "ticket repair");
        assert_eq!(Command::CommentAdd.name(), "comment add");
    }

    #[test]
    fn the_stopping_commands_and_their_stages_and_confirmation() {
        for command in Command::ALL {
            let local = matches!(
                command,
                Command::LocalContextSync | Command::LocalContextResolve
            );
            // A comment's submission publishes it by a synchronization
            // of its own, and stops at that synchronization's safe
            // points; what it does first is write the comment.
            let comment = command == Command::CommentAdd;
            let stopping =
                local || comment || matches!(command, Command::ItemSync | Command::RepoSync);
            assert_eq!(command.stops_at_safe_points(), stopping, "{command:?}");
            if stopping {
                assert!(!command.needs_confirmation(), "{command:?}");
                assert_eq!(
                    command.has_operation(),
                    command != Command::LocalContextResolve,
                    "{command:?}"
                );
                assert_eq!(
                    command.stage(),
                    if local {
                        ProgressStage::Observing
                    } else if comment {
                        ProgressStage::Writing
                    } else {
                        ProgressStage::Connecting
                    },
                    "{command:?}"
                );
            }
        }
        // The rest of the comment submission's predicates (R-32).
        assert!(Command::CommentAdd.finishes_as_a_synchronization());
        assert!(!Command::CommentAdd.was_accepted_without_operation());
        assert!(!Command::CommentAdd.restores_index());
        assert!(!Command::CommentAdd.creates_its_repository());
        assert!(!Command::CommentAdd.hands_off_to_any_refresh());
        assert_eq!(Command::IndexRefresh.stage(), ProgressStage::Indexing);
        assert_eq!(Command::IndexRebuild.stage(), ProgressStage::Indexing);
        assert_eq!(Command::TicketSave.stage(), ProgressStage::Writing);
        assert_eq!(Command::OperationResume.stage(), ProgressStage::Observing);
    }

    #[test]
    fn only_a_synchronization_and_a_resume_can_finish_as_a_synchronization() {
        for command in Command::ALL {
            assert_eq!(
                command.finishes_as_a_synchronization(),
                command.stops_at_safe_points() || command == Command::OperationResume,
                "{command:?}"
            );
        }
        // A resume is no synchronization for anything else.
        assert!(!Command::OperationResume.stops_at_safe_points());
        assert!(!Command::OperationResume.needs_confirmation());
        assert!(!Command::OperationResume.restores_index());
        assert!(!Command::OperationResume.creates_its_repository());
    }

    #[test]
    fn the_commands_whose_caller_must_first_be_shown_need_confirmation() {
        for command in Command::ALL {
            assert_eq!(
                command.needs_confirmation(),
                matches!(
                    command,
                    Command::RepoCreate
                        | Command::RepoEnable
                        | Command::RemoteRemove
                        | Command::RepoRemove
                        | Command::KeyDelete
                        | Command::OperationAbandon
                        | Command::RepoIdentitySet
                        | Command::HostApprove
                        | Command::HostReplace
                        | Command::DocumentRepair
                        | Command::TicketRepair
                ),
                "{command:?}"
            );
        }
    }

    #[test]
    fn only_a_command_whose_work_no_journal_records_has_no_operation() {
        for command in Command::ALL {
            // The removal of a key, which is never made twice, and an
            // abandon, which closes another request's operation and has
            // none of its own. An import, a selection and a clearing of
            // a key are one transaction each as well, and are recorded
            // against an operation in it.
            let single_transaction =
                matches!(command, Command::KeyRemove | Command::OperationAbandon);
            // A resume takes up another request's operation, and has
            // none of its own either.
            let names_another_s = matches!(
                command,
                Command::OperationResume | Command::LocalContextResolve
            );
            // An identity is Git configuration, written under the
            // repository lease with no row.
            // So is a host's pin, written by the pin store's own
            // transaction with no row.
            let unjournaled = matches!(
                command,
                Command::RepoIdentitySet | Command::HostApprove | Command::HostReplace
            );
            assert_eq!(
                command.has_operation(),
                !single_transaction && !names_another_s && !unjournaled,
                "{command:?}"
            );
        }
    }

    #[test]
    fn only_the_recorded_key_registry_changes_were_once_accepted_without_an_operation() {
        for command in Command::ALL {
            let recorded_change = matches!(
                command,
                Command::KeyImport | Command::KeySelect | Command::KeyClear
            );
            assert_eq!(
                command.was_accepted_without_operation(),
                recorded_change,
                "{command:?}"
            );
            // Only a command that has an operation now can have lacked it.
            if command.was_accepted_without_operation() {
                assert!(command.has_operation(), "{command:?}");
                assert!(!command.needs_confirmation(), "{command:?}");
            }
        }
    }

    #[test]
    fn only_a_folder_creation_hands_off_to_any_refresh() {
        for command in Command::ALL {
            assert_eq!(
                command.hands_off_to_any_refresh(),
                command == Command::FolderCreate,
                "{command:?}"
            );
        }
        // Its one operation is that refresh, and it shows nothing first.
        assert!(Command::FolderCreate.has_operation());
        assert!(!Command::FolderCreate.needs_confirmation());
        assert!(!Command::FolderCreate.stops_at_safe_points());
        assert_eq!(Command::FolderCreate.stage(), ProgressStage::Writing);
    }

    #[test]
    fn only_a_rebuild_runs_while_the_index_is_unavailable() {
        for command in Command::ALL {
            assert_eq!(
                command.restores_index(),
                command == Command::IndexRebuild,
                "{command:?}"
            );
        }
    }

    #[test]
    fn a_name_that_is_no_bound_command_s_is_not_a_command() {
        for name in ["", "ticket", "ticket  save", "Ticket Save", "request show"] {
            assert_eq!(Command::parse(name), None, "{name:?}");
        }
    }
}
