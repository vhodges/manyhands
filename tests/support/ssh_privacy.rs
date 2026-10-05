//! Test-only raw byte scanner; failures never render the detected material.
#![allow(dead_code)]
use super::ssh_remote::{FixtureError, fixed};
use std::path::{Path, PathBuf};
pub const PROBES: &str = "MANYHANDS_SSH_PRIVACY_PROBES";
pub fn clean(bytes: &[u8], probes: &[Vec<u8>]) -> Result<(), FixtureError> {
    if probes.is_empty()
        || probes
            .iter()
            .any(|p| p.is_empty() || bytes.windows(p.len()).any(|w| w == p))
    {
        return Err(FixtureError);
    }
    Ok(())
}
pub fn load(root: &Path) -> Result<Vec<Vec<u8>>, FixtureError> {
    let probes: Vec<_> = fixed(std::fs::read_dir(root))?
        .map(|e| fixed(std::fs::read(fixed(e)?.path())))
        .collect::<Result<_, _>>()?;
    clean(b"", &probes)?;
    Ok(probes)
}
pub fn save(probes: &[Vec<u8>]) -> Result<(), FixtureError> {
    clean(b"", probes)?;
    let root = PathBuf::from(std::env::var_os(PROBES).ok_or(FixtureError)?);
    for (i, probe) in probes.iter().enumerate() {
        fixed(std::fs::write(root.join(i.to_string()), probe))?;
    }
    Ok(())
}
pub fn scan(root: &Path, probes: &[Vec<u8>]) -> Result<Vec<(PathBuf, usize)>, FixtureError> {
    let mut scanned = Vec::new();
    for entry in fixed(std::fs::read_dir(root))? {
        let path = fixed(entry)?.path();
        let meta = fixed(std::fs::symlink_metadata(&path))?;
        if meta.file_type().is_symlink() {
            return Err(FixtureError);
        }
        clean(path.to_string_lossy().as_bytes(), probes)?;
        if meta.is_dir() {
            scanned.extend(scan(&path, probes)?);
        } else {
            let bytes = fixed(std::fs::read(&path))?;
            clean(&bytes, probes)?;
            scanned.push((path, bytes.len()));
        }
    }
    Ok(scanned)
}
