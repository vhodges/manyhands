//! Win32 pathname policy shared by the Windows seam and host-side tests.

use std::{ffi::OsStr, io};

pub(super) fn validate_component(name: &OsStr) -> io::Result<()> {
    let name = name.to_str().ok_or(io::ErrorKind::InvalidInput)?;
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ')
        .to_uppercase();
    let reserved = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if name.is_empty()
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || reserved
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_git_and_document_components_are_accepted() {
        for name in [
            ".git",
            "docs",
            "document.md",
            "index.lock",
            "ref-log-baseline-anchor",
            "日本語.md",
        ] {
            validate_component(OsStr::new(name)).unwrap();
        }
    }

    #[test]
    fn aliases_streams_and_device_components_are_refused() {
        for name in [
            "",
            ".",
            "..",
            "index:stream",
            "NUL",
            "con.txt",
            "COM1",
            "lpt9.md",
            "COM¹",
            "tail.",
            "tail ",
            "a/b",
            "a\\b",
            "a\0b",
            "a?b",
            "a*b",
            "a\"b",
            "a<b",
            "a>b",
            "a|b",
            "a\nb",
        ] {
            assert!(validate_component(OsStr::new(name)).is_err(), "{name:?}");
        }
    }
}
