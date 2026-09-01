---
name: interactive-shell
description: Operate unknown full-screen and interactive terminal programs through a PTY-backed wrapper, compact screen observations, and a local Unix-socket input client.
---

# Interactive shell

Use this skill when an agent must discover and operate a full-screen or
interactive program such as nano, Midnight Commander, Lynx, a pager, a shell,
or a terminal menu. This is an agent-driven interface: never replace the
interaction with a replayed shell script or assume that a program has the
shortcuts used in an example.

Read [README.md](README.md) for the command reference and an end-to-end
workflow. Start
`interactive-shell --session <ID> --cols 80 --rows 24 --idle-timeout 300 -- <COMMAND>`
or `interactive-shell --socket <SOCKET> --cols 80 --rows 24 --idle-timeout 300 -- <COMMAND>`
and observe its JSONL stdout. Use the smallest practical `--cols` and `--rows`
to preserve agent context; start around 80x24 or smaller when the program
allows it, and resize only when the UI needs more space. Use `view [<ROW>...]`
for a compact plain-text
screen view labeled with terminal rows and rendered column spans; use
`view-delta [<ROW>...]` for only rows changed since the previous view request.
Compact view responses also include the current `cols` and `rows`, so an agent
can recognize that a title or path may be clipped and request a larger resize.
Use `rgbview [<ROW>...]` or `rgbview-delta [<ROW>...]` when ANSI foreground,
background, bold, or reverse styling is useful; these still omit JSON but emit
terminal SGR sequences for a color-capable consumer. `view` is preferable for
minimum token cost.
For an unknown interface, identify the current focus, visible labels, selection
state, and available navigation controls before acting. Prefer visible UI
elements and keyboard navigation, including TAB to move focus and arrows or
page keys to move within a pane. Ask the application's built-in help or a
manpage when the screen does not explain an operation. Re-observe after each
action and branch on what is actually shown.
Use `observe` when you need structured JSON for cursor state, styles, scrollback,
or all screen metadata. Use `elements [<ROW>...]` when you need only verified
actionable elements and their labels/coordinates; row ranges such as `10-15`
are accepted. Send one request at a time with
`interactive-shell-input --socket <SOCKET> locate '<TEXT>'` to find visible text
and receive 1-based row/column matches; it does not type, navigate, or assert
that the match is clickable. Send one request at a time with
`interactive-shell-input --socket <SOCKET> text '<TEXT>'`, `key <KEY>`, `combo <KEY> [CTRL] [ALT] [SHIFT]`, `paste '<TEXT>'`,
`mouse <X> <Y> <BUTTON> down|up|move`, `resize <COLS> <ROWS>`, `view [<ROW>...]` (rows may be `10-15`), `view-delta [<ROW>...]`, `rgbview [<ROW>...]`, `rgbview-delta [<ROW>...]`, `elements [<ROW>...]`, `observe`, `wait '<TEXT>' [<TIMEOUT_MS>]`, `raw <HEX>`, or
`click-id <ID> <BUTTON>`, `click-label '<LABEL>' <BUTTON>`, `click-at <X> <Y> <BUTTON>`, or
`shutdown`.

Angle-bracketed words are placeholders, not literal arguments. For example,
`text 'hello world'` sends one payload containing a space. Mouse coordinates are
1-based; button `0` is the primary button and buttons `1` through `7` are the
remaining protocol button codes.

`--session <ID>` stores the socket, dimensions, timeout, and command in a
private session file. Later wrapper starts and every input operation can use
`--session <ID>` without repeating those arguments. `--agent <ID>` is an alias
that selects an agent-keyed session; when omitted, the wrapper checks
`INTERACTIVE_SHELL_AGENT`, `CODEX_AGENT_ID`, and `AGENT_ID` in that order.
Session files live below `$INTERACTIVE_SHELL_HOME`, or below the private
`$XDG_RUNTIME_DIR/interactive-shell` directory when that variable is set.

