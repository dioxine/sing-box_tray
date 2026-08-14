#!/bin/bash

# 1. Get the current username
CURRENT_USER=$(whoami)
# Target rules file path
RULES_FILE="/etc/sudoers.d/singbox_tray_rules"

echo "Configuring passwordless sudo for user: $CURRENT_USER"

# 2. Generate the security rule line
RULE_LINE="$CURRENT_USER ALL=(root) NOPASSWD: /usr/local/bin/sing-box, /usr/bin/killall"

# 3. Write the rule to a temporary file for syntax validation
TMP_FILE=$(mktemp)
echo "$RULE_LINE" > "$TMP_FILE"

echo "Validating rule syntax using visudo..."
if sudo visudo -c -f "$TMP_FILE" > /dev/null 2>&1; then
    echo "Syntax is valid. Applying configuration..."
    
    # Create the target directory if it does not exist
    sudo mkdir -p /etc/sudoers.d
    
    # Copy the verified temporary file to the final destination
    sudo cp "$TMP_FILE" "$RULES_FILE"
    
    # macOS requires strict 0440 (read-only) permissions for files in sudoers.d
    sudo chmod 0440 "$RULES_FILE"
    
    echo "SUCCESS: sing-box and killall can now run without a password prompt."
else
    echo "ERROR: The generated rule has an invalid syntax!"
    echo "Changes were not applied to keep your system safe."
fi

# Clean up the temporary file
rm -f "$TMP_FILE"

