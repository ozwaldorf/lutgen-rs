{
  description = "Lutgen";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      fenix,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs.lib) optionals;

        stableCraneLib = crane.mkLib pkgs;
        craneLib = stableCraneLib.overrideToolchain fenix.packages.${system}.complete.toolchain;

        src = craneLib.path ./.;
        commonArgs = {
          inherit src;
          strictDeps = true;
          buildInputs = [ ] ++ optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
          nativeBuildInputs = [ pkgs.installShellFiles ];
        };

        lutgen = stableCraneLib.buildPackage (
          commonArgs
          // {
            doCheck = false;
            postInstall = pkgs.lib.optionalString (pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform) ''
              installManPage docs/lutgen.1
              installShellCompletion --cmd lutgen \
                --bash <($out/bin/lutgen --bpaf-complete-style-bash) \
                --fish <($out/bin/lutgen --bpaf-complete-style-fish) \
                --zsh <($out/bin/lutgen --bpaf-complete-style-zsh)
            '';
          }
        );
      in
      {
        checks =
          let
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          in
          {
            fmt = craneLib.cargoFmt (commonArgs // { inherit cargoArtifacts; });
            doc = craneLib.cargoDoc (
              commonArgs
              // {
                inherit cargoArtifacts;
                RUSTFLAGS = "-Dwarnings";
              }
            );
            clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets --all-features -- -Dclippy::all -Dwarnings";
              }
            );
            nextest = craneLib.cargoNextest (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoNextestExtraArgs = "--all-targets --all-features --all";
              }
            );
            doctest = craneLib.cargoTest (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoTextExtraArgs = "--doc";
              }
            );
          };
        packages = {
          inherit lutgen;
          default = lutgen;
        };
        apps.default = flake-utils.lib.mkApp { drv = lutgen; };
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            rust-analyzer
            jekyll
            bundler
          ];
        };
        formatter = pkgs.nixfmt-rfc-style;
      }
    )
    // {
      overlays.default = _: prev: { lutgen = self.packages.${prev.system}.default; };
    };
}
