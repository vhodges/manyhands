//! Thin pinned-candidate seam; no second editing model and no resource loaders.
use super::{
    evidence::{Provenance, Readback},
    session::Draft,
};
use gpui_kit::*;
use zorite_editor::{EditorEvent, EditorState, SyntaxStyle};

pub fn create(body: &str, window: &mut Window, cx: &mut App) -> Entity<EditorState> {
    cx.new(|cx| {
        let mut editor = EditorState::new(window, cx).with_text(body);
        // Providers, icons, block resolvers and clipboard writer stay absent.
        editor.set_markdown_style(style(), cx);
        editor
    })
}

pub fn text<'a>(editor: &'a Entity<EditorState>, cx: &'a App) -> &'a str {
    editor.read(cx).text()
}

pub fn readback(editor: &Entity<EditorState>, generation: u64, cx: &App) -> Readback {
    Readback {
        generation,
        body: text(editor, cx).to_owned(),
        provenance: Provenance::ActualEditorReadback,
    }
}

pub fn presentation(editor: &Entity<EditorState>, rich: bool, cx: &mut App) {
    editor.update(cx, |editor, cx| {
        if rich {
            editor.set_markdown_style(style(), cx);
        } else {
            editor.clear_markdown_style(cx);
        }
    });
}

pub fn focus(editor: &Entity<EditorState>, window: &mut Window, cx: &mut App) {
    editor.read(cx).focus_handle(cx).focus(window, cx);
}

/// Stable document identity + generation, not the current selector index.
/// Missing/dropped docs are rejected by the host before this guard.
pub fn accepts(
    draft: &Draft,
    expected_id: &str,
    id: &str,
    generation: u64,
    same_entity: bool,
) -> bool {
    same_entity && expected_id == id && draft.check_generation(generation).is_ok()
}

/// Fixed categories only: never logs destination strings, LaTeX or properties.
pub fn event_category(event: &EditorEvent) -> &'static str {
    match event {
        EditorEvent::Changed => "changed-event",
        EditorEvent::SelectionChanged => "selection-event",
        EditorEvent::OpenLink(_) => "denied-link-request",
        EditorEvent::OpenWikiLink(_) => "denied-wiki-request",
        EditorEvent::PreviewImage(_) => "denied-image-preview-request",
        EditorEvent::EditMath { .. } => "unsupported-math-edit-request",
        EditorEvent::MathMenu { .. } => "unsupported-math-menu-request",
        EditorEvent::EditProperties { .. } => "unsupported-properties-edit-request",
    }
}

fn style() -> SyntaxStyle {
    let muted = rgb(0x8593a3).into();
    let accent = rgb(0x78b8ff).into();
    SyntaxStyle {
        marker: muted,
        code: rgb(0xf4c280).into(),
        code_bg: rgb(0x293241).into(),
        link: accent,
        tag: accent,
        quote: muted,
        alert_note: accent,
        alert_tip: rgb(0x81c995).into(),
        alert_important: rgb(0xc5a3ff).into(),
        alert_warning: rgb(0xf4c280).into(),
        alert_caution: rgb(0xff9292).into(),
        alert_icons: None,
        rule: muted,
        mark_bg: rgb(0x665b28).into(),
        block_label: None,
        block_label_gen: 0,
        block_ref_count: None,
        popover_bg: rgb(0x202733).into(),
        popover_border: muted,
        popover_fg: rgb(0xe6edf3).into(),
        popover_hover: rgb(0x34465f).into(),
        popover_divider: muted,
        popover_danger: rgb(0xff9292).into(),
        mono: font("monospace"),
        property_icon: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{accepts, event_category};
    use crate::session::{Draft, Original};
    use zorite_editor::EditorEvent;

    #[test]
    fn routing_rejects_other_doc_and_stale_generation() {
        let mut draft = Draft::new(Original::new("body".into()).unwrap());
        draft.observe(0, "body".into()).unwrap();
        assert!(accepts(&draft, "a", "a", 0, true));
        assert!(!accepts(&draft, "a", "b", 0, true));
        assert!(!accepts(&draft, "a", "a", 0, false));
        draft
            .replace_clean(0, Original::new("next".into()).unwrap())
            .unwrap();
        assert!(!accepts(&draft, "a", "a", 0, true));
        assert!(accepts(&draft, "a", "a", 1, true));
    }

    #[test]
    fn requests_are_denied_categories_not_content_or_actions() {
        assert_eq!(
            event_category(&EditorEvent::OpenLink("https://invalid.example".into())),
            "denied-link-request"
        );
        assert_eq!(
            event_category(&EditorEvent::PreviewImage("../../private".into())),
            "denied-image-preview-request"
        );
        assert_eq!(
            event_category(&EditorEvent::OpenWikiLink("hidden title".into())),
            "denied-wiki-request"
        );
    }
}
