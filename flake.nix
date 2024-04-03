{
  description = "Lutgen";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs.lib) optionals;

        craneLib = crane.lib.${system};

        src = craneLib.path ./.;
        commonArgs = {
          inherit src;
          strictDeps = true;
          buildInputs = [ ] ++ optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        lutgen =
          craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
      in {
        checks = {
          fmt = craneLib.cargoFmt (commonArgs // { inherit cargoArtifacts; });
          doc = craneLib.cargoDoc (commonArgs // { inherit cargoArtifacts; });
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs =
              "--all-targets --all-features -- -Dclippy::all -Dwarnings";
          });
          nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            cargoNextestExtraArgs = "--all-targets --all-features --all";
          });
          doctest = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
            pnameSuffix = "-doctest";
            cargoTextArgs = "--doc";
          });
        };
        packages = { default = lutgen; };
        apps.default = flake-utils.lib.mkApp { drv = lutgen; };
        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};
        };
      });
}
