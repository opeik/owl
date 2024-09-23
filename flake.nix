# Nix flake, see: https://nixos.org/manual/nix/stable/command-ref/new-cli/nix3-flake
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
        with pkgs; {
          # `nix fmt`
          formatter = alejandra;
          # `nix develop`
          devShell = let
            tools = {
              owl = [
                self.formatter.${system}
                nil
                cargo-expand
                cargo-udeps
                (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              ];

              libcec =
                [ninja cmake clang_16]
                ++ (lib.optional stdenv.isLinux [openssl.dev])
                ++ (lib.optional stdenv.isDarwin (with darwin.apple_sdk.frameworks; [
                  SystemConfiguration
                  CoreFoundation
                  IOKit
                  CoreVideo
                ]));

              ci = [dasel];
            };
          in
            mkShell {
              LIBCLANG_PATH = "${llvmPackages_16.libclang.lib}/lib";
              packages = tools.owl ++ tools.libcec ++ tools.ci;
            };
        }
    );
}
