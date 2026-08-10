{
  description = "Python environment for ai-skills benchmark tooling";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forEachSystem = function:
        nixpkgs.lib.genAttrs systems (system: function {
          pkgs = import nixpkgs { inherit system; };
        });
    in {
      devShells = forEachSystem ({ pkgs }: {
        default = pkgs.mkShell {
          packages = [
            (pkgs.python3.withPackages (pythonPackages: [
              pythonPackages.pyyaml
            ]))
          ];
        };
      });

      packages = forEachSystem ({ pkgs }: {
        default = pkgs.python3.withPackages (pythonPackages: [
          pythonPackages.pyyaml
        ]);
      });
    };
}
