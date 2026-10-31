# Manyhands $VERSION

- Source tag: `$TAG`
- Source commit: `$COMMIT`
- Target: `$TARGET`

Manyhands is a local-first, Git-backed project management and documentation
tool. This package contains the desktop application and the headless CLI.

## Installation and startup

$INSTALLATION

Keep the accompanying files together when extracting this package. The
desktop and CLI share the same application data by default. Install Git
and make it available on PATH for operations that invoke Git helpers.

## Runtime and signing

$RUNTIME

$SIGNING

`release.json` records this package's version, source revision, target,
compiler, runtime requirements, and signing mode.

## Verify the download

Compare the archive's SHA-256 hash with `SHA256SUMS` attached to the same
GitHub Release. Checksums identify the release bytes; they do not turn an
ad-hoc or self-signed application into a trusted-publisher application.

## Licences and source

Manyhands is licensed under the MIT licence in `LICENSE`.
`THIRD-PARTY-NOTICES.html` contains dependency and font terms/attributions.
`licenses/` preserves additional original notices and checksum-verified
source archives for source-covered dependencies; see its `README.md`.

IBM Plex Sans has its own SIL Open Font License 1.1, separate from
Manyhands' MIT licence. Its provenance README and complete licence are
under `licenses/fonts/`. Linux embeds the fonts; no system font
installation is required. Other platforms use native system UI fonts.

On macOS the app bundle also contains these documents under
`Manyhands.app/Contents/Resources/`, so moving the app to Applications
retains the disclosures.
