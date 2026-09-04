{
  description = "ai-skills development environment: shell portability testing and linting";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forEachSystem = function:
        nixpkgs.lib.genAttrs systems (system: function {
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
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

          # One script on the floor, with the same PATH injection the suite gets.
          # `bash32 some-script.sh` is not the same test: a #!/usr/bin/env bash
          # child still resolves to whatever bash PATH offers, so half the run
          # can be 5.3. That divergence is what hid the plan-context.sh init
          # defect -- it only reproduced under the forced PATH.
          runOne32 = pkgs.writeShellScriptBin "bash32-run" ''
            set -euo pipefail
            if [ "$#" -eq 0 ]; then
              echo "usage: bash32-run <script> [args...]" >&2
              exit 64
            fi
            PATH="${bash32}/bin/bash32-bin:$PATH" exec ${bash32}/bin/bash32 "$@"
          '';

          # One toolchain: the NEWEST stable rust the pinned nixpkgs offers.
          #
          # A version literal here rots silently until something refuses to
          # build. The workspace now has one root rust-toolchain.toml, so this
          # shell and every crate use the same pinned compiler and components.
          #
          # There is deliberately no older floor toolchain. Every crate here is
          # pure Rust with no system library dependency a newer compiler cannot
          # satisfy, and a binary built by a newer compiler runs on the target
          # systems all the same, so carrying a second compiler to prove an old
          # one still works buys nothing and costs a build.
          #
          # `flake.lock` is what makes "latest" reproducible: everyone on one
          # lock gets one compiler, and `nix flake update` is the deliberate
          # moment it moves.
          rustTargets = [
            "x86_64-unknown-linux-musl"
            "aarch64-unknown-linux-musl"
            "x86_64-apple-darwin"
            "aarch64-apple-darwin"
            "x86_64-pc-windows-msvc"
          ];
          rustLatest = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "rustfmt" "clippy" ];
            targets = rustTargets;
          };
          # A C compiler that targets musl, for the crates whose dependencies
          # build C: ai-text-editor pulls pcre2-sys, libsqlite3-sys and blake3.
          # Without it cc-rs compiles them with the host's glibc cc and the link
          # against musl fails on __memcpy_chk, __memmove_chk and open64 --
          # glibc fortify and LFS symbols musl does not carry. CI installs
          # musl-tools for this reason; the dev shell owes the same.
          #
          # Linux only: host_triple() in setup-dev-env.sh resolves a Darwin host
          # to *-apple-darwin, so no musl target is ever built there.
          muslTarget =
            if pkgs.stdenv.hostPlatform.isAarch64
            then "aarch64-unknown-linux-musl"
            else "x86_64-unknown-linux-musl";
          muslCross =
            if !pkgs.stdenv.hostPlatform.isLinux then null
            else if pkgs.stdenv.hostPlatform.isAarch64
            then pkgs.pkgsCross.aarch64-multiplatform-musl.stdenv.cc
            else pkgs.pkgsCross.musl64.stdenv.cc;
        in {
          default = pkgs.mkShell {
            packages = [
              bash32
              runOn32
              runOne32
              pkgs.shellcheck
              pkgs.actionlint
              pkgs.git
              # The build toolchain for the crates under src/ (CODE-STYLE 1b).
              # This was development-only while nothing shipped was Rust; that
              # stopped being true when src/tony-the-pony and src/chat landed, so
              # cargo now builds artifacts a release carries. It stays a DEV
              # dependency all the same: a shipped binary asks nothing of the
              # target box, which is why shipping one lowers the runtime budget
              # in CODE-STYLE section 1 rather than widening it.
              rustLatest
              # Renders the architecture diagrams. Development-only: the test
              # suite must still run without nix, so the render check reports
              # UNCONFIGURED when mmdc is absent rather than failing.
              pkgs.nodejs
              pkgs.mermaid-cli
              # Dev tooling resolves from the flake; nothing but nix itself is
              # assumed present. Plain python3, not withPackages: tracked dev
              # scripts import stdlib only, and benchmark owns the pyyaml shell.
              pkgs.python3
              # ai-text-editor's storage CLI; the package is sqlite, the binary
              # it provides is sqlite3.
              pkgs.sqlite
              # The installer's fetch path (curl | bash), declared so the shell
              # owns it rather than borrowing the machine's.
              pkgs.curl
              # The pre-rjq filter tool is deliberately ABSENT: rjq is the
              # mandated register runtime, and the older one on PATH can mask a
              # defect in it -- T85 tracks rjq's missing IN/1. openssl likewise,
              # since plan-crypt owns digests now. Named obliquely because
              # test-rjq-active-references treats the bare word as a call site
              # anywhere but column zero (B154, filed against that gate).
            ] ++ pkgs.lib.optional (muslCross != null) muslCross;
            shellHook = ''
              ${pkgs.lib.optionalString (muslCross != null) ''
                export CC_${builtins.replaceStrings ["-"] ["_"] muslTarget}="${muslCross}/bin/${muslTarget}-gcc"
                export CARGO_TARGET_${pkgs.lib.toUpper (builtins.replaceStrings ["-"] ["_"] muslTarget)}_LINKER="${muslCross}/bin/${muslTarget}-gcc"
              ''}
              # This tree's own compiled helpers win over any installed copy.
              # setup-dev-env.sh builds them into bin/<target triple>/; without
              # this, rjq resolved to ~/.local/bin/rjq -- an installed binary
              # that need not match the src/rjq this tree builds.
              for _bin in "$PWD"/bin/*/; do
                [ -d "$_bin" ] && PATH="$_bin:$PATH"
              done
              unset _bin
              export PATH
              echo "ai-skills dev shell"
              echo "  bash32           $(${bash32}/bin/bash32 --version | head -1 | cut -d' ' -f4)  (CODE-STYLE.md §1 floor)"
              echo "  bash32-run-tests run ./run-tests.sh entirely under bash 3.2"
              echo "  bash32-run       run one script under bash 3.2, children included"
              echo "  shellcheck       $(shellcheck --version | awk '/^version:/ { print $2 }')"
              echo "  mmdc             $(mmdc --version 2>/dev/null || echo unavailable)  (renders the mermaid diagrams)"
              echo "  cargo            $(cargo --version | cut -d' ' -f2)  (builds the crates under src/)"
              echo "  cargo186         $(cargo186 --version | cut -d' ' -f2)  (plan-overview's declared floor)"
              echo "  ./setup-dev-env.sh  build the crates into this tree so the skills use them"
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
              pkgs.shellcheck
            ];
          };
        });
    };
}
