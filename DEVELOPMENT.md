# Development

This repository contains portable coding-agent skills and a shell installer.
Skills remain at the repository root; do not move them into a separate package
directory.

## Repository layout

- `planning/` — durable planning skill and helper scripts.
- `project-specifics/` — project-deviation skill and example note files.
- `resource-limited-testing/` — resource-limiting guidance and wrapper.
- `src/installer/` — the compiled Rust installer: interactive picker,
  headless `install`/`install-skill` subcommands, MCP/permission registration.
- `installer/` — `bootstrap.sh` (the curl-piped entry point that downloads
  a release and hands off to the binary above), `build-release.sh` (packs a
  release tarball), and `tools.tsv`, the shared registry of how to verify and
  install each runtime tool (`include_str!`'d into the compiled binary). Each
  skill's own `requires.tsv` says what it needs. `installer/src/05-config.sh`
  and `installer/src/50-manifest.sh` are the two surviving fragments of the
  retired bash install.sh — see git history — still sourced by
  `build-release.sh` for the skill list and file manifest.
- `package.json` — npm package metadata and the `ai-skills-install` binary.

Each skill directory contains a `SKILL.md` with YAML frontmatter. Supporting
scripts and references should stay inside the skill directory that uses them.

## Adding or changing a skill

1. Create or update the skill directory at the repository root.
2. Add a valid `SKILL.md` with a unique `name` and a precise `description`.
3. Document when the skill should and should not be used.
4. Register the skill if it is new:
   - `SKILL_NAMES` and `SKILL_DESCRIPTIONS` in `installer/src/05-config.sh`
   - `skill_files()` in `installer/src/50-manifest.sh`
   - a `<skill>/requires.tsv`, even when the skill needs nothing — the empty
     table is the statement that it has no runtime dependencies
   The picker's list and numeric selection derive from `SKILL_NAMES`, so they
   need no separate edit.
5. Add it to the skills table in `README.md` and the npm `files` list in
   `package.json`.

Keep skill instructions portable across supported agent tools. Avoid adding
runtime dependencies unless the skill genuinely needs them.

## Shell contract

Every shell file here targets bash 3.2 on macOS, bash 4/5 on Linux, and GNU
*or* BSD userland — macOS `/bin/bash` is the floor, so bash 4 syntax and
GNU-only utility flags are out. Git for Windows' bash is a supported floor too
(`CODE-STYLE.md` section 1). `PORTABILITY.md` (generated) catalogues the traps already hit.
`CODE-STYLE.md` is the authority: it lists the
banned constructs with their replacements, the file skeleton, the exit-code
vocabulary, and the pre-commit checklist. What CI runs to enforce it, on which
platforms, is in `.agents/MAINTAINER.md` section 3.

## Testing the installer

Check shell syntax and formatting before committing:

```bash
cargo build --release -p installer
cargo test -p installer
cargo clippy -p installer --all-targets -- -D warnings
bash -n installer/bootstrap.sh installer/build-release.sh installer/src/*.sh
bash -n planning/scripts/*.sh
bash -n resource-limited-testing/scripts/limited-run.sh
shellcheck -s bash installer/bootstrap.sh planning/scripts/*.sh   # no new findings
./run-tests.sh                                        # all bash suites
git diff --check
```

`./run-tests.sh` runs every shell test suite in the repository and `cargo test`
for each crate (`./run-tests.sh --list-only` names them); run it rather than
naming individual test scripts. It needs the compiled runner, so run
`./setup-dev-env.sh` first (`.agents/MAINTAINER.md` 1.9). Two context-cache tests
report `UNCONFIGURED` without `PLANNING_CONTEXT_CACHE`, which is expected.

Show installer options without making changes:

```bash
./target/release/installer --help
```

For an isolated non-interactive install from the checkout, use a temporary
target:

```bash
target="$(mktemp -d)"
./target/release/installer install --all --source . --target "$target" --yes
find "$target" -maxdepth 2 -name SKILL.md -print
```

Review the temporary target before removing it. Do not test against a real
agent skill root unless replacement behavior is specifically being verified.

## Testing the npm package

The npm package deliberately does not install skills during `npm install`.
Installation is explicit through the exposed binary:

```bash
npm_config_cache="$(mktemp -d)" npm pack --dry-run --json
npm run install-skills -- --help
```

The package contents should include `installer/bootstrap.sh`, `package.json`,
`README.md`, `LICENSE`, and every skill directory. The `ai-skills-install`
binary must point to `installer/bootstrap.sh`, which downloads the matching
compiled installer release on first run; do not duplicate the installer in
JavaScript or move the skills to satisfy npm packaging.

The generated artifacts the package ships — the five compiled plan libraries
and `planning/REVIEWER.md` — are built by `npm prepack` from the tracked
sources, never committed (`.agents/MAINTAINER.md` 1.10). A pack from a clean
checkout is therefore complete without any generated file in git.

## Verifying on both shells

`./verify-both-shells.sh` runs the whole suite twice — local bash and the bash
3.2 floor via the dev flake — in a detached worktree under `TMPDIR`, so it
verifies what is in front of you without blocking edits here. The macOS legs
of CI are blocking, so this is also where a BSD-only failure first shows.

- One verification at a time: two concurrent runs used to sweep each other's
  worktree away mid-run (B16). `--keep` preserves the logs and worktree of a
  red run; everything else cleans up after itself.
- Never rewrite `verify-both-shells.sh` while a run is in flight. Bash reads
  its source incrementally, so an edit lands mid-parse and executes comment
  fragments as commands (B17, the `been: command not found` ghost). This is a
  bash-fallback-path hazard specifically: once `setup-dev-env.sh` has staged
  the compiled `verify-both-shells` binary (T145 goal 19), a run through it
  reads no script source at all, so a concurrent edit to the `.sh` file
  cannot land mid-parse there.
- Editing any other file during a run is fine: the worktree is overlaid once,
  at startup, from the then-current tree — later edits belong to the next run.
- A failure that only exists on macOS cannot be reproduced on Linux. Diagnose
  it by pushing and reading the leg's output, not by reading code — which only
  works if tests can speak: never redirect a setup command to `/dev/null`
  under `set -euo pipefail`, because `set -e` kills the test there with its
  diagnosis already discarded (B31).

## Versioning and publishing

Use semantic versioning for releases:

- Increase the patch version for bug fixes and documentation-only corrections.
- Increase the minor version for backwards-compatible new features or new
  skills.
- Increase the major version for breaking changes, including removing skills
  or wholesale altering the behavior or instructions of existing skills.

For each release, update `package.json`, run the validation and npm package
checks above, commit the changes, and create a matching annotated git tag:

```bash
git add README.md DEVELOPMENT.md package.json
git commit -m "Prepare <version> release"
git tag -a v<version> -m "Release <version>"
git push origin master --follow-tags
```

Then cut the GitHub release; `RELEASE.md`'s own protocol (tag and push, create
the release as a draft with the universal tarball, run `release-installer.yml`
to attach the per-platform assets and publish it) is the one to follow, not a
shortened version here -- it exists specifically because a published-then-
attach ordering hits GitHub's immutable-releases protection (T70/W06).

Publishing to npm is no longer a local step run by hand. The moment the
release goes public, `.github/workflows/release-npm.yml` reacts to that same
`release: published` event on its own: it builds every shipping skill for all
five targets, assembles and dry-run verifies the npm package, then runs `npm
publish` -- gated behind the `npm-publish` environment's required-reviewer
approval. Review the dry-run package contents beforehand with
`npm_config_cache="$(mktemp -d)" npm pack --dry-run`, or trigger
`release-npm.yml` via `workflow_dispatch` for the same dry run without cutting
a release at all; running `npm publish` locally is only for reproducing a
packaging problem, never the release path.

## Commits and review

Before opening a pull request:

- inspect `git diff` and `git status`;
- run the shell and package checks above, including `shellcheck -s bash` on
  every edited script;
- verify README examples match the current installer options;
- confirm no generated archives, npm cache files, or temporary targets were
  added to the repository.
