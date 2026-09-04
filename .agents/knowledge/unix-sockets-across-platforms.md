<!-- MODE: DEV -->
# Unix sockets: what does not port from Linux

Measured 2026-09-03 while diagnosing `interactive-shell`'s macOS test failures.

**`/proc/self/fd/N` is a directory you can path through. `/dev/fd/N` is not.**
That single difference is what broke seventeen of nineteen tests on both macOS
legs while every Linux and Windows leg passed.

## The Linuxism

Binding a socket "safely" — so nothing can swap the directory underneath you
between the check and the bind — is usually written by holding the parent
directory open and binding *through* the fd:

```rust
#[cfg(target_os = "linux")]  fn fd_path(fd) -> "/proc/self/fd/{fd}"
#[cfg(target_os = "macos")]  fn fd_path(fd) -> "/dev/fd/{fd}"     // WRONG
UnixListener::bind(fd_path(parent_fd).join("socket"))
```

On Linux `/proc/self/fd/N` is a magic **symlink to the directory**, so
appending `/socket` traverses into it. On macOS `/dev/fd/N` comes from the
`fdesc` filesystem and stands in for the **open file itself** — it is not a
traversable directory entry, so `/dev/fd/7/socket` does not resolve and the
bind fails `ENOENT`.

Swapping the prefix per platform *looks* like porting and is not. The two
paths are not the same kind of object.

## What ports

`fchdir()` into the held directory fd, bind the **relative** name, restore the
previous directory and check the restore succeeded. The fd still pins the
directory, so the race the original code was guarding against is still closed,
and it works identically on both platforms.

It also retires the length problem below **for that call**, because `sun_path`
becomes six bytes instead of an absolute path. Only for that call: the same
treatment is needed on connect, which is the next section.

Two cautions, both real:

- The current directory is **process-global**. It is safe here only because
  each wrapper is its own process; `thread::spawn` appears nowhere in the
  crate. In a threaded program this needs serialising or doing in the child
  before `exec`.
- Restore *before* spawning anything. A child inherits the cwd, and a command
  built from caller-relative arguments (`nano file.txt`) resolves them against
  whatever directory it inherits.

## The length limit, which bit the other half of the same socket

`sun_path` is **104 bytes on macOS** and 108 on Linux. macOS's `$TMPDIR` is
`/var/folders/<2>/<~30 chars>/T/`, about 49 characters before anything of yours
is added, where Linux gets `/tmp`.

**Both directions of a paired operation need the fd-relative treatment.** The
bind was fixed first and the cap was ruled out for it — correctly: the longest
bind path budgeted to ~98 of 104, one test created and removed a socket
successfully, and short-label tests failed too, so length did not separate the
failures. All true, and all about the **bind**.

The client still connected by an *absolute* path, so once the bind stopped
failing first, macOS refused the connect with `path must be shorter than
SUN_LEN`. Half a fix moves the failure rather than removing it.

The durable rule is about testing, not sockets: **a mechanism test covering one
direction of a paired operation lets the other half through.** The test asserted
`local_addr()` was `Some("socket")` — that the bind was relative — and said
nothing about connect. Give each direction its own leg, each with a control
asserting the absolute form fails first, or the untested half ships.

`verify-both-shells.sh` gives each test `/tmp/t.XXXXX` for this reason;
chromium's socket path caps near the same figure.

## There is no short blessed socket directory on macOS

Worth knowing before reaching for one:

- **`$TMPDIR`** is the correct private scratch — per-user, per-session, already
  mode 0700, unlike `/tmp`. Its only fault is length.
- **launchd-managed sockets** (a `Sockets` key in a plist, the fd passed in)
  are the native IPC answer, and are a daemon contract — useless for a CLI or
  a test.
- **`$XDG_RUNTIME_DIR`** does not exist on macOS at all.
- `bindat()` is FreeBSD-only, so there is no syscall that avoids the cwd.

## How this was found, and the trap in finding it

The tests ran the wrapper with **`.stderr(Stdio::null())`**, so seventeen
failures reported assertion text and never the reason. It was diagnosed by
guesswork twice — once as a timeout, once as path length — before the stderr
was captured to a file per test directory. A harness that cannot say why
invites invention; that cost more than the bug.

The confirmation route, absent a Mac: point `fd_path` at a non-directory on
Linux and check the fingerprint matches — same 17 failures, same 2 survivors,
same `ENOENT`. That is confirmation by controlled intervention, not
observation, and the distinction is worth keeping in mind: the original Darwin
errno was never captured, because pushing the fix cancelled the queued
diagnostics run.
