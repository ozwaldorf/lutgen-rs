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
        inherit (builtins) fromTOML readFile;

        cLib = crane.mkLib pkgs;
        stableCraneLib = cLib.overrideToolchain fenix.packages.${system}.complete.toolchain;
        craneLib = cLib.overrideToolchain (
          fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "UAoZcxg3iWtS+2n8TFNfANFt/GmkuOMDf7QAE0fRxeA=";
          }
        );

        # source and package versions
        src = craneLib.path ./.;
        version = (fromTOML (readFile ./crates/cli/Cargo.toml)).package.version;
        studioVersion = (fromTOML (readFile ./crates/studio/Cargo.toml)).package.version;

        # Common args
        commonArgs = {
          inherit src version;
          strictDeps = true;
          pname = "lutgen";
          buildInputs = [ ] ++ optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
        };

        lutgen = stableCraneLib.buildPackage (
          commonArgs
          // {
            doCheck = false;
            strictDeps = true;
            nativeBuildInputs = [ pkgs.installShellFiles ];
            cargoExtraArgs = "--locked --bin lutgen";
            postInstall = pkgs.lib.optionalString (pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform) ''
              installManPage docs/man/lutgen.1
              installShellCompletion --cmd lutgen \
                --bash <($out/bin/lutgen --bpaf-complete-style-bash) \
                --fish <($out/bin/lutgen --bpaf-complete-style-fish) \
                --zsh <($out/bin/lutgen --bpaf-complete-style-zsh)
            '';
          }
        );

        # runtime libraries for studio
        LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath (
          with pkgs;
          [
            libxkbcommon
            libGL
            fontconfig
            wayland
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libX11
          ]
        )}";

        lutgen-studio = stableCraneLib.buildPackage (
          commonArgs
          // rec {
            doCheck = false;
            pname = "lutgen-studio";
            version = studioVersion;
            nativeBuildInputs = [ pkgs.makeWrapper ];
            cargoExtraArgs = "--locked --bin lutgen-studio";
            postInstall = ''
              wrapProgram "$out/bin/${pname}" \
                --set LD_LIBRARY_PATH "${LD_LIBRARY_PATH}"
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
          inherit lutgen lutgen-studio;
          default = lutgen;
        };

        devShells.default = craneLib.devShell {
          inherit LD_LIBRARY_PATH;
          checks = self.checks.${system};
          packages = with pkgs; [
            jekyll
            bundler
            trunk
          ];

        };
        formatter = pkgs.nixfmt-rfc-style;
      }
    )
    // {
      overlays.default = _: prev: {
        lutgen = self.packages.${prev.system}.default;
        lutgen-studio = self.packages.${prev.system}.default;
      };
    };
}
