# vi / vim

### Identity
Modal text editor. `vi FILE`. On this system, `vi` is VIM 9.1 (not POSIX
vi/nvi), so vim-only features (visual mode, multi-level undo, window
splits, `:set`) are available; a different system's `vi` may not be vim --
check `vi --version` or the startup screen there.

### Layout
- Body -> buffer, one line per row, `~` marks rows past end-of-file
- Last row -> status/command line: filename + cursor position in Normal
  mode; echoes `:`/`/`/`?` input while typing it; shows error/info messages

### Modes
- **Normal** (start mode): keys are commands/motions, not text
- **Insert**: keys are typed into the buffer; most builds show `-- INSERT
  --` on the status line. Entered via `i`/`a`/`A`/`o`/`O` from Normal;
  exited with ESC back to Normal.
- **Command-line** (`:`, `/`, `?`): entered by typing that char from
  Normal; status line echoes input. ENTER executes, ESC cancels back to
  Normal.

Check the status line before sending the next key -- the same key means
different things in different modes.

### Keys
| Key | Action | Mode | Notes |
|-----|--------|------|-------|
| i | Insert before cursor | Normal->Insert | |
| a | Insert after cursor | Normal->Insert | |
| A | Insert at end of line | Normal->Insert | |
| o / O | Open line below/above, insert | Normal->Insert | |
| ESC | Return to Normal | Insert or Command-line -> Normal | |
| `:w` ENTER | Write | Normal | |
| `:q` ENTER | Quit | Normal | Refuses with unsaved changes |
| `:wq` ENTER / `:x` ENTER | Write and quit | Normal | |
| `:q!` ENTER | Quit, discard changes | Normal | |
| `/text` ENTER | Search forward | Normal | `n`/`N` repeat fwd/back |
| dd | Delete (cut) line | Normal | |
| yy | Yank (copy) line | Normal | |
| p / P | Paste after/before cursor | Normal | |

### Workflows
1. Edit and save: `A` (or `i`/`o`), type change, ESC, `:wq` ENTER. Check
   on-disk content changed.
2. Discard and quit: ESC, `:q!`, ENTER.

### Quirks
None found beyond the standard modal behavior above.
