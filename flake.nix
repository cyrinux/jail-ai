{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    let systems = [ "x86_64-linux" "aarch64-linux" ];
    in flake-utils.lib.eachSystem systems (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
        libs = with pkgs; [
          pkg-config
        ];
      in
      {
        defaultPackage = naersk-lib.buildPackage {
          src = ./.;
          meta.mainProgram = "jail-ai";
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = libs;
        };
        devShell = with pkgs; mkShell {
          buildInputs = [
            # Build tools
            cargo
            cargo-bloat
            cargo-bump
            cargo-deny
            cargo-feature
            clippy
            gcc
            glib
            glib.dev
            gnumake
            pkg-config
            rust-analyzer
            rustc
            # Rust development
            rustfmt
            rustPackages.clippy
          ] ++ libs;
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      }
    );
}

