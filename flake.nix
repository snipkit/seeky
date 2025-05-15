{
  description = "Development Nix flake for KhulnaSoft Seeky CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }: 
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        pkgsWithRust = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        monorepo-deps = with pkgs; [
          # for precommit hook
          pnpm
          husky
        ];
        seeky-cli = import ./seeky-cli {
          inherit pkgs monorepo-deps;
        };
        seeky-rs = import ./seeky-rs {
          pkgs = pkgsWithRust;
          inherit monorepo-deps;
        };
      in
      rec {
        packages = {
          seeky-cli = seeky-cli.package;
          seeky-rs = seeky-rs.package;
        };

        devShells = {
          seeky-cli = seeky-cli.devShell;
          seeky-rs = seeky-rs.devShell;
        };

        apps = {
          seeky-cli = seeky-cli.app;
          seeky-rs = seeky-rs.app;
        };

        defaultPackage = packages.seeky-cli;
        defaultApp = apps.seeky-cli;
        defaultDevShell = devShells.seeky-cli;
      }
    );
}
