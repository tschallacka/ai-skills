# read (bash builtin)

### Identity
Bash builtin for reading a line of input from stdin. Not a standalone program — used in scripts or via `bash -c 'read...'`. Syntax: `read [-p "prompt"] [-s] variable_name`. Waits for user input and stores it in the named variable.

### Layout
- Prompt text (if `-p "text"` is given) printed without trailing newline
- Cursor positioned after prompt, waiting for input
- No paging, no screen editing beyond normal terminal line editing

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| Printable characters | Type input | Appear on screen (or hidden if `-s` flag used) |
| BACKSPACE | Delete previous character | Standard terminal editing |
| CTRL-U | Clear line | Erase all input back to prompt |
| RETURN / ENTER | Submit input | Read the line and store in variable |
| CTRL-D / EOF | End of input | Acts like RETURN (may be platform-dependent) |

### Options
| Flag | Effect | Notes |
|------|--------|-------|
| `-p "text"` | Print prompt | Text printed before waiting for input, no trailing newline |
| `-s` | Silent mode | Input is not echoed to screen (for passwords) |
| `-r` | Raw input | Do not interpret backslash escapes |
| `-a array` | Read into array | Each word becomes an array element |
| `-t timeout` | Timeout | Exit if no input after N seconds |

### Workflows
1. Simple prompt: `bash -c 'read -p "Name: " name; echo "Hi $name"'`, shows "Name: " (no newline), user types "Alice" and presses ENTER, outputs "Hi Alice", exits.
2. Silent input (password): `bash -c 'read -s -p "Password: " pwd; echo'`, prompt shown but input is hidden, ENTER submits.
3. Array input: `bash -c 'read -a words; echo ${words[0]}'`, reads one line, splits into array, accesses first word.

### Quirks
- The prompt text is printed with NO trailing newline; user input appears on the same line.
- Without `-p`, read waits silently for input (no visible prompt).
- read consumes one line of input up to RETURN.
- Silent mode (`-s`) hides echoing but may still show cursor movement or other terminal feedback.

### Unconfirmed
- Exact behavior of `read -t timeout` when timeout expires
- Whether `-r` (raw) prevents common escapes like `\n` or only backslash interpretation
- Behavior of EOF (CTRL-D) vs. RETURN (may differ by shell or platform)
