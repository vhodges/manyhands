//! The Documents pane: the selected repository's tree in the Kit's tree,
//! with its arrow bindings. Folders show by name, empty ones say so;
//! documents show by title with their path as secondary text; a
//! nonconforming document shows its path and problem code. Enter or a
//! click opens a document. New document… opens its dialog
//! (`view::create`); folder creation is visibly unavailable.

use std::{collections::HashMap, rc::Rc};

use gpui_kit::{
    App, AppContext, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, WeakEntity, Window,
    component::{
        ActiveTheme, Disableable,
        button::Button,
        h_flex,
        list::ListItem,
        tree::{TreeItem, TreeState, tree},
        v_flex,
    },
    div, px,
};

use super::{create::NEW_DOCUMENT, workspace::Workspace};
use crate::desktop::model::{documents::TreeNode, navigation::OpenTarget, stamp::RepoKey};

pub(crate) const NONCONFORMING: &str = "Nonconforming";
pub(crate) const EMPTY_FOLDER: &str = "Empty folder";
pub(crate) const NEW_FOLDER: &str = "New folder";
pub(crate) const NEW_FOLDER_UNAVAILABLE: &str = "Creating folders is not available yet.";

/// A folder's name: the last part of its path.
fn folder_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

/// What a node is labeled by: a folder by name, a document by title, and
/// a nonconforming document, which has no title, by path.
pub(crate) fn node_label(node: &TreeNode) -> String {
    match node {
        TreeNode::Folder { path, .. } => folder_name(path).to_owned(),
        TreeNode::Document(item) => item.title.clone().unwrap_or_else(|| item.path.clone()),
    }
}

/// A drawn node's facts, by its tree id.
enum Node {
    Folder {
        label: String,
        empty: bool,
    },
    Document {
        label: String,
        path: String,
        problem: Option<&'static str>,
        target: OpenTarget,
    },
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

    /// Shows `nodes`, the tree of `repo`, every folder open. The selected
    /// row stays selected when the new tree still has it: a reload after a
    /// refresh does not lose the user's place.
    pub(crate) fn set(&mut self, repo: &RepoKey, nodes: &[TreeNode], cx: &mut App) {
        let mut facts = HashMap::new();
        let items = items(repo, nodes, &mut facts);
        self.nodes = Rc::new(facts);
        self.tree.update(cx, |tree, cx| {
            let selected = tree.selected_item().map(|item| item.id.clone());
            tree.set_items(items, cx);
            if let Some(selected) = selected {
                let at = tree.index_of(&selected);
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

    /// What the selected node opens; nothing for a folder.
    pub(crate) fn selected_target(&self, cx: &App) -> Option<OpenTarget> {
        let id = &self.tree.read(cx).selected_item()?.id;
        match self.nodes.get(id)? {
            Node::Document { target, .. } => Some(target.clone()),
            Node::Folder { .. } => None,
        }
    }

    /// The tree, with folder creation shown unavailable above it.
    pub(crate) fn render(
        &self,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Workspace>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let nodes = self.nodes.clone();
        let opener = workspace.clone();
        let rows = tree(&self.tree, move |ix, entry, selected, _, _| {
            let indent = px(12. * entry.depth() as f32);
            let item = ListItem::new(("doc-item", ix))
                .pl(indent)
                .selected(selected);
            let Some(node) = nodes.get(&entry.item().id) else {
                return item;
            };
            let (label, secondary) = match node {
                Node::Folder { label, empty } => (
                    label.clone(),
                    empty.then(|| {
                        h_flex().child(
                            div()
                                .id(("doc-empty", ix))
                                .test_support()
                                .aria_label(EMPTY_FOLDER)
                                .text_xs()
                                .text_color(muted)
                                .child(EMPTY_FOLDER),
                        )
                    }),
                ),
                Node::Document {
                    label,
                    path,
                    problem,
                    ..
                } => (
                    label.clone(),
                    Some(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .id(("doc-path", ix))
                                    .test_support()
                                    .aria_label(path.clone())
                                    .text_xs()
                                    .text_color(muted)
                                    .child(path.clone()),
                            )
                            .children(problem.map(|code| {
                                h_flex()
                                    .gap_1()
                                    .text_xs()
                                    .child(
                                        div()
                                            .id(("doc-nonconforming", ix))
                                            .test_support()
                                            .aria_label(NONCONFORMING)
                                            .child(NONCONFORMING),
                                    )
                                    .child(
                                        div()
                                            .id(("doc-problem", ix))
                                            .test_support()
                                            .aria_label(code)
                                            .child(code),
                                    )
                            })),
                    ),
                ),
            };
            let item = match node {
                Node::Document { target, .. } => {
                    let target = target.clone();
                    let workspace = workspace.clone();
                    item.on_click(move |_: &ClickEvent, _, cx| {
                        let _ = workspace
                            .update(cx, |workspace, cx| workspace.open_item(target.clone(), cx));
                    })
                }
                Node::Folder { .. } => item,
            };
            item.child(
                v_flex()
                    .id(("doc-node", ix))
                    .test_support()
                    .aria_label(label.clone())
                    .aria_selected(selected)
                    .child(div().text_sm().child(label))
                    .children(secondary),
            )
        });
        // New document…, before New folder (`03-panels.md`): its dialog
        // (`view::create`).
        let new_document = div().id("new-document").test_support().child(
            Button::new("new-document-button")
                .label(NEW_DOCUMENT)
                .on_click(move |_: &ClickEvent, window, cx| {
                    let _ = opener.update(cx, |workspace, cx| workspace.new_item(window, cx));
                }),
        );
        v_flex()
            .gap_1()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .gap_2()
                    .child(new_document)
                    .child(
                        div().id("new-folder").test_support().child(
                            Button::new("new-folder-button")
                                .label(NEW_FOLDER)
                                .disabled(true),
                        ),
                    )
                    .child(
                        div()
                            .id("new-folder-unavailable")
                            .test_support()
                            .aria_label(NEW_FOLDER_UNAVAILABLE)
                            .text_xs()
                            .text_color(muted)
                            .child(NEW_FOLDER_UNAVAILABLE),
                    ),
            )
            .child(
                div()
                    .id("documents-tree")
                    .test_support()
                    .key_context(crate::desktop::keys::DOCUMENTS_CONTEXT)
                    .flex_1()
                    .min_h_0()
                    .child(rows),
            )
    }
}

/// The Kit's items for `nodes`, recording each id's facts in `facts`.
fn items(
    repo: &RepoKey,
    nodes: &[TreeNode],
    facts: &mut HashMap<SharedString, Node>,
) -> Vec<TreeItem> {
    nodes
        .iter()
        .map(|node| {
            let label = node_label(node);
            match node {
                TreeNode::Folder {
                    path,
                    empty,
                    children,
                } => {
                    let id: SharedString = format!("folder:{path}").into();
                    let children = items(repo, children, facts);
                    facts.insert(
                        id.clone(),
                        Node::Folder {
                            label: label.clone(),
                            empty: *empty,
                        },
                    );
                    TreeItem::new(id, label).expanded(true).children(children)
                }
                TreeNode::Document(item) => {
                    // Paths are unique within one context's list; a
                    // document in an item worktree has its own.
                    let id: SharedString =
                        format!("document:{}:{}", item.context.worktree, item.path).into();
                    let problem = match item.id {
                        Some(_) => None,
                        None => item.problems.first().map(|problem| problem.code.as_str()),
                    };
                    facts.insert(
                        id.clone(),
                        Node::Document {
                            label: label.clone(),
                            path: item.path.clone(),
                            problem,
                            target: OpenTarget::for_item(repo, item),
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
