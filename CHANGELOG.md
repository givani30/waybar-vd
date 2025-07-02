# Changelog

All notable changes to the Waybar Virtual Desktops CFFI Module will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-07-02

### Added
- Initial release of the Waybar Virtual Desktops CFFI Module
- Native Rust implementation using waybar-cffi crate
- Real-time virtual desktop monitoring via Hyprland IPC
- Click handling for virtual desktop switching
- Configurable format strings with variables: `{name}`, `{id}`, `{icon}`, `{window_count}`
- Support for custom icons via `format_icons` configuration
- CSS styling with classes: `.vdesk-focused`, `.vdesk-unfocused`, `.hidden`
- Tooltip support with window count information
- Multiple sorting options: "number", "name", "focused-first"
- Option to show/hide empty virtual desktops
- GTK3 integration with proper widget management
- Background thread for IPC event monitoring
- Comprehensive error handling and logging

### Features
- **Performance**: Native code with minimal CPU overhead
- **Real-time Updates**: Instant response to virtual desktop changes
- **Customization**: Flexible format strings and icon mapping
- **Integration**: Seamless GTK3 integration with Waybar
- **Reliability**: Robust error handling and automatic reconnection
- **Documentation**: Complete installation and migration guides

### Configuration Options
- `library-path`: Path to the compiled CFFI library (required)
- `format`: Format string for display (default: "{name}")
- `show_empty`: Show empty virtual desktops (default: false)
- `separator`: Separator between elements (default: " ")
- `format_icons`: Icon mapping for virtual desktop IDs
- `show_window_count`: Show window count in tooltip (default: false)
- `sort_by`: Sort method for virtual desktops (default: "number")

### Installation
- Automated build script (`build.sh`)
- Interactive installation script (`install.sh`)
- Example configurations and CSS styling
- Migration guide from shell script approach

### Documentation
- Comprehensive README with configuration examples
- Step-by-step migration guide (MIGRATION.md)
- Installation instructions and troubleshooting
- CSS styling guide with available classes

### Dependencies
- Rust toolchain (for building)
- Waybar with CFFI support (0.12.0+)
- Hyprland with virtual desktop plugin
- waybar-cffi crate (0.1.1)
- tokio for async runtime
- serde for configuration parsing
- anyhow for error handling

### Architecture
- **Module Trait**: Implements waybar-cffi Module trait
- **IPC Monitoring**: Background thread for Hyprland event listening
- **State Management**: Thread-safe virtual desktop state tracking
- **GTK Integration**: Native widget creation and management
- **Configuration**: Serde-based JSONC configuration parsing

### Tested Environments
- Hyprland with virtual desktop plugin
- Waybar 0.12.0+ with CFFI support
- Arch Linux, Ubuntu, and other Linux distributions

## [Unreleased]

### Planned Features
- Support for virtual desktop renaming
- Custom click actions configuration
- Workspace-specific icon themes
- Integration with other Hyprland plugins
- Performance optimizations
- Additional sorting and filtering options

