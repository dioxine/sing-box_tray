#!/bin/bash

RULES_FILE="/etc/sudoers.d/singbox_tray_rules"
PLIST_FILE="$HOME/Library/LaunchAgents/com.user.singboxtray.plist"

echo "=========================================="
echo "    SingBoxTray Uninstallation Script     "
echo "=========================================="
echo ""

# --- PART 1: Remove LaunchAgent ---
echo "[1/2] Removing LaunchAgent..."
if [ -f "$PLIST_FILE" ]; then
    launchctl unload "$PLIST_FILE" > /dev/null 2>&1
    rm -f "$PLIST_FILE"
    echo "  -> SUCCESS: LaunchAgent removed."
else
    echo "  -> LaunchAgent not found. Skipping."
fi

# --- PART 2: Remove Sudoers rules ---
echo ""
echo "[2/2] Removing passwordless sudo rules..."
if [ -f "$RULES_FILE" ]; then
    echo "Authentication required to remove sudo rules."
    sudo rm -f "$RULES_FILE"
    echo "  -> SUCCESS: Sudo rules removed."
else
    echo "  -> Sudo rules not found. Skipping."
fi

echo ""
echo "=========================================="
echo " Uninstallation complete! You can now     "
echo " delete SingBoxTray.app from Applications."
echo "=========================================="
echo ""
read -p "Press [Enter] to close this window..."

