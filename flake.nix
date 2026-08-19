{
  description = "ai-skills development environment: shell portability testing and linting";

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

      # CODE-STYLE.md §1 sets bash 3.2 as the portability floor, because that is
      # what stock macOS ships as /bin/bash. nixpkgs carries no bash 3.x, so
      # build it: without a real 3.2 the floor is only asserted, never tested,
      # which is exactly how the breakage this fixes accumulated.
      #
      # Installed as `bash32` rather than `bash` so it cannot shadow the shell
      # you are running. Point the suite at it with:
      #   bash32-run-tests            (wrapper below)
      # or, to force every `#!/usr/bin/env bash` child onto 3.2 as well:
      #   PATH="$(dirname "$(command -v bash32)")/bash32-bin:$PATH"
      mkBash32 = pkgs: pkgs.stdenv.mkDerivation rec {
        pname = "bash32";
        version = "3.2.57";

        src = pkgs.fetchurl {
          url = "mirror://gnu/bash/bash-${version}.tar.gz";
          hash = "sha256-P6na+F6/NQaPCQzlEoPd7rPHXrW8cLGkp8sFhov+BqQ=";
        };

        # bash 3.2 predates C99 prototypes, so a current gcc rejects its K&R
        # declarations outright ("too many arguments to function 'xmalloc'").
        # -std=gnu89 has to reach the build tools it compiles too (mkbuiltins),
        # so it goes on CC rather than CFLAGS.
        #
        # CFLAGS must reach ./configure as well as make. With the flags on make
        # only, configure probes with a different compiler dialect than the build
        # uses and mis-detects: it decided strsignal() was absent, so siglist.h
        # defined it as a macro that then collided with glibc's declaration
        # ("expected identifier or '(' before 'char'").
        CFLAGS = "-O2 -w -std=gnu89 -Wno-implicit-function-declaration";

        # Build tools that the Makefile compiles itself (mkbuiltins, mksyntax)
        # do not pick up CFLAGS, so -std=gnu89 also goes on CC. Via
        # makeFlagsArray, not makeFlags: nix word-splits makeFlags entries, which
        # would hand make `-std=gnu89` as its own flag and it exits with usage.
        preBuild = ''
          makeFlagsArray+=("CC=cc -std=gnu89 -w -Wno-implicit-function-declaration")
        '';
        configureFlags = [ "--without-bash-malloc" ];

        # The tarball ships y.tab.c, but unpacking normalises timestamps so make
        # decides parse.y is newer and regenerates it.
        nativeBuildInputs = [ pkgs.bison ];
        # bash 3.2's Makefile does not declare builtins/builtext.h as a
        # prerequisite of the objects that include it, so a parallel build races
        # and fails with "builtins/builtext.h: No such file or directory".
        enableParallelBuilding = false;
        doCheck = false;

        # Ship it under two names: `bash32` on PATH, and a bash32-bin/bash
        # directory to prepend when child scripts must resolve to 3.2 as well.
        postInstall = ''
          mv "$out/bin/bash" "$out/bin/bash32"
          rm -f "$out/bin/sh" "$out/bin/bashbug"
          mkdir -p "$out/bin/bash32-bin"
          ln -s "$out/bin/bash32" "$out/bin/bash32-bin/bash"
        '';

        meta = with pkgs.lib; {
          description = "GNU Bash 3.2.57, the macOS /bin/bash portability floor";
          homepage = "https://www.gnu.org/software/bash/";
          license = licenses.gpl2Plus;
          platforms = platforms.unix;
        };
      };
    in {
      packages = forEachSystem ({ pkgs }: rec {
        bash32 = mkBash32 pkgs;
        default = bash32;
      });

      devShells = forEachSystem ({ pkgs }:
        let
          bash32 = mkBash32 pkgs;
          # Run the whole suite on the floor, with children resolving to 3.2 too.
          runOn32 = pkgs.writeShellScriptBin "bash32-run-tests" ''
            set -euo pipefail
            repo="$(git rev-parse --show-toplevel)"
            echo "bash: $(${bash32}/bin/bash32 --version | head -1)"
            cd "$repo"
            PATH="${bash32}/bin/bash32-bin:$PATH" exec ${bash32}/bin/bash32 ./run-tests.sh "$@"
          '';
        in {
          default = pkgs.mkShell {
            packages = [
              bash32
              runOn32
              pkgs.shellcheck
              pkgs.jq
              pkgs.git
              # Renders the architecture diagrams. Development-only: the test
              # suite must still run without nix, so the render check reports
              # UNCONFIGURED when mmdc is absent rather than failing.
              pkgs.nodejs
              pkgs.mermaid-cli
            ];
            shellHook = ''
              echo "ai-skills dev shell"
              echo "  bash32           $(${bash32}/bin/bash32 --version | head -1 | cut -d' ' -f4)  (CODE-STYLE.md §1 floor)"
              echo "  bash32-run-tests run ./run-tests.sh entirely under bash 3.2"
              echo "  shellcheck       $(shellcheck --version | awk '/^version:/ { print $2 }')"
              echo "  mmdc             $(mmdc --version 2>/dev/null || echo unavailable)  (renders the mermaid diagrams)"
              echo
              echo "Portability checks:"
              echo "  ./run-tests.sh                    # your bash"
              echo "  bash32-run-tests                  # the floor"
              echo "  shellcheck -s bash --severity=error \$(git ls-files '*.sh' | grep -v '^benchmark/results/')"
            '';
          };

          # The benchmark tooling's python environment; benchmark/flake.nix keeps
          # its own copy for use from inside that directory.
          benchmark = pkgs.mkShell {
            packages = [
              (pkgs.python3.withPackages (pythonPackages: [ pythonPackages.pyyaml ]))
              pkgs.jq
            ];
          };
        });
    };
}
