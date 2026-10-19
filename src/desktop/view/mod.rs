//! GPUI views.

pub(crate) mod documents;
#[cfg(all(test, feature = "desktop-test-support"))]
pub(crate) mod harness;
pub(crate) mod sidebar;
pub(crate) mod tickets;
pub(crate) mod workspace;
