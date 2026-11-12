//! The binding table of the design's Layout And Keyboard section.

use gpui_kit::{Action, Keystroke, TestAppContext};

use super::{
    CloseInspect, CloseTab, Find, FocusContent, FocusNavigation, FocusOperations,
    FocusRepositories, NextRegion, NextTab, OpenDockMenu, OpenDrafts, OpenSelected, OpenSettings,
    PreviousRegion, PreviousTab, Quit, QuitNow, Rebuild, Refresh, SelectFirst, SelectLast,
    ToggleBottomDock, ToggleLeftDock, ToggleRightDock, bind,
};

#[gpui_kit::test]
fn each_key_is_bound_to_its_action(cx: &mut TestAppContext) {
    cx.update(bind);
    let table: [(&str, Box<dyn Action>); 24] = [
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
        // In the ticket list and the document tree: the first and last row.
        ("home", Box::new(SelectFirst)),
        ("end", Box::new(SelectLast)),
        // In the Drafts page's Inspect panel.
        ("escape", Box::new(CloseInspect)),
        // The dock keys, in the workspace (Bold wins in an editable editor, decision 2).
        ("secondary-b", Box::new(ToggleLeftDock)),
        ("secondary-alt-b", Box::new(ToggleRightDock)),
        ("secondary-j", Box::new(ToggleBottomDock)),
        // On a focused panel toggle ("PanelToggle"): the dock menu.
        ("menu", Box::new(OpenDockMenu)),
        ("shift-f10", Box::new(OpenDockMenu)),
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

/// Ctrl B (Cmd B on macOS) is Bold in an editable editor and the left dock's
/// everywhere else, a read-only editor included, whichever of the two sets
/// of bindings is registered first (decision 2).
#[gpui_kit::test]
fn ctrl_b_is_bold_in_an_editable_editor_only_in_either_binding_order(cx: &mut TestAppContext) {
    use gpui_kit::KeyContext;
    use zorite_editor::Bold;

    let keystroke = Keystroke::parse(if cfg!(target_os = "macos") {
        "cmd-b"
    } else {
        "ctrl-b"
    })
    .unwrap();
    let stack = |contexts: &[&str]| -> Vec<KeyContext> {
        contexts
            .iter()
            .map(|context| KeyContext::parse(context).unwrap())
            .collect()
    };
    let editable = stack(&["Root", "Workspace", "Content", "Editor"]);
    let read_only = stack(&["Root", "Workspace", "Content", "ReadOnly", "Editor"]);
    let page = stack(&["Root", "Workspace", "Content"]);

    for editors_first in [false, true] {
        cx.update(|cx| {
            cx.clear_key_bindings();
            if editors_first {
                crate::desktop::view::editor::keys::bind(cx);
                bind(cx);
            } else {
                bind(cx);
                crate::desktop::view::editor::keys::bind(cx);
            }
            let first = |stack: &[KeyContext]| -> Box<dyn Action> {
                let keymap = cx.key_bindings();
                let (bindings, _) = keymap
                    .borrow()
                    .bindings_for_input(std::slice::from_ref(&keystroke), stack);
                bindings
                    .first()
                    .expect("Ctrl B is bound")
                    .action()
                    .boxed_clone()
            };
            let why = format!("editors registered first: {editors_first}");
            assert!(first(&editable).partial_eq(&Bold), "editable: Bold; {why}");
            assert!(
                first(&read_only).partial_eq(&ToggleLeftDock),
                "read-only: the dock; {why}"
            );
            assert!(
                first(&page).partial_eq(&ToggleLeftDock),
                "elsewhere: the dock; {why}"
            );
        });
    }
}
