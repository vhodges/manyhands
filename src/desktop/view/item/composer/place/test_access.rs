//! What the tests read of the composers.

use gpui_kit::Entity;
use manyhands::drafts::CommentTarget;

use crate::desktop::view::item::{ItemPane, composer::Composer};

impl ItemPane {
    /// The new comment's composer.
    pub(crate) fn new_composer(&self) -> Option<Entity<Composer>> {
        Some(self.composers.new.as_ref()?.composer.clone())
    }

    /// The reply composer and its thread's root.
    pub(crate) fn reply_composer(&self) -> Option<(String, Entity<Composer>)> {
        let (root, slot) = self.composers.reply.as_ref()?;
        Some((root.clone(), slot.composer.clone()))
    }

    /// The comment `composer` writes, once its ID is allocated.
    pub(crate) fn composer_target(&self, composer: &Entity<Composer>) -> Option<CommentTarget> {
        self.composers
            .slots()
            .find(|slot| slot.composer == *composer)?
            .target
            .clone()
    }
}
