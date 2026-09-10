# bluetoothctl

BlueZ interactive management console for Bluetooth devices. `bluetoothctl`, etc.

### Identity

Interactive REPL for managing Bluetooth adapters, devices, and connections.

### Layout

- Prompt -> `[bluetooth]#` (or similar; may vary based on version/state)
- Command input -> accepts BlueZ/bluetoothctl commands
- Output -> device list, connection status, or error messages

### Keys

| Key | Action | Notes |
|-----|--------|-------|
| ENTER | Execute current command | Sends command to BlueZ daemon |
| CTRL-D | Exit bluetoothctl | Closes the REPL session |
| UP / DOWN | Navigate command history | If readline support is compiled in |
| help | Show available commands | Lists all bluetoothctl commands |
| quit / exit | Exit the REPL | Same as CTRL-D |

### Workflows

1. **List Bluetooth adapters**:
   - Type: `list`
   - ENTER

2. **Show default adapter info**:
   - Type: `show`
   - ENTER

3. **Scan for devices** (requires hardware and proper permissions):
   - Type: `scan on`
   - ENTER
   - Devices appear as discovered; `scan off` to stop

4. **Pair with a device** (requires hardware):
   - Type: `pair <MAC_ADDRESS>`
   - ENTER
   - May prompt for PIN or confirmation

5. **Connect to a device**:
   - Type: `connect <MAC_ADDRESS>`
   - ENTER

6. **Exit**:
   - Type: `quit` or `exit` or press CTRL-D

### Quirks

- Bluetooth adapter must be present and powered on for most commands (e.g., `scan`, `pair`, `connect`)
- `list` and `show` work even without an adapter (report "No default controller available" or similar)
- Commands like `scan`, `pair`, `connect` require elevated privileges (typically root or `bluetooth` group membership)
- Pairing and connection status persists across sessions (Bluetooth state is system-wide)
- Device discovery may take several seconds; `scan on` does not return immediately

### Unconfirmed

- Exact error messages and behavior without a Bluetooth adapter [unconfirmed]
- PIN entry and authentication flow details [unconfirmed]
- Device trust and pairing state management [unconfirmed]
