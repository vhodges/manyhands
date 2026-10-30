//! The bundled IBM Plex Sans faces, registered on Linux before the Kit
//! starts (SIL Open Font License, `assets/fonts/LICENSE.txt`).
//!
//! On Linux GPUI draws the system UI font as the family "IBM Plex Sans",
//! which Zed bundles and most machines lack. Without it GPUI falls back to
//! an installed family, and a variable font there has one face for every
//! weight, which GPUI draws at its normal weight: bold, semibold and
//! medium text all come out regular, and the editor's Bold shows nothing.
//! The static faces give each weight its own face. They are added before
//! `gpui_kit::init`, where the Kit names the family the system UI font
//! resolves to. The Zorite app does the same (`register_bundled_fonts`).
//! macOS and Windows draw a real system font.

use std::borrow::Cow;

use gpui_kit::App;

/// IBM Plex Sans 1.1.0 (`@ibm/plex-sans@1.1.0`, `fonts/complete/ttf`):
/// the weights the desktop draws, regular, medium, semibold and bold, and
/// the italics the editor's marks need.
const FACES: [&[u8]; 6] = [
    include_bytes!("../../assets/fonts/IBMPlexSans-Regular.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexSans-Italic.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexSans-Medium.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexSans-Bold.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexSans-BoldItalic.ttf"),
];

/// Adds the faces to GPUI's text system. Call before `gpui_kit::init`.
pub(crate) fn register(cx: &App) {
    // The faces are fixed bytes. Were they to fail, text keeps GPUI's
    // fallback family, as it did before they were bundled.
    let _ = cx
        .text_system()
        .add_fonts(FACES.map(Cow::Borrowed).to_vec());
}
