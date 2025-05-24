{ pkgs, monorepo-deps }:

let
  rust = pkgs.rustc;
  cargo = pkgs.cargo;
in
{
  package = pkgs.stdenv.mkDerivation {
    pname = "seeky-rs";
    version = "0.1.0";
    src = ./.;

    buildInputs = [ rust cargo ] ++ monorepo-deps;

    buildPhase = ''
      cargo build --release
    '';

    installPhase = ''
      mkdir -p $out/bin
      cp target/release/seeky-rs $out/bin/
    '';

    # Optional for reproducibility
    CARGO_HOME = "${pkgs.cargo}";
    RUST_BACKTRACE = 1;
  };

  devShell = pkgs.mkShell {
    buildInputs = [ rust cargo ] ++ monorepo-deps;
    shellHook = ''
      echo "Welcome to seeky-rs dev shell"
    '';
  };

  app = {
    type = "app";
    program = "${pkgs.writeShellScript "seeky-rs-wrapper" ''
      exec ${pkgs.stdenv.mkDerivation { ... }}/bin/seeky-rs
    ''}";
  };
}
