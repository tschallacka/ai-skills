# git-subtree

Non-interactive Git subcommand for managing subtrees (embedded repositories). `git subtree add/pull/push/split --prefix=DIR REPO BRANCH`, etc.

### Identity

Entirely non-interactive command for integrating external repositories as subdirectories.

### Quirks

- `git subtree` is non-interactive; no prompts or interactive features
- Commands complete or error without further user input
- `split` can take significant time on large repositories (generates new commits)
- Merge commits are created by `add` and `pull`; can clutter history
- No progress bar; long operations show no feedback until completion

### Unconfirmed

- Performance and history characteristics with very large subtrees [unconfirmed]
