{
  description = "Manyhands source-built Linux desktop and headless CLI";

  # This revision provides the release toolchain, Rust/Cargo 1.98.1.
  # Keep independent of devenv.lock: consumer packages do not replace Devenv.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/c7def046b9a883d46974757852106483d741586f";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forSystems = nixpkgs.lib.genAttrs systems;
      packagesFor = system:
        let
          pkgs = import nixpkgs { inherit system; };
          noticesTool = pkgs.callPackage ./packaging/nix/cargo-about.nix { };
          cli = pkgs.callPackage ./packaging/nix/package.nix {
            inherit noticesTool;
            root = self.outPath;
            desktop = false;
          };
          desktop = pkgs.callPackage ./packaging/nix/package.nix {
            inherit noticesTool;
            root = self.outPath;
            desktop = true;
          };
        in {
          default = desktop;
          manyhands = desktop;
          manyhands-cli = cli;
        };
    in {
      packages = forSystems packagesFor;

      apps = forSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/manyhands";
          meta.description = "Launch the Manyhands desktop";
        };
        manyhands-cli = {
          type = "app";
          program = "${self.packages.${system}.manyhands-cli}/bin/manyhands-cli";
          meta.description = "Run the headless Manyhands CLI";
        };
      });

      checks = forSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          packages = self.packages.${system};
        in import ./packaging/nix/checks.nix { inherit pkgs packages; });
    };
}
