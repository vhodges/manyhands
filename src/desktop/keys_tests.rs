//! The binding table of the design's Layout And Keyboard section.

use gpui_kit::{Action, Keystroke, TestAppContext};

use super::{
    CloseTab, Find, FocusContent, FocusNavigation, FocusOperations, FocusRepositories, NextRegion,
    NextTab, OpenDrafts, OpenSelected, OpenSettings, PreviousRegion, PreviousTab, Quit, QuitNow,
    Rebuild, Refresh, bind,
};

#[gpui_kit::test]
fn each_key_is_bound_to_its_action(cx: &mut TestAppContext) {
    cx.update(bind);
    let table: [(&str, Box<dyn Action>); 16] = [
        ("f6", Box::new(NextRegion)),
        ("shift-f6", Box::new(PreviousRegion)),
        ("secondary-1", Box::new(FocusRepositories)),
        ("secondary-2", Box::new(FocusNavigation)),
        ("secondary-3", Box::new(FocusContent)),
        ("secondary-4", Box::new(FocusOperations)),
        ("secondary-p", Box::new(Find)),
        ("ctrl-tab", Box::new(NextTab)),
        ("ctrl-shift-tab", Box::new(PreviousTab)),
        ("secondary-w", Box::new(CloseTab)),
        ("secondary-r", Box::new(Refresh)),
        ("secondary-shift-r", Box::new(Rebuild)),
        ("secondary-,", Box::new(OpenSettings)),
        ("secondary-shift-d", Box::new(OpenDrafts)),
        ("secondary-q", Box::new(Quit)),
        // In the document tree; the ticket list is the Kit's own.
        ("enter", Box::new(OpenSelected)),
    ];
    cx.update(|cx| {
        for (keys, action) in &table {
            let keystroke = Keystroke::parse(keys).unwrap();
            let bound = cx.all_bindings_for_input(&[keystroke]);
            assert!(
                bound
                    .iter()
                    .any(|binding| binding.action().partial_eq(action.as_ref())),
                "{keys} is bound to {}",
                action.name()
            );
        }
        // Quit now is the focused button, never a key.
        assert!(
            cx.key_bindings()
                .borrow()
                .bindings_for_action(&QuitNow)
                .next()
                .is_none()
        );
    });
}
