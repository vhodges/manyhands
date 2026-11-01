{ lib, stdenv, rustPlatform, rustc, cargo, fetchurl, linkFarm, makeWrapper
, pkg-config, python3, openssl, zlib, gitMinimal, clang
, wayland, vulkan-loader, vulkan-headers, libxcb, libxkbcommon, libglvnd
, atk, fontconfig, freetype, glib, gtk3, alsa-lib
, root, noticesTool, desktop ? true
}:
let
  manifest = builtins.fromTOML (builtins.readFile (root + "/Cargo.toml"));
  lock = builtins.fromTOML (builtins.readFile (root + "/Cargo.lock"));
  versions = lib.listToAttrs (map (line:
    let parts = lib.splitString "=" line;
    in { name = builtins.head parts; value = lib.concatStringsSep "=" (builtins.tail parts); }
  ) (lib.filter (line: line != "" && !(lib.hasPrefix "#" line))
    (lib.splitString "\n" (builtins.readFile (root + "/packaging/tool-versions.env")))));
  source = lib.cleanSourceWith {
    name = "manyhands-${if desktop then "desktop" else "cli"}-source";
    src = root;
    filter = import ./source-filter.nix { inherit lib root desktop; };
  };
  target = stdenv.hostPlatform.rust.rustcTarget;
  # New source-covered normal dependencies fail notice generation until this
  # reviewed set is updated. Never substitute a developer's Cargo cache.
  sourcePackages = lib.filter (package: builtins.elem package.name (
    [ "option-ext" "libgit2-sys" ] ++ lib.optional desktop "wayland-protocols-plasma"
  )) lock.package;
  sourceArchives = linkFarm "manyhands-original-crate-sources" (map (package: {
    name = "${package.name}-${package.version}.crate";
    path = fetchurl {
      url = "https://static.crates.io/crates/${package.name}/${package.name}-${package.version}.crate";
      sha256 = package.checksum;
    };
  }) sourcePackages);
  runtimeLibraries = [
    wayland vulkan-loader libxcb libxkbcommon libglvnd
    atk fontconfig freetype glib gtk3 openssl
  ];
  runtimePath = lib.makeLibraryPath runtimeLibraries;
  doc = "$out/share/doc/manyhands";
in
assert lib.assertMsg (rustc.version == versions.RUST_VERSION && cargo.version == versions.RUST_VERSION)
  "Manyhands Nix and release Rust/Cargo pins must match";
assert lib.assertMsg (noticesTool.version == versions.CARGO_ABOUT_VERSION)
  "Manyhands Nix and release cargo-about pins must match";
rustPlatform.buildRustPackage {
  pname = if desktop then "manyhands" else "manyhands-cli";
  version = manifest.package.version;
  src = source;
  # importCargoLock symlinks this file into its vendor output. Isolate it so
  # that symlink cannot retain/transfer the unfiltered flake's private state.
  cargoLock.lockFile = builtins.toFile "manyhands-Cargo.lock"
    (builtins.readFile (root + "/Cargo.lock"));
  nativeBuildInputs = [ pkg-config makeWrapper python3 noticesTool ]
    ++ lib.optionals desktop [ clang rustPlatform.bindgenHook ];
  buildInputs = [ openssl zlib ] ++ lib.optionals desktop [
    wayland vulkan-loader vulkan-headers libxcb libxkbcommon
    fontconfig freetype glib gtk3 alsa-lib
  ];
  env = {
    CARGO_NET_OFFLINE = "true";
    OPENSSL_NO_VENDOR = "1";
  };
  # Build the CLI first with no desktop feature; the later desktop build must
  # not replace it with a feature-unified CLI. No probes/test features ship.
  buildPhase = ''
    runHook preBuild
    sh scripts/check-cli-no-gpui.sh
    cargo build --frozen --release --no-default-features --bin manyhands-cli --target ${target} -j "$NIX_BUILD_CORES"
    ${lib.optionalString desktop ''
      cargo build --frozen --release --no-default-features --features desktop --bin manyhands --target ${target} -j "$NIX_BUILD_CORES"
    ''}
    python3 -B scripts/release/notices.py --repo "$PWD" --target ${target} \
      --output "$TMPDIR/manyhands-notices" --offline \
      --source-archives ${sourceArchives} ${lib.optionalString (!desktop) "--cli-only"}
    runHook postBuild
  '';
  # The project suite is a separate Devenv/native gate. Package checks below
  # exercise installed CLI/disclosures, not test-only remote servers or a GUI.
  doCheck = false;
  installPhase = ''
    runHook preInstall
    install -Dm755 target/${target}/release/manyhands-cli "$out/bin/manyhands-cli"
    mkdir -p ${doc}
    install -m644 LICENSE nix/README.md ${doc}/
    cp -r "$TMPDIR/manyhands-notices/THIRD-PARTY-NOTICES.html" "$TMPDIR/manyhands-notices/licenses" ${doc}/
    wrapProgram "$out/bin/manyhands-cli" --prefix PATH : ${lib.makeBinPath [ gitMinimal ]}
    ${lib.optionalString desktop ''
      install -Dm755 target/${target}/release/manyhands "$out/bin/manyhands"
      install -Dm644 packaging/linux/manyhands.desktop "$out/share/applications/manyhands.desktop"
      install -Dm644 packaging/icons/manyhands-1024.png "$out/share/icons/hicolor/1024x1024/apps/manyhands.png"
      wrapProgram "$out/bin/manyhands" --prefix PATH : ${lib.makeBinPath [ gitMinimal ]} \
        --prefix LD_LIBRARY_PATH : ${runtimePath}
    ''}
    runHook postInstall
  '';
  doInstallCheck = true;
  installCheckPhase = ''
    runHook preInstallCheck
    export HOME="$TMPDIR/manyhands-check-home"
    export XDG_CONFIG_HOME="$HOME/config" XDG_DATA_HOME="$HOME/data" XDG_CACHE_HOME="$HOME/cache"
    mkdir -p "$HOME"
    env -u LD_LIBRARY_PATH -u DYLD_LIBRARY_PATH "$out/bin/manyhands-cli" --help > "$TMPDIR/cli-help"
    grep -qi manyhands "$TMPDIR/cli-help"
    test -s ${doc}/LICENSE
    test -s ${doc}/THIRD-PARTY-NOTICES.html
    test -s ${doc}/licenses/inventory.json
    test -s ${doc}/licenses/sources/option-ext-0.2.0.crate
    ${if desktop then ''
      test -s ${doc}/licenses/fonts/LICENSE.txt
      test -s ${doc}/licenses/fonts/README.md
    '' else ''
      test ! -e ${doc}/licenses/fonts
      test ! -e "$out/bin/manyhands"
    ''}
    runHook postInstallCheck
  '';
  passthru = {
    inherit desktop noticesTool;
    runtimeLibraries = lib.optionals desktop runtimeLibraries;
    toolchainVersion = rustc.version;
  };
  meta = {
    description = "Local-first, Git-backed project management${lib.optionalString (!desktop) " (headless CLI)"}";
    homepage = "https://github.com/vhodges/manyhands";
    license = lib.licenses.mit;
    platforms = [ "x86_64-linux" "aarch64-linux" ];
    mainProgram = if desktop then "manyhands" else "manyhands-cli";
  };
}
