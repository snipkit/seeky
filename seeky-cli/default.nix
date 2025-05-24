{ pkgs, monorepo-deps }:

{
  package = pkgs.stdenv.mkDerivation {
    name = "seeky-cli";
    src = ./.;
    buildInputs = [];
    installPhase = "mkdir -p $out/bin; echo '#!/bin/sh\necho Seeky CLI' > $out/bin/seeky-cli; chmod +x $out/bin/seeky-cli";
  };

  devShell = pkgs.mkShell {
    packages = monorepo-deps;
  };

  app = {
    type = "app";
    program = "${pkgs.writeShellScript "seeky-cli" "echo Seeky CLI"}";
  };
}
