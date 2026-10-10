{
  inputs = {
    v_flakes.url = "github:valeratrades/v_flakes?ref=v1.6";
  };

  outputs = { self, v_flakes }:
    let
      inherit (v_flakes) flake-utils pre-commit-hooks;
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import v_flakes.default_nixpkgs { inherit system; };
        rust = v_flakes.rs.default_nightly system;

        pre-commit-check = pre-commit-hooks.lib.${system}.run (v_flakes.files.preCommit { inherit pkgs; });
        manifest = (pkgs.lib.importTOML ./derivs/Cargo.toml).package;
        pname = manifest.name;
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

        rs = v_flakes.rs {
          inherit pkgs rust;
          build = {
            enable = true;
            workspace."./derivs" = [ ];
            workspace."./derivs_view" = [ ];
          };
        };
        github = v_flakes.github {
          inherit pkgs pname rs;
          lastSupportedVersion = "nightly-${v_flakes.rs.nightly_version}";
          enable = true;
          jobs.default = true;
        };
        readme = v_flakes.readme-fw { inherit pkgs pname; defaults = true; lastSupportedVersion = "nightly-${v_flakes.rs.nightly_version}"; rootDir = ./.; badges = [ "msrv" "loc" "ci" ]; };
        combined = v_flakes.utils.combine { inherit rust; modules = [ rs github readme ]; };
      in
      {
        apps.help = {
          type = "app";
          program = "${pkgs.writeShellScriptBin "help" ''
            cat <<EOF
            cargo r -p derivs_view -- view <tape.json>   re-render a saved tape as html, next to it
            nix develop                                  dev shell; regenerates CI workflows and README
            EOF
          ''}/bin/help";
        };

        devShells.default = with pkgs; mkShell {
          inherit stdenv;
          shellHook =
            pre-commit-check.shellHook
            + combined.shellHook
            + ''
              cp -f ${(v_flakes.files.treefmt) {inherit pkgs;}} ./.treefmt.toml
            '';

          buildInputs = [
            mold
            rust
          ] ++ pre-commit-check.enabledPackages ++ combined.enabledPackages;
        };
      }
    );
}