Screen events contain only rows changed since the previous event, a monotonically
increasing `seq`, the preceding `base`, and the cursor. An `observe` request
returns a `snapshot` event containing every current row, followed by an
acknowledgement; use it to resynchronize after missed deltas. `wait TEXT`
waits up to the optional timeout (default 30 seconds) for visible screen text,
publishes intervening deltas, and returns a full snapshot with `matched` true
or false. Snapshot and screen
events also contain `elements`: OSC 8 hyperlinks appear as elements with an id,
label, URI, row, column, width, `actionable`, `highlighted`, and optional
`highlight_source`; visible non-whitespace
text runs are reported as `actionable: false` coordinate hints for TUIs that
expose no semantic metadata. Use `click-id` or `click-label` only for
actionable elements; use `click-at` when the TUI exposes only a
coordinate-based action. Text targets are hints, not proof that a TUI will
respond to a mouse click. `highlighted: true` means the text run is reverse-video
or a row-level color outlier relative to adjacent rows; `highlight_source`
identifies the heuristic as `reverse-video` or `color-outlier`. It is a heuristic
selection hint for TUIs such as mc, not a guarantee of application focus and
may also flag decorative regions such as scrollbars. Parse those JSON fields;
do not grep terminal output or acknowledgements. An acknowledgement means only
that input was accepted by the wrapper, not that the program completed the
resulting action. Wait for a screen predicate or lifecycle event after every
state-changing request. Function-key bytes can be correct while an application
still treats them as text when a modal field or unfocused pane owns input; use
the next screen to verify interpretation. The idle timeout measures PTY output
inactivity, not agent input or a pending `wait` request.

In file managers, typing a filename may enter the application's command line
instead of searching the active pane. Discover the pane's own search or
navigation control from the current screen, built-in help, or its manpage; do
not assume that typing a visible filename selects it.

Named keys include ENTER, CTRL-A through CTRL-Z, ALT-graphic keys, BACKSPACE,
TAB, ESC, META-LEFT, META-RIGHT, UP, DOWN, LEFT, RIGHT, HOME, END, PAGEUP, PAGEDOWN, INSERT,
DELETE, CTRL-PAGEUP, CTRL-PAGEDOWN, CTRL-INSERT, CTRL-DELETE, SHIFT/CTRL
cursor variants, and F1 through F12. Use `raw` for an
explicit byte sequence, encoded as non-empty even-length hexadecimal. Use
`combo` when the needed modifier combination is not in the named-key list;
for example, `combo h ctrl alt shift` sends Alt followed by Ctrl-H, while
named cursor/function keys use xterm modifier encoding.

`paste` sends bracketed paste delimiters around the text. `mouse` uses 1-based
coordinates and sends an xterm mouse event. `resize` changes both the PTY and
the screen model, and rejects dimensions outside the wrapper bounds.

`combo` also accepts PAGEUP, PAGEDOWN, INSERT, and DELETE. For example,
`combo PAGEUP ctrl` sends the standard Ctrl-PageUp sequence; this is useful
when the current application documents a modified navigation key that is not
listed as a standalone named key.

Do not treat a successful socket acknowledgement as proof that the application
accepted the key. A screen delta, changed selection, prompt, or lifecycle event
is the evidence for the next decision. When output is large, request a row
range or a delta instead of repeatedly asking for the whole screen. Use
`rgbview` only when color is semantically useful; it preserves SGR while still
avoiding the JSON metadata overhead.

The wrapper allocates a real PTY, applies the requested dimensions, and owns a
0600 socket inside a private 0700 directory. It emits a final lifecycle event
for child exit, client shutdown, or idle timeout and removes only the socket
inode it created. A child process group is terminated during cleanup.

Screen events expose `styles` as row-based spans for basic ANSI colors, bold,
and reverse video, plus bounded `scrollback`. These observations are useful for
navigation but do not claim complete terminal emulation.

The screen model stores PTY text as terminal bytes rather than full Unicode
cells; non-ASCII glyph widths and coordinates may therefore be approximate.
The screen model covers common cursor movement, erase, visibility, OSC,
alternate-screen, charset, line-drawing, scroll-region, and insert/delete-line
controls, including sequences split across PTY reads. Every ncurses extension
remains a possible unsupported control. For mc or another TUI, use predicates
based only on represented controls, or extend and test the model before relying
on a new control.
