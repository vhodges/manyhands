{ lib, root, desktop }:
path: type:
let
  relative = lib.removePrefix "${toString root}/" (toString path);
  parts = lib.splitString "/" relative;
  hidden = lib.any (part: lib.hasPrefix "." part) parts;
  code = lib.any (prefix: relative == prefix || lib.hasPrefix "${prefix}/" relative) [ "src" "tests" "examples" ];
  desktopCode = relative == "src/desktop" || lib.hasPrefix "src/desktop/" relative;
  # Read beside this filter rather than under root: policy tests deliberately
  # use a nonexistent root. Only explicitly listed, safe basenames enter src.
  supplemental = builtins.fromJSON (builtins.readFile ../packaging/licenses/supplemental.json);
  supplementalNames = map (notice: notice.file)
    (lib.filter (notice: notice ? file) supplemental.notices);
  safeSupplementalName = name: builtins.isString name
    && builtins.match "[A-Za-z0-9][A-Za-z0-9._-]*" name != null;
  supplementalFiles = assert lib.all safeSupplementalName supplementalNames;
    map (name: "packaging/licenses/supplemental/${name}") (lib.unique supplementalNames);
  files = [
    "Cargo.toml" "Cargo.lock" "LICENSE" "README.md"
    "scripts/release/common.py" "scripts/release/notices.py"
    "scripts/check-cli-no-gpui.sh"
    "packaging/tool-versions.env"
    "packaging/licenses/about.toml" "packaging/licenses/about.hbs"
    "packaging/licenses/README.md" "packaging/licenses/README-cli.md"
    "nix/README.md"
  ] ++ lib.optionals desktop [
    "packaging/licenses/supplemental.json"
    "packaging/icons/manyhands-1024.png" "packaging/linux/manyhands.desktop"
    "packaging/linux/dynamic-libraries.txt"
  ] ++ lib.optionals desktop supplementalFiles;
  font = desktop && (relative == "assets/fonts" || lib.hasPrefix "assets/fonts/" relative);
  ancestor = lib.any (file: lib.hasPrefix "${relative}/" file) files
    || (desktop && (relative == "assets"));
in
type != "symlink" && !hidden && (
  (toString path == toString root)
  || (code && (!desktopCode || desktop) && (type == "directory" || lib.hasSuffix ".rs" relative))
  || font
  || builtins.elem relative files
  || (type == "directory" && ancestor)
)
