# Development

This repository contains portable coding-agent skills and a shell installer.
Skills remain at the repository root; do not move them into a separate package
directory.

## Repository layout

- `planning/` — durable planning skill and helper scripts.
- `project-specificies/` — project-deviation skill and example note files.
- `resource-limited-testing/` — resource-limiting guidance and wrapper.
- `install.sh` — interactive and non-interactive skill installer. **Generated**:
  edit `installer/src/` and run `./installer/build.sh`.
- `installer/` — the installer's source parts, the build script, and
  `tools.tsv`, the shared registry of how to verify and install each
  runtime tool. Each skill's own `requires.tsv` says what it needs.
- `package.json` — npm package metadata and the `ai-skills-install` binary.

Each skill directory contains a `SKILL.md` with YAML frontmatter. Supporting
scripts and references should stay inside the skill directory that uses them.

## Adding or changing a skill

1. Create or update the skill directory at the repository root.
2. Add a valid `SKILL.md` with a unique `name` and a precise `description`.
3. Document when the skill should and should not be used.
4. Add the skill to the installer parts if it is new, then run
   `./installer/build.sh`:
   - `SKILL_NAMES` and `SKILL_DESCRIPTIONS` in `installer/src/05-config.sh`
   - `skill_files()` in `installer/src/50-manifest.sh`
   - a `<skill>/requires.tsv`, even when the skill needs nothing — the empty
     table is the statement that it has no runtime dependencies
   The shop menu and numeric selection derive from `SKILL_NAMES`, so they need
   no separate edit.
5. Add it to the skills table in `README.md` and the npm `files` list in
   `package.json`.

Keep skill instructions portable across supported agent tools. Avoid adding
runtime dependencies unless the skill genuinely needs them.

## Shell contract

Every shell file here targets bash 3.2 on macOS, bash 4/5 on Linux, and GNU
*or* BSD userland — macOS `/bin/bash` is the floor, so bash 4 syntax and
GNU-only utility flags are out. `PORTABILITY.md` (generated) catalogues the traps already hit.
`CODE-STYLE.md` is the authority: it lists the
banned constructs with their replacements, the file skeleton, the exit-code
vocabulary, and the pre-commit checklist. CI enforces it by running the suite on
`ubuntu-latest` and `macos-latest` (including explicitly under system bash 3.2)
plus a `shellcheck` pass.

## Testing the installer

Check shell syntax and formatting before committing:

```bash
./installer/build.sh --check                          # install.sh matches its parts
bash -n install.sh installer/build.sh installer/src/*.sh
bash -n planning/scripts/*.sh
bash -n resource-limited-testing/scripts/limited-run.sh
shellcheck -s bash install.sh planning/scripts/*.sh   # no new findings
./run-tests.sh                                        # all 30 tests
git diff --check
```

`./run-tests.sh` runs every test under `planning/tests/` and
`benchmark/planning/tests/`; run it rather than naming individual test scripts.
Two context-cache tests report `UNCONFIGURED` without `PLANNING_CONTEXT_CACHE`,
which is expected.

Show installer options without making changes:

```bash
AI_SKILLS_NO_SPLASH=1 ./install.sh --help
```

For an isolated non-interactive install, use a temporary target:

```bash
target="$(mktemp -d)"
AI_SKILLS_NO_SPLASH=1 ./install.sh --all --target "$target" --yes
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

The package contents should include `install.sh`, `package.json`, `README.md`,
`LICENSE`, and every skill directory. The `ai-skills-install` binary must point
to the existing `install.sh`; do not duplicate the installer in JavaScript or
move the skills to satisfy npm packaging.

The generated artifacts the package ships — the five compiled plan libraries
and `planning/REVIEWER.md` — are built by `npm prepack` from the tracked
sources, never committed (`planning/MAINTAINER.md` §2.15). A pack from a clean
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
  fragments as commands (B17, the `been: command not found` ghost).
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

Then publish the exact package version and create the corresponding GitHub
release:

Review the dry-run package contents, then publish from a clean checkout with
the appropriate npm credentials:

```bash
npm_config_cache="$(mktemp -d)" npm pack --dry-run
npm publish --access public
gh release create v<version> --title "<version>" --generate-notes
```

Publishing is an external release action. Confirm the version, package name,
and included files before running `npm publish`.

## Commits and review

Before opening a pull request:

- inspect `git diff` and `git status`;
- run the shell and package checks above, including `shellcheck -s bash` on
  every edited script;
- verify README examples match the current installer options;
- confirm no generated archives, npm cache files, or temporary targets were
  added to the repository.
