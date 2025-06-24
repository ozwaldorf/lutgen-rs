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

        lutgen = stableCraneLib.buildPackage {
          inherit src;
          doCheck = false;
          pname = "lutgen";

          strictDeps = true;
          buildInputs = [ ] ++ optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
          nativeBuildInputs = [ pkgs.installShellFiles ];
          cargoExtraArgs = "--locked --bin lutgen";
          postInstall = pkgs.lib.optionalString (pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform) ''
            installManPage docs/man/lutgen.1
            installShellCompletion --cmd lutgen \
              --bash <($out/bin/lutgen --bpaf-complete-style-bash) \
              --fish <($out/bin/lutgen --bpaf-complete-style-fish) \
              --zsh <($out/bin/lutgen --bpaf-complete-style-zsh)
          '';
        };

        commonArgs = with pkgs; rec {
          inherit src;
          strictDeps = true;
          nativeBuildInputs = [
            pkg-config
          ];
          buildInputs = (
            [
              openssl
              libxkbcommon
              libGL
              fontconfig
              wayland
              xorg.libXcursor
              xorg.libXrandr
              xorg.libXi
              xorg.libX11
              zenity # file dialog
            ]
            ++ optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ]
          );
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
        };

        lutgen-studio = stableCraneLib.buildPackage (
          commonArgs
          // {
            doCheck = false;
            pname = "lutgen-studio";
            cargoExtraArgs = "--locked --bin lutgen-studio";
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
          inherit lutgen lutgen-studio;
          default = lutgen;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            rust-analyzer
            jekyll
            bundler
          ];
          inherit (commonArgs) LD_LIBRARY_PATH;
        };
        formatter = pkgs.nixfmt-rfc-style;
      }
    )
    // {
      overlays.default = _: prev: { lutgen = self.packages.${prev.system}.default; };
    };
}
