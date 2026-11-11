//! The Documents pane: the selected repository's tree in the Kit's base
//! tree, one `ListRow` per node (Part F), always drawn while a repository is
//! selected (R-34). A folder shows a chevron, its name, and its document
//! count while it is closed; a document its title and, while a draft of it
//! is held, the unsaved marker; a nonconforming file its file name in amber.
//! A row's path and the reason for its mark are its tooltip, which is also
//! its accessible description.
//!
//! The tree's own keys are up, down, left and right; Enter and Home and End
//! are the Workspace's (`workspace::lists`). A click on a document opens it;
//! a click on a folder toggles it, the tree's own doing. The folders the
//! person closed are the model's (`Documents::expansion`): the Workspace
//! hears the tree's events and keeps them, and a rebuild sets each item's
//! state from them. New document… and New folder… open their dialogs
//! (`view::create`, `view::new_folder`).

use std::{collections::HashMap, rc::Rc};

use gpui_kit::{
    App, AppContext, ClickEvent, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, ScrollStrategy, SharedString, StyleRefinement, Styled, TestSupportExt,
    WeakEntity, Window,
    base::Tree,
    component::{
        button::Button,
        h_flex,
        scroll::ScrollableElement,
        tree::{TreeItem, TreeState},
        v_flex,
    },
    div,
};
use manyhands::canonical::ItemId;

use self::row::document_row;
use super::{
    create::NEW_DOCUMENT,
    workspace::{Workspace, unless_dialog},
};
use crate::desktop::{
    keys::{DOCUMENTS_CONTEXT, SelectFirst, SelectLast},
    model::{
        doc_rows::{DocRow, Expansion, doc_row, visible_selection},
        documents::{TreeNode, document_id, folder_id, unreadable_id},
        navigation::{OpenTarget, Pane},
        stamp::RepoKey,
    },
};

mod row;

pub(crate) const NEW_FOLDER: &str = "New folder\u{2026}";
/// An empty folder's trailing cell, where a count would be (decision 14).
pub(crate) const EMPTY_TRAILING: &str = "empty";

/// Whether the person has unsaved work on a document, by its ID.
pub(crate) type Unsaved = Rc<dyn Fn(&ItemId, &App) -> bool>;

/// A drawn document: what a click or Enter opens.
struct Document {
    target: OpenTarget,
    /// The path, which a test reads back (E-8).
    #[cfg(all(test, feature = "desktop-test-support"))]
    path: String,
}

/// A drawn node's facts, by its tree id.
struct Node {
    /// The row while its folder is closed, and while it is open: the tree
    /// toggles a folder with no word to the pane, so both are kept. A node
    /// that is not a folder with child nodes has the one row twice.
    rows: [DocRow; 2],
    /// A folder's path, below the repository root: where New folder… makes
    /// a folder while this one is selected.
    folder: Option<String>,
    document: Option<Document>,
}

impl Node {
    fn row(&self, open: bool) -> &DocRow {
        &self.rows[usize::from(open)]
    }

    /// The ID a held draft of the node's document would name.
    fn item_id(&self) -> Option<&ItemId> {
        match &self.document.as_ref()?.target {
            OpenTarget::Item { id, .. } => Some(id),
            OpenTarget::Inspect { .. } => None,
        }
    }
}

/// The tree's Kit state, and what each of its ids stands for.
pub(crate) struct DocumentsPane {
    tree: Entity<TreeState>,
    nodes: Rc<HashMap<SharedString, Node>>,
}

