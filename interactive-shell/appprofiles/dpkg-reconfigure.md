# dpkg-reconfigure

### Identity
Reconfigure an installed Debian package by running its postinst configuration script. `dpkg-reconfigure [options] PACKAGE`. Requires root privileges.

### Layout
- Body -> package-specific dialog (whiptail or dialog-based UI, varies per package) [unconfirmed]
- Dialog elements -> checkboxes, radio buttons, text input fields, buttons (OK, Cancel) [unconfirmed]

### Modes
Modal dialogs presented by the package's postinst script [unconfirmed]. Common UI frameworks: whiptail/dialog with keyboard navigation.

### Keys
| Key | Action | Notes |
|-----|--------|-------|
| Arrow keys UP/DOWN | Navigate between items | [unconfirmed] |
| SPACE | Toggle checkbox | [unconfirmed] |
| TAB | Move focus between list and buttons | [unconfirmed] |
| ENTER | Activate focused button | [unconfirmed] |
| ESC | Cancel/close dialog | [unconfirmed] |

### Dialogs
Package-specific dialogs [unconfirmed]. Examples [unconfirmed]:
- Checklist: "Which options do you want?" with multiple checkbox items, [OK] [Cancel] buttons. Navigation: arrow keys move between items, SPACE toggles, TAB moves to buttons, ENTER activates button.
- Radio list: "Select one option" with radio buttons. Navigation: arrow keys, SPACE selects, TAB/ENTER to confirm.

### Workflows
1. Reconfigure a package: run `sudo dpkg-reconfigure -p high PACKAGE` (e.g., `console-setup`). Observe interactive dialog. Press ESC or select Cancel before any final confirmation to abort without saving changes. Verify system configuration did not change (e.g., `date` for timezone, or `localectl` for keyboard).
2. Reconfigure with terse mode: run `sudo dpkg-reconfigure --terse PACKAGE`. Observe simplified (or noninteractive) dialog [unconfirmed].

### Quirks
- Requires root privileges [unconfirmed]
- Dialog UI framework and content vary per package [unconfirmed]
- Final confirmation usually happens on a separate dialog; ESC/Cancel on intermediate dialogs may not prevent the change if you've already confirmed on a later screen [unconfirmed]
- Some packages offer a `--unseen-only` flag to reconfigure only options that have not yet been set [unconfirmed]

### Unconfirmed
- All interactive behavior: keyboard navigation, key names, dialog flow [unconfirmed]
- Whether the dialog is whiptail, dialog, or another framework [unconfirmed]
- Exact button labels and navigation order [unconfirmed]
- Behavior when running as non-root (error or sudo prompt) [unconfirmed]
- Whether intermediate dialogs can be cancelled without affecting already-changed settings [unconfirmed]
