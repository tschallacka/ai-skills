# iptraf

Not tested this session -- typically requires root or raw socket access for packet capture; the sandbox does not permit this. Real packet capture cannot be attempted. Profile from standard documented behavior only. [unconfirmed] throughout.

### Identity
Network traffic monitor (interactive UI with menus). Available as `iptraf` (legacy) or `iptraf-ng` (newer fork). `iptraf` (starts menu interface), `iptraf -i eth0` (monitor specific interface directly [unconfirmed]).

### Layout (main menu mode)
- Menu-driven interface with options:
  - IP traffic monitor
  - General interface statistics
  - Detailed interface statistics
  - Statistical breakdowns
  - LAN station monitor
  - Filters
  - Configure
- Selection via arrow keys + ENTER, ESC to go back

### Layout (monitoring mode)
- Interface name in header
- Live packet/traffic statistics (flows, protocols, packet counts, byte rates)
- Auto-refreshing display
- ESC/q to exit back to menu

### Keys (menu mode)
| Key | Action | Notes |
|-----|--------|-------|
| Arrow keys | Navigate menu | UP/DOWN to highlight options |
| RETURN | Select option | Activate highlighted menu item |
| ESC | Go back | Return to previous menu or main menu |

### Keys (monitoring mode)
| Key | Action | Notes |
|-----|--------|-------|
| q / ESC | Stop monitoring | Return to main menu |
| f | Filter | Apply traffic filter [unconfirmed] |
| CTRL-X | Exit entirely | Quit iptraf [unconfirmed] |

### Workflows
1. Start menu: `sudo iptraf`, navigate to "IP traffic monitor", select an interface, view live stats, ESC to return to menu, ESC again to quit.
2. Monitor specific interface: `sudo iptraf -i eth0` (starts monitoring directly, skip menu [unconfirmed]).

### Quirks
- iptraf requires root or raw socket privileges to capture packets; will refuse to run without them.
- Menu-driven design (unlike top/htop which are direct-view).
- Captures real network traffic; must be run with caution on active networks.
- iptraf-ng is the maintained fork; iptraf itself is legacy.

### Unconfirmed
- Exact menu structure and all available options
- Whether `-i` flag skips menu and goes straight to monitoring
- Whether filters can be applied mid-monitoring or only from menu
- Exact packet/byte rate metrics shown
- Whether CTRL-X stops monitoring or exits entirely vs. ESC behavior
- Color coding or highlighting scheme for different protocols [unconfirmed]