impl DocumentsPane {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            tree: cx.new(|cx| TreeState::new(cx)),
            nodes: Rc::default(),
        }
    }

    /// The Kit's state, whose events tell the Workspace what the person
    /// opened and closed.
    pub(crate) fn tree(&self) -> &Entity<TreeState> {
        &self.tree
    }

    /// Shows `nodes`, the tree of `repo`, each folder open or closed as
    /// `expansion` says. The selected row stays selected when the new tree
    /// still has it, and goes to its nearest visible folder when a
    /// collapsed one hides it: a reload after a refresh does not lose the
    /// user's place. The selection is set by index, never by an item,
    /// which would expand its ancestors (R-27).
    pub(crate) fn set(
        &mut self,
        repo: &RepoKey,
        nodes: &[TreeNode],
        expansion: &Expansion,
        cx: &mut App,
    ) {
        let mut facts = HashMap::new();
        let items = items(repo, nodes, expansion, &mut facts);
        self.nodes = Rc::new(facts);
        self.tree.update(cx, |tree, cx| {
            let selected = tree.selected_item().map(|item| item.id.to_string());
            tree.set_items(items, cx);
            let visible = selected.and_then(|id| visible_selection(nodes, expansion, &id));
            if let Some(visible) = visible {
                let at = tree.index_of(&SharedString::from(visible));
                tree.set_selected_index(at, cx);
            }
        });
    }

    /// Nothing is shown.
    pub(crate) fn clear(&mut self, cx: &mut App) {
        self.nodes = Rc::default();
        self.tree
            .update(cx, |tree, cx| tree.set_items(Vec::new(), cx));
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut App) {
        self.tree.update(cx, |tree, cx| tree.focus(window, cx));
    }

    /// Selects the first or the last row and scrolls to it (R-15). Nothing
    /// when there is none.
    pub(crate) fn select_edge(&self, last: bool, cx: &mut App) {
        self.tree.update(cx, |tree, cx| {
            let count = (0..).take_while(|ix| tree.entry(*ix).is_some()).count();
            if count == 0 {
                return;
            }
            let (ix, strategy) = if last {
                (count - 1, ScrollStrategy::Bottom)
            } else {
                (0, ScrollStrategy::Top)
            };
            tree.set_selected_index(Some(ix), cx);
            tree.scroll_to_item(ix, strategy);
        });
    }

    /// What the selected node opens; nothing for a folder.
    pub(crate) fn selected_target(&self, cx: &App) -> Option<OpenTarget> {
        let id = &self.tree.read(cx).selected_item()?.id;
        Some(self.nodes.get(id)?.document.as_ref()?.target.clone())
    }

    /// The path of the selected folder; nothing for a document, or with
    /// nothing selected.
    pub(crate) fn selected_folder(&self, cx: &App) -> Option<String> {
        let id = &self.tree.read(cx).selected_item()?.id;
        self.nodes.get(id)?.folder.clone()
    }

    /// The selected folder, when Enter can toggle it (R-24): its path and
    /// whether it is open. A folder with no child nodes cannot toggle.
    pub(crate) fn toggle_target(&self, cx: &App) -> Option<(String, bool)> {
        let tree = self.tree.read(cx);
        let entry = tree.selected_entry()?;
        if !entry.is_folder() {
            return None;
        }
        let folder = self.nodes.get(&entry.item().id)?.folder.clone()?;
        Some((folder, entry.is_expanded()))
    }

    /// The paths of the documents drawn, in the tree's order (E-8).
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn document_paths(&self, cx: &App) -> Vec<String> {
        let tree = self.tree.read(cx);
        (0..)
            .map_while(|ix| tree.entry(ix))
            .filter_map(|entry| self.nodes.get(&entry.item().id)?.document.as_ref())
            .map(|document| document.path.clone())
            .collect()
    }

    /// The tree, with New document… and New folder… above it. `focus` is
    /// the Documents region's handle: its selected row draws a ring while
    /// the region holds focus; `unsaved` says which documents hold a draft.
    pub(crate) fn render(
        &self,
        workspace: WeakEntity<Workspace>,
        focus: FocusHandle,
        unsaved: Unsaved,
        cx: &mut Context<Workspace>,
    ) -> impl IntoElement + use<> {
        let nodes = self.nodes.clone();
        let opener = workspace.clone();
        let folder_opener = workspace.clone();
        let scroll = self.tree.read(cx).scroll_handle().clone();
        let rows = Tree::new(&self.tree)
            .item(move |ix, entry, state, window, cx| {
                let Some(node) = nodes.get(&entry.item().id) else {
                    return div().into_any_element();
                };
                let held = node.item_id().is_some_and(|id| unsaved(id, cx));
                document_row(
                    ix,
                    entry,
                    node,
                    (
                        state.is_selected(),
                        focus.contains_focused(window, cx),
                        held,
                    ),
                    workspace.clone(),
                    cx,
                )
                .into_any_element()
            })
            .list_style(StyleRefinement::default().flex_grow_1().size_full())
            .relative()
            .size_full();
        // New document…, before New folder… (`03-panels.md`): each opens
        // its dialog (`view::create`, `view::new_folder`).
        let new_document = div().id("new-document").test_support().child(
            Button::new("new-document-button")
                .label(NEW_DOCUMENT)
                .on_click(move |_: &ClickEvent, window, cx| {
                    let _ = opener.update(cx, |workspace, cx| {
                        workspace.new_item(Pane::Documents, window, cx)
                    });
                }),
        );
        let new_folder = div().id("new-folder").test_support().child(
            Button::new("new-folder-button").label(NEW_FOLDER).on_click(
                move |_: &ClickEvent, window, cx| {
                    let _ =
                        folder_opener.update(cx, |workspace, cx| workspace.new_folder(window, cx));
                },
            ),
        );
        v_flex()
            .gap_1()
            .size_full()
            .min_h_0()
            .child(h_flex().gap_2().child(new_document).child(new_folder))
            .child(
                div()
                    .id("documents-tree")
                    .test_support()
                    .key_context(DOCUMENTS_CONTEXT)
                    .on_action(unless_dialog(cx, |this, _: &SelectFirst, _, cx| {
                        this.select_document_edge(false, cx)
                    }))
                    .on_action(unless_dialog(cx, |this, _: &SelectLast, _, cx| {
                        this.select_document_edge(true, cx)
                    }))
                    .flex_1()
                    .min_h_0()
                    .child(rows)
                    .vertical_scrollbar(&scroll),
            )
    }
}

