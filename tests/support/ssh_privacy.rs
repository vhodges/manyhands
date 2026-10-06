//! Test-only raw byte scanner; failures never render the detected material.
#![allow(dead_code)]
use super::ssh_remote::{FixtureError, fixed};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
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
    let root_meta = fixed(std::fs::symlink_metadata(root))?;
    if !root_meta.is_dir() || root_meta.file_type().is_symlink() {
        return Err(FixtureError);
    }
    let probes: Vec<_> = fixed(std::fs::read_dir(root))?
        .map(|entry| {
            let path = fixed(entry)?.path();
            let meta = fixed(std::fs::symlink_metadata(&path))?;
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err(FixtureError);
            }
            let bytes = fixed(std::fs::read(&path))?;
            if path.file_name().and_then(|name| name.to_str())
                != Some(blake3::hash(&bytes).to_hex().as_str())
            {
                return Err(FixtureError);
            }
            Ok(bytes)
        })
        .collect::<Result<_, _>>()?;
    clean(b"", &probes)?;
    Ok(probes)
}
pub fn save(probes: &[Vec<u8>]) -> Result<(), FixtureError> {
    clean(b"", probes)?;
    let root = PathBuf::from(std::env::var_os(PROBES).ok_or(FixtureError)?);
    let root_meta = fixed(std::fs::symlink_metadata(&root))?;
    if !root_meta.is_dir() || root_meta.file_type().is_symlink() {
        return Err(FixtureError);
    }
    // Case-wide union: a later World/snapshot can neither replace nor truncate
    // earlier material. Names are digests, never raw secret/URL bytes.
    for probe in probes {
        let path = root.join(blake3::hash(probe).to_hex().as_str());
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => fixed(file.write_all(probe))?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let meta = fixed(std::fs::symlink_metadata(&path))?;
                if !meta.is_file()
                    || meta.file_type().is_symlink()
                    || fixed(std::fs::read(&path))? != *probe
                {
                    return Err(FixtureError);
                }
            }
            Err(_) => return Err(FixtureError),
        }
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
