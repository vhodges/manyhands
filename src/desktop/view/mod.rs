//! GPUI views.

pub(crate) mod confirm;
pub(crate) mod documents;
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) mod harness;
pub(crate) mod item;
pub(crate) mod operations;
pub(crate) mod sidebar;
pub(crate) mod tickets;
pub(crate) mod workspace;
