//! Shared name components and fallbacks; each surface keeps its own layout.

pub(crate) struct TicketName<'a> {
    pub slug: Option<&'a str>,
    pub title: Option<&'a str>,
}

impl<'a> TicketName<'a> {
    pub(crate) fn title_or(&self, fallback: &'a str) -> &'a str {
        self.title.unwrap_or(fallback)
    }

    pub(crate) fn short_or(&self, fallback: &'a str) -> &'a str {
        self.slug.unwrap_or(fallback)
    }

    pub(crate) fn with_short(&self, fallback_id: &str) -> String {
        let short = self.slug.unwrap_or(fallback_id);
        match self.title {
            Some(title) => format!("{short} {title}"),
            None => short.to_owned(),
        }
    }

    pub(crate) fn parts(&self) -> impl Iterator<Item = &'a str> {
        [self.slug, self.title].into_iter().flatten()
    }
}
