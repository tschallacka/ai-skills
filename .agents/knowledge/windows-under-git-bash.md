<!-- MODE: DEV -->
# Windows under Git for Windows' bash: what breaks, and why

Nothing that passes on Linux says anything about Windows, and the failures do
not look like Windows failures: a bash script that is correct fails on its
first line, a test that is correct compares two spellings of one path, a
socket that works sits in the wrong error branch. Every row below cost at least
one CI run to find. The rules that follow from them are in
`.agents/MAINTAINER.md` section 1.16.

Measured 2026-09-21 on GitHub's `windows-latest` runner (Git for Windows' bash,
`x86_64-pc-windows-msvc`, Rust 1.98), by making the whole workspace and
the whole shell suite pass on a dedicated branch and then merging it. At that
point `cargo test --workspace --all-targets` ran 186 test binaries and the
shell suite ran 247 tests.

## Symptom, cause, what the repository does

| symptom | cause | what the repo does |
|---|---|---|
| every test that spawns `bash script.sh` fails, none for a reason in the script | `bash` on the runner's PATH resolves to `C:\Windows\System32\bash.exe`, the WSL launcher, which exits with an error when no distro is installed | Git's `bin` and `usr\bin` are put first on PATH, in every job that runs bash |
| `set -euo pipefail` errors on line 1; awk and cut keep a trailing CR on the last field; byte-for-byte fixtures stop matching | Git for Windows checks text files out as CRLF (`core.autocrlf=true`); bash reads the option as `pipefail<CR>` | `.gitattributes` pins `eol=lf` for the text types, and CI sets `core.autocrlf false` before checkout |
| a checkout fails on a path | `benchmark/results/` holds archived paths longer than Windows allows | the sparse checkout is `/*` minus `!/benchmark/results/`, non-cone |
| a test spawns a "command" that does not exist | a Windows program needs `.exe`, and a shebang script is not an executable there | `planning_core::exe_name`, `EXE_SUFFIX`, and `tests/rust-support/script_stub.rs` (a compiled shim that runs `bash <sibling script>`) with the shell twin `planning/tests/lib-script-stub.sh` |
| a path compares unequal to itself | bash says `/d/a/x`, a native tool says `C:/Users/x` or `C:\Users\x`, and a Rust `join` produces a mix | tests compare against `t_native_path` (`cygpath -m`) and `t_slashes`; `canonicalize` output has its `\\?\` prefix stripped before it is compared or handed to bash |
| JSON built with `printf` is invalid; `sha256sum` cannot find a file | a path carrying backslashes is an escape sequence to both | environment variables hold `C:/...`: `run-tests` hands scripts `PLANNING_AGENT_TMPDIR` with forward slashes, and `t_begin` converts it with `cygpath -m` |
| `chmod` seems to do nothing; `stat` reports 644 or 755 whatever was set | NTFS has no unix permission bits | assertions on mode are skipped on Windows |
| a test about symlinks sees a plain copy | under MSYS `ln -s` copies the target unless `MSYS=winsymlinks:nativestrict` is set, and a real link needs a privilege the runner may not grant | `t_enable_symlinks` asks the filesystem and the test skips that part when it says no |
| `ps -o state=` fails | Windows has no `ps` that takes `-o` | liveness is `Child::try_wait` |
| a `SIGHUP` test cannot be written the same way | there is no SIGHUP | `verify-both-shells` installs a console control handler (Ctrl-C, Ctrl-Break, window close) that cleans up and exits with the code a shell reports for that signal |
| a socket read times out and the code treats it as an error | a read timeout is `ErrorKind::TimedOut` on Windows and `WouldBlock` on unix | `chat-client-rs`'s `net::is_timeout` matches both |
| a killed chat client's next send arrives as `nick-2` | a killed peer sends a reset, not a FIN; `read_tls` returned `Err(ConnectionReset)`, the loop counted only `Ok(0)` as closed, so the dead connection stayed open and kept its nick | the chat server ends a connection on `Ok(0)` and on `ConnectionReset`, `ConnectionAborted`, `BrokenPipe`, `NotConnected` or `UnexpectedEof` |
| a server that binds a Unix socket cannot run | there is no Unix socket to bind | `planning-server`'s `transport.rs` binds a Unix socket where there is one, and loopback TCP with a per-start nonce in a discovery file where there is not |

## Fixing one platform's error kind can break another

Ending a connection on **any** read error, to fix the killed-peer row, broke
the macOS `chat-mcp` join with "unexpected end of file" (run 35545991115, commit
`1a7f4c2f`, job 106171959866, "x86_64-apple-darwin builds and runs every subject
in scope", failing
`a_registered_trigger_wakes_wait_on_a_message_with_no_mention_at_all` in
`src/chat-mcp/tests/mcp_flow.rs` with `join refused: "send: unexpected end of
file"`): a read error that is
not a lost peer is not the end of the connection. The list above is the fix,
not "any error". A change made for one platform's error kind has to be
re-checked on the others.

## What it means here

- Windows behaviour is only ever established by a CI leg. `windows.yml` on the
  `windows` branch is a shortcut **only with `.github/windows-focus.txt`**, which
  makes it run one thing: measured 2026-09-21, a focused run (run 35543660777,
  commit `c0f5a598`) took 1 min 15 s, while an unfocused one (run 35543977213,
  commit `1a7f4c2f`, workspace tests plus the unsharded shell suite) took
  37 min 30 s. That is not faster than the whole main workflow: `ci.yml` runs on
  `nextupdate` took 26 min 20 s (run 35563751715, `f650bf56`), 29 min 29 s (run
  35570621053, `48a09027`) and 39 min 45 s (run 35573178188, `a3a5df90`), each
  including the four Windows shell-suite shards. These are wall-clock times from
  each run's `createdAt` to `updatedAt` (`gh run view <id> --json
  createdAt,updatedAt`), so they include time spent waiting for a runner.
- When a Windows-only failure appears, look first for a spelling difference
  (path, line ending, exit code, error kind) between what the test expects and
  what the platform produces, before suspecting the code under test.
