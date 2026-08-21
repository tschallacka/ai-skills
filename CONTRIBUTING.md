# Contributing

Two documents matter before you write anything here:

- **`CODE-STYLE.md`** — the contract for every shell file in the repo:
  portability target, file skeleton, size limits, exit codes, output discipline.
  Read it first; it is what review checks against.
- **`DEVELOPMENT.md`** — release workflow, the npm package, versioning.

`AGENTS.md` holds the same orientation for coding agents, and
`planning/MAINTAINER-STYLE-CONTRACT.md` covers the *content* of generated plan
documents (a separate concern from the code that generates them).

## Development environment

Everything you need is in the flake, including the one tool you cannot get from
a package manager: **bash 3.2**.

```bash
nix develop           # bash32, bash32-run-tests, shellcheck, jq, git
```

Without nix, install `shellcheck` and `jq` yourself and see
[Testing on bash 3.2 without nix](#testing-on-bash-32-without-nix).

The flake is a **development** dependency only. Nothing it provides is required
to *use* the skills — those need `bash`, POSIX coreutils, `awk`, `sed`, `grep`,
`git`, plus `jq` for the `planning` skill and `memlimit` for
`resource-limited-testing` on Apple Silicon macOS (that one is the only memory
mechanism macOS has; Intel Macs degrade instead, since memlimit is arm64-only). See `CODE-STYLE.md` §1 for the
dependency budget, which is deliberately tight because these scripts get
installed onto other people's machines.

`benchmark/flake.nix` stays separate: it is the python environment for the
benchmark harness, usable from inside that directory. The root flake also
exposes it as `nix develop .#benchmark`.

## Why the flake builds bash 3.2 from source

`CODE-STYLE.md` §1 sets **bash 3.2 on macOS with BSD userland** as the
portability floor, because that is what stock macOS still ships as `/bin/bash`.
nixpkgs carries no bash 3.x, so the flake builds 3.2.57.

This is not ceremony. Before a real 3.2 was available, the floor was asserted in
documentation and never executed, and that is exactly how the breakage
accumulated: `declare -A`, `mapfile`, `${var,,}`, `stat -c`, `sed -i`,
`readlink -f`, `find -printf`. Six of 32 tests failed the first time the suite
was run on 3.2 — including `validate-plan.sh`, the plan readiness gate, which
aborted outright with `declare: -A: invalid option`.

Building it needs three non-obvious things, all recorded in `flake.nix`:
bash 3.2 predates C99 prototypes so a current gcc rejects its K&R declarations
(`-std=gnu89`, passed via `CC` so it reaches the build tools too); `yacc` is
needed because unpacking normalises timestamps and make regenerates the shipped
`y.tab.c`; and parallel builds race because the Makefile does not declare
`builtins/builtext.h` as a prerequisite.

## Running the tests

```bash
./run-tests.sh                 # your bash
./run-tests.sh --verbose
bash32-run-tests               # the same suite, entirely under bash 3.2
```

`run-tests.sh` runs every `test-*.sh` under `planning/tests/` and
`benchmark/planning/tests/`, each under the resource wrapper, in sorted order.
Both suites must be green on **both** bashes.

Two context-cache tests report `UNCONFIGURED` without `PLANNING_CONTEXT_CACHE`.
That is expected and is not a failure.

`bash32-run-tests` prepends a directory whose `bash` *is* 3.2, so every
`#!/usr/bin/env bash` child resolves to 3.2 as well. Running
`bash32 ./run-tests.sh` alone would only put the runner on 3.2 while its
children stayed on your system bash — which is the trap that makes a "we tested
it" claim worthless.

## Portability gotchas

`PORTABILITY.md` catalogues every portability trap this repository has hit, with
the symptom, the platform, the replacement, and the sites where each fix lives.
It is **generated** — edit `portability-rules.json`, or the marker at the site,
then run:

```bash
./generate-portability.sh
```

A workaround at a site carries a tagged marker whose id must exist in the
registry:

```bash
# PORTABILITY(<rule-id>): <one line, why this local code is shaped this way>
```

`PORTABILITY.md` carries a `<!-- generated: … -->` stamp. It is cheap to
rebuild, so **regenerate rather than reason about it**: `./generate-portability.sh`.
`--check` compares content while ignoring the stamp, and separately warns when a
scanned file was committed after the stamp — that warning means the "In the tree"
lists may be behind, not that anything is broken.

When several people or agents edit at once, each of them invalidates it. That is
expected and harmless. Never hand-edit it, and never resolve a merge conflict in
it by hand: take either side and re-run the generator.

`planning/tests/test-portability-contract.sh` enforces all of it: the doc must be
fresh, every marker id must resolve, the untagged `# PORTABILITY:` form is
rejected, and **no script may contain a banned construct** unless it is
allowlisted. That last assertion is the point — a gotcha caught once stays
caught, so the next person does not rediscover it three files away.

`planning/tests/lib-test.sh` holds the test-side shims (`t_sed_i`, `t_stat_mode`,
`t_unique_suffix`, `t_copy_tree`). Use them rather than reintroducing `sed -i`
or `stat -c` in a test.

## Linting

```bash
shellcheck -s bash --severity=error $(git ls-files '*.sh' | grep -v '^benchmark/results/')
```

Zero `error`-severity findings is the gate CI enforces. Lower severities are a
ratchet — do not add to them. `.shellcheckrc` disables exactly three checks
(SC1091, SC2016, SC2034), each with a comment saying why; do not add a fourth
without one.

Note that `git ls-files` only sees tracked files, so `git add` new scripts
before trusting a clean lint run.

## Before you open a pull request

```bash
bash -n <every edited script>
shellcheck -s bash <every edited script>
./run-tests.sh
bash32-run-tests
git diff --check
```

Then, specific to this repo:

- **A behavioural change needs a before/after diff, not an assertion.** The
  convention here is to capture a script's stdout, stderr and exit code across
  real inputs before the change and diff it after. Every difference should be
  one you intended and can name. This is how the refactors in this repo were
  verified, and it catches things the suite does not.
- **Update the registers.** `BUGS.json` holds defects, `TODO.json` queued work,
  written with the recipes in `bug-report/SKILL.md` and `todo/SKILL.md`. Close
  what you fixed, add what you found and did not fix, and name those entries in
  the commit message. Nothing fails when you skip this, which is exactly why it
  needs saying: a fix whose entry is never closed reads as still broken, and a
  defect noticed in passing and left only in prose is a defect nobody can find.
- **`install.sh` is generated — never edit it.** It is assembled by
  `installer/build.sh` from the ordered parts in `installer/src/NN-<concern>.sh`,
  with the runtime-dependency tables generated into it between the
  `# BEGIN/END GENERATED DEPENDENCY BLOCK` markers from `installer/tools.tsv` and
  each skill's `requires.tsv`. Edit the part (or the table), run
  `./installer/build.sh`, and commit the artifact along with the source. It stays
  committed and shipped because the README's first command is `curl … | bash` and
  it is the npm `bin`, so at runtime it has no siblings to source.
  `./installer/build.sh --check` (mirroring `./generate-portability.sh --check`),
  `planning/tests/test-installer-build.sh`, and the `installer-build` CI job all
  fail on a hand edit.
- **Ordering inside the parts is load-bearing**, and each part's banner says why:
  the CLI-mode `case` must stay the first argument consumer, `trap cleanup EXIT`
  must precede the first `mktemp -d`, the fd-3 block must precede any
  `ask`/`confirm`, and `show_splash` must run before `download_source`. The
  numeric prefixes are the build order, with gaps so a part can be inserted
  without renumbering.
- **A skill's runtime dependencies live in `<skill>/requires.tsv`**: tool id, a
  `<uname -s>:<uname -m>` condition, a strength, and the capability lost without
  it. `hard` means the installer refuses to install *that skill* and exits
  non-zero; `soft` means it installs and warns. How to verify and how to install
  a tool belongs in the shared `installer/tools.tsv`, once per tool. Both are
  line-oriented TSV rather than JSON, deliberately: `jq` is itself declared
  there, so a format needing `jq` to read it could not be read on the machine
  that is missing it.
- **A new or renamed file under a skill directory moves in four places** or
  `planning/tests/test-installer-manifest.sh` fails:
  `planning/PACKAGE-MANIFEST.txt`, `planning/PACKAGE-MAP.tsv`,
  `installer/src/50-manifest.sh`'s `skill_files()` (then rebuild), and
  `package.json`'s `files` (directory level only). The manifest and the map are
  compared byte-for-byte, so row *order* matters.
- **Every hard rule needs a regression test.** A rule in `CODE-STYLE.md` with no
  test and no CI leg is a suggestion, and suggestions rot.
- Match the commit style: short, lowercase-prefixed subjects (`planning:`,
  `benchmark:`, `docs:`).

## CI

`.github/workflows/ci.yml` runs the suite on `ubuntu-latest` and
`macos-latest`, plus a leg pinned to macOS's system bash 3.2, the shellcheck
gate, and the `installer-build` job that rebuilds `install.sh` and fails if the
committed artifact differs. The macOS legs are currently `continue-on-error` behind a
checked-in list of remaining blockers; that list is the definition of done for
making them blocking, and it lives in the workflow so the flag and its reason
travel together.

## Testing on bash 3.2 without nix

macOS already has it at `/bin/bash`. On Linux, either use the flake or build it
yourself with the same three fixes:

```bash
curl -sSLO https://ftp.gnu.org/gnu/bash/bash-3.2.57.tar.gz
tar xzf bash-3.2.57.tar.gz && cd bash-3.2.57
./configure --without-bash-malloc
make -j1 CC="gcc -std=gnu89 -w -Wno-implicit-function-declaration"
```

Then run the suite with that binary first on `PATH`, so children resolve to it
too:

```bash
mkdir -p /tmp/bash32bin && cp bash /tmp/bash32bin/bash
PATH=/tmp/bash32bin:$PATH /tmp/bash32bin/bash ./run-tests.sh
```
