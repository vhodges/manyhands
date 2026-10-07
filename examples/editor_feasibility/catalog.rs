//! Fixed public repository documents only; never accepts a user-supplied path.
use super::session::Original;
use std::{fs, io, path::Path};

pub struct Document {
    pub id: &'static str,
    pub label: &'static str,
    pub original: Original,
}

const SOURCES: [(&str, &str); 4] = [
    ("readme", "README.md"),
    ("cli-contract", "docs/RFC/cli-contract.md"),
    ("wave-03", "docs/Waves/wave-03-dogfooding.md"),
    (
        "implementation",
        "docs/plans/2026-10-06-wave-03-readiness-exploration-implementation.md",
    ),
];

pub fn load() -> io::Result<Vec<Document>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // S1 CaptureRun uses a fixed relative target path. Refuse another cwd rather
    // than silently writing captures into an unrelated tree.
    if fs::canonicalize(std::env::current_dir()?)? != fs::canonicalize(root)? {
        return Err(io::Error::other(
            "launch from the example's repository root",
        ));
    }
    let mut docs = Vec::new();
    for (id, label) in SOURCES {
        let full = fs::read_to_string(root.join(label))?;
        docs.push(Document {
            id,
            label,
            original: Original::new(full).map_err(|e| io::Error::other(format!("{e:?}")))?,
        });
    }
    docs.push(Document {
        id: "mixed-math",
        label: "Synthetic mixed-math (normalization counterexample)",
        original: Original::new("What if words $$E=mc^2$$ more".into())
            .map_err(|e| io::Error::other(format!("{e:?}")))?,
    });
    Ok(docs)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_catalog_is_actual_source_with_protected_headers() {
        let docs = load().unwrap();
        assert_eq!(docs.len(), 5);
        for (doc, (_, path)) in docs.iter().zip(SOURCES) {
            assert_eq!(doc.original.full(), fs::read_to_string(path).unwrap());
            assert!(!doc.original.body().is_empty());
            assert_eq!(doc.original.header().is_empty(), doc.id == "readme");
        }
        assert_eq!(docs[4].original.body(), "What if words $$E=mc^2$$ more");
        let mut ids = docs.iter().map(|d| d.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), docs.len());
    }
}
