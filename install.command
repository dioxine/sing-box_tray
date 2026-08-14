#!/bin/bash

CURRENT_USER=$(whoami)
RULES_FILE="/etc/sudoers.d/singbox_tray_rules"
PLIST_FILE="$HOME/Library/LaunchAgents/com.user.singboxtray.plist"

echo "=========================================="
echo "    SingBoxTray Installation Script       "
echo "=========================================="
echo ""

# --- PART 1: Sudoers Configuration ---
echo "[1/2] Configuring passwordless sudo for: $CURRENT_USER..."

RULE_LINE="$CURRENT_USER ALL=(root) NOPASSWD: /usr/local/bin/sing-box, /usr/bin/killall"
TMP_FILE=$(mktemp)
echo "$RULE_LINE" > "$TMP_FILE"

if sudo visudo -c -f "$TMP_FILE" > /dev/null 2>&1; then
    sudo mkdir -p /etc/sudoers.d
    sudo cp "$TMP_FILE" "$RULES_FILE"
    sudo chmod 0440 "$RULES_FILE"
    echo "  -> SUCCESS: Sudo rules installed."
else
    echo "  -> ERROR: Invalid sudoers syntax. Sudo rules skipped!"
fi
rm -f "$TMP_FILE"

# --- PART 2: LaunchAgent configuration ---
echo ""
echo "[2/2] Configuring LaunchAgent (Auto-start on boot)..."

# Ensure the LaunchAgents directory exists
mkdir -p "$HOME/Library/LaunchAgents"

# Create the plist file contents
cat << EOF > "$PLIST_FILE"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://apple.com">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.user.singboxtray</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/open</string>
        <string>/Applications/SingBoxTray.app</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
EOF

# Safe reload using launchctl
launchctl unload "$PLIST_FILE" > /dev/null 2>&1
if launchctl load "$PLIST_FILE"; then
    echo "  -> SUCCESS: LaunchAgent registered successfully."
else
    echo "  -> ERROR: Failed to load LaunchAgent via launchctl."
fi

echo ""
echo "=========================================="
echo " Setup complete! Please ensure you have   "
echo " moved SingBoxTray.app to /Applications/  "
echo "=========================================="
echo ""
read -p "Press [Enter] to close this window..."

