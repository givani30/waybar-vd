# Migration Guide: Shell Script to CFFI Module

This guide helps you migrate from the shell script-based virtual desktop module to the new high-performance CFFI module.

## Why Migrate?

The CFFI module offers several advantages over the shell script approach:

- **Better Performance**: Native Rust implementation with minimal overhead
- **Real-time Updates**: Instant response to virtual desktop changes via IPC
- **Simplified Configuration**: Single module instead of multiple custom modules
- **Better Integration**: Native GTK widgets with proper styling support
- **Reduced Dependencies**: No external shell scripts or signal handling needed

## Before You Start

### Backup Your Current Configuration

```bash
# Backup your current waybar config
cp ~/.config/waybar/config ~/.config/waybar/config.backup
cp ~/.config/waybar/style.css ~/.config/waybar/style.css.backup
```

### Verify Prerequisites

```bash
# Check Hyprland virtual desktop plugin
hyprctl dispatch vdesk 1

# Check Waybar CFFI support
waybar --version | grep -i cffi
```

## Step-by-Step Migration

### Step 1: Install the CFFI Module

```bash
# Clone and build the module
git clone <repository-url>
cd waybar-virtual-desktops-cffi
./build.sh
```

### Step 2: Update Waybar Configuration

#### Old Configuration (Shell Script)

```json
{
    "modules-center": [
        "custom/vdesk-1", 
        "custom/vdesk-2", 
        "custom/vdesk-3", 
        "custom/vdesk-4", 
        "custom/vdesk-5"
    ],
    
    "custom/vdesk-1": {
        "format": "{}",
        "return-type": "json",
        "exec": "~/.config/waybar/scripts/virtual-desktop.sh 1",
        "on-click": "~/.config/waybar/scripts/virtual-desktop.sh 1 click",
        "interval": "once",
        "signal": 8
    },
    "custom/vdesk-2": {
        "format": "{}",
        "return-type": "json",
        "exec": "~/.config/waybar/scripts/virtual-desktop.sh 2",
        "on-click": "~/.config/waybar/scripts/virtual-desktop.sh 2 click",
        "interval": "once",
        "signal": 8
    }
    // ... more vdesk modules
}
```

#### New Configuration (CFFI)

```json
{
    "modules-center": ["cffi/virtual-desktops"],
    
    "cffi/virtual-desktops": {
        "library-path": "~/.local/lib/waybar-modules/libwaybar_virtual_desktops_cffi.so",
        "format": "{name}",
        "show_empty": false,
        "format_icons": {
            "1": "1",
            "2": "2",
            "3": "3",
            "4": "4",
            "5": "5"
        }
    }
}
```

### Step 3: Update CSS Styling

#### Old CSS (Custom Modules)

```css
#custom-vdesk-1,
#custom-vdesk-2,
#custom-vdesk-3,
#custom-vdesk-4,
#custom-vdesk-5 {
    background-color: #4c566a;
    color: #d8dee9;
    border-radius: 3px;
    padding: 2px 6px;
    margin: 0 2px;
}

#custom-vdesk-1.focused,
#custom-vdesk-2.focused,
#custom-vdesk-3.focused,
#custom-vdesk-4.focused,
#custom-vdesk-5.focused {
    background-color: #5e81ac;
    color: #eceff4;
    font-weight: bold;
}
```

#### New CSS (CFFI Module)

```css
#cffi-virtual-desktops {
    background-color: transparent;
    padding: 0 10px;
}

#cffi-virtual-desktops .vdesk-focused {
    background-color: #5e81ac;
    color: #eceff4;
    border-radius: 3px;
    padding: 2px 6px;
    margin: 0 2px;
    font-weight: bold;
}

#cffi-virtual-desktops .vdesk-unfocused {
    background-color: #4c566a;
    color: #d8dee9;
    border-radius: 3px;
    padding: 2px 6px;
    margin: 0 2px;
}

#cffi-virtual-desktops .vdesk-unfocused:hover {
    background-color: #5e81ac;
    color: #eceff4;
}
```

### Step 4: Remove Old Files

After confirming the new module works:

```bash
# Remove old shell scripts (if you no longer need them)
rm ~/.config/waybar/scripts/virtual-desktop.sh

# Remove old systemd service (if used)
systemctl --user disable waybar-vdesk-monitor.service
rm ~/.config/systemd/user/waybar-vdesk-monitor.service
```

### Step 5: Restart Waybar

