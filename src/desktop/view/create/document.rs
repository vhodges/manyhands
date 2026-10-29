//! A new document's place (design, Create (decision 3); R-22): its Folder
//! and File name, checked as they are typed, first here
//! (`model::create::document_path`), then with `observe_path` in the
//! primary context, which must find nothing there. Every change checks
//! again under a new stamp, and the last check's task is dropped, so an
//! answer for an older name never offers Create.
//!
//! The path is the user's text, so nothing here derives `Debug`.

use std::path::{Path, PathBuf};

use gpui_kit::{Context, Entity, component::input::InputState};
use manyhands::{repository::PathObservationDto, results::ResultCode};

use super::{CreateDialog, Inputs};
use crate::desktop::{
    model::{
        create::{document_path, path_held},
        stamp::{Stamp, ViewScope},
    },
    services::{ReadKind, ReadResult},
};

/// Where a document's path check stands.
pub(super) enum PathCheck {
    /// Refused here, in these words.
    Refused(&'static str),
    /// `observe_path` was asked about it.
    Checking,
    /// Nothing is there.
    Free(String),
    /// A file is there.
    Held(String),
    /// The observe gave no answer.
    Failed(ResultCode),
}

/// A new document's Folder and File name, and their check.
pub(super) struct Place {
    pub(super) folder: Entity<InputState>,
    pub(super) file_name: Entity<InputState>,
    pub(super) check: PathCheck,
}

impl CreateDialog {
    /// What the document's path check says, while it says anything.
    pub(super) fn path_note(&self) -> Option<String> {
        let Inputs::Document(place) = &self.inputs else {
            return None;
        };
        match &place.check {
            PathCheck::Refused(words) => Some((*words).to_owned()),
            PathCheck::Held(path) => Some(path_held(path)),
            PathCheck::Failed(code) => Some(code.message().to_owned()),
            PathCheck::Checking | PathCheck::Free(_) => None,
        }
    }

    /// Where the new item goes: a ticket's place follows from its ID
    /// (`Some(None)`); a document's is its path, once `observe_path` found
    /// nothing there. `None` while it is not known to be free.
    pub(super) fn place(&self) -> Option<Option<String>> {
        match &self.inputs {
            Inputs::Ticket { .. } => Some(None),
            Inputs::Document(Place {
                check: PathCheck::Free(path),
                ..
            }) => Some(Some(path.clone())),
            Inputs::Document(_) => None,
        }
    }

    /// Checks the document's path as its fields hold it now, under a new
    /// stamp: here first, then with `observe_path` in the primary context.
    /// The last check is dropped, and its answer, were it to land, is not
    /// the latest's (R-22).
    #[allow(clippy::result_large_err)] // The library's read error, as every read gives it.
    pub(super) fn check_path(&mut self, cx: &mut Context<Self>) {
        let stamp = self.checks.next(ViewScope::Create);
        self.observe = None;
        let Inputs::Document(place) = &mut self.inputs else {
            return;
        };
        let folder = place.folder.read(cx).value();
        let file_name = place.file_name.read(cx).value();
        let path = match document_path(&folder, &file_name) {
            Ok(path) => path,
            Err(words) => {
                place.check = PathCheck::Refused(words);
                return;
            }
        };
        place.check = PathCheck::Checking;
        let root = PathBuf::from(&self.root);
        let asked = path.clone();
        self.observe = Some(self.services.read(
            ReadKind::ObservePath,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                service.observe_path(&repo, None, Path::new(&asked))
            },
            cx,
            move |dialog: &mut Self, stamp, result, cx| dialog.observed(stamp, path, result, cx),
        ));
    }

    /// The path check's answer: kept only when it is the latest asked.
    fn observed(
        &mut self,
        stamp: Stamp,
        path: String,
        result: ReadResult<PathObservationDto>,
        cx: &mut Context<Self>,
    ) {
        if !self.checks.is_current(&stamp) {
            return;
        }
        let Inputs::Document(place) = &mut self.inputs else {
            return;
        };
        place.check = match result {
            ReadResult::Ok(observed) if observed.exists => PathCheck::Held(path),
            ReadResult::Ok(_) => PathCheck::Free(path),
            ReadResult::Busy => PathCheck::Failed(ResultCode::Busy),
            ReadResult::Failed(failure) => PathCheck::Failed(failure.code),
        };
        cx.notify();
    }
}
