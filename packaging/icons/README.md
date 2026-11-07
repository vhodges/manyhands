# Application artwork

`manyhands-1024.png` is a generated 1024 × 1024 raster of the project's
`brand/manyhands-icon.svg`. Regenerate it from that vector source rather
than editing the PNG. Native macOS packaging uses `sips` and `iconutil` to
create the standard `.icns` sizes; the Windows installer will derive an
`.ico` from the same source artwork.

Generated with resvg **0.48.1**:

```sh
nix shell nixpkgs#resvg -c resvg --width 1024 --height 1024 brand/manyhands-icon.svg packaging/icons/manyhands-1024.png
```
