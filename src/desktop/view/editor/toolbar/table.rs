//! The toolbar's table controls (design, "The toolbar", the Table row):
//! the editor's own table methods (facts 36 and 80), on the table the
//! caret is in.
//!
//! Markdown mode draws no table, and `set_caret_table_align` reads the
//! drawn rows, so it does nothing there. In Markdown the alignment is the
//! host edit `markdown_edit::align_column` instead, which writes the
//! separator row exactly as the editor would. The row and column methods
//! read the text, and work in both modes.

use gpui_kit::Context;
use zorite_editor::CellAlign;

use super::{Toolbar, apply, controls::Control};
use crate::desktop::model::markdown_edit::{ColumnAlign, align_column};

impl Toolbar {
    /// Does what the table control `control` names.
    pub(super) fn table(&mut self, control: Control, cx: &mut Context<Self>) {
        let (cell, column) = match control {
            Control::RowBelow => {
                return self
                    .editor
                    .update(cx, |editor, cx| editor.insert_table_row(true, cx));
            }
            Control::ColumnRight => {
                return self
                    .editor
                    .update(cx, |editor, cx| editor.insert_table_column(true, cx));
            }
            Control::DeleteRow => {
                return self
                    .editor
                    .update(cx, |editor, cx| editor.delete_table_row(cx));
            }
            Control::DeleteColumn => {
                return self
                    .editor
                    .update(cx, |editor, cx| editor.delete_table_column(cx));
            }
            Control::AlignLeft => (CellAlign::Left, ColumnAlign::Left),
            Control::AlignCentre => (CellAlign::Center, ColumnAlign::Centre),
            Control::AlignRight => (CellAlign::Right, ColumnAlign::Right),
            _ => return,
        };
        if self.host.read(cx).is_rich() {
            self.editor
                .update(cx, |editor, cx| editor.set_caret_table_align(cell, cx));
            return;
        }
        let editor = self.editor.read(cx);
        if let Some(edit) = align_column(editor.text(), editor.cursor(), column) {
            apply(&self.editor, edit, cx);
        }
    }
}
