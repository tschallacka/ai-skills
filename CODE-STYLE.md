# Shell code style and structure

The contract every shell file in this repository conforms to. `DEVELOPMENT.md`
covers the release workflow, `AGENTS.md` covers how to operate in the repo, and
`planning/MAINTAINER-STYLE-CONTRACT.md` covers the *content* of generated plan
documents, and `CODE-CONTRACTS.md` covers how a script must behave toward what
other scripts and other runs depend on. This file covers the *code*.

Two audiences read these scripts: a developer on an unknown machine, and a
maintainer AI agent that has to locate one behaviour in a tree of ~90 scripts.
Both are served by the same thing — small files with a predictable skeleton, one
way of doing each job, and no platform surprises.

- [1. Portability contract](#1-portability-contract)
- [2. File skeleton](#2-file-skeleton)
- [3. Size and decomposition limits](#3-size-and-decomposition-limits)
- [4. Usage and help](#4-usage-and-help)
- [5. Errors and exit codes](#5-errors-and-exit-codes)
- [6. Argument parsing](#6-argument-parsing)
- [7. Libraries and sourcing](#7-libraries-and-sourcing)
- [8. Writing files atomically](#8-writing-files-atomically)
- [9. Naming, quoting, tests](#9-naming-quoting-tests)
- [10. Output discipline](#10-output-discipline)
- [11. Comments and diagrams](#11-comments-and-diagrams)
- [12. Test scripts](#12-test-scripts)
- [13. Checklist before committing](#13-checklist-before-committing)

Reference implementations to copy from, in order of how much they get right:
`resource-limited-testing/scripts/limited-run.sh` (skeleton, `uname` dispatch,
exit codes), `plan-context-lib.sh`'s `context_hash_file` (guarded optional
dependency), `planning/scripts/plan-root.sh` (docblock, decision rules stated
before the code).

---

## 1. Portability contract

**Target: bash 3.2 on macOS, bash 4/5 on Linux, GNU *or* BSD userland.**

macOS ships `/bin/bash` 3.2.57 and has done for over a decade. `#!/usr/bin/env
bash` resolves to it unless the user installed Homebrew bash *and* has it first
on `PATH`, which we do not get to assume. So bash 3.2 is the floor, and BSD
coreutils are the floor for utilities.

Interpretation of "works in every shell": the **shebang stays bash** — these are
bash scripts and rewriting them for `dash`/busybox `ash` would cost more than it
buys. What must be shell- and OS-agnostic is everything *around* the bash
syntax: the utilities invoked, their flags, the locale, and the tty assumptions.
A script that runs under bash 3.2 with BSD userland runs everywhere we care
about.

### Banned constructs and their replacements

| Banned | Why | Use instead |
|---|---|---|
| `declare -A` | bash 4 | `plan_map_set` / `plan_map_get` (`plan-document-lib.sh`) |
| `mapfile` / `readarray` | bash 4 | `arr=(); while IFS= read -r l; do arr+=("$l"); done < <(…)` |
| `${var,,}` / `${var^^}` | bash 4 | `$(printf '%s' "$var" \| tr '[:upper:]' '[:lower:]')` |
| `local -n`, `wait -n`, `&>>`, `**` | bash 4.3+ | restructure |
| `"${arr[@]}"` when possibly empty | unbound under `set -u` before bash 4.4 | `${arr[@]+"${arr[@]}"}` |
| `stat -c FMT` | GNU only | `plan_stat_mode` / `plan_stat_uid` |
| `sed -i` | BSD requires a suffix argument | `sed … "$f" > "$f.new" && mv "$f.new" "$f"` |
| `readlink -f` | GNU; macOS only since 12.3 | `plan_resolve_symlink` |
| `find … -printf` | GNU only | `find … -print` + `dirname` in the read loop |
| `sort -z` | GNU only | sort newline-safe values, or drop the sort |
| `date +%s%N` | `%N` is GNU only | `date +%s` plus `$RANDOM$RANDOM` |
| `grep -P`, ERE `\b` | GNU only / undefined in POSIX ERE | `([^A-Za-z0-9_]\|^)…([^A-Za-z0-9_]\|$)` |
| `sed -r` | GNU only | `sed -E` (POSIX, works on both) |
| `cp -R src/. dst` | source ending in `/.` is unspecified | `(cd src && tar cf - .) \| (cd dst && tar xf -)` |
| `echo -e`, `echo -n` | not portable | `printf` |
| bare `sort` on data users see | locale-dependent collation | `LC_ALL=C sort` (or the global `export LC_ALL=C`) |
| `ps -p $$ -o tty=` as a tty probe | Linux prints `?`, macOS `??` | `[ -t 0 ]`, then try `exec 3</dev/tty 2>/dev/null` |
| `timeout`, `nproc`, `free`, `realpath`, `tac`, `base64 -w`, `head -n -N`, `xargs -r`, `/proc/*`, `getopt` | GNU/Linux only | ask before introducing any of these |

An exception needs a comment saying which platform it is for and a
`command -v`-guarded fallback. The pattern:

```bash
# Hash helper: GNU and BSD boxes both appear in the wild.
if command -v sha256sum >/dev/null 2>&1; then
    hash_file() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
    hash_file() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
    plan_die 'need sha256sum or shasum' 69
fi
```

### Dependencies

The dependency budget differs between what we ship to a user's machine and what
only a contributor runs. Know which side of the line your file is on.

**Shipped runtime** — `install.sh`, `planning/scripts/**`, anything registered
in `planning/PACKAGE-MANIFEST.tsv`, and the other skill directories:

| Tool | Status |
|---|---|
| `bash`, POSIX `coreutils`, `awk`, `sed`, `grep`, `git` | assumed present |
| `rjq` | allowed — the furthest a runtime dependency may go. Declared `hard` in `planning/requires.tsv`, so the installer refuses to install *that skill* up front with a per-platform hint rather than failing halfway |
| `openssl` | **no longer used, and no longer declared**. It was the last rung of the planning skill's SHA-256 chain and its only source of random bytes. Both are now the shipped `plan-crypt` binary (section 1b), so the `soft` row left `planning/requires.tsv`: a static binary asks the target machine for nothing. A new `openssl` call needs the row back, and `installer/build.sh` fails the build if one appears without it |
| `memlimit` | allowed for `resource-limited-testing` on Apple Silicon macOS only — the documented exception to the `rjq` ceiling, because macOS offers no other way to cap memory at all. Declared `soft` in `resource-limited-testing/requires.tsv`, so the skill installs with a warning and degrades to `nice` + `cpulimit`; never vendored |
| **`python3`** | **not allowed, in any form — not even guarded-optional** |

This table governs what the target machine must already **have**. It does not
govern what a skill **delivers**. A skill may ship prebuilt, self-contained
binaries as artifacts, declared row by row in its own `binaries.tsv` and
validated by `tests/test-shipped-binaries.sh`. That does not widen the budget
above — it narrows it, because a static binary asks nothing of the box. The
`chat` skill's `python3 → node → perl → socat` any-of group in
`chat/requires.tsv` exists only because there is no binary yet; when one ships,
those rows go away and chat's runtime requirement drops to `bash` for the
helpers.

The build toolchain for such a binary (Rust, `cargo`) is a **dev** dependency
like `shellcheck` or `bash32`: contributors need it, users never do. The crate
lives under `src/<binary>/` and its sources carry `MODE: DEV` **and**
`PACKAGE: PROD` — see section 1b, which is the authority on the layout and the
markers. What ships is the compiled artifact under
`<skill>/bin/<target triple>/`, exempt from the marker rule for the obvious
reason that a Mach-O, ELF or PE file has no comment syntax.

The `memlimit` exception is narrow on purpose. It buys a capability that cannot
be had otherwise — macOS has no cgroups and rejects `setrlimit(RLIMIT_AS)` on
Apple Silicon — it is confined to one skill on one platform, and no other file
may depend on it. It is declared `soft` rather than `hard` because
`limited-run.sh` degrades *honestly* without it: it says the RAM cap is not
enforced and still limits CPU through `nice` and `cpulimit`. That is the line
between the two strengths, and it is also why `python3` stays banned rather than
guarded: a guard is unacceptable when it turns a documented feature into one that
silently does not exist, and `rjq` is `hard` for the planning skill for exactly
that reason — `validate-plan.sh` exits 69 without it.

`python3` is banned here rather than merely guarded because a guard turns a
documented feature into one that silently does not exist on a machine without
the interpreter. Text and line processing goes to `awk`; JSON goes to `rjq`.
Neither of the two former python sites needed it: `install.sh`'s permission
editors became `rjq` (already guaranteed present — the permission step only runs
when `planning` was selected, and `planning` requires `rjq`), and
`plan-content.sh diff` became `awk`, which is its natural home since it is
parsing `git diff -U0` hunk headers line by line.

**Development tooling** — `benchmark/**` and unregistered files under
`planning/tests/`: `python3` is fine, and the benchmark harness uses it heavily
for JSON synthesis. Do not let it leak across the line. Note that some test
files under `planning/tests/` *are* registered in the manifest and therefore
ship; those follow the shipped-runtime rule (see
`planning/tests/test-installer-manifest.sh`, which uses an `awk` state machine
where it once used a python regex).

Check which side you are on before adding a dependency:

```bash
grep -Fq "<your/file.sh>" planning/PACKAGE-MANIFEST.tsv && echo shipped || echo dev-only
```

`awk` must stay POSIX regardless of side — see the paragraph below.

`awk` must stay POSIX: no `gensub`, `asort`, `length(array)`, `IGNORECASE`,
`ENVIRON`, multi-char `RS`, or backslash escapes inside a bracket expression
(`[[:space:]\n]` is wrong — `[[:space:]]` already covers newline).

### Text is bytes or characters — pick one

`${#str}` counts characters; `head -c`, `wc -c`, and byte budgets count bytes.
Plan documents are full of multi-byte glyphs (`§ 💤 ⏳ ✅ —`), so mixing the two
truncates mid-sequence and mis-bills page budgets. Do all budget arithmetic in
one unit and say which in a comment.

### Enforcement

Portability is enforced by CI, not by good intentions: `.github/workflows/ci.yml`
runs the suite on `ubuntu-latest` **and** `macos-latest`, plus a `shellcheck`
pass. A rule in this file without a CI leg or a regression test in
`planning/tests/` is a suggestion, and suggestions rot.

---

## 1b. Where compiled tools live

A shipped binary gets **one directory per binary under `src/`**, sitting beside
the skill directories rather than inside one:

```
src/tony-the-pony/      Cargo.toml, rust-toolchain.toml, src/*.rs
src/<next-binary>/      likewise
```

One binary, one crate, one directory. A binary does not live inside the skill
that happens to use it: several skills may consume one tool, and a skill
directory is what the installer copies, while a crate is what CI builds.

**Markers.** Crate sources are `MODE: DEV` **and** `PACKAGE: PROD` — the same
pair used for the library sources under `planning/scripts/lib/`, and for the
same reason: the file itself is never delivered, but its content ends up inside
something that is. Put both markers on the **first two lines**, before any `//!`
module documentation. `marker_of` reads only the head of a file, so a marker
placed after a long doc comment silently falls out of the window and the file
reads as unmarked. `rust-toolchain.toml` is `MODE: DEV` alone: it configures the
build and is compiled into nothing. `Cargo.lock` is generated and exempt.

**This does not widen the dependency budget in section 1.** That budget governs
what the *target machine must already have*; a static binary asks it for
nothing, so shipping one lowers the requirement rather than raising it. What
ships is declared per target in the skill's own `binaries.tsv`, validated by
`tests/test-shipped-binaries.sh`. The compiler is a dev dependency and lives in
the flake.

---

## 2. File skeleton

Every script, in this order, no exceptions:

```bash
#!/usr/bin/env bash
# <name> — one-line purpose, imperative mood.
#
# Longer explanation when the mechanism is not obvious from the usage: what
# decision this script owns, what it writes, what it refuses to do.
#
# Usage:
#   <name> <required-arg> [--optional-flag <value>]
#
# Exit codes beyond the standard vocabulary, if any.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "$(plan_resolve_symlink "${BASH_SOURCE[0]}")")" && pwd)"
source "$script_dir/plan-document-lib.sh"
```

The docblock is at the top and starts at line 2 because `monitor-read.sh`,
`supervision-frame.sh` and `role-context.sh` print it as their own `--help` via
`sed -n '1,20p' "$0"`. Keep the usage block inside the first 20 lines.

`set -euo pipefail` goes on the first executable line. The one sanctioned
exception is a runner that must survive a child's failure to print a summary
(`run-tests.sh`); it uses `set -uo pipefail` and says why in the docblock.

`export LC_ALL=C` goes in every script that sorts, compares, or byte-counts
text. That is nearly all of them, so just put it everywhere.

---

## 3. Size and decomposition limits

| Unit | Limit | On exceeding |
|---|---|---|
| Executable script | 400 lines | extract a `*-lib.sh` sibling |
| Library | 500 lines | split by concern |

**A library function lives in its own file, and the library is compiled.**
`planning/scripts/lib/<group>/<function>.sh` holds one function, its comment, and
nothing else. `planning/scripts/build-plan-libs.sh` concatenates each group into
the `plan-*-lib.sh` that ships, so the runtime cost stays one file per library --
sourcing 47 files measured 2.6x the cost of one, paid on every helper
invocation -- while the maintained form is one function per file.

Adding a function means creating one file in the right group directory and
running the build. The directory is the registration; there is no list to
update. Group state goes in `00-*.sh`, which sorts first, and anything that must
run after every definition goes in `99-*.sh`, which sorts last.

Each file carries its own shebang so a test can source it alone, and the compiler
strips the shebang and the `set` line so the output declares them once. Sourcing
one function file must succeed on its own; calling the function may still need
its siblings, which is the dependency made visible rather than hidden.

The compiled libraries are generated artifacts under CODE-CONTRACTS.md contract
7: never hand-edited, and `test-plan-libs-build.sh` fails when one is stale.
| Function | 40 lines | extract a helper |
| Inline `awk` program | 15 lines | move to a lib function or a `.awk` file |
| Unbroken block with no comment or function boundary | 60 lines | add a section banner and a diagram |

These are the numbers that make a file navigable by an agent that greps for a
behaviour and reads 100 lines around the hit. A 1300-line script with no
function decomposition costs an order of magnitude more context to answer one
question about.

Split by **concern**, not by line count: each extracted file gets a docblock
naming the one job it owns. Sibling libraries are named
`<subject>-lib.sh` (`plan-document-lib.sh`, `plan-context-lib.sh`) and are
sourced, never executed.

When a file in `planning/` is added, renamed, or removed, four places move
together or `planning/tests/test-installer-manifest.sh` fails:

1. `planning/PACKAGE-MANIFEST.tsv`
2. `planning/PACKAGE-MAP.tsv`
3. `installer/src/50-manifest.sh` → `skill_files()`, then `installer/build.sh`
4. `package.json` `files` (directory level only — no change for a new sibling)

`install.sh` is the one file that must ship as a single artifact — it is fetched
and run standalone (`curl … | bash`) and is the npm `bin`, so it has no siblings
to source. It is no longer hand-maintained as one file: it is **assembled from
parts** by `installer/build.sh` out of `installer/src/NN-<concern>.sh`, plus the
dependency tables generated from `installer/tools.tsv` and each skill's
`requires.tsv`. Edit a part and run the build; never edit `install.sh` itself.
`planning/tests/test-installer-build.sh` and the `installer-build` CI job fail on
a hand edit.

Long section banners inside a part, so the assembled artifact stays navigable:

```bash
# ─────────────────────────────────────────────────────────────────────────────
# Skill registry — the single list every other section derives from.
# ─────────────────────────────────────────────────────────────────────────────
```

and a table of contents in the docblock listing the banners in order.

---

## 4. Usage and help

One function, stdout, optional return code:

```bash
usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <plan-directory> <goal-name> --title <title>
       ${0##*/} --help
USAGE
    exit "$rc"
}
```

- `usage 0` from `-h|--help`; bare `usage` on an argument error.
- `${0##*/}`, never `$(basename "$0")` — no subshell, and correct.
- `cat <<USAGE` or `printf`, never `echo`.
- Help goes to **stdout** (the user asked for it); an argument error's usage
  goes to stdout too, with the diagnostic line on stderr before it.
- Never define a `usage()`/`help()` pair with duplicated text. One function.

### Help output is user-facing, so format it

`--help` is interface, not a comment dump. Two rules:

**1. Never print raw comment syntax.** A script that serves its help from its own
docblock (`sed -n '1,20p' "$0"`) must strip the leading `# ` and skip the
shebang. Printing `#!/usr/bin/env bash` as the first line of help is a defect.

**2. Fence verbatim sections**, with the same dividers as comments (§11) minus
the `#`, so a reader can tell the synopsis and examples from the prose:

```
Usage:
  monitor-read.sh show <frame-file>

---- quoted: output shape ----
subagent=<name> status=<green|escalated|blocked>
---- end quoted ----
```

`planning/tests/test-comment-format.sh` checks both: no help output may contain a
shebang or a `# `-prefixed line, and any fence in it must be balanced and named.

**Help output has no length limit.** It is output, not a comment, so neither the
three-line rule nor the 20-line docblock cap applies to it. Say what the user
needs.

A script that serves help from its docblock must print the **whole leading
comment block**, never a fixed line window:

```bash
awk 'NR == 1 { next }
     /^#/ {
         sub(/^#[[:space:]]?/, "")
         if ($0 ~ /^----[[:space:]]*(quoted:|end quoted)/) next
         print; next
     }
     { exit }' "$0"
```

It strips the shebang, strips the leading `# `, **drops the fence dividers**, and
stops at the first non-comment line. The fence is a convention for whoever reads
the source; a user reading `--help` wants the quoted content, not the scaffolding
around it. The content between the dividers prints normally.

A hardcoded `sed -n '2,20p'` silently truncates the moment the docblock grows,
which is how a fence in a docblock once looked like it would delete help text.
It does not. `planning/tests/test-comment-format.sh` fails any script that serves
help from a fixed line window.


---

## 5. Errors and exit codes

`plan_die "<message>" [code]` from `plan-document-lib.sh` is the only fatal
path. It prefixes `${0##*/}: `, writes to stderr, and exits with the code.

```bash
plan_require_directory "$plan_dir"                       # 66 if absent
[ ! -e "$goal_dir" ] || plan_die "goal exists: $goal_dir" 73
```

Exit-code vocabulary (sysexits.h, and it is a contract — callers branch on it):

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | a verdict of "no" from a checker (`validate-plan.sh` found failures) |
| 64 | the caller invoked this wrong — bad flags, bad arity, malformed ID |
| 65 | input data exists but is malformed (a corrupt manifest, a bad table) |
| 66 | a required input file or directory is missing |
| 69 | a required external tool or marker is unavailable |
| 70 | internal error — a bug in this script |
| 73 | refusing to overwrite an artifact that already exists |
| 78 | the environment is misconfigured (unsupported bash, missing variable) |

`64` is *not* the answer to everything. "The plan is in the wrong state" is 65,
66 or 73 and a caller must be able to tell it apart from "you typed it wrong".

For a checker that accumulates findings rather than dying on the first one, use
`plan_fail` / `plan_warn` (increment a counter, print `FAIL:`/`WARN:` to
stderr, keep going) and exit 1 at the end if the counter is non-zero.

Never `exit` from a sourced library function — `return` a code and let the
caller decide.

---

## 6. Argument parsing

**Three or more inputs, or any optional input: a flag loop.**

```bash
plan_dir=""
title=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --title) [ "$#" -ge 2 ] || usage; title="$2"; shift 2 ;;
        --) shift; break ;;
        -*) plan_die "unknown option: $1" 64 ;;
        *) [ -z "$plan_dir" ] || usage; plan_dir="$1"; shift ;;
    esac
done
[ -n "$plan_dir" ] || usage
```

**Two or fewer unambiguous inputs: positional**, with `[ "$#" -eq 2 ] || usage`.

Ten positional arguments is a defect: unnamed, order-dependent, unextendable,
and impossible to read at a call site. `add-work-unit.sh` had such a form and it
was removed outright rather than deprecated. Convert to flags when you touch
such a script.

Do not pre-scan `"$@"` into a `filtered_args` array to find `--help`. A flag loop
handles `-h` in band, and the pre-scan trips `set -u` on bash 3.2 when the array
ends up empty.

Defaults are plain assignments before the loop, never `"${5:-open}"` buried at a
use site.

---

## 7. Libraries and sourcing

```bash
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
```

- Lowercase `script_dir` — it is script-local, not exported.
- `${BASH_SOURCE[0]}`, never `$0` (wrong when sourced) and never
  `dirname "$0"` for locating a sibling.
- Invoke every sibling through `"$script_dir/…"`.

This form resolves symlinked *directories* (via `cd` + `pwd`) but not a
symlinked script *file*. That is deliberate and it is the standard here:
`plan_resolve_symlink` cannot help, because `script_dir` is what locates the
library the function lives in — it must be computed before anything is sourced.
The installer *copies* files rather than symlinking them, so a symlinked script
file is an edge case, not the norm.

Where being invoked through a symlink is a real requirement, resolve it inline
before computing `script_dir`, and say in a comment why the script needs it:

```bash
# Invoked through a symlink from a monitor wrapper, so resolve the link before
# locating siblings. No readlink -f: it is GNU-only (macOS only since 12.3).
self="${BASH_SOURCE[0]}"
while [ -L "$self" ]; do
    target="$(readlink "$self")"
    case "$target" in
        /*) self="$target" ;;
        *) self="$(dirname "$self")/$target" ;;
    esac
done
script_dir="$(cd "$(dirname "$self")" && pwd)"
```

`monitor-read.sh` is the one script with that requirement today.

Every executable script sources `plan-document-lib.sh` unconditionally. No
`[ -f … ] && source …` — a missing library is a broken install, so fail loudly.

`plan-reconcile-lib.sh` requires `plan-document-lib.sh` first; a library that
depends on another sources it itself rather than trusting the caller's order.

**Every function in a sourced file carries its file's prefix.** Bare names in a
sourced file shadow the caller's functions. `usage`, `help` and `main` are the
only bare names allowed, and only in files that are never sourced.

The prefix is per-library and must be used consistently within it, not globally
`plan_`: `plan-document-lib.sh` and `plan-reconcile-lib.sh` use `plan_`,
`plan-context-lib.sh` uses `context_` (all 32 of its functions), and the
`validate-plan-*-lib.sh` pass drivers use `plan_validate_`. Any of those
prevents collision, which is the point of the rule. Match the file you are in
rather than renaming a whole library to satisfy a global spelling.

Two known deviations, left deliberately because the churn outweighs the risk —
neither file is sourced by anything today, so nothing can shadow:
`validate-plan-*-lib.sh` keeps the pre-existing bare `fail`/`warn`/`trim`/
`require_heading` and the command-detector helpers (~24 functions, ~200 call
sites), and `plan-env.sh` keeps bare `die`/`usage`/`absolute_path` behind a
`# Not sourced:` note. If either becomes sourceable, the rename comes first.

Before writing a helper, grep the libs — the five clusters below were each
re-implemented between 3 and 26 times, and any new copy is a review finding:

| Job | Use |
|---|---|
| Read a work-unit inventory row | `plan_inventory_row` |
| Trim a markdown table cell in awk | the shared `plan_awk_trim` prelude |
| Write a file atomically | `plan_atomic_write` |
| Render a progress bar / percentage / status icon | `plan_progress_bar`, `plan_progress_icon` |
| Require a file, directory, or absence | `plan_require_file`, `plan_require_directory`, `plan_refuse_existing` |

---

## 8. Writing files atomically

```bash
{
    printf '# Goal: %s\n\n' "$title"
    …
} | plan_atomic_write "$goal_file"
```

`plan_atomic_write` creates its temp with `mktemp` **in the target's own
directory** (so the rename is atomic — same filesystem), registers it with the
single process-wide cleanup list, `chmod`s to match, and `mv -f`s into place.

Never hand-roll it. The old `"$f.tmp.$$"` + `trap … EXIT` + `trap - EXIT`
pattern appeared ~40 times, four of those without a trap at all, and one
nesting broke the outer trap so two temps leaked. `$$` is not collision-safe
(PIDs are reused) and leaves predictable debris.

Never `trap - EXIT` to "release" a cleanup: it discards the *outer* handler too.
Register with the cleanup list and let it run.

---

## 9. Naming, quoting, tests

- `local x="$1"` — always `local`, always quoted.
- `snake_case` for everything script-local. `UPPER_CASE` is reserved for
  exported/environment variables (`PLANS_ROOT`, `TMPDIR`,
  `PLANNING_AGENT_TMPDIR`). A local named `MODE` is a lie about its scope.
- `[ … ]` by default. `[[ … ]]` **only** for `=~`, where it is required.
- Quote every expansion. The exceptions (`$(( ))`, an intentional glob) get a
  comment.
- No lazy quantifiers (`.*?`) in a bash regex — POSIX ERE has none, it silently
  parses as `(.*)?` and stays greedy.

---

## 10. Output discipline

**stdout is the result. stderr is everything else.** A caller must be able to
`x="$(script …)"` and get only the answer.

On success, print exactly one line: `<Verb> <absolute-path-or-id>`.

```
Created /home/u/.plans/demo/01-first-goal
Added W03 and 01-step-parse-input.md
Updated /home/u/.plans/demo/progress.md
```

No blank-line padding, no `Next: run …` advice, no `note:` nudges — those belong
in the docblock, where they are still there tomorrow. Progress chatter,
diagnostics, and warnings go to stderr.

A script whose output another script parses emits TSV (`plan-content.sh find`)
or grows a `--format json` flag (`plan-context.sh`). It does not emit prose that
a caller has to `sed` apart.

---

## 11. Comments and diagrams

Two comment rules exist in this repository and they govern different code. Know
which one applies before you write a comment:

| Code | Rule |
|---|---|
| Code a skill **produces in a user's project** under a plan | `planning/references/comment-discipline-contract.md` — self-documenting by default, **three-line hard limit**, genuine non-evident specifics only, no why-prose, no history, no cross-file relationship notes |
| **This repository's own source** | this section |

They differ because the audiences differ: produced code sits in a repo whose
owner has the full context, while these scripts ship to strangers' machines and
are read by agents that cannot see this conversation. The shipped contract is
the stricter one, and it is the one users' agents are held to — so do not let
this section be quoted as a licence to ignore it.

**In-body comments here follow the three-line limit too.** One block, three
lines, recording a constraint a reader would otherwise undo:
`add-goal.sh:44-46` ("emitted empty — a placeholder is valid to write and
invalid to keep") and `create-step-testing.sh:61-63` (single-char `RS` for
mawk/BSD-awk parity) are the model. What does *not* belong in a comment:

- **Measurements and benchmark numbers.** They rot and they are history.
- **Justification of a past decision.** That is what `ARCHITECTURE.md`,
  `CONTRIBUTING.md` and the commit message are for.
- **Cross-file relationships** ("see also X", "replace with Y from Z").
  Clause 5 of the shipped contract bans these outright, for the good reason that
  they duplicate what symbol search already answers and drift when the other
  file moves. A `# TODO`-style marker naming a concrete follow-up is the one
  exception, and it should be a single line.

### Quoting verbatim material in a comment

Prose and quoted material are different things and must look different. When a
comment carries something **verbatim** — an emitted-output contract, a data
shape, an example invocation, a message another program matches on — fence it:

```bash
# The telemetry stdout contract, first match wins:
# ---- quoted: telemetry keys ----
# thread_id=<session id>
# usage_records=<n>|unavailable
# total_usage_tokens=<n>|unavailable
# telemetry_status=available|unavailable:<reason>
# ---- end quoted ----
```

Rules:

- The opening divider names what is quoted: `# ---- quoted: <what> ----`. The
  closing divider is exactly `# ---- end quoted ----`.
- **A quoted block does not count toward the three-line limit.** The prose
  around it still does. That exemption is the reason the dividers are mandatory
  rather than decorative: they are what makes the quoted region recognisable to
  a reader and to `planning/tests/test-comment-format.sh`.
- **Verbatim only.** No prose, no reasoning, no history inside the fence. If a
  line is explaining rather than quoting, it belongs outside — and inside the
  three-line budget.
- Never fence something that is not reproduced somewhere else. A fence around
  invented prose is the loophole this rule exists to close.
- A `Usage:` block in a file docblock is already quoted material by nature and
  needs no fence; it is covered by the 20-line docblock allowance.

The form is deliberately plain ASCII so it survives any editor and is greppable
(`grep -n 'quoted:'`), and it does not collide with heredoc or here-string
syntax the way `<<<` or `>>>` would.

### The one tagged comment format

A portability workaround gets a marker, because without one the next reader
"simplifies" it back into the bug:

```bash
# PORTABILITY(<rule-id>): <one line, why this local code is shaped this way>
```

- `<rule-id>` MUST be an `id` in `portability-rules.json`.
  `planning/tests/test-portability-contract.sh` fails on an unknown id.
- The trailing text is optional, at most three lines, and holds only what is
  *local* to this site. The symptom, the platform and the replacement live in the
  registry — do not repeat them at every site.
- `# PORTABILITY(empty-array-setu)` with no text is a complete marker.
- The untagged `# PORTABILITY:` form is rejected: it cannot be indexed.

`generate-portability.sh` harvests these into `PORTABILITY.md`, so the marker
does double duty — a local warning and the catalogue's index. That is also how
the cross-file information stays out of comments: it is generated, not written.

**The file docblock is the deliberate exception** to the three-line limit. Its
cap is 20 lines **of prose** — a fenced quoted block never counts toward any
comment budget, docblocks included. The cap is about keeping the header
scannable, not about limiting what the file can state: `monitor-read.sh`, `supervision-frame.sh` and
`role-context.sh` print it verbatim as their `--help` via `sed -n '1,20p' "$0"`,
so it is interface, not commentary. Keep it to name, purpose, and `Usage:`.
Numbered decision rules belong there when a script owns a real decision tree, as
`plan-root.sh` does — an agent reading the first 20 lines then knows it.

**Any flow that crosses more than two scripts, or has more than three states,
gets a mermaid diagram** in the maintainer docs (`planning/ARCHITECTURE.md`,
`benchmark/planning/ARCHITECTURE.md`), with a small inline one next to the
prose it explains where that helps. Diagrams live in markdown, never in a
comment block, so they render.

Pick the form by what you are explaining:

| Explaining | Diagram |
|---|---|
| A pipeline with branches and error paths | `flowchart TD` |
| Who calls whom, and what each receives | `sequenceDiagram` |
| Whether a thing may proceed | `stateDiagram-v2` |
| Which file sources or invokes which | `flowchart LR` with `subgraph` per layer |
| Who writes and who reads a file | `flowchart LR`, writer → file → reader |

Keep a diagram at the level of its label: node text names the script or the
artifact, decision diamonds carry the actual condition, and anything needing a
paragraph goes in the prose under it. A diagram that has to be corrected on
every commit is drawn too low.

### Quote every node label and edge label

`A["User request"]`, `B{"Durable plan requested?"}`, `X -->|"no"| Y`. Unquoted
text ends the label at the first character mermaid treats as syntax, so a label
containing `/`, `;`, `(` or `,` is silently truncated — and
`test-mermaid-accuracy.sh` cannot see the node at all, which quietly disables
the undefined-node check for that whole block. Quoting is what makes the
structural check apply rather than merely run.

### A diagram moves with the code it draws

**After changing a flow a diagram covers, re-read that diagram, correct it in
the same change, and say in the report that you did.** Nothing enforces this:
whether an arrow is the real control flow, whether a diamond carries the true
condition, and whether a stage order matches the code are not mechanically
checkable, so a diagram rots exactly the way the `file:line` citations did.

`planning/tests/test-mermaid-accuracy.sh` covers only the mechanical half —
every block parses, and every script, artifact, node id and function a diagram
names exists. It also renders each block with `mmdc` when the dev flake provides
it, and the `mermaid-render` CI job always does. **Rendering proves syntax, not
accuracy**: a diagram can render perfectly and still describe a flow the code
does not have.

---

## 12. Test scripts

Same skeleton, same portability rules. Beyond that:

- One behaviour per test file, named `test-<subject>.sh`, discovered by
  `run-tests.sh` from `planning/tests/` and `benchmark/planning/tests/`.
- Self-contained: build the fixture in a `mktemp -d` under `$TMPDIR`, clean it
  in a `trap … EXIT`. A test that depends on a gitignored `.plans/` tree is a
  test that fails for the next contributor.
- **A failing test must say what failed.** Exiting 1 in silence makes a red
  suite useless: the reader knows something broke and nothing else.
- New tests report every finding, then exit non-zero once, using the helpers in
  `planning/tests/lib-test.sh`: `t_begin`, `t_fail`, `t_assert_eq`,
  `t_assert_contains`, `t_expect_exit`, `t_end`. Findings go to a file, not a
  counter variable, because a helper called inside a `$( )` runs in a subshell
  where an incremented counter is discarded — that made one test's exit-code
  assertions inert until a mutation exposed it.
- A test that prints its own message and prefix — most do, and the prefix says
  which test spoke — keeps that message and calls `t_record` instead of
  incrementing a local counter, reading `t_failures` in its epilogue. A counter
  incremented inside a `$( )` is discarded with the subshell, so the epilogue
  reads zero and reports PASS over a real finding.
- A test written as bare `[ … ]` under `set -e` calls `t_trap_assertions` once,
  which installs an `ERR` trap reporting the line and the failing expression
  verbatim. It aborts at the first failure, which is the price of not rewriting
  the assertions; prefer the reporting helpers for anything new.
- `lib-test.sh` also carries the portability shims (`t_sed_i`,
  `t_sed_insert_before`, `t_stat_mode`, `t_sha256`, `t_unique_suffix`,
  `t_copy_tree`).
- Exit 0 for pass, non-zero for fail. A test that cannot run because an optional
  fixture is absent prints `UNCONFIGURED (<VAR>)` and exits 0 — never a silent
  skip and never a hard failure.
- `! cmd` skips `errexit`. When asserting a failure, capture the status and
  compare it (`rc=0; cmd || rc=$?`), so a crash is not mistaken for the expected
  refusal. Run a subject that may call `exit` in a subshell.
- Every hard rule in this file and in
  `planning/MAINTAINER-STYLE-CONTRACT.md` has a regression assertion. That is
  what keeps them true.

**A zero-match `grep` is not evidence.** It is indistinguishable from a wrong
pattern, so an assertion built on one passes whether the code is right or the
pattern is broken. Before believing a `0`, confirm the pattern matches something
it should — a positive control on a line you know is there. Two checks in this
repo were wrong for exactly this reason: one guard pattern matched nothing, so
its test passed unconditionally, and one merge check reported a data-loss fix
missing when it was present.

Three mechanisms cause it, and all three arise from the same situation —
grepping shell source for literal shell syntax, which is dense in characters
special to the shell, the regex engine, or both:

- **`$` is not reliably literal in a BRE.** `grep -c 'cd "$dir"'` returns 0 on a
  line containing exactly that; `grep -cF` returns 1. Use `grep -F` for a
  literal fragment, which is what most checks want, or escape as `\$`.
- **Double quotes hand the pattern to the shell first.** `grep "$0"` searches for
  the script's name, not the two characters. Single-quote every pattern.
- **`^` assumes column 1.** A function defined inside a conditional is indented,
  so `^name()` misses it.

This one is a review item, not a gate: a double-quoted pattern containing `$` is
sometimes exactly right (`grep "$unit"`), so a detector would cry wolf. It is
enforced by reading the check, and by the positive control above.

**The rule is about probes, not about `grep`.** Anything whose *negative* result
you are about to believe needs a positive control first — a run where it visibly
succeeds. These all failed silently in one session, each read as confirmation:

- a `grep` for `env VAR=x bash` whose pattern could not match the shape it
  described, reported as "clean";
- a mutation that produced **no change to the file**, whose passing test was read
  as the fix being verified — assert the file differs before trusting the result;
- a test probe that planted `declare -A` to prove a scanner pruned foreign
  checkouts, when the scanner lists `# PORTABILITY(...)` markers and not
  constructs, so nothing was detectable either way;
- a mutation applied to the *wrong one* of two call sites, leaving the test green
  and the conclusion wrong.

**Do not parse shell structure with a pattern.** An `awk` range
`/^fail\(\)/,/^}/` runs past a one-line `fail() { …; }` into the next function
and reports *its* `exit`: that is how a count of 6 fail-fast test reporters was
recorded as 18. A function body needs brace matching — or better, ask the
question behaviourally. "Does this reporter hide later findings?" is answered by
breaking one assertion and counting what comes out, and no parse can be wrong
about it.

**A comment naming a banned pattern counts as a use of it.** Every gate here
scans source, so explaining a rule in the words the gate greps for trips the
gate: a comment saying a helper replaced a hand-rolled `.tmp.$$` kept the
duplication count at its old value, and a test that spelled out a
`# PORTABILITY(...)` marker was published in the catalogue as a real sighting.
Describe the pattern without spelling it.

These four are review items like the one above, not gates: a detector for "you
believed a negative result" would have to know what you intended. What *is*
enforced is the consequence — the duplication ratchet, the portability contract
and the manifest test all fail when a scan-based rule is broken, which is how
each of the failures listed here was eventually caught.

**Prefer a registry to a scan.** `portability-rules.json`,
`planning/document-sections.json`, `coupling.tsv` and `planning/PACKAGE-MAP.tsv`
exist because scraping prose or source for structure resolved 29 of 36 sections
and reported success. If a check needs to know something about the code, record
it as data and read it with `rjq` or `cut`; scan only for what a registry cannot
hold.

---

## 13. Checklist before committing

```bash
bash -n <every edited script>
shellcheck -s bash <every edited script>      # 0 new findings
./run-tests.sh                                # 30+ PASS, 0 FAIL
git diff --check
```

Then, for a file added or removed under `planning/`, confirm the four manifest
places in §3 moved together, and that `README.md`'s skills table and
`package.json`'s `files` list still match reality.
