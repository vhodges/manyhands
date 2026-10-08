//! The data shapes of JSON v1 reads.
//!
//! These are the only read types that serialize. A DTO is built from a
//! domain value field by field; no domain type serializes. Every field is
//! always present, and an absent value is `null`.

use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::results::ProblemCode;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NewIdDto {
    pub id: String,
}

/// A problem found in observed content.
///
/// `guidance` is written from the code's registry entry when the problem
/// serializes, so text stored with a problem can never take its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProblemDto {
    pub code: ProblemCode,
    /// Repository-relative, or `None` when the problem has no path or its
    /// path cannot be written as one.
    pub path: Option<String>,
}

impl ProblemDto {
    pub fn guidance(&self) -> &'static str {
        self.code.guidance()
    }
}

impl Serialize for ProblemDto {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut problem = serializer.serialize_struct("ProblemDto", 3)?;
        problem.serialize_field("code", &self.code)?;
        problem.serialize_field("path", &self.path)?;
        problem.serialize_field("guidance", self.guidance())?;
        problem.end()
    }
}