```bash
# Kill existing waybar instances
pkill waybar

# Start waybar with new configuration
waybar &
```

## Configuration Mapping

### Format Strings

| Shell Script Output | CFFI Format | Description |
|-------------------|-------------|-------------|
| `{"text": "Desktop 1"}` | `"{name}"` | Simple desktop name |
| `{"text": "1", "class": "focused"}` | `"{id}"` with CSS | Desktop number with styling |
| `{"text": "󰲠 Work"}` | `"{icon} {name}"` | Icon + name |
| `{"tooltip": "Desktop 1 (3 windows)"}` | Automatic tooltip | Window count in tooltip |

### CSS Classes

| Shell Script | CFFI Module | Usage |
|-------------|-------------|-------|
| `.focused` | `.vdesk-focused` | Currently active desktop |
| `.unfocused` | `.vdesk-unfocused` | Inactive desktops |
| `.empty` | `.hidden` | Empty desktops (when show_empty=false) |

### Click Handling

| Shell Script | CFFI Module | Description |
|-------------|-------------|-------------|
| `on-click: "script.sh N click"` | Automatic | Click handling built-in |
| Manual signal sending | Automatic IPC | Real-time updates |

## Advanced Migration Scenarios

### Custom Icons

If you used custom icons in your shell script:

**Shell Script:**
```bash
case $vdesk_id in
    1) icon="󰲠" ;;
    2) icon="󰲢" ;;
    3) icon="󰲤" ;;
esac
```

**CFFI Configuration:**
```json
"format_icons": {
    "1": "󰲠",
    "2": "󰲢",
    "3": "󰲤"
}
```

### Dynamic Desktop Names

If your shell script used dynamic desktop names:

**Shell Script:**
```bash
case $vdesk_id in
    1) name="Work" ;;
    2) name="Web" ;;
    3) name="Media" ;;
esac
```

**CFFI Configuration:**
```json
"format_icons": {
    "work": "💼",
    "web": "🌐", 
    "media": "🎵"
}
```

### Window Count Display

**Shell Script:**
```bash
window_count=$(hyprctl printstate | jq ...)
echo "{\"text\": \"$name ($window_count)\"}"
```

**CFFI Configuration:**
```json
"format": "{name} ({window_count})",
"show_window_count": true
```

## Troubleshooting Migration Issues

### Module Not Loading

1. **Check library path**: Ensure the path in configuration matches the installed location
2. **Verify CFFI support**: Run `waybar --version` to confirm CFFI support
3. **Check permissions**: Ensure the library file is readable

### Different Appearance

1. **Update CSS selectors**: Use new class names (`.vdesk-focused` instead of `.focused`)
2. **Adjust container styling**: The CFFI module uses a different container structure
3. **Test with minimal CSS**: Start with basic styling and add complexity

### Click Handling Issues

1. **Remove old click handlers**: The CFFI module handles clicks automatically
2. **Check Hyprland commands**: Verify `hyprctl dispatch vdesk N` works
3. **Test with debug output**: Check Waybar logs for click events

### Performance Issues

1. **Check IPC connection**: Ensure `HYPRLAND_INSTANCE_SIGNATURE` is set
2. **Monitor resource usage**: The CFFI module should use less CPU than shell scripts
3. **Verify real-time updates**: Changes should appear instantly without signals

## Rollback Plan

If you need to rollback to the shell script approach:

1. **Restore configuration backup**:
   ```bash
   cp ~/.config/waybar/config.backup ~/.config/waybar/config
   cp ~/.config/waybar/style.css.backup ~/.config/waybar/style.css
   ```

2. **Restart Waybar**:
   ```bash
   pkill waybar && waybar &
   ```

3. **Re-enable services** (if used):
   ```bash
   systemctl --user enable waybar-vdesk-monitor.service
   systemctl --user start waybar-vdesk-monitor.service
   ```

## Getting Help

If you encounter issues during migration:

1. **Check the logs**: Look at Waybar output for error messages
2. **Test the module**: Use the test script to verify basic functionality
3. **Compare configurations**: Ensure all required fields are present
4. **Ask for help**: Open an issue with your configuration and error messages

## Post-Migration Benefits

After successful migration, you should notice:

- **Faster updates**: Virtual desktop changes appear instantly
- **Lower CPU usage**: No periodic shell script execution
- **Cleaner configuration**: Single module instead of multiple custom modules
- **Better reliability**: Native code with proper error handling
- **Enhanced features**: Tooltips, window counts, and better styling support
