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
  cli-smoke = pkgs.runCommand "manyhands-cli-smoke" {
    nativeBuildInputs = [ pkgs.python3 ];
  } ''
    export HOME="$TMPDIR/home" XDG_CONFIG_HOME="$TMPDIR/config" XDG_DATA_HOME="$TMPDIR/data" XDG_CACHE_HOME="$TMPDIR/cache"
    mkdir -p "$HOME"
    env -u LD_LIBRARY_PATH PATH= ${cli}/bin/manyhands-cli --help > "$out"
    test -s ${cli}/share/doc/manyhands/LICENSE
    test -s ${cli}/share/doc/manyhands/THIRD-PARTY-NOTICES.html
    test ! -e ${cli}/share/doc/manyhands/licenses/fonts
    test ! -e ${cli}/bin/.manyhands-cli-wrapped
    python3 - ${cli}/bin/manyhands-cli "$TMPDIR" <<'PY'
    import json, os, pathlib, subprocess, sys
    executable, temporary = sys.argv[1:]
    root = pathlib.Path(temporary) / 'repository'
    request = pathlib.Path(temporary) / 'create.json'
    request.write_text(json.dumps({
        'primary_branch': 'main',
        'identity': {'name': 'Nix smoke test', 'email': 'nix-smoke@example.invalid'},
        'ticket_slug_prefix': 'mh',
    }))
    env = {name: value for name, value in os.environ.items()
           if not name.startswith(('GIT_', 'LD_', 'DYLD_'))}
    env.update(PATH="", GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null')
    command = [executable, 'repo', 'create', '--repo', str(root), '--input', str(request), '--json']
    def run(arguments):
        return json.loads(subprocess.run(arguments, env=env, check=True, capture_output=True, text=True).stdout)
    prepared = run(command + ['--prepare'])
    assert prepared['code'] == 'ok', prepared
    created = run(command + ['--request-id', '01M4CC0VMTTNKVYS6XWQG4CK1N',
                             '--confirm', prepared['data']['confirmation_id']])
    assert created['code'] == 'ok', created
    assert (root / '.git/HEAD').read_text().strip() == 'ref: refs/heads/main'
    assert len((root / '.git/refs/heads/main').read_text().strip()) == 40
    PY
  '';
  cli-closure = pkgs.runCommand "manyhands-cli-no-graphics" { } ''
    if grep -Ei -- '-(git(-minimal)?|gpui|zorite|wayland|vulkan|gtk[+-]?|fontconfig|freetype|libglvnd|libxcb|libxkbcommon)[-0-9]' ${closure}/store-paths; then
      echo 'CLI runtime closure unexpectedly contains Git or desktop/graphics packages' >&2
      exit 1
    fi
    touch "$out"
  '';
  desktop-disclosures = pkgs.runCommand "manyhands-desktop-disclosures" {
    nativeBuildInputs = [ pkgs.python3 ];
  } ''
    test -s ${full}/share/doc/manyhands/licenses/fonts/LICENSE.txt
    test -s ${full}/share/doc/manyhands/licenses/fonts/README.md
    python3 ${./check_disclosures.py} --licenses ${full}/share/doc/manyhands/licenses \
      --locked-sources ${full.lockedSourceManifest} --desktop
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