/// The Kit's items for `nodes`, recording each id's facts in `facts`.
fn items(
    repo: &RepoKey,
    nodes: &[TreeNode],
    expansion: &Expansion,
    facts: &mut HashMap<SharedString, Node>,
) -> Vec<TreeItem> {
    nodes
        .iter()
        .map(|node| {
            let rows = [doc_row(node, false), doc_row(node, true)];
            let label = rows[0].label.clone();
            match node {
                TreeNode::Folder { path, children, .. } => {
                    let id: SharedString = folder_id(path).into();
                    let children = items(repo, children, expansion, facts);
                    facts.insert(
                        id.clone(),
                        Node {
                            rows,
                            folder: Some(path.clone()),
                            document: None,
                        },
                    );
                    TreeItem::new(id, label)
                        .expanded(expansion.is_expanded(path))
                        .children(children)
                }
                TreeNode::Document(item) => {
                    let id: SharedString = document_id(item).into();
                    facts.insert(
                        id.clone(),
                        Node {
                            rows,
                            folder: None,
                            document: Some(Document {
                                target: OpenTarget::for_item(repo, item),
                                #[cfg(all(test, feature = "desktop-test-support"))]
                                path: item.path.clone(),
                            }),
                        },
                    );
                    TreeItem::new(id, label)
                }
                // It selects, and it opens nothing: no `Document`, so
                // neither a click nor Enter has a target (R-38).
                TreeNode::Unreadable(file) => {
                    let id: SharedString = unreadable_id(file).into();
                    facts.insert(
                        id.clone(),
                        Node {
                            rows,
                            folder: None,
                            document: None,
                        },
                    );
                    TreeItem::new(id, label)
                }
            }
        })
        .collect()
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "documents_tests.rs"]
mod tests;
