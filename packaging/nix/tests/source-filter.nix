{ lib }:
let
  root = "/manyhands-source";
  filter = desktop: import ../source-filter.nix { inherit lib root desktop; };
  cases = [
    { path = "Cargo.toml"; type = "regular"; cli = true; desktop = true; }
    { path = "Cargo.lock"; type = "regular"; cli = true; desktop = true; }
    { path = "src/lib.rs"; type = "regular"; cli = true; desktop = true; }
    { path = "src/bin/manyhands-cli.rs"; type = "regular"; cli = true; desktop = true; }
    { path = "tests/cli.rs"; type = "regular"; cli = true; desktop = true; }
    { path = "src/desktop"; type = "directory"; cli = false; desktop = true; }
    { path = "src/desktop/fonts.rs"; type = "regular"; cli = false; desktop = true; }
    { path = "assets/fonts/IBMPlexSans-Regular.ttf"; type = "regular"; cli = false; desktop = true; }
    { path = "packaging/licenses/about.toml"; type = "regular"; cli = true; desktop = true; }
    { path = "packaging/licenses/README-cli.md"; type = "regular"; cli = true; desktop = true; }
    { path = "packaging/licenses/supplemental.json"; type = "regular"; cli = false; desktop = true; }
    { path = "packaging/licenses/supplemental/taffy-0.13.0-LICENSE"; type = "regular"; cli = false; desktop = true; }
    { path = "packaging/licenses/supplemental/accesskit-LICENSE-MIT"; type = "regular"; cli = false; desktop = true; }
    { path = "packaging/licenses/supplemental"; type = "directory"; cli = false; desktop = true; }
    { path = "packaging/licenses/supplemental/private.txt"; type = "regular"; cli = false; desktop = false; }
    { path = "packaging/licenses/supplemental/private"; type = "directory"; cli = false; desktop = false; }
    { path = "packaging/licenses/supplemental/private/key.pem"; type = "regular"; cli = false; desktop = false; }
    { path = "packaging/licenses/supplemental/.private/key.pem"; type = "regular"; cli = false; desktop = false; }
    { path = "packaging/licenses/supplemental/taffy-0.13.0-LICENSE"; type = "symlink"; cli = false; desktop = false; }
    { path = "scripts/release/notices.py"; type = "regular"; cli = true; desktop = true; }
    { path = "packaging"; type = "directory"; cli = true; desktop = true; }
    { path = "packaging/nix"; type = "directory"; cli = true; desktop = true; }
    { path = "packaging/nix/README.md"; type = "regular"; cli = true; desktop = true; }
    { path = "packaging/nix/README.md"; type = "symlink"; cli = false; desktop = false; }
    { path = "packaging/nix/package.nix"; type = "regular"; cli = false; desktop = false; }
    { path = "packaging/nix/private.pfx"; type = "regular"; cli = false; desktop = false; }
    { path = "nix"; type = "directory"; cli = false; desktop = false; }
    { path = "nix/README.md"; type = "regular"; cli = false; desktop = false; }
    { path = "packaging/icons/manyhands-1024.png"; type = "regular"; cli = false; desktop = true; }
    { path = "packaging/linux/manyhands.desktop"; type = "regular"; cli = false; desktop = true; }
    { path = ".manyhands"; type = "directory"; cli = false; desktop = false; }
    { path = ".manyhands/ticket.md"; type = "regular"; cli = false; desktop = false; }
    { path = ".worktrees/other/src/lib.rs"; type = "regular"; cli = false; desktop = false; }
    { path = ".git"; type = "regular"; cli = false; desktop = false; }
    { path = ".devenv/state"; type = "regular"; cli = false; desktop = false; }
    { path = ".direnv"; type = "directory"; cli = false; desktop = false; }
    { path = "target/release/manyhands"; type = "regular"; cli = false; desktop = false; }
    { path = "result"; type = "symlink"; cli = false; desktop = false; }
    { path = "src/lib.rs"; type = "symlink"; cli = false; desktop = false; }
    { path = "src/.private/key.rs"; type = "regular"; cli = false; desktop = false; }
    { path = "packaging/windows/signing.json"; type = "regular"; cli = false; desktop = false; }
    { path = "release-dist/archive.zip"; type = "regular"; cli = false; desktop = false; }
    { path = "private.pfx"; type = "regular"; cli = false; desktop = false; }
    { path = "PHASE-II-PACKAGING-HANDOFF.md"; type = "regular"; cli = false; desktop = false; }
  ];
  valid = lib.all (case:
    filter false "${root}/${case.path}" case.type == case.cli
    && filter true "${root}/${case.path}" case.type == case.desktop
  ) cases;
  supplemental = builtins.fromJSON (builtins.readFile ../../licenses/supplemental.json);
  listedFiles = map (notice: notice.file)
    (lib.filter (notice: notice ? file) supplemental.notices);
  allListedFiles = lib.all (file:
    filter true "${root}/packaging/licenses/supplemental/${file}" "regular"
    && !(filter false "${root}/packaging/licenses/supplemental/${file}" "regular")
    && !(filter true "${root}/packaging/licenses/supplemental/${file}" "symlink")
  ) listedFiles;
in
assert valid && allListedFiles;
true
