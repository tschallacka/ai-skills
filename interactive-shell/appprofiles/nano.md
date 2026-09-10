# nano

### Identity
Modeless text editor, common default `$EDITOR`. `nano FILE`.
`nano --ignorercfiles FILE` runs with no user config.

### Layout
- Row 1 -> title bar: version, filename (or `New Buffer`), `*` at right edge
  when modified
- Body -> buffer
- Second-to-last row -> status/prompt line (search text, save-path prompt,
  yes/no question)
- Last two rows -> help bar, `^`-prefixed shortcuts in a fixed grid, contents
  vary by context

### Modes
Not modal -- typed characters always insert. Prompts (save filename, search,
yes/no/cancel) take the status line until answered; ENTER accepts, ESC (or
the prompt's own cancel key) aborts.

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| CTRL-O | Write Out (save) | Prompts for a filename, defaults to current; ENTER accepts default |
| CTRL-X | Exit | Prompts "Save modified buffer?" if unsaved changes |
| CTRL-K | Cut line | |
| CTRL-U | Paste (uncut) | |
| CTRL-W | Where Is (search) | |
| CTRL-C | Show cursor position | Does NOT exit or cancel |
| F-keys (F1-F12) | Not reliably bound | F2 does not trigger Write Out; use CTRL-O/CTRL-X, not F-keys |

### Dialogs
- **Save prompt** (CTRL-O): status line becomes "File Name to Write:" with
  the current path pre-filled. ENTER accepts, ESC cancels.
- **Exit-with-unsaved-changes** (CTRL-X with modified buffer): "Save
  modified buffer?" -- answer keys shown in the same prompt (commonly
  Y/N), read the actual prompt rather than assuming a fixed key.

### Workflows
1. Edit and save: type the change, CTRL-O, check "File Name to Write:"
   appeared, ENTER, check on-disk content changed.
2. Exit without losing changes: CTRL-X; answer the "Save modified buffer?"
   prompt per its own shown keys.

### Quirks
- F2 does not open Write Out; it triggers behavior consistent with an
  Exit-adjacent binding (a "Save modified buffer?" dialog appears). Use
  CTRL-O, not F2.
- CTRL-O/ENTER save and CTRL-X exit work identically driven through mc
  (mc.md).

### Unconfirmed
- Exact F-key bindings beyond F1/F2 (F1 opens Help viewer; F2 opens "Save modified buffer?" when buffer is modified, not Write Out)
