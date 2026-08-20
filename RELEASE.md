<!-- MODE: DEV -->
# Release protocol

A release ships a **prod package**: the files an end user needs and nothing else.
The maintainer's files — the per-function library sources, the 70 test scripts and
their fixtures, the compiler, the architecture and maintainer documentation — stay
in the repository. They are not bloat to a maintainer and they are nothing but
bloat to a user, so the two are separated by declaration rather than by judgement.

## What decides whether a file ships

Every file says so in its own header:

| marker | meaning |
|---|---|
| `MODE: PROD` | the end user receives this file. Exclusive: if a release needs it, it has to say PROD |
| `MODE: DEV` | only a maintainer needs it. Inclusive: the dev side has every PROD file as well |
| `PACKAGE: PROD` | a compiler input, compiled into the end-user artifact |
| `PACKAGE: DEV` | a compiler input for the dev build only, which carries the dev and prod inputs together |

`PACKAGE` appears only on what a compiler reads. A compiled artifact carries
`MODE` alone, because that axis belongs to inputs.

Two independent lists have to agree, and neither is derived from the other:
the marker in each file, and `skill_files()` in `installer/src/50-manifest.sh`.
`tests/test-mode-markers.sh` fails on any disagreement. That duplication is the
cross-check, the same arrangement `planning/PACKAGE-MANIFEST.txt` has.

## Cutting a release

1. **Be on a clean tree, with the branch merged and tests green on both shells.**
   `./run-tests.sh` runs every suite; bash 3.2 matters as much as the local bash,
   because stock macOS ships 3.2 and the code targets it.

2. **Bump the version** in `package.json`. Nothing else records it: the installer
   reads it with `sed` (jq may not be installed yet) and stamps it into each
   skill's `.version` file.

3. **Ship a schema for the new version.** Both register skills need
   `schema.<version>.json` beside their `SKILL.md`, and the new one needs an
   `upgrade_from` entry for the version it supersedes — there is no backwards
   compatibility, so an agent holding an older file runs that recipe or rewrites
   the file. `tests/test-register-schemas.sh` fails if the schema is missing, and
   runs the upgrade recipes to prove they work.

4. **Regenerate every generated artifact and confirm each is fresh.** Each has a
   `--check` that fails rather than silently rebuilding:

   ```sh
   ./installer/build.sh --check                    # install.sh
   ./planning/scripts/build-plan-libs.sh --check   # the four plan-*-lib.sh
   ./generate-portability.sh --check               # PORTABILITY.md
   ./blast-radius.sh                               # every coupling in coupling.tsv
   ```

   `build-plan-libs.sh --check` always compares against a **prod** build, so a dev
   build left in the tree reads as stale. That is deliberate: it is what stops a
   dev library, with its provenance comments and its dev-only helpers, being
   committed or released by accident.

5. **Build the package.**

   ```sh
   ./installer/build-release.sh              # dist/ai-skills-<version>.tar.gz
   ./installer/build-release.sh --list       # what will be in it, one path per line
   ```

   The tarball is assembled from the markers, so there is no third list to keep in
   step. Read `--list` before tagging: it is the last point at which a file that
   should not ship is cheap to catch.

6. **Regenerate `.npmignore`** so `npm publish` excludes the same set:

   ```sh
   ./installer/build-release.sh --npmignore > .npmignore
   ```

   `package.json`'s `files` array ships whole skill directories, which would
   otherwise publish the maintainer's files. `npm pack --dry-run` lists what would
   go; it should match `--list`.

7. **Tag and push**, then attach the tarball to the GitHub release. The tag is
   what the installer resolves, so it must exist before the asset is useful.

8. **Verify the published article, not the local one.** Install from the tag into a
   scratch directory and confirm no maintainer file arrived:

   ```sh
   curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/<tag>/install.sh \
     | bash -s -- --all --target /tmp/release-check
   find /tmp/release-check -path '*/tests/*' -o -path '*/scripts/lib/*' | head
   ```

   That `find` should print nothing.

## The two packages

```sh
install.sh --all                    # prod: what an end user needs
install.sh --all --package dev      # prod plus the maintainer's files
```

`--package dev` exists so a contributor can install a working development copy
anywhere: the per-function library sources, the compiler, every test and fixture,
`ARCHITECTURE.md` and `MAINTAINER.md`. For the planning skill that is 241 files
against 85 in a prod install.

`prod` is the default deliberately. The one-line install is the common path and
must never quietly deliver a maintainer's tree.

## What a release must never contain

- a dev build of any `plan-*-lib.sh` (step 4 catches it)
- a file marked `MODE: DEV` (the marker gate and `--list` both catch it)
- an artifact stale against its source (each `--check` catches it)
- a register schema absent for the version being released (step 3 catches it)

## Adding a file between releases

Mark it. A file with no `MODE` marker fails `tests/test-mode-markers.sh`, which
is the point: the decision about who receives a file is made when the file is
written, by whoever knows the answer, and not at release time by whoever is
holding the tag.

Then register it in `skill_files()` — the prod arm if it ships, the dev arm if it
does not — and in `planning/PACKAGE-MANIFEST.txt` and `PACKAGE-MAP.tsv` for a
planning file. `tests/test-skill-files-manifest.sh` asserts the two arms account
for every tracked file, so a file in neither is a failure rather than a silent
omission.
