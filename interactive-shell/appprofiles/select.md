# select (bash builtin)

### Identity
Bash builtin for menu-driven input within shell scripts. Used in a bash script or via `bash -c 'select...'`. Not a standalone program. Syntax: `select var in option1 option2 ...; do ... ; done`. Presents a numbered menu and waits for numeric input.

### Layout
- Numbered list: "1) option1", "2) option2", etc.
- Prompt line -> "#?" waiting for input
- No pager, no screen editing, just a static list and prompt

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| 0-9+ (digits) | Select an option | Type the number (1, 2, 3, etc.) and ENTER |
| RETURN (alone) | Re-display menu | Hitting ENTER without a number redisplays the menu [unconfirmed] |
| CTRL-D / EOF | Exit loop | Terminates select (closes the loop) |
| Literal option text | Select by name | Type the exact option text instead of its number [unconfirmed] |

### Workflows
1. Simple menu: `bash -c 'select opt in one two three; do echo "You picked: $opt"; break; done'`, see numbered list, type 2, press ENTER, outputs "You picked: two", exits.
2. In a script: Include select in a loop to allow repeated choices.

### Quirks
- The menu is reprinted on each invalid input; a valid number selects that option.
- No arrow-key navigation; purely numeric input.
- The special variable `$REPLY` holds the raw user input (not just the selected option); `$opt` in the example holds the chosen option text.
- select does not consume the variable name; after selection, the chosen option is available in the loop body.

### Unconfirmed
- Whether typing the option text (e.g., "one") works instead of a number
- Behavior when input is out of range (negative, too high, letters)
- Whether blank input redisplays or re-prompts
