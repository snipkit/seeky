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
        seeky = import ./seeky {
          inherit pkgs monorepo-deps;
        };
        seeky-rs = import ./seeky-rs {
          pkgs = pkgsWithRust;
          inherit monorepo-deps;
        };
      in
      rec {
        packages = {
          seeky = seeky.package;
          seeky-rs = seeky-rs.package;
        };

        devShells = {
          seeky = seeky.devShell;
          seeky-rs = seeky-rs.devShell;
        };

        apps = {
          seeky = seeky.app;
          seeky-rs = seeky-rs.app;
        };

        defaultPackage = packages.seeky;
        defaultApp = apps.seeky;
        defaultDevShell = devShells.seeky;
      }
    );
}
