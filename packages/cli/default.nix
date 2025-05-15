{ pkgs, monorep-deps ? [], ... }:
let
  node = pkgs.nodejs_22;
in
rec {
  package = pkgs.buildNpmPackage {
    pname       = "seeky-cli";
    version     = "0.1.0";
    src         = ./.;
    npmDepsHash = "sha256-3tAalmh50I0fhhd7XreM+jvl0n4zcRhqygFNB1Olst8";
    nodejs      = node;
    npmInstallFlags = [ "--frozen-lockfile" ];
    meta = with pkgs.lib; {
      description = "KhulnaSoft Seeky command‑line interface";
      license     = licenses.asl20;
      homepage    = "https://github.com/khulnasoft/seeky";
    };
  };
  devShell = pkgs.mkShell {
    name        = "seeky-cli-dev";
    buildInputs = monorep-deps ++ [
      node
      pkgs.pnpm
    ];
    shellHook = ''
      echo "Entering development shell for seeky-cli"
      # cd seeky-cli
      if [ -f package-lock.json ]; then
        pnpm ci || echo "npm ci failed"
      else
        pnpm install || echo "npm install failed"
      fi
      npm run build || echo "npm build failed"
      export PATH=$PWD/node_modules/.bin:$PATH
      alias seeky="node $PWD/dist/cli.js"
    '';
  };
  app = {
    type    = "app";
    program = "${package}/bin/seeky";
  };
}

