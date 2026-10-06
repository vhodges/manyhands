//! Honest byte comparisons and explicit, fresh scratch captures. No native
//! execution is provided here; only the S2 adapter may label Editor readback.
use super::session::{Draft, Original, SessionError};
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Provenance {
    ActualEditorReadback,
    SimulatedHelper,
}

#[derive(Debug)]
pub struct Readback {
    pub generation: u64,
    pub body: String,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Comparison {
    pub header_exact: bool,
    pub body_exact: bool,
    pub full_exact: bool,
}

pub fn compare_full(original: &Original, candidate: &str) -> Comparison {
    let header_exact = candidate
        .as_bytes()
        .starts_with(original.header().as_bytes());
    let body_exact = header_exact && candidate[original.header().len()..] == *original.body();
    Comparison {
        header_exact,
        body_exact,
        full_exact: candidate == original.full(),
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub enum EvidenceStatus {
    MissingNotTested,
    SimulatedHelper,
    ActualEditorReadback,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Assessment {
    pub evidence_status: EvidenceStatus,
    /// None is missing/not-tested, not an empty or original candidate.
    pub provenance: Option<Provenance>,
    pub comparison: Option<Comparison>,
    pub actual_exact_preservation: bool,
}

pub fn assess(draft: &Draft, readback: Option<&Readback>) -> Result<Assessment, SessionError> {
    let Some(readback) = readback else {
        return Ok(Assessment {
            evidence_status: EvidenceStatus::MissingNotTested,
            provenance: None,
            comparison: None,
            actual_exact_preservation: false,
        });
    };
    draft.check_generation(readback.generation)?;
    // The protected header is host-owned, not passed through the editor.
    let comparison = compare_full(
        draft.original(),
        &draft.original().with_body(&readback.body),
    );
    let actual_exact_preservation =
        readback.provenance == Provenance::ActualEditorReadback && comparison.full_exact;
    Ok(Assessment {
        evidence_status: match readback.provenance {
            Provenance::ActualEditorReadback => EvidenceStatus::ActualEditorReadback,
            Provenance::SimulatedHelper => EvidenceStatus::SimulatedHelper,
        },
        provenance: Some(readback.provenance),
        comparison: Some(comparison),
        actual_exact_preservation,
    })
}

pub fn hash(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}

#[derive(Serialize)]
struct CaptureManifest<'a> {
    generation: u64,
    original_bytes: usize,
    original_blake3: String,
    header_bytes: usize,
    header_blake3: String,
    candidate_body_bytes: Option<usize>,
    candidate_body_blake3: Option<String>,
    candidate_full_blake3: Option<String>,
    changed_on_load: Option<bool>,
    user_edits: Option<bool>,
    dirty: Option<bool>,
    assessment: Assessment,
    context: &'a CaptureContext,
}

/// Supplied by the host from the running build and explicit action trace.
/// These strings record assertions, not automatic native verification.
#[derive(Serialize)]
pub struct CaptureContext {
    pub tested_head: String,
    pub lock_sha256: String,
    pub action_status: Vec<String>,
}

/// Owns a newly created ignored directory, never an arbitrary output path.
/// No home/cache/key reads and no writes to the original document are possible.
pub struct CaptureRun {
    path: PathBuf,
}

fn safe_component(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

fn checked_directory(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(io::Error::other(
            "capture directory is not a plain directory",
        ));
    }
    Ok(())
}

fn fresh_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)
}

impl CaptureRun {
    /// Explicit action only. Existing run IDs (including symlinks) are rejected.
    pub fn create(run_id: &str) -> io::Result<Self> {
        if !safe_component(run_id) {
            return Err(io::Error::other("invalid capture run ID"));
        }
        checked_directory(Path::new("target"))?;
        let parent = Path::new("target/editor-feasibility");
        match fs::create_dir(parent) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        checked_directory(parent)?;
        let path = parent.join(run_id);
        fs::create_dir(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Only fresh files. Missing readback writes no candidate file; simulated
    /// output remains labeled simulated even when byte-identical. Partial I/O
    /// failures retain partial evidence and are returned, never overwritten.
    pub fn capture(
        &self,
        capture_id: &str,
        draft: &Draft,
        readback: Option<&Readback>,
        context: &CaptureContext,
    ) -> io::Result<PathBuf> {
        if !safe_component(capture_id) {
            return Err(io::Error::other("invalid capture ID"));
        }
        let assessment =
            assess(draft, readback).map_err(|error| io::Error::other(format!("{error:?}")))?;
        if let Some(readback) = readback
            && draft.current() != Some(readback.body.as_str())
        {
            return Err(io::Error::other(
                "readback is not the current observed draft",
            ));
        }
        checked_directory(Path::new("target"))?;
        checked_directory(Path::new("target/editor-feasibility"))?;
        checked_directory(&self.path)?;
        let path = self.path.join(capture_id);
        fs::create_dir(&path)?;
        fresh_file(
            &path.join("original.md"),
            draft.original().full().as_bytes(),
        )?;
        fresh_file(
            &path.join("header.md"),
            draft.original().header().as_bytes(),
        )?;
        if let Some(readback) = readback {
            fresh_file(&path.join("candidate-body.md"), readback.body.as_bytes())?;
            fresh_file(
                &path.join("candidate-full.md"),
                draft.original().with_body(&readback.body).as_bytes(),
            )?;
        }
        let status = draft.status();
        let manifest = CaptureManifest {
            generation: draft.generation(),
            original_bytes: draft.original().full().len(),
            original_blake3: hash(draft.original().full()),
            header_bytes: draft.original().header().len(),
            header_blake3: hash(draft.original().header()),
            candidate_body_bytes: readback.map(|r| r.body.len()),
            candidate_body_blake3: readback.map(|r| hash(&r.body)),
            candidate_full_blake3: readback.map(|r| hash(&draft.original().with_body(&r.body))),
            changed_on_load: status.map(|s| s.changed_on_load),
            user_edits: status.map(|s| s.user_edits),
            dirty: status.map(|s| s.dirty),
            assessment,
            context,
        };
        let manifest = toml::to_string_pretty(&manifest).map_err(io::Error::other)?;
        fresh_file(&path.join("manifest.toml"), manifest.as_bytes())?;
        Ok(path)
    }
}
