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
    IndexRefresh => "index refresh",
    IndexRebuild => "index rebuild",
}

impl Command {
    /// The command a record stored the name of. `None` for a name that is
    /// no bound command's: a record this build did not write.
    pub(crate) fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.name() == name)
    }

    /// Whether the command is how the index comes back, and so runs when
    /// the index is unavailable and no request record can be read or
    /// written.
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
            | Self::IndexRefresh => false,
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
        assert_eq!(Command::IndexRefresh.name(), "index refresh");
        assert_eq!(Command::IndexRebuild.name(), "index rebuild");
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
