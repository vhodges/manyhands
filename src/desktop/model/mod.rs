//! View state, free of GPUI but for the discussion's avatar colour, a
//! GPUI `Hsla` (`discussion`).

pub(crate) mod activity;
pub(crate) mod attention;
pub(crate) mod conflict;
pub(crate) mod create;
pub(crate) mod discussion;
pub(crate) mod doc_rows;
pub(crate) mod documents;
pub(crate) mod drafts;
pub(crate) mod edit;
pub(crate) mod editor_text;
pub(crate) mod find;
pub(crate) mod item;
pub(crate) mod layout;
pub(crate) mod lifecycle_lock;
pub(crate) mod links;
pub(crate) mod list_state;
pub(crate) mod list_view;
pub(crate) mod markdown_edit;
pub(crate) mod markers;
pub(crate) mod names;
pub(crate) mod navigation;
pub(crate) mod polling;
pub(crate) mod publication;
pub(crate) mod read_state;
pub(crate) mod relations_edit;
pub(crate) mod relationships;
pub(crate) mod repository_state;
pub(crate) mod resources;
pub(crate) mod review;
pub(crate) mod stamp;
pub(crate) mod status;
pub(crate) mod tabs;
pub(crate) mod ticket_view;
pub(crate) mod tickets;
pub(crate) mod unreadable;

pub(crate) mod operations;
