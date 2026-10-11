//! The commands the boundary binds, as a closed set.
//!
//! Everything that depends on which command a request is of is a `match`
//! on `Command` with no wildcard arm, so a command added here does not
//! compile until each of those places has decided about it. A record
//! stores the command's name; a stored name is read back with `parse`.

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
            /// made with. This is the one place a name is written.
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
}

impl Command {
    /// The command a record stored the name of. `None` for a name that is
    /// no bound command's: a record this build did not write.
    pub(crate) fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.name() == name)
    }

    /// Whether a request of the command has an operation: a domain
    /// operation under an operation ID, with a row in a journal. A change
    /// to the key registry is one transaction with neither. A record of
    /// the command holds an operation exactly when this says so, and one
    /// that does not is not a record any call of this build wrote.
    pub(crate) const fn has_operation(self) -> bool {
        match self {
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
            | Self::RepoCreate
            | Self::RepoEnable
            | Self::RemoteRemove
            | Self::RepoRemove => true,
            Self::KeyImport | Self::KeySelect | Self::KeyClear | Self::KeyRemove => false,
        }
    }

    /// Whether the command is how the index comes back. When the index is
    /// unavailable and no request record can be read, the boundary brings
    /// the index back for such a command, and for no other, before it
    /// records and runs it.
    pub(crate) const fn restores_index(self) -> bool {
        match self {
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
            | Self::RepoRemove => false,
        }
    }

    /// Whether the command makes the repository it works in. Before it
    /// has, there is no repository to set anything on: an identity it
    /// lacks is supplied with the request.
    pub(crate) const fn creates_its_repository(self) -> bool {
        match self {
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
            | Self::RepoRemove => false,
        }
    }

    /// Whether the command changes something its caller must first be
    /// shown: `prepare` gives a preview of it and a confirmation, and
    /// `execute` runs it only for a request that presents that
    /// confirmation. For any other command `prepare` is
    /// `not_confirmable`, and so is an `execute` that presents one.
    pub(crate) const fn needs_confirmation(self) -> bool {
        match self {
            Self::RepoCreate | Self::RepoEnable | Self::RemoteRemove | Self::RepoRemove => true,
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
            | Self::KeyRemove => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    fn a_stored_name_is_read_back_as_its_command() {
        for command in Command::ALL {
            assert_eq!(Command::parse(command.name()), Some(command));
        }
    }

    #[test]
    fn the_published_names_are_the_ones_records_hold() {
        assert_eq!(Command::TicketCreate.name(), "ticket create");
        assert_eq!(Command::TicketSave.name(), "ticket save");
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
                ),
                "{command:?}"
            );
        }
    }

    #[test]
    fn only_a_change_to_the_key_registry_has_no_operation() {
        for command in Command::ALL {
            let registry_change = matches!(
                command,
                Command::KeyImport | Command::KeySelect | Command::KeyClear | Command::KeyRemove
            );
            assert_eq!(command.has_operation(), !registry_change, "{command:?}");
        }
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
