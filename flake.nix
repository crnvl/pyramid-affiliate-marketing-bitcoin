{
  description = "pyramid-affiliate-marketing-bitcoin";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    fenix = {
      url = "github:nix-community/fenix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      # self,
      nixpkgs,
      fenix,
      ...
    }:
    let
      mkPkgs =
        system:
        import nixpkgs {
          inherit system;
        };
      forAllSystems =
        function:
        nixpkgs.lib.genAttrs [
          "x86_64-linux"
          "x86_64-darwin"
          "aarch64-linux"
          "aarch64-darwin"
        ] (system: function system);
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
        in
        rec {
          c3pixelflut = pkgs.rustPlatform.buildRustPackage {
            inherit ((fromTOML (builtins.readFile ./Cargo.toml)).package) name;
            inherit ((fromTOML (builtins.readFile ./Cargo.toml)).package) version;

            src = pkgs.lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;
            doCheck = true;

            meta.mainProgram = ((fromTOML (builtins.readFile ./Cargo.toml)).package).name;
          };

          default = c3pixelflut;
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          f =
            with fenix.packages.${system};
            combine [
              latest.toolchain
            ];
        in
        {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              f
              cargo-watch
              openssl
              pkg-config
              ffmpeg
            ];

            RUST_BACKTRACE = 1;
          };
        }
      );
    };
}
