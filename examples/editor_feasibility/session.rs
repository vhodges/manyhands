//! In-memory scratch policy only; no editor, repository service or persistence.
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionError {
    UnclosedHeader,
    InvalidRange,
    StaleGeneration,
    DirtyDraft,
    MissingReadback,
    GenerationOverflow,
}

/// Exact immutable UTF-8 source. A leading YAML delimiter is retained verbatim;
/// the spike does not parse, repair or serialize canonical metadata.
#[derive(Clone, Debug)]
pub struct Original {
    full: String,
    body_start: usize,
}

impl Original {
    pub fn new(full: String) -> Result<Self, SessionError> {
        let mut lines = full.split_inclusive('\n');
        let first = lines.next().unwrap_or("");
        let body_start = if first == "---\n" || first == "---\r\n" {
            let mut offset = first.len();
            let mut end = None;
            for line in lines {
                offset += line.len();
                if line == "---\n" || line == "---\r\n" || line == "---" {
                    end = Some(offset);
                    break;
                }
            }
            end.ok_or(SessionError::UnclosedHeader)?
        } else {
            0
        };
        Ok(Self { full, body_start })
    }

    pub fn full(&self) -> &str {
        &self.full
    }

    pub fn header(&self) -> &str {
        &self.full[..self.body_start]
    }

    pub fn body(&self) -> &str {
        &self.full[self.body_start..]
    }

    pub fn with_body(&self, body: &str) -> String {
        format!("{}{body}", self.header())
    }
}

pub fn validate_range(text: &str, range: &Range<usize>) -> Result<(), SessionError> {
    if range.start > range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
    {
        return Err(SessionError::InvalidRange);
    }
    Ok(())
}

/// Independent expected edit, not an editor operation. Candidate readback must
/// be compared against this, including every untouched byte.
pub fn expected_edit(
    text: &str,
    range: Range<usize>,
    replacement: &str,
) -> Result<String, SessionError> {
    validate_range(text, &range)?;
    Ok(format!(
        "{}{}{}",
        &text[..range.start],
        replacement,
        &text[range.end..]
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraftStatus {
    pub changed_on_load: bool,
    pub user_edits: bool,
    /// Conservative byte-dirty: normalization alone prevents clean replacement.
    pub dirty: bool,
}

#[derive(Debug)]
pub struct Draft {
    original: Original,
    generation: u64,
    loaded: Option<String>,
    current: Option<String>,
}

impl Draft {
    pub fn new(original: Original) -> Self {
        Self {
            original,
            generation: 0,
            loaded: None,
            current: None,
        }
    }

    pub fn original(&self) -> &Original {
        &self.original
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// The host must sample actual text after load, actions, undo/redo and mode
    /// switches; Changed alone is insufficient. First readback defines load.
    /// Calling this in a helper test is simulated evidence, never editor proof.
    pub fn observe(&mut self, generation: u64, body: String) -> Result<(), SessionError> {
        self.check_generation(generation)?;
        if self.loaded.is_none() {
            self.loaded = Some(body.clone());
        }
        self.current = Some(body);
        Ok(())
    }

    pub fn status(&self) -> Option<DraftStatus> {
        let loaded = self.loaded.as_deref()?;
        let current = self.current.as_deref()?;
        Some(DraftStatus {
            changed_on_load: loaded != self.original.body(),
            user_edits: current != loaded,
            dirty: current != self.original.body() || current != loaded,
        })
    }

    pub fn check_generation(&self, generation: u64) -> Result<(), SessionError> {
        if self.generation != generation {
            return Err(SessionError::StaleGeneration);
        }
        Ok(())
    }

    /// Explicit clean replacement only; callers reload the editor afterwards.
    /// Old-generation completions cannot mutate the new scratch draft.
    pub fn replace_clean(
        &mut self,
        generation: u64,
        original: Original,
    ) -> Result<u64, SessionError> {
        self.check_generation(generation)?;
        let status = self.status().ok_or(SessionError::MissingReadback)?;
        if status.dirty {
            return Err(SessionError::DirtyDraft);
        }
        let next = self
            .generation
            .checked_add(1)
            .ok_or(SessionError::GenerationOverflow)?;
        self.original = original;
        self.generation = next;
        self.loaded = None;
        self.current = None;
        Ok(next)
    }
}
