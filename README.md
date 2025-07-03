# waybar-vd

A high-performance CFFI module for [Waybar](https://github.com/Alexays/Waybar) that displays [Hyprland virtual desktops](https://github.com/levnikmyskin/hyprland-virtual-desktops) with real-time updates and click handling.

![Waybar-VD in action](screenshots/hero/basic-functionality.png)

> **🔗 Virtual Desktops vs Workspaces**: Unlike regular Hyprland workspaces that switch per monitor, virtual desktops switch ALL monitors simultaneously, creating a multi-monitor desktop environment.

![Multi-monitor virtual desktop switching](screenshots/hero/multi-monitor-switching-final.gif)

[![Rust](https://img.shields.io/badge/rust-1.75+-blue.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Waybar](https://img.shields.io/badge/waybar-0.12.0+-green.svg)](https://github.com/Alexays/Waybar)

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Project Structure](#project-structure)
- [Installation](#installation)
- [Configuration](#configuration)
- [CSS Styling](#css-styling)
- [Troubleshooting](#troubleshooting)
- [Development](#development)

## Features

- **Multi-Monitor Virtual Desktops**: Unlike workspaces, switches ALL monitors simultaneously
- **Real-time Updates**: Monitors Hyprland IPC events for instant virtual desktop state changes
- **Interactive UI**: Click handling and smooth hover effects for enhanced user experience
- **Smooth Animations**: Beautiful fade-in/fade-out transitions for desktop creation and destruction
- **Customizable Display**: Configurable format strings, icons, and styling
- **Performance**: Native Rust implementation with minimal overhead
- **GTK Integration**: Seamless integration with Waybar's GTK3 interface

## Prerequisites

- **Hyprland** with virtual desktop plugin installed
- **Waybar** with CFFI support (version 0.12.0+)
- **Rust** toolchain (only if building from source)

### Hyprland Virtual Desktop Plugin

This module requires the [Hyprland Virtual Desktop Plugin](https://github.com/levnikmyskin/hyprland-virtual-desktops). Make sure you have:

1. The virtual desktop plugin installed and loaded in your Hyprland configuration
2. The `hyprctl` command available
3. Virtual desktop commands working: `hyprctl dispatch vdesk 1`

**Installation of the plugin:**
```bash
# Install via hyprpm (recommended). Hyprpm is Hyprlands plugin manager which is bundled in with recent versions of Hyprland
hyprpm update
hyprpm add https://github.com/levnikmyskin/hyprland-virtual-desktops
hyprpm enable virtual-desktops

# Or follow the manual installation guide in the plugin repository
```

## Project Structure

```
waybar-vd/
├── examples/                  # Example configuration files
│   ├── config.json           # Example Waybar configuration
│   └── style.css             # Example CSS styling
├── src/                      # Rust source code
│   ├── ui/                   # UI components
│   ├── config.rs             # Configuration handling
│   ├── hyprland.rs           # Hyprland IPC client
│   ├── lib.rs                # Main CFFI module
│   └── ...
├── test/                     # Test configuration and scripts
│   ├── test.sh               # Test suite runner
│   ├── waybar-config.json    # Test configuration
│   └── style.css             # Test styling
├── build.sh                  # Build script
├── install.sh                # Interactive installation script
└── Cargo.toml                # Rust project configuration
```

## Installation

### Download Pre-compiled Binary (Recommended)

Download the latest release from GitHub:

```bash
# Create modules directory
mkdir -p ~/.config/waybar/modules

# Download the latest release
wget -O ~/.config/waybar/modules/libwaybar_vd.so \
  https://github.com/givani30/waybar-vd/releases/latest/download/libwaybar_vd.so

# Or using curl
curl -L -o ~/.config/waybar/modules/libwaybar_vd.so \
  https://github.com/givani30/waybar-vd/releases/latest/download/libwaybar_vd.so
```

### Quick Install from Source (Alternative)

```bash
# Clone the repository
git clone https://github.com/givani30/waybar-vd.git
cd waybar-vd

# Interactive installation with examples
./install.sh
```

### Build Only

```bash
# Build and install library only
./build.sh
```

### Manual Installation

```bash
# Build the library
cargo build --release

# Install to waybar modules directory
mkdir -p ~/.config/waybar/modules
cp target/release/libwaybar_vd.so ~/.config/waybar/modules/

# Copy example configurations
mkdir -p ~/.config/waybar/examples/virtual-desktops-cffi
cp examples/* ~/.config/waybar/examples/virtual-desktops-cffi/
```

## Configuration

### Basic Waybar Configuration

Add the module to your Waybar configuration:

```json
{
    "modules-center": ["cffi/virtual-desktops"],
    
    "cffi/virtual-desktops": {
        "module_path": "~/.config/waybar/modules/libwaybar_vd.so",
        "format": "{name}",
        "show_empty": false
    }
}
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `module_path` | string | **required** | Path to the compiled CFFI module_path |
| `format` | string | `"{name}"` | Format string for virtual desktop display |
| `show_empty` | boolean | `false` | Whether to show empty virtual desktops |
| `separator` | string | `" "` | Separator between virtual desktop elements |
| `format_icons` | object | `{}` | Icon mapping for virtual desktop IDs |
| `show_window_count` | boolean | `false` | Show window count in tooltip |
| `sort_by` | string | `"number"` | Sort method: "number", "name", "focused-first" |
| `retry_max` | number | `10` | Maximum number of retry attempts for IPC operations |
| `retry_base_delay_ms` | number | `500` | Base delay in milliseconds for exponential backoff |

### Format String Variables

The `format` string supports these variables:

- `{name}` - Virtual desktop name
- `{id}` - Virtual desktop ID number
- `{icon}` - Icon from format_icons mapping
- `{window_count}` - Number of windows on the virtual desktop

### Example Configurations

After installation, example configurations are available in `~/.config/waybar/examples/virtual-desktops-cffi/` or in the project's `examples/` directory.

#### Simple Text Display
```json
"cffi/virtual-desktops": {
    "module_path": "~/.config/waybar/modules/libwaybar_vd.so",
    "format": "{name}",
    "show_empty": false
}
```

#### With Icons
```json
"cffi/virtual-desktops": {
    "module_path": "~/.config/waybar/modules/libwaybar_vd.so",
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
    "module_path": "~/.config/waybar/modules/libwaybar_vd.so",
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

> **💡 Tip**: Check the `examples/` directory for complete configuration examples including both JSON config and CSS styling.

## CSS Styling

The module uses GTK Button widgets and applies CSS classes for comprehensive styling support. Complete styling examples are available in the `examples/style.css` file.

### Styling Examples

**Material Design Theme**
![Material Design](screenshots/hero/basic-functionality.png)
*Rounded corners, colored backgrounds, smooth transitions*

**Minimal Theme**
![Minimal Theme](screenshots/styling/minimal-theme.png)
*Clean lines, simple colors, subtle focus indicators*

**Available Style Examples:**
- `examples/material-design-style.css` - Modern Material Design theme with Matugen support
- `examples/minimal-style.css` - Clean, minimal styling with subtle highlights

### Modern Styling Example

```css
/* Virtual Desktop Module Container */
#cffi-virtual-desktops {
    padding: 0px 5px;
}

/* Reset button defaults for clean styling */
#cffi-virtual-desktops button {
    background: none;
    border: none;
    box-shadow: none;
    padding: 2px 8px;
    margin: 0 2px;
    border-radius: 4px;
    color: rgba(205, 189, 255, 0.4);
    font-weight: normal;
    /* Smooth transitions for all interactions */
    transition: all 0.15s cubic-bezier(0.25, 0.46, 0.45, 0.94);
    font-size: inherit;
    font-family: inherit;
}

/* Focused Virtual Desktop */
#cffi-virtual-desktops button.vdesk-focused {
    color: #cdbdff;
    font-weight: bold;
    background-color: rgba(205, 189, 255, 0.1);
}

/* Unfocused Virtual Desktop */
#cffi-virtual-desktops button.vdesk-unfocused {
    color: rgba(205, 189, 255, 0.3);
    font-weight: normal;
}

/* Manual Hover Effects (CSS :hover doesn't work in CFFI modules) */
#cffi-virtual-desktops button.hover {
    background-color: rgba(205, 189, 255, 0.15);
    color: rgba(205, 189, 255, 0.9);
}

#cffi-virtual-desktops button.hover.vdesk-focused {
    background-color: rgba(205, 189, 255, 0.25);
    color: #cdbdff;
}

#cffi-virtual-desktops button.hover.vdesk-unfocused {
    background-color: rgba(205, 189, 255, 0.12);
    color: rgba(205, 189, 255, 0.7);
}

/* Animation States */
#cffi-virtual-desktops button.creating {
    opacity: 0;
}

#cffi-virtual-desktops button.destroying {
    opacity: 0;
    padding: 0;
    margin: 0;
}

/* Hidden Virtual Desktops */
#cffi-virtual-desktops button.hidden {
    opacity: 0;
    padding: 0;
    margin: 0;
}
```

### Available CSS Classes

#### State Classes
- `button.vdesk-focused` - Applied to the currently focused virtual desktop
- `button.vdesk-unfocused` - Applied to unfocused virtual desktops
- `button.hidden` - Applied to empty virtual desktops when `show_empty` is false

#### Interactive Classes
- `button.hover` - Applied during mouse hover (manual hover state management)
- `button.creating` - Applied briefly when new desktop buttons are created
- `button.destroying` - Applied briefly when desktop buttons are being removed

### Styling Notes

#### Hover Effects
**Important**: Due to Waybar CFFI module limitations, native CSS `:hover` pseudo-selectors don't work. The module implements manual hover state management using the `.hover` class. Always style hover effects using `button.hover` instead of `button:hover`.

#### Button Reset
The module uses GTK Button widgets for proper click handling and accessibility. Use the button reset styles shown above to achieve a clean, label-like appearance while maintaining interactive functionality.

#### Animations
The module includes smooth 150ms fade-in/fade-out animations for desktop creation and destruction. The CSS transitions should match this timing for consistent visual feedback.

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
git clone https://github.com/givani30/waybar-vd.git
cd waybar-vd

# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test
```

### Testing
Run cargo test for unit test:
```bash
cargo test
```

There is also a self-contained test suite in the `test` directory, which spawns a waybar instance with a minimal configuration to test the module in isolation:

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

## Architecture

### Core Components

- **VirtualDesktopsModule**: Main CFFI module implementing the Waybar Module trait
- **HyprlandIPC**: Async IPC client for communicating with Hyprland's Unix sockets
- **VirtualDesktopsManager**: State manager for tracking virtual desktop information
- **ModuleConfig**: Configuration handler with format string processing

### Technical Details

- **Language**: Rust (2021 edition)
- **Runtime**: Tokio async runtime for IPC operations
- **UI Framework**: GTK3 Button widgets via waybar-cffi bindings
- **IPC Protocol**: Direct Unix socket communication with Hyprland
- **Threading**: Background thread for event monitoring, main thread for UI updates
- **Animations**: Manual hover state management with 150ms CSS transitions
- **Event Handling**: Native GTK enter/leave notify events for reliable hover detection

## Performance

- **Memory**: Low memory footprint with efficient widget reuse
- **CPU**: Minimal CPU usage through event-driven updates
- **Network**: Local Unix socket communication (no network overhead)
- **Responsiveness**: Real-time updates via Hyprland event system

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Guidelines

- Follow Rust best practices and clippy suggestions
- Add tests for new functionality
- Update documentation for configuration changes
- Test with the provided test suite before submitting

## Acknowledgments

- [Waybar](https://github.com/Alexays/Waybar) - The fantastic status bar this module extends
- [Hyprland](https://hyprland.org/) - The dynamic tiling Wayland compositor
- [Hyprland Virtual Desktops Plugin](https://github.com/levnikmyskin/hyprland-virtual-desktops) - The essential plugin that enables virtual desktop functionality
- [waybar-cffi](https://crates.io/crates/waybar-cffi) - The CFFI interface enabling native modules
