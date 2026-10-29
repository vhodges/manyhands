//! A document's details (design, Details sheet, Document): Title,
//! required; Status, Date and Author from its other keys, exactly as the
//! library gives them and never editable (fact 58, decision 14); its
//! Folder; the remaining keys as Other fields; the Facts, the Problems and
//! the Inspect link. Move… joins the Folder in Part E.

use gpui_kit::{AnyElement, Context, IntoElement, Window};
use manyhands::repository::{ItemDto, ItemDtoKind};

use super::{super::header::folder_and_file, Row};
use crate::desktop::{
    model::item::{NOT_REPRESENTABLE, metadata_value},
    view::item::{ChipField, ItemPane, text},
};

const STATUS: Row = Row::new("sheet-status", "Status");
const DATE: Row = Row::new("sheet-date", "Date");
const AUTHOR: Row = Row::new("sheet-author", "Author");
const FOLDER: Row = Row::new("sheet-folder", "Folder");

/// The other keys a document's sheet shows in rows of their own.
const NAMED: [&str; 3] = ["status", "date", "author"];

/// The value of the other key `key`, as the library gives it.
fn key_value(item: &ItemDto, id: &'static str, key: &str) -> Option<AnyElement> {
    let shown = metadata_value(item.unknown_metadata.get(key)?)
        .unwrap_or_else(|| NOT_REPRESENTABLE.to_owned());
    Some(text(id, shown).into_any_element())
}

impl ItemPane {
    pub(super) fn document_rows(
        &mut self,
        item: &ItemDto,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut rows = vec![self.title_row(item, window, cx)];
        for (row, id, key, field) in [
            (STATUS, "item-status", NAMED[0], ChipField::Status),
            (DATE, "item-date", NAMED[1], ChipField::Date),
            (AUTHOR, "item-author", NAMED[2], ChipField::Author),
        ] {
            let focus = self.sheet_focus[field.index()].clone();
            rows.push(self.row(row, key_value(item, id, key), Some(&focus), window, cx));
        }
        let (folder, _) = folder_and_file(&item.path);
        let folder = folder.map(|folder| text("item-folder", folder).into_any_element());
        rows.push(self.row(FOLDER, folder, None, window, cx));
        rows.push(self.other_fields(item, &NAMED, cx));
        rows.push(self.facts(item, ItemDtoKind::Document, window, cx));
        rows.extend(self.file_rows(item, cx));
        rows
    }
}
