# Development

This repository contains portable coding-agent skills and a shell installer.
Skills remain at the repository root; do not move them into a separate package
directory.

## Repository layout

- `planning/` — durable planning skill and helper scripts.
- `project-specificies/` — project-deviation skill and example note files.
- `resource-limited-testing/` — resource-limiting guidance and wrapper.
- `install.sh` — interactive and non-interactive skill installer.
- `package.json` — npm package metadata and the `ai-skills-install` binary.

Each skill directory contains a `SKILL.md` with YAML frontmatter. Supporting
scripts and references should stay inside the skill directory that uses them.

## Adding or changing a skill

1. Create or update the skill directory at the repository root.
2. Add a valid `SKILL.md` with a unique `name` and a precise `description`.
3. Document when the skill should and should not be used.
4. Add the skill to `install.sh` if it is new:
   - `SKILL_NAMES`
   - the interactive menu in `show_shop_menu`
   - numeric selection handling in `select_skills`
   - the copy/install logic if it uses an explicit allowlist
5. Add it to the skills table in `README.md` and the npm `files` list in
   `package.json`.

Keep skill instructions portable across supported agent tools. Avoid adding
runtime dependencies unless the skill genuinely needs them.

## Testing the installer

Check shell syntax and formatting before committing:

```bash
bash -n install.sh
bash -n planning/scripts/*.sh
bash -n resource-limited-testing/scripts/limited-run.sh
git diff --check
```

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

## Publishing

Update the version in `package.json` according to the intended release, review
the dry-run package contents, and then publish from a clean checkout with the
appropriate npm credentials:

```bash
npm_config_cache="$(mktemp -d)" npm pack --dry-run
npm publish --access public
```

Publishing is an external release action. Confirm the version, package name,
and included files before running `npm publish`.

## Commits and review

Before opening a pull request:

- inspect `git diff` and `git status`;
- run the shell and package checks above;
- verify README examples match the current installer options;
- confirm no generated archives, npm cache files, or temporary targets were
  added to the repository.
