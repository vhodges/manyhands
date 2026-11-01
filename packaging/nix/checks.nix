{ pkgs, packages }:
let
  inherit (pkgs) lib;
  cli = packages.manyhands-cli;
  full = packages.default;
  closure = pkgs.closureInfo { rootPaths = [ cli ]; };
  sourceInputs = pkgs.closureInfo {
    rootPaths = [ cli.src full.src cli.cargoDeps full.cargoDeps ];
  };
  sourcePolicy = import ./tests/source-filter.nix { inherit lib; };
in {
  source-policy = assert sourcePolicy; pkgs.runCommand "manyhands-source-policy" { } ''
    test ! -e ${cli.src}/.manyhands
    test ! -e ${cli.src}/.worktrees
    test ! -e ${cli.src}/target
    test ! -e ${cli.src}/assets/fonts
    test ! -e ${cli.src}/src/desktop
    # Vendor symlinks must not smuggle the unfiltered flake snapshot back
    # into the transitive build inputs after application source filtering.
    while IFS= read -r path; do
      for excluded in .manyhands .worktrees target; do
        if test -e "$path/$excluded"; then
          echo "Excluded development state in build input: $path/$excluded" >&2
          exit 1
        fi
      done
    done < ${sourceInputs}/store-paths
    touch "$out"
  '';
  cli-smoke = pkgs.runCommand "manyhands-cli-smoke" { } ''
    export HOME="$TMPDIR/home" XDG_CONFIG_HOME="$TMPDIR/config" XDG_DATA_HOME="$TMPDIR/data" XDG_CACHE_HOME="$TMPDIR/cache"
    mkdir -p "$HOME"
    env -u LD_LIBRARY_PATH ${cli}/bin/manyhands-cli --help > "$out"
    test -s ${cli}/share/doc/manyhands/LICENSE
    test -s ${cli}/share/doc/manyhands/THIRD-PARTY-NOTICES.html
    test ! -e ${cli}/share/doc/manyhands/licenses/fonts
  '';
  cli-closure = pkgs.runCommand "manyhands-cli-no-graphics" { } ''
    if grep -Ei -- '-(gpui|zorite|wayland|vulkan|gtk[+-]?|fontconfig|freetype|libglvnd|libxcb|libxkbcommon)[-0-9]' ${closure}/store-paths; then
      echo 'CLI runtime closure unexpectedly contains desktop/graphics packages' >&2
      exit 1
    fi
    touch "$out"
  '';
  desktop-disclosures = pkgs.runCommand "manyhands-desktop-disclosures" { } ''
    test -s ${full}/share/doc/manyhands/licenses/fonts/LICENSE.txt
    test -s ${full}/share/doc/manyhands/licenses/fonts/README.md
    test -s ${full}/share/doc/manyhands/licenses/sources/wayland-protocols-plasma-0.3.12.crate
    test -s ${full}/share/doc/manyhands/licenses/dependencies/wayland-protocols-plasma-0.3.12/plasma-wayland-protocols/src/protocols/blur.xml
    test -s ${full}/share/doc/manyhands/licenses/dependencies/wayland-protocols-plasma-0.3.12/plasma-wayland-protocols/COPYING.LIB
    test -s ${full}/share/applications/manyhands.desktop
    test -s ${full}/share/icons/hicolor/1024x1024/apps/manyhands.png
    touch "$out"
  '';
  desktop-loaders = pkgs.runCommand "manyhands-desktop-runtime-loaders" {
    nativeBuildInputs = [ pkgs.python3 ];
    LD_LIBRARY_PATH = lib.makeLibraryPath full.runtimeLibraries;
  } ''
    python3 - <<'PY'
    import ctypes
    for line in ${builtins.toJSON (lib.splitString "\n" (builtins.readFile ../linux/dynamic-libraries.txt))}:
        if line and not line.startswith('#'):
            ctypes.CDLL(line)
    PY
    touch "$out"
  '';
}
