# git-add-patch

### Identity

Interactive patch staging tool for selectively adding changes to the index. `git add -p`, `git add --patch`, `git add -i` (interactive mode), etc. Interactive workflow for staging individual hunks (pieces) of modified files.

### Invocation

^git\s+add\s+.*(-p\b|--patch\b)
^git\s+add\s+.*(-i\b|--interactive\b)

### Dialogs

#### Hunk staging prompt
- **Triggered by**: `git add -p` when a file has unstaged changes
- **Display**: Shows the diff hunk (unified diff format)
- **Prompt**: `(N/M) Stage this hunk [y,n,q,a,d,s,e,p,P,?]?`
  - N/M indicates current hunk number and total
- **Options**:
  - `y` - stage this hunk
  - `n` - skip this hunk
  - `q` - quit; stage nothing more
  - `a` - stage this and all remaining hunks
  - `d` - discard this hunk (do not stage)
  - `s` - split this hunk into smaller parts
  - `e` - edit this hunk manually (opens in `$EDITOR`)
  - `p` - show this hunk again
  - `P` - show all hunks again
  - `?` - show help for options

### Workflows

1. **Stage selected hunks from a file**:
   - Run: `git add -p`
   - For each hunk, press `y` to stage or `n` to skip
   - Continue through all hunks; unstaged hunks remain in working directory

2. **Split a large hunk into smaller ones**:
   - When presented with a hunk, press `s`
   - Git re-splits the hunk if possible (not all hunks can be split further)
   - Re-prompted for the smaller pieces

3. **Edit a hunk manually**:
   - Press `e` to open the hunk in your text editor
   - Remove lines starting with `-` to keep the deletion, or remove both +/- to discard
   - Save and quit editor; git applies the edited hunk

4. **Stage all remaining hunks**:
   - Press `a` to skip interactive confirmation for remaining hunks

### Quirks

- Hunk splitting may not be possible if the hunk contains only one logical change
- `s` (split) attempts to divide the hunk at line boundaries; not always successful
- Editing a hunk (`e`) requires `$EDITOR` to be set; falls back to vi if not
- Exit code 0 if any hunks were staged; 1 if none were staged or if interrupted
- `git add -i` offers an additional menu with options like `patch` (same as `add -p`), `status`, `diff`, etc.

### Unconfirmed

- Exact behavior when hunk cannot be split further [unconfirmed]
- Editor integration details (what editors are supported) [unconfirmed]
- Behavior with binary files or complex diffs [unconfirmed]
