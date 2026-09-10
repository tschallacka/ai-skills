# git-bisect

Interactive binary search tool for finding the commit that introduced a bug or change. `git bisect start`, `git bisect good`, `git bisect bad`, etc.

### Identity

Sequential CLI workflow (not a curses TUI) for narrowing down a problematic commit via binary search.

### Workflows

1. **Start a bisection**:
   - Run: `git bisect start`
   - Repo is placed in bisect mode; HEAD is not yet checked out

2. **Mark the current (broken) state**:
   - Run: `git bisect bad` (or `git bisect bad <commit>` to mark a specific commit)

3. **Mark a known good state**:
   - Run: `git bisect good <commit>` (e.g., a release tag or earlier working commit)
   - Git checks out the midpoint commit between good and bad

4. **Test the checked-out commit**:
   - Run your test/reproduction steps
   - Determine if this commit is good or bad

5. **Mark result and continue**:
   - Run: `git bisect good` if the current commit is good, or `git bisect bad` if it's bad
   - Git checks out the next midpoint; repeat step 4

6. **Bisection completes**:
   - After sufficient narrowing, git reports the culprit: `<SHA> is the first bad commit`
   - Shows the commit's details

7. **Exit bisection**:
   - Run: `git bisect reset` to return to the original branch
   - Cleans up bisection state

### Keys / Input

- No special keys during bisection; each step is a separate command
- Output is plain text (not full-screen TUI); all info is shown in stdout

### Quirks

- Bisection assumes linear history; behavior with merge commits can be complex
- If the bisection cannot narrow down further (e.g., all intermediate commits have the same issue), git reports ambiguity
- `git bisect run <script>` automates the testing: script returns 0 for good, non-zero for bad, 125 to skip
- `git bisect skip` can mark a commit as untestable (not counted in the binary search)
- You must explicitly `git bisect reset` to exit bisection mode; leaving it mid-session doesn't automatically clean up

### Unconfirmed

- Exact behavior with very deep histories or complex branching [unconfirmed]
- Performance characteristics on large repositories [unconfirmed]
- Interaction with rebase and reflog during bisection [unconfirmed]
