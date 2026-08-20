# MODE: DEV
# PACKAGE: DEV
set -euo pipefail

# Interactive installer for the skills in this repository.
# It is intentionally self-contained so it can be used as:
#   curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/main/install.sh | bash
#
# Staying one file is a hard constraint, not an oversight: the curl|bash form has
# no siblings to source, and download_source() distinguishes a local checkout
# from a download purely by whether ${BASH_SOURCE[0]} is a readable file. So the
# sections below take the place of separate files. Table of contents, in order:
#
#   1.  Configuration and registries
#   2.  CLI-mode argument parsing
#   3.  Interactive input channel
#   4.  Runtime tool verification
#   5.  Agent target detection
#   6.  Terminal capability, splash, and menu rendering
#   6b. Skill picker: state, text metrics, requirement model
#   6c. Skill picker: the sprite and the frame
#   6d. Skill picker: input, the terminal, and the seam
#   7.  Skill and target selection
#   8.  Source acquisition
#   9.  Per-skill file manifest
#   10. CLI-mode handlers
#   11. Interactive install, backup, and merge
#   12. Post-install plan root migration
#   13. Step 2: planning runtime permissions (interactive main path only)
#   14. Main
#
# Usage:
#   install.sh [--all | --skill <name>] [--target <path>] [--yes]
#   install.sh --help
#
# Exit codes: 0 success, 1 any error, plus the machine contract of the
# --install-skill CLI mode: 2 = approval declined, 3 = unsafe collision.

