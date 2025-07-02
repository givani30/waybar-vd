# Waybar Virtual Desktops CFFI Module

A high-performance CFFI module for Waybar that displays Hyprland virtual desktops with real-time updates and click handling.

## Features

- **Real-time Updates**: Monitors Hyprland IPC events for instant virtual desktop state changes
- **Click Handling**: Click on virtual desktops to switch between them
- **Customizable Display**: Configurable format strings, icons, and styling
- **Performance**: Native Rust implementation with minimal overhead
- **GTK Integration**: Seamless integration with Waybar's GTK3 interface

## Prerequisites

- **Hyprland** with virtual desktop plugin installed
- **Waybar** with CFFI support (version 0.12.0+)
- **Rust** toolchain (for building)

### Hyprland Virtual Desktop Plugin

This module requires the Hyprland virtual desktop plugin. Make sure you have:

1. The virtual desktop plugin loaded in your Hyprland configuration
2. The `hyprctl` command available
3. Virtual desktop commands working: `hyprctl dispatch vdesk 1`

## Installation

### Quick Install

```bash
# Clone the repository
git clone <repository-url>
cd waybar-virtual-desktops-cffi

# Build and install
./build.sh
```

### Manual Installation

```bash
# Build the library
cargo build --release

# Install to waybar modules directory
mkdir -p ~/.local/lib/waybar-modules
cp target/release/libwaybar_virtual_desktops_cffi.so ~/.local/lib/waybar-modules/
```

## Configuration

### Basic Waybar Configuration

Add the module to your Waybar configuration:

```json
{
    "modules-center": ["cffi/virtual-desktops"],
    
    "cffi/virtual-desktops": {
        "library-path": "~/.local/lib/waybar-modules/libwaybar_virtual_desktops_cffi.so",
        "format": "{name}",
        "show_empty": false
    }
}
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `library-path` | string | **required** | Path to the compiled CFFI library |
| `format` | string | `"{name}"` | Format string for virtual desktop display |
| `show_empty` | boolean | `false` | Whether to show empty virtual desktops |
| `separator` | string | `" "` | Separator between virtual desktop elements |
| `format_icons` | object | `{}` | Icon mapping for virtual desktop IDs |
| `show_window_count` | boolean | `false` | Show window count in tooltip |
| `sort_by` | string | `"number"` | Sort method: "number", "name", "focused-first" |

### Format String Variables

The `format` string supports these variables:

- `{name}` - Virtual desktop name
- `{id}` - Virtual desktop ID number
- `{icon}` - Icon from format_icons mapping
- `{window_count}` - Number of windows on the virtual desktop

### Example Configurations

#### Simple Text Display
```json
"cffi/virtual-desktops": {
    "library-path": "~/.local/lib/waybar-modules/libwaybar_virtual_desktops_cffi.so",
    "format": "{name}",
    "show_empty": false
}
```

#### With Icons
```json
"cffi/virtual-desktops": {
    "library-path": "~/.local/lib/waybar-modules/libwaybar_virtual_desktops_cffi.so",
    "format": "{icon} {name}",
    "format_icons": {
        "1": "󰲠",
        "2": "󰲢", 
        "3": "󰲤",
        "4": "󰲦",
        "5": "󰲨"
    },
    "show_empty": true,
    "show_window_count": true
}
```

#### Advanced Configuration
```json
"cffi/virtual-desktops": {
    "library-path": "~/.local/lib/waybar-modules/libwaybar_virtual_desktops_cffi.so",
    "format": "{icon} {name} ({window_count})",
    "format_icons": {
        "work": "💼",
        "web": "🌐",
        "media": "🎵",
        "games": "🎮"
    },
    "show_empty": false,
    "show_window_count": true,
    "sort_by": "focused-first"
}
```

## CSS Styling

The module applies CSS classes that you can style:

```css
/* Virtual Desktop Module Container */
#cffi-virtual-desktops {
    background-color: transparent;
    padding: 0 10px;
}

/* Focused Virtual Desktop */
#cffi-virtual-desktops .vdesk-focused {
    background-color: #5e81ac;
    color: #eceff4;
    border-radius: 3px;
    padding: 2px 6px;
    margin: 0 2px;
    font-weight: bold;
}

/* Unfocused Virtual Desktop */
#cffi-virtual-desktops .vdesk-unfocused {
    background-color: #4c566a;
    color: #d8dee9;
    border-radius: 3px;
    padding: 2px 6px;
    margin: 0 2px;
}

/* Hover Effect */
#cffi-virtual-desktops .vdesk-unfocused:hover {
    background-color: #5e81ac;
    color: #eceff4;
}

/* Hidden Virtual Desktops */
#cffi-virtual-desktops .hidden {
    display: none;
}
```

### Available CSS Classes

- `.vdesk-focused` - Applied to the currently focused virtual desktop
- `.vdesk-unfocused` - Applied to unfocused virtual desktops
- `.hidden` - Applied to empty virtual desktops when `show_empty` is false

## Migration from Shell Script

If you're migrating from the shell script-based virtual desktop module:

### 1. Replace Module Configuration

**Old (shell script):**
```json
"custom/vdesk-1": {
    "format": "{}",
    "return-type": "json",
    "exec": "~/.config/waybar/scripts/virtual-desktop.sh 1",
    "on-click": "~/.config/waybar/scripts/virtual-desktop.sh 1 click",
    "interval": "once",
    "signal": 8
}
```

**New (CFFI):**
```json
"cffi/virtual-desktops": {
    "library-path": "~/.local/lib/waybar-modules/libwaybar_virtual_desktops_cffi.so",
    "format": "{name}",
    "show_empty": false
}
```

### 2. Update Module List

**Old:**
```json
"modules-center": ["custom/vdesk-1", "custom/vdesk-2", "custom/vdesk-3", "custom/vdesk-4", "custom/vdesk-5"]
```

**New:**
```json
"modules-center": ["cffi/virtual-desktops"]
```

### 3. Remove Signal Handling

The CFFI module handles real-time updates automatically, so you can remove any signal-based update scripts.

## Troubleshooting

### Module Not Loading

1. Check that the library path is correct
2. Verify Waybar has CFFI support: `waybar --version`
3. Check Waybar logs for error messages

### No Virtual Desktops Showing

1. Verify Hyprland virtual desktop plugin is loaded
2. Test virtual desktop commands: `hyprctl dispatch vdesk 1`
3. Check that `HYPRLAND_INSTANCE_SIGNATURE` environment variable is set

### Click Handling Not Working

1. Ensure the module is receiving click events
2. Check Waybar configuration for click handling
3. Verify `hyprctl dispatch vdesk` commands work manually

## Development

### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd waybar-virtual-desktops-cffi

# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test
```

### Testing

Use the self-contained test system in the repository:

```bash
# Check prerequisites
./test/test.sh --check

# Validate configuration 
./test/test.sh --dry-run

# Run full test (starts separate waybar instance)
./test/test.sh --run

# Build library only
./test/test.sh --build
```

The test system is **completely safe** - it runs a separate waybar instance that won't interfere with your main setup.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
