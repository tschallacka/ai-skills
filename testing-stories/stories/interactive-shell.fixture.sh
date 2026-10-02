#!/usr/bin/env bash
# Seeds a small interactive onboarding wizard the interactive-shell testing
# story asks the agent to run. It is deliberately NOT a program the agent
# could have seen before, and it deliberately cannot be driven headlessly:
# every prompt is a blocking `read` after a `clear`, so a plain foreground
# invocation or a piped set of answers either hangs waiting on a real
# terminal or silently consumes the wrong prompt.
set -euo pipefail

mkdir -p tools

cat > tools/onboarding-wizard.sh <<'WIZARD'
#!/usr/bin/env bash
set -euo pipefail

clear
echo "=================================="
echo "  Welcome to the project wizard"
echo "=================================="
echo
read -r -p "What's your name? " wizard_name

clear
echo "=================================="
echo "  Pick your team"
echo "=================================="
echo
echo "  1) Frontend"
echo "  2) Backend"
echo "  3) Infra"
echo
wizard_team=""
while [ -z "$wizard_team" ]; do
    read -r -p "Enter a number (1-3): " choice
    case "$choice" in
        1) wizard_team="frontend" ;;
        2) wizard_team="backend" ;;
        3) wizard_team="infra" ;;
        *) echo "Please enter 1, 2, or 3." ;;
    esac
done

clear
echo "=================================="
echo "  Confirm"
echo "=================================="
echo
echo "  Name: $wizard_name"
echo "  Team: $wizard_team"
echo
read -r -p "Look right? (y/n) " confirm
if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "Setup cancelled. Run this again to retry."
    exit 1
fi

cat > workspace-config.json <<CONFIG
{
  "name": "$wizard_name",
  "team": "$wizard_team"
}
CONFIG

clear
echo "=================================="
echo "  Setup complete!"
echo "=================================="
echo
echo "Wrote workspace-config.json"
WIZARD

chmod +x tools/onboarding-wizard.sh
