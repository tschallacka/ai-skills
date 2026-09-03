# Rust development guidelines

How we set up a Rust binary in this house, and the order to do it in. The
reference implementation is `magequery` (cresset-tools): cargo-dist +
release-please + cosign keyless signing, six shipped targets. Where this
document and that repository disagree, that repository is probably right and
this document is stale — it was derived from it.

## 0. First decide which kind of binary this is

Two kinds, and almost every later decision follows from the answer.

- **A dev tool.** Contributors run it; users never see it. It needs none of the
  release machinery below — a pinned toolchain, clippy, and tests are enough.
- **A shipped artifact.** It lands on a user's machine. It needs the whole
  chain: pinned toolchain, target matrix, reproducible release, signing, and a
  declaration of what ships.

Rust itself is a **dev** dependency in both cases. `cargo` belongs in the dev
shell next to `shellcheck`; a user of a shipped binary needs no toolchain, no
interpreter, and no package manager. This is why shipping a static binary
*lowers* a project's dependency budget rather than raising it — see
`CODE-STYLE.md`, "Shipped runtime".

## 1. Pin the toolchain before writing code

`rust-toolchain.toml` at the repo root:

```toml
[toolchain]
channel = "1.96"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

Pin a concrete minor, not `stable`. An unpinned toolchain means CI and every
contributor compile with whatever they happen to have, and a lint that fails
only on the newest compiler becomes someone else's surprise. Bump it in its own
commit so a regression is bisectable.

## 2. Layout: split the library from the binary early

For anything beyond a single file, use a workspace and split the crate:

```
crates/<name>-core/     the logic, no argv, no process exit, no printing
crates/<name>-cli/      argv parsing, output, exit codes — a thin shell
```

`magequery` does this (`magequery-core`, `magequery-cli`, `magequery-lsp`), and
the payoff is that the core is testable without a process and reusable by a
second front end — an LSP, a second binary, a benchmark. Retrofitting the split
after the logic has grown into `main.rs` is the expensive version.

Put shared dependency versions in `[workspace.dependencies]` so two crates
cannot drift onto two versions of the same library.

## 2b. Ignore the build artifacts before the first `git add`

Two patterns, in the **repository-root** `.gitignore`, cover Cargo's workspace
layout and the directory-add safety net:

```gitignore
# Rust build artifacts for the workspace (CODE-STYLE 1b).
# Never tracked: they are per-platform, large, and rebuilt by CI.
/target/
# Defensive coverage for a standalone crate or a directory add.
src/*/target/
```

Both patterns live at the repository root, not in per-crate `.gitignore` files.
The first covers every crate because the workspace writes one shared target
directory. The second is belt-and-braces coverage for a standalone crate or a
directory add before it has joined the workspace; it prevents the `tony-the-pony`
incident below from staging build output. A per-crate `.gitignore` would
describe a layout Cargo does not use and could leave the shared output exposed.

This is recorded because it was walked into, not predicted. `src/tony-the-pony`
was copied in from a standalone repository whose ignore rule (`/target`) was not
carried across, and a single `git add src/tony-the-pony` staged **19 files: the 6
sources plus 13 build artifacts**, including the compiled binary and the cargo
lock files under `target/release`. Nothing warned; `git add` of a directory is
silent about what it sweeps in.

Mind the exact patterns. `/target/` matches only the workspace output directory
at the repository root, while `src/*/target/` protects crate-shaped directories
under `src/`. A bare `target` would also match an unrelated file or directory
of that name anywhere in the tree.

Check it landed rather than assuming, since the failure is silent:

```bash
git add src/<crate> && git ls-files src/<crate>
```

The listing must show sources only. If `target/` appears, the pattern is wrong
or the file was already tracked — and `.gitignore` does not untrack anything
already in the index.

---

## 3. Release profile

```toml
[profile.release]
lto = "thin"
strip = true

