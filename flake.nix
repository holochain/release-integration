{
  description = "Development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { system, pkgs, ... }:
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              (import inputs.rust-overlay)
            ];
            config = { };
          };

          formatter = pkgs.nixpkgs-fmt;

          devShells = {
            default = pkgs.mkShell {
              packages = with pkgs; [
                (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
                git
                git-cliff
                cargo-semver-checks
                httpie
                gitea
                initool
              ];
            };
          };
        };
    };
}
