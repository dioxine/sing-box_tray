# SingBoxTray Setup Guide

This application monitors the `sing-box` process state and allows you to control it directly from your macOS Menu Bar.

## Expected System Paths

For the application to function correctly, your backend files must be placed at these exact paths:

1. **Sing-box Binary:**
   `/usr/local/bin/sing-box`

2. **Configuration File:**
   `~/.config/sing-box/config.json` 
   *(where `~` is your user's home directory, e.g., `/Users/your_username/`)*

## First-Time Installation

Because `sing-box` requires root privileges to create a TUN network interface, macOS normally asks for your password every time you click **Start** or **Stop**. 

To bypass password prompts and configure auto-start, follow these steps:

1. Drag **`SingBoxTray.app`** into your system **Applications** folder.
2. Double-click the **`install.command`** script included in this DMG disk.
3. Provide your administrator password once when prompted by the Terminal window.

### What the Install Script Does:
* Grants `SingBoxTray.app` permission to run `sing-box` and `killall` silently without password prompts.
* Registers a system LaunchAgent to automatically start the Menu Bar icon when you log into your Mac (the VPN tunnel itself remains stopped until you click **Start**).

---

## Uninstallation

If you ever wish to completely remove the application and revert all system settings:

1. Double-click the **`uninstall.command`** script included in this DMG disk.
2. Provide your administrator password to clean up system security rules.
3. Delete **`SingBoxTray.app`** from your **Applications** folder.

### What the Uninstall Script Does:
* Safely removes the custom LaunchAgent background rule.
* Cleans up the custom passwordless security rule from the system configuration.