[profile.dist]
inherits = "release"
lto = "thin"
```

`strip` and LTO because these binaries are downloaded, sometimes on metered
connections, and size is the only cost a user pays for a feature they do not
use. `thin` over `fat` LTO: nearly the same result for a fraction of the build
time.

The `dist` profile is not optional if you use cargo-dist — it builds with
`--profile dist` and fails if the profile is absent. `dist init` normally adds
it; if you hand-write the config, add it yourself.

## 4. Targets

The house list, and why each one:

| Target | Serves |
|---|---|
| `x86_64-unknown-linux-musl` | any x86-64 Linux, any glibc vintage, Alpine included |
| `aarch64-unknown-linux-musl` | Graviton, Ampere, arm64 devcontainers, 64-bit Pi |
| `x86_64-apple-darwin` | Intel macOS |
| `aarch64-apple-darwin` | Apple Silicon macOS |
| `x86_64-pc-windows-msvc` | Windows, and Git Bash / MSYS2 / Cygwin on it |

Rules that cost time to rediscover:

- **Prefer musl to gnu for Linux** when the crate has no C dependencies. One
  static binary then runs everywhere and you ship one Linux row per
  architecture instead of two. `magequery` ships gnu *and* musl only because it
  links a bundled MySQL C client; that is the exception, not the default.
- **Cross-built `aarch64-apple-darwin` must be signed.** Apple Silicon refuses
  to exec an unsigned arm64 Mach-O. Building on a Mac ad-hoc signs it for you;
  cross-compiling from Linux does not, so add an ad-hoc signing step
  (`rcodesign sign`) — no Apple certificate and no account required.
- **Notarization is a different problem from signing** and usually not ours.
  `com.apple.quarantine` is set by browsers and quarantine-aware downloaders,
  not by `npm`/`tar` extraction, so a binary arriving inside a package is not
  quarantined.

  Do not take the previous two bullets on trust: `tony-the-pony`'s
  `.github/workflows/ci.yml` has a `macos-signing` job that answers all of it
  empirically on a real runner — what the local linker signs a build with,
  whether `curl` sets the quarantine attribute, whether an explicitly
  quarantined CLI binary still runs, whether stripping the signature stops
  execution on that arch, and whether `codesign --sign -` restores it. Read
  that job's output before designing a macOS distribution story; it is the
  house's measured answer rather than a remembered one.
- **Do not ship tier-3 targets** (`x86_64-pc-cygwin` and friends). Tier 3 has
  no prebuilt `std`, so it needs a `build-std` step and gets no upstream CI
  guarantee. Check whether an existing row already serves those users first —
  Cygwin runs the `windows-msvc` binary.
- **`aarch64-unknown-linux-gnu` does not cross-compile from x86 under
  cargo-dist.** Give it a native runner (`ubuntu-22.04-arm`).

## 5. Release pipeline

Division of labour that works, from `magequery`:

- **release-please** owns versioning and the changelog. It publishes the GitHub
  Release and creates the tag.
- **cargo-dist** only builds binaries and attaches them to that release:
  `create-release = false`, and it is triggered by `release: published`.
- **cosign keyless (Sigstore)** signs each archive into a `.sig` bundle
  alongside a `.sha256` sidecar, as a post-announce job.

Three traps, all of which have already bitten:

1. `dist generate` rewrites `.github/workflows/release.yml` **and resets its
   trigger** to `on: push: tags`. release-please creates tags via the API,
   which does not reliably fire `push`, so the trigger must be hand-edited back
   to `on: release: published` after every regenerate. Declare
   `allow-dirty = ["ci"]` so `generate --check` does not abort on your edit.
2. Pin `cargo-dist-version` in the config. An unpinned dist silently changes
   the generated workflow under you.
3. Order the post-announce jobs so nothing publishes before signing completes.
   A release whose signing failed must never reach a package index.

## 6. Declaring what ships (this repo specifically)

A binary shipped by an `ai-skills` skill is declared in that skill's
`binaries.tsv` — one row per (target triple, binary): triple, host condition,
filename, why. A skill may ship several binaries for one target (for example a
server and a client), each with its own row; the installer resolves the set for
the host's platform. The rules:

- `binaries.tsv` is `MODE: PROD` (the installer reads it on the target) and
  must appear in `skill_files()` in `installer/src/50-manifest.sh`.
- Rust **source** is `MODE: DEV`, so it never ships.
- The artifact lives at `<skill>/bin/<target triple>/<binary>` and is exempt
  from the marker rule, because a Mach-O, ELF or PE file has no comment syntax.
  The exemption is keyed on the triple-shaped directory name in
  `tests/test-mode-markers.sh`.
- `tests/test-shipped-binaries.sh` validates the registry and cross-checks it
  against what is actually on disk. A declared-but-unbuilt row is legal; a
  built-but-undeclared binary is not.
- Every target's binary ships in the npm package, because we do not know who or
  what pulls it.
- `install.sh` is **generated**. Never hand-edit it; run `bash
  installer/build.sh` and confirm the diff is only what you intended.

## 7. CI gates, in the order they should fail

`tony-the-pony`'s CI is also the model for proving a *distribution* claim
rather than asserting it: it runs the built binary directly instead of through
`cargo run`, because the whole claim is that the binary needs no runtime, and it
greps `ldd` output to prove the musl build is actually static.

1. `cargo fmt --check` — cheapest, most mechanical.
2. `cargo clippy --all-targets -- -D warnings`. Warnings must be errors in CI
   or they accumulate until nobody reads the output.
3. `cargo test --workspace`.
4. Per-target build matrix.

The planning-command CI matrix also publishes `RUST-ARTIFACTS.tsv` and
`SHA256SUMS` with the target bundles. Run the Rust checks and release tooling
inside `nix develop .#default`; that shell supplies the pinned Cargo toolchain
used by CI. From a downloaded bundle, verify the recorded bytes with:

```bash
sha256sum -c SHA256SUMS
```

## 8. Verification standard

The same rule as the rest of this repository, and it is not negotiable: **a
test that passes with and without your change proves nothing.** For every fix,
name the mutation that fails without it, run it, and report the observed
failure. `tests/test-shipped-binaries.sh` was built this way — eleven
mutations, each shown to trip its own assertion with its own message.

Two corollaries specific to Rust:

- A `#[test]` that only asserts a function returns `Ok(())` is a compile check
  wearing a test's clothes. Assert the value.
- Prefer one test that fault-injects over three that restate the happy path.
