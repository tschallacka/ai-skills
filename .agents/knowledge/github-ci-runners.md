<!-- MODE: DEV -->
# GitHub's runners: what this repository has measured about them

Measured 2026-09-04, mostly while diagnosing `interactive-shell`'s macOS legs.
Every figure here comes from a run's own log, named so it can be re-read.

## macos-latest is arm64, and has been since macos-14

    Image: macos-26-arm64
    GNU bash, version 3.2.57(1)-release (arm64-apple-darwin25)

Both macOS legs — the default-bash one and the bash 3.2 portability floor — run
that image (run 33871455553). **Do not reason about "the Intel macOS runner"**:
there is not one on `macos-latest`. `x86_64-apple-darwin` appears in this repo
only as a *cross-compilation target*, built on the same arm64 host, so a leg
named for it is not evidence of an Intel machine.

## The macOS runner is ~150x slower than a developer machine, not 2x

The `interactive-shell` protocol suite, same 19 tests, same commit:

| where | wall clock |
|---|---|
| this workstation | **1.05 s** |
| aarch64-apple-darwin CI leg | **151.94 s** |

It is a shared, oversubscribed VPS that pauses for other tenants. Two
consequences, both learned by paying for them:

1. **Any wall-clock number in a test is a bet, and this runner wins it.** A
   fixture parked on `sleep 30` outlived every assertion locally and died
   mid-test on CI, taking its socket with it — the failure then reads as a
   missing socket rather than an expired fixture. Give a fixture a lifetime
   with room for the whole test, not for the fast case.
2. **A readiness budget is a ceiling, not a sleep.** A loop that returns the
   moment its condition holds costs a healthy run nothing, so 30 s of budget is
   free until something is actually broken. `READY_POLLS = 3000` at a 10 ms
   interval exists for this runner, not for this laptop.

### A duration equal to the budget is a timeout, not slow work

Worth reading a wall clock for, because it separates "this runner is slow" from
"this test is broken" before any log is opened:

| run | wall clock | what it means |
|---|---|---|
| working | **1.05 s** | every wait returned on its condition |
| broken | **30.26 s** | every wait spun its full 3000 polls and then failed |
| broken, with a 35 s stall injected | **65.64 s** | 35 s of stall **plus** the whole 30 s budget |

A healthy run never approaches the budget, so a figure that lands *on* it is a
loop that gave up rather than a machine under load. The corollary is the
cheaper one: after a fix, the same test finishing in **seconds** rather than
paying the budget is itself evidence the condition is now being met, not merely
that the assertion stopped firing.

## $TMPDIR is long enough to break Unix sockets

    /var/folders/<2 chars>/<28 chars>/T/

About 49 bytes before anything of yours is added, where Linux gives `/tmp`.
`run-tests.sh` adds its own per-run scratch directory and each test adds one
more, which put a socket address at **110 bytes against `sun_path`'s 104-byte
cap on macOS**. See `unix-sockets-across-platforms.md`; the short version is
that both `bind` and `connect` must address the socket by name from inside its
directory, and so must any test harness that talks to it.

This is reproducible on Linux without a Mac: pad `$TMPDIR` to ~72 bytes and the
macOS failure appears exactly, same count, same tests.

## The RAM cap in limited-run.sh cannot be enforced on these runners

`resource-limited-testing`'s wrapper caps memory with a transient systemd scope
on Linux and with [`memlimit`](https://github.com/pingiun/memlimit) on macOS.
On GitHub's macOS runners **neither arch can enforce it**, for two different
reasons:

- **Intel**: `memlimit` publishes one artifact, `memlimit-aarch64-apple-darwin`,
  so there is nothing to install.
- **arm64**: it installs, and then cannot run. It works by
  `DYLD_INSERT_LIBRARIES`, and macOS **SIP strips that variable for system
  binaries**; its own installer says so —

        memlimit: warning: no non-SIP shell found (looked for a bash or zsh

  and its hook library is built for the wrong ABI besides:

        dyld: terminating because inserted dylib '.../hook-*.dylib' could not
        be loaded: (mach-o file, but is an incompatible architecture
        (have 'arm64', need 'arm64e'))

So on macOS CI the cap is **advisory**, the suite prints
`Warning: the requested RAM limit (2G) is not enforced.`, and the tests run
uncapped. That is a platform limitation, not a missing step, so the workflow
installs `memlimit` on arm64 and then reports a failed capability probe as a
`::notice` rather than failing the leg. Do not "fix" it by deleting the install
or by asserting the cap works.

## nano's title bar is not a readiness predicate on macOS

nano fits the version, the filename and the modified flag into the terminal
width and **drops the version first**. With macOS's long `$TMPDIR` an absolute
filename ran to ~90 characters in an 80-column terminal, so `GNU nano` was
never on screen and a `wait_screen "GNU nano"` timed out while nano ran
perfectly, shortcut bar and all. Open the file by its **bare name** from inside
its directory rather than widening the predicate — widening it would hide that
the assertion depended on the temp directory's length.

## Reading a failing run's logs

`gh run view` refuses while a run is in progress. The per-job API does not, so
a failing leg can be read without waiting for its siblings or cancelling
anything:

    gh api --allow-escape-sequences \
      "repos/<owner>/<repo>/actions/jobs/<job-id>/logs"

`--allow-escape-sequences` is required — `gh` otherwise refuses the body
because CI logs carry terminal colour codes. Strip them with
`tr -d '\r' | sed -e 's/\x1b\[[0-9;]*[a-zA-Z]//g'`.

## Do not push while a run is queued

The workflow's concurrency group cancels the older run, and registration on
these runners takes ~15 minutes, so a push inside that window destroys the
result you were waiting for. It has cost this repository a Darwin errno it then
had to re-derive by controlled intervention, and both macOS suite legs twice
over. Batch the work, or wait.
